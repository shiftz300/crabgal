use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use crabgal_core::step;
use crabgal_core::{Program, State};
use crabgal_loader::DiagnosticLevel;

use crate::runtime::platform::InputActions;
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
    next_pause: usize,
    pause: TypewriterPause,
}

#[derive(Default)]
enum TypewriterPause {
    #[default]
    Idle,
    Timed(f64),
    Input,
}

#[derive(SystemParam)]
pub struct TickContext<'w, 's> {
    time: Res<'w, Time>,
    state: ResMut<'w, GameState>,
    settings: ResMut<'w, RuntimeSettings>,
    project_root: Res<'w, ProjectRoot>,
    content: Res<'w, ContentProjectResource>,
    config: ResMut<'w, crate::runtime::resources::GameConfigResource>,
    languages: Res<'w, ScriptLanguages>,
    actions: Res<'w, InputActions>,
    watcher: Option<Res<'w, ScriptWatcherResource>>,
    asset_manifest: ResMut<'w, LocalAssetManifest>,
    toggles: ResMut<'w, ToggleStates>,
    buttons: Query<
        'w,
        's,
        (
            &'static Interaction,
            Option<&'static ButtonAction>,
            &'static ComputedNode,
            &'static UiGlobalTransform,
        ),
        With<Button>,
    >,
    windows: Query<'w, 's, &'static Window>,
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
        &mut context.config,
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
    if context.actions.skip_video {
        let before = context.state.videos.len();
        context
            .state
            .videos
            .retain(|_, video| !video.spec.skippable || video.spec.looped);
        state_changed |= before != context.state.videos.len();
    }
    let presentation_advance = advance_requested(
        &context.actions,
        &context.buttons,
        &context.windows,
        context.toggles.hide,
    );
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

    if advance_requested(
        &context.actions,
        &context.buttons,
        &context.windows,
        context.toggles.hide,
    ) {
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
    config: &mut crate::runtime::resources::GameConfigResource,
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

    let cursor_changed = changes
        .iter()
        .any(|path| content.is_debug_cursor_change(path));
    let source_change_count = changes
        .iter()
        .filter(|path| !content.is_debug_cursor_change(path))
        .count();

    let mut changed = false;
    if source_change_count > 0 {
        changed |= reload_project_sources(content, languages, state, asset_manifest, config);
        if changed {
            log::info!("reloaded {source_change_count} changed project source(s)");
        }
    }
    if cursor_changed {
        changed |= sync_editor_cursor(content, state, asset_manifest);
    }
    changed
}

fn reload_project_sources(
    content: &crabgal_loader::ContentProject,
    languages: &crabgal_loader::ScriptLanguageRegistry,
    state: &mut State,
    asset_manifest: &mut LocalAssetManifest,
    config: &mut crate::runtime::resources::GameConfigResource,
) -> bool {
    let refreshed_config = match content.reload_config() {
        Ok(config) => config,
        Err(error) => {
            log::error!("failed to reload project config: {error:#}");
            return false;
        }
    };
    let Ok(scenes) = crabgal_loader::load_scenes_with(content, languages) else {
        log::error!("failed to reload scripts from configured content sources");
        return false;
    };
    if let Some(refreshed_config) = refreshed_config {
        config.0 = refreshed_config;
    }
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
                action_spans: scene.action_spans,
            },
        );
        program_scenes.push((scene.name, scene.actions));
    }
    restart_after_program_reload(state, Program::from_scenes(program_scenes));
    true
}

pub(crate) fn sync_editor_cursor(
    content: &crabgal_loader::ContentProject,
    state: &mut State,
    asset_manifest: &LocalAssetManifest,
) -> bool {
    sync_editor_cursor_inner(content, state, asset_manifest, false)
}

