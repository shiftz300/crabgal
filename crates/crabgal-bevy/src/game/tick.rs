use std::path::Path;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use crabgal_core::State;
use crabgal_core::step;
use crabgal_script::{DiagnosticLevel, load_scenes};

use crate::resources::{
    GameConfigResource, GameState, LocalAssetManifest, LocalSceneAssets, ProjectRoot,
    ScriptWatcherResource,
};
use crate::ui::control_bar::{ButtonAction, ToggleStates};
use crate::ui::dialog::DialogRequest;

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
    config: Res<'w, GameConfigResource>,
    project_root: Res<'w, ProjectRoot>,
    keys: Res<'w, ButtonInput<KeyCode>>,
    mouse: Res<'w, ButtonInput<MouseButton>>,
    watcher: Option<Res<'w, ScriptWatcherResource>>,
    asset_manifest: ResMut<'w, LocalAssetManifest>,
    toggles: ResMut<'w, ToggleStates>,
    buttons: Query<'w, 's, (&'static Interaction, Option<&'static ButtonAction>), With<Button>>,
    dialog: Option<Res<'w, DialogRequest>>,
    auto_timer: Local<'s, f64>,
    typewriter_clock: Local<'s, TypewriterClock>,
}

/// Advances input, text timing, script hot reload, and transition state.
pub fn tick(mut context: TickContext) {
    let delta_seconds = context.time.delta_secs_f64();
    update_toggle_shortcuts(&context.keys, &mut context.toggles, &mut context.auto_timer);
    reload_scripts_if_changed(
        &context.watcher,
        &context.project_root,
        &mut context.state,
        &mut context.asset_manifest,
    );

    if context.toggles.skip {
        skip_once(&mut context.state);
        update_transitions(&mut context.state, delta_seconds as f32);
        return;
    }

    update_typewriter(
        &mut context.state,
        delta_seconds,
        context.config.styles.typewriter_speed,
        &mut context.typewriter_clock,
    );
    update_notend(&mut context.state);
    update_auto_mode(
        &mut context.state,
        context.toggles.auto,
        delta_seconds,
        context.config.styles.auto_delay,
        &mut context.auto_timer,
    );

    if context.dialog.is_none()
        && advance_requested(
            &context.keys,
            &context.mouse,
            &context.buttons,
            context.toggles.hide,
        )
    {
        advance_once(&mut context.state);
        *context.auto_timer = 0.0;
    }

    update_transitions(&mut context.state, delta_seconds as f32);
}

fn update_notend(state: &mut State) {
    let should_advance = state.dialogue.as_ref().is_some_and(|dialogue| {
        dialogue.auto_advance && dialogue.visible_chars >= dialogue.text.chars().count()
    });
    if should_advance {
        step::advance(state);
        step::step(state);
    }
}

fn update_toggle_shortcuts(
    keys: &ButtonInput<KeyCode>,
    toggles: &mut ToggleStates,
    auto_timer: &mut f64,
) {
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
        toggles.skip = !toggles.skip;
    }
}

fn reload_scripts_if_changed(
    watcher: &Option<Res<ScriptWatcherResource>>,
    project_root: &Path,
    state: &mut State,
    asset_manifest: &mut LocalAssetManifest,
) {
    let Some(watcher) = watcher else {
        return;
    };
    let Ok(watcher) = watcher.0.lock() else {
        log::error!("script watcher lock is poisoned");
        return;
    };
    let changes = watcher.drain();
    if changes.is_empty() {
        return;
    }

    let script_dir = project_root.join("scripts");
    let Ok(scenes) = load_scenes(&script_dir) else {
        log::error!("failed to reload scripts from {}", script_dir.display());
        return;
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
}

fn skip_once(state: &mut State) {
    if let Some(dialogue) = &mut state.dialogue {
        let target = dialogue.text.chars().count();
        if dialogue.visible_chars < target {
            dialogue.visible_chars = target;
            return;
        }
        step::advance(state);
    }
    step::step(state);
}

fn update_typewriter(
    state: &mut State,
    delta_seconds: f64,
    chars_per_second: f64,
    clock: &mut TypewriterClock,
) {
    if clock.scene != state.current_scene || clock.cursor != state.cursor {
        clock.scene.clone_from(&state.current_scene);
        clock.cursor = state.cursor;
        clock.fractional_chars = 0.0;
    }

    let Some(dialogue) = &mut state.dialogue else {
        clock.fractional_chars = 0.0;
        return;
    };
    let target = dialogue.text.chars().count();
    if dialogue.visible_chars < target {
        let exact_chars = clock.fractional_chars + delta_seconds * chars_per_second.max(0.0);
        let added = exact_chars.floor() as usize;
        clock.fractional_chars = exact_chars.fract();
        dialogue.visible_chars = (dialogue.visible_chars + added).min(target);
    }
}

fn update_auto_mode(
    state: &mut State,
    enabled: bool,
    delta_seconds: f64,
    delay: f64,
    timer: &mut f64,
) {
    if !enabled {
        *timer = 0.0;
        return;
    }

    let ready = state
        .dialogue
        .as_ref()
        .is_none_or(|dialogue| dialogue.visible_chars >= dialogue.text.chars().count());
    if !ready {
        *timer = 0.0;
        return;
    }

    *timer += delta_seconds;
    if *timer >= delay {
        *timer = 0.0;
        if state.dialogue.is_some() {
            step::advance(state);
        }
        step::step(state);
    }
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

fn advance_once(state: &mut State) {
    if let Some(dialogue) = &mut state.dialogue {
        let target = dialogue.text.chars().count();
        if dialogue.visible_chars < target {
            dialogue.visible_chars = target;
            return;
        }
        step::advance(state);
    }
    step::step(state);
}

fn update_transitions(state: &mut State, delta_seconds: f32) {
    for sprite in state.sprites.values_mut() {
        if let Some(animation) = &mut sprite.transform_animation {
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
            sprite.transition_progress = (sprite.transition_progress + delta).min(1.0);
        } else {
            sprite.transition_progress = (sprite.transition_progress - delta).max(0.0);
        }
    }
    state
        .sprites
        .retain(|_, sprite| sprite.entering || sprite.transition_progress > 0.0);

    let transition_finished = if let Some(transition) = &mut state.bg_transition {
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
        state.mini_avatar_progress = (state.mini_avatar_progress + avatar_delta).min(1.0);
    } else {
        state.mini_avatar_progress = (state.mini_avatar_progress - avatar_delta).max(0.0);
    }
}

#[cfg(test)]
mod tests {
    use crabgal_core::state::Dialogue;

    use super::*;

    #[test]
    fn typewriter_preserves_fractional_progress() {
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
        let mut clock = TypewriterClock::default();

        for _ in 0..4 {
            update_typewriter(&mut state, 0.05, 10.0, &mut clock);
        }

        assert_eq!(state.dialogue.unwrap().visible_chars, 2);
    }
}
