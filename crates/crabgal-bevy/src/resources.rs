use bevy::prelude::*;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use crabgal_core::config::GameConfig;
use crabgal_core::state::State;

/// Shared game state (wrapped for thread-safe interior mutability)
#[derive(Resource, Clone)]
pub struct AppState(pub Arc<RwLock<State>>);

/// Project assets directory
#[derive(Resource, Clone)]
pub struct ProjectDir(pub PathBuf);

/// Game configuration loaded from config.yaml
#[derive(Resource, Clone)]
pub struct Cfg(pub GameConfig);

/// A loaded texture with its natural dimensions.
#[derive(Clone)]
pub struct TexInfo {
    pub handle: Handle<Image>,
    pub width: u32,
    pub height: u32,
}

/// Loaded texture handles keyed by filename.
#[derive(Resource, Default)]
pub struct TextureMap {
    pub bg: Vec<(String, TexInfo)>,
    pub sprites: Vec<(String, TexInfo)>,
}

/// Hot-reload script watcher receiver (Mutex-wrapped for thread safety)
#[derive(Resource)]
pub struct WatcherRx(pub std::sync::Mutex<std::sync::mpsc::Receiver<PathBuf>>);