/// Rebuild an editor preview even when its source cursor did not move.
///
/// A host run button uses this path so it can restart crabgal without entering
/// the editor's own debug runtime.
pub(crate) fn restart_editor_cursor(
    content: &crabgal_loader::ContentProject,
    state: &mut State,
    asset_manifest: &LocalAssetManifest,
) -> bool {
    sync_editor_cursor_inner(content, state, asset_manifest, true)
}

fn sync_editor_cursor_inner(
    content: &crabgal_loader::ContentProject,
    state: &mut State,
    asset_manifest: &LocalAssetManifest,
    force: bool,
) -> bool {
    let cursor = match content.debug_cursor() {
        Ok(Some(cursor)) => cursor,
        Ok(None) => return false,
        Err(error) => {
            log::warn!("failed to read editor cursor: {error:#}");
            return false;
        }
    };
    sync_editor_position_inner(
        state,
        asset_manifest,
        &cursor.scene,
        cursor.source_step,
        force,
    )
}

/// Rebuild an editor preview at a cursor captured from the host's live UI.
///
/// LetsGal persists its cursor asynchronously, so a run-button click can be
/// newer than `.studio/state.json`. The bridge supplies that live cursor while
/// keeping the VM and replay logic adapter-neutral.
pub(crate) fn restart_editor_position(
    state: &mut State,
    asset_manifest: &LocalAssetManifest,
    scene_name: &str,
    source_step: usize,
) -> bool {
    sync_editor_position_inner(state, asset_manifest, scene_name, source_step, true)
}

fn sync_editor_position_inner(
    state: &mut State,
    asset_manifest: &LocalAssetManifest,
    scene_name: &str,
    source_step: usize,
    force: bool,
) -> bool {
    let Some(scene) = asset_manifest.get(scene_name) else {
        log::warn!("editor selected unknown fragment {scene_name:?}");
        return false;
    };
    let target = scene
        .action_spans
        .iter()
        .position(|span| span.line > source_step)
        .unwrap_or(scene.action_spans.len());
    if !force && !state.ended && state.current_scene == scene_name && state.cursor == target {
        return false;
    }

    let new_preview = || State {
        program: state.program.clone(),
        program_fingerprint: state.program_fingerprint,
        vars: state.vars.clone(),
        global_vars: state.global_vars.clone(),
        read_dialogues: state.read_dialogues.clone(),
        unlocked_cg: state.unlocked_cg.clone(),
        unlocked_bgm: state.unlocked_bgm.clone(),
        ..State::new()
    };
    let mut preview = new_preview();
    preview.current_scene = crate::scene::entry_scene(&preview);
    preview.ended = false;

    // Editor previews reconstruct state from the project entry through the
    // selected block. Replaying only the selected fragment loses the scene,
    // characters and audio inherited from earlier chapters and is the reason
    // later chapters intermittently appeared to have missing resources.
    if !seek_editor_state(&mut preview, scene_name, target) {
        // Some editor-only/title fragments are deliberately unreachable from
        // crabgal's normal entry. They still need direct block inspection.
        preview = new_preview();
        preview.current_scene = scene_name.to_owned();
        preview.ended = false;
        let _ = seek_editor_state(&mut preview, scene_name, target);
    }
    log::info!(
        "editor seek · fragment {} · block {}",
        scene_name,
        source_step
    );
    *state = preview;
    true
}

const MAX_EDITOR_REPLAY_STEPS: usize = 65_536;

