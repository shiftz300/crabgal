use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use anyhow::{Context, Result};
use bevy::asset::io::AssetSourceId;
use bevy::asset::{AssetApp, AssetPlugin, RenderAssetUsages};
use bevy::camera::visibility::RenderLayers;
use bevy::diagnostic::{EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin};
use bevy::ecs::system::NonSendMarker;
use bevy::image::{CompressedImageFormats, ImageSampler, ImageType};
use bevy::prelude::*;
use bevy::window::{CursorOptions, PrimaryWindow, WindowLevel, WindowPosition, WindowResolution};
use bevy::winit::WINIT_WINDOWS;
use crabgal_core::config::GameConfig;
use crabgal_core::{Action, DESIGN_HEIGHT, DESIGN_WIDTH, Program, State};
use crabgal_loader::{
    ContentProject, DiagnosticLevel, LoaderRegistry, ScriptWatcher, load_project_with,
    load_scenes_with,
};

use crate::render::blur::{BlurCamera, BlurPlugin, DialogCamera, SceneBlurCamera, UiBlurCamera};
use crate::runtime::GamePlugin;
use crate::runtime::resources::{
    ContentProjectResource, GameConfigResource, GameState, LocalAssetCache, LocalAssetManifest,
    LocalSceneAssets, ProjectRoot, ScriptLanguages, ScriptWatcherResource, StoreCodec,
};

const DEFAULT_STUDIO_SYNC_PORT: u16 = 39_698;

pub fn run() {
    run_with_loader(LoaderRegistry::default());
}

pub fn run_cli() -> std::process::ExitCode {
    match try_run_with_loader(LoaderRegistry::default()) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            super::platform::startup_error("failed to open project", &error);
            std::process::ExitCode::FAILURE
        }
    }
}

pub fn run_with_loader(loader: LoaderRegistry) {
    if let Err(error) = try_run_with_loader(loader) {
        super::platform::startup_error("failed to open project", &error);
    }
}

fn try_run_with_loader(loader: LoaderRegistry) -> Result<()> {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    if run_editor_integration_command(&loader, &args)? {
        return Ok(());
    }
    let check_only = args.first().is_some_and(|command| command == "check");
    let editor_port = editor_bridge_port(&args)?;
    let editor_embedded = editor_port.is_some() && !editor_standalone_window(&args);
    let project_path = project_root_from_args(args.iter().cloned());
    let (project_root, config, content) = open_project(&project_path, &loader)?;
    let languages = loader
        .languages(&config.adapter.script)
        .context("failed to select script adapter")?;
    if check_only {
        return check_project(&config, &content, &languages);
    }
    let store = loader
        .store(&config.adapter.store)
        .context("failed to select store adapter")?;
    let mut app = build_opened_app(
        project_root,
        config,
        content,
        languages,
        store,
        editor_port,
        editor_embedded,
    );
    app.run();
    Ok(())
}

/// Builds a customizable Bevy application for one project without running it.
/// Extension plugins can claim and consume [`crate::HostCommandMessage`] before
/// calling `App::run`, while built-in adapter semantics stay on typed actions.
pub fn build_app_with_loader(
    project_path: impl AsRef<Path>,
    loader: LoaderRegistry,
) -> Result<App> {
    let (project_root, config, content) = open_project(project_path.as_ref(), &loader)?;
    let languages = loader
        .languages(&config.adapter.script)
        .context("failed to select script adapter")?;
    let store = loader
        .store(&config.adapter.store)
        .context("failed to select store adapter")?;
    Ok(build_opened_app(
        project_root,
        config,
        content,
        languages,
        store,
        None,
        false,
    ))
}

