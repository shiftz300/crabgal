use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use crabgal_core::Action;

use crate::{Diagnostic, DiagnosticLevel, ResourceRef, SceneRef, ScriptLanguageRegistry};

#[derive(Debug, Clone, PartialEq)]
pub struct LoadedScene {
    pub name: String,
    pub path: PathBuf,
    pub actions: Vec<Action>,
    pub diagnostics: Vec<Diagnostic>,
    pub resources: Vec<ResourceRef>,
    pub sub_scenes: Vec<SceneRef>,
}

/// Loads supported scripts in stable filename order.
pub fn load_scenes(script_dir: &Path) -> Result<Vec<LoadedScene>> {
    load_scenes_with(script_dir, &ScriptLanguageRegistry::default())
}

pub fn load_scenes_with(
    script_dir: &Path,
    languages: &ScriptLanguageRegistry,
) -> Result<Vec<LoadedScene>> {
    let entries = fs::read_dir(script_dir)
        .with_context(|| format!("failed to read script directory {}", script_dir.display()))?;

    let mut paths = entries
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<Vec<_>>>()
        .with_context(|| format!("failed to enumerate scripts in {}", script_dir.display()))?;
    paths.retain(|path| languages.supports(path));
    paths.sort();

    let mut scenes = paths
        .into_iter()
        .map(|path| load_scene(path, languages))
        .collect::<Result<Vec<_>>>()?;
    let mut names = HashSet::with_capacity(scenes.len());
    for scene in &scenes {
        if !names.insert(scene.name.clone()) {
            anyhow::bail!(
                "duplicate scene name {:?} in {}",
                scene.name,
                script_dir.display()
            );
        }
    }
    for scene in &mut scenes {
        for reference in &scene.sub_scenes {
            if !reference.scene.contains('{') && !names.contains(&reference.scene) {
                scene.diagnostics.push(Diagnostic {
                    level: DiagnosticLevel::Error,
                    span: reference.span,
                    message: format!("referenced scene {:?} does not exist", reference.scene),
                });
            }
        }
    }
    Ok(scenes)
}

fn load_scene(path: PathBuf, languages: &ScriptLanguageRegistry) -> Result<LoadedScene> {
    let language = languages
        .language_for(&path)
        .with_context(|| format!("unsupported script format: {}", path.display()))?;
    let source = fs::read_to_string(&path)
        .with_context(|| format!("failed to read script {}", path.display()))?;
    let name = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .with_context(|| format!("script has no valid UTF-8 stem: {}", path.display()))?
        .to_owned();

    let report = language.parse(&source);
    Ok(LoadedScene {
        name,
        path,
        actions: report.actions,
        diagnostics: report.diagnostics,
        resources: report.resources,
        sub_scenes: report.sub_scenes,
    })
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;
    use crabgal_core::{State, StepResult, step};

    #[test]
    fn detects_supported_languages() {
        let languages = ScriptLanguageRegistry::default();
        assert!(languages.supports(Path::new("scene.txt")));
        assert!(!languages.supports(Path::new("scene.md")));
    }

    #[test]
    fn loads_scenes_in_filename_order() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("crabgal-scenes-{nonce}"));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("b.txt"), "B:second;").unwrap();
        fs::write(root.join("a.txt"), "A:first;").unwrap();
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
    fn loaded_scenes_execute_call_and_return_end_to_end() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("crabgal-scene-flow-{nonce}"));
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("main.txt"),
            "callScene:aside.txt;\nMain:returned;",
        )
        .unwrap();
        fs::write(root.join("aside.txt"), "Aside: inside;").unwrap();

        let mut state = State::new();
        state.scenes = load_scenes(&root)
            .unwrap()
            .into_iter()
            .map(|scene| (scene.name, scene.actions))
            .collect();
        state.current_scene = "main".into();
        step::index_labels(&mut state);

        assert_eq!(step::step(&mut state), StepResult::AwaitClick);
        assert_eq!(state.current_scene, "aside");
        assert_eq!(state.dialogue.as_ref().unwrap().text, "inside");
        step::advance(&mut state);
        assert_eq!(step::step(&mut state), StepResult::AwaitClick);
        assert_eq!(state.current_scene, "main");
        assert_eq!(state.dialogue.as_ref().unwrap().text, "returned");

        let _ = fs::remove_dir_all(root);
    }
}
