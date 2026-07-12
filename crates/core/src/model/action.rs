// Action enum: the script DSL compiles into a Vec<Action>.
//
// Inspired by:
//   WebGAL's command DSL (simplicity)
//   Siglus's command dispatch (structure)

use serde::{Deserialize, Serialize};

use crate::types::{BlendMode, Easing, Position, SpriteTransform, Transition};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SayOptions {
    pub vocal: Option<String>,
    pub volume: f32,
    pub concat: bool,
    pub auto_advance: bool,
    pub inherit_speaker: bool,
}

impl Default for SayOptions {
    fn default() -> Self {
        Self {
            vocal: None,
            volume: 1.0,
            concat: false,
            auto_advance: false,
            inherit_speaker: false,
        }
    }
}

/// A single script action — the entire script language compiles to this.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Action {
    // ── Display ──
    /// Set the background image.
    ShowBg {
        image: String,
        transition: Transition,
        transform: SpriteTransform,
    },
    /// Remove the current background.
    HideBg { transition: Transition },
    /// Show a sprite (character / effect).
    ShowSprite {
        id: String,
        image: String,
        position: Position,
        transition: Transition,
        transform: SpriteTransform,
        z_index: i32,
        blend: BlendMode,
    },
    /// Remove a sprite.
    HideSprite { id: String, transition: Transition },

    // ── Dialogue ──
    /// Display dialogue text (triggers click-to-continue).
    Say {
        speaker: String,
        text: String,
        options: SayOptions,
    },

    // ── Choice ──
    /// Show a choice menu (triggers click-to-choose).
    Menu {
        prompt: String,
        choices: Vec<Choice>,
    },

    // ── Control flow ──
    /// Jump to a named label.
    Jump(String),
    /// Define a label (no-op in execution, used for jump resolution).
    Label(String),
    /// Replace the current scene without adding a return point.
    ChangeScene(String),
    /// Enter a scene and return to the following action when it finishes.
    CallScene(String),
    /// End the current game flow without returning through the scene stack.
    End,

    // ── Audio (stub for now) ──
    /// Play background music.
    Bgm { file: String, volume: f32 },
    /// Stop background music.
    StopBgm,

    // ── UI ──
    /// Show mini avatar beside the text box.
    MiniAvatar { image: String },
    /// Hide the mini avatar.
    HideMiniAvatar,
    /// Evaluate an expression and set a local or persistent variable.
    Set {
        name: String,
        expression: String,
        global: bool,
    },

    /// Common WebGAL flow parameters applied to any command.
    Flow {
        action: Box<Action>,
        when: Option<String>,
        next: bool,
    },

    // ── Transform ──
    /// Modify an existing sprite's transform (position offset, alpha, scale, etc).
    SetTransform {
        /// Target sprite id (e.g. "fig-center", "fig-left", or custom id).
        id: String,
        /// Transform fields to apply (partial — only non-default fields have effect).
        transform: SpriteTransform,
        duration: f32,
        easing: Easing,
    },
}

/// A single choice in a menu.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Choice {
    pub text: String,
    pub target: ChoiceTarget,
    pub show_when: Option<String>,
    pub enable_when: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ChoiceTarget {
    Label(String),
    ChangeScene(String),
    CallScene(String),
}
