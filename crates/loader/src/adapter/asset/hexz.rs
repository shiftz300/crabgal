use std::path::Path;

use anyhow::Result;

use crate::adapter::{FormatAdapter, resolve_local};
use crate::loader::{HexzArchive, SourceMount};

/// Standard Hexz asset archive backed by `hexz_k`.
pub(crate) struct HexzFormat;

impl FormatAdapter for HexzFormat {
    fn name(&self) -> &'static str {
        "hexz"
    }

    fn mount(&self, project_root: &Path, location: &str) -> Result<SourceMount> {
        let package = resolve_local(project_root, location)?;
        let archive = mount(&package)?;
        if archive.is_directory(Path::new("assets")) || archive.is_directory(Path::new("scripts")) {
            SourceMount::hexz_project(self.name(), archive, "")
        } else {
            SourceMount::hexz_assets(self.name(), archive, "")
        }
    }
}

/// Open a standard Hexz archive without extracting it.
pub fn mount(package: &Path) -> Result<HexzArchive> {
    HexzArchive::open(package)
}
