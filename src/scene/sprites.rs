use std::collections::HashMap;

use bevy::camera::visibility::RenderLayers;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use crabgal_core::{Anchor, BlendMode};

use crate::runtime::resources::{GameConfigResource, GameState};
use crate::runtime::viewport::DesignViewport;
use crate::scene::effects::material::{StageMaterial, StageQuad, animation_uniform};
use crate::scene::images::ImageDimensions;

/// Marker for sprite entities with stable IDs used by diff-based synchronization.
#[derive(Component)]
pub(crate) struct SpriteNode(pub(crate) String);

#[derive(Default)]
pub(crate) struct SpriteRenderCache {
    initialized: bool,
    sprites: HashMap<String, crabgal_core::state::Sprite>,
}

impl SpriteRenderCache {
    fn matches(&self, state: &GameState) -> bool {
        self.initialized && self.sprites == state.sprites
    }

    fn capture(&mut self, state: &GameState) {
        self.initialized = true;
        self.sprites.clone_from(&state.sprites);
    }
}

fn sprite_center_y(
    position_y: f32,
    base_height: f32,
    project_offset_y: f32,
    transform_offset_y: f32,
) -> f32 {
    position_y + base_height * 0.5 + project_offset_y + transform_offset_y
}

#[derive(SystemParam)]
pub(crate) struct SpriteRenderResources<'w> {
    asset_server: Res<'w, AssetServer>,
    dimensions: Res<'w, ImageDimensions>,
    quad: Res<'w, StageQuad>,
    materials: ResMut<'w, Assets<StageMaterial>>,
}

type RenderedSpriteQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static mut Transform,
        Option<&'static mut Sprite>,
        Option<&'static MeshMaterial2d<StageMaterial>>,
    ),
>;

#[derive(SystemParam)]
pub(crate) struct SpriteEntityContext<'w, 's> {
    commands: Commands<'w, 's>,
    nodes: Query<'w, 's, (Entity, &'static SpriteNode)>,
    rendered: RenderedSpriteQuery<'w, 's>,
    windows: Query<'w, 's, Ref<'static, Window>>,
    cache: Local<'s, SpriteRenderCache>,
}

/// Synchronizes character sprites with the engine state via stable sprite IDs.
pub(crate) fn sync_sprites(
    state: Res<GameState>,
    config: Res<GameConfigResource>,
    mut render: SpriteRenderResources,
    mut entities: SpriteEntityContext,
) {
    let Ok(window) = entities.windows.single() else {
        return;
    };
    let sprites_changed = !entities.cache.matches(&state);
    if !sprites_changed && !render.dimensions.is_changed() && !window.is_changed() {
        return;
    }
    if sprites_changed {
        entities.cache.capture(&state);
    }
    let viewport = DesignViewport::from_window(&window);
    for (entity, node) in &entities.nodes {
        if !state.sprites.contains_key(&node.0) {
            entities.commands.entity(entity).despawn();
        }
    }

    for (id, data) in &state.sprites {
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
        let center_y = sprite_center_y(
            data.position.y,
            base_height,
            config.layout.sprite_y_offset,
            transform.offset_y,
        );
        let world_position = viewport.world_from_design(Vec2::new(
            center_x + transform.offset_x + transition_x,
            center_y,
        ));
        let z =
            0.1 + data.z_index as f32 * 0.001 + data.position.y.clamp(-999.0, 999.0) * 0.000_000_01;

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

        if let Some(entity) = entities
            .nodes
            .iter()
            .find_map(|(entity, node)| (node.0 == *id).then_some(entity))
        {
            let Ok((mut current_transform, current_sprite, existing_material)) =
                entities.rendered.get_mut(entity)
            else {
                continue;
            };
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
                *current_transform = mesh_transform;
                if existing_material.is_none() {
                    entities.commands.entity(entity).remove::<Sprite>().insert((
                        Mesh2d(render.quad.0.clone()),
                        MeshMaterial2d(material_handle),
                    ));
                }
            } else {
                *current_transform = entity_transform;
                if let Some(mut current_sprite) = current_sprite {
                    *current_sprite = sprite;
                } else {
                    entities
                        .commands
                        .entity(entity)
                        .remove::<Mesh2d>()
                        .remove::<MeshMaterial2d<StageMaterial>>()
                        .insert(sprite);
                }
            }
        } else {
            let mut entity = entities.commands.spawn((
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

#[cfg(test)]
mod tests {
    use super::sprite_center_y;

    #[test]
    fn project_offset_moves_the_shared_sprite_baseline() {
        let center = sprite_center_y(0.0, 1080.0, -90.0, 0.0);

        assert_eq!(center, 450.0);
        assert_eq!(center - 540.0, -90.0);
        assert_eq!(center + 540.0, 990.0);
    }

    #[test]
    fn script_transform_remains_relative_to_the_project_offset() {
        assert_eq!(sprite_center_y(12.0, 1080.0, -90.0, 24.0), 486.0);
    }
}
