use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use crabgal_core::step;
use crabgal_core::{Program, State};
use crabgal_loader::DiagnosticLevel;

use crate::runtime::input::InputActions;
use crate::runtime::resources::{
    AssetLoadingGate, ContentProjectResource, GameState, LocalAssetManifest, LocalSceneAssets,
    ProjectRoot, ScriptLanguages, ScriptWatcherResource,
};
use crate::storage::settings::RuntimeSettings;
use crate::ui::control_bar::{ButtonAction, SkipMode, ToggleStates};
use crate::ui::input_scope::UiInputScope;

#[derive(Default)]
struct TypewriterClock {
    scene: String,
    cursor: usize,
    fractional_chars: f64,
}

#[derive(SystemParam)]
pub struct TickContext<'w, 's> {
    time: Res<'w, Time>,
    state: ResMut<'w, GameState>,
    settings: ResMut<'w, RuntimeSettings>,
    project_root: Res<'w, ProjectRoot>,
    content: Res<'w, ContentProjectResource>,
    languages: Res<'w, ScriptLanguages>,
    actions: Res<'w, InputActions>,
    watcher: Option<Res<'w, ScriptWatcherResource>>,
    asset_manifest: ResMut<'w, LocalAssetManifest>,
    toggles: ResMut<'w, ToggleStates>,
    buttons: Query<'w, 's, (&'static Interaction, Option<&'static ButtonAction>), With<Button>>,
    input_scope: Res<'w, UiInputScope>,
    loading: Res<'w, AssetLoadingGate>,
    auto_timer: Local<'s, f64>,
    typewriter_clock: Local<'s, TypewriterClock>,
}

/// Advances input, text timing, script hot reload, and transition state.
pub fn tick(mut context: TickContext) {
    let delta_seconds = context.time.delta_secs_f64();
    let mut state_changed = reload_scripts_if_changed(
        &context.watcher,
        &context.content,
        &context.languages,
        context.state.bypass_change_detection(),
        &mut context.asset_manifest,
    );
    if context.loading.blocked {
        context.toggles.skip = false;
        if state_changed {
            context.state.set_changed();
        }
        return;
    }
    if *context.input_scope != UiInputScope::Stage {
        context.toggles.skip = false;
        if state_changed {
            context.state.set_changed();
        }
        return;
    }
    if update_toggle_shortcuts(
        &context.actions,
        &mut context.toggles,
        &mut context.settings,
        &mut context.auto_timer,
    ) && let Err(error) =
        crate::storage::settings::persist(&context.settings, &context.project_root)
    {
        log::error!("failed to persist skip mode: {error:#}");
    }
    let presentation_was_blocked = context.state.presentation_blocked();
    let presentation_advance =
        advance_requested(&context.actions, &context.buttons, context.toggles.hide);
    state_changed |= update_transitions(
        context.state.bypass_change_detection(),
        delta_seconds as f32,
        presentation_advance,
    );
    if presentation_was_blocked {
        context.toggles.skip = false;
        if !context.state.presentation_blocked() {
            step::step(context.state.bypass_change_detection());
            state_changed = true;
        }
        if state_changed {
            context.state.set_changed();
        }
        return;
    }
    if context.toggles.skip {
        state_changed |= skip_once(
            context.state.bypass_change_detection(),
            &mut context.toggles,
        );
        if state_changed {
            context.state.set_changed();
        }
        return;
    }

    state_changed |= update_typewriter(
        context.state.bypass_change_detection(),
        delta_seconds,
        context.settings.typewriter_speed,
        &mut context.typewriter_clock,
    );
    state_changed |= update_notend(context.state.bypass_change_detection());
    state_changed |= update_auto_mode(
        context.state.bypass_change_detection(),
        context.toggles.auto,
        delta_seconds,
        context.settings.auto_delay,
        &mut context.auto_timer,
    );

    if advance_requested(&context.actions, &context.buttons, context.toggles.hide) {
        state_changed |= advance_once(context.state.bypass_change_detection());
        *context.auto_timer = 0.0;
    }

    if state_changed {
        context.state.set_changed();
    }
}

