use bevy::audio::{PlaybackMode, Volume};
use bevy::prelude::*;

use crate::resources::{GameConfigResource, GameState};

#[derive(Component)]
pub struct VocalPlayer;

#[derive(Resource, Default)]
pub struct VocalPlayback {
    key: Option<(String, usize, String)>,
}

pub fn sync_vocal(
    state: Res<GameState>,
    config: Res<GameConfigResource>,
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
            volume: Volume::Linear(volume),
            ..default()
        },
    ));
}
