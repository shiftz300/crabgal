// crabgal-bevy — Bevy VN engine
mod components;
mod game;
mod locale;
mod plugin;
mod render;
mod resources;
mod save;
mod scene;
mod ui;

use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use bevy::prelude::*;
use bevy::asset::AssetPlugin;
use bevy::asset::AssetMode;
use bevy::camera::visibility::RenderLayers;
use bevy::log::LogPlugin;
use bevy::window::WindowResolution;

use crabgal_core::config::GameConfig;
use crabgal_core::step;
use crabgal_core::Action;
use crabgal_core::state::State;
use crabgal_script::parser::parse_script;

use render::blur::{BlurCamera, BlurPlugin};
use resources::*;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let dir = args.get(2).map(PathBuf::from).unwrap_or_default();
    let dir = std::env::current_dir().unwrap_or_default().join(&dir);
    let config = GameConfig::load(&dir.join("config.yaml"));
    let assets_path = dir.join("assets");

    App::new()
        .add_plugins(
            DefaultPlugins
                .build()
                .set(AssetPlugin {
                    file_path: assets_path.to_string_lossy().into(),
                    processed_file_path: dir.join("imported_assets").to_string_lossy().into(),
                    mode: AssetMode::Processed,
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: config.title.clone().into(),
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
        .add_plugins(plugin::GamePlugin)
        .add_plugins(BlurPlugin)
        .insert_resource(ProjectDir(dir.clone()))
        .insert_resource(Cfg(config))
        .add_systems(Startup, (setup).chain())
        .run();
}

// ── Startup ──

fn setup(mut commands: Commands, dir: Res<ProjectDir>) {
    // Camera 0: background + sprites (layer 0), with blur post-processing
    commands.spawn((
        Camera2d,
        Camera { order: 0, ..default() },
        RenderLayers::layer(0),
        BlurCamera::default(),
    ));
    // Camera 1: textbox + control bar UI (layer 1), renders after blur
    commands.spawn((
        Camera2d,
        Camera { order: 1, clear_color: ClearColorConfig::None, ..default() },
        RenderLayers::layer(1),
    ));

    let sd = dir.0.join("scripts");
    let _ = std::fs::create_dir_all(&sd);
    let _ = std::fs::create_dir_all(dir.0.join("assets").join("fonts"));

    let mut s = State::new();
    load_scenes(&mut s, &sd);
    if s.scenes.is_empty() {
        s.scenes.insert(
            "s".into(),
            vec![
                Action::ShowBg {
                    image: "bg.webp".into(),
                    transition: Default::default(),
                },
                Action::Say {
                    speaker: "?".into(),
                    text: "no script".into(),
                },
            ],
        );
    }
    s.current_scene = s.scenes.keys().next().cloned().unwrap_or_default();
    step::index_labels(&mut s);
    step::step(&mut s);

    let wrx = crabgal_script::watcher::start_watcher(&sd)
        .ok()
        .unwrap_or_else(|| {
            let (_, rx) = std::sync::mpsc::channel();
            rx
        });

    commands.insert_resource(WatcherRx(std::sync::Mutex::new(wrx)));
    commands.insert_resource(AppState(Arc::new(RwLock::new(s))));
}

// ── Helpers ──

fn load_scenes(s: &mut State, d: &std::path::Path) {
    s.scenes.clear();
    let dir = match std::fs::read_dir(d) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in dir.flatten() {
        let p = entry.path();
        let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("");
        if ext != "crab" && ext != "txt" {
            continue;
        }
        let name = p.file_stem().unwrap().to_string_lossy().to_string();
        let content = match std::fs::read_to_string(&p) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let actions = if ext == "txt" {
            crabgal_script::parse_webgal(&content)
        } else {
            parse_script(&content)
        };
        s.scenes.insert(name.clone(), actions);
        if s.current_scene.is_empty() {
            s.current_scene = name;
        }
    }
}
