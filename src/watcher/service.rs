//! Real-time file system watching service.
//!
//! This module provides the main [`WatcherService`] that monitors file system
//! changes and automatically updates the search index. It handles debouncing
//! of rapid changes and provides graceful shutdown capabilities.
//!
//! # Architecture
//!
//! The service uses a multi-threaded architecture:
//! - **File System Watcher**: Monitors directory for changes using the `notify` crate
//! - **Debouncer**: Prevents rapid successive changes from causing excessive updates
//! - **Handler**: Processes debounced events and updates the SQLite index
//!
//! # Thread Safety
//!
//! The service is designed to be thread-safe and can be safely used in
//! concurrent applications. All shared state is properly synchronized.

use std::{
    path::PathBuf,
    sync::mpsc::{Sender, channel},
    thread::{self, JoinHandle},
    time::Duration,
};

use notify::{EventKind, RecommendedWatcher};

use crate::{
    error::OraError,
    watcher::{debounce::Debouncer, event::setup_file_watcher, handler::FileIndexHandler, index},
};

/// A service that monitors file system changes and maintains an up-to-date search index.
///
/// The `WatcherService` provides real-time monitoring of a directory tree,
/// automatically indexing new, modified, and deleted Markdown files. It uses
/// debouncing to prevent excessive updates during rapid file changes.
///
/// # Lifecycle
///
/// 1. **Creation**: Use [`WatcherService::create`] to initialize the service
/// 2. **Start**: Call [`WatcherService::run`] to begin monitoring
/// 3. **Shutdown**: Use [`WatcherService::shutdown`] to stop gracefully
///
/// # Thread Management
///
/// The service spawns two background threads:
/// - A debouncer thread that processes raw file system events
/// - A handler thread that updates the search index
///
/// Both threads are properly joined during shutdown to ensure clean termination.
pub struct WatcherService {
    /// Handles file system events and updates the search index.
    handler: FileIndexHandler,

    /// Handle to the debouncer thread.
    debouncer_thread: Option<JoinHandle<()>>,

    /// Handle to the handler thread.
    handler_thread: Option<JoinHandle<()>>,

    /// Channel for signaling shutdown to background threads.
    shutdown_tx: Option<Sender<()>>,

    /// Debounce duration for file system events.
    duration: Duration,

    /// Path being monitored for changes.
    watch_path: PathBuf,

    /// The underlying file system watcher.
    watcher: Option<RecommendedWatcher>,
}

impl WatcherService {
    /// Creates a new watcher service for the given shelf path.
    ///
    /// Initializes the search index and prepares the service for monitoring.
    /// The service must be started by calling [`WatcherService::run`].
    ///
    /// # Arguments
    /// * `shelf_path` - The directory path to monitor for changes
    /// * `debounce_duration` - How long to wait before processing file changes
    ///
    /// # Returns
    /// A new `WatcherService` instance ready to be started
    ///
    /// # Panics
    /// Panics if the search index cannot be created. This typically indicates
    /// permission issues or an invalid path.
    ///
    /// # Examples
    /// ```rust,no_run
    /// use ora_core::watcher::service::WatcherService;
    /// use std::time::Duration;
    /// use std::path::PathBuf;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let shelf_path = PathBuf::from("/path/to/notes");
    /// let mut watcher = WatcherService::create(&shelf_path, Duration::from_millis(100))?;
    /// # Ok(())
    /// # }
    /// ```
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
            watcher: None,
        })
    }

    /// Starts the file system monitoring service.
    ///
    /// This method spawns background threads to handle file system events
    /// and update the search index. It returns immediately after starting
    /// the threads, allowing the service to run concurrently.
    ///
    /// # Thread Behavior
    ///
    /// - **Debouncer Thread**: Processes raw file system events and applies debouncing
    /// - **Handler Thread**: Receives debounced events and updates the SQLite index
    ///
    /// # Event Processing
    ///
    /// Only Markdown files (`.md`) that are not hidden (don't start with `.`)
    /// are processed. Other files are ignored.
    ///
    /// # Errors
    ///
    /// Returns an error if the file system watcher cannot be initialized.
    /// This can happen due to permission issues or invalid paths.
    ///
    /// # Examples
    /// ```rust,no_run
    /// use ora_core::watcher::service::WatcherService;
    /// use std::time::Duration;
    /// use std::path::PathBuf;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut watcher = WatcherService::create(
    ///     &PathBuf::from("/path/to/notes"),
    ///     Duration::from_millis(100)
    /// )?;
    ///
    /// // Start monitoring
    /// watcher.run()?;
    ///
    /// // Service is now running in background threads
    /// # Ok(())
    /// # }
    /// ```
    pub fn run(&mut self) -> Result<(), OraError> {
        let (raw_tx, raw_rx) = channel::<(EventKind, PathBuf)>();
        let (debounced_tx, debounced_rx) = channel::<(EventKind, PathBuf)>();

        let watcher = setup_file_watcher(&self.watch_path, raw_tx)?;
        self.watcher = Some(watcher);

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

    /// Shuts down the watcher service gracefully.
    ///
    /// This method stops the file system watcher and waits for all background
    /// threads to complete their work. It ensures clean shutdown without
    /// losing any pending file system events.
    ///
    /// # Shutdown Process
    ///
    /// 1. Stops the file system watcher (prevents new events)
    /// 2. Closes the shutdown channel (signals threads to exit)
    /// 3. Waits for debouncer thread to finish
    /// 4. Waits for handler thread to finish
    ///
    /// # Blocking Behavior
    ///
    /// This method blocks until all background threads have terminated.
    /// The shutdown typically completes quickly, but may take longer if
    /// there are many pending file system events.
    ///
    /// # Examples
    /// ```rust,no_run
    /// use ora_core::watcher::service::WatcherService;
    /// use std::time::Duration;
    /// use std::path::PathBuf;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut watcher = WatcherService::create(
    ///     &PathBuf::from("/path/to/notes"),
    ///     Duration::from_millis(100)
    /// )?;
    ///
    /// watcher.run()?;
    ///
    /// // Later, when shutting down:
    /// watcher.shutdown()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn shutdown(&mut self) -> Result<(), OraError> {
        self.watcher.take();
        self.shutdown_tx.take();

        if let Some(handle) = self.debouncer_thread.take() {
            let _ = handle.join();
        }

        if let Some(handle) = self.handler_thread.take() {
            let _ = handle.join();
        }

        Ok(())
    }

    /// Gets access to the underlying search index.
    ///
    /// This method is only available when running with the `test-methods` feature.
    /// It provides direct access to the same index instance that the watcher
    /// uses, which is useful for testing to avoid double indexing issues.
    ///
    /// # Feature Flag
    ///
    /// This method is only available when compiling with `--features test-methods`.
    ///
    /// # Testing Usage
    ///
    /// In tests, use this method to access the index directly rather than
    /// creating a new index instance, which could cause conflicts with the
    /// running watcher service.
    #[cfg(feature = "test-methods")]
    pub fn get_index(&self) -> index::Index {
        self.handler.get_index()
    }
}
