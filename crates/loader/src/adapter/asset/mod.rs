mod fs;
mod hexz;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::loader::SourceMount;

pub(crate) use fs::FsFormat;
pub use hexz::mount as mount_hexz;
pub(crate) use hexz::{HexzFormat, HexzProjectAdapter};

/// Physical layout/container rules owned by one asset adapter.
pub trait FormatAdapter: Send + Sync {
    fn name(&self) -> &'static str;
    fn mount(&self, project_root: &Path, location: &str) -> Result<SourceMount>;
}

fn resolve_local(project_root: &Path, location: &str) -> Result<PathBuf> {
    let unresolved = project_root.join(location);
    unresolved
        .canonicalize()
        .with_context(|| format!("failed to resolve adapter source {}", unresolved.display()))
}

/// Convenience selector for development inputs; concrete formats keep their
/// own adapter modules below this category.
pub(crate) struct AutoFormat;

impl FormatAdapter for AutoFormat {
    fn name(&self) -> &'static str {
        "auto"
    }

    fn mount(&self, project_root: &Path, location: &str) -> Result<SourceMount> {
        let path = resolve_local(project_root, location)?;
        if path.extension().and_then(|value| value.to_str()) == Some("hxz") {
            HexzFormat.mount(project_root, location)
        } else {
            FsFormat.mount(project_root, location)
        }
    }
}
