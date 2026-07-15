// Step engine — executes Actions until an interactive point.
//
// Inspired by Ayaka's next_run() and Siglus's step_until_yield().
// The key insight: VN execution pauses at user interaction points (click/choice),
// NOT at every frame. This is how we achieve "frame rate insensitive" execution.

use std::sync::Arc;

use log::debug;

use crate::action::Action;
use crate::action::ChoiceTarget;
use crate::expression::{evaluate, interpolate};
use crate::state::{
    BgTransition, Dialogue, IntroState, MenuChoice, MenuState, PresetAnimation, SceneFrame, Sprite,
    State, TransformAnimation, TransitionRule,
};
use crate::types::{Anchor, Transition};

/// Result of a step() call.
#[derive(Debug, Clone, PartialEq)]
pub enum StepResult {
    /// Engine is waiting for the user to click (dialogue shown).
    AwaitClick,
    /// Engine is waiting for the user to choose (menu shown).
    AwaitChoice,
    /// Engine is waiting for a timed presentation action to finish.
    AwaitPresentation,
    /// Engine is waiting for text input confirmation.
    AwaitInput,
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
    if state.user_input.is_some() {
        return StepResult::AwaitInput;
    }
    if state.presentation_blocked() {
        return StepResult::AwaitPresentation;
    }

    // Keep an independent handle to immutable script data. Actions can then be
    // borrowed for dispatch while the rest of `state` is updated in place.
    let program = Arc::clone(&state.program);

