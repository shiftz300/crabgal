use bevy::camera::Viewport;
use bevy::prelude::*;
use crabgal_core::{DESIGN_HEIGHT, DESIGN_WIDTH};

use crate::render::blur::{DialogCamera, UiBlurCamera};
use crate::ui::textbox::{ContentRoot, QuickPreviewLayer};

type UiCameraFilter = Or<(With<UiBlurCamera>, With<DialogCamera>)>;

#[derive(Debug, Clone, Copy)]
pub struct DesignViewport {
    pub scale: f32,
    pub offset: Vec2,
    pub window_size: Vec2,
}

impl DesignViewport {
    pub fn from_window(window: &Window) -> Self {
        let window_size = Vec2::new(window.width(), window.height());
        let scale = (window_size.x / DESIGN_WIDTH)
            .min(window_size.y / DESIGN_HEIGHT)
            .max(f32::EPSILON);
        let content_size = Vec2::new(DESIGN_WIDTH, DESIGN_HEIGHT) * scale;

        Self {
            scale,
            offset: (window_size - content_size) * 0.5,
            window_size,
        }
    }

    pub fn world_from_design(self, point: Vec2) -> Vec2 {
        self.offset + point * self.scale - self.window_size * 0.5
    }

    pub fn content_center(self) -> Vec2 {
        self.world_from_design(Vec2::new(DESIGN_WIDTH, DESIGN_HEIGHT) * 0.5)
    }

    pub fn camera_viewport(self, window: &Window) -> Viewport {
        let scale_factor = window.scale_factor();
        let position = (self.offset * scale_factor).round().as_uvec2();
        let size = (Vec2::new(DESIGN_WIDTH, DESIGN_HEIGHT) * self.scale * scale_factor)
            .round()
            .as_uvec2()
            .max(UVec2::ONE);
        Viewport {
            physical_position: position,
            physical_size: size,
            ..default()
        }
    }
}

/// Keeps the fixed design canvas centered inside the window letterbox.
pub(crate) fn on_resize(
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

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::window::WindowResolution;

    #[test]
    fn wide_window_centers_a_sixteen_by_nine_camera_viewport() {
        let window = Window {
            resolution: WindowResolution::new(2560, 1080),
            ..default()
        };
        let design = DesignViewport::from_window(&window);
        let camera = design.camera_viewport(&window);

        assert_eq!(design.offset, Vec2::new(320.0, 0.0));
        assert_eq!(camera.physical_position, UVec2::new(320, 0));
        assert_eq!(camera.physical_size, UVec2::new(1920, 1080));
    }

    #[test]
    fn tall_window_centers_a_sixteen_by_nine_camera_viewport() {
        let window = Window {
            resolution: WindowResolution::new(1280, 1024),
            ..default()
        };
        let design = DesignViewport::from_window(&window);
        let camera = design.camera_viewport(&window);

        assert_eq!(design.offset, Vec2::new(0.0, 152.0));
        assert_eq!(camera.physical_position, UVec2::new(0, 152));
        assert_eq!(camera.physical_size, UVec2::new(1280, 720));
    }
}