fn update_notend(state: &mut State) -> bool {
    let should_advance = state.dialogue.as_ref().is_some_and(|dialogue| {
        dialogue.auto_advance && dialogue.visible_chars >= dialogue.text.chars().count()
    });
    if should_advance {
        step::advance(state);
        step::step(state);
    }
    should_advance
}

fn update_toggle_shortcuts(
    actions: &InputActions,
    toggles: &mut ToggleStates,
    settings: &mut RuntimeSettings,
    auto_timer: &mut f64,
) -> bool {
    let mut settings_changed = false;
    if actions.skip_pressed {
        toggles.skip = true;
    }
    if actions.skip_released {
        toggles.skip = false;
    }
    if actions.toggle_auto {
        toggles.auto = !toggles.auto;
        *auto_timer = 0.0;
    }
    if actions.toggle_skip || actions.toggle_skip_mode {
        if actions.toggle_skip_mode {
            toggles.skip_mode = match toggles.skip_mode {
                SkipMode::Read => SkipMode::All,
                SkipMode::All => SkipMode::Read,
            };
            settings.skip_all = toggles.skip_mode == SkipMode::All;
            settings_changed = true;
            toggles.skip = false;
            log::info!("skip mode: {:?}", toggles.skip_mode);
        } else {
            toggles.skip = !toggles.skip;
        }
    }
    settings_changed
}

fn reload_scripts_if_changed(
    watcher: &Option<Res<ScriptWatcherResource>>,
    content: &crabgal_loader::ContentProject,
    languages: &crabgal_loader::ScriptLanguageRegistry,
    state: &mut State,
    asset_manifest: &mut LocalAssetManifest,
) -> bool {
    let Some(watcher) = watcher else {
        return false;
    };
    let Ok(watcher) = watcher.0.lock() else {
        log::error!("script watcher lock is poisoned");
        return false;
    };
    let changes = watcher.drain();
    if changes.is_empty() {
        return false;
    }

    let Ok(scenes) = crabgal_loader::load_scenes_with(content, languages) else {
        log::error!("failed to reload scripts from configured content sources");
        return false;
    };
    asset_manifest.clear();
    let mut program_scenes = Vec::with_capacity(scenes.len());
    for scene in scenes {
        for diagnostic in &scene.diagnostics {
            let message = format!(
                "{}:{}:{}: {}",
                scene.path.display(),
                diagnostic.span.line,
                diagnostic.span.column,
                diagnostic.message
            );
            match diagnostic.level {
                DiagnosticLevel::Warning => log::warn!("{message}"),
                DiagnosticLevel::Error => log::error!("{message}"),
            }
        }
        asset_manifest.insert(
            scene.name.clone(),
            LocalSceneAssets {
                resources: scene.resources,
                sub_scenes: scene.sub_scenes,
            },
        );
        program_scenes.push((scene.name, scene.actions));
    }
    restart_after_program_reload(state, Program::from_scenes(program_scenes));
    log::info!("reloaded {} changed script file(s)", changes.len());
    true
}

/// Re-enter one scene against the new Program without carrying presentation
/// or interaction state produced by the previous script fingerprint.
///
/// Development reload keeps local/global variables and durable gallery
/// unlocks so authors can iterate near the current branch. Execution frames,
/// read positions, backlog, stage, audio and open UI interactions are rebuilt
/// from the beginning of the selected scene.
fn restart_after_program_reload(state: &mut State, program: Program) {
    let previous_scene = state.current_scene.clone();
    let was_ended = state.ended;
    let vars = std::mem::take(&mut state.vars);
    let global_vars = std::mem::take(&mut state.global_vars);
    let unlocked_cg = std::mem::take(&mut state.unlocked_cg);
    let unlocked_bgm = std::mem::take(&mut state.unlocked_bgm);

    let mut restarted = State {
        vars,
        global_vars,
        unlocked_cg,
        unlocked_bgm,
        ..State::new()
    };
    restarted.install_program(program);
    restarted.current_scene = if restarted.program.contains_scene(&previous_scene) {
        previous_scene
    } else {
        crate::scene::entry_scene(&restarted)
    };
    restarted.ended = was_ended || restarted.current_scene.is_empty();
    restarted.effect_queue.push(crabgal_core::EffectEvent::Stop);
    if !restarted.ended {
        step::step(&mut restarted);
    }
    *state = restarted;
}

