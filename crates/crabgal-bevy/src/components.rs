use bevy::prelude::*;

/// Marker for background entities
#[derive(Component)]
pub struct Bg;

/// Marker for sprite entities with stable ID for diff-based sync
#[derive(Component)]
pub struct SpriteNode(pub String);
