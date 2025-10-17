use notify::{Config, Event, EventKind, RecommendedWatcher, Watcher};
use std::{
    path::{Path, PathBuf},
    sync::mpsc::Sender,
};

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
