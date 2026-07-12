//! Serializable, language-neutral visual-novel model.

pub mod action;
pub mod state;
pub mod types;

pub use action::{Action, ChoiceTarget, SayOptions};
pub use state::{
    BgmState, EffectCue, EffectEvent, EffectState, MenuChoice, MenuState, SceneFrame, State,
};
pub use types::*;