fn build_opened_app(
    project_root: PathBuf,
    config: GameConfig,
    content: ContentProject,
    languages: crabgal_loader::ScriptLanguageRegistry,
    store: std::sync::Arc<dyn crabgal_loader::StoreAdapter>,
    editor_port: Option<u16>,
    editor_overlay: bool,
) -> App {
    let webp = crate::scene::images::NativeWebpPlugin::new(config.layout.sprite_height);
    let asset_mounts = content.asset_mounts();
    let watch_assets = asset_mounts
        .iter()
        .any(|mount| mount.filesystem_root().is_some());

    let mut app = App::new();
    app.register_asset_source(
        AssetSourceId::Default,
        crate::runtime::asset_reader::overlay_source(asset_mounts),
    );
    let initial_editor_frame = editor_overlay
        .then(super::editor_bridge::initial_editor_frame)
        .flatten();
    let mut initial_resolution = WindowResolution::new(DESIGN_WIDTH as u32, DESIGN_HEIGHT as u32);
    if let Some(frame) = initial_editor_frame {
        // The first winit window is created before the backend reports its
        // monitor scale. Seed that scale here so a physical editor rectangle is
        // not interpreted as a logical size and doubled on Retina/HiDPI hosts.
        initial_resolution.set_scale_factor_override(Some(frame.scale_factor));
        initial_resolution.set_physical_resolution(frame.size.x, frame.size.y);
    } else {
        // Keep the native runtime on the engine's 1920x1080 design grid even
        // on Retina/HiDPI monitors. Without an override, winit can reinterpret
        // the requested physical size as logical pixels after discovering the
        // monitor scale, producing an oversized and clipped preview window.
        initial_resolution.set_scale_factor_override(Some(1.0));
    }
    app.add_plugins(
        DefaultPlugins
            .build()
            .set(AssetPlugin {
                watch_for_changes_override: Some(watch_assets),
                ..default()
            })
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: config.title.clone(),
                    resolution: initial_resolution,
                    position: if editor_overlay {
                        initial_editor_frame
                            .map_or(WindowPosition::At(IVec2::splat(-32_000)), |frame| {
                                WindowPosition::At(frame.position)
                            })
                    } else {
                        WindowPosition::default()
                    },
                    resizable: !editor_overlay,
                    decorations: !editor_overlay,
                    visible: !editor_overlay,
                    focused: !editor_overlay,
                    skip_taskbar: editor_overlay,
                    window_level: if editor_overlay {
                        WindowLevel::AlwaysOnTop
                    } else {
                        WindowLevel::Normal
                    },
                    ..default()
                }),
                // The editor host owns input. The native preview is a visual
                // surface, so pointer events must reach the editor canvas
                // below the borderless Bevy window.
                primary_cursor_options: Some(CursorOptions {
                    hit_test: !editor_overlay,
                    ..default()
                }),
                ..default()
            })
            .set(ImagePlugin::default())
            .set(super::platform::log_plugin()),
    )
    .add_plugins((
        webp,
        GamePlugin,
        BlurPlugin,
        FrameTimeDiagnosticsPlugin::default(),
        EntityCountDiagnosticsPlugin::default(),
    ))
    .insert_resource(ProjectRoot(project_root))
    .insert_resource(ContentProjectResource(content))
    .insert_resource(ScriptLanguages(languages))
    .insert_resource(StoreCodec(store))
    .insert_resource(GameConfigResource(config))
    .add_systems(PreStartup, bootstrap_project)
    .add_systems(PostStartup, set_primary_window_icon);
    if let Some(port) = editor_port {
        app.add_plugins(super::editor_bridge::EditorBridgePlugin::new(
            port,
            editor_overlay,
        ));
    }
    super::platform::install_runtime_diagnostics(&mut app);
    app
}

fn set_primary_window_icon(
    window: Query<Entity, With<PrimaryWindow>>,
    _main_thread: NonSendMarker,
) {
    #[cfg(target_os = "macos")]
    if let Err(error) = set_macos_application_icon() {
        log::warn!("failed to set macOS application icon: {error:#}");
    }

    let Ok(window_entity) = window.single() else {
        return;
    };
    let icon = match load_window_icon() {
        Ok(icon) => icon,
        Err(error) => {
            log::warn!("failed to load application icon: {error:#}");
            return;
        }
    };

    WINIT_WINDOWS.with_borrow(|windows| {
        if let Some(window) = windows.get_window(window_entity) {
            window.set_window_icon(Some(icon));
        }
    });
}

