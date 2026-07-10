use std::path::PathBuf;
use std::sync::Mutex;

use bevy::prelude::*;
use crabgal_core::State;
use crabgal_core::config::GameConfig;
use crabgal_script::ScriptWatcher;

#[derive(Resource, Deref, DerefMut)]
pub struct GameState(pub State);

#[derive(Resource, Deref)]
pub struct GameConfigResource(pub GameConfig);

#[derive(Resource, Deref)]
pub struct ProjectRoot(pub PathBuf);

#[derive(Resource)]
pub struct ScriptWatcherResource(pub Mutex<ScriptWatcher>);
