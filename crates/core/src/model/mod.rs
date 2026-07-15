//! Serializable, language-neutral visual-novel model.

pub mod action;
pub mod state;
pub mod types;

pub use action::{Action, ChoiceTarget, Program, SayOptions};
pub use state::{
    BgmState, EffectCue, EffectEvent, EffectState, MenuChoice, MenuState, RestoreError, SceneFrame,
    State,
};
pub use types::*;