fn skip_once(state: &mut State, toggles: &mut ToggleStates) -> bool {
    if toggles.skip_mode == SkipMode::Read && !state.current_dialogue_is_read() {
        toggles.skip = false;
        return false;
    }
    if let Some(dialogue) = &mut state.dialogue {
        let target = dialogue.text.chars().count();
        if dialogue.visible_chars < target {
            dialogue.visible_chars = target;
            return true;
        }
        step::advance(state);
    }
    step::step(state);
    true
}

fn update_typewriter(
    state: &mut State,
    delta_seconds: f64,
    chars_per_second: f64,
    clock: &mut TypewriterClock,
) -> bool {
    let dialogue_changed = clock.scene != state.current_scene || clock.cursor != state.cursor;
    if dialogue_changed {
        clock.scene.clone_from(&state.current_scene);
        clock.cursor = state.cursor;
        clock.fractional_chars = 0.0;
    }

    let Some(dialogue) = &mut state.dialogue else {
        clock.fractional_chars = 0.0;
        return false;
    };
    let target = dialogue.text.chars().count();
    // WebGAL K starts the first glyph at delay 0. Avoid making a new line
    // feel unresponsive while waiting for the first full character period.
    if dialogue_changed && target > 0 {
        let previous = dialogue.visible_chars;
        dialogue.visible_chars = dialogue.visible_chars.max(1);
        return dialogue.visible_chars != previous;
    }
    if dialogue.visible_chars < target {
        let exact_chars = clock.fractional_chars + delta_seconds * chars_per_second.max(0.0);
        let added = exact_chars.floor() as usize;
        clock.fractional_chars = exact_chars.fract();
        let previous = dialogue.visible_chars;
        dialogue.visible_chars = (dialogue.visible_chars + added).min(target);
        return dialogue.visible_chars != previous;
    }
    false
}

fn update_auto_mode(
    state: &mut State,
    enabled: bool,
    delta_seconds: f64,
    delay: f64,
    timer: &mut f64,
) -> bool {
    if !enabled {
        *timer = 0.0;
        return false;
    }

    let ready = state
        .dialogue
        .as_ref()
        .is_none_or(|dialogue| dialogue.visible_chars >= dialogue.text.chars().count());
    if !ready {
        *timer = 0.0;
        return false;
    }

    *timer += delta_seconds;
    if *timer >= delay {
        *timer = 0.0;
        if state.dialogue.is_some() {
            step::advance(state);
        }
        step::step(state);
        return true;
    }
    false
}

fn advance_requested(
    actions: &InputActions,
    buttons: &Query<(&Interaction, Option<&ButtonAction>), With<Button>>,
    content_hidden: bool,
) -> bool {
    if !actions.advance {
        return false;
    }
    !buttons.iter().any(|(interaction, action)| {
        matches!(interaction, Interaction::Pressed)
            && (!content_hidden || matches!(action, Some(ButtonAction::Hide)))
    })
}

fn advance_once(state: &mut State) -> bool {
    if let Some(dialogue) = &mut state.dialogue {
        let target = dialogue.text.chars().count();
        if dialogue.visible_chars < target {
            dialogue.visible_chars = target;
            return true;
        }
        step::advance(state);
    }
    step::step(state);
    true
}

