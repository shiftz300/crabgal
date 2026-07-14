// Loading screen — shown while assets are being loaded.
use crate::render::blur::DialogCamera;
use crate::runtime::resources::AssetLoadingGate;
use crate::ui::foundation::UiFonts;
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy::ui::FocusPolicy;

#[derive(Component)]
pub(crate) struct LoadingText;

#[derive(Component)]
pub(crate) struct LoadingOverlay;

pub fn setup_loading(
    mut commands: Commands,
    fonts: Res<UiFonts>,
    camera: Query<Entity, With<DialogCamera>>,
) {
    let Ok(camera) = camera.single() else {
        return;
    };
    let font = fonts.text.clone();
    commands
        .spawn((
            Name::new("loading_overlay"),
            LoadingOverlay,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::NONE),
            FocusPolicy::Block,
            GlobalZIndex(2000),
            UiTargetCamera(camera),
            RenderLayers::layer(2),
        ))
        .with_child((
            LoadingText,
            Text::new("Loading..."),
            TextFont {
                font: font.into(),
                font_size: FontSize::from(18.0),
                ..default()
            },
            TextColor(Color::WHITE),
        ));
}

pub fn update_loading(
    gate: Res<AssetLoadingGate>,
    mut query: Query<&mut Visibility, With<LoadingOverlay>>,
) {
    for mut visibility in &mut query {
        *visibility = if gate.blocked {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

pub fn assets_ready(gate: Res<AssetLoadingGate>) -> bool {
    !gate.blocked
}