#[cfg(target_os = "macos")]
fn set_macos_application_icon() -> Result<()> {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSApplication, NSImage};
    use objc2_foundation::NSData;

    let main_thread =
        MainThreadMarker::new().context("application icon must be set on main thread")?;
    let bytes = include_bytes!("../../assets/icons/crabgal-256.png");
    // SAFETY: `NSData` copies exactly `bytes.len()` readable bytes from this
    // process-owned static buffer before returning.
    let data = unsafe { NSData::dataWithBytes_length(bytes.as_ptr().cast(), bytes.len()) };
    let image = NSImage::initWithData(main_thread.alloc(), &data)
        .context("AppKit rejected the embedded PNG application icon")?;
    let application = NSApplication::sharedApplication(main_thread);
    // SAFETY: This setter is called on AppKit's main thread and retains the
    // supplied NSImage for the application's Dock lifetime.
    unsafe { application.setApplicationIconImage(Some(&image)) };
    Ok(())
}

fn load_window_icon() -> Result<winit::window::Icon> {
    let (rgba, width, height) = decode_window_icon()?;
    winit::window::Icon::from_rgba(rgba, width, height)
        .context("embedded application icon has invalid RGBA data")
}

fn decode_window_icon() -> Result<(Vec<u8>, u32, u32)> {
    let image = Image::from_buffer(
        include_bytes!("../../assets/icons/crabgal-256.png"),
        ImageType::Extension("png"),
        CompressedImageFormats::NONE,
        true,
        ImageSampler::default(),
        RenderAssetUsages::MAIN_WORLD,
    )
    .context("failed to decode embedded application icon")?;
    let width = image.texture_descriptor.size.width;
    let height = image.texture_descriptor.size.height;
    let rgba = image
        .data
        .context("embedded application icon has no CPU pixel data")?;
    Ok((rgba, width, height))
}

fn check_project(
    config: &GameConfig,
    content: &ContentProject,
    languages: &crabgal_loader::ScriptLanguageRegistry,
) -> Result<()> {
    let scenes =
        load_scenes_with(content, languages).context("failed to compile project scenes")?;
    let mut actions = 0usize;
    let mut warnings = 0usize;
    let mut errors = 0usize;
    let mut missing_resources = HashSet::new();
    for scene in &scenes {
        actions += scene.actions.len();
        for diagnostic in &scene.diagnostics {
            let level = match diagnostic.level {
                DiagnosticLevel::Warning => {
                    warnings += 1;
                    "warning"
                }
                DiagnosticLevel::Error => {
                    errors += 1;
                    "error"
                }
            };
            eprintln!(
                "{level}: {}:{}:{}: {}",
                scene.path.display(),
                diagnostic.span.line,
                diagnostic.span.column,
                diagnostic.message
            );
        }
        for resource in &scene.resources {
            let path = resource.resolved_path(config);
            if path.contains('{') || !missing_resources.insert(path.clone()) {
                continue;
            }
            if !content.contains_asset(Path::new(&path)) {
                errors += 1;
                eprintln!(
                    "error: {}:{}:{}: resource does not exist: {path}",
                    scene.path.display(),
                    resource.span.line,
                    resource.span.column,
                );
            }
        }
    }
    if errors > 0 {
        anyhow::bail!("project check failed with {errors} error(s) and {warnings} warning(s)");
    }
    println!(
        "project valid · {} · {} scene(s) · {actions} action(s) · {} source(s) · {warnings} warning(s)",
        config.title,
        scenes.len(),
        content.sources.len(),
    );
    Ok(())
}

