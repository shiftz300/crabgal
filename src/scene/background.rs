use std::collections::HashSet;

use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use crabgal_core::SpriteTransform;
use crabgal_core::dissolve;
use crabgal_core::{DESIGN_HEIGHT, DESIGN_WIDTH};

use crate::runtime::resources::GameState;
use crate::runtime::viewport::DesignViewport;
use crate::scene::components::{BackgroundLayer, BackgroundNode};
use crate::scene::effects::material::{StageMaterial, StageQuad, animation_uniform};

struct DesiredBackground<'a> {
    layer: BackgroundLayer,
    image: &'a str,
    alpha: f32,
    z: f32,
    transform: SpriteTransform,
    transition: Vec4,
}

/// Synchronizes background entities without recreating them every frame.
pub fn sync_bg(
    state: Res<GameState>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    mut backgrounds: Query<(
        Entity,
        &mut BackgroundNode,
        &mut Transform,
        Option<&MeshMaterial2d<StageMaterial>>,
    )>,
    quad: Res<StageQuad>,
    mut materials: ResMut<Assets<StageMaterial>>,
    window_query: Query<Ref<Window>>,
) {
    let Ok(window) = window_query.single() else {
        return;
    };
    if !state.is_changed() && !window.is_changed() {
        return;
    }
    let viewport = DesignViewport::from_window(&window);
    let desired = desired_backgrounds(&state);
    let mut existing_layers = HashSet::new();

    for (entity, mut node, mut transform, existing_material) in &mut backgrounds {
        let Some(background) = desired.iter().find(|item| item.layer == node.layer) else {
            commands.entity(entity).despawn();
            continue;
        };

        existing_layers.insert(node.layer);
        node.image = background.image.to_owned();
        let image = asset_server.load(format!("background/{}", background.image));
        apply_background_entity(
            &mut commands,
            entity,
            &mut transform,
            existing_material,
            &mut materials,
            &quad,
            image,
            background,
            viewport,
            state.bg_filter,
        );
    }

    for background in desired {
        if existing_layers.contains(&background.layer) {
            continue;
        }
        let mut transform = Transform::default();
        let image = asset_server.load(format!("background/{}", background.image));
        let entity = commands
            .spawn((
                Name::new(format!("background::{:?}", background.layer)),
                BackgroundNode {
                    layer: background.layer,
                    image: background.image.to_owned(),
                },
                transform,
                RenderLayers::layer(0),
            ))
            .id();
        apply_background_entity(
            &mut commands,
            entity,
            &mut transform,
            None,
            &mut materials,
            &quad,
            image,
            &background,
            viewport,
            state.bg_filter,
        );
    }
}

fn desired_backgrounds(state: &GameState) -> Vec<DesiredBackground<'_>> {
    let animation = animation_uniform(state.bg_animation.as_ref());
    if let Some(transition) = &state.bg_transition {
        let mut backgrounds = Vec::with_capacity(2);
        if let Some(previous) = &transition.from {
            backgrounds.push(DesiredBackground {
                layer: BackgroundLayer::Previous,
                image: previous,
                alpha: 1.0,
                z: -1.0,
                transform: state.bg_transform,
                transition: animation,
            });
        }
        if !transition.to.is_empty() {
            backgrounds.push(DesiredBackground {
                layer: BackgroundLayer::Current,
                image: &transition.to,
                alpha: match transition.kind {
                    crabgal_core::Transition::Wipe(_) | crabgal_core::Transition::Dissolve(_) => {
                        1.0
                    }
                    _ => dissolve::smooth_fade(transition.progress),
                },
                z: 0.0,
                transform: state.bg_transform,
                transition: match transition.kind {
                    crabgal_core::Transition::Wipe(_) => {
                        Vec4::new(1.0, transition.progress, animation.z, animation.w)
                    }
                    crabgal_core::Transition::Dissolve(_) => {
                        Vec4::new(2.0, transition.progress, animation.z, animation.w)
                    }
                    _ => animation,
                },
            });
        }
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
                    transform: state.bg_transform,
                    transition: animation,
                }]
            })
            .unwrap_or_default()
    }
}

#[allow(clippy::too_many_arguments)]
fn apply_background_entity(
    commands: &mut Commands,
    entity: Entity,
    transform: &mut Transform,
    existing_material: Option<&MeshMaterial2d<StageMaterial>>,
    materials: &mut Assets<StageMaterial>,
    quad: &StageQuad,
    image: Handle<Image>,
    background: &DesiredBackground<'_>,
    viewport: DesignViewport,
    mut filter: crabgal_core::VisualFilter,
) {
    let effect = background.transform;
    filter.blur += effect.blur;
    let size = Vec2::new(
        DESIGN_WIDTH * effect.scale_x,
        DESIGN_HEIGHT * effect.scale_y,
    ) * viewport.scale;
    let alpha = background.alpha * effect.alpha;
    transform.translation = (viewport.content_center()
        + Vec2::new(effect.offset_x, effect.offset_y) * viewport.scale)
        .extend(background.z);
    transform.rotation = Quat::from_rotation_z(effect.rotation);
    if !filter.is_identity() || background.transition.x > 0.0 || background.transition.z > 0.0 {
        transform.scale = size.extend(1.0);
        let material = StageMaterial::new(
            image,
            alpha,
            filter,
            crabgal_core::BlendMode::Alpha,
            background.transition,
        );
        let material_handle = if let Some(existing) = existing_material {
            if let Some(mut current) = materials.get_mut(&existing.0) {
                *current = material;
            }
            existing.0.clone()
        } else {
            materials.add(material)
        };
        commands
            .entity(entity)
            .remove::<Sprite>()
            .insert((Mesh2d(quad.0.clone()), MeshMaterial2d(material_handle)));
    } else {
        transform.scale = Vec3::ONE;
        commands
            .entity(entity)
            .remove::<Mesh2d>()
            .remove::<MeshMaterial2d<StageMaterial>>()
            .insert(Sprite {
                image,
                custom_size: Some(size),
                color: Color::srgba(1.0, 1.0, 1.0, alpha),
                ..default()
            });
    }
    commands.entity(entity).insert(*transform);
}
