//! Native editor project adapters.
//!
//! Unlike script adapters, project adapters may need several related JSON
//! files to resolve IDs and compile one adapter-neutral program. They remain
//! inside the loader crate so neither the core VM nor the Bevy runtime needs
//! to know which editor authored the project.

mod letsgal;

pub use letsgal::{LetsGalProjectAdapter, LetsGalStudioIntegration};

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use crabgal_core::config::GameConfig;

use crate::{ContentProject, LoadedScene};

/// Editor cursor expressed in the stable scene id and one-based source step
/// used by a structured compiler's [`crate::SourceSpan`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectDebugCursor {
    pub scene: String,
    pub source_step: usize,
}

/// Multi-file scene compiler retained by [`ContentProject`] for startup and
/// hot reload. Keeping this interface narrow makes editor formats replaceable
/// without leaking their data model into the runtime.
pub trait StructuredSceneLoader: Send + Sync {
    fn name(&self) -> &'static str;
    fn load(&self, project_root: &Path) -> Result<Vec<LoadedScene>>;
    fn watch_roots(&self, project_root: &Path) -> Vec<PathBuf>;
    fn accepts_change(&self, path: &Path) -> bool;

    /// Rebuilds adapter-derived configuration such as asset aliases after an
    /// editor manifest changes. Script-only adapters keep the startup config.
    fn load_config(&self, _project_root: &Path) -> Result<Option<GameConfig>> {
        Ok(None)
    }

    fn is_debug_cursor_change(&self, _path: &Path) -> bool {
        false
    }

    fn debug_cursor(&self, _project_root: &Path) -> Result<Option<ProjectDebugCursor>> {
        Ok(None)
    }
}

/// Optional development-host integration owned by an editor adapter.
///
/// This is deliberately separate from [`ProjectAdapter`]: opening and hot
/// reloading a project remain read-only, while an explicit installation
/// command may write the editor's user extension directory and opt a project
/// into that extension.
pub trait EditorIntegrationAdapter: Send + Sync {
    fn name(&self) -> &'static str;
    fn install(&self, executable: &Path, project: Option<&Path>) -> Result<()>;
    fn uninstall(&self) -> Result<()>;

    /// Runs an adapter-owned development command without leaking editor
    /// protocol details into the engine runtime.
    fn control(&self, _args: &[String]) -> Result<()> {
        anyhow::bail!(
            "editor integration {:?} has no control interface",
            self.name()
        )
    }
}

pub(super) fn structured_project(
    root: PathBuf,
    sources: Vec<crate::SourceMount>,
    loader: Arc<dyn StructuredSceneLoader>,
) -> ContentProject {
    ContentProject::with_structured_scenes(root, sources, loader)
}