fn open_project(
    project_path: &Path,
    loader: &LoaderRegistry,
) -> Result<(PathBuf, GameConfig, ContentProject)> {
    if let Some(project) = loader.open_project(project_path)? {
        return Ok((project.root, project.config, project.content));
    }

    ensure_project_directory(project_path)?;
    let config_path = project_path.join("config.yaml");
    let yaml = std::fs::read_to_string(&config_path)
        .with_context(|| format!("failed to read {}", config_path.display()))?;
    let config = GameConfig::from_yaml(&yaml)
        .with_context(|| format!("invalid project config {}", config_path.display()))?;
    let content = load_project_with(project_path, &config.adapter.asset, loader)?;
    Ok((content.root.clone(), config, content))
}

fn ensure_project_directory(project_path: &Path) -> Result<()> {
    if !project_path.is_dir() {
        anyhow::bail!(
            "project directory does not exist: {}",
            project_path.display()
        );
    }
    let config_path = project_path.join("config.yaml");
    if !config_path.is_file() {
        anyhow::bail!("project config does not exist: {}", config_path.display());
    }
    Ok(())
}

fn project_root_from_args(args: impl Iterator<Item = std::ffi::OsString>) -> PathBuf {
    let args = args.collect::<Vec<_>>();
    let relative = match args.as_slice() {
        [command, path, ..]
            if command == "dev"
                || command == "check"
                || command == "editor"
                || command == "studio" =>
        {
            PathBuf::from(path)
        }
        [path, ..] => PathBuf::from(path),
        [] => PathBuf::new(),
    };

    std::env::current_dir()
        .unwrap_or_else(|error| {
            log::warn!("failed to read current directory: {error}");
            PathBuf::from(".")
        })
        .join(relative)
}

fn editor_bridge_port(args: &[std::ffi::OsString]) -> Result<Option<u16>> {
    if args
        .first()
        .is_none_or(|command| command != "editor" && command != "studio")
    {
        return Ok(None);
    }
    let Some(index) = args.iter().position(|arg| arg == "--bridge-port") else {
        return if args.first().is_some_and(|command| command == "studio") {
            Ok(Some(DEFAULT_STUDIO_SYNC_PORT))
        } else {
            anyhow::bail!("editor mode requires --bridge-port <port>")
        };
    };
    let raw = args
        .get(index + 1)
        .context("--bridge-port requires a port number")?;
    let port = raw
        .to_string_lossy()
        .parse::<u16>()
        .context("invalid --bridge-port value")?;
    Ok(Some(port))
}

fn editor_standalone_window(args: &[std::ffi::OsString]) -> bool {
    args.first().is_some_and(|command| command == "studio")
        || args.iter().any(|arg| arg == "--standalone-window")
}

fn run_editor_integration_command(
    loader: &LoaderRegistry,
    args: &[std::ffi::OsString],
) -> Result<bool> {
    let Some(command) = args.first().and_then(|value| value.to_str()) else {
        return Ok(false);
    };
    match command {
        "integration-install" => {
            let name = args
                .get(1)
                .context("integration-install requires an adapter name")?
                .to_string_lossy();
            let executable = std::env::current_exe().context("failed to locate crabgal binary")?;
            let project = args.get(2).map(PathBuf::from);
            loader.install_editor_integration(&name, &executable, project.as_deref())?;
            Ok(true)
        }
        "integration-uninstall" => {
            let name = args
                .get(1)
                .context("integration-uninstall requires an adapter name")?
                .to_string_lossy();
            loader.uninstall_editor_integration(&name)?;
            Ok(true)
        }
        "integration-control" => {
            let name = args
                .get(1)
                .context("integration-control requires an adapter name")?
                .to_string_lossy();
            let control_args = args[2..]
                .iter()
                .map(|value| value.to_string_lossy().into_owned())
                .collect::<Vec<_>>();
            loader.control_editor_integration(&name, &control_args)?;
            Ok(true)
        }
        _ => Ok(false),
    }
}