    for _ in 0..MAX_FORWARD_ACTIONS {
        let Some(action) = program
            .scene(&state.current_scene)
            .and_then(|scene| scene.get(state.cursor))
        else {
            if let Some(frame) = state.scene_stack.pop() {
                state.current_scene = frame.scene;
                state.cursor = frame.cursor;
                continue;
            }
            return StepResult::EndOfScene;
        };
        state.cursor += 1;

        let (action, next) = match action {
            Action::Flow { action, when, next } => {
                if let Some(condition) = when {
                    match evaluate(condition, &state.vars, &state.global_vars) {
                        Ok(value) if value.truthy() => {}
                        Ok(_) => continue,
                        Err(error) => {
                            log::error!("invalid -when expression {condition:?}: {error}");
                            continue;
                        }
                    }
                }
                (action.as_ref(), *next)
            }
            action => (action, false),
        };

        match action {
            Action::ShowBg {
                image,
                transition,
                transform,
            } => {
                let transition = *transition;
                let transform = *transform;
                let image = interpolate(image, &state.vars, &state.global_vars);
                debug!("ShowBg: {} ({:?})", image, transition);
                let from = state.bg.take();
                state.bg_transform = transform;
                state.bg_transform_animation = None;
                let rule_animation = state.transition_rules.get("bg-main").and_then(|rule| {
                    rule.enter.as_ref().map(|preset| PresetAnimation {
                        preset: preset.clone(),
                        base: transform,
                        elapsed: 0.0,
                        duration: rule.duration.max(f32::EPSILON),
                        blocking: !next,
                        remove_on_finish: false,
                    })
                });
                let rule_blocks = rule_animation.is_some() && !next;
                if let Some(animation) = &rule_animation {
                    state.bg_transform =
                        preset_initial_transform(animation.base, &animation.preset);
                }
                state.bg_animation = rule_animation;
                state.bg_transition = if transition == Transition::Instant {
                    None
                } else {
                    Some(BgTransition {
                        from,
                        to: image.clone(),
                        progress: 0.0,
                        kind: transition,
                        blocking: !next,
                    })
                };
                state.bg = Some(image);
                if rule_blocks || (!next && transition != Transition::Instant) {
                    return StepResult::AwaitPresentation;
                }
            }
            Action::HideBg { transition } => {
                let transition = *transition;
                if state.bg.is_some()
                    && let Some((preset, duration)) =
                        state.transition_rules.get("bg-main").and_then(|rule| {
                            rule.exit
                                .as_ref()
                                .map(|preset| (preset.clone(), rule.duration))
                        })
                {
                    state.bg_animation = Some(PresetAnimation {
                        preset,
                        base: state.bg_transform,
                        elapsed: 0.0,
                        duration: duration.max(f32::EPSILON),
                        blocking: !next,
                        remove_on_finish: true,
                    });
                    if !next {
                        return StepResult::AwaitPresentation;
                    }
                    continue;
                }
                let from = state.bg.take();
                state.bg_transition = match (from, transition) {
                    (Some(from), transition) if transition != Transition::Instant => {
                        Some(BgTransition {
                            from: Some(from),
                            to: String::new(),
                            progress: 0.0,
                            kind: transition,
                            blocking: !next,
                        })
                    }
                    _ => None,
                };
                if !next && transition != Transition::Instant {
                    return StepResult::AwaitPresentation;
                }
            }
            Action::SetTransform {
                id,
                transform: target,
                duration,
                easing,
            } => {
                let duration = *duration;
                let easing = *easing;
                let id = interpolate(id, &state.vars, &state.global_vars);
                debug!("SetTransform: {} -> {:?}", id, target);
                let mut started = false;
                if matches!(id.as_str(), "bg-main" | "background") {
                    started = true;
                    let target = target.apply_to(state.bg_transform);
                    if duration > 0.0 {
                        state.bg_transform_animation = Some(TransformAnimation {
                            from: state.bg_transform,
                            to: target,
                            elapsed: 0.0,
                            duration,
                            easing,
                            blocking: !next,
                        });
                    } else {
                        state.bg_transform = target;
                        state.bg_transform_animation = None;
                    }
                } else if let Some(sprite) = state.sprites.get_mut(&id) {
                    started = true;
                    let target = target.apply_to(sprite.transform);
                    if duration > 0.0 {
                        sprite.transform_animation = Some(TransformAnimation {
                            from: sprite.transform,
                            to: target,
                            elapsed: 0.0,
                            duration,
                            easing,
                            blocking: !next,
                        });
                    } else {
                        sprite.transform = target;
                        sprite.transform_animation = None;
                    }
                }
                if started && !next && duration > 0.0 {
                    return StepResult::AwaitPresentation;
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
                let position = *position;
                let transition = *transition;
                let transform = *transform;
                let z_index = *z_index;
                let blend = *blend;
                let id = interpolate(id, &state.vars, &state.global_vars);
                let image = interpolate(image, &state.vars, &state.global_vars);
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
                let rule_animation = state.transition_rules.get(&id).and_then(|rule| {
                    rule.enter.as_ref().map(|preset| PresetAnimation {
                        preset: preset.clone(),
                        base: transform,
                        elapsed: 0.0,
                        duration: rule.duration.max(f32::EPSILON),
                        blocking: !next,
                        remove_on_finish: false,
                    })
                });
                let rule_blocks = rule_animation.is_some() && !next;
                let initial_transform = rule_animation.as_ref().map_or(transform, |animation| {
                    preset_initial_transform(animation.base, &animation.preset)
                });
                state.sprites.insert(
                    id,
                    Sprite {
                        image,
                        position,
                        transition_progress,
                        transition,
                        entering: true,
                        transition_offset_x,
                        transition_blocking: !next,
                        transform: initial_transform,
                        transform_animation: None,
                        filter: Default::default(),
                        animation: rule_animation,
                        z_index,
                        blend,
                    },
                );
                if rule_blocks || (!next && transition != Transition::Instant) {
                    return StepResult::AwaitPresentation;
                }
            }
            Action::HideSprite { id, transition } => {
                let transition = *transition;
                let id = interpolate(id, &state.vars, &state.global_vars);
                debug!("HideSprite: {}", id);
                let exit_rule = state
                    .transition_rules
                    .get(&id)
                    .and_then(|rule| rule.exit.as_ref().map(|preset| (preset, rule.duration)));
                if let Some((sprite, (preset, duration))) =
                    state.sprites.get_mut(&id).zip(exit_rule)
                {
                    sprite.animation = Some(PresetAnimation {
                        preset: preset.clone(),
                        base: sprite.transform,
                        elapsed: 0.0,
                        duration: duration.max(f32::EPSILON),
                        blocking: !next,
                        remove_on_finish: true,
                    });
                    // The preset owns the entire exit lifecycle. Keep the
                    // sprite fully present until it finishes so the ordinary
                    // transition cleanup cannot remove it on the first frame
                    // (especially when its original transition was Instant).
                    sprite.entering = true;
                    sprite.transition_progress = 1.0;
                    sprite.transition_blocking = false;
                    if !next {
                        return StepResult::AwaitPresentation;
                    }
                } else if transition == Transition::Instant {
                    state.sprites.remove(&id);
                } else if let Some(sprite) = state.sprites.get_mut(&id) {
                    sprite.transition = transition;
                    sprite.entering = false;
                    sprite.transition_blocking = !next;
                    sprite.transition_offset_x = match transition {
                        Transition::SlideFromLeft(_) => -400.0,
                        Transition::SlideFromRight(_) => 400.0,
                        _ => 0.0,
                    };
                    if !next {
                        return StepResult::AwaitPresentation;
                    }
                }
            }
            Action::Say {
                speaker,
                text,
                options,
            } => {
                if state.textbox_auto_hidden {
                    state.textbox_hidden = false;
                    state.textbox_auto_hidden = false;
                }
                let mut speaker = resolve_speaker(speaker, state);
                let mut markup = interpolate(text, &state.vars, &state.global_vars)
                    .replace("<br/>", "\n")
                    .replace("<br>", "\n");
                let mut text = compile_rich_text(&markup);
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
                    markup = previous.markup.clone() + &markup;
                    if speaker.is_empty() {
                        speaker.clone_from(&previous.speaker);
                    }
                }
                debug!("Say: {}: {}", speaker, text);
                state.dialogue = Some(Dialogue {
                    speaker,
                    text,
                    markup,
                    visible_chars: 0,
                    vocal: options
                        .vocal
                        .as_deref()
                        .map(|vocal| interpolate(vocal, &state.vars, &state.global_vars)),
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
                    .iter()
                    .filter(|choice| condition_matches(choice.show_when.as_deref(), state))
                    .map(|choice| MenuChoice {
                        text: interpolate(&choice.text, &state.vars, &state.global_vars),
                        target: interpolate_choice_target(&choice.target, state),
                        enabled: condition_matches(choice.enable_when.as_deref(), state),
                    })
                    .collect::<Vec<_>>();
                if choices.is_empty() {
                    log::warn!("choice menu has no visible options; continuing");
                    continue;
                }
                debug!("Menu: {} visible choices", choices.len());
                state.menu = Some(MenuState {
                    prompt: interpolate(prompt, &state.vars, &state.global_vars),
                    choices,
                });
                state.dialogue = None;
                return StepResult::AwaitChoice;
            }
            Action::Jump(label) => {
                let label = interpolate(label, &state.vars, &state.global_vars);
                if let Some(idx) = program.label(&state.current_scene, &label) {
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
                let scene = interpolate(scene, &state.vars, &state.global_vars);
                debug!("ChangeScene: {}", scene);
                enter_scene(state, &scene);
            }
            Action::CallScene(scene) => {
                let scene = interpolate(scene, &state.vars, &state.global_vars);
                debug!("CallScene: {}", scene);
                if program.contains_scene(&scene) {
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
            Action::Bgm {
                file,
                volume,
                fade_seconds,
            } => {
                let file = interpolate(file, &state.vars, &state.global_vars);
                state.bgm.file = (file != "none" && !file.is_empty()).then_some(file);
                state.bgm.volume = volume.clamp(0.0, 1.0);
                state.bgm.fade_seconds = fade_seconds.max(0.0);
                state.bgm.revision = state.bgm.revision.wrapping_add(1);
            }
            Action::Effect { file, volume, id } => {
                let id = id
                    .as_deref()
                    .map(|id| interpolate(id, &state.vars, &state.global_vars));
                let file = file
                    .as_deref()
                    .map(|file| interpolate(file, &state.vars, &state.global_vars));
                match (id, file) {
                    (Some(id), Some(file)) => {
                        state.looping_effects.insert(
                            id,
                            crate::state::EffectState {
                                file,
                                volume: volume.clamp(0.0, 1.0),
                            },
                        );
                    }
                    (Some(id), None) => {
                        state.looping_effects.remove(&id);
                    }
                    (None, Some(file)) => {
                        state.effect_queue.push(crate::state::EffectEvent::Play(
                            crate::state::EffectCue {
                                file,
                                volume: volume.clamp(0.0, 1.0),
                            },
                        ));
                    }
                    (None, None) => state.effect_queue.push(crate::state::EffectEvent::Stop),
                }
            }
            Action::Set {
                name,
                expression,
                global,
            } => {
                let name = interpolate(name, &state.vars, &state.global_vars);
                match evaluate(expression, &state.vars, &state.global_vars) {
                    Ok(value) => {
                        debug!("Set: {} = {:?}", name, value);
                        if *global {
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
                state.mini_avatar = Some(image.clone());
                state.mini_avatar_progress = 0.0;
            }
            Action::HideMiniAvatar => {
                debug!("HideMiniAvatar");
                state.mini_avatar = None;
                state.mini_avatar_progress = 0.0;
            }
            Action::Animate {
                target,
                preset,
                duration,
            } => {
                let duration = *duration;
                let target = interpolate(target, &state.vars, &state.global_vars);
                let animation = |base| PresetAnimation {
                    preset: preset.clone(),
                    base,
                    elapsed: 0.0,
                    duration: duration.max(f32::EPSILON),
                    blocking: !next,
                    remove_on_finish: matches!(preset, crate::types::AnimationPreset::Exit),
                };
                let mut started = false;
                if is_background_target(&target) {
                    started = true;
                    let animation = animation(state.bg_transform);
                    state.bg_transform =
                        preset_initial_transform(animation.base, &animation.preset);
                    state.bg_animation = Some(animation);
                } else if let Some(sprite) = state.sprites.get_mut(&target) {
                    started = true;
                    let animation = animation(sprite.transform);
                    sprite.transform = preset_initial_transform(animation.base, &animation.preset);
                    sprite.animation = Some(animation);
                } else {
                    log::warn!("animation target does not exist: {target}");
                }
                if started && !next && duration > 0.0 {
                    return StepResult::AwaitPresentation;
                }
            }
            Action::SetTransition {
                target,
                enter,
                exit,
                duration,
            } => {
                let target = interpolate(target, &state.vars, &state.global_vars);
                state.transition_rules.insert(
                    target,
                    TransitionRule {
                        enter: enter.clone(),
                        exit: exit.clone(),
                        duration: duration.max(f32::EPSILON),
                    },
                );
            }
            Action::SetFilter { target, filter } => {
                let target = interpolate(target, &state.vars, &state.global_vars);
                if is_background_target(&target) {
                    state.bg_filter = *filter;
                } else if let Some(sprite) = state.sprites.get_mut(&target) {
                    sprite.filter = *filter;
                } else {
                    log::warn!("filter target does not exist: {target}");
                }
            }
            Action::Wait { seconds } => {
                state.wait_remaining = seconds.max(0.0);
                state.wait_blocking = !next;
                if !next && state.wait_remaining > 0.0 {
                    return StepResult::AwaitPresentation;
                }
            }
            Action::Intro { pages, hold } => {
                let pages = pages
                    .iter()
                    .map(|page| interpolate(page, &state.vars, &state.global_vars))
                    .collect();
                state.dialogue = None;
                state.intro = Some(IntroState {
                    pages,
                    page: 0,
                    elapsed: 0.0,
                    hold: *hold,
                    blocking: !next,
                });
                if !next {
                    return StepResult::AwaitPresentation;
                }
            }
            Action::FilmMode { enabled } => state.film_mode = *enabled,
            Action::Particle { effect } => {
                state.particle_effect = effect
                    .as_deref()
                    .map(|effect| interpolate(effect, &state.vars, &state.global_vars));
            }
            Action::SetTextbox { visible, auto } => {
                state.textbox_hidden = !*visible;
                state.textbox_auto_hidden = !*visible && *auto;
            }
            Action::UserInput {
                variable,
                title,
                button,
            } => {
                state.user_input = Some(crate::state::UserInputState {
                    variable: interpolate(variable, &state.vars, &state.global_vars),
                    title: interpolate(title, &state.vars, &state.global_vars),
                    button: interpolate(button, &state.vars, &state.global_vars),
                    value: String::new(),
                });
                return StepResult::AwaitInput;
            }
            Action::Comment => {}
            Action::Unlock { kind, file, name } => {
                let file = interpolate(file, &state.vars, &state.global_vars);
                let name = interpolate(name, &state.vars, &state.global_vars);
                match kind {
                    crate::types::UnlockKind::Cg => {
                        state.unlocked_cg.insert(file, name);
                    }
                    crate::types::UnlockKind::Bgm => {
                        state.unlocked_bgm.insert(file, name);
                    }
                }
            }
            Action::Flow { .. } => unreachable!("flow wrappers are removed before dispatch"),
        }
    }

    log::error!("script exceeded {MAX_FORWARD_ACTIONS} actions without yielding");
    StepResult::ExecutionLimit
}

fn is_background_target(target: &str) -> bool {
    matches!(target, "bg-main" | "background")
}

fn preset_initial_transform(
    base: crate::types::SpriteTransform,
    preset: &crate::types::AnimationPreset,
) -> crate::types::SpriteTransform {
    use crate::types::AnimationPreset;
    let mut initial = base;
    match preset {
        AnimationPreset::Enter => initial.alpha = 0.0,
        AnimationPreset::EnterFromBottom => {
            initial.offset_y += 180.0;
            initial.alpha = 0.0;
        }
        AnimationPreset::EnterFromLeft => {
            initial.offset_x -= 220.0;
            initial.alpha = 0.0;
        }
        AnimationPreset::EnterFromRight => {
            initial.offset_x += 220.0;
            initial.alpha = 0.0;
        }
        _ => {}
    }
    initial
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
    if !state.program.contains_scene(scene) {
        log::warn!("scene target does not exist: {scene}");
        return false;
    }
    state.current_scene = scene.to_owned();
    state.cursor = 0;
    state.dialogue = None;
    state.menu = None;
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
    state.bg_animation = None;
    state.bg_filter = Default::default();
    state.sprites.clear();
    state.mini_avatar = None;
    state.textbox_hidden = false;
    state.textbox_auto_hidden = false;
    state.user_input = None;
    state.wait_remaining = 0.0;
    state.wait_blocking = false;
    state.intro = None;
    state.film_mode = false;
    state.particle_effect = None;
    state.transition_rules.clear();
    state.bgm.file = None;
    state.bgm.fade_seconds = 0.0;
    state.bgm.revision = state.bgm.revision.wrapping_add(1);
    state.looping_effects.clear();
    state.effect_queue.clear();
    state.vars.clear();
    state.cursor = state.program.scene_len(&state.current_scene).unwrap_or(0);
    state.ended = true;
}

pub fn submit_user_input(state: &mut State) -> bool {
    let Some(input) = state.user_input.take() else {
        return false;
    };
    state
        .vars
        .insert(input.variable, crate::Value::Str(input.value));
    true
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
            if let Some(cursor) = state.program.label(&state.current_scene, &label) {
                state.cursor = cursor;
            }
        }
        ChoiceTarget::ChangeScene(scene) => {
            enter_scene(state, &scene);
        }
        ChoiceTarget::CallScene(scene) => {
            if state.program.contains_scene(&scene) {
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

fn interpolate_choice_target(target: &ChoiceTarget, state: &State) -> ChoiceTarget {
    let interpolate_target = |value: &str| interpolate(value, &state.vars, &state.global_vars);
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
        output.push_str(&label);
        cursor = argument_end + 1;
    }
    output
}

/// Compatibility no-op. Labels are indexed once when `Program` is built.
pub fn index_labels(_state: &mut State) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Value;
    use crate::action::{Choice, SayOptions};
    use crate::types::{BlendMode, Easing, Position, SpriteTransform, TransformPatch};

    fn state_with(actions: Vec<Action>) -> State {
        let mut state = State::new();
        state.current_scene = "main".into();
        state.insert_scene("main".into(), actions);
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
    fn transform_animation_targets_a_sparse_patch_of_current_sprite_state() {
        let base = SpriteTransform {
            offset_x: 15.0,
            offset_y: -20.0,
            alpha: 0.7,
            scale_x: 1.2,
            scale_y: 0.85,
            rotation: 0.1,
            blur: 5.0,
        };
        let mut patch = TransformPatch::default();
        patch.set_offset_x(160.0);
        patch.set_alpha(0.3);
        let mut state = state_with(vec![
            Action::ShowSprite {
                id: "hero".into(),
                image: "hero.webp".into(),
                position: Position::center(0.0),
                transition: Transition::Instant,
                transform: base,
                z_index: 0,
                blend: BlendMode::Alpha,
            },
            Action::SetTransform {
                id: "hero".into(),
                transform: patch,
                duration: 0.5,
                easing: Easing::EaseOut,
            },
        ]);

        assert_eq!(step(&mut state), StepResult::AwaitPresentation);
        let sprite = &state.sprites["hero"];
        let animation = sprite.transform_animation.as_ref().unwrap();
        assert_eq!(animation.from, base);
        assert_eq!(
            animation.to,
            SpriteTransform {
                offset_x: 160.0,
                alpha: 0.3,
                ..base
            }
        );
        assert_eq!(animation.duration, 0.5);
        assert_eq!(animation.easing, Easing::EaseOut);
    }

    #[test]
    fn zero_duration_transform_patch_is_applied_immediately_to_background() {
        let base = SpriteTransform {
            offset_y: 40.0,
            alpha: 0.8,
            scale_x: 1.1,
            scale_y: 1.1,
            blur: 7.0,
            ..SpriteTransform::default()
        };
        let mut patch = TransformPatch::default();
        patch.set_rotation(0.4);
        patch.set_blur(0.0);
        let mut state = state_with(vec![
            Action::ShowBg {
                image: "room.webp".into(),
                transition: Transition::Instant,
                transform: base,
            },
            Action::SetTransform {
                id: "bg-main".into(),
                transform: patch,
                duration: 0.0,
                easing: Easing::EaseInOut,
            },
        ]);

        assert_eq!(step(&mut state), StepResult::EndOfScene);
        assert_eq!(
            state.bg_transform,
            SpriteTransform {
                rotation: 0.4,
                blur: 0.0,
                ..base
            }
        );
        assert!(state.bg_transform_animation.is_none());
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
        state.insert_scene(
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
        state.insert_scene(
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
        state.insert_scene(
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
        state.insert_scene("second".into(), Vec::new());

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
        state.insert_scene("ending".into(), vec![Action::End]);

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
        state.insert_scene(
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

    #[test]
    fn audio_commands_update_persistent_and_transient_state() {
        let mut state = state_with(vec![
            Action::Bgm {
                file: "theme.ogg".into(),
                volume: 0.7,
                fade_seconds: 1.5,
            },
            Action::Effect {
                file: Some("rain.ogg".into()),
                volume: 0.4,
                id: Some("weather".into()),
            },
            Action::Effect {
                file: Some("click.wav".into()),
                volume: 0.8,
                id: None,
            },
        ]);

        assert_eq!(step(&mut state), StepResult::EndOfScene);
        assert_eq!(state.bgm.file.as_deref(), Some("theme.ogg"));
        assert_eq!(state.bgm.volume, 0.7);
        assert_eq!(state.bgm.fade_seconds, 1.5);
        assert_eq!(state.looping_effects["weather"].file, "rain.ogg");
        assert!(matches!(
            &state.effect_queue[0],
            crate::state::EffectEvent::Play(cue) if cue.file == "click.wav"
        ));
    }

    #[test]
    fn presentation_commands_block_and_persist_stage_state() {
        use crate::types::{AnimationPreset, BlendMode, Position, VisualFilter};

        let filter = VisualFilter {
            blur: 4.0,
            brightness: 0.9,
            ..Default::default()
        };
        let mut state = state_with(vec![
            Action::ShowSprite {
                id: "hero".into(),
                image: "hero.webp".into(),
                position: Position::center(0.0),
                transition: Transition::Instant,
                transform: Default::default(),
                z_index: 0,
                blend: BlendMode::Screen,
            },
            Action::Animate {
                target: "hero".into(),
                preset: AnimationPreset::Shake,
                duration: 0.4,
            },
            Action::SetFilter {
                target: "hero".into(),
                filter,
            },
            Action::FilmMode { enabled: true },
            Action::Particle {
                effect: Some("rain".into()),
            },
            Action::Wait { seconds: 0.5 },
        ]);

        assert_eq!(step(&mut state), StepResult::AwaitPresentation);
        assert!(state.sprites["hero"].animation.is_some());
        state.sprites.get_mut("hero").unwrap().animation = None;
        assert_eq!(step(&mut state), StepResult::AwaitPresentation);
        assert_eq!(state.sprites["hero"].filter, filter);
        assert!(state.film_mode);
        assert_eq!(state.particle_effect.as_deref(), Some("rain"));
        assert_eq!(state.wait_remaining, 0.5);
    }

    #[test]
    fn borrowed_non_copy_action_payloads_persist_in_runtime_state() {
        use crate::types::AnimationPreset;

        let mut state = state_with(vec![
            Action::SetTransition {
                target: "hero".into(),
                enter: Some(AnimationPreset::Custom("soft-enter".into())),
                exit: Some(AnimationPreset::Custom("soft-exit".into())),
                duration: 0.3,
            },
            Action::MiniAvatar {
                image: "avatar.webp".into(),
            },
            Action::Animate {
                target: "bg-main".into(),
                preset: AnimationPreset::Custom("ambient-drift".into()),
                duration: 0.4,
            },
        ]);
        let program = Arc::clone(&state.program);

        assert_eq!(step(&mut state), StepResult::AwaitPresentation);
        assert!(Arc::ptr_eq(&program, &state.program));
        assert_eq!(state.mini_avatar.as_deref(), Some("avatar.webp"));
        assert!(matches!(
            state.transition_rules["hero"].enter.as_ref(),
            Some(AnimationPreset::Custom(name)) if name == "soft-enter"
        ));
        assert!(matches!(
            state.bg_animation.as_ref().map(|animation| &animation.preset),
            Some(AnimationPreset::Custom(name)) if name == "ambient-drift"
        ));
    }

    #[test]
    fn rich_text_input_and_textbox_commands_share_runtime_state() {
        let mut state = state_with(vec![
            Action::SetTextbox {
                visible: false,
                auto: true,
            },
            Action::Say {
                speaker: "A".into(),
                text: "[蟹](かに)[色](color=#fff)".into(),
                options: SayOptions::default(),
            },
            Action::UserInput {
                variable: "name".into(),
                title: "Name".into(),
                button: "OK".into(),
            },
        ]);
        assert_eq!(step(&mut state), StepResult::AwaitClick);
        let dialogue = state.dialogue.as_ref().unwrap();
        assert_eq!(dialogue.text, "蟹色");
        assert_eq!(dialogue.markup, "[蟹](かに)[色](color=#fff)");
        assert!(!state.textbox_hidden);
        advance(&mut state);
        assert_eq!(step(&mut state), StepResult::AwaitInput);
        state.user_input.as_mut().unwrap().value = "小夜".into();
        assert!(submit_user_input(&mut state));
        assert_eq!(state.vars["name"], crate::Value::Str("小夜".into()));
    }
}
