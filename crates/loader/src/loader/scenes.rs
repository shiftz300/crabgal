use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use crabgal_core::Action;

use crate::{
    ContentMount, ContentProject, Diagnostic, DiagnosticLevel, ResourceRef, SceneRef,
    ScriptLanguageRegistry,
};

#[derive(Debug, Clone, PartialEq)]
pub struct LoadedScene {
    pub name: String,
    pub path: PathBuf,
    pub actions: Vec<Action>,
    pub diagnostics: Vec<Diagnostic>,
    pub resources: Vec<ResourceRef>,
    pub sub_scenes: Vec<SceneRef>,
}

/// Loads every script layer in stable order. A scene in a later content source
/// replaces one with the same name from an earlier source.
pub fn load_scenes(project: &ContentProject) -> Result<Vec<LoadedScene>> {
    load_scenes_with(project, &ScriptLanguageRegistry::default())
}

pub fn load_scenes_with(
    project: &ContentProject,
    languages: &ScriptLanguageRegistry,
) -> Result<Vec<LoadedScene>> {
    let mut merged = BTreeMap::new();
    for script_mount in project.script_mounts() {
        for mut scene in load_directory(&script_mount, languages)? {
            if let Some(previous) = merged.insert(scene.name.clone(), scene.clone()) {
                scene.diagnostics.push(Diagnostic {
                    level: DiagnosticLevel::Warning,
                    span: crate::SourceSpan { line: 1, column: 1 },
                    message: format!(
                        "scene {:?} overrides {}",
                        scene.name,
                        previous.path.display()
                    ),
                });
                merged.insert(scene.name.clone(), scene);
            }
        }
    }
    let mut scenes = merged.into_values().collect::<Vec<_>>();
    validate_scene_references(&mut scenes);
    Ok(scenes)
}

fn load_directory(
    scripts: &ContentMount,
    languages: &ScriptLanguageRegistry,
) -> Result<Vec<LoadedScene>> {
    let mut paths = scripts.read_directory(Path::new(""))?;
    paths.retain(|path| languages.supports(path));
    paths.sort();

    let scenes = paths
        .into_iter()
        .map(|path| load_scene(scripts, path, languages))
        .collect::<Result<Vec<_>>>()?;
    let mut names = HashSet::with_capacity(scenes.len());
    for scene in &scenes {
        if !names.insert(scene.name.clone()) {
            anyhow::bail!(
                "duplicate scene name {:?} in {}",
                scene.name,
                scripts.prefix().display()
            );
        }
    }
    Ok(scenes)
}

fn validate_scene_references(scenes: &mut [LoadedScene]) {
    let names = scenes
        .iter()
        .map(|scene| scene.name.clone())
        .collect::<HashSet<_>>();
    for scene in scenes {
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
}

fn load_scene(
    scripts: &ContentMount,
    path: PathBuf,
    languages: &ScriptLanguageRegistry,
) -> Result<LoadedScene> {
    let language = languages
        .language_for(&path)
        .with_context(|| format!("unsupported script format: {}", path.display()))?;
    let bytes = scripts
        .read(&path)
        .with_context(|| format!("failed to read script {}", path.display()))?;
    let source = String::from_utf8(bytes)
        .with_context(|| format!("script is not UTF-8: {}", path.display()))?;
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
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;
    use crabgal_core::{State, StepResult, step};

    fn project(root: &Path) -> ContentProject {
        ContentProject {
            root: root.to_owned(),
            sources: vec![crate::SourceMount::project("project", root.to_owned())],
        }
    }

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
        let project_root = std::env::temp_dir().join(format!("crabgal-scenes-{nonce}"));
        let root = project_root.join("scripts");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("b.txt"), "B:second;").unwrap();
        fs::write(root.join("a.txt"), "A:first;").unwrap();
        fs::write(root.join("ignored.md"), "not a scene").unwrap();

        let scenes = load_scenes(&project(&project_root)).unwrap();

        assert_eq!(
            scenes
                .iter()
                .map(|scene| scene.name.as_str())
                .collect::<Vec<_>>(),
            ["a", "b"]
        );
        let _ = fs::remove_dir_all(project_root);
    }

    #[test]
    fn loaded_scenes_execute_call_and_return_end_to_end() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let project_root = std::env::temp_dir().join(format!("crabgal-scene-flow-{nonce}"));
        let root = project_root.join("scripts");
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("main.txt"),
            "callScene:aside.txt;\nMain:returned;",
        )
        .unwrap();
        fs::write(root.join("aside.txt"), "Aside: inside;").unwrap();

        let mut state = State::new();
        state.scenes = load_scenes(&project(&project_root))
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

        let _ = fs::remove_dir_all(project_root);
    }

    #[test]
    fn later_script_sources_override_earlier_scenes() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("crabgal-scene-layers-{nonce}"));
        let base_project = root.join("base-project");
        let patch_project = root.join("patch-project");
        let base = base_project.join("scripts");
        let patch = patch_project.join("scripts");
        fs::create_dir_all(&base).unwrap();
        fs::create_dir_all(&patch).unwrap();
        fs::write(base.join("main.txt"), "Base:old;").unwrap();
        fs::write(patch.join("main.txt"), "Patch:new;").unwrap();
        let project = ContentProject {
            root: root.clone(),
            sources: vec![
                crate::SourceMount::project("project", base_project),
                crate::SourceMount::project("project", patch_project),
            ],
        };

        let scenes = load_scenes(&project).unwrap();
        assert_eq!(scenes.len(), 1);
        assert!(
            scenes[0]
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.message.contains("overrides"))
        );
        let crabgal_core::Action::Say { text, .. } = &scenes[0].actions[0] else {
            panic!("expected dialogue");
        };
        assert_eq!(text, "new");
        let _ = fs::remove_dir_all(root);
    }
}
