use std::path::Path;

use anyhow::{Result, bail};

use crate::adapter::{FormatAdapter, resolve_local};
use crate::loader::SourceMount;

/// Development filesystem source with direct logical-path access and no unpack step.
pub(crate) struct FsFormat;

impl FormatAdapter for FsFormat {
    fn name(&self) -> &'static str {
        "fs"
    }

    fn mount(&self, project_root: &Path, location: &str) -> Result<SourceMount> {
        let root = resolve_local(project_root, location)?;
        if !root.is_dir() {
            bail!(
                "filesystem asset source is not a directory: {}",
                root.display()
            );
        }
        if root.join("assets").is_dir() || root.join("scripts").is_dir() {
            Ok(SourceMount::project(self.name(), root))
        } else {
            Ok(SourceMount::assets(
                self.name(),
                root.display().to_string(),
                root,
            ))
        }
    }
}