fn seek_editor_state(preview: &mut State, target_scene: &str, target: usize) -> bool {
    for _ in 0..MAX_EDITOR_REPLAY_STEPS {
        if preview.current_scene == target_scene && preview.cursor >= target {
            return true;
        }
        match step::step_until_cursor(preview, target_scene, target) {
            crabgal_core::StepResult::AwaitClick => step::advance(preview),
            crabgal_core::StepResult::AwaitPresentation => {
                while preview.presentation_blocked() {
                    update_transitions(preview, 86_400.0, true);
                }
            }
            crabgal_core::StepResult::AwaitInput => {
                let _ = step::submit_user_input(preview);
            }
            crabgal_core::StepResult::AwaitChoice => {
                let direct = preview.menu.as_ref().and_then(|menu| {
                    menu.choices.iter().position(|choice| {
                        choice.enabled
                            && match &choice.target {
                                crabgal_core::ChoiceTarget::ChangeScene(scene)
                                | crabgal_core::ChoiceTarget::CallScene(scene) => {
                                    scene == target_scene
                                }
                                crabgal_core::ChoiceTarget::Label(_) => false,
                            }
                    })
                });
                let fallback = preview
                    .menu
                    .as_ref()
                    .and_then(|menu| menu.choices.iter().position(|choice| choice.enabled));
                let Some(index) = direct.or(fallback) else {
                    return false;
                };
                step::select_choice(preview, index);
            }
            crabgal_core::StepResult::EndOfScene => {
                return preview.current_scene == target_scene && preview.cursor >= target;
            }
            crabgal_core::StepResult::ExecutionLimit => return false,
        }
    }
    log::warn!("editor seek exceeded the deterministic replay limit");
    false
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
        clock.next_pause = 0;
        clock.pause = TypewriterPause::Idle;
    }

    let Some(dialogue) = &mut state.dialogue else {
        clock.fractional_chars = 0.0;
        return false;
    };
    let target = dialogue.text.chars().count();
    if dialogue.visible_chars >= target {
        clock.next_pause = dialogue.pauses.len();
        clock.pause = TypewriterPause::Idle;
        return false;
    }
    // WebGAL K starts the first glyph at delay 0. Avoid making a new line
    // feel unresponsive while waiting for the first full character period.
    if dialogue_changed && target > 0 {
        if dialogue
            .pauses
            .get(clock.next_pause)
            .is_some_and(|pause| pause.at == 0)
        {
            start_inline_pause(dialogue, clock);
            return false;
        }
        let previous = dialogue.visible_chars;
        let pause_at = dialogue
            .pauses
            .get(clock.next_pause)
            .map_or(target, |pause| pause.at);
        dialogue.visible_chars = dialogue.visible_chars.max(1).min(pause_at);
        if dialogue.visible_chars == pause_at && pause_at < target {
            start_inline_pause(dialogue, clock);
        }
        return dialogue.visible_chars != previous;
    }

    let speed = chars_per_second.max(0.0);
    let mut remaining = delta_seconds.max(0.0);
    let mut changed = false;
    let iteration_limit = dialogue.pauses.len().saturating_mul(2).saturating_add(2);
    for _ in 0..iteration_limit {
        match &mut clock.pause {
            TypewriterPause::Timed(wait) if remaining < *wait => {
                *wait -= remaining;
                break;
            }
            TypewriterPause::Timed(wait) => {
                remaining -= *wait;
                clock.pause = TypewriterPause::Idle;
                clock.next_pause += 1;
                continue;
            }
            TypewriterPause::Input => break,
            TypewriterPause::Idle => {}
        }

        if dialogue
            .pauses
            .get(clock.next_pause)
            .is_some_and(|pause| pause.at <= dialogue.visible_chars)
        {
            start_inline_pause(dialogue, clock);
            continue;
        }
        if remaining <= 0.0 || speed <= 0.0 {
            break;
        }

        let pause_at = dialogue
            .pauses
            .get(clock.next_pause)
            .map_or(target, |pause| pause.at.min(target));
        let capacity = pause_at.saturating_sub(dialogue.visible_chars);
        if capacity == 0 {
            start_inline_pause(dialogue, clock);
            continue;
        }
        let exact_chars = clock.fractional_chars + remaining * speed;
        let added = exact_chars.floor() as usize;
        if added < capacity {
            if added > 0 {
                dialogue.visible_chars += added;
                changed = true;
            }
            clock.fractional_chars = exact_chars.fract();
            break;
        }

        let seconds_used = (capacity as f64 - clock.fractional_chars).max(0.0) / speed;
        remaining = (remaining - seconds_used).max(0.0);
        dialogue.visible_chars = pause_at;
        clock.fractional_chars = 0.0;
        changed = true;
        if pause_at >= target {
            break;
        }
        start_inline_pause(dialogue, clock);
    }
    changed
}

