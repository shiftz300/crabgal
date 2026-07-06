// Shared types for the VN engine.

use serde::{Deserialize, Serialize};

/// Design resolution (fixed, everything is drawn in this space).
pub const DESIGN_WIDTH: f32 = 1600.0;
pub const DESIGN_HEIGHT: f32 = 900.0;

/// Position anchors, inspired by WebGAL's -left/-right/-center system.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Anchor {
    /// Offset from left edge (in design pixels).
    Left(f32),
    /// Offset from right edge.
    Right(f32),
    /// Centered horizontally with optional offset.
    Center(f32),
}

impl Anchor {
    /// Resolve anchor to an absolute x position within the design width.
    pub fn resolve(self, object_width: f32) -> f32 {
        match self {
            Anchor::Left(offset) => offset,
            Anchor::Right(offset) => DESIGN_WIDTH - object_width - offset,
            Anchor::Center(offset) => (DESIGN_WIDTH - object_width) / 2.0 + offset,
        }
    }
}

/// A 2D position in design-space coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Position {
    pub x: Anchor,
    pub y: f32,
}

impl Position {
    pub fn left(y: f32) -> Self {
        Self { x: Anchor::Left(0.0), y }
    }
    pub fn right(y: f32) -> Self {
        Self { x: Anchor::Right(0.0), y }
    }
    pub fn center(y: f32) -> Self {
        Self { x: Anchor::Center(0.0), y }
    }
}

/// Transition / enter animation kind.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub enum Transition {
    /// Instant (no animation).
    #[default]
    Instant,
    /// Fade in/out over `duration` seconds.
    Fade(f32),
    /// Slide from left over `duration` seconds.
    SlideFromLeft(f32),
    /// Slide from right over `duration` seconds.
    SlideFromRight(f32),
    /// Crossfade between scenes over `duration` seconds.
    Crossfade(f32),
}

/// A color with alpha.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rgba {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Rgba {
    pub const WHITE: Self = Self { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
    pub const BLACK: Self = Self { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self { Self { r, g, b, a } }
}

/// Game variable value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Value {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
}
