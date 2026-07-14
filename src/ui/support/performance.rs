use bevy::camera::visibility::RenderLayers;
use bevy::diagnostic::{
    DiagnosticsStore, EntityCountDiagnosticsPlugin, FrameTimeDiagnosticsPlugin,
};
use bevy::prelude::*;

use crate::render::blur::DialogCamera;
use crate::ui::foundation::UiFonts;

#[derive(Component)]
pub(crate) struct PerformanceOverlay;

pub fn setup_performance_overlay(
    mut commands: Commands,
    camera: Query<Entity, With<DialogCamera>>,
    fonts: Res<UiFonts>,
) {
    let Ok(camera) = camera.single() else {
        return;
    };
    commands.spawn((
        Name::new("performance_overlay"),
        PerformanceOverlay,
        Text::new("Performance data warming up..."),
        TextFont {
            font: fonts.text.clone().into(),
            font_size: FontSize::from(15.0),
            ..default()
        },
        TextColor(Color::srgb(0.45, 1.0, 0.55)),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            right: Val::Px(12.0),
            padding: UiRect::axes(Val::Px(9.0), Val::Px(6.0)),
            display: Display::None,
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.72)),
        GlobalZIndex(1000),
        UiTargetCamera(camera),
        RenderLayers::layer(2),
    ));
}

pub fn toggle_performance_overlay(
    keys: Res<ButtonInput<KeyCode>>,
    mut overlays: Query<&mut Node, With<PerformanceOverlay>>,
) {
    if !keys.just_pressed(KeyCode::F3) {
        return;
    }
    for mut node in &mut overlays {
        node.display = if node.display == Display::None {
            Display::Flex
        } else {
            Display::None
        };
    }
}

pub fn update_performance_overlay(
    time: Res<Time>,
    diagnostics: Res<DiagnosticsStore>,
    mut elapsed: Local<f32>,
    mut overlays: Query<(&Node, &mut Text, &mut TextColor), With<PerformanceOverlay>>,
) {
    *elapsed += time.delta_secs();
    if *elapsed < 0.25 {
        return;
    }
    *elapsed = 0.0;

    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|value| value.smoothed())
        .unwrap_or_default();
    let frame_ms = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
        .and_then(|value| value.smoothed())
        .unwrap_or_default();
    let entities = diagnostics
        .get(&EntityCountDiagnosticsPlugin::ENTITY_COUNT)
        .and_then(|value| value.smoothed())
        .unwrap_or_default();
    let (status, color) = if fps >= 55.0 {
        ("GOOD", Color::srgb(0.45, 1.0, 0.55))
    } else if fps >= 30.0 {
        ("CHECK", Color::srgb(1.0, 0.82, 0.3))
    } else {
        ("SLOW", Color::srgb(1.0, 0.35, 0.3))
    };

    for (node, mut text, mut text_color) in &mut overlays {
        if node.display == Display::None {
            continue;
        }
        text.0 = format!("{status}  {fps:.0} FPS  {frame_ms:.1} ms  {entities:.0} entities");
        text_color.0 = color;
    }
}
