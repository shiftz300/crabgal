// Step engine — executes Actions until an interactive point.
//
// Inspired by Ayaka's next_run() and Siglus's step_until_yield().
// The key insight: VN execution pauses at user interaction points (click/choice),
// NOT at every frame. This is how we achieve "frame rate insensitive" execution.

use log::debug;

use crate::action::Action;
use crate::action::ChoiceTarget;
use crate::expression::{evaluate, interpolate};
use crate::state::{
    BgTransition, Dialogue, MenuChoice, MenuState, SceneFrame, Sprite, State, TransformAnimation,
};
use crate::types::{Anchor, Transition};

/// Result of a step() call.
#[derive(Debug, Clone, PartialEq)]
pub enum StepResult {
    /// Engine is waiting for the user to click (dialogue shown).
    AwaitClick,
    /// Engine is waiting for the user to choose (menu shown).
    AwaitChoice,
    /// No more actions in this scene.
    EndOfScene,
    /// Forward execution exceeded the deterministic safety limit.
    ExecutionLimit,
}

const MAX_FORWARD_ACTIONS: usize = 1024;

/// Execute actions from the current cursor position until we hit
/// an interactive point (Say or Menu) or end of scene.
pub fn step(state: &mut State) -> StepResult {
    if state.menu.is_some() {
        return StepResult::AwaitChoice;
    }

    for _ in 0..MAX_FORWARD_ACTIONS {
        let Some(action) = state
            .scenes
            .get(&state.current_scene)
            .and_then(|scene| scene.get(state.cursor))
            .cloned()
        else {
            if let Some(frame) = state.scene_stack.pop() {
                state.current_scene = frame.scene;
                state.cursor = frame.cursor;
                index_labels(state);
                continue;
            }
            return StepResult::EndOfScene;
        };
        state.cursor += 1;

        let (action, next) = match action {
            Action::Flow { action, when, next } => {
                if let Some(condition) = when {
                    match evaluate(&condition, &state.vars, &state.global_vars) {
                        Ok(value) if value.truthy() => {}
                        Ok(_) => continue,
                        Err(error) => {
                            log::error!("invalid -when expression {condition:?}: {error}");
                            continue;
                        }
                    }
                }
                (*action, next)
            }
            action => (action, false),
        };

        match action {
            Action::ShowBg {
                image,
                transition,
                transform,
            } => {
                let image = interpolate(&image, &state.vars, &state.global_vars);
                debug!("ShowBg: {} ({:?})", image, transition);
                let from = state.bg.take();
                state.bg = Some(image.clone());
                state.bg_transform = transform;
                state.bg_transform_animation = None;
                state.bg_transition = (transition != Transition::Instant).then_some(BgTransition {
                    from,
                    to: image,
                    progress: 0.0,
                    kind: transition,
                });
            }
            Action::HideBg { transition } => {
                let from = state.bg.take();
                state.bg_transition = match (from, transition) {
                    (Some(from), transition) if transition != Transition::Instant => {
                        Some(BgTransition {
                            from: Some(from),
                            to: String::new(),
                            progress: 0.0,
                            kind: transition,
                        })
                    }
                    _ => None,
                };
            }
            Action::SetTransform {
                id,
                transform: target,
                duration,
                easing,
            } => {
                let id = interpolate(&id, &state.vars, &state.global_vars);
                debug!("SetTransform: {} -> {:?}", id, target);
                if matches!(id.as_str(), "bg-main" | "background") {
                    if duration > 0.0 {
                        state.bg_transform_animation = Some(TransformAnimation {
                            from: state.bg_transform,
                            to: target,
                            elapsed: 0.0,
                            duration,
                            easing,
                        });
                    } else {
                        state.bg_transform = target;
                        state.bg_transform_animation = None;
                    }
                } else if let Some(sprite) = state.sprites.get_mut(&id) {
                    if duration > 0.0 {
                        sprite.transform_animation = Some(TransformAnimation {
                            from: sprite.transform,
                            to: target,
                            elapsed: 0.0,
                            duration,
                            easing,
                        });
                    } else {
                        sprite.transform = target;
                        sprite.transform_animation = None;
                    }
                }
            }
            Action::ShowSprite {
                id,
                image,
                position,
                transition,
                transform,
                z_index,
                blend,
            } => {
                let id = interpolate(&id, &state.vars, &state.global_vars);
                let image = interpolate(&image, &state.vars, &state.global_vars);
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
                        transform,
                        transform_animation: None,
                        z_index,
                        blend,
                    },
                );
            }
            Action::HideSprite { id, transition } => {
                let id = interpolate(&id, &state.vars, &state.global_vars);
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
            Action::Say {
                speaker,
                text,
                options,
            } => {
                let mut speaker = resolve_speaker(&speaker, state);
                let mut text =
                    compile_rich_text(&interpolate(&text, &state.vars, &state.global_vars));
                if options.inherit_speaker
                    && let Some(previous) =
                        state.dialogue.as_ref().or(state.previous_dialogue.as_ref())
                {
                    speaker.clone_from(&previous.speaker);
                }
                if options.concat
                    && let Some(previous) =
                        state.dialogue.as_ref().or(state.previous_dialogue.as_ref())
                {
                    text = previous.text.clone() + &text;
                    if speaker.is_empty() {
                        speaker.clone_from(&previous.speaker);
                    }
                }
                debug!("Say: {}: {}", speaker, text);
                state.dialogue = Some(Dialogue {
                    speaker,
                    text,
                    visible_chars: 0,
                    vocal: options
                        .vocal
                        .map(|vocal| interpolate(&vocal, &state.vars, &state.global_vars)),
                    volume: options.volume.clamp(0.0, 1.0),
                    auto_advance: options.auto_advance,
                });
                state.record_dialogue(state.cursor - 1);
                state.menu = None;
                if !next {
                    return StepResult::AwaitClick;
                }
            }
            Action::Menu { prompt, choices } => {
                let choices = choices
                    .into_iter()
                    .filter(|choice| condition_matches(choice.show_when.as_deref(), state))
                    .map(|choice| MenuChoice {
                        text: interpolate(&choice.text, &state.vars, &state.global_vars),
                        target: interpolate_choice_target(choice.target, state),
                        enabled: condition_matches(choice.enable_when.as_deref(), state),
                    })
                    .collect::<Vec<_>>();
                if choices.is_empty() {
                    log::warn!("choice menu has no visible options; continuing");
                    continue;
                }
                debug!("Menu: {} visible choices", choices.len());
                state.menu = Some(MenuState {
                    prompt: interpolate(&prompt, &state.vars, &state.global_vars),
                    choices,
                });
                state.dialogue = None;
                return StepResult::AwaitChoice;
            }
            Action::Jump(label) => {
                let label = interpolate(&label, &state.vars, &state.global_vars);
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
            Action::ChangeScene(scene) => {
                let scene = interpolate(&scene, &state.vars, &state.global_vars);
                debug!("ChangeScene: {}", scene);
                enter_scene(state, &scene);
            }
            Action::CallScene(scene) => {
                let scene = interpolate(&scene, &state.vars, &state.global_vars);
                debug!("CallScene: {}", scene);
                if state.scenes.contains_key(&scene) {
                    state.scene_stack.push(SceneFrame {
                        scene: state.current_scene.clone(),
                        cursor: state.cursor,
                    });
                    enter_scene(state, &scene);
                } else {
                    log::warn!("CallScene target does not exist: {scene}");
                }
            }
            Action::End => {
                debug!("End");
                end_game(state);
                return StepResult::EndOfScene;
            }
            Action::Bgm { file, volume } => {
                debug!("Bgm: {} (vol {})", file, volume);
                // Audio stubbed — will be wired to rodio later.
            }
            Action::StopBgm => {
                debug!("StopBgm");
            }
            Action::Set {
                name,
                expression,
                global,
            } => {
                let name = interpolate(&name, &state.vars, &state.global_vars);
                match evaluate(&expression, &state.vars, &state.global_vars) {
                    Ok(value) => {
                        debug!("Set: {} = {:?}", name, value);
                        if global {
                            assign_value(&mut state.global_vars, &name, value);
                        } else {
                            assign_value(&mut state.vars, &name, value);
                        }
                    }
                    Err(error) => log::error!("failed to evaluate {name} = {expression}: {error}"),
                }
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
            Action::Flow { .. } => unreachable!("flow wrappers are removed before dispatch"),
        }
    }

    log::error!("script exceeded {MAX_FORWARD_ACTIONS} actions without yielding");
    StepResult::ExecutionLimit
}

fn assign_value(
    variables: &mut std::collections::HashMap<String, crate::Value>,
    target: &str,
    value: crate::Value,
) {
    let Some((name, index)) = target
        .strip_suffix(']')
        .and_then(|target| target.rsplit_once('['))
    else {
        variables.insert(target.to_owned(), value);
        return;
    };
    let Ok(index) = index.parse::<usize>() else {
        log::error!("invalid array assignment target {target:?}");
        return;
    };
    let Some(crate::Value::Array(values)) = variables.get_mut(name) else {
        log::error!("array assignment target {name:?} does not exist");
        return;
    };
    if let Some(slot) = values.get_mut(index) {
        *slot = value;
    } else {
        log::error!("array assignment index {index} is out of bounds for {name:?}");
    }
}

fn enter_scene(state: &mut State, scene: &str) -> bool {
    if !state.scenes.contains_key(scene) {
        log::warn!("scene target does not exist: {scene}");
        return false;
    }
    state.current_scene = scene.to_owned();
    state.cursor = 0;
    state.dialogue = None;
    state.menu = None;
    index_labels(state);
    true
}

/// Handle user clicking to advance past a dialogue.
pub fn advance(state: &mut State) {
    state.mark_current_dialogue_read();
    state.previous_dialogue = state.dialogue.take();
}

pub fn end_game(state: &mut State) {
    state.scene_stack.clear();
    state.dialogue = None;
    state.previous_dialogue = None;
    state.menu = None;
    state.bg = None;
    state.bg_transition = None;
    state.sprites.clear();
    state.mini_avatar = None;
    state.vars.clear();
    state.cursor = state.scenes.get(&state.current_scene).map_or(0, Vec::len);
    state.ended = true;
}

/// Handle user selecting a menu choice.
pub fn select_choice(state: &mut State, index: usize) {
    let Some(target) = state
        .menu
        .as_ref()
        .and_then(|menu| menu.choices.get(index))
        .filter(|choice| choice.enabled)
        .map(|choice| choice.target.clone())
    else {
        return;
    };

    state.menu = None;
    match target {
        ChoiceTarget::Label(label) => {
            if let Some(&cursor) = state.labels.get(&label) {
                state.cursor = cursor;
            }
        }
        ChoiceTarget::ChangeScene(scene) => {
            enter_scene(state, &scene);
        }
        ChoiceTarget::CallScene(scene) => {
            if state.scenes.contains_key(&scene) {
                state.scene_stack.push(SceneFrame {
                    scene: state.current_scene.clone(),
                    cursor: state.cursor,
                });
                enter_scene(state, &scene);
            }
        }
    }
}

fn condition_matches(condition: Option<&str>, state: &State) -> bool {
    let Some(condition) = condition else {
        return true;
    };
    match evaluate(condition, &state.vars, &state.global_vars) {
        Ok(value) => value.truthy(),
        Err(error) => {
            log::error!("invalid choice condition {condition:?}: {error}");
            false
        }
    }
}

fn interpolate_choice_target(target: ChoiceTarget, state: &State) -> ChoiceTarget {
    let interpolate_target = |value: String| interpolate(&value, &state.vars, &state.global_vars);
    match target {
        ChoiceTarget::Label(value) => ChoiceTarget::Label(interpolate_target(value)),
        ChoiceTarget::ChangeScene(value) => ChoiceTarget::ChangeScene(interpolate_target(value)),
        ChoiceTarget::CallScene(value) => ChoiceTarget::CallScene(interpolate_target(value)),
    }
}

fn resolve_speaker(source: &str, state: &State) -> String {
    let speaker = interpolate(source, &state.vars, &state.global_vars);
    state
        .vars
        .get(&speaker)
        .or_else(|| state.global_vars.get(&speaker))
        .map_or(speaker, crate::Value::display)
}

fn compile_rich_text(source: &str) -> String {
    let source = source.replace("<br/>", "\n").replace("<br>", "\n");
    let chars = source.chars().collect::<Vec<_>>();
    let mut output = String::new();
    let mut cursor = 0;
    while cursor < chars.len() {
        if chars[cursor] != '[' {
            output.push(chars[cursor]);
            cursor += 1;
            continue;
        }
        let Some(label_end) = chars[cursor + 1..].iter().position(|value| *value == ']') else {
            output.push(chars[cursor]);
            cursor += 1;
            continue;
        };
        let label_end = cursor + 1 + label_end;
        if chars.get(label_end + 1) != Some(&'(') {
            output.push(chars[cursor]);
            cursor += 1;
            continue;
        }
        let Some(argument_end) = chars[label_end + 2..]
            .iter()
            .position(|value| *value == ')')
        else {
            output.push(chars[cursor]);
            cursor += 1;
            continue;
        };
        let argument_end = label_end + 2 + argument_end;
        let label = chars[cursor + 1..label_end].iter().collect::<String>();
        let argument = chars[label_end + 2..argument_end]
            .iter()
            .collect::<String>();
        output.push_str(&label);
        if !argument.contains('=') && !argument.is_empty() {
            output.push('（');
            output.push_str(&argument);
            output.push('）');
        }
        cursor = argument_end + 1;
    }
    output
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
    use crate::Value;
    use crate::action::{Choice, SayOptions};
    use crate::types::{BlendMode, Position, SpriteTransform};

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
                transform: SpriteTransform::default(),
            },
            Action::Say {
                speaker: "A".into(),
                text: "Hello".into(),
                options: SayOptions::default(),
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
                options: SayOptions::default(),
            },
            Action::Label("end".into()),
            Action::Say {
                speaker: "".into(),
                text: "done".into(),
                options: SayOptions::default(),
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
                    target: ChoiceTarget::Label("next".into()),
                    show_when: None,
                    enable_when: None,
                }],
            },
            Action::Label("next".into()),
        ]);

        assert_eq!(step(&mut state), StepResult::AwaitChoice);
        assert_eq!(state.menu.as_ref().unwrap().prompt, "");
        select_choice(&mut state, 0);
        assert!(state.menu.is_none());
        assert_eq!(state.cursor, 1);
    }

    #[test]
    fn invalid_choice_keeps_menu_open() {
        let mut state = state_with(vec![Action::Menu {
            prompt: "Pick one".into(),
            choices: vec![Choice {
                text: "Go".into(),
                target: ChoiceTarget::Label("next".into()),
                show_when: None,
                enable_when: None,
            }],
        }]);

        assert_eq!(step(&mut state), StepResult::AwaitChoice);
        select_choice(&mut state, 4);

        assert_eq!(state.menu.as_ref().unwrap().prompt, "Pick one");
        assert_eq!(state.cursor, 1);
    }

    #[test]
    fn repeated_step_does_not_advance_past_open_menu() {
        let mut state = state_with(vec![
            Action::Menu {
                prompt: String::new(),
                choices: vec![Choice {
                    text: "Go".into(),
                    target: ChoiceTarget::Label("next".into()),
                    show_when: None,
                    enable_when: None,
                }],
            },
            Action::Say {
                speaker: String::new(),
                text: "must wait".into(),
                options: SayOptions::default(),
            },
        ]);

        assert_eq!(step(&mut state), StepResult::AwaitChoice);
        assert_eq!(step(&mut state), StepResult::AwaitChoice);
        assert_eq!(state.cursor, 1);
        assert!(state.dialogue.is_none());
    }

    #[test]
    fn instant_sprite_is_immediately_visible_and_removable() {
        let mut state = state_with(vec![
            Action::ShowSprite {
                id: "hero".into(),
                image: "hero.webp".into(),
                position: Position::center(0.0),
                transition: Transition::Instant,
                transform: SpriteTransform::default(),
                z_index: 0,
                blend: BlendMode::Alpha,
            },
            Action::HideSprite {
                id: "hero".into(),
                transition: Transition::Instant,
            },
        ]);

        assert_eq!(step(&mut state), StepResult::EndOfScene);
        assert!(!state.sprites.contains_key("hero"));
    }

    #[test]
    fn change_scene_replaces_current_flow() {
        let mut state = state_with(vec![
            Action::ChangeScene("chapter".into()),
            Action::Say {
                speaker: String::new(),
                text: "unreachable".into(),
                options: SayOptions::default(),
            },
        ]);
        state.scenes.insert(
            "chapter".into(),
            vec![Action::Say {
                speaker: String::new(),
                text: "chapter".into(),
                options: SayOptions::default(),
            }],
        );

        assert_eq!(step(&mut state), StepResult::AwaitClick);
        assert_eq!(state.current_scene, "chapter");
        assert_eq!(state.dialogue.as_ref().unwrap().text, "chapter");
        assert!(state.scene_stack.is_empty());
    }

    #[test]
    fn call_scene_returns_to_following_action() {
        let mut state = state_with(vec![
            Action::CallScene("aside".into()),
            Action::Say {
                speaker: String::new(),
                text: "back".into(),
                options: SayOptions::default(),
            },
        ]);
        state.scenes.insert(
            "aside".into(),
            vec![Action::Say {
                speaker: String::new(),
                text: "inside".into(),
                options: SayOptions::default(),
            }],
        );

        assert_eq!(step(&mut state), StepResult::AwaitClick);
        assert_eq!(state.current_scene, "aside");
        assert_eq!(state.scene_stack.len(), 1);
        advance(&mut state);
        assert_eq!(step(&mut state), StepResult::AwaitClick);
        assert_eq!(state.current_scene, "main");
        assert_eq!(state.dialogue.as_ref().unwrap().text, "back");
        assert!(state.scene_stack.is_empty());
    }

    #[test]
    fn nested_scene_calls_restore_in_lifo_order() {
        let mut state = state_with(vec![Action::CallScene("first".into())]);
        state.scenes.insert(
            "first".into(),
            vec![
                Action::CallScene("second".into()),
                Action::Say {
                    speaker: String::new(),
                    text: "first".into(),
                    options: SayOptions::default(),
                },
            ],
        );
        state.scenes.insert("second".into(), Vec::new());

        assert_eq!(step(&mut state), StepResult::AwaitClick);
        assert_eq!(state.current_scene, "first");
        assert_eq!(state.dialogue.as_ref().unwrap().text, "first");
        assert_eq!(state.scene_stack.len(), 1);
        advance(&mut state);
        assert_eq!(step(&mut state), StepResult::EndOfScene);
        assert_eq!(state.current_scene, "main");
        assert!(state.scene_stack.is_empty());
    }

    #[test]
    fn explicit_end_discards_call_stack() {
        let mut state = state_with(vec![
            Action::CallScene("ending".into()),
            Action::Say {
                speaker: String::new(),
                text: "must not return".into(),
                options: SayOptions::default(),
            },
        ]);
        state.scenes.insert("ending".into(), vec![Action::End]);

        assert_eq!(step(&mut state), StepResult::EndOfScene);
        assert_eq!(state.current_scene, "ending");
        assert!(state.scene_stack.is_empty());
        assert!(state.dialogue.is_none());
        assert!(state.ended);
        assert_eq!(step(&mut state), StepResult::EndOfScene);
    }

    #[test]
    fn expressions_globals_conditions_and_interpolation_share_one_runtime() {
        let mut state = state_with(vec![
            Action::Set {
                name: "score".into(),
                expression: "2 + 3 * 2".into(),
                global: false,
            },
            Action::Set {
                name: "name".into(),
                expression: "'MainCore'".into(),
                global: true,
            },
            Action::Flow {
                action: Box::new(Action::Say {
                    speaker: "{name}".into(),
                    text: "score={score}".into(),
                    options: SayOptions::default(),
                }),
                when: Some("score == 8".into()),
                next: false,
            },
        ]);

        assert_eq!(step(&mut state), StepResult::AwaitClick);
        let dialogue = state.dialogue.as_ref().unwrap();
        assert_eq!(dialogue.speaker, "MainCore");
        assert_eq!(dialogue.text, "score=8");
        assert_eq!(
            state.global_vars.get("name"),
            Some(&Value::Str("MainCore".into()))
        );
    }

    #[test]
    fn next_and_concat_follow_webgal_flow() {
        let concat = SayOptions {
            concat: true,
            inherit_speaker: true,
            ..SayOptions::default()
        };
        let mut state = state_with(vec![
            Action::Flow {
                action: Box::new(Action::Say {
                    speaker: "A".into(),
                    text: "ignored by next".into(),
                    options: SayOptions::default(),
                }),
                when: None,
                next: true,
            },
            Action::Say {
                speaker: "A".into(),
                text: "Hello".into(),
                options: SayOptions::default(),
            },
            Action::Say {
                speaker: String::new(),
                text: " world".into(),
                options: concat,
            },
        ]);

        assert_eq!(step(&mut state), StepResult::AwaitClick);
        assert_eq!(state.dialogue.as_ref().unwrap().text, "Hello");
        advance(&mut state);
        assert_eq!(step(&mut state), StepResult::AwaitClick);
        assert_eq!(state.dialogue.as_ref().unwrap().speaker, "A");
        assert_eq!(state.dialogue.as_ref().unwrap().text, "Hello world");
    }

    #[test]
    fn choice_filters_disables_and_changes_scene() {
        let mut state = state_with(vec![Action::Menu {
            prompt: String::new(),
            choices: vec![
                Choice {
                    text: "hidden".into(),
                    target: ChoiceTarget::Label("none".into()),
                    show_when: Some("false".into()),
                    enable_when: None,
                },
                Choice {
                    text: "disabled".into(),
                    target: ChoiceTarget::Label("none".into()),
                    show_when: None,
                    enable_when: Some("false".into()),
                },
                Choice {
                    text: "scene".into(),
                    target: ChoiceTarget::ChangeScene("chapter".into()),
                    show_when: None,
                    enable_when: Some("true".into()),
                },
            ],
        }]);
        state.scenes.insert(
            "chapter".into(),
            vec![Action::Say {
                speaker: String::new(),
                text: "arrived".into(),
                options: SayOptions::default(),
            }],
        );

        assert_eq!(step(&mut state), StepResult::AwaitChoice);
        assert_eq!(state.menu.as_ref().unwrap().choices.len(), 2);
        select_choice(&mut state, 0);
        assert!(state.menu.is_some());
        select_choice(&mut state, 1);
        assert_eq!(state.current_scene, "chapter");
        assert_eq!(step(&mut state), StepResult::AwaitClick);
    }

    #[test]
    fn runaway_jump_hits_execution_limit() {
        let mut state = state_with(vec![
            Action::Label("loop".into()),
            Action::Jump("loop".into()),
        ]);
        assert_eq!(step(&mut state), StepResult::ExecutionLimit);
    }
}
