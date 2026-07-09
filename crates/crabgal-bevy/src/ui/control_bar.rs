// WebGAL-style control bar icon definitions and interaction.
// Both top and bottom bars spawn as children of TextBoxRoot in textbox.rs.
use bevy::prelude::*;

use crate::ui::dialog::{DialogAction, DialogRequest};

#[derive(Component)] pub(crate) struct ControlBarTop;
#[derive(Component)] pub(crate) struct ControlBarBot;

/// Identifies which button was clicked, attached to each Button entity at spawn.
#[derive(Component, Clone, Copy, Debug)]
pub(crate) enum ButtonAction {
    // Top row
    Backlog,
    Replay,
    Auto,
    Skip,
    Hide,
    Lock,
    // Bottom row
    QuickSave,
    QuickLoad,
    Save,
    Load,
    System,
    Title,
}

/// Per-button hover alpha state for CSS-like transition animation.
#[derive(Component)]
pub(crate) struct HoverAlpha {
    pub(crate) target: f32,
    pub(crate) current: f32,
    /// When true (toggle is on), target stays at 0.06 even when not hovering.
    pub(crate) active: bool,
}

impl Default for HoverAlpha {
    fn default() -> Self {
        Self { target: 0.0, current: 0.0, active: false }
    }
}

/// Toggle state for binary buttons in the top control bar.
#[derive(Resource)]
pub(crate) struct ToggleStates {
    pub auto: bool,
    pub skip: bool,
    pub hide: bool,
    /// Default: locked (control bars always visible).
    pub lock: bool,
}

impl Default for ToggleStates {
    fn default() -> Self {
        Self { auto: false, skip: false, hide: false, lock: true }
    }
}

const fn icon(codepoint: u32) -> char {
    char::from_u32(codepoint).expect("invalid icon codepoint")
}

// Top-right icon row
pub(crate) const TOP_ICONS: &[(char, &str)] = &[
    (icon(0xf3b9), "file-text"),
    (icon(0xf116), "arrow-clockwise"),
    (icon(0xf4f5), "play"),
    (icon(0xf7f4), "fast-forward"),
    (icon(0xf340), "eye-slash"),
    (icon(0xf47b), "lock"),
];

// Bottom quick menu
pub(crate) const BOT_ITEMS: &[(char, &str)] = &[
    (icon(0xf27e), crate::locale::menu::QSAVE),
    (icon(0xf281), crate::locale::menu::QLOAD),
    (icon(0xf7e4), crate::locale::menu::SAVE),
    (icon(0xf3d8), crate::locale::menu::LOAD),
    (icon(0xf789), crate::locale::menu::SYSTEM),
    (icon(0xf425), crate::locale::menu::TITLE),
];

// ── Interaction systems ──

/// Sets the target hover alpha on interaction change, respecting active toggle state.
/// Suppresses hover on all buttons except Hide when hide mode is active.
pub fn set_hover_target(
    toggles: Res<ToggleStates>,
    mut q: Query<(&Interaction, &mut HoverAlpha, Option<&ButtonAction>), Changed<Interaction>>,
) {
    for (interaction, mut ha, action) in q.iter_mut() {
        // In hide mode, only the hide button responds to hover
        if toggles.hide && !matches!(action, Some(ButtonAction::Hide)) {
            ha.target = 0.0;
            continue;
        }
        let hover = matches!(interaction, Interaction::Hovered | Interaction::Pressed);
        ha.target = if hover || ha.active { 0.06 } else { 0.0 };
    }
}

/// Smoothly lerps hover alpha towards target, applying to BackgroundColor.
pub fn animate_hover(
    time: Res<Time>,
    mut q: Query<(&mut HoverAlpha, &mut BackgroundColor)>,
) {
    let speed = 12.0;
    for (mut ha, mut bg) in q.iter_mut() {
        ha.current += (ha.target - ha.current) * speed * time.delta_secs().min(1.0);
        if ha.current < 0.002 {
            bg.0 = Color::NONE;
        } else {
            bg.0 = Color::srgba(1.0, 1.0, 1.0, ha.current);
        }
    }
}

