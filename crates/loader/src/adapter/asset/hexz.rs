#[cfg(feature = "hexz-pack")]
use std::fs;
use std::path::Path;

#[cfg(feature = "hexz-pack")]
use anyhow::Context;
use anyhow::{Result, bail};
#[cfg(feature = "hexz-pack")]
use hexz_ops::pack::{PackConfig, PackTransformFlags, pack_archive};

use crate::adapter::{FormatAdapter, resolve_local};
#[cfg(feature = "hexz-pack")]
use crate::loader::hexz_password;
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

/// Build a standard encrypted `.hxz` archive through the official Hexz
/// packer. The compile-time key is intentionally weak protection against
/// casual extraction, not a DRM guarantee.
#[cfg(feature = "hexz-pack")]
pub fn pack(project: &Path, output: &Path) -> Result<()> {
    let project = project
        .canonicalize()
        .with_context(|| format!("failed to resolve project {}", project.display()))?;
    if !project.is_dir() {
        bail!("Hexz input is not a directory: {}", project.display());
    }
    if let Some(parent) = output.parent().filter(|path| !path.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    let config = PackConfig {
        input: project,
        output: output.to_owned(),
        compression: "zstd".into(),
        password: Some(hexz_password().to_owned()),
        block_size: 64 * 1024,
        num_workers: 0,
        transform: PackTransformFlags {
            encrypt: true,
            train_dict: false,
            // Authenticated per-block encryption currently uses Hexz's
            // sequential path so nonces remain deterministic and safe.
            parallel: false,
        },
        analysis: hexz_ops::pack::PackAnalysisFlags {
            show_progress: false,
            ..Default::default()
        },
        ..Default::default()
    };
    let progress = |_done: u64, _total: u64| {};
    pack_archive(&config, Some(&progress)).context("Hexz packing failed")
}

/// Packing is development tooling and is excluded from production binaries by
/// default so `hexz-ops` does not enlarge the shipped engine.
#[cfg(not(feature = "hexz-pack"))]
pub fn pack(_project: &Path, _output: &Path) -> Result<()> {
    bail!("Hexz packing is disabled; rebuild with --features hexz-pack")
}

/// Open a standard Hexz archive without extracting it.
pub fn mount(package: &Path) -> Result<HexzArchive> {
    HexzArchive::open(package)
}

#[cfg(all(test, feature = "hexz-pack"))]
mod tests {
    use super::*;

    #[test]
    fn encrypted_hexz_round_trip_never_materializes_project_files() {
        let nonce = std::process::id();
        let root = std::env::temp_dir().join(format!("crabgal-hexz-test-{nonce}"));
        let package = root.with_extension("hxz");
        fs::create_dir_all(root.join("scripts")).unwrap();
        fs::write(root.join("config.yaml"), "title: test").unwrap();
        fs::write(root.join("scripts/start.txt"), "Hello;").unwrap();
        pack(&root, &package).unwrap();

        assert!(hexz_k::is_encrypted(&package).unwrap());
        let mounted = mount(&package).unwrap();
        assert_eq!(
            mounted.read(Path::new("config.yaml")).unwrap(),
            b"title: test"
        );
        assert_eq!(
            mounted.read(Path::new("scripts/start.txt")).unwrap(),
            b"Hello;"
        );

        let corrupt = root.with_extension("corrupt.hxz");
        let mut bytes = fs::read(&package).unwrap();
        bytes.truncate(bytes.len() / 2);
        fs::write(&corrupt, bytes).unwrap();
        assert!(mount(&corrupt).is_err());
        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_file(package);
        let _ = fs::remove_file(corrupt);
    }
}
