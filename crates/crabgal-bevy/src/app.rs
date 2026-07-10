use std::path::{Path, PathBuf};
use std::sync::Mutex;

use bevy::asset::{AssetMode, AssetPlugin};
use bevy::camera::visibility::RenderLayers;
use bevy::log::LogPlugin;
use bevy::prelude::*;
use bevy::window::WindowResolution;
use crabgal_core::config::GameConfig;
use crabgal_core::step;
use crabgal_core::{Action, State};
use crabgal_script::{ScriptWatcher, load_scenes};

use crate::plugin::GamePlugin;
use crate::render::blur::{BlurCamera, BlurPlugin, DialogCamera, SceneBlurCamera, UiBlurCamera};
use crate::resources::{GameConfigResource, GameState, ProjectRoot, ScriptWatcherResource};

pub fn run() {
    let project_root = project_root_from_args(std::env::args_os().skip(1));
    let config = GameConfig::load(&project_root.join("config.yaml"));
    let assets_path = project_root.join("assets");

    App::new()
        .add_plugins(
            DefaultPlugins
                .build()
                .set(AssetPlugin {
                    file_path: assets_path.to_string_lossy().into(),
                    processed_file_path: project_root
                        .join("imported_assets")
                        .to_string_lossy()
                        .into(),
                    mode: AssetMode::Processed,
                    ..default()
                })
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
                .set(LogPlugin {
                    filter: "info".into(),
                    ..default()
                }),
        )
        .add_plugins((GamePlugin, BlurPlugin))
        .insert_resource(ProjectRoot(project_root))
        .insert_resource(GameConfigResource(config))
        .add_systems(PreStartup, bootstrap_project)
        .run();
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

fn bootstrap_project(mut commands: Commands, project_root: Res<ProjectRoot>) {
    spawn_cameras(&mut commands);

    let script_dir = project_root.join("scripts");
    create_project_directories(&project_root, &script_dir);

    let mut state = State::new();
    match load_scenes(&script_dir) {
        Ok(scenes) => {
            for scene in scenes {
                state.scenes.insert(scene.name, scene.actions);
            }
        }
        Err(error) => log::error!("failed to load scripts: {error:#}"),
    }
    ensure_playable_scene(&mut state);
    step::index_labels(&mut state);
    step::step(&mut state);
    commands.insert_resource(GameState(state));

    match ScriptWatcher::start(&script_dir) {
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
    if state.scenes.is_empty() {
        state.scenes.insert(
            "main".into(),
            vec![
                Action::ShowBg {
                    image: "bg.webp".into(),
                    transition: Default::default(),
                },
                Action::Say {
                    speaker: "crabgal".into(),
                    text: "No script found.".into(),
                },
            ],
        );
    }

    state.current_scene = state.scenes.keys().min().cloned().unwrap_or_default();
}
