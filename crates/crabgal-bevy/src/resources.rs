use bevy::prelude::*;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use crabgal_core::config::GameConfig;
use crabgal_core::state::State;

/// Shared game state (wrapped for thread-safe interior mutability)
#[derive(Resource, Clone)]
pub struct AppState(pub Arc<RwLock<State>>);

/// Cached design-to-window scale factor, updated on resize.
#[derive(Resource, Clone, Copy)]
pub struct DesignScale(pub f32);

impl Default for DesignScale {
    fn default() -> Self { Self(1.0) }
}

/// Project assets directory
#[derive(Resource, Clone)]
pub struct ProjectDir(pub PathBuf);

/// Game configuration loaded from config.yaml
#[derive(Resource, Clone)]
pub struct Cfg(pub GameConfig);

/// Hot-reload script watcher receiver (Mutex-wrapped for thread safety)
#[derive(Resource)]
pub struct WatcherRx(pub std::sync::Mutex<std::sync::mpsc::Receiver<PathBuf>>);
