use std::collections::{HashMap, HashSet};

use bevy::camera::visibility::RenderLayers;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use crabgal_core::{Anchor, BlendMode};

use crate::runtime::resources::{GameConfigResource, GameState};
use crate::runtime::viewport::DesignViewport;
use crate::scene::components::SpriteNode;
use crate::scene::effects::material::{StageMaterial, StageQuad, animation_uniform};
use crate::scene::images::ImageDimensions;

#[derive(SystemParam)]
pub(crate) struct SpriteRenderResources<'w> {
    asset_server: Res<'w, AssetServer>,
    dimensions: Res<'w, ImageDimensions>,
    quad: Res<'w, StageQuad>,
    materials: ResMut<'w, Assets<StageMaterial>>,
}

/// Synchronizes character sprites with the engine state via stable sprite IDs.
pub(crate) fn sync_sprites(
    state: Res<GameState>,
    config: Res<GameConfigResource>,
    mut render: SpriteRenderResources,
    mut commands: Commands,
    sprite_query: Query<(Entity, &SpriteNode, Option<&MeshMaterial2d<StageMaterial>>)>,
    window_query: Query<Ref<Window>>,
) {
    let Ok(window) = window_query.single() else {
        return;
    };
    if !state.is_changed() && !render.dimensions.is_changed() && !window.is_changed() {
        return;
    }
    let viewport = DesignViewport::from_window(&window);
    let desired_ids = state.sprites.keys().collect::<HashSet<_>>();

    for (entity, node, _) in &sprite_query {
        if !desired_ids.contains(&node.0) {
            commands.entity(entity).despawn();
        }
    }

    let existing = sprite_query
        .iter()
        .map(|(entity, node, material)| (node.0.as_str(), (entity, material)))
        .collect::<HashMap<_, _>>();
    let mut sprites = state.sprites.iter().collect::<Vec<_>>();
    sprites.sort_by(|(_, left), (_, right)| left.position.y.total_cmp(&right.position.y));

    for (index, (id, data)) in sprites.into_iter().enumerate() {
        let handle: Handle<Image> = render.asset_server.load(format!("figure/{}", data.image));
        let texture_aspect = render.dimensions.aspect(&handle).unwrap_or(0.7);

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
            image: handle.clone(),
            custom_size: Some(Vec2::new(
                width * viewport.scale,
                base_height * transform.scale_y * viewport.scale,
            )),
            color: Color::srgba(1.0, 1.0, 1.0, alpha),
            ..default()
        };
        let entity_transform = Transform::from_translation(world_position.extend(z))
            .with_rotation(Quat::from_rotation_z(transform.rotation));
        let mut filter = data.filter;
        filter.blur += transform.blur;
        let animation = animation_uniform(data.animation.as_ref());
        let uses_material =
            data.blend != BlendMode::Alpha || !filter.is_identity() || animation.z > 0.0;

        if let Some(&(entity, existing_material)) = existing.get(id.as_str()) {
            if uses_material {
                let material = StageMaterial::new(handle, alpha, filter, data.blend, animation);
                let material_handle = if let Some(existing_material) = existing_material {
                    if let Some(mut current) = render.materials.get_mut(&existing_material.0) {
                        *current = material;
                    }
                    existing_material.0.clone()
                } else {
                    render.materials.add(material)
                };
                let mesh_transform = entity_transform.with_scale(Vec3::new(
                    width * viewport.scale,
                    base_height * transform.scale_y * viewport.scale,
                    1.0,
                ));
                commands.entity(entity).remove::<Sprite>().insert((
                    Mesh2d(render.quad.0.clone()),
                    MeshMaterial2d(material_handle),
                    mesh_transform,
                    RenderLayers::layer(0),
                ));
            } else {
                commands
                    .entity(entity)
                    .remove::<Mesh2d>()
                    .remove::<MeshMaterial2d<StageMaterial>>()
                    .insert((sprite, entity_transform, RenderLayers::layer(0)));
            }
        } else {
            let mut entity = commands.spawn((
                Name::new(format!("sprite::{id}")),
                SpriteNode(id.clone()),
                RenderLayers::layer(0),
            ));
            if uses_material {
                let material = render.materials.add(StageMaterial::new(
                    handle, alpha, filter, data.blend, animation,
                ));
                entity.insert((
                    Mesh2d(render.quad.0.clone()),
                    MeshMaterial2d(material),
                    entity_transform.with_scale(Vec3::new(
                        width * viewport.scale,
                        base_height * transform.scale_y * viewport.scale,
                        1.0,
                    )),
                ));
            } else {
                entity.insert((sprite, entity_transform));
            }
        }
    }
}
