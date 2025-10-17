use notify::EventKind;
use std::{collections::HashMap, path::PathBuf, sync::mpsc, thread, time::Duration};

pub struct Debouncer {
    active_timers: HashMap<PathBuf, mpsc::Sender<()>>,
    output_tx: mpsc::Sender<(EventKind, PathBuf)>,
    duration: Duration,
}

impl Debouncer {
    pub fn new(output_tx: mpsc::Sender<(EventKind, PathBuf)>, duration: Duration) -> Self {
        Debouncer {
            active_timers: HashMap::new(),
            output_tx,
            duration,
        }
    }

    pub fn run(&mut self, input_rx: mpsc::Receiver<(EventKind, PathBuf)>) {
        while let Ok((event, path)) = input_rx.recv() {
            // NOTE: ec is the canceller
            if let Some(ec) = self.active_timers.remove(&path) {
                let _ = ec.send(());
            }

            // NOTE: c_tx is cancel sender, c_rx is cancel reciever
            let (c_tx, c_rx) = mpsc::channel();
            let output_tx = self.output_tx.clone();
            let dur = self.duration;

            let key_path = path.clone();

            thread::spawn(move || {
                if let Err(mpsc::RecvTimeoutError::Timeout) = c_rx.recv_timeout(dur) {
                    let _ = output_tx.send((event, path));
                }
            });

            self.active_timers.insert(key_path, c_tx);
        }
    }
}
