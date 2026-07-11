use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy::ui::FocusPolicy;

use crate::render::blur::UiBlurCamera;
use crate::resources::{GameConfigResource, GameState};

#[derive(Component)]
pub struct TitleRoot;

#[derive(Component)]
pub struct StartButton;

pub fn sync_title(
    state: Res<GameState>,
    config: Res<GameConfigResource>,
    roots: Query<Entity, With<TitleRoot>>,
    camera: Query<Entity, With<UiBlurCamera>>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
) {
    if state.ended != roots.is_empty() {
        return;
    }
    for entity in &roots {
        commands.entity(entity).despawn();
    }
    if !state.ended {
        return;
    }
    let Ok(camera) = camera.single() else { return };
    let font: Handle<Font> = asset_server.load("fonts/MavenPro-CJK.ttf");
    commands
        .spawn((
            Name::new("title"),
            TitleRoot,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(80.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.02, 0.02, 0.03, 0.96)),
            FocusPolicy::Block,
            GlobalZIndex(190),
            UiTargetCamera(camera),
            RenderLayers::layer(1),
        ))
        .with_children(|title| {
            title.spawn((
                Text::new(config.title.clone()),
                TextFont {
                    font: font.clone().into(),
                    font_size: FontSize::from(96.0),
                    ..default()
                },
                TextColor(Color::WHITE),
            ));
            title
                .spawn((
                    Button,
                    StartButton,
                    Node {
                        padding: UiRect::axes(Val::Px(64.0), Val::Px(20.0)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.08)),
                ))
                .with_child((
                    Text::new("START"),
                    TextFont {
                        font: font.into(),
                        font_size: FontSize::from(42.0),
                        ..default()
                    },
                    TextColor(Color::srgba(1.0, 1.0, 1.0, 0.8)),
                ));
        });
}

pub fn handle_title_input(
    buttons: Query<&Interaction, (With<StartButton>, Changed<Interaction>)>,
    keys: Res<ButtonInput<KeyCode>>,
    mut state: ResMut<GameState>,
) {
    let requested = buttons
        .iter()
        .any(|interaction| *interaction == Interaction::Pressed)
        || keys.just_pressed(KeyCode::Enter)
        || keys.just_pressed(KeyCode::Space);
    if !state.ended || !requested {
        return;
    }
    state.ended = false;
    state.current_scene = crate::scene::entry_scene(&state);
    state.cursor = 0;
    crabgal_core::step::index_labels(&mut state);
    crabgal_core::step::step(&mut state);
}
