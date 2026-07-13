//! Multi-source script hot reload.

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};

use anyhow::Result;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use crate::{ContentProject, ScriptLanguageRegistry};

/// Owns the notification backend and filters changes through the registered
/// source-language adapters.
pub struct ScriptWatcher {
    receiver: Receiver<PathBuf>,
    _watcher: RecommendedWatcher,
}

impl ScriptWatcher {
    pub fn start(project: &ContentProject) -> Result<Self> {
        Self::start_with_languages(
            &project.watched_script_roots(),
            ScriptLanguageRegistry::default(),
        )
    }

    pub fn start_with_languages(
        script_dirs: &[PathBuf],
        languages: ScriptLanguageRegistry,
    ) -> Result<Self> {
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
                if languages.supports(&path) {
                    let _ = sender.send(path);
                }
            }
        })?;
        for script_dir in script_dirs.iter().filter(|path| path.is_dir()) {
            watcher.watch(script_dir, RecursiveMode::Recursive)?;
        }

        Ok(Self {
            receiver,
            _watcher: watcher,
        })
    }

    pub fn drain(&self) -> Vec<PathBuf> {
        self.receiver.try_iter().collect()
    }
}