fn update_transitions(state: &mut State, delta_seconds: f32, advance_intro: bool) -> bool {
    let mut changed = false;
    if state.wait_remaining > 0.0 {
        state.wait_remaining = (state.wait_remaining - delta_seconds).max(0.0);
        changed = true;
    }
    if let Some(intro) = &mut state.intro {
        intro.elapsed += delta_seconds;
        changed = true;
        let advance = advance_intro || (!intro.hold && intro.elapsed >= 1.6);
        if advance {
            if intro.page + 1 < intro.pages.len() {
                intro.page += 1;
                intro.elapsed = 0.0;
            } else {
                state.intro = None;
            }
        }
    }
    for sprite in state.sprites.values_mut() {
        if let Some(animation) = &mut sprite.transform_animation {
            changed = true;
            animation.elapsed = (animation.elapsed + delta_seconds).min(animation.duration);
            let progress = animation
                .easing
                .sample(animation.elapsed / animation.duration);
            sprite.transform = animation.from.lerp(animation.to, progress);
            if animation.elapsed >= animation.duration {
                sprite.transform_animation = None;
            }
        }
        if let Some(animation) = &mut sprite.animation {
            changed = true;
            animation.elapsed = (animation.elapsed + delta_seconds).min(animation.duration);
            let progress = (animation.elapsed / animation.duration).clamp(0.0, 1.0);
            sprite.transform = sample_preset(animation.base, &animation.preset, progress);
            if animation.elapsed >= animation.duration {
                let exiting = animation.remove_on_finish;
                sprite.transform = if exiting {
                    let mut transform = animation.base;
                    transform.alpha = 0.0;
                    transform
                } else {
                    animation.base
                };
                sprite.animation = None;
                if exiting {
                    sprite.entering = false;
                    sprite.transition_progress = 0.0;
                }
            }
        }
        let delta = sprite
            .transition
            .duration()
            .map_or(1.0, |duration| delta_seconds / duration.max(f32::EPSILON));
        if sprite.entering {
            if sprite.transition_progress < 1.0 {
                changed = true;
                sprite.transition_progress = (sprite.transition_progress + delta).min(1.0);
            }
        } else {
            if sprite.transition_progress > 0.0 {
                changed = true;
                sprite.transition_progress = (sprite.transition_progress - delta).max(0.0);
            }
        }
    }
    let sprite_count = state.sprites.len();
    state
        .sprites
        .retain(|_, sprite| sprite.entering || sprite.transition_progress > 0.0);
    changed |= state.sprites.len() != sprite_count;

    let transition_finished = if let Some(transition) = &mut state.bg_transition {
        changed = true;
        let delta = transition
            .kind
            .duration()
            .map_or(1.0, |duration| delta_seconds / duration.max(f32::EPSILON));
        transition.progress = (transition.progress + delta).min(1.0);
        transition.progress >= 1.0
    } else {
        false
    };
    if transition_finished {
        state.bg_transition = None;
    }

    if let Some(animation) = &mut state.bg_transform_animation {
        changed = true;
        animation.elapsed = (animation.elapsed + delta_seconds).min(animation.duration);
        let progress = animation
            .easing
            .sample(animation.elapsed / animation.duration);
        state.bg_transform = animation.from.lerp(animation.to, progress);
        if animation.elapsed >= animation.duration {
            state.bg_transform_animation = None;
        }
    }

    if let Some(animation) = &mut state.bg_animation {
        changed = true;
        animation.elapsed = (animation.elapsed + delta_seconds).min(animation.duration);
        let progress = (animation.elapsed / animation.duration).clamp(0.0, 1.0);
        state.bg_transform = sample_preset(animation.base, &animation.preset, progress);
        if animation.elapsed >= animation.duration {
            let exiting = animation.remove_on_finish;
            state.bg_transform = animation.base;
            state.bg_animation = None;
            if exiting {
                state.bg = None;
            }
        }
    }

    let avatar_delta = delta_seconds * 3.0;
    if state.mini_avatar.is_some() {
        if state.mini_avatar_progress < 1.0 {
            changed = true;
            state.mini_avatar_progress = (state.mini_avatar_progress + avatar_delta).min(1.0);
        }
    } else {
        if state.mini_avatar_progress > 0.0 {
            changed = true;
            state.mini_avatar_progress = (state.mini_avatar_progress - avatar_delta).max(0.0);
        }
    }
    changed
}

