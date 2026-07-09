// Central game state — the single source of truth.
//
// No ECS. No VM. Just a struct.
// All fields are bincode-serializable (save/rollback ready).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::types::{Position, Transition, Value, SpriteTransform};
use crate::Action;

/// The complete game state at any point in time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    // ── Script execution ──
    /// All parsed actions across all scenes, keyed by scene name.
    pub scenes: HashMap<String, Vec<Action>>,
    /// Current scene being executed.
    pub current_scene: String,
    /// Index into the current scene's action list.
    pub cursor: usize,

    // ── Display state ──
    /// Current background image path.
    pub bg: Option<String>,
    /// Current background transition in progress.
    pub bg_transition: Option<BgTransition>,
    /// Active sprites, keyed by id.
    pub sprites: HashMap<String, Sprite>,
    /// Current dialogue text (if any).
    pub dialogue: Option<Dialogue>,
    /// Mini avatar image path (displayed beside text box).
    pub mini_avatar: Option<String>,
    /// Mini avatar enter/exit transition progress (0→1).
    pub mini_avatar_progress: f32,

    // ── Choice state ──
    /// Active choice menu (if any).
    pub menu: Option<Vec<crate::action::Choice>>,

    // ── Variables ──
    /// Game variables (set by scripts).
    pub vars: HashMap<String, Value>,

    // ── Scene labels ──
    /// Label → action index mapping for the current scene.
    pub labels: HashMap<String, usize>,
}

/// A sprite displayed on screen.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sprite {
    /// Asset path.
    pub image: String,
    /// Design-space position.
    pub position: Position,
    /// Current animation progress (0.0 → 1.0).
    pub transition_progress: f32,
    /// Active enter/exit transition.
    pub transition: Transition,
    /// Whether this sprite is entering (true) or exiting (false).
    pub entering: bool,
    /// Rendered y-offset for vertical positioning.
    pub y_offset: f32,
    /// Transform applied on top of base position (offset, alpha, scale, etc).
    #[serde(default)]
    pub transform: SpriteTransform,
}

/// Background transition state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BgTransition {
    /// Previous background path (for crossfade).
    pub from: Option<String>,
    /// Target background path.
    pub to: String,
    /// Transition progress (0.0 → 1.0).
    pub progress: f32,
    /// Transition kind.
    pub kind: Transition,
}

/// Current dialogue being displayed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dialogue {
    /// Speaker name.
    pub speaker: String,
    /// Full text content.
    pub text: String,
    /// Number of visible characters (typewriter effect).
    pub visible_chars: usize,
}

impl State {
    pub fn new() -> Self {
        Self {
            scenes: HashMap::new(),
            current_scene: String::new(),
            cursor: 0,
            bg: None,
            bg_transition: None,
            sprites: HashMap::new(),
            dialogue: None,
            mini_avatar: None,
            mini_avatar_progress: 0.0,
            menu: None,
            vars: HashMap::new(),
            labels: HashMap::new(),
        }
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}
