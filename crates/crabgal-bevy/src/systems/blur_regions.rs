// Blur regions integration — wraps bevy_blur_regions_fork for our textbox use case.
use bevy::prelude::*;
use bevy_blur_regions_fork::prelude::*;
use crabgal_core::Anchor;
use crate::resources::AppState;
use crate::systems::textbox::NameBarRoot;

/// System: push textbox rectangle into the blur regions camera each frame.
/// Mirrors the layout logic in textbox.rs.
pub fn update_blur_regions(
    state: Res<AppState>,
    window_query: Query<&Window>,
    name_bar_q: Query<&ComputedNode, With<NameBarRoot>>,
    mut blur_q: Query<&mut DefaultBlurRegionsCamera>,
) {
    let Ok(mut bc) = blur_q.get_single_mut() else { return };
    let s = state.0.read().unwrap();
    let w = window_query.single();

    // Same dodge calculation as textbox.rs::update_textbox
    let sc = (w.width() / 2560.0).min(w.height() / 1440.0);
    let mut dodge: f32 = 0.0;
    if s.mini_avatar.is_some() {
        dodge = s.mini_avatar_progress * 200.0 * sc;
    }
    for (_, sp) in s.sprites.iter() {
        if sp.entering {
            if let Anchor::Left(_) = sp.position.x {
                dodge = dodge.max(sp.transition_progress * 200.0 * sc);
            }
        }
    }

    let sf = w.scale_factor() as f32;
    let left = w.width() * 0.07 + dodge;
    let right = left + w.width() * 0.86; // left(7%) + width(86%) = 93%

    let l = left * sf;
    let r = right * sf;

    // Name bar: bottom 22%, width/height from ComputedNode (fits actual text + padding)
    if let Ok(node) = name_bar_q.get_single() {
        let nb_h = node.size().y; // physical px, from text + padding
        let nb_w = node.size().x;
        let nb_b = w.height() * (1.0 - 0.22) * sf;
        bc.blur(Rect::new(l, nb_b - nb_h, l + nb_w, nb_b));
    }

    // Dialogue: bottom 3%, height 18%
    let tb_t = w.height() * (1.0 - 0.03 - 0.18) * sf;
    let tb_b = w.height() * (1.0 - 0.03) * sf;
    bc.blur(Rect::new(l, tb_t, r, tb_b));
}