fn bootstrap_project(
    mut commands: Commands,
    project_root: Res<ProjectRoot>,
    content: Res<ContentProjectResource>,
    languages: Res<ScriptLanguages>,
    config: Res<GameConfigResource>,
    editor_overlay: Option<Res<super::editor_bridge::EditorOverlay>>,
) {
    spawn_cameras(&mut commands);

    let mut state = State::new();
    state.global_vars = crate::storage::profile::load(&project_root);
    crate::storage::gallery::load(&mut state, &project_root);
    state.read_dialogues = crate::storage::read_history::load(&project_root);
    let read_history_count = state.read_dialogues.len();
    let mut scene_count = 0;
    let mut action_count = 0;
    let mut manifest = LocalAssetManifest::default();
    match load_scenes_with(&content, &languages) {
        Ok(scenes) => {
            let mut program_scenes = Vec::with_capacity(scenes.len());
            for scene in scenes {
                scene_count += 1;
                action_count += scene.actions.len();
                for diagnostic in &scene.diagnostics {
                    let message = format!(
                        "{}:{}:{}: {}",
                        scene.path.display(),
                        diagnostic.span.line,
                        diagnostic.span.column,
                        diagnostic.message
                    );
                    match diagnostic.level {
                        DiagnosticLevel::Warning => log::warn!("{message}"),
                        DiagnosticLevel::Error => log::error!("{message}"),
                    }
                }
                manifest.insert(
                    scene.name.clone(),
                    LocalSceneAssets {
                        resources: scene.resources,
                        sub_scenes: scene.sub_scenes,
                        action_spans: scene.action_spans,
                    },
                );
                program_scenes.push((scene.name, scene.actions));
            }
            state.install_program(Program::from_scenes(program_scenes));
        }
        Err(error) => log::error!("failed to load scripts: {error:#}"),
    }
    ensure_playable_scene(&mut state);
    if editor_overlay.is_some() {
        // An editor is already the outer shell. Enter its current cursor directly
        // so the native overlay never flashes crabgal's title screen first.
        state.ended = false;
        if !crate::runtime::tick::sync_editor_cursor(&content, &mut state, &manifest) {
            crabgal_core::step::step(&mut state);
        }
    } else {
        // Normal binaries prepare the entry scene, but execution belongs to
        // the title screen's START action.
        state.ended = true;
    }
    log::info!(
        "project ready · {} · {scene_count} scene(s) · {action_count} action(s) · {} source(s)",
        config.title,
        content.sources.len(),
    );
    let profile_writer = crate::storage::profile::ProfileWriter::loaded(&state.global_vars);
    commands.insert_resource(GameState(state));
    commands.insert_resource(crate::storage::read_history::ReadHistoryWriter::loaded(
        read_history_count,
    ));
    commands.insert_resource(profile_writer);
    commands.insert_resource(manifest);
    commands.insert_resource(LocalAssetCache::default());

    match ScriptWatcher::start_for_project(&content, languages.0.clone()) {
        Ok(watcher) => {
            commands.insert_resource(ScriptWatcherResource(Mutex::new(watcher)));
        }
        Err(error) => log::warn!("script hot reload disabled: {error:#}"),
    }
}