fn start_inline_pause(dialogue: &crabgal_core::state::Dialogue, clock: &mut TypewriterClock) {
    clock.pause = match dialogue.pauses[clock.next_pause].duration {
        Some(seconds) => TypewriterPause::Timed(f64::from(seconds.max(0.0))),
        None => TypewriterPause::Input,
    };
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
    buttons: &Query<
        (
            &Interaction,
            Option<&ButtonAction>,
            &ComputedNode,
            &UiGlobalTransform,
        ),
        With<Button>,
    >,
    windows: &Query<&Window>,
    content_hidden: bool,
) -> bool {
    if !actions.advance {
        return false;
    }
    if actions.pointer_advance
        && buttons
            .iter()
            .any(|(interaction, _, _, _)| !matches!(interaction, Interaction::None))
    {
        return false;
    }
    if actions.pointer_advance
        && windows
            .single()
            .ok()
            .and_then(Window::physical_cursor_position)
            .is_some_and(|cursor| {
                buttons.iter().any(|(_, _, node, transform)| {
                    point_inside_rect(cursor, transform.translation, node.size())
                })
            })
    {
        return false;
    }
    !buttons.iter().any(|(interaction, action, _, _)| {
        matches!(interaction, Interaction::Pressed)
            && (!content_hidden || matches!(action, Some(ButtonAction::Hide)))
    })
}

