// crabgal-bevy — Bevy VN engine
mod components;
mod plugin;
mod resources;
mod systems;

use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use bevy::prelude::*;
use bevy::asset::AssetPlugin;

use crabgal_core::config::GameConfig;
use crabgal_core::step;
use crabgal_core::Action;
use crabgal_core::state::State;
use crabgal_script::parser::parse_script;
use bevy::core_pipeline::tonemapping::DebandDither;
use bevy_blur_regions_fork::prelude::*;
use resources::*;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let dir = args.get(2).map(PathBuf::from).unwrap_or_default();
    // Resolve to absolute — AssetPlugin needs it
    let dir = std::env::current_dir().unwrap_or_default().join(&dir);
    let config = GameConfig::load(&dir.join("config.yaml"));
    let assets_path = dir.join("assets");

    App::new()
        .add_plugins(
            DefaultPlugins
                .set(AssetPlugin {
                    file_path: assets_path.to_string_lossy().into(),
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: config.title.clone().into(),
                        resolution: (1280.0, 720.0).into(),
                        resizable: true,
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default()),
        )
        .add_plugins(plugin::GamePlugin)
        .add_plugins(BlurRegionsPlugin::default())
        .insert_resource(ProjectDir(dir.clone()))
        .insert_resource(Cfg(config))
        .insert_resource(TextureMap::default())
        .add_systems(Startup, (setup, load_textures).chain())
        .run();
}

// ── Startup ──

fn setup(mut commands: Commands, dir: Res<ProjectDir>) {
    commands.spawn((Camera2d, BlurRegionsCamera::default(), DebandDither::Enabled));

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

fn load_textures(
    dir: Res<ProjectDir>,
    asset_server: Res<AssetServer>,
    mut texture_map: ResMut<TextureMap>,
) {
    // Load backgrounds
    load_dir(&dir.0, "background", &asset_server, &mut texture_map.bg);
    // Load figures
    load_dir(&dir.0, "figure", &asset_server, &mut texture_map.sprites);
}

fn load_dir(
    dir: &std::path::Path,
    subdir: &str,
    asset_server: &AssetServer,
    list: &mut Vec<(String, TexInfo)>,
) {
    let path = dir.join("assets").join(subdir);
    let entries = match std::fs::read_dir(&path) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let p = entry.path();
        let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("");
        if !matches!(ext, "png" | "webp" | "jpg" | "jpeg") {
            continue;
        }
        let name = p.file_name().unwrap().to_string_lossy().to_string();
        let relative = format!("{}/{}", subdir, name);
        let handle: Handle<Image> = asset_server.load(&relative);
        info!("Queued texture: {}", relative);
        list.push((name, TexInfo { handle, width: 0, height: 0 }));
    }
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
