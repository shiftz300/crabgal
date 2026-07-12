use bevy::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BackgroundLayer {
    Previous,
    Current,
}

#[derive(Component)]
pub struct BackgroundNode {
    pub layer: BackgroundLayer,
    pub image: String,
}

/// Marker for sprite entities with stable ID for diff-based sync
#[derive(Component)]
pub struct SpriteNode(pub String);