/// Dispatches button clicks: toggle state for binary buttons,
/// save/load for bottom row, log for the rest.
pub fn handle_button_click(
    mut q: Query<(&Interaction, &ButtonAction, &mut HoverAlpha), Changed<Interaction>>,
    mut toggles: ResMut<ToggleStates>,
    mut commands: Commands,
) {
    for (interaction, action, mut ha) in q.iter_mut() {
        if !matches!(interaction, Interaction::Pressed) {
            continue;
        }
        // In hide mode, only the hide button responds to clicks
        if toggles.hide && !matches!(action, ButtonAction::Hide) {
            continue;
        }
        match action {
            ButtonAction::Auto => { toggles.auto = !toggles.auto; ha.active = toggles.auto; ha.target = if toggles.auto { 0.06 } else { 0.0 }; }
            ButtonAction::Skip => { toggles.skip = !toggles.skip; ha.active = toggles.skip; ha.target = if toggles.skip { 0.06 } else { 0.0 }; }
            ButtonAction::Hide => { toggles.hide = !toggles.hide; }
            ButtonAction::Lock => { toggles.lock = !toggles.lock; }

            ButtonAction::QuickSave => {
                commands.insert_resource(DialogRequest {
                    title: crate::locale::dialog::QSAVE_TITLE.into(),
                    left_text: crate::locale::dialog::CONFIRM.into(),
                    right_text: crate::locale::dialog::CANCEL.into(),
                    action: DialogAction::QuickSave,
                });
            }
            ButtonAction::QuickLoad => {
                commands.insert_resource(DialogRequest {
                    title: crate::locale::dialog::QLOAD_TITLE.into(),
                    left_text: crate::locale::dialog::CONFIRM.into(),
                    right_text: crate::locale::dialog::CANCEL.into(),
                    action: DialogAction::QuickLoad,
                });
            }
            ButtonAction::Title => {
                commands.insert_resource(DialogRequest {
                    title: crate::locale::dialog::TITLE_TITLE.into(),
                    left_text: crate::locale::dialog::CONFIRM.into(),
                    right_text: crate::locale::dialog::CANCEL.into(),
                    action: DialogAction::BackToTitle,
                });
            }
            _ => log::info!("[click] {:?}", action),
        }
    }
}

// ── Auto-hide control bars ──

/// Marker component for control bar containers that auto-hide.
#[derive(Component)]
pub(crate) struct AutoHideBar;

/// Marker for text nodes inside auto-hide bars whose TextColor alpha is modulated.
#[derive(Component)]
pub(crate) struct AutoHideText {
    /// Original alpha set at spawn, used as multiplier base.
    pub base_alpha: f32,
}

impl AutoHideText {
    pub fn new(base_alpha: f32) -> Self {
        Self { base_alpha }
    }
}

/// Marks a text node whose alpha is modulated by the Hide toggle (dialogue, speaker name).
#[derive(Component)]
pub(crate) struct HideContentText {
    pub base_alpha: f32,
}

impl HideContentText {
    pub fn new(base_alpha: f32) -> Self { Self { base_alpha } }
}

/// Marks a background node whose alpha is modulated by the Hide toggle.
#[derive(Component)]
pub(crate) struct HideContentBg {
    pub base_alpha: f32,
}

impl HideContentBg {
    pub fn new(base_alpha: f32) -> Self { Self { base_alpha } }
}

/// Tracks cursor inactivity and smooth alpha for control bar auto-hide.
#[derive(Resource)]
pub(crate) struct AutoHideTiming {
    /// Time::elapsed_secs() at last cursor move (or startup).
    pub last_move: f32,
    /// Previous cursor position for move detection.
    last_cursor: Option<Vec2>,
    /// Current alpha for control bar auto-hide, lerped toward target each frame.
    pub alpha: f32,
    /// Current alpha for hide-toggle content (dialogue, namebar, bg, blur).
    pub hide_alpha: f32,
    /// Alpha for hide button when hide is ON — follows idle timer regardless of lock.
    pub hide_btn_alpha: f32,
}

impl Default for AutoHideTiming {
    fn default() -> Self {
        Self { last_move: 0.0, last_cursor: None, alpha: 1.0, hide_alpha: 1.0, hide_btn_alpha: 1.0 }
    }
}

