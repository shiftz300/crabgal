//! Language-neutral visual-novel model and deterministic state machine.

pub mod config;
pub mod model;
pub mod runtime;

// Compatibility facades keep the stable public API while implementation files
// live under their actual architectural owners.
pub use model::{action, state, types};
pub use runtime::{dissolve, expression, step};

pub use model::types::*;
pub use model::{Action, ChoiceTarget, MenuChoice, MenuState, SayOptions, SceneFrame, State};
pub use runtime::StepResult;
