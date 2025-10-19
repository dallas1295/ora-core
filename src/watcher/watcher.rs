//! Low-level file system watcher setup.
//!
//! This module provides the foundation for file system monitoring using
//! the `notify` crate. It sets up a recursive file system watcher that
//! sends events through a channel for further processing.
//!
//! # Event Filtering
//!
//! The watcher filters events to only include relevant file operations:
//! - Create events (new files)
//! - Modify events (file changes)
//! - Remove events (file deletions)
//!
//! Other events like metadata changes or directory operations are ignored.

use notify::{Config, Event, EventKind, RecommendedWatcher, Watcher};
use std::{
    path::{Path, PathBuf},
    sync::mpsc::Sender,
};

/// Sets up a file system watcher for the given path.
///
/// Creates a recursive file system watcher that monitors the specified
/// directory and all subdirectories for file changes. Events are sent
/// through the provided channel for further processing.
///
/// # Arguments
/// * `watch_path` - The directory path to monitor
/// * `raw_event_tx` - Channel for sending file system events
///
/// # Returns
/// A `RecommendedWatcher` instance that can be used to control monitoring
///
/// # Behavior
/// - Monitors the directory recursively (all subdirectories)
/// - Only forwards create, modify, and remove events
/// - Ignores metadata changes and other non-essential events
/// - Sends events as `(EventKind, PathBuf)` tuples
///
/// # Errors
/// Returns `notify::Error` if the watcher cannot be initialized
///
/// # Examples
/// ```rust,no_run
/// use notify::EventKind;
/// use std::sync::mpsc::channel;
/// use ora_core::watcher::watcher::setup_file_watcher;
/// use std::path::Path;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let (tx, rx) = channel();
/// let watcher = setup_file_watcher(Path::new("/path/to/watch"), tx)?;
/// # Ok(())
/// # }
/// ```
pub fn setup_file_watcher(
    watch_path: &Path,
    raw_event_tx: Sender<(EventKind, PathBuf)>,
) -> Result<RecommendedWatcher, notify::Error> {
    let event_handler = move |res: Result<Event, notify::Error>| {
        if let Ok(event) = res {
            for path in event.paths {
                match event.kind {
                    EventKind::Modify(_) | EventKind::Create(_) | EventKind::Remove(_) => {
                        let _ = raw_event_tx.send((event.kind, path));
                    }
                    _ => {}
                }
            }
        }
    };

    let mut watcher = RecommendedWatcher::new(event_handler, Config::default())?;
    watcher.watch(watch_path, notify::RecursiveMode::Recursive)?;

    Ok(watcher)
}
