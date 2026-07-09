use std::collections::HashSet;

use bevy::prelude::*;
use bevy::camera::visibility::RenderLayers;

use crate::components::*;
use crate::resources::*;

const DESIGN_W: f32 = 2560.0;
const DESIGN_H: f32 = 1440.0;

/// Sync character sprites with State.sprites via diff (no flicker).
pub fn sync_sprites(
    state: Res<AppState>,
    cfg: Res<Cfg>,
    asset_server: Res<AssetServer>,
    images: Res<Assets<Image>>,
    mut commands: Commands,
    sprite_query: Query<(Entity, &SpriteNode)>,
    window_query: Query<&Window>,
) {
    let s = state.0.read().unwrap();

    let window = match window_query.single() {
        Ok(w) => w,
        Err(_) => return,
    };
    let ww = window.width();
    let wh = window.height();
    let sc = (ww / DESIGN_W).min(wh / DESIGN_H);
    let ox = (ww - DESIGN_W * sc) * 0.5;
    let oy = (wh - DESIGN_H * sc) * 0.5;

    // Desired sprite IDs
    let desired_ids: HashSet<&String> = s.sprites.keys().collect();

    // Despawn sprites no longer in state
    for (entity, node) in sprite_query.iter() {
        if !desired_ids.contains(&node.0) {
            commands.entity(entity).despawn();
        }
    }

    // Existing sprite entities (id → entity)
    let existing: std::collections::HashMap<&str, Entity> = sprite_query
        .iter()
        .map(|(e, n)| (n.0.as_str(), e))
        .collect();

    // Collect and sort sprites by y (back to front)
    let mut sorted: Vec<(&String, &crabgal_core::state::Sprite)> =
        s.sprites.iter().collect();
    sorted.sort_by(|(_, a), (_, b)| {
        a.position
            .y
            .partial_cmp(&b.position.y)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    for (i, (id, sprite_data)) in sorted.iter().enumerate() {
        if sprite_data.transition_progress <= 0.0 && !sprite_data.entering {
            continue;
        }

        let handle: Handle<Image> = asset_server.load(format!("figure/{}", sprite_data.image));

        let y_offset = if sprite_data.entering {
            (1.0 - sprite_data.transition_progress) * 50.0
        } else {
            sprite_data.transition_progress * 50.0
        };

        let alpha = (sprite_data.transition_progress * sprite_data.transform.alpha).clamp(0.0, 1.0);
        let t = &sprite_data.transform;

        // Design space → Bevy world.
        // position.y = bottom of sprite (feet on ground).
        // Sprite center = position.y + half height.
        let sprite_h_ds = cfg.0.layout.sprite_height;
        let edge_pad = cfg.0.layout.anchor_offset;

        let aspect = images.get(&handle).map_or(0.7, |img| {
            let s = img.size();
            if s.y > 0 { s.x as f32 / s.y as f32 } else { 0.7 }
        });
        let sprite_w_ds = sprite_h_ds * aspect * t.scale_x / t.scale_y;

        // Anchor → design-space sprite CENTER.
        // offset from script; edge_pad from config provides default breathing room.
        use crabgal_core::Anchor;
        let center_x_ds = match sprite_data.position.x {
            Anchor::Left(o)   => o.max(edge_pad) + sprite_w_ds * 0.5,
            Anchor::Center(o) => DESIGN_W * 0.5 + o,
            Anchor::Right(o)  => DESIGN_W - o.max(edge_pad) - sprite_w_ds * 0.5,
        };
        let design_center_y = sprite_data.position.y + sprite_h_ds * 0.5 + y_offset;
        let world_x = ox + (center_x_ds + t.offset_x) * sc - ww * 0.5;
        let world_y = oy + (design_center_y + t.offset_y) * sc - wh * 0.5;
        let z = 0.1 + (i as f32) * 0.01;

        let sprite_h = sprite_h_ds * sc * t.scale_y;
        let sprite_w = sprite_w_ds * sc;
        let rotation = t.rotation;

        if let Some(&entity) = existing.get(id.as_str()) {
            commands.entity(entity).insert((
                Sprite {
                    image: handle.clone(),
                    custom_size: Some(Vec2::new(sprite_w, sprite_h)),
                    color: Color::srgba(1.0, 1.0, 1.0, alpha),
                    ..default()
                },
                Transform::from_xyz(world_x, world_y, z).with_rotation(Quat::from_rotation_z(rotation)),
                RenderLayers::layer(0),
            ));
        } else {
            commands.spawn((
                Sprite {
                    image: handle,
                    custom_size: Some(Vec2::new(sprite_w, sprite_h)),
                    color: Color::srgba(1.0, 1.0, 1.0, alpha),
                    ..default()
                },
                Transform::from_xyz(world_x, world_y, z).with_rotation(Quat::from_rotation_z(rotation)),
                SpriteNode((*id).clone()),
                RenderLayers::layer(0),
            ));
        }
    }
}

