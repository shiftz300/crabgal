mod compile;
mod model;
mod studio;

pub use studio::LetsGalStudioIntegration;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::{ProjectDebugCursor, StructuredSceneLoader, structured_project};
use crate::{AdaptedProject, LoadedScene, ProjectAdapter, SourceMount};
use anyhow::{Context, Result, bail};

use model::{AssetManifest, CharactersDocument, ProjectDocument, ScenesDocument};

const PROJECT_FILE: &str = "project.json";

#[derive(Debug, Clone, Copy)]
pub struct LetsGalProjectAdapter;

impl ProjectAdapter for LetsGalProjectAdapter {
    fn name(&self) -> &'static str {
        "letsgal"
    }

    fn detect(&self, project_root: &Path) -> Result<bool> {
        if !project_root.is_dir() {
            return Ok(false);
        }
        let path = project_root.join(PROJECT_FILE);
        if !path.is_file() {
            return Ok(false);
        }
        let value: serde_json::Value = read_json(&path)?;
        Ok(value.get("chapterOrder").is_some()
            && value.get("engineVersion").is_some()
            && project_root.join("chapters").is_dir())
    }

    fn open(&self, project_root: &Path) -> Result<AdaptedProject> {
        let root = project_root.canonicalize().with_context(|| {
            format!(
                "failed to resolve LetsGal project {}",
                project_root.display()
            )
        })?;
        let project: ProjectDocument = read_json(&root.join(PROJECT_FILE))?;
        validate_project(&root, &project)?;
        let manifest: AssetManifest = read_json(&root.join("assets/.manifest.json"))?;
        let config = compile::game_config(&project, &manifest);
        let assets = root.join("assets");
        let source = SourceMount::assets("letsgal", assets.display().to_string(), assets);
        let content = structured_project(root.clone(), vec![source], Arc::new(*self));
        Ok(AdaptedProject {
            format: "letsgal",
            root,
            config,
            content,
        })
    }
}

impl StructuredSceneLoader for LetsGalProjectAdapter {
    fn name(&self) -> &'static str {
        "letsgal"
    }

    fn load(&self, project_root: &Path) -> Result<Vec<LoadedScene>> {
        let project: ProjectDocument = read_json(&project_root.join(PROJECT_FILE))?;
        let characters: CharactersDocument =
            read_json_or_default(&project_root.join("characters.json"))?;
        let scenes: ScenesDocument = read_json_or_default(&project_root.join("scenes.json"))?;
        let manifest: AssetManifest =
            read_json_or_default(&project_root.join("assets/.manifest.json"))?;
        let chapters = compile::load_chapters(project_root, &project)?;
        compile::compile_project(
            project_root,
            &project,
            &chapters,
            &characters,
            &scenes,
            &manifest,
        )
    }

    fn watch_roots(&self, project_root: &Path) -> Vec<PathBuf> {
        vec![project_root.to_owned()]
    }

    fn accepts_change(&self, path: &Path) -> bool {
        let file = path.file_name().and_then(|value| value.to_str());
        matches!(
            file,
            Some(
                "project.json"
                    | "characters.json"
                    | "scenes.json"
                    | "project.variables.json"
                    | ".manifest.json"
                    | "state.json"
            )
        ) || path
            .components()
            .any(|component| component.as_os_str() == "chapters")
            && path.extension().and_then(|value| value.to_str()) == Some("json")
    }

    fn load_config(&self, project_root: &Path) -> Result<Option<crabgal_core::config::GameConfig>> {
        let project: ProjectDocument = read_json(&project_root.join(PROJECT_FILE))?;
        let manifest: AssetManifest = read_json(&project_root.join("assets/.manifest.json"))?;
        Ok(Some(compile::game_config(&project, &manifest)))
    }

    fn is_debug_cursor_change(&self, path: &Path) -> bool {
        path.ends_with(".studio/state.json")
    }

    fn debug_cursor(&self, project_root: &Path) -> Result<Option<ProjectDebugCursor>> {
        let path = project_root.join(".studio/state.json");
        if !path.is_file() {
            return Ok(None);
        }
        let state: model::StudioState = read_json(&path)?;
        if state.active_fragment_id.is_empty() {
            return Ok(None);
        }
        let index = state
            .cursor_block_index_by_fragment
            .get(&state.active_fragment_id)
            .copied()
            .unwrap_or(state.cursor_block_index);
        Ok(Some(ProjectDebugCursor {
            scene: state.active_fragment_id,
            source_step: index.saturating_add(1),
        }))
    }
}

fn validate_project(root: &Path, project: &ProjectDocument) -> Result<()> {
    if project.id.trim().is_empty() || project.name.trim().is_empty() {
        bail!("LetsGal project id and name must not be empty");
    }
    if project.resolution.width == 0 || project.resolution.height == 0 {
        bail!("LetsGal project resolution must be non-zero");
    }
    for required in ["chapters", "assets"] {
        let path = root.join(required);
        if !path.is_dir() {
            bail!("LetsGal project directory is missing: {}", path.display());
        }
    }
    Ok(())
}

pub(super) fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    let bytes = fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_slice(&bytes).with_context(|| format!("invalid JSON in {}", path.display()))
}

