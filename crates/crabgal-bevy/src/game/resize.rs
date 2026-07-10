use crate::ui::textbox::ContentRoot;
use crate::viewport::DesignViewport;
use bevy::prelude::*;

/// Keeps the fixed design canvas centered inside the window letterbox.
pub fn on_resize(
    mut content_root: Query<&mut Node, With<ContentRoot>>,
    window_query: Query<&Window>,
    mut ui_scale: ResMut<UiScale>,
) {
    let Ok(window) = window_query.single() else {
        return;
    };
    let viewport = DesignViewport::from_window(window);

    ui_scale.0 = viewport.scale;
    if let Ok(mut node) = content_root.single_mut() {
        node.left = Val::Px(viewport.offset.x / viewport.scale);
        node.top = Val::Px(viewport.offset.y / viewport.scale);
    }
}
