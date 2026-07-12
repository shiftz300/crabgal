use std::path::Path;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use crabgal_core::State;
use crabgal_core::step;
use crabgal_script::{DiagnosticLevel, load_scenes};

use crate::runtime::resources::{
    AssetLoadingGate, GameState, LocalAssetManifest, LocalSceneAssets, ProjectRoot,
    ScriptWatcherResource,
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
    keys: Res<'w, ButtonInput<KeyCode>>,
    mouse: Res<'w, ButtonInput<MouseButton>>,
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
        &context.project_root,
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
        &context.keys,
        &mut context.toggles,
        &mut context.settings,
        &mut context.auto_timer,
    ) && let Err(error) =
        crate::storage::settings::persist(&context.settings, &context.project_root)
    {
        log::error!("failed to persist skip mode: {error:#}");
    }
    if context.toggles.skip {
        state_changed |= skip_once(
            context.state.bypass_change_detection(),
            &mut context.toggles,
        );
        state_changed |= update_transitions(
            context.state.bypass_change_detection(),
            delta_seconds as f32,
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
        &context.keys,
        &context.mouse,
        &context.buttons,
        context.toggles.hide,
    ) {
        state_changed |= advance_once(context.state.bypass_change_detection());
        *context.auto_timer = 0.0;
    }

    state_changed |= update_transitions(
        context.state.bypass_change_detection(),
        delta_seconds as f32,
    );
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
    keys: &ButtonInput<KeyCode>,
    toggles: &mut ToggleStates,
    settings: &mut RuntimeSettings,
    auto_timer: &mut f64,
) -> bool {
    let mut settings_changed = false;
    if keys.just_pressed(KeyCode::ControlLeft) || keys.just_pressed(KeyCode::ControlRight) {
        toggles.skip = true;
    }
    if keys.just_released(KeyCode::ControlLeft) || keys.just_released(KeyCode::ControlRight) {
        toggles.skip = false;
    }
    if keys.just_pressed(KeyCode::KeyA) {
        toggles.auto = !toggles.auto;
        *auto_timer = 0.0;
    }
    if keys.just_pressed(KeyCode::KeyS) {
        if keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]) {
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
    project_root: &Path,
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

    let script_dir = project_root.join("scripts");
    let Ok(scenes) = load_scenes(&script_dir) else {
        log::error!("failed to reload scripts from {}", script_dir.display());
        return false;
    };
    asset_manifest.clear();
    state.scenes.clear();
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
        state.scenes.insert(scene.name, scene.actions);
    }
    state.scene_stack.retain_mut(|frame| {
        let Some(scene) = state.scenes.get(&frame.scene) else {
            return false;
        };
        frame.cursor = frame.cursor.min(scene.len());
        true
    });

    if !state.scenes.contains_key(&state.current_scene) {
        state.current_scene = crate::scene::entry_scene(state);
        state.cursor = 0;
    } else if let Some(scene) = state.scenes.get(&state.current_scene) {
        state.cursor = state.cursor.min(scene.len());
    }
    step::index_labels(state);
    log::info!("reloaded {} changed script file(s)", changes.len());
    true
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
    keys: &ButtonInput<KeyCode>,
    mouse: &ButtonInput<MouseButton>,
    buttons: &Query<(&Interaction, Option<&ButtonAction>), With<Button>>,
    content_hidden: bool,
) -> bool {
    if keys.just_pressed(KeyCode::Space) || keys.just_pressed(KeyCode::Enter) {
        return true;
    }
    if !mouse.just_pressed(MouseButton::Left) {
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

fn update_transitions(state: &mut State, delta_seconds: f32) -> bool {
    let mut changed = false;
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

#[cfg(test)]
mod tests {
    use crabgal_core::state::Dialogue;

    use super::*;

    fn dialogue_state() -> State {
        let mut state = State::new();
        state.current_scene = "main".into();
        state.cursor = 1;
        state.dialogue = Some(Dialogue {
            speaker: String::new(),
            text: "abcdefghij".into(),
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
}
