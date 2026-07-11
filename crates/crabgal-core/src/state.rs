// Central game state — the single source of truth.
//
// No ECS. No VM. Just a struct.
// All fields are bincode-serializable (save/rollback ready).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::Action;
use crate::action::ChoiceTarget;
use crate::types::{BlendMode, Easing, Position, SpriteTransform, Transition, Value};

/// The complete game state at any point in time.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct State {
    // ── Script execution ──
    /// All parsed actions across all scenes, keyed by scene name.
    pub scenes: HashMap<String, Vec<Action>>,
    /// Current scene being executed.
    pub current_scene: String,
    /// Index into the current scene's action list.
    pub cursor: usize,
    /// Suspended callers, most recent frame last.
    pub scene_stack: Vec<SceneFrame>,
    /// Explicit `end` reached; the presentation layer should show the title.
    pub ended: bool,

    // ── Display state ──
    /// Current background image path.
    pub bg: Option<String>,
    /// Current background transition in progress.
    pub bg_transition: Option<BgTransition>,
    pub bg_transform: SpriteTransform,
    pub bg_transform_animation: Option<TransformAnimation>,
    /// Active sprites, keyed by id.
    pub sprites: HashMap<String, Sprite>,
    /// Current dialogue text (if any).
    pub dialogue: Option<Dialogue>,
    /// Last settled dialogue, used by WebGAL `-concat`.
    pub previous_dialogue: Option<Dialogue>,
    /// Mini avatar image path (displayed beside text box).
    pub mini_avatar: Option<String>,
    /// Mini avatar enter/exit transition progress (0→1).
    pub mini_avatar_progress: f32,

    // ── Choice state ──
    /// Active choice menu (if any).
    pub menu: Option<MenuState>,

    // ── Variables ──
    /// Game variables (set by scripts).
    pub vars: HashMap<String, Value>,
    /// Persistent variables requested with WebGAL's `-global` flag.
    pub global_vars: HashMap<String, Value>,

    // ── Scene labels ──
    /// Label → action index mapping for the current scene.
    pub labels: HashMap<String, usize>,
}

/// Return point saved by `callScene`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneFrame {
    pub scene: String,
    pub cursor: usize,
}

/// Choice menu currently blocking script execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MenuState {
    pub prompt: String,
    pub choices: Vec<MenuChoice>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MenuChoice {
    pub text: String,
    pub target: ChoiceTarget,
    pub enabled: bool,
}

/// A sprite displayed on screen.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    /// Horizontal offset at the start of a slide transition.
    ///
    /// This field keeps its original serialization position for save compatibility.
    pub transition_offset_x: f32,
    /// Transform applied on top of base position (offset, alpha, scale, etc).
    #[serde(default)]
    pub transform: SpriteTransform,
    pub transform_animation: Option<TransformAnimation>,
    pub z_index: i32,
    pub blend: BlendMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TransformAnimation {
    pub from: SpriteTransform,
    pub to: SpriteTransform,
    pub elapsed: f32,
    pub duration: f32,
    pub easing: Easing,
}

/// Background transition state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Dialogue {
    /// Speaker name.
    pub speaker: String,
    /// Full text content.
    pub text: String,
    /// Number of visible characters (typewriter effect).
    pub visible_chars: usize,
    pub vocal: Option<String>,
    pub volume: f32,
    pub auto_advance: bool,
}

impl State {
    pub fn new() -> Self {
        Self::default()
    }
}