fn spawn_cameras(commands: &mut Commands) {
    commands.spawn((
        Name::new("scene_camera"),
        Camera2d,
        Camera {
            order: 0,
            ..default()
        },
        RenderLayers::layer(0),
        BlurCamera::default(),
        SceneBlurCamera,
    ));
    commands.spawn((
        Name::new("ui_camera"),
        Camera2d,
        Camera {
            order: 1,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        RenderLayers::layer(1),
        BlurCamera::default(),
        UiBlurCamera,
    ));
    commands.spawn((
        Name::new("dialog_camera"),
        Camera2d,
        Camera {
            order: 2,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        RenderLayers::layer(2),
        DialogCamera,
    ));
}

fn ensure_playable_scene(state: &mut State) {
    if state.program.is_empty() {
        state.insert_scene(
            "main".into(),
            vec![
                Action::ShowBg {
                    image: "bg.webp".into(),
                    transition: Default::default(),
                    transform: Default::default(),
                },
                Action::Say {
                    speaker: "crabgal".into(),
                    text: "No script found.".into(),
                    options: Default::default(),
                },
            ],
        );
    }

    state.current_scene = crate::scene::entry_scene(state);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_temp_path(name: &str) -> PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock should be after the Unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("crabgal-{name}-{}-{nonce}", std::process::id()))
    }

    #[test]
    fn missing_project_is_rejected_without_creating_it() {
        let path = unique_temp_path("missing-project");
        assert!(!path.exists());

        let error = open_project(&path, &LoaderRegistry::default()).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("project directory does not exist")
        );
        assert!(!path.exists());
    }

    #[test]
    fn project_without_config_is_rejected_without_scaffolding() {
        let path = unique_temp_path("missing-config");
        std::fs::create_dir_all(&path).unwrap();

        let error = open_project(&path, &LoaderRegistry::default()).unwrap_err();

        assert!(error.to_string().contains("project config does not exist"));
        assert!(!path.join("scripts").exists());
        assert!(!path.join("assets").exists());
        std::fs::remove_dir(&path).unwrap();
    }

    #[test]
    fn check_command_uses_the_explicit_project_path() {
        let path = project_root_from_args(
            ["check", "/tmp/editor-project"]
                .into_iter()
                .map(std::ffi::OsString::from),
        );
        assert_eq!(path, Path::new("/tmp/editor-project"));
    }

    #[test]
    fn editor_command_uses_the_explicit_project_and_port() {
        let args = ["editor", "/tmp/editor-project", "--bridge-port", "39412"]
            .into_iter()
            .map(std::ffi::OsString::from)
            .collect::<Vec<_>>();
        assert_eq!(
            project_root_from_args(args.iter().cloned()),
            Path::new("/tmp/editor-project")
        );
        assert_eq!(editor_bridge_port(&args).unwrap(), Some(39412));
        assert!(!editor_standalone_window(&args));
    }

    #[test]
    fn editor_can_use_a_normal_standalone_window() {
        let args = [
            "editor",
            "/tmp/editor-project",
            "--bridge-port",
            "39412",
            "--standalone-window",
        ]
        .into_iter()
        .map(std::ffi::OsString::from)
        .collect::<Vec<_>>();
        assert_eq!(editor_bridge_port(&args).unwrap(), Some(39412));
        assert!(editor_standalone_window(&args));
    }

    #[test]
    fn normal_project_commands_do_not_enable_the_editor_bridge() {
        let args = ["dev", "/tmp/project"]
            .into_iter()
            .map(std::ffi::OsString::from)
            .collect::<Vec<_>>();
        assert_eq!(editor_bridge_port(&args).unwrap(), None);
    }

    #[test]
    fn studio_command_uses_the_sdk_sync_defaults() {
        let args = ["studio", "/tmp/editor-project"]
            .into_iter()
            .map(std::ffi::OsString::from)
            .collect::<Vec<_>>();
        assert_eq!(
            project_root_from_args(args.iter().cloned()),
            Path::new("/tmp/editor-project")
        );
        assert_eq!(
            editor_bridge_port(&args).unwrap(),
            Some(DEFAULT_STUDIO_SYNC_PORT)
        );
        assert!(editor_standalone_window(&args));
    }

    #[test]
    fn embedded_window_icon_is_valid_rgba() {
        let (rgba, width, height) = decode_window_icon().unwrap();
        assert_eq!((width, height), (256, 256));
        assert_eq!(rgba.len(), width as usize * height as usize * 4);
    }
}
