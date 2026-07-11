use std::collections::{HashMap, HashSet};

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use crabgal_core::Anchor;

use crate::components::SpriteNode;
use crate::resources::{GameConfigResource, GameState};
use crate::viewport::DesignViewport;

/// Synchronizes character sprites with the engine state via stable sprite IDs.
pub fn sync_sprites(
    state: Res<GameState>,
    config: Res<GameConfigResource>,
    asset_server: Res<AssetServer>,
    images: Res<Assets<Image>>,
    mut commands: Commands,
    sprite_query: Query<(Entity, &SpriteNode)>,
    window_query: Query<&Window>,
) {
    let Ok(window) = window_query.single() else {
        return;
    };
    let viewport = DesignViewport::from_window(window);
    let desired_ids = state.sprites.keys().collect::<HashSet<_>>();

    for (entity, node) in &sprite_query {
        if !desired_ids.contains(&node.0) {
            commands.entity(entity).despawn();
        }
    }

    let existing = sprite_query
        .iter()
        .map(|(entity, node)| (node.0.as_str(), entity))
        .collect::<HashMap<_, _>>();
    let mut sprites = state.sprites.iter().collect::<Vec<_>>();
    sprites.sort_by(|(_, left), (_, right)| left.position.y.total_cmp(&right.position.y));

    for (index, (id, data)) in sprites.into_iter().enumerate() {
        let handle: Handle<Image> = asset_server.load(format!("figure/{}", data.image));
        let texture_aspect = images.get(&handle).map_or(0.7, |image| {
            let size = image.size();
            if size.y == 0 {
                0.7
            } else {
                size.x as f32 / size.y as f32
            }
        });

        let base_height = config.layout.sprite_height;
        let base_width = base_height * texture_aspect;
        let transform = data.transform;
        let progress = data.transition_progress.clamp(0.0, 1.0);
        let transition_x = (1.0 - progress) * data.transition_offset_x;
        let alpha = (progress * transform.alpha).clamp(0.0, 1.0);
        let width = base_width * transform.scale_x;

        let center_x = match data.position.x {
            Anchor::Left(offset) => offset.max(config.layout.anchor_offset) + width * 0.5,
            Anchor::Center(offset) => crabgal_core::DESIGN_WIDTH * 0.5 + offset,
            Anchor::Right(offset) => {
                crabgal_core::DESIGN_WIDTH - offset.max(config.layout.anchor_offset) - width * 0.5
            }
        };
        let center_y = data.position.y + base_height * 0.5;
        let world_position = viewport.world_from_design(Vec2::new(
            center_x + transform.offset_x + transition_x,
            center_y + transform.offset_y,
        ));
        let z = 0.1 + data.z_index as f32 * 0.001 + index as f32 * 0.000_001;

        let sprite = Sprite {
            image: handle,
            custom_size: Some(Vec2::new(
                width * viewport.scale,
                base_height * transform.scale_y * viewport.scale,
            )),
            color: Color::srgba(1.0, 1.0, 1.0, alpha),
            ..default()
        };
        let entity_transform = Transform::from_translation(world_position.extend(z))
            .with_rotation(Quat::from_rotation_z(transform.rotation));

        if let Some(&entity) = existing.get(id.as_str()) {
            commands
                .entity(entity)
                .insert((sprite, entity_transform, RenderLayers::layer(0)));
        } else {
            commands.spawn((
                Name::new(format!("sprite::{id}")),
                SpriteNode(id.clone()),
                sprite,
                entity_transform,
                RenderLayers::layer(0),
            ));
        }
    }
}