fn point_inside_rect(point: Vec2, center: Vec2, size: Vec2) -> bool {
    size.x > 0.0
        && size.y > 0.0
        && (point.x - center.x).abs() <= size.x * 0.5
        && (point.y - center.y).abs() <= size.y * 0.5
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
    if (state.curtain.current - state.curtain.target).abs() > f32::EPSILON {
        changed = true;
        state.curtain.elapsed = (state.curtain.elapsed + delta_seconds).min(state.curtain.duration);
        let progress = if state.curtain.duration <= f32::EPSILON {
            1.0
        } else {
            (state.curtain.elapsed / state.curtain.duration).clamp(0.0, 1.0)
        };
        let eased = progress * progress * (3.0 - 2.0 * progress);
        state.curtain.current =
            state.curtain.from + (state.curtain.target - state.curtain.from) * eased;
        if progress >= 1.0 {
            state.curtain.current = state.curtain.target;
            state.curtain.blocking = false;
        }
    }
    if let Some(text) = &mut state.floating_text {
        changed = true;
        text.elapsed += delta_seconds;
        if text.elapsed >= text.duration() {
            state.floating_text = None;
        }
    }
    for effect in state.particle_effects.values_mut() {
        if effect.fading_out || effect.elapsed < effect.effect.fade_in {
            effect.elapsed += delta_seconds;
            changed = true;
        }
    }
    let effect_count = state.particle_effects.len();
    state
        .particle_effects
        .retain(|_, effect| !effect.finished());
    changed |= state.particle_effects.len() != effect_count;

    if let Some(mut animation) = state.camera_effect_animation.take() {
        changed = true;
        animation.elapsed = (animation.elapsed + delta_seconds).min(animation.duration);
        let progress = animation
            .easing
            .sample(animation.elapsed / animation.duration.max(f32::EPSILON));
        state.camera_effect = animation.from.interpolate(&animation.to, progress);
        if animation.elapsed < animation.duration {
            state.camera_effect_animation = Some(animation);
        } else {
            state.camera_effect = animation.to;
        }
    }

    if let Some(mut animation) = state.camera_transform_animation.take() {
        changed = true;
        animation.elapsed = (animation.elapsed + delta_seconds).min(animation.duration);
        let progress = animation
            .easing
            .sample(animation.elapsed / animation.duration.max(f32::EPSILON));
        state.camera_transform = animation.from.lerp(animation.to, progress);
        if animation.elapsed < animation.duration {
            state.camera_transform_animation = Some(animation);
        } else {
            state.camera_transform = animation.to;
        }
    }

    let shake_finished = if let Some(shake) = &mut state.camera_shake {
        use crabgal_core::{CameraShakeAxis, CameraShakeFalloff};

        changed = true;
        shake.elapsed = (shake.elapsed + delta_seconds).min(shake.spec.duration);
        let progress = shake.elapsed / shake.spec.duration.max(f32::EPSILON);
        let envelope = match shake.spec.falloff {
            CameraShakeFalloff::Linear => 1.0 - progress,
            CameraShakeFalloff::Exponential => (1.0 - progress).powi(2),
        };
        let phase = std::f32::consts::TAU * shake.spec.frequency * shake.elapsed;
        let amplitude = shake.spec.amplitude * envelope;
        shake.offset_x = if shake.spec.axis == CameraShakeAxis::Y {
            0.0
        } else {
            amplitude * phase.sin()
        };
        shake.offset_y = if shake.spec.axis == CameraShakeAxis::X {
            0.0
        } else {
            amplitude * (phase + std::f32::consts::FRAC_PI_3).sin()
        };
        shake.elapsed >= shake.spec.duration
    } else {
        false
    };
    if shake_finished {
        state.camera_shake = None;
    }

    for video in state.videos.values_mut() {
        video.elapsed += delta_seconds;
        if video.stopping {
            changed = true;
            let fade = video.fade_out.max(f32::EPSILON);
            video.opacity = (video.opacity - video.spec.alpha * delta_seconds / fade).max(0.0);
        }
    }
    let video_count = state.videos.len();
    state.videos.retain(|_, video| video.opacity > 0.0);
    changed |= state.videos.len() != video_count;

    for sprite in state.sprites.values_mut() {
        changed |= sprite.keyframe_animation.is_some();
        let keyframes_finished = sprite.keyframe_animation.as_mut().is_some_and(|animation| {
            advance_keyframes(&mut sprite.transform, animation, delta_seconds)
        });
        if keyframes_finished {
            sprite.keyframe_animation = None;
        }
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
                    preset_final_transform(animation.base, &animation.preset)
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
        if state
            .bg_transition
            .as_ref()
            .is_some_and(|transition| transition.to.is_empty())
        {
            state.bg_camera_distance = None;
        }
        state.bg_transition = None;
    }

    changed |= state.bg_keyframe_animation.is_some();
    let bg_keyframes_finished = state
        .bg_keyframe_animation
        .as_mut()
        .is_some_and(|animation| {
            advance_keyframes(&mut state.bg_transform, animation, delta_seconds)
        });
    if bg_keyframes_finished {
        state.bg_keyframe_animation = None;
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
            state.bg_transform = preset_final_transform(animation.base, &animation.preset);
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

fn advance_keyframes(
    transform: &mut crabgal_core::SpriteTransform,
    animation: &mut crabgal_core::state::KeyframeAnimation,
    delta_seconds: f32,
) -> bool {
    if animation.frames.is_empty() {
        return true;
    }
    let mut remaining = delta_seconds.max(0.0);
    // A large frame delta can cross several short segments. The bound also
    // prevents a malformed zero-duration looping timeline from spinning.
    let limit = animation.frames.len().saturating_mul(2).max(1);
    for _ in 0..limit {
        let frame = &mut animation.frames[animation.index];
        let available = (frame.duration - frame.elapsed).max(0.0);
        let consumed = remaining.min(available);
        frame.elapsed += consumed;
        remaining -= consumed;
        let progress = if frame.duration <= f32::EPSILON {
            1.0
        } else {
            frame.easing.sample(frame.elapsed / frame.duration)
        };
        *transform = frame.from.lerp(frame.to, progress);
        if frame.elapsed + f32::EPSILON < frame.duration {
            return false;
        }

        animation.index += 1;
        if animation.index == animation.frames.len() {
            if animation.repeat_remaining == 0 {
                return true;
            }
            animation.repeat_remaining -= 1;
            animation.index = 0;
            *transform = animation.initial;
            for frame in &mut animation.frames {
                frame.elapsed = 0.0;
            }
        }
        if remaining <= f32::EPSILON && animation.frames[animation.index].duration > f32::EPSILON {
            return false;
        }
    }
    false
}

fn sample_preset(
    base: crabgal_core::SpriteTransform,
    preset: &crabgal_core::AnimationPreset,
    progress: f32,
) -> crabgal_core::SpriteTransform {
    use crabgal_core::AnimationPreset;
    let progress = progress.clamp(0.0, 1.0);
    let mut result = base;
    let eased = 1.0 - (1.0 - progress).powi(3);
    match preset {
        AnimationPreset::Enter => result.alpha *= eased,
        AnimationPreset::Exit => result.alpha *= 1.0 - progress * progress,
        AnimationPreset::EnterFromBottom => {
            result.offset_y += 220.0 * (1.0 - eased);
            result.blur += 5.0 * (1.0 - eased);
            result.alpha *= eased;
        }
        AnimationPreset::EnterFromLeft => {
            result.offset_x -= 280.0 * (1.0 - eased);
            result.blur += 5.0 * (1.0 - eased);
            result.alpha *= eased;
        }
        AnimationPreset::EnterFromRight => {
            result.offset_x += 280.0 * (1.0 - eased);
            result.blur += 5.0 * (1.0 - eased);
            result.alpha *= eased;
        }
        AnimationPreset::Shake => {
            let offset = if progress < 0.25 {
                -100.0 * (progress / 0.25)
            } else if progress < 0.75 {
                -100.0 + 200.0 * ((progress - 0.25) / 0.5)
            } else {
                100.0 * (1.0 - (progress - 0.75) / 0.25)
            };
            result.offset_x += offset;
        }
        AnimationPreset::MoveFrontAndBack => {
            let scale = 1.0 + (progress * std::f32::consts::PI).sin() * 0.15;
            result.scale_x *= scale;
            result.scale_y *= scale;
        }
        AnimationPreset::Blur => {
            result.blur += (progress * std::f32::consts::PI).sin() * 4.0;
        }
        AnimationPreset::ShockwaveIn | AnimationPreset::ShockwaveOut => {}
        AnimationPreset::OldFilm
        | AnimationPreset::DotFilm
        | AnimationPreset::ReflectionFilm
        | AnimationPreset::GlitchFilm
        | AnimationPreset::RgbFilm
        | AnimationPreset::GodrayFilm
        | AnimationPreset::RemoveFilm
        | AnimationPreset::Custom(_) => {}
    }
    result
}

fn preset_final_transform(
    base: crabgal_core::SpriteTransform,
    _preset: &crabgal_core::AnimationPreset,
) -> crabgal_core::SpriteTransform {
    base
}

#[cfg(test)]
mod tests {
    use crabgal_core::state::{Dialogue, KeyframeAnimation, TransformAnimation};
    use crabgal_core::{
        Action, AnimationPreset, BlendMode, DialoguePause, Easing, Position, SpriteTransform,
        Transition, Value,
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
            pauses: Vec::new(),
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
    fn pointer_hit_test_consumes_ui_clicks_before_interaction_updates() {
        assert!(point_inside_rect(
            Vec2::new(110.0, 85.0),
            Vec2::new(100.0, 100.0),
            Vec2::new(80.0, 40.0),
        ));
        assert!(!point_inside_rect(
            Vec2::new(145.0, 85.0),
            Vec2::new(100.0, 100.0),
            Vec2::new(80.0, 40.0),
        ));
    }

    #[test]
    fn editor_seek_keeps_resources_inherited_from_an_earlier_fragment() {
        let mut state = State::new();
        state.install_program(Program::from_scenes([
            (
                "start".into(),
                vec![
                    Action::ShowBg {
                        image: "backgrounds/inherited.webp".into(),
                        transition: Transition::Instant,
                        transform: SpriteTransform::default(),
                    },
                    Action::ChangeScene("chapter-two".into()),
                ],
            ),
            (
                "chapter-two".into(),
                vec![Action::Say {
                    speaker: String::new(),
                    text: "continued".into(),
                    options: Default::default(),
                }],
            ),
        ]));
        state.current_scene = "start".into();
        state.ended = false;

        assert!(seek_editor_state(&mut state, "chapter-two", 0));
        assert_eq!(state.current_scene, "chapter-two");
        assert_eq!(state.bg.as_deref(), Some("backgrounds/inherited.webp"));
    }

    #[test]
    fn editor_seek_keeps_the_selected_dialogue_visible_after_replay() {
        let mut state = State::new();
        state.install_program(Program::from_scenes([(
            "main".into(),
            vec![Action::Say {
                speaker: "小夜".into(),
                text: "被选中的对白".into(),
                options: Default::default(),
            }],
        )]));
        state.current_scene = "main".into();
        state.ended = false;

        assert!(seek_editor_state(&mut state, "main", 1));
        assert!(state.dialogue.is_none());
        let retained = state
            .previous_dialogue
            .as_ref()
            .expect("selected dialogue must survive deterministic editor replay");
        assert_eq!(retained.speaker, "小夜");
        assert_eq!(retained.text, "被选中的对白");
        assert!(!state.textbox_hidden);
    }

    #[test]
    fn typewriter_reveals_first_character_immediately() {
        let mut state = dialogue_state();
        let mut clock = TypewriterClock::default();

        update_typewriter(&mut state, 0.0, 10.0, &mut clock);

        assert_eq!(state.dialogue.unwrap().visible_chars, 1);
    }

    #[test]
    fn typewriter_waits_at_zero_width_inline_markers() {
        let mut state = dialogue_state();
        state.dialogue.as_mut().unwrap().pauses = vec![DialoguePause {
            at: 2,
            duration: Some(1.0),
        }];
        let mut clock = TypewriterClock::default();

        update_typewriter(&mut state, 0.0, 10.0, &mut clock);
        update_typewriter(&mut state, 0.2, 10.0, &mut clock);
        assert_eq!(state.dialogue.as_ref().unwrap().visible_chars, 2);

        update_typewriter(&mut state, 0.8, 10.0, &mut clock);
        assert_eq!(state.dialogue.as_ref().unwrap().visible_chars, 2);

        update_typewriter(&mut state, 0.2, 10.0, &mut clock);
        assert!(state.dialogue.unwrap().visible_chars > 2);
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
    fn keyframe_timeline_consumes_large_frame_deltas_without_rate_dependence() {
        let initial = SpriteTransform::default();
        let mut first = initial;
        first.offset_x = 100.0;
        let mut second = first;
        second.offset_x = 160.0;
        let mut timeline = KeyframeAnimation {
            initial,
            frames: vec![
                TransformAnimation {
                    from: initial,
                    to: first,
                    elapsed: 0.0,
                    duration: 1.0,
                    easing: Easing::Linear,
                    blocking: false,
                },
                TransformAnimation {
                    from: first,
                    to: second,
                    elapsed: 0.0,
                    duration: 0.5,
                    easing: Easing::Linear,
                    blocking: false,
                },
            ],
            index: 0,
            repeat_remaining: 0,
            blocking: true,
        };
        let mut transform = initial;

        assert!(!advance_keyframes(&mut transform, &mut timeline, 0.75));
        assert_eq!(transform.offset_x, 75.0);
        assert!(advance_keyframes(&mut transform, &mut timeline, 0.75));
        assert_eq!(transform.offset_x, 160.0);
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
                    layout: crabgal_core::SpriteLayout::Natural,
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
