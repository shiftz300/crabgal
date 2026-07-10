use std::collections::HashSet;

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use crabgal_core::dissolve;
use crabgal_core::{DESIGN_HEIGHT, DESIGN_WIDTH};

use crate::components::{BackgroundLayer, BackgroundNode};
use crate::resources::GameState;
use crate::viewport::DesignViewport;

struct DesiredBackground<'a> {
    layer: BackgroundLayer,
    image: &'a str,
    alpha: f32,
    z: f32,
}

/// Synchronizes background entities without recreating them every frame.
pub fn sync_bg(
    state: Res<GameState>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    mut backgrounds: Query<(Entity, &mut BackgroundNode, &mut Sprite, &mut Transform)>,
    window_query: Query<&Window>,
) {
    let Ok(window) = window_query.single() else {
        return;
    };
    let viewport = DesignViewport::from_window(window);
    let desired = desired_backgrounds(&state);
    let mut existing_layers = HashSet::new();

    for (entity, mut node, mut sprite, mut transform) in &mut backgrounds {
        let Some(background) = desired.iter().find(|item| item.layer == node.layer) else {
            commands.entity(entity).despawn();
            continue;
        };

        existing_layers.insert(node.layer);
        if node.image != background.image {
            node.image = background.image.to_owned();
            sprite.image = asset_server.load(format!("background/{}", background.image));
        }
        apply_background_layout(&mut sprite, &mut transform, background, viewport);
    }

    for background in desired {
        if existing_layers.contains(&background.layer) {
            continue;
        }
        let mut sprite = Sprite {
            image: asset_server.load(format!("background/{}", background.image)),
            ..default()
        };
        let mut transform = Transform::default();
        apply_background_layout(&mut sprite, &mut transform, &background, viewport);
        commands.spawn((
            Name::new(format!("background::{:?}", background.layer)),
            BackgroundNode {
                layer: background.layer,
                image: background.image.to_owned(),
            },
            sprite,
            transform,
            RenderLayers::layer(0),
        ));
    }
}

fn desired_backgrounds(state: &GameState) -> Vec<DesiredBackground<'_>> {
    if let Some(transition) = &state.bg_transition {
        let mut backgrounds = Vec::with_capacity(2);
        if let Some(previous) = &transition.from {
            backgrounds.push(DesiredBackground {
                layer: BackgroundLayer::Previous,
                image: previous,
                alpha: 1.0,
                z: -1.0,
            });
        }
        backgrounds.push(DesiredBackground {
            layer: BackgroundLayer::Current,
            image: &transition.to,
            alpha: dissolve::smooth_fade(transition.progress),
            z: 0.0,
        });
        backgrounds
    } else {
        state
            .bg
            .as_deref()
            .map(|image| {
                vec![DesiredBackground {
                    layer: BackgroundLayer::Current,
                    image,
                    alpha: 1.0,
                    z: 0.0,
                }]
            })
            .unwrap_or_default()
    }
}

fn apply_background_layout(
    sprite: &mut Sprite,
    transform: &mut Transform,
    background: &DesiredBackground<'_>,
    viewport: DesignViewport,
) {
    sprite.custom_size = Some(Vec2::new(DESIGN_WIDTH, DESIGN_HEIGHT) * viewport.scale);
    sprite.color = Color::srgba(1.0, 1.0, 1.0, background.alpha);
    transform.translation = viewport.content_center().extend(background.z);
}