fn sample_preset(
    base: crabgal_core::SpriteTransform,
    preset: &crabgal_core::AnimationPreset,
    progress: f32,
) -> crabgal_core::SpriteTransform {
    use crabgal_core::AnimationPreset;
    let mut result = base;
    let eased = 1.0 - (1.0 - progress).powi(3);
    match preset {
        AnimationPreset::Enter => result.alpha *= eased,
        AnimationPreset::Exit => result.alpha *= 1.0 - progress * progress,
        AnimationPreset::EnterFromBottom => {
            result.offset_y += 180.0 * (1.0 - eased);
            result.alpha *= eased;
        }
        AnimationPreset::EnterFromLeft => {
            result.offset_x -= 220.0 * (1.0 - eased);
            result.alpha *= eased;
        }
        AnimationPreset::EnterFromRight => {
            result.offset_x += 220.0 * (1.0 - eased);
            result.alpha *= eased;
        }
        AnimationPreset::Shake => {
            result.offset_x +=
                (progress * std::f32::consts::TAU * 8.0).sin() * (1.0 - progress) * 24.0;
        }
        AnimationPreset::MoveFrontAndBack => {
            let scale = 1.0 + (progress * std::f32::consts::PI).sin() * 0.08;
            result.scale_x *= scale;
            result.scale_y *= scale;
        }
        AnimationPreset::Blur => {
            result.blur += (progress * std::f32::consts::PI).sin() * 12.0;
        }
        AnimationPreset::ShockwaveIn | AnimationPreset::ShockwaveOut => {
            let wave = (progress * std::f32::consts::PI).sin();
            let direction = if matches!(preset, AnimationPreset::ShockwaveIn) {
                -1.0
            } else {
                1.0
            };
            result.scale_x *= 1.0 + wave * 0.06 * direction;
            result.scale_y *= 1.0 + wave * 0.06 * direction;
            result.blur += wave * 5.0;
        }
        AnimationPreset::OldFilm
        | AnimationPreset::DotFilm
        | AnimationPreset::ReflectionFilm
        | AnimationPreset::GlitchFilm
        | AnimationPreset::RgbFilm
        | AnimationPreset::GodrayFilm
        | AnimationPreset::RemoveFilm
        | AnimationPreset::Custom(_) => {
            result.blur += (progress * std::f32::consts::PI).sin() * 2.0;
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use crabgal_core::state::Dialogue;
    use crabgal_core::{
        Action, AnimationPreset, BlendMode, Position, SpriteTransform, Transition, Value,
    };

    use super::*;

    fn dialogue_state() -> State {
        let mut state = State::new();
        state.current_scene = "main".into();
        state.cursor = 1;
        state.dialogue = Some(Dialogue {
            speaker: String::new(),
            text: "abcdefghij".into(),
            markup: "abcdefghij".into(),
            visible_chars: 0,
            vocal: None,
            volume: 1.0,
            auto_advance: false,
        });
        state
    }

    #[test]
    fn typewriter_preserves_fractional_progress() {
        let mut state = dialogue_state();
        let mut clock = TypewriterClock::default();

        for _ in 0..4 {
            update_typewriter(&mut state, 0.05, 10.0, &mut clock);
        }

        assert_eq!(state.dialogue.unwrap().visible_chars, 2);
    }

    #[test]
    fn typewriter_reveals_first_character_immediately() {
        let mut state = dialogue_state();
        let mut clock = TypewriterClock::default();

        update_typewriter(&mut state, 0.0, 10.0, &mut clock);

        assert_eq!(state.dialogue.unwrap().visible_chars, 1);
    }

    #[test]
    fn skip_read_stops_at_unread_dialogue() {
        let mut state = dialogue_state();
        let mut toggles = ToggleStates {
            skip: true,
            ..default()
        };

        skip_once(&mut state, &mut toggles);

        assert!(!toggles.skip);
        assert_eq!(state.dialogue.unwrap().visible_chars, 0);
    }

    #[test]
    fn skip_all_reveals_unread_dialogue() {
        let mut state = dialogue_state();
        let mut toggles = ToggleStates {
            skip: true,
            skip_mode: SkipMode::All,
            ..default()
        };

        skip_once(&mut state, &mut toggles);

        assert!(toggles.skip);
        assert_eq!(state.dialogue.unwrap().visible_chars, 10);
    }

    #[test]
    fn custom_exit_animation_keeps_sprite_until_its_last_frame() {
        let mut state = State::new();
        state.install_program(Program::from_scenes([(
            "main".into(),
            vec![
                Action::SetTransition {
                    target: "hero".into(),
                    enter: None,
                    exit: Some(AnimationPreset::Exit),
                    duration: 1.0,
                },
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
            ],
        )]));
        state.current_scene = "main".into();

        assert_eq!(
            step::step(&mut state),
            crabgal_core::StepResult::AwaitPresentation
        );
        assert!(state.sprites.contains_key("hero"));

        update_transitions(&mut state, 0.5, false);
        let halfway = &state.sprites["hero"];
        assert!(halfway.animation.is_some());
        assert!(halfway.transform.alpha > 0.0);

        update_transitions(&mut state, 0.5, false);
        assert!(!state.sprites.contains_key("hero"));
        assert!(!state.presentation_blocked());
    }

    #[test]
    fn program_reload_rebuilds_interaction_state_from_the_new_scene() {
        let mut state = State::new();
        state.install_program(Program::from_scenes([(
            "main".into(),
            vec![Action::Say {
                speaker: "old".into(),
                text: "old line".into(),
                options: Default::default(),
            }],
        )]));
        state.current_scene = "main".into();
        state.vars.insert("route".into(), Value::Str("kept".into()));
        state.global_vars.insert("chapter".into(), Value::Int(2));
        state.unlocked_cg.insert("old.webp".into(), "Old".into());
        assert_eq!(step::step(&mut state), crabgal_core::StepResult::AwaitClick);
        state.record_dialogue(0);
        state.mark_current_dialogue_read();

        restart_after_program_reload(
            &mut state,
            Program::from_scenes([(
                "main".into(),
                vec![
                    Action::ShowBg {
                        image: "new.webp".into(),
                        transition: Transition::Instant,
                        transform: SpriteTransform::default(),
                    },
                    Action::Say {
                        speaker: "new".into(),
                        text: "new line".into(),
                        options: Default::default(),
                    },
                ],
            )]),
        );

        assert_eq!(state.current_scene, "main");
        assert_eq!(state.bg.as_deref(), Some("new.webp"));
        assert_eq!(
            state.dialogue.as_ref().map(|line| line.text.as_str()),
            Some("new line")
        );
        assert_eq!(state.vars.get("route"), Some(&Value::Str("kept".into())));
        assert!(state.global_vars.contains_key("chapter"));
        assert!(state.unlocked_cg.contains_key("old.webp"));
        assert!(state.scene_stack.is_empty());
        assert_eq!(state.backlog.len(), 1);
        assert_eq!(state.backlog[0].text, "new line");
        assert_eq!(
            state.backlog[0].snapshot.program_fingerprint,
            state.program_fingerprint
        );
        assert!(state.read_dialogues.is_empty());
        assert_eq!(state.effect_queue, [crabgal_core::EffectEvent::Stop]);
    }
}
