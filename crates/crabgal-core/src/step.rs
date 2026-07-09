// Step engine — executes Actions until an interactive point.
//
// Inspired by Ayaka's next_run() and Siglus's step_until_yield().
// The key insight: VN execution pauses at user interaction points (click/choice),
// NOT at every frame. This is how we achieve "frame rate insensitive" execution.

use log::debug;

use crate::action::Action;
use crate::state::{BgTransition, Dialogue, Sprite, State};
use crate::types::{Anchor, SpriteTransform, Transition};

/// Result of a step() call.
#[derive(Debug, Clone, PartialEq)]
pub enum StepResult {
    /// More actions executed, keep going.
    Continue,
    /// Engine is waiting for the user to click (dialogue shown).
    AwaitClick,
    /// Engine is waiting for the user to choose (menu shown).
    AwaitChoice,
    /// No more actions in this scene.
    EndOfScene,
}

/// Execute actions from the current cursor position until we hit
/// an interactive point (Say or Menu) or end of scene.
pub fn step(state: &mut State) -> StepResult {
    let scene = state
        .scenes
        .get(&state.current_scene)
        .cloned()
        .unwrap_or_default();

    loop {
        if state.cursor >= scene.len() {
            return StepResult::EndOfScene;
        }

        let action = &scene[state.cursor];
        state.cursor += 1;

        match action {
            Action::ShowBg { image, transition } => {
                debug!("ShowBg: {} ({:?})", image, transition);
                let from = state.bg.take();
                state.bg = Some(image.clone());
                state.bg_transition = Some(BgTransition {
                    from,
                    to: image.clone(),
                    progress: 0.0,
                    kind: *transition,
                });
            }
            Action::SetTransform { id, transform: t } => {
                debug!("SetTransform: {} -> {:?}", id, t);
                if let Some(sprite) = state.sprites.get_mut(id) {
                    sprite.transform = *t;
                }
            }
            Action::ShowSprite { id, image, position, transition } => {
                debug!("ShowSprite: {} {} at {:?}", id, image, position);
                let x_offset = match (position.x, transition) {
                    (Anchor::Left(_), Transition::SlideFromLeft(_)) => -400.0,
                    (Anchor::Right(_), Transition::SlideFromRight(_)) => 400.0,
                    _ => 0.0,
                };
                state.sprites.insert(id.clone(), Sprite {
                    image: image.clone(),
                    position: *position,
                    transition_progress: 0.0,
                    transition: *transition,
                    entering: true,
                    y_offset: x_offset,
                    transform: SpriteTransform::default(),
                });
            }
            Action::HideSprite { id, transition } => {
                debug!("HideSprite: {}", id);
                if let Some(sprite) = state.sprites.get_mut(id) {
                    sprite.transition = *transition;
                    sprite.entering = false;
                    sprite.transition_progress = 0.0;
                }
            }
            Action::Say { speaker, text } => {
                debug!("Say: {}: {}", speaker, text);
                state.dialogue = Some(Dialogue {
                    speaker: speaker.clone(),
                    text: text.clone(),
                    visible_chars: 0,
                });
                state.menu = None;
                return StepResult::AwaitClick;
            }
            Action::Menu { prompt: _, choices } => {
                debug!("Menu: {} choices", choices.len());
                state.menu = Some(choices.clone());
                state.dialogue = None;
                return StepResult::AwaitChoice;
            }
            Action::Jump(label) => {
                if let Some(&idx) = state.labels.get(label) {
                    debug!("Jump: {} -> {}", label, idx);
                    state.cursor = idx;
                } else {
                    debug!("Jump: {} (label not found, skipping)", label);
                }
            }
            Action::Label(name) => {
                // Labels are pre-indexed; no-op during execution.
                debug!("Label: {}", name);
            }
            Action::Bgm { file, volume } => {
                debug!("Bgm: {} (vol {})", file, volume);
                // Audio stubbed — will be wired to rodio later.
            }
            Action::StopBgm => {
                debug!("StopBgm");
            }
            Action::Set { name, value } => {
                debug!("Set: {} = {:?}", name, value);
                state.vars.insert(name.clone(), value.clone());
            }
            Action::MiniAvatar { image } => {
                debug!("MiniAvatar: {}", image);
                state.mini_avatar = Some(image.clone());
                state.mini_avatar_progress = 0.0;
            }
            Action::HideMiniAvatar => {
                debug!("HideMiniAvatar");
                state.mini_avatar = None;
                state.mini_avatar_progress = 0.0;
            }
        }
    }
}

/// Handle user clicking to advance past a dialogue.
pub fn advance(state: &mut State) {
    state.dialogue = None;
}

/// Handle user selecting a menu choice.
pub fn select_choice(state: &mut State, index: usize) {
    if let Some(choices) = &state.menu {
        if let Some(choice) = choices.get(index) {
            let target = choice.target.clone();
            state.menu = None;
            // Jump to the chosen label
            if let Some(&idx) = state.labels.get(&target) {
                state.cursor = idx;
            }
        }
    }
}

/// Re-index labels for the current scene (called after script changes).
pub fn index_labels(state: &mut State) {
    state.labels.clear();
    let scene = state
        .scenes
        .get(&state.current_scene)
        .cloned()
        .unwrap_or_default();
    for (i, action) in scene.iter().enumerate() {
        if let Action::Label(name) = action {
            state.labels.insert(name.clone(), i);
        }
    }
}
