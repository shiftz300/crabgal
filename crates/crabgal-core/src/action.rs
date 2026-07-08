// Action enum: the script DSL compiles into a Vec<Action>.
//
// Inspired by:
//   WebGAL's command DSL (simplicity)
//   Siglus's command dispatch (structure)

use serde::{Deserialize, Serialize};

use crate::types::{Position, Transition, Value};

/// A single script action — the entire script language compiles to this.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Action {
    // ── Display ──
    /// Set the background image.
    ShowBg {
        image: String,
        transition: Transition,
    },
    /// Show a sprite (character / effect).
    ShowSprite {
        id: String,
        image: String,
        position: Position,
        transition: Transition,
    },
    /// Remove a sprite.
    HideSprite {
        id: String,
        transition: Transition,
    },

    // ── Dialogue ──
    /// Display dialogue text (triggers click-to-continue).
    Say {
        speaker: String,
        text: String,
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

    // ── Audio (stub for now) ──
    /// Play background music.
    Bgm {
        file: String,
        volume: f32,
    },
    /// Stop background music.
    StopBgm,

    // ── UI ──
    /// Show mini avatar beside the text box.
    MiniAvatar { image: String },
    /// Hide the mini avatar.
    HideMiniAvatar,
    /// Set a variable.
    Set {
        name: String,
        value: Value,
    },
}

/// A single choice in a menu.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Choice {
    pub text: String,
    pub target: String,  // label name to jump to
}
