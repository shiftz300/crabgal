use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};

use anyhow::Result;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use crate::ScriptFormat;

/// Owns both the notification backend and its receiver.
///
/// Keeping the backend in this value avoids leaking it for process lifetime.
pub struct ScriptWatcher {
    receiver: Receiver<PathBuf>,
    _watcher: RecommendedWatcher,
}

impl ScriptWatcher {
    pub fn start(script_dir: &Path) -> Result<Self> {
        let (sender, receiver) = mpsc::channel();
        let mut watcher = notify::recommended_watcher(move |result: notify::Result<Event>| {
            let event = match result {
                Ok(event) => event,
                Err(error) => {
                    log::warn!("script watcher error: {error}");
                    return;
                }
            };
            if !matches!(
                event.kind,
                EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
            ) {
                return;
            }

            for path in event.paths {
                if ScriptFormat::from_path(&path).is_some() {
                    let _ = sender.send(path);
                }
            }
        })?;
        watcher.watch(script_dir, RecursiveMode::Recursive)?;

        Ok(Self {
            receiver,
            _watcher: watcher,
        })
    }

    pub fn drain(&self) -> Vec<PathBuf> {
        self.receiver.try_iter().collect()
    }
}
