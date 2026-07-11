// WebGAL-style control bar icon definitions and interaction.
// Both top and bottom bars spawn as children of TextBoxRoot in textbox.rs.
use bevy::asset::RenderAssetUsages;
use bevy::image::{CompressedImageFormats, ImageSampler, ImageType};
use bevy::prelude::*;
use crabgal_core::State;

use crate::resources::ProjectRoot;
use crate::save::QUICK_SAVE_SLOT;
use crate::ui::dialog::{DialogAction, DialogRequest};
use crate::ui::textbox::ContentRoot;

#[derive(Component)]
pub(crate) struct ControlBarTop;
#[derive(Component)]
pub(crate) struct ControlBarBot;

/// Identifies which button was clicked, attached to each Button entity at spawn.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
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

#[derive(Resource, Default)]
pub(crate) struct QuickSavePreview {
    pub(crate) state: Option<State>,
    pub(crate) image: Option<Handle<Image>>,
}

#[derive(Component)]
pub(crate) struct QuickPreviewPanel {
    pub(crate) owner: ButtonAction,
}

#[derive(Component, Default)]
pub(crate) struct QuickPreviewFade {
    pub(crate) target: f32,
    pub(crate) current: f32,
}

#[derive(Component)]
pub(crate) struct QuickPreviewSurface;

#[derive(Component)]
pub(crate) struct QuickPreviewVisual {
    pub(crate) owner: ButtonAction,
    pub(crate) base_alpha: f32,
}

#[derive(Component)]
pub(crate) struct QuickPreviewImage;

#[derive(Component)]
pub(crate) struct QuickPreviewContent;

#[derive(Component)]
pub(crate) struct QuickPreviewSpeaker;

#[derive(Component)]
pub(crate) struct QuickPreviewDialogue;

#[derive(Component)]
pub(crate) struct QuickPreviewEmpty;

type EmptyPreviewQuery<'w, 's> = Query<
    'w,
    's,
    &'static mut Node,
    (
        With<QuickPreviewEmpty>,
        Without<QuickPreviewContent>,
        Without<QuickPreviewImage>,
    ),
>;

type PreviewAnimationQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static QuickPreviewPanel,
        &'static mut QuickPreviewFade,
        &'static mut Node,
        Option<&'static QuickPreviewSurface>,
        Option<&'static mut BackgroundColor>,
        Option<&'static mut BlurStrength>,
    ),
>;

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
        Self {
            target: 0.0,
            current: 0.0,
            active: false,
        }
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
        Self {
            auto: false,
            skip: false,
            hide: false,
            lock: true,
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct ControlItem {
    pub icon: char,
    pub label: &'static str,
    pub action: ButtonAction,
}

pub(crate) const TOP_ITEMS: &[ControlItem] = &[
    ControlItem {
        icon: '\u{f3b9}',
        label: "file-text",
        action: ButtonAction::Backlog,
    },
    ControlItem {
        icon: '\u{f116}',
        label: "arrow-clockwise",
        action: ButtonAction::Replay,
    },
    ControlItem {
        icon: '\u{f4f5}',
        label: "play",
        action: ButtonAction::Auto,
    },
    ControlItem {
        icon: '\u{f7f4}',
        label: "fast-forward",
        action: ButtonAction::Skip,
    },
    ControlItem {
        icon: '\u{f340}',
        label: "eye-slash",
        action: ButtonAction::Hide,
    },
    ControlItem {
        icon: '\u{f47b}',
        label: "lock",
        action: ButtonAction::Lock,
    },
];

pub(crate) const BOTTOM_ITEMS: &[ControlItem] = &[
    ControlItem {
        icon: '\u{f27e}',
        label: crate::locale::menu::QSAVE,
        action: ButtonAction::QuickSave,
    },
    ControlItem {
        icon: '\u{f281}',
        label: crate::locale::menu::QLOAD,
        action: ButtonAction::QuickLoad,
    },
    ControlItem {
        icon: '\u{f7e4}',
        label: crate::locale::menu::SAVE,
        action: ButtonAction::Save,
    },
    ControlItem {
        icon: '\u{f3d8}',
        label: crate::locale::menu::LOAD,
        action: ButtonAction::Load,
    },
    ControlItem {
        icon: '\u{f789}',
        label: crate::locale::menu::SYSTEM,
        action: ButtonAction::System,
    },
    ControlItem {
        icon: '\u{f425}',
        label: crate::locale::menu::TITLE,
        action: ButtonAction::Title,
    },
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
pub fn animate_hover(time: Res<Time>, mut q: Query<(&mut HoverAlpha, &mut BackgroundColor)>) {
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
            ButtonAction::Auto => {
                toggles.auto = !toggles.auto;
                ha.active = toggles.auto;
                ha.target = if toggles.auto { 0.06 } else { 0.0 };
            }
            ButtonAction::Skip => {
                toggles.skip = !toggles.skip;
                ha.active = toggles.skip;
                ha.target = if toggles.skip { 0.06 } else { 0.0 };
            }
            ButtonAction::Hide => {
                toggles.hide = !toggles.hide;
            }
            ButtonAction::Lock => {
                toggles.lock = !toggles.lock;
            }

            ButtonAction::QuickSave => {
                commands.insert_resource(DialogRequest::confirmation(
                    crate::locale::dialog::QSAVE_TITLE,
                    DialogAction::QuickSave,
                ));
            }
            ButtonAction::QuickLoad => {
                commands.insert_resource(DialogRequest::confirmation(
                    crate::locale::dialog::QLOAD_TITLE,
                    DialogAction::QuickLoad,
                ));
            }
            ButtonAction::Title => {
                commands.insert_resource(DialogRequest::confirmation(
                    crate::locale::dialog::TITLE_TITLE,
                    DialogAction::BackToTitle,
                ));
            }
            _ => log::info!("[click] {:?}", action),
        }
    }
}

