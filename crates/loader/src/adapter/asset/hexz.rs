use std::path::Path;

use anyhow::{Context, Result};
use crabgal_core::config::GameConfig;

use crate::loader::{HexzArchive, SourceMount, load_hexz_project_from_archive};
use crate::{AdaptedProject, ProjectAdapter};

use super::{FormatAdapter, resolve_local};

/// Standard Hexz asset archive backed by `hexz_k`.
pub(crate) struct HexzFormat;

/// Complete packaged-project opener kept next to the Hexz asset format.
pub(crate) struct HexzProjectAdapter;

impl ProjectAdapter for HexzProjectAdapter {
    fn name(&self) -> &'static str {
        "hexz"
    }

    fn detect(&self, project_root: &Path) -> Result<bool> {
        Ok(project_root.is_file()
            && project_root.extension().and_then(|value| value.to_str()) == Some("hxz"))
    }

    fn open(&self, project_root: &Path) -> Result<AdaptedProject> {
        let archive = mount(project_root)?;
        let yaml = archive.read(Path::new("config.yaml"))?;
        let yaml = std::str::from_utf8(&yaml).context("Hexz config.yaml is not UTF-8")?;
        let config = GameConfig::from_yaml(yaml).context("invalid Hexz config.yaml")?;
        let content = load_hexz_project_from_archive(archive, &config.adapter.asset)?;
        let root = project_root
            .canonicalize()
            .unwrap_or_else(|_| project_root.to_owned())
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_owned();
        Ok(AdaptedProject {
            format: self.name(),
            root,
            config,
            content,
        })
    }
}

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
