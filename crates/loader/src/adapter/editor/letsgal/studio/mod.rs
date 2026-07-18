use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};
use serde_json::{Value, json};

use crate::EditorIntegrationAdapter;

const EXTENSION_ID: &str = "maincore.crabgal-preview";
const MANIFEST: &str = include_str!("extension.json");
const PROGRAM_TEMPLATE: &str = include_str!("dist/index.mjs");

/// LetsGal Studio host package paired with [`super::LetsGalProjectAdapter`].
///
/// The project adapter stays read-only during normal loading. Only an explicit
/// integration install command writes Studio's extension directory and project
/// opt-in flag.
pub struct LetsGalStudioIntegration;

impl EditorIntegrationAdapter for LetsGalStudioIntegration {
    fn name(&self) -> &'static str {
        "letsgal-studio"
    }

    fn install(&self, executable: &Path, project: Option<&Path>) -> Result<()> {
        if studio_is_running() {
            anyhow::bail!(
                "LetsGal Studio is running; close it before installing the extension so it cannot overwrite extension-workspaces.json on exit"
            );
        }
        if !executable.is_file() {
            anyhow::bail!("engine executable does not exist: {}", executable.display());
        }
        let extension_root = studio_user_data()?.join("extensions").join(EXTENSION_ID);
        fs::create_dir_all(extension_root.join("dist"))?;
        fs::write(extension_root.join("extension.json"), MANIFEST)?;
        fs::write(
            extension_root.join("dist/index.mjs"),
            render_program(executable, project)?,
        )?;
        let user_data = studio_user_data()?;
        register_extension_workspace(&user_data, &extension_root)?;
        verify_extension_workspace(&user_data, &extension_root)?;

        if let Some(project) = project {
            enable_for_project(project)?;
        }
        println!(
            "LetsGal SDK sync installed · {} · validated with {}",
            extension_root.display(),
            executable.display()
        );
        println!(
            "Restart LetsGal Studio, then use CRABGAL > Run CRABGAL for synchronized stepping."
        );
        Ok(())
    }

    fn uninstall(&self) -> Result<()> {
        let user_data = studio_user_data()?;
        let extension_root = user_data.join("extensions").join(EXTENSION_ID);
        unregister_extension_workspace(&user_data)?;
        if extension_root.exists() {
            fs::remove_dir_all(&extension_root)?;
        }
        println!("LetsGal bridge removed · {}", extension_root.display());
        Ok(())
    }
}

