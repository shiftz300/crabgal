use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use crabgal_core::Action;

use crate::{parse_script, parse_webgal};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptFormat {
    Crab,
    WebGal,
}

impl ScriptFormat {
    pub fn from_path(path: &Path) -> Option<Self> {
        match path.extension().and_then(|extension| extension.to_str()) {
            Some("crab") => Some(Self::Crab),
            Some("txt") => Some(Self::WebGal),
            _ => None,
        }
    }

    pub fn parse(self, source: &str) -> Vec<Action> {
        match self {
            Self::Crab => parse_script(source),
            Self::WebGal => parse_webgal(source),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LoadedScene {
    pub name: String,
    pub path: PathBuf,
    pub actions: Vec<Action>,
}

/// Loads supported scripts in stable filename order.
pub fn load_scenes(script_dir: &Path) -> Result<Vec<LoadedScene>> {
    let entries = fs::read_dir(script_dir)
        .with_context(|| format!("failed to read script directory {}", script_dir.display()))?;

    let mut paths = entries
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<Vec<_>>>()
        .with_context(|| format!("failed to enumerate scripts in {}", script_dir.display()))?;
    paths.retain(|path| ScriptFormat::from_path(path).is_some());
    paths.sort();

    let scenes = paths
        .into_iter()
        .map(load_scene)
        .collect::<Result<Vec<_>>>()?;
    let mut names = HashSet::with_capacity(scenes.len());
    for scene in &scenes {
        if !names.insert(&scene.name) {
            anyhow::bail!(
                "duplicate scene name {:?} in {}",
                scene.name,
                script_dir.display()
            );
        }
    }
    Ok(scenes)
}

fn load_scene(path: PathBuf) -> Result<LoadedScene> {
    let format = ScriptFormat::from_path(&path)
        .with_context(|| format!("unsupported script format: {}", path.display()))?;
    let source = fs::read_to_string(&path)
        .with_context(|| format!("failed to read script {}", path.display()))?;
    let name = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .with_context(|| format!("script has no valid UTF-8 stem: {}", path.display()))?
        .to_owned();

    Ok(LoadedScene {
        name,
        path,
        actions: format.parse(&source),
    })
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    #[test]
    fn detects_supported_formats() {
        assert_eq!(
            ScriptFormat::from_path(Path::new("scene.crab")),
            Some(ScriptFormat::Crab)
        );
        assert_eq!(
            ScriptFormat::from_path(Path::new("scene.txt")),
            Some(ScriptFormat::WebGal)
        );
        assert_eq!(ScriptFormat::from_path(Path::new("scene.md")), None);
    }

    #[test]
    fn loads_scenes_in_filename_order() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("crabgal-scenes-{nonce}"));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("b.crab"), "say B: second").unwrap();
        fs::write(root.join("a.crab"), "say A: first").unwrap();
        fs::write(root.join("ignored.md"), "not a scene").unwrap();

        let scenes = load_scenes(&root).unwrap();

        assert_eq!(
            scenes
                .iter()
                .map(|scene| scene.name.as_str())
                .collect::<Vec<_>>(),
            ["a", "b"]
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn rejects_duplicate_scene_stems() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("crabgal-duplicate-scenes-{nonce}"));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("main.crab"), "say A: first").unwrap();
        fs::write(root.join("main.txt"), "A:second;").unwrap();

        let error = load_scenes(&root).unwrap_err();

        assert!(error.to_string().contains("duplicate scene name"));
        let _ = fs::remove_dir_all(root);
    }
}
