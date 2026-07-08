use std::collections::HashSet;

use bevy::prelude::*;

use crate::components::*;
use crate::resources::*;

const DESIGN_W: f32 = 2560.0;
const DESIGN_H: f32 = 1440.0;

/// Sync character sprites with State.sprites via diff (no flicker).
pub fn sync_sprites(
    state: Res<AppState>,
    texture_map: Res<TextureMap>,
    mut commands: Commands,
    sprite_query: Query<(Entity, &SpriteNode)>,
    window_query: Query<&Window>,
) {
    let s = state.0.read().unwrap();

    let window = match window_query.get_single() {
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

        let info = texture_map
            .sprites
            .iter()
            .find_map(|(name, t)| {
                let stem = std::path::Path::new(name)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or(name);
                if sprite_data.image.contains(stem) || stem.contains(sprite_data.image.as_str()) {
                    Some(t.clone())
                } else {
                    None
                }
            });

        let Some(info) = info else {
            continue;
        };

        let y_offset = if sprite_data.entering {
            (1.0 - sprite_data.transition_progress) * 50.0
        } else {
            sprite_data.transition_progress * 50.0
        };

        let alpha = sprite_data.transition_progress;

        // Design space → Bevy world.
        // position.y = bottom of sprite (feet on ground).
        // Sprite center = position.y + half height.
        let sprite_h_ds = 960.0; // design-space sprite height

        // Anchor → natural screen position (VN convention: left/center/right)
        use crabgal_core::Anchor;
        let x = match sprite_data.position.x {
            Anchor::Left(offset) => DESIGN_W * 0.06 + offset,
            Anchor::Center(offset) => DESIGN_W * 0.50 + offset,
            Anchor::Right(offset) => DESIGN_W * 0.85 + offset,
        };
        let design_center_y = sprite_data.position.y + sprite_h_ds * 0.5 + y_offset;
        let world_x = ox + x * sc - ww * 0.5;
        let world_y = oy + design_center_y * sc - wh * 0.5;
        let z = 0.1 + (i as f32) * 0.01;

        let sprite_h = 960.0 * sc;
        let aspect = if info.width > 0 && info.height > 0 {
            info.width as f32 / info.height as f32
        } else {
            0.7 // fallback while loading
        };
        let sprite_w = sprite_h * aspect;

        if let Some(&entity) = existing.get(id.as_str()) {
            commands.entity(entity).insert((
                Sprite {
                    image: info.handle.clone(),
                    custom_size: Some(Vec2::new(sprite_w, sprite_h)),
                    color: Color::srgba(1.0, 1.0, 1.0, alpha),
                    ..default()
                },
                Transform::from_xyz(world_x, world_y, z),
            ));
        } else {
            commands.spawn((
                Sprite {
                    image: info.handle,
                    custom_size: Some(Vec2::new(sprite_w, sprite_h)),
                    color: Color::srgba(1.0, 1.0, 1.0, alpha),
                    ..default()
                },
                Transform::from_xyz(world_x, world_y, z),
                SpriteNode((*id).clone()),
            ));
        }
    }
}

/// Fill in TexInfo dimensions from loaded assets (width/height initially 0).
pub fn update_tex_dims(
    images: Res<Assets<Image>>,
    mut texture_map: ResMut<TextureMap>,
) {
    for (_, info) in texture_map.bg.iter_mut() {
        if info.width == 0 {
            if let Some(img) = images.get(&info.handle) {
                info.width = img.size().x as u32;
                info.height = img.size().y as u32;
            }
        }
    }
    for (_, info) in texture_map.sprites.iter_mut() {
        if info.width == 0 {
            if let Some(img) = images.get(&info.handle) {
                info.width = img.size().x as u32;
                info.height = img.size().y as u32;
            }
        }
    }
}

