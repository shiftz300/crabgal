//! Language-neutral visual-novel model and deterministic state machine.

pub mod config;
pub mod model;
pub mod runtime;

// Compatibility facades keep the stable public API while implementation files
// live under their actual architectural owners.
pub use model::{action, state, types};
pub use runtime::{dissolve, expression, step};

pub use model::ShellEvent;
pub use model::types::*;
pub use model::{
    Action, ActiveParticleEffect, BgmState, CameraShakeState, ChoiceTarget, DialoguePause,
    EffectCue, EffectEvent, EffectState, HostCommandEvent, MenuChoice, MenuState,
    PostProcessAnimation, Program, RestoreError, SayOptions, SceneFrame, State, SystemUiSlot,
    TransformKeyframe, VideoState, VocalCue,
};
pub use runtime::StepResult;
