use bevy::prelude::*;

use crate::components::Bg;

const DESIGN_W: f32 = 2560.0;
const DESIGN_H: f32 = 1440.0;

/// Handle window resize: update Bg transforms and sizes for letterbox
pub fn on_resize(
    mut bg_query: Query<(&mut Transform, &mut Sprite), With<Bg>>,
    window_query: Query<&Window>,
) {
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

    for (mut transform, mut sprite) in bg_query.iter_mut() {
        sprite.custom_size = Some(Vec2::new(DESIGN_W * sc, DESIGN_H * sc));
        transform.translation = Vec3::new(cx, cy, transform.translation.z);
    }
}
