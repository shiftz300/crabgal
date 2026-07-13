use std::collections::{HashMap, HashSet};

use bevy::asset::LoadState;
use bevy::prelude::*;
use crabgal_loader::ResourceKind;

use crate::runtime::resources::{
    AssetLoadingGate, GameConfigResource, GameState, LocalAssetCache, LocalAssetManifest,
};
use crate::ui::foundation::UiFonts;

const LOOKAHEAD_ACTIONS: usize = 20;

pub fn prefetch_local_assets(
    state: Res<GameState>,
    config: Res<GameConfigResource>,
    manifest: Res<LocalAssetManifest>,
    asset_server: Res<AssetServer>,
    mut cache: ResMut<LocalAssetCache>,
) {
    if !state.is_changed() && !manifest.is_changed() {
        return;
    }
    let mut desired = HashMap::new();
    // While the title is open, use otherwise idle time to warm the entry scene.
    // This also keeps its handles alive after returning from the game, instead
    // of releasing and recreating them on the next START click.
    let (scene_name, cursor) = if state.ended {
        (crate::scene::entry_scene(&state), 0)
    } else {
        (state.current_scene.clone(), state.cursor)
    };
    if let Some(scene) = manifest.get(&scene_name) {
        for resource in scene.resources.iter().filter(|resource| {
            resource.action_index >= cursor && resource.action_index <= cursor + LOOKAHEAD_ACTIONS
        }) {
            let path = resolve_path(resource.kind, &resource.path, &config);
            desired.insert(path, resource.kind);
        }
        for reference in scene.sub_scenes.iter().filter(|reference| {
            reference.action_index >= cursor && reference.action_index <= cursor + LOOKAHEAD_ACTIONS
        }) {
            if let Some(called_scene) = manifest.get(&reference.scene) {
                // A callScene may be large. Warm only its opening window; the
                // normal cursor lookahead takes over after entering it.
                for resource in called_scene
                    .resources
                    .iter()
                    .filter(|resource| resource.action_index <= LOOKAHEAD_ACTIONS)
                {
                    let path = resolve_path(resource.kind, &resource.path, &config);
                    desired.insert(path, resource.kind);
                }
            }
        }
    }

    if state.ended {
        desired.insert(
            config.bg_path(&config.title_background),
            ResourceKind::Background,
        );
    }

    if let Some(background) = &state.bg {
        desired.insert(config.bg_path(background), ResourceKind::Background);
    }
    for sprite in state.sprites.values() {
        desired.insert(config.figure_path(&sprite.image), ResourceKind::Figure);
    }
    if let Some(vocal) = state
        .dialogue
        .as_ref()
        .and_then(|dialogue| dialogue.vocal.as_ref())
    {
        desired.insert(config.voice_path(vocal), ResourceKind::Voice);
    }
    if let Some(bgm) = &state.bgm.file {
        desired.insert(config.bgm_path(bgm), ResourceKind::Bgm);
    }
    for effect in state.looping_effects.values() {
        desired.insert(config.effect_path(&effect.file), ResourceKind::Effect);
    }

    let desired_paths = desired.keys().cloned().collect::<HashSet<_>>();
    cache.0.retain(|path, _| desired_paths.contains(path));
    for (path, kind) in desired {
        cache.0.entry(path.clone()).or_insert_with(|| match kind {
            ResourceKind::Background | ResourceKind::Figure | ResourceKind::MiniAvatar => {
                asset_server.load::<Image>(path).untyped()
            }
            ResourceKind::Voice | ResourceKind::Bgm | ResourceKind::Effect => {
                asset_server.load::<AudioSource>(path).untyped()
            }
        });
    }
}

pub fn update_loading_gate(
    asset_server: Res<AssetServer>,
    cache: Res<LocalAssetCache>,
    fonts: Res<UiFonts>,
    mut gate: ResMut<AssetLoadingGate>,
) {
    let pending = |id| {
        matches!(
            asset_server.load_state(id),
            LoadState::NotLoaded | LoadState::Loading
        )
    };
    gate.blocked = cache.0.values().any(|handle| pending(handle.id()))
        || pending(fonts.text.id().untyped())
        || pending(fonts.icons.id().untyped());
}

fn resolve_path(kind: ResourceKind, path: &str, config: &GameConfigResource) -> String {
    match kind {
        ResourceKind::Background => config.bg_path(path),
        ResourceKind::Figure | ResourceKind::MiniAvatar => config.figure_path(path),
        ResourceKind::Voice => config.voice_path(path),
        ResourceKind::Bgm => config.bgm_path(path),
        ResourceKind::Effect => config.effect_path(path),
    }
}
