use std::path::PathBuf;
use std::sync::Mutex;

use bevy::prelude::*;
use crabgal_core::State;
use crabgal_core::config::GameConfig;
use crabgal_script::{ResourceRef, SceneRef, ScriptWatcher};
use std::collections::HashMap;

#[derive(Resource, Deref, DerefMut)]
pub struct GameState(pub State);

#[derive(Resource, Deref)]
pub struct GameConfigResource(pub GameConfig);

#[derive(Resource, Deref)]
pub struct ProjectRoot(pub PathBuf);

#[derive(Resource)]
pub struct ScriptWatcherResource(pub Mutex<ScriptWatcher>);

#[derive(Resource, Default, Deref, DerefMut)]
pub struct LocalAssetManifest(pub HashMap<String, LocalSceneAssets>);

#[derive(Default)]
pub struct LocalSceneAssets {
    pub resources: Vec<ResourceRef>,
    pub sub_scenes: Vec<SceneRef>,
}

#[derive(Resource, Default)]
pub struct LocalAssetCache(pub HashMap<String, UntypedHandle>);
