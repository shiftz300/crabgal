use bevy::prelude::*;
use crabgal_core::{DESIGN_HEIGHT, DESIGN_WIDTH};

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
}
