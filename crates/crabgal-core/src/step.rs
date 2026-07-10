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
    loop {
        let Some(action) = state
            .scenes
            .get(&state.current_scene)
            .and_then(|scene| scene.get(state.cursor))
            .cloned()
        else {
            return StepResult::EndOfScene;
        };
        state.cursor += 1;

        match action {
            Action::ShowBg { image, transition } => {
                debug!("ShowBg: {} ({:?})", image, transition);
                let from = state.bg.take();
                state.bg = Some(image.clone());
                state.bg_transition = (transition != Transition::Instant).then_some(BgTransition {
                    from,
                    to: image,
                    progress: 0.0,
                    kind: transition,
                });
            }
            Action::SetTransform { id, transform: t } => {
                debug!("SetTransform: {} -> {:?}", id, t);
                if let Some(sprite) = state.sprites.get_mut(&id) {
                    sprite.transform = t;
                }
            }
            Action::ShowSprite {
                id,
                image,
                position,
                transition,
            } => {
                debug!("ShowSprite: {} {} at {:?}", id, image, position);
                let transition_offset_x = match (position.x, transition) {
                    (Anchor::Left(_), Transition::SlideFromLeft(_)) => -400.0,
                    (Anchor::Right(_), Transition::SlideFromRight(_)) => 400.0,
                    _ => 0.0,
                };
                let transition_progress = if transition == Transition::Instant {
                    1.0
                } else {
                    0.0
                };
                state.sprites.insert(
                    id,
                    Sprite {
                        image,
                        position,
                        transition_progress,
                        transition,
                        entering: true,
                        transition_offset_x,
                        transform: SpriteTransform::default(),
                    },
                );
            }
            Action::HideSprite { id, transition } => {
                debug!("HideSprite: {}", id);
                if transition == Transition::Instant {
                    state.sprites.remove(&id);
                } else if let Some(sprite) = state.sprites.get_mut(&id) {
                    sprite.transition = transition;
                    sprite.entering = false;
                    sprite.transition_offset_x = match transition {
                        Transition::SlideFromLeft(_) => -400.0,
                        Transition::SlideFromRight(_) => 400.0,
                        _ => 0.0,
                    };
                }
            }
            Action::Say { speaker, text } => {
                debug!("Say: {}: {}", speaker, text);
                state.dialogue = Some(Dialogue {
                    speaker,
                    text,
                    visible_chars: 0,
                });
                state.menu = None;
                return StepResult::AwaitClick;
            }
            Action::Menu { prompt: _, choices } => {
                debug!("Menu: {} choices", choices.len());
                state.menu = Some(choices);
                state.dialogue = None;
                return StepResult::AwaitChoice;
            }
            Action::Jump(label) => {
                if let Some(&idx) = state.labels.get(&label) {
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
                state.vars.insert(name, value);
            }
            Action::MiniAvatar { image } => {
                debug!("MiniAvatar: {}", image);
                state.mini_avatar = Some(image);
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
    let Some(target) = state
        .menu
        .as_ref()
        .and_then(|choices| choices.get(index))
        .map(|choice| choice.target.clone())
    else {
        return;
    };

    state.menu = None;
    if let Some(&cursor) = state.labels.get(&target) {
        state.cursor = cursor;
    }
}

/// Re-index labels for the current scene (called after script changes).
pub fn index_labels(state: &mut State) {
    state.labels.clear();
    let Some(scene) = state.scenes.get(&state.current_scene) else {
        return;
    };
    for (i, action) in scene.iter().enumerate() {
        if let Action::Label(name) = action {
            state.labels.insert(name.clone(), i);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::Choice;
    use crate::types::Position;

    fn state_with(actions: Vec<Action>) -> State {
        let mut state = State::new();
        state.current_scene = "main".into();
        state.scenes.insert("main".into(), actions);
        index_labels(&mut state);
        state
    }

    #[test]
    fn executes_until_dialogue() {
        let mut state = state_with(vec![
            Action::ShowBg {
                image: "room.webp".into(),
                transition: Transition::Instant,
            },
            Action::Say {
                speaker: "A".into(),
                text: "Hello".into(),
            },
        ]);

        assert_eq!(step(&mut state), StepResult::AwaitClick);
        assert_eq!(state.bg.as_deref(), Some("room.webp"));
        assert_eq!(
            state.dialogue.as_ref().map(|d| d.text.as_str()),
            Some("Hello")
        );
    }

    #[test]
    fn jump_uses_precomputed_label_index() {
        let mut state = state_with(vec![
            Action::Jump("end".into()),
            Action::Say {
                speaker: "".into(),
                text: "skip".into(),
            },
            Action::Label("end".into()),
            Action::Say {
                speaker: "".into(),
                text: "done".into(),
            },
        ]);

        assert_eq!(step(&mut state), StepResult::AwaitClick);
        assert_eq!(
            state.dialogue.as_ref().map(|d| d.text.as_str()),
            Some("done")
        );
    }

    #[test]
    fn selecting_choice_moves_cursor_and_clears_menu() {
        let mut state = state_with(vec![
            Action::Menu {
                prompt: String::new(),
                choices: vec![Choice {
                    text: "Go".into(),
                    target: "next".into(),
                }],
            },
            Action::Label("next".into()),
        ]);

        assert_eq!(step(&mut state), StepResult::AwaitChoice);
        select_choice(&mut state, 0);
        assert!(state.menu.is_none());
        assert_eq!(state.cursor, 1);
    }

    #[test]
    fn instant_sprite_is_immediately_visible_and_removable() {
        let mut state = state_with(vec![
            Action::ShowSprite {
                id: "hero".into(),
                image: "hero.webp".into(),
                position: Position::center(0.0),
                transition: Transition::Instant,
            },
            Action::HideSprite {
                id: "hero".into(),
                transition: Transition::Instant,
            },
        ]);

        assert_eq!(step(&mut state), StepResult::EndOfScene);
        assert!(!state.sprites.contains_key("hero"));
    }
}