/// Updates last_move from cursor position changes; sets alpha/hide_alpha targets.
pub fn auto_hide_tick(
    time: Res<Time>,
    mut timing: ResMut<AutoHideTiming>,
    toggles: Res<ToggleStates>,
    win: Query<&Window>,
) {
    let now = time.elapsed_secs();
    let dt = time.delta_secs().min(1.0);
    if timing.last_move < 0.01 {
        timing.last_move = now;
    }
    if let Ok(w) = win.single() {
        if let Some(pos) = w.cursor_position() {
            let moved = match timing.last_cursor {
                Some(prev) => (prev - pos).length_squared() > 1.0,
                None => true,
            };
            timing.last_cursor = Some(pos);
            if moved {
                timing.last_move = now;
            }
        }
    }
    let idle = now - timing.last_move;
    let speed = 5.0;

    // Control bar auto-hide (lock-aware)
    let bar_target: f32 = if toggles.lock || idle < 2.5 { 1.0 } else { 0.0 };
    timing.alpha += (bar_target - timing.alpha) * speed * dt;

    // Hide toggle: smooth lerp — bg+blur fade out, independent of mouse movement.
    let hide_target: f32 = if toggles.hide { 0.0 } else { 1.0 };
    timing.hide_alpha += (hide_target - timing.hide_alpha) * speed * dt;

    // Hide button: when hide is ON, fades after 1s idle ignoring lock state.
    let hide_btn_target: f32 = if idle < 1.0 { 1.0 } else { 0.0 };
    timing.hide_btn_alpha += (hide_btn_target - timing.hide_btn_alpha) * speed * dt;
}

/// Marker for the hide-button text that stays visible during hide mode.
#[derive(Component)]
pub(crate) struct HideButtonText;

/// Marker for the lock-button text whose icon swaps between lock/unlock.
#[derive(Component)]
pub(crate) struct LockIcon;

/// Marker for UI nodes whose behind-the-scene should be blurred.
/// The blur post-process auto-collects all ComputedNode + BlurSource entities.
#[derive(Component)]
pub(crate) struct BlurSource;

const LOCK_ICON: char = icon(0xf47b);
const UNLOCK_ICON: char = icon(0xf600);

/// Applies the lerped alpha to all AutoHideText nodes.
/// When hide is on: other buttons vanish immediately, hide button follows the 2.5s timer.
pub fn auto_hide_apply(
    timing: Res<AutoHideTiming>,
    toggles: Res<ToggleStates>,
    mut normal_q: Query<(&mut TextColor, &AutoHideText), Without<HideButtonText>>,
    mut hide_btn_q: Query<(&mut TextColor, &AutoHideText), With<HideButtonText>>,
) {
    let a = timing.alpha.clamp(0.0, 1.0);
    // When hide is on, all other buttons vanish instantly; otherwise follow auto-hide timer.
    let normal_a = if toggles.hide { 0.0 } else { a };
    for (mut tc, ht) in normal_q.iter_mut() {
        tc.0 = tc.0.with_alpha(ht.base_alpha * normal_a);
    }
    // Hide button: when hide is ON, follows idle timer ignoring lock. Otherwise normal.
    let hide_a = if toggles.hide { timing.hide_btn_alpha } else { a };
    for (mut tc, ht) in hide_btn_q.iter_mut() {
        tc.0 = tc.0.with_alpha(ht.base_alpha * hide_a);
    }
}

/// Syncs HoverAlpha active state with ToggleStates (for dialog-driven unhide etc.).
pub fn sync_toggle_highlights(
    toggles: Res<ToggleStates>,
    mut q: Query<(&ButtonAction, &mut HoverAlpha)>,
) {
    if !toggles.is_changed() {
        return;
    }
    for (action, mut ha) in q.iter_mut() {
        let active = match action {
            ButtonAction::Auto => toggles.auto,
            ButtonAction::Skip => toggles.skip,
            _ => continue,
        };
        ha.active = active;
        ha.target = if active { 0.06 } else { 0.0 };
    }
}

/// Swaps the lock icon between locked/unlocked when toggle state changes.
pub fn update_lock_icon(
    toggles: Res<ToggleStates>,
    mut q: Query<&mut Text, With<LockIcon>>,
) {
    if !toggles.is_changed() {
        return;
    }
    let ch = if toggles.lock { LOCK_ICON } else { UNLOCK_ICON };
    let s = ch.to_string();
    for mut text in q.iter_mut() {
        **text = s.clone();
    }
}

