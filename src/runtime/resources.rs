use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use bevy::prelude::*;
use crabgal_core::State;
use crabgal_core::config::GameConfig;
use crabgal_loader::{
    ContentProject, ResourceRef, SceneRef, ScriptLanguageRegistry, ScriptWatcher, StoreAdapter,
};
use std::collections::HashMap;

#[derive(Resource, Deref, DerefMut)]
pub struct GameState(pub State);

#[derive(Resource, Deref)]
pub struct GameConfigResource(pub GameConfig);

#[derive(Resource, Deref)]
pub struct ProjectRoot(pub PathBuf);

#[derive(Resource, Deref)]
pub struct ContentProjectResource(pub ContentProject);

#[derive(Resource, Deref)]
pub struct ScriptLanguages(pub ScriptLanguageRegistry);

#[derive(Resource, Clone)]
pub struct StoreCodec(pub Arc<dyn StoreAdapter>);

#[derive(Resource)]
pub struct ScriptWatcherResource(pub Mutex<ScriptWatcher>);

#[derive(Resource, Default, Deref, DerefMut)]
pub struct LocalAssetManifest(pub HashMap<String, LocalSceneAssets>);

#[derive(Default)]
pub struct LocalSceneAssets {
    pub resources: Vec<ResourceRef>,
    pub sub_scenes: Vec<SceneRef>,
    pub action_spans: Vec<crabgal_loader::SourceSpan>,
}

#[derive(Resource, Default)]
pub struct LocalAssetCache(pub HashMap<String, UntypedHandle>);

#[derive(Resource)]
pub struct AssetLoadingGate {
    pub blocked: bool,
}

impl Default for AssetLoadingGate {
    fn default() -> Self {
        Self { blocked: true }
    }
}
