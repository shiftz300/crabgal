use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::winit::{UpdateMode, WinitSettings};

use crate::runtime::resources::{AssetLoadingGate, GameState};
use crate::scene::audio::AudioAnimationActivity;
use crate::ui::activity::UiAnimationActivity;
use crate::ui::control_bar::{AutoHideTiming, ToggleStates};
use crate::ui::user_input::UserInputCaretBlink;

#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum RuntimeActivity {
    #[default]
    Active,
    Idle,
    Loading,
    Background,
}

#[derive(SystemParam)]
pub(crate) struct LifecycleContext<'w, 's> {
    state: Res<'w, GameState>,
    loading: Res<'w, AssetLoadingGate>,
    ui: Res<'w, UiAnimationActivity>,
    audio: Res<'w, AudioAnimationActivity>,
    toggles: Res<'w, ToggleStates>,
    auto_hide: Res<'w, AutoHideTiming>,
    input_caret: Res<'w, UserInputCaretBlink>,
    real_time: Res<'w, Time<Real>>,
    windows: Query<'w, 's, &'static Window>,
}

pub(crate) fn update(
    context: LifecycleContext,
    mut activity: ResMut<RuntimeActivity>,
    mut winit: ResMut<WinitSettings>,
    mut virtual_time: ResMut<Time<Virtual>>,
) {
    let focused = context.windows.single().is_ok_and(|window| window.focused);
    let auto_hide = context
        .auto_hide
        .lifecycle(context.real_time.elapsed_secs(), &context.toggles);
    let reactive_wait = auto_hide.1.min(
        context
            .input_caret
            .next_toggle_in(context.real_time.elapsed_secs()),
    );
    let next = if !focused {
        RuntimeActivity::Background
    } else if context.loading.blocked {
        RuntimeActivity::Loading
    } else if core_is_animating(&context.state)
        || context.ui.0
        || context.audio.0
        || context.toggles.auto
        || context.toggles.skip
        || auto_hide.0
    {
        RuntimeActivity::Active
    } else {
        RuntimeActivity::Idle
    };

    let focused_mode = match next {
        RuntimeActivity::Active | RuntimeActivity::Loading => UpdateMode::Continuous,
        RuntimeActivity::Idle | RuntimeActivity::Background => {
            UpdateMode::reactive_low_power(reactive_wait)
        }
    };
    if winit.focused_mode != focused_mode {
        winit.focused_mode = focused_mode;
    }
    let unfocused_mode = UpdateMode::reactive_low_power(std::time::Duration::MAX);
    if winit.unfocused_mode != unfocused_mode {
        winit.unfocused_mode = unfocused_mode;
    }
    if *activity != next {
        *activity = next;
    }
    if matches!(next, RuntimeActivity::Idle | RuntimeActivity::Background) {
        virtual_time.pause();
    } else {
        virtual_time.unpause();
    }
}

fn core_is_animating(state: &GameState) -> bool {
    state
        .dialogue
        .as_ref()
        .is_some_and(|dialogue| dialogue.visible_chars < dialogue.text.chars().count())
        || state.presentation_blocked()
        || state.particle_effect.is_some()
        || state.bg_transition.is_some()
        || state.bg_transform_animation.is_some()
        || state.bg_animation.is_some()
        || state.sprites.values().any(|sprite| {
            sprite.animation.is_some()
                || sprite.transform_animation.is_some()
                || (sprite.entering && sprite.transition_progress < 1.0)
                || (!sprite.entering && sprite.transition_progress > 0.0)
        })
        || (state.mini_avatar.is_some() && state.mini_avatar_progress < 1.0)
        || (state.mini_avatar.is_none() && state.mini_avatar_progress > 0.0)
}