fn verify_extension_workspace(user_data: &Path, extension_root: &Path) -> Result<()> {
    let registry_path = user_data.join("extension-workspaces.json");
    let expected = fs::canonicalize(extension_root).unwrap_or_else(|_| extension_root.into());
    let registry = read_workspace_registry(&registry_path)?;
    let registered = registry["workspaces"].as_array().is_some_and(|workspaces| {
        workspaces.iter().any(|workspace| {
            workspace.get("id").and_then(Value::as_str) == Some(EXTENSION_ID)
                && workspace
                    .get("dir")
                    .and_then(Value::as_str)
                    .is_some_and(|path| Path::new(path) == expected)
        })
    });
    if !registered {
        anyhow::bail!(
            "LetsGal extension workspace registration was not persisted in {}",
            registry_path.display()
        );
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn studio_is_running() -> bool {
    Command::new("pgrep")
        .args(["-f", "/LetsGal Studio.app/Contents/MacOS/LetsGal Studio"])
        .status()
        .is_ok_and(|status| status.success())
}

#[cfg(target_os = "windows")]
fn studio_is_running() -> bool {
    Command::new("tasklist")
        .args(["/FI", "IMAGENAME eq LetsGal Studio.exe", "/NH"])
        .output()
        .is_ok_and(|output| {
            String::from_utf8_lossy(&output.stdout)
                .to_ascii_lowercase()
                .contains("letsgal studio.exe")
        })
}

#[cfg(all(unix, not(target_os = "macos")))]
fn studio_is_running() -> bool {
    Command::new("pgrep")
        .args(["-f", "letsgal-studio"])
        .status()
        .is_ok_and(|status| status.success())
}

fn render_program(executable: &Path, project: Option<&Path>) -> Result<String> {
    let executable = fs::canonicalize(executable).unwrap_or_else(|_| executable.to_path_buf());
    let project = project
        .map(|path| fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf()))
        .map(|path| path.to_string_lossy().into_owned());
    Ok(PROGRAM_TEMPLATE
        .replace(
            "__CRABGAL_ENGINE_PATH__",
            &serde_json::to_string(&executable.to_string_lossy())?,
        )
        .replace(
            "__CRABGAL_PROJECT_PATH__",
            &serde_json::to_string(&project)?,
        ))
}

fn register_extension_workspace(user_data: &Path, extension_root: &Path) -> Result<()> {
    let registry_path = user_data.join("extension-workspaces.json");
    let canonical_root = fs::canonicalize(extension_root).unwrap_or_else(|_| extension_root.into());
    let mut registry = read_workspace_registry(&registry_path)?;
    let workspaces = registry
        .get_mut("workspaces")
        .and_then(Value::as_array_mut)
        .context("extension-workspaces.json has no workspaces array")?;
    if let Some(workspace) = workspaces
        .iter_mut()
        .find(|workspace| workspace.get("id").and_then(Value::as_str) == Some(EXTENSION_ID))
    {
        workspace["dir"] = json!(canonical_root);
    } else {
        workspaces.push(json!({ "id": EXTENSION_ID, "dir": canonical_root }));
    }
    write_json_atomically(&registry_path, &registry)
}

fn unregister_extension_workspace(user_data: &Path) -> Result<()> {
    let registry_path = user_data.join("extension-workspaces.json");
    if !registry_path.exists() {
        return Ok(());
    }
    let mut registry = read_workspace_registry(&registry_path)?;
    let workspaces = registry
        .get_mut("workspaces")
        .and_then(Value::as_array_mut)
        .context("extension-workspaces.json has no workspaces array")?;
    workspaces
        .retain(|workspace| workspace.get("id").and_then(Value::as_str) != Some(EXTENSION_ID));
    write_json_atomically(&registry_path, &registry)
}

fn read_workspace_registry(path: &Path) -> Result<Value> {
    if !path.exists() {
        return Ok(json!({ "version": 1, "workspaces": [] }));
    }
    let source =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let registry: Value = serde_json::from_str(&source)
        .with_context(|| format!("invalid workspace registry {}", path.display()))?;
    if !registry.get("workspaces").is_some_and(Value::is_array) {
        anyhow::bail!("{} has no workspaces array", path.display());
    }
    Ok(registry)
}

fn write_json_atomically(path: &Path, value: &Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension("json.crabgal.tmp");
    fs::write(
        &temporary,
        format!("{}\n", serde_json::to_string_pretty(value)?),
    )?;
    fs::rename(&temporary, path)?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn studio_user_data() -> Result<PathBuf> {
    Ok(home_dir()?.join("Library/Application Support/letsgal-studio"))
}

#[cfg(target_os = "windows")]
fn studio_user_data() -> Result<PathBuf> {
    let root = std::env::var_os("APPDATA").context("APPDATA is not set")?;
    Ok(PathBuf::from(root).join("letsgal-studio"))
}

#[cfg(all(unix, not(target_os = "macos")))]
fn studio_user_data() -> Result<PathBuf> {
    let root = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or(home_dir()?.join(".config"));
    Ok(root.join("letsgal-studio"))
}

fn home_dir() -> Result<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .context("HOME is not set")
}

fn enable_for_project(project: &Path) -> Result<()> {
    let project_file = project.join("project.json");
    let source = fs::read_to_string(&project_file)
        .with_context(|| format!("failed to read {}", project_file.display()))?;
    let key = format!("\"{EXTENSION_ID}\"");
    if let Some(extension_offset) = source.find(&key) {
        let tail = &source[extension_offset + key.len()..];
        let object_end = tail
            .find('}')
            .context("invalid existing LetsGal extension entry")?;
        let object = &tail[..object_end];
        let enabled_offset = object
            .find("\"enabled\"")
            .context("existing LetsGal bridge entry has no enabled field")?;
        let value_offset = object[enabled_offset..]
            .find(':')
            .map(|offset| extension_offset + key.len() + enabled_offset + offset + 1)
            .context("invalid LetsGal extension enabled field")?;
        let value_start = source[value_offset..]
            .find(|character: char| !character.is_whitespace())
            .map(|offset| value_offset + offset)
            .context("missing LetsGal extension enabled value")?;
        if source[value_start..].starts_with("false") {
            let mut updated = source;
            updated.replace_range(value_start..value_start + 5, "true");
            write_project_atomically(&project_file, updated)?;
            println!("LetsGal bridge enabled · {}", project_file.display());
        } else {
            println!(
                "LetsGal bridge already enabled in {}",
                project_file.display()
            );
        }
        return Ok(());
    }

    let marker = "\"extensions\": {";
    let Some(offset) = source.find(marker) else {
        anyhow::bail!("{} has no extensions object", project_file.display());
    };
    let insertion =
        format!("{marker}\n    \"{EXTENSION_ID}\": {{\n      \"enabled\": true\n    }},");
    let mut updated = source;
    updated.replace_range(offset..offset + marker.len(), &insertion);
    write_project_atomically(&project_file, updated)?;
    println!("LetsGal bridge enabled · {}", project_file.display());
    Ok(())
}

fn write_project_atomically(project_file: &Path, updated: String) -> Result<()> {
    let temporary = project_file.with_extension("json.crabgal.tmp");
    fs::write(&temporary, updated)?;
    fs::rename(&temporary, project_file)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temporary_project(name: &str) -> PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("crabgal-{name}-{}-{nonce}", std::process::id()))
    }

    #[test]
    fn installer_enables_an_existing_disabled_bridge_without_reformatting_project() {
        let root = temporary_project("studio-bridge-enable");
        fs::create_dir_all(&root).unwrap();
        fs::write(
            root.join("project.json"),
            r#"{
  "name": "fixture",
  "extensions": {
    "maincore.crabgal-preview": { "enabled": false },
    "keep.formatting": { "enabled": true }
  }
}"#,
        )
        .unwrap();

        enable_for_project(&root).unwrap();
        let updated = fs::read_to_string(root.join("project.json")).unwrap();
        assert!(updated.contains(r#""maincore.crabgal-preview": { "enabled": true }"#));
        assert!(updated.contains(r#""keep.formatting": { "enabled": true }"#));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn bundled_extension_keeps_the_editor_hook_narrow() {
        let manifest: Value = serde_json::from_str(MANIFEST).unwrap();
        assert_eq!(manifest["version"], "0.9.0");
        assert_eq!(manifest["minHostVersion"], "1.7.0");
        assert_eq!(manifest["riskTier"], "privileged");
        assert_eq!(
            manifest["permissions"],
            json!(["local.network", "process.spawn", "filesystem.read"])
        );
        for private_surface in [
            "getHost(",
            "data-crabgal-run-control",
            "studioRunControls",
            "routeRunButton",
            "studio:playtest-hotkey",
        ] {
            assert!(
                !PROGRAM_TEMPLATE.contains(private_surface),
                "extension depends on private host surface {private_surface:?}"
            );
        }
        for public_surface in [
            "ctx.settings",
            "ctx.subscribe",
            "fragment:entered",
            "\"heartbeat\"",
            "\"restart\"",
            "__CRABGAL_ENGINE_PATH__",
            "__CRABGAL_PROJECT_PATH__",
        ] {
            assert!(PROGRAM_TEMPLATE.contains(public_surface));
        }
    }

    #[test]
    fn installer_renders_native_paths_without_leaking_placeholders() {
        let rendered = render_program(
            Path::new("/tmp/crabgal engine"),
            Some(Path::new("/tmp/LetsGal project")),
        )
        .unwrap();
        assert!(!rendered.contains("__CRABGAL_ENGINE_PATH__"));
        assert!(!rendered.contains("__CRABGAL_PROJECT_PATH__"));
        assert!(rendered.contains(r#"const ENGINE_PATH = "/tmp/crabgal engine";"#));
        assert!(rendered.contains(r#"const PROJECT_PATH = "/tmp/LetsGal project";"#));
    }

    #[test]
    fn workspace_registration_is_idempotent_and_preserves_other_extensions() {
        let user_data = temporary_project("studio-workspace-registry");
        let extension_root = user_data.join("extensions").join(EXTENSION_ID);
        fs::create_dir_all(&extension_root).unwrap();
        fs::write(
            user_data.join("extension-workspaces.json"),
            r#"{
  "version": 1,
  "workspaces": [
    { "id": "keep.extension", "dir": "/tmp/keep", "custom": true }
  ]
}"#,
        )
        .unwrap();

        register_extension_workspace(&user_data, &extension_root).unwrap();
        register_extension_workspace(&user_data, &extension_root).unwrap();

        let registry: Value = serde_json::from_str(
            &fs::read_to_string(user_data.join("extension-workspaces.json")).unwrap(),
        )
        .unwrap();
        let workspaces = registry["workspaces"].as_array().unwrap();
        assert_eq!(workspaces.len(), 2);
        assert_eq!(workspaces[0]["id"], "keep.extension");
        assert_eq!(workspaces[0]["custom"], true);
        let crabgal = workspaces
            .iter()
            .find(|workspace| workspace["id"] == EXTENSION_ID)
            .unwrap();
        assert_eq!(
            crabgal["dir"],
            extension_root
                .canonicalize()
                .unwrap()
                .to_string_lossy()
                .as_ref()
        );

        unregister_extension_workspace(&user_data).unwrap();
        let registry: Value = serde_json::from_str(
            &fs::read_to_string(user_data.join("extension-workspaces.json")).unwrap(),
        )
        .unwrap();
        let workspaces = registry["workspaces"].as_array().unwrap();
        assert_eq!(workspaces.len(), 1);
        assert_eq!(workspaces[0]["id"], "keep.extension");
        fs::remove_dir_all(user_data).unwrap();
    }
}