fn read_json_or_default<T: serde::de::DeserializeOwned + Default>(path: &Path) -> Result<T> {
    if !path.is_file() {
        return Ok(T::default());
    }
    read_json(path)
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    fn fixture() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("crabgal-letsgal-{nonce}"));
        fs::create_dir_all(root.join("chapters")).unwrap();
        fs::create_dir_all(root.join("assets")).unwrap();
        fs::write(
            root.join(PROJECT_FILE),
            r#"{"id":"p","name":"Studio project","engineVersion":"1.0.0","chapterOrder":["Start"],"resolution":{"width":1920,"height":1080}}"#,
        )
        .unwrap();
        fs::write(
            root.join("chapters/Start.json"),
            r#"{"id":"c","name":"Start","fragments":[{"id":"f","name":"main","blocks":[{"type":"narration","content":[{"type":"text","text":"hello"}],"props":{}}]}]}"#,
        )
        .unwrap();
        fs::write(
            root.join("assets/.manifest.json"),
            r#"{"version":1,"entries":{}}"#,
        )
        .unwrap();
        root
    }

    fn file_snapshot(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
        fn visit(root: &Path, directory: &Path, files: &mut BTreeMap<PathBuf, Vec<u8>>) {
            let mut entries = fs::read_dir(directory)
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .collect::<Vec<_>>();
            entries.sort();
            for path in entries {
                if path.is_dir() {
                    visit(root, &path, files);
                } else if path.is_file() {
                    files.insert(
                        path.strip_prefix(root).unwrap().to_owned(),
                        fs::read(path).unwrap(),
                    );
                }
            }
        }

        let mut files = BTreeMap::new();
        visit(root, root, &mut files);
        files
    }

    #[test]
    fn detects_and_opens_native_studio_project() {
        let root = fixture();
        fs::create_dir_all(root.join(".studio")).unwrap();
        fs::write(
            root.join(".studio/state.json"),
            r#"{"activeFragmentId":"f","cursorBlockIndex":0,"cursorBlockIndexByFragment":{"f":2}}"#,
        )
        .unwrap();
        let adapter = LetsGalProjectAdapter;
        assert!(adapter.detect(&root).unwrap());
        let project = adapter.open(&root).unwrap();
        assert_eq!(project.format, "letsgal");
        assert_eq!(project.content.project_adapter(), Some("letsgal"));
        assert_eq!(project.config.title, "Studio project");
        fs::write(
            root.join("assets/.manifest.json"),
            r#"{"version":1,"entries":{"new":{"path":"backgrounds/new.png"}}}"#,
        )
        .unwrap();
        assert_eq!(
            project
                .content
                .reload_config()
                .unwrap()
                .unwrap()
                .bg_path("new"),
            "backgrounds/new.png"
        );
        let scenes = project.content.scene_loader().unwrap().load(&root).unwrap();
        assert!(scenes.iter().any(|scene| scene.name == "start"));
        assert!(scenes.iter().any(|scene| scene.name == "f"));
        assert!(adapter.accepts_change(&root.join(".studio/state.json")));
        assert!(adapter.is_debug_cursor_change(&root.join(".studio/state.json")));
        assert_eq!(
            project.content.debug_cursor().unwrap(),
            Some(ProjectDebugCursor {
                scene: "f".into(),
                source_step: 3,
            })
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn checked_in_163_fixture_covers_every_runtime_block() {
        use crabgal_core::Action;

        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/letsgal-1.6.3");
        let chapter: model::ChapterDocument =
            read_json(&root.join("chapters/Compatibility.json")).unwrap();
        let actual = chapter.fragments[0]
            .blocks
            .iter()
            .map(|block| block.kind.as_str())
            .collect::<BTreeSet<_>>();
        let expected = compile::BUILTIN_BLOCK_TYPES
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        assert_eq!(actual, expected);

        let adapter = LetsGalProjectAdapter;
        assert!(adapter.detect(&root).unwrap());
        let project = adapter.open(&root).unwrap();
        let scenes = project.content.scene_loader().unwrap().load(&root).unwrap();
        assert_eq!(scenes.len(), 3);
        assert!(scenes.iter().all(|scene| {
            scene
                .diagnostics
                .iter()
                .all(|diagnostic| diagnostic.level != crate::DiagnosticLevel::Error)
        }));
        assert_eq!(
            scenes
                .iter()
                .find(|scene| scene.name == "start")
                .unwrap()
                .actions,
            vec![Action::ChangeScene("fragment-compatibility".into())]
        );
        assert_eq!(
            project.content.debug_cursor().unwrap(),
            Some(ProjectDebugCursor {
                scene: "fragment-compatibility".into(),
                source_step: 9,
            })
        );
    }

    #[test]
    fn adapter_never_mutates_the_source_project() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/letsgal-1.6.3");
        let before = file_snapshot(&root);
        let adapter = LetsGalProjectAdapter;

        assert!(adapter.detect(&root).unwrap());
        let project = adapter.open(&root).unwrap();
        project.content.scene_loader().unwrap().load(&root).unwrap();
        project.content.reload_config().unwrap();
        project.content.debug_cursor().unwrap();

        assert_eq!(file_snapshot(&root), before);
    }
}
