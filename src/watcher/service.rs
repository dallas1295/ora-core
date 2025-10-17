use std::{
    path::PathBuf,
    sync::mpsc::{Sender, channel},
    thread::{self, JoinHandle},
    time::Duration,
};

use notify::EventKind;

use crate::{
    error::OraError,
    watcher::{debounce::Debouncer, handler::FileIndexHandler, index, watcher::setup_file_watcher},
};

pub struct WatcherService {
    handler: FileIndexHandler,
    debouncer_thread: Option<JoinHandle<()>>,
    handler_thread: Option<JoinHandle<()>>,
    shutdown_tx: Option<Sender<()>>,
    duration: Duration,
    watch_path: PathBuf,
}

impl WatcherService {
    pub fn create(shelf_path: &PathBuf, debounce_duration: Duration) -> Result<Self, OraError> {
        let index = index::Index::new(shelf_path)
            .expect("failed to create index, check provided path or permissions");
        let handler = FileIndexHandler::new(index);

        Ok(WatcherService {
            handler,
            debouncer_thread: None,
            handler_thread: None,
            shutdown_tx: None,
            duration: debounce_duration,
            watch_path: shelf_path.to_path_buf(),
        })
    }

    pub fn run(&mut self) -> Result<(), OraError> {
        let (raw_tx, raw_rx) = channel::<(EventKind, PathBuf)>();
        let (debounced_tx, debounced_rx) = channel::<(EventKind, PathBuf)>();

        let _watcher = setup_file_watcher(&self.watch_path, raw_tx)?;

        let mut debouncer = Debouncer::new(debounced_tx, self.duration);

        let debouncer_thread = thread::spawn(move || {
            debouncer.run(raw_rx);
        });

        let handler = self.handler.clone();

        let handler_thread = thread::spawn(move || {
            while let Ok((event_kind, path)) = debounced_rx.recv() {
                match event_kind {
                    EventKind::Create(_) => {
                        if let Err(e) = handler.handle_create(&path) {
                            eprintln!("Handler create error: {}", e);
                        }
                    }
                    EventKind::Modify(_) => {
                        if let Err(e) = handler.handle_modify(&path) {
                            eprintln!("Handler modify error: {}", e);
                        }
                    }
                    EventKind::Remove(_) => {
                        if let Err(e) = handler.handle_remove(&path) {
                            eprintln!("Handler remove error: {}", e);
                        }
                    }
                    _ => {}
                }
            }
        });

        self.debouncer_thread = Some(debouncer_thread);
        self.handler_thread = Some(handler_thread);

        Ok(())
    }

    pub fn shutdown(&mut self) -> Result<(), OraError> {
        // NOTE: drops the sender to close the channels
        self.shutdown_tx.take();

        // NOTE: Joins threads to stop everything.
        if let Some(handle) = self.debouncer_thread.take() {
            let _ = handle.join();
        }

        if let Some(handle) = self.handler_thread.take() {
            let _ = handle.join();
        }

        Ok(())
    }
}
