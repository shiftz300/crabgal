// crabgal-core: Visual novel engine core types and state machine.
//
// Architecture (best of all worlds):
//   Siglus   → Stage/Layer model for render ordering
//   Ren'Py   → Displayable(st,at) for animation timing
//   WebGAL   → Simple command DSL (not bytecode VM)
//   Ayaka    → Event-driven step() like next_run()
//
// Design principles:
//   1. Single State struct — no ECS until needed
//   2. step() executes until interactive point
//   3. All state is bincode-serializable (for save/rollback later)
//   4. Fixed design resolution with viewport scaling

// crabgal-core: state, action types, step engine, UI panel state.

pub mod action;
pub mod config;
pub mod dissolve;
pub mod state;
pub mod step;
pub mod types;

pub use action::Action;
pub use state::State;
pub use step::StepResult;
pub use types::*;
