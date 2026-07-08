use bevy::prelude::*;

/// Marker for background entities
#[derive(Component)]
pub struct Bg;

/// Transition state: old bg fading out, new bg fading in (Phase 2 crossfade)
#[derive(Component)]
#[allow(dead_code)]
pub struct BgTransition {
    pub progress: f32,
}

/// Marker for sprite entities with stable ID for diff-based sync
#[derive(Component)]
pub struct SpriteNode(pub String);

/// Marker for choice button entities (Phase 5)
#[derive(Component)]
#[allow(dead_code)]
pub struct ChoiceItem(pub usize);
