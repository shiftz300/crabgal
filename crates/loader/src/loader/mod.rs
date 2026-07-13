mod scenes;
mod source;
mod watcher;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use crabgal_core::config::AssetSourceConfig;

use crate::LoaderRegistry;

pub use scenes::{LoadedScene, load_scenes, load_scenes_with};
pub use source::{ContentBackend, ContentMount, HexzArchive, HexzCursor, HexzFile, hexz_password};
pub use watcher::ScriptWatcher;

/// Mounted roots produced by one complete format adapter.
#[derive(Debug, Clone)]
pub struct SourceMount {
    pub adapter: String,
    pub origin: String,
    pub asset: Option<ContentMount>,
    pub scripts: Option<ContentMount>,
}

impl SourceMount {
    pub fn project(adapter: impl Into<String>, root: PathBuf) -> Self {
        let backend = ContentBackend::FileSystem(root.clone());
        Self {
            adapter: adapter.into(),
            origin: root.display().to_string(),
            asset: Some(ContentMount::new(backend.clone(), "assets").expect("static path")),
            scripts: Some(ContentMount::new(backend, "scripts").expect("static path")),
        }
    }

    pub fn assets(adapter: impl Into<String>, origin: impl Into<String>, root: PathBuf) -> Self {
        Self {
            adapter: adapter.into(),
            origin: origin.into(),
            asset: Some(
                ContentMount::new(ContentBackend::FileSystem(root), PathBuf::new())
                    .expect("empty path"),
            ),
            scripts: None,
        }
    }

    pub fn hexz_project(
        adapter: impl Into<String>,
        archive: HexzArchive,
        prefix: impl Into<PathBuf>,
    ) -> Result<Self> {
        let prefix = prefix.into();
        let backend = ContentBackend::Hexz(archive.clone());
        Ok(Self {
            adapter: adapter.into(),
            origin: archive.path().display().to_string(),
            asset: Some(ContentMount::new(backend.clone(), prefix.join("assets"))?),
            scripts: Some(ContentMount::new(backend, prefix.join("scripts"))?),
        })
    }

    pub fn hexz_assets(
        adapter: impl Into<String>,
        archive: HexzArchive,
        prefix: impl Into<PathBuf>,
    ) -> Result<Self> {
        let origin = archive.path().display().to_string();
        Ok(Self {
            adapter: adapter.into(),
            origin,
            asset: Some(ContentMount::new(ContentBackend::Hexz(archive), prefix)?),
            scripts: None,
        })
    }
}

/// Ordered mounted view of a project. Consumers resolve from the end, so a
/// later source deterministically overrides an earlier source.
#[derive(Debug, Clone)]
pub struct ContentProject {
    pub root: PathBuf,
    pub sources: Vec<SourceMount>,
}

impl ContentProject {
    pub fn asset_mounts(&self) -> Vec<ContentMount> {
        self.sources
            .iter()
            .filter_map(|source| source.asset.clone())
            .collect()
    }

    pub fn script_mounts(&self) -> Vec<ContentMount> {
        self.sources
            .iter()
            .filter_map(|source| source.scripts.clone())
            .collect()
    }

    pub fn watched_script_roots(&self) -> Vec<PathBuf> {
        self.script_mounts()
            .into_iter()
            .filter_map(|mount| mount.filesystem_root())
            .collect()
    }
}

pub fn load_project(root: &Path, sources: &[AssetSourceConfig]) -> Result<ContentProject> {
    load_project_with(root, sources, &LoaderRegistry::default())
}

pub fn load_project_with(
    root: &Path,
    sources: &[AssetSourceConfig],
    adapters: &LoaderRegistry,
) -> Result<ContentProject> {
    if sources.is_empty() {
        bail!("project must declare at least one adapter source");
    }
    let root = root
        .canonicalize()
        .with_context(|| format!("failed to resolve project root {}", root.display()))?;
    let mut mounted = Vec::with_capacity(sources.len());
    for source in sources {
        let mount = adapters
            .mount(&source.format, &root, &source.path)
            .with_context(|| format!("failed to mount adapter source {:?}", source.path))?;
        mounted.push(mount);
    }
    Ok(ContentProject {
        root,
        sources: mounted,
    })
}

/// Open a packaged project without extracting any file. Source paths from the
/// embedded config become logical prefixes inside the same archive, preserving
/// the same low-to-high override order used during development.
pub fn load_hexz_project(package: &Path, sources: &[AssetSourceConfig]) -> Result<ContentProject> {
    load_hexz_project_from_archive(HexzArchive::open(package)?, sources)
}

pub fn load_hexz_project_from_archive(
    archive: HexzArchive,
    sources: &[AssetSourceConfig],
) -> Result<ContentProject> {
    if sources.is_empty() {
        bail!("project must declare at least one adapter source");
    }
    let mut mounted = Vec::with_capacity(sources.len());
    for source in sources {
        let path = PathBuf::from(&source.path);
        let external_hexz = matches!(source.format.as_str(), "hexz")
            || (source.format == "auto"
                && path.extension().and_then(|value| value.to_str()) == Some("hxz"));
        if external_hexz && path != Path::new(".") {
            let parent = archive.path().parent().unwrap_or_else(|| Path::new("."));
            let external = parent.join(&path);
            let external = HexzArchive::open(&external).with_context(|| {
                format!("failed to open packaged source {}", external.display())
            })?;
            let project_layout = external.is_directory(Path::new("assets"))
                || external.is_directory(Path::new("scripts"));
            mounted.push(if project_layout {
                SourceMount::hexz_project("hexz", external, "")?
            } else {
                SourceMount::hexz_assets("hexz", external, "")?
            });
            continue;
        }
        if !matches!(source.format.as_str(), "fs" | "auto" | "hexz") {
            bail!(
                "adapter {:?} cannot be resolved from inside a Hexz project",
                source.format
            );
        }
        let project_layout = archive.is_directory(&path.join("assets"))
            || archive.is_directory(&path.join("scripts"));
        mounted.push(if project_layout {
            SourceMount::hexz_project("hexz", archive.clone(), path)?
        } else {
            SourceMount::hexz_assets("hexz", archive.clone(), path)?
        });
    }
    Ok(ContentProject {
        root: archive.path().to_owned(),
        sources: mounted,
    })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    #[test]
    fn mounts_ordered_filesystem_layers() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("crabgal-content-{nonce}"));
        fs::create_dir_all(root.join("assets")).unwrap();
        fs::create_dir_all(root.join("scripts")).unwrap();
        fs::create_dir_all(root.join("packs/voices")).unwrap();
        let sources = vec![
            AssetSourceConfig::default(),
            AssetSourceConfig {
                path: "packs/voices".into(),
                format: "fs".into(),
            },
        ];

        let project = load_project(&root, &sources).unwrap();
        let root = root.canonicalize().unwrap();
        assert_eq!(
            project.asset_mounts()[0].filesystem_root().unwrap(),
            root.join("assets")
        );
        assert_eq!(
            project.asset_mounts()[1].filesystem_root().unwrap(),
            root.join("packs/voices")
        );
        assert_eq!(project.watched_script_roots(), vec![root.join("scripts")]);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_unknown_adapters() {
        let source = AssetSourceConfig {
            path: ".".into(),
            format: "missing".into(),
        };
        assert!(load_project(Path::new("."), &[source]).is_err());
    }
}