pub fn load_quick_save_preview(
    project_root: Res<ProjectRoot>,
    mut images: ResMut<Assets<Image>>,
    mut preview: ResMut<QuickSavePreview>,
) {
    preview.state = crate::save::load_game(QUICK_SAVE_SLOT, &project_root).ok();
    let path = crate::save::preview_path(&project_root, QUICK_SAVE_SLOT);
    preview.image = std::fs::read(&path)
        .map_err(anyhow::Error::from)
        .and_then(|bytes| {
            Image::from_buffer(
                &bytes,
                ImageType::Extension("png"),
                CompressedImageFormats::NONE,
                true,
                ImageSampler::default(),
                RenderAssetUsages::default(),
            )
            .map_err(anyhow::Error::from)
        })
        .map(|image| images.add(image))
        .map_err(|error| log::debug!("quick-save preview unavailable: {error:#}"))
        .ok();
}

pub fn show_quick_preview(
    buttons: Query<(&Interaction, &ButtonAction), Changed<Interaction>>,
    mut previews: Query<(&QuickPreviewPanel, &mut QuickPreviewFade, &mut Node)>,
) {
    for (interaction, action) in &buttons {
        if !matches!(action, ButtonAction::QuickSave | ButtonAction::QuickLoad) {
            continue;
        }
        for (preview, mut fade, mut node) in &mut previews {
            if preview.owner == *action {
                fade.target = if matches!(interaction, Interaction::Hovered | Interaction::Pressed)
                {
                    node.display = Display::Flex;
                    1.0
                } else {
                    0.0
                };
            }
        }
    }
}

/// Runs a short CSS-like transition for the preview surface, its content, and
/// the matching regional blur proxy. Keeping the proxy alive until fade-out is
/// complete avoids a sharp blur pop at either end of the animation.
pub fn animate_quick_previews(
    time: Res<Time>,
    mut panels: PreviewAnimationQuery,
    mut text: Query<(&QuickPreviewVisual, &mut TextColor)>,
    mut images: Query<(&QuickPreviewVisual, &mut ImageNode)>,
) {
    const TRANSITION_SECONDS: f32 = 0.13;
    const SURFACE_ALPHA: f32 = 0.56;
    const BLUR_STRENGTH: f32 = 42.0;

    let amount = (time.delta_secs() / TRANSITION_SECONDS).min(1.0);
    let mut visual_alpha = [0.0; 2];
    for (panel, mut fade, mut node, surface, background, strength) in &mut panels {
        fade.current = if fade.current < fade.target {
            (fade.current + amount).min(fade.target)
        } else {
            (fade.current - amount).max(fade.target)
        };
        let eased = fade.current * fade.current * (3.0 - 2.0 * fade.current);

        if let Some(mut background) = background {
            background.0 = Color::srgba(0.0, 0.0, 0.0, SURFACE_ALPHA * eased);
        }
        if let Some(mut strength) = strength {
            strength.0 = BLUR_STRENGTH * eased;
        }
        if surface.is_some() {
            visual_alpha[preview_index(panel.owner)] = eased;
        }
        if fade.target == 0.0 && fade.current == 0.0 {
            node.display = Display::None;
        }
    }

    for (visual, mut color) in &mut text {
        color.0 = color
            .0
            .with_alpha(visual.base_alpha * visual_alpha[preview_index(visual.owner)]);
    }
    for (visual, mut image) in &mut images {
        image.color = image
            .color
            .with_alpha(visual.base_alpha * visual_alpha[preview_index(visual.owner)]);
    }
}

fn preview_index(owner: ButtonAction) -> usize {
    usize::from(matches!(owner, ButtonAction::QuickLoad))
}

