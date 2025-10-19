//! Event debouncing for file system watching.
//!
//! This module provides debouncing functionality to prevent rapid successive
//! file system events from causing excessive updates. When multiple events
//! occur for the same file within a short time window, only the last event
//! is processed.
//!
//! # Debouncing Logic
//!
//! - When an event is received for a file, any existing timer is cancelled
//! - A new timer is started for the configured duration
//! - If the timer completes without being cancelled, the event is forwarded
//! - If another event arrives, the timer is reset
//!
//! # Performance Benefits
//!
//! Debouncing improves performance by:
//! - Reducing database writes during rapid file changes
//! - Preventing excessive search index updates
//! - Smoothing out bursty file system activity

use notify::EventKind;
use std::{collections::HashMap, path::PathBuf, sync::mpsc, thread, time::Duration};

/// Debounces file system events to prevent excessive processing.
///
/// The `Debouncer` maintains a collection of active timers, one for each
/// file path that has pending events. When multiple events arrive for the
/// same file, only the final event is processed after the debounce delay.
///
/// # Thread Safety
///
/// The debouncer is designed to run in its own thread and communicates
/// via channels. It's not thread-safe for concurrent access from multiple
/// threads.
pub struct Debouncer {
    /// Map of file paths to their timer cancellation channels.
    active_timers: HashMap<PathBuf, mpsc::Sender<()>>,
    
    /// Channel for sending debounced events to the handler.
    output_tx: mpsc::Sender<(EventKind, PathBuf)>,
    
    /// Duration to wait before forwarding events.
    duration: Duration,
}

impl Debouncer {
    /// Creates a new debouncer with the specified output channel and duration.
    ///
    /// # Arguments
    /// * `output_tx` - Channel for sending debounced events
    /// * `duration` - Time to wait before forwarding events
    ///
    /// # Returns
    /// A new `Debouncer` instance
    pub fn new(output_tx: mpsc::Sender<(EventKind, PathBuf)>, duration: Duration) -> Self {
        Debouncer {
            active_timers: HashMap::new(),
            output_tx,
            duration,
        }
    }

    /// Runs the debouncer, processing events from the input channel.
    ///
    /// This method blocks until the input channel is closed, processing
    /// events as they arrive. For each event, it cancels any existing
    /// timer for that file and starts a new one.
    ///
    /// # Arguments
    /// * `input_rx` - Channel to receive raw file system events
    ///
    /// # Behavior
    /// - Blocks until input channel is closed
    /// - Spawns a new thread for each debounced timer
    /// - Forwards events after the debounce delay if not cancelled
    ///
    /// # Thread Management
    ///
    /// Each pending event spawns a short-lived timer thread. These threads
    /// automatically terminate when either the timer expires or is cancelled.
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
