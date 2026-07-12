use bevy::audio::{AudioSinkPlayback, PlaybackMode, Volume};
use bevy::prelude::*;

use crate::runtime::resources::{GameConfigResource, GameState};
use crate::storage::settings::RuntimeSettings;

#[derive(Component)]
pub struct VocalPlayer;

#[derive(Resource, Default)]
pub struct VocalPlayback {
    key: Option<(String, usize, String)>,
}

pub fn apply_master_volume(
    settings: Res<RuntimeSettings>,
    state: Res<GameState>,
    mut sinks: Query<&mut AudioSink, With<VocalPlayer>>,
) {
    if !settings.is_changed() {
        return;
    }
    let line_volume = state
        .dialogue
        .as_ref()
        .map_or(1.0, |dialogue| dialogue.volume);
    for mut sink in &mut sinks {
        sink.set_volume(Volume::Linear(
            line_volume * settings.master_volume * settings.vocal_volume,
        ));
    }
}

pub fn sync_vocal(
    state: Res<GameState>,
    config: Res<GameConfigResource>,
    settings: Res<RuntimeSettings>,
    asset_server: Res<AssetServer>,
    mut playback: ResMut<VocalPlayback>,
    players: Query<Entity, With<VocalPlayer>>,
    mut commands: Commands,
) {
    let key = state.dialogue.as_ref().and_then(|dialogue| {
        dialogue
            .vocal
            .as_ref()
            .map(|vocal| (state.current_scene.clone(), state.cursor, vocal.clone()))
    });
    if playback.key == key {
        return;
    }
    playback.key.clone_from(&key);
    for entity in &players {
        commands.entity(entity).despawn();
    }

    let Some((_, _, vocal)) = key else { return };
    let volume = state
        .dialogue
        .as_ref()
        .map_or(1.0, |dialogue| dialogue.volume);
    commands.spawn((
        Name::new(format!("vocal::{vocal}")),
        VocalPlayer,
        AudioPlayer::new(asset_server.load(config.voice_path(&vocal))),
        PlaybackSettings {
            mode: PlaybackMode::Despawn,
            volume: Volume::Linear(volume * settings.master_volume * settings.vocal_volume),
            ..default()
        },
    ));
}
