use std::path::{Path, PathBuf};
use std::sync::Mutex;

use anyhow::{Context, Result};
use bevy::asset::io::AssetSourceId;
use bevy::asset::{AssetApp, AssetPlugin};
use bevy::camera::visibility::RenderLayers;
use bevy::diagnostic::{EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::window::WindowResolution;
use crabgal_core::config::GameConfig;
use crabgal_core::{Action, Program, State};
use crabgal_loader::{
    ContentProject, DiagnosticLevel, LoaderRegistry, ScriptWatcher, load_hexz_project_from_archive,
    load_project_with, load_scenes_with,
};

use crate::render::blur::{BlurCamera, BlurPlugin, DialogCamera, SceneBlurCamera, UiBlurCamera};
use crate::runtime::GamePlugin;
use crate::runtime::resources::{
    ContentProjectResource, GameConfigResource, GameState, LocalAssetCache, LocalAssetManifest,
    LocalSceneAssets, ProjectRoot, ScriptLanguages, ScriptWatcherResource, StoreCodec,
};

pub fn run() {
    run_with_loader(LoaderRegistry::default());
}

pub fn run_with_loader(loader: LoaderRegistry) {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    let project_path = project_root_from_args(args.into_iter());
    let (project_root, config, content) = match open_project(&project_path, &loader) {
        Ok(project) => project,
        Err(error) => {
            super::logging::startup_error("failed to open project", &error);
            return;
        }
    };
    let languages = match loader.languages(&config.adapter.script) {
        Ok(languages) => languages,
        Err(error) => {
            super::logging::startup_error("failed to select script adapter", &error);
            return;
        }
    };
    let store = match loader.store(&config.adapter.store) {
        Ok(store) => store,
        Err(error) => {
            super::logging::startup_error("failed to select store adapter", &error);
            return;
        }
    };
    let webp = crate::scene::images::NativeWebpPlugin::new(config.layout.sprite_height);

    let mut app = App::new();
    app.register_asset_source(
        AssetSourceId::Default,
        crate::runtime::asset_reader::overlay_source(content.asset_mounts()),
    );
    app.add_plugins(
        DefaultPlugins
            .build()
            .set(AssetPlugin::default())
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: config.title.clone(),
                    resolution: WindowResolution::new(1280, 720),
                    resizable: true,
                    ..default()
                }),
                ..default()
            })
            .set(ImagePlugin::default())
            .set(super::logging::plugin()),
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
    .add_systems(PreStartup, bootstrap_project);
    super::logging::install_runtime_diagnostics(&mut app);
    app.run();
}

fn open_project(
    project_path: &Path,
    loader: &LoaderRegistry,
) -> Result<(PathBuf, GameConfig, ContentProject)> {
    if project_path.extension().and_then(|value| value.to_str()) == Some("hxz") {
        let archive = crabgal_loader::mount_hexz(project_path)?;
        let yaml = archive.read(Path::new("config.yaml"))?;
        let yaml = std::str::from_utf8(&yaml).context("Hexz config.yaml is not UTF-8")?;
        let config = GameConfig::from_yaml(yaml).context("invalid Hexz config.yaml")?;
        let content = load_hexz_project_from_archive(archive, &config.adapter.asset)?;
        let writable_root = project_path
            .canonicalize()
            .unwrap_or_else(|_| project_path.to_owned())
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_owned();
        return Ok((writable_root, config, content));
    }

    let script_dir = project_path.join("scripts");
    create_project_directories(project_path, &script_dir);
    let config = GameConfig::load(&project_path.join("config.yaml"));
    let content = load_project_with(project_path, &config.adapter.asset, loader)?;
    Ok((content.root.clone(), config, content))
}

fn project_root_from_args(args: impl Iterator<Item = std::ffi::OsString>) -> PathBuf {
    let args = args.collect::<Vec<_>>();
    let relative = match args.as_slice() {
        [command, path, ..] if command == "dev" => PathBuf::from(path),
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

fn bootstrap_project(
    mut commands: Commands,
    project_root: Res<ProjectRoot>,
    content: Res<ContentProjectResource>,
    languages: Res<ScriptLanguages>,
    config: Res<GameConfigResource>,
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
                    },
                );
                program_scenes.push((scene.name, scene.actions));
            }
            state.install_program(Program::from_scenes(program_scenes));
        }
        Err(error) => log::error!("failed to load scripts: {error:#}"),
    }
    ensure_playable_scene(&mut state);
    // Loading scripts prepares the entry scene, but execution belongs to the
    // title screen's START action. This also lets title assets and the first
    // scene warm up without briefly exposing gameplay UI during startup.
    state.ended = true;
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

    match ScriptWatcher::start_with_languages(&content.watched_script_roots(), languages.0.clone())
    {
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

fn create_project_directories(project_root: &Path, script_dir: &Path) {
    for path in [script_dir.to_path_buf(), project_root.join("assets/fonts")] {
        if let Err(error) = std::fs::create_dir_all(&path) {
            log::error!("failed to create {}: {error}", path.display());
        }
    }
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