/// Anchors both the UI-layer blur proxy and the Dialog-camera preview to the
/// corresponding control button. All inputs are physical layout values, then
/// converted back into the shared 2560x1440 design canvas.
pub fn position_quick_previews(
    buttons: Query<(&ButtonAction, &ComputedNode, &UiGlobalTransform), With<Button>>,
    content_root: Query<(&ComputedNode, &UiGlobalTransform), With<ContentRoot>>,
    mut previews: Query<(&QuickPreviewPanel, &mut Node)>,
) {
    let Ok((root_node, root_transform)) = content_root.single() else {
        return;
    };
    let root_center = root_transform.translation;
    let to_design = root_node.inverse_scale_factor();
    let root_size = root_node.size() * to_design;

    for (action, button_node, button_transform) in &buttons {
        if !matches!(action, ButtonAction::QuickSave | ButtonAction::QuickLoad) {
            continue;
        }
        let button_center =
            root_size * 0.5 + (button_transform.translation - root_center) * to_design;
        let button_size = button_node.size() * to_design;
        let left = button_center.x + button_size.x * 0.5 - 1050.0;
        let top = button_center.y - button_size.y * 0.5 - 270.0 - 8.0;

        for (preview, mut node) in &mut previews {
            if preview.owner == *action {
                node.left = Val::Px(left);
                node.top = Val::Px(top);
                node.right = Val::Auto;
                node.bottom = Val::Auto;
            }
        }
    }
}

pub fn sync_quick_preview(
    preview: Res<QuickSavePreview>,
    asset_server: Res<AssetServer>,
    mut images: Query<(&mut ImageNode, &mut Node), With<QuickPreviewImage>>,
    mut contents: Query<&mut Node, (With<QuickPreviewContent>, Without<QuickPreviewImage>)>,
    mut speakers: Query<&mut Text, With<QuickPreviewSpeaker>>,
    mut dialogues: Query<&mut Text, (With<QuickPreviewDialogue>, Without<QuickPreviewSpeaker>)>,
    mut empty: EmptyPreviewQuery,
) {
    if !preview.is_changed() {
        return;
    }

    let Some(state) = &preview.state else {
        for (_, mut node) in &mut images {
            node.display = Display::None;
        }
        for mut node in &mut contents {
            node.display = Display::None;
        }
        for mut node in &mut empty {
            node.display = Display::Flex;
        }
        return;
    };

    for (mut image, mut node) in &mut images {
        if let Some(preview_image) = &preview.image {
            image.image = preview_image.clone();
            node.display = Display::Flex;
        } else if let Some(background) = &state.bg {
            image.image = asset_server.load(format!("background/{background}"));
            node.display = Display::Flex;
        } else {
            node.display = Display::None;
        }
    }
    for mut node in &mut contents {
        node.display = Display::Flex;
    }
    for mut node in &mut empty {
        node.display = Display::None;
    }

    let (speaker, dialogue) = state.dialogue.as_ref().map_or(("", ""), |dialogue| {
        (dialogue.speaker.as_str(), dialogue.text.as_str())
    });
    for mut text in &mut speakers {
        **text = speaker.to_owned();
    }
    for mut text in &mut dialogues {
        **text = dialogue.to_owned();
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
    pub fn new(base_alpha: f32) -> Self {
        Self { base_alpha }
    }
}

/// Marks a background node whose alpha is modulated by the Hide toggle.
#[derive(Component)]
pub(crate) struct HideContentBg {
    pub base_alpha: f32,
}

impl HideContentBg {
    pub fn new(base_alpha: f32) -> Self {
        Self { base_alpha }
    }
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
        Self {
            last_move: 0.0,
            last_cursor: None,
            alpha: 1.0,
            hide_alpha: 1.0,
            hide_btn_alpha: 1.0,
        }
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
    if let Ok(w) = win.single()
        && let Some(pos) = w.cursor_position()
    {
        let moved = match timing.last_cursor {
            Some(prev) => (prev - pos).length_squared() > 1.0,
            None => true,
        };
        timing.last_cursor = Some(pos);
        if moved {
            timing.last_move = now;
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

#[derive(Component)]
pub(crate) struct BlurStrength(pub f32);

#[derive(Component)]
pub(crate) struct UiBlurSource;

const LOCK_ICON: char = '\u{f47b}';
const UNLOCK_ICON: char = '\u{f600}';

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
    let hide_a = if toggles.hide {
        timing.hide_btn_alpha
    } else {
        a
    };
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
pub fn update_lock_icon(toggles: Res<ToggleStates>, mut q: Query<&mut Text, With<LockIcon>>) {
    if !toggles.is_changed() {
        return;
    }
    let ch = if toggles.lock { LOCK_ICON } else { UNLOCK_ICON };
    let s = ch.to_string();
    for mut text in q.iter_mut() {
        **text = s.clone();
    }
}
