// Loading screen — shown while assets are being loaded.
use crate::render::blur::UiBlurCamera;
use crate::resources::{GameState, LocalAssetCache};
use bevy::asset::LoadState;
use bevy::prelude::*;

#[derive(Component)]
pub(crate) struct LoadingText;

pub fn setup_loading(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    ui_camera_q: Query<Entity, With<UiBlurCamera>>,
) {
    let Ok(ui_camera) = ui_camera_q.single() else {
        return;
    };
    let font: Handle<Font> = asset_server.load("fonts/MavenPro-CJK.ttf");
    commands.spawn((
        Name::new("loading"),
        LoadingText,
        Text::new("Loading..."),
        TextFont {
            font: font.into(),
            font_size: FontSize::from(24.0),
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Percent(50.0),
            left: Val::Percent(50.0),
            ..default()
        },
        ZIndex(200),
        UiTargetCamera(ui_camera),
    ));
}

pub fn update_loading(
    state: Res<GameState>,
    asset_server: Res<AssetServer>,
    cache: Res<LocalAssetCache>,
    mut query: Query<&mut Visibility, With<LoadingText>>,
) {
    let loading = !state.ended
        && cache.0.values().any(|handle| {
            matches!(
                asset_server.load_state(handle.id()),
                LoadState::NotLoaded | LoadState::Loading
            )
        });
    for mut visibility in &mut query {
        *visibility = if loading {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}
