use bevy::prelude::*;

use crate::components::Bg;
use crate::resources::DesignScale;
use crate::ui::textbox::ContentRoot;

const DESIGN_W: f32 = 2560.0;
const DESIGN_H: f32 = 1440.0;

/// Handle window resize: update Bg transforms, sizes for letterbox, UiScale, and ContentRoot position.
pub fn on_resize(
    mut bg_query: Query<(&mut Transform, &mut Sprite), With<Bg>>,
    mut root_query: Query<&mut Node, With<ContentRoot>>,
    window_query: Query<&Window>,
    mut ui_scale: ResMut<UiScale>,
    mut design_scale: ResMut<DesignScale>,
) {
    let window = match window_query.single() {
        Ok(w) => w,
        Err(_) => return,
    };
    let ww = window.width();
    let wh = window.height();
    let sc = (ww / DESIGN_W).min(wh / DESIGN_H);

    // Letterbox offset (centered black bars)
    let ox = (ww - DESIGN_W * sc) * 0.5;
    let oy = (wh - DESIGN_H * sc) * 0.5;

    // Bevy world-space center of the letterbox area
    let cx = ox + DESIGN_W * sc * 0.5 - ww * 0.5;
    let cy = oy + DESIGN_H * sc * 0.5 - wh * 0.5;

    for (mut transform, mut sprite) in bg_query.iter_mut() {
        sprite.custom_size = Some(Vec2::new(DESIGN_W * sc, DESIGN_H * sc));
        transform.translation = Vec3::new(cx, cy, transform.translation.z);
    }

    // Scale entire UI so that 2560x1440 design space maps to the letterbox area.
    ui_scale.0 = sc;
    design_scale.0 = sc;

    // Position ContentRoot at letterbox offset.
    // node.left = Val::Px(v) renders at v * sc physical px, but ox is already
    // in logical px.  Compensate: divide by sc so the rendered position = ox.
    if let Ok(mut node) = root_query.single_mut() {
        node.left = Val::Px(ox / sc);
        node.top = Val::Px(oy / sc);
    }
}
