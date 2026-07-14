use bevy::camera::visibility::RenderLayers;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use crabgal_core::SpriteTransform;
use crabgal_core::dissolve;
use crabgal_core::{DESIGN_HEIGHT, DESIGN_WIDTH};

use crate::runtime::resources::GameState;
use crate::runtime::viewport::DesignViewport;
use crate::scene::components::{BackgroundLayer, BackgroundNode};
use crate::scene::effects::material::{StageMaterial, StageQuad, animation_uniform};

#[derive(Default)]
pub(crate) struct BackgroundRenderCache {
    initialized: bool,
    image: Option<String>,
    transition: Option<crabgal_core::state::BgTransition>,
    transform: SpriteTransform,
    filter: crabgal_core::VisualFilter,
    animation: Option<crabgal_core::state::PresetAnimation>,
}

impl BackgroundRenderCache {
    fn matches(&self, state: &GameState) -> bool {
        self.initialized
            && self.image == state.bg
            && self.transition == state.bg_transition
            && self.transform == state.bg_transform
            && self.filter == state.bg_filter
            && self.animation == state.bg_animation
    }

    fn capture(&mut self, state: &GameState) {
        self.initialized = true;
        self.image.clone_from(&state.bg);
        self.transition.clone_from(&state.bg_transition);
        self.transform = state.bg_transform;
        self.filter = state.bg_filter;
        self.animation.clone_from(&state.bg_animation);
    }
}

struct DesiredBackground<'a> {
    layer: BackgroundLayer,
    image: &'a str,
    alpha: f32,
    z: f32,
    transform: SpriteTransform,
    transition: Vec4,
}

type BackgroundQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static mut BackgroundNode,
        &'static mut Transform,
        Option<&'static MeshMaterial2d<StageMaterial>>,
        Option<&'static mut Sprite>,
    ),
>;

#[derive(SystemParam)]
pub(crate) struct BackgroundSyncContext<'w, 's> {
    state: Res<'w, GameState>,
    asset_server: Res<'w, AssetServer>,
    commands: Commands<'w, 's>,
    backgrounds: BackgroundQuery<'w, 's>,
    quad: Res<'w, StageQuad>,
    materials: ResMut<'w, Assets<StageMaterial>>,
    windows: Query<'w, 's, Ref<'static, Window>>,
    cache: Local<'s, BackgroundRenderCache>,
}

/// Synchronizes background entities without recreating them every frame.
pub fn sync_bg(context: BackgroundSyncContext) {
    let BackgroundSyncContext {
        state,
        asset_server,
        mut commands,
        mut backgrounds,
        quad,
        mut materials,
        windows,
        mut cache,
    } = context;
    let Ok(window) = windows.single() else {
        return;
    };
    if cache.matches(&state) && !window.is_changed() {
        return;
    }
    cache.capture(&state);
    let viewport = DesignViewport::from_window(&window);
    let desired = desired_backgrounds(&state);
    let mut previous_exists = false;
    let mut current_exists = false;

    for (entity, mut node, mut transform, existing_material, existing_sprite) in &mut backgrounds {
        let Some(background) = desired.iter().find(|item| item.layer == node.layer) else {
            commands.entity(entity).despawn();
            continue;
        };

        match node.layer {
            BackgroundLayer::Previous => previous_exists = true,
            BackgroundLayer::Current => current_exists = true,
        }
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
            existing_sprite,
            true,
        );
    }

    for background in desired {
        let exists = match background.layer {
            BackgroundLayer::Previous => previous_exists,
            BackgroundLayer::Current => current_exists,
        };
        if exists {
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
            None,
            false,
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
    existing_sprite: Option<Mut<'_, Sprite>>,
    existing_entity: bool,
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
        if existing_material.is_none() {
            commands
                .entity(entity)
                .remove::<Sprite>()
                .insert((Mesh2d(quad.0.clone()), MeshMaterial2d(material_handle)));
        }
    } else {
        transform.scale = Vec3::ONE;
        let desired = Sprite {
            image,
            custom_size: Some(size),
            color: Color::srgba(1.0, 1.0, 1.0, alpha),
            ..default()
        };
        if let Some(mut sprite) = existing_sprite {
            *sprite = desired;
        } else {
            commands
                .entity(entity)
                .remove::<Mesh2d>()
                .remove::<MeshMaterial2d<StageMaterial>>()
                .insert(desired);
        }
    }
    if !existing_entity {
        commands.entity(entity).insert(*transform);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dialogue_changes_do_not_invalidate_the_background() {
        let mut state = crabgal_core::State::new();
        state.bg = Some("scene.webp".into());
        let mut cache = BackgroundRenderCache::default();
        cache.capture(&GameState(state.clone()));

        state.dialogue = Some(crabgal_core::state::Dialogue {
            speaker: "A".into(),
            text: "hello".into(),
            markup: "hello".into(),
            visible_chars: 1,
            vocal: None,
            volume: 1.0,
            auto_advance: false,
        });
        assert!(cache.matches(&GameState(state.clone())));
        state.bg_transform.offset_x = 10.0;
        assert!(!cache.matches(&GameState(state)));
    }
}
