use bevy::prelude::*;

use crabgal_core::dissolve;

use crate::components::*;
use crate::resources::*;

const DESIGN_W: f32 = 2560.0;
const DESIGN_H: f32 = 1440.0;

/// Sync background sprites with State.
/// Bevy Camera2d has (0,0) at screen center, y-up.
/// We convert letterbox offset + design coords to Bevy world space.
pub fn sync_bg(
    state: Res<AppState>,
    texture_map: Res<TextureMap>,
    mut commands: Commands,
    bg_query: Query<Entity, With<Bg>>,
    window_query: Query<&Window>,
) {
    // Despawn existing BGs
    for entity in bg_query.iter() {
        commands.entity(entity).despawn();
    }

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

    // Bevy world-space center of the letterbox area
    let cx = ox + DESIGN_W * sc * 0.5 - ww * 0.5;
    let cy = oy + DESIGN_H * sc * 0.5 - wh * 0.5;

    let spawn_bg = |commands: &mut Commands, name: &str, alpha: f32, z: f32| {
        let info = texture_map.bg.iter().find_map(|(n, t)| {
            let stem = std::path::Path::new(n)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(n);
            if name.contains(stem) || stem.contains(name) {
                Some(t.clone())
            } else {
                None
            }
        });
        if let Some(info) = info {
            commands.spawn((
                Sprite {
                    image: info.handle,
                    custom_size: Some(Vec2::new(DESIGN_W * sc, DESIGN_H * sc)),
                    color: Color::srgba(1.0, 1.0, 1.0, alpha),
                    ..default()
                },
                Transform::from_xyz(cx, cy, z),
                Bg,
            ));
        }
    };

    if let Some(ref t) = s.bg_transition {
        if let Some(ref from) = t.from {
            spawn_bg(&mut commands, from, 1.0, -1.0);
        }
        let eased = dissolve::smooth_fade(t.progress);
        spawn_bg(&mut commands, &t.to, eased, 0.0);
    } else if let Some(ref bg) = s.bg {
        spawn_bg(&mut commands, bg, 1.0, 0.0);
    }
}

