//! Serializable, language-neutral visual-novel model.

pub mod action;
pub mod state;
pub mod types;

pub use action::{Action, ChoiceTarget, Program, SayOptions, SystemUiSlot, TransformKeyframe};
pub use state::{
    ActiveParticleEffect, BgmState, CameraShakeState, DialoguePause, EffectCue, EffectEvent,
    EffectState, HostCommandEvent, MenuChoice, MenuState, PostProcessAnimation, RestoreError,
    SceneFrame, ShellEvent, State, VideoState, VocalCue,
};
pub use types::*;
