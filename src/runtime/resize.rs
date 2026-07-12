use crate::render::blur::{DialogCamera, UiBlurCamera};
use crate::runtime::viewport::DesignViewport;
use crate::ui::textbox::{ContentRoot, QuickPreviewLayer};
use bevy::prelude::*;

type UiCameraFilter = Or<(With<UiBlurCamera>, With<DialogCamera>)>;

/// Keeps the fixed design canvas centered inside the window letterbox.
pub fn on_resize(
    mut content_root: Query<&mut Node, (With<ContentRoot>, Without<QuickPreviewLayer>)>,
    mut quick_preview_layer: Query<&mut Node, (With<QuickPreviewLayer>, Without<ContentRoot>)>,
    window_query: Query<&Window>,
    mut ui_cameras: Query<&mut Camera, UiCameraFilter>,
    mut ui_scale: ResMut<UiScale>,
) {
    let Ok(window) = window_query.single() else {
        return;
    };
    let viewport = DesignViewport::from_window(window);

    ui_scale.0 = viewport.scale;
    for mut camera in &mut ui_cameras {
        camera.viewport = Some(viewport.camera_viewport(window));
    }
    if let Ok(mut node) = content_root.single_mut() {
        node.left = Val::ZERO;
        node.top = Val::ZERO;
    }
    if let Ok(mut node) = quick_preview_layer.single_mut() {
        node.left = Val::ZERO;
        node.top = Val::ZERO;
    }
}
