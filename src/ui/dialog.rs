// GlobalDialog — WebGAL-style confirmation overlay with title + two buttons.
use crate::render::blur::DialogCamera;
use crate::render::blur::UiBlurCamera;
use crate::storage::save::QUICK_SAVE_SLOT;
use crate::ui::backlog::BacklogRoot;
use crate::ui::control_bar::QuickSavePreview;
use crate::ui::foundation::{UiFonts, exp_lerp};
use crate::ui::save_load::SaveLoadRoot;
use crate::ui::settings_panel::SettingsRoot;
use bevy::camera::{RenderTarget, visibility::RenderLayers};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};
use bevy::ui::FocusPolicy;

const FADE_DURATION: f32 = 0.2;
const OVERLAY_ALPHA: f32 = 0.16;
const PANEL_ALPHA: f32 = 0.78;
const BUTTON_HOVER_ALPHA: f32 = 0.0625;

/// Which action to perform when the user confirms.
#[derive(Clone, Copy, Debug)]
pub(crate) enum DialogAction {
    QuickSave,
    QuickLoad,
    SaveSlot(u32),
    LoadSlot(u32),
    DeleteSlot(u32),
    BackToTitle,
    Noop,
    ExitGame,
}

/// Active dialog request. When set, the overlay + dialog UI is shown.
#[derive(Resource, Clone)]
pub(crate) struct DialogRequest {
    pub title: String,
    pub left_text: String,
    pub right_text: String,
    pub action: DialogAction,
}

impl DialogRequest {
    pub fn confirmation(title: impl Into<String>, action: DialogAction) -> Self {
        Self {
            title: title.into(),
            left_text: crate::ui::locale::dialog::CONFIRM.into(),
            right_text: crate::ui::locale::dialog::CANCEL.into(),
            action,
        }
    }
}

#[derive(Component)]
pub(crate) struct DialogRoot;
#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DialogButton {
    Confirm,
    Cancel,
}

#[derive(Component)]
pub(crate) struct DialogFade(f32);

impl DialogFade {
    pub(crate) fn is_animating(&self) -> bool {
        self.0 < 0.999
    }
}

#[derive(Component)]
pub(crate) struct DialogBackground {
    alpha: f32,
}

#[derive(Component)]
pub(crate) struct DialogBorder {
    alpha: f32,
}

#[derive(Component)]
pub(crate) struct DialogText {
    alpha: f32,
}

#[derive(Component, Default)]
pub(crate) struct DialogButtonVisual {
    current: f32,
    target: f32,
}

impl DialogButtonVisual {
    pub(crate) fn is_animating(&self) -> bool {
        (self.current - self.target).abs() > 0.001
    }
}

type ModalBackdropQuery<'w, 's> = Query<
    'w,
    's,
    (&'static mut UiTargetCamera, &'static mut RenderLayers),
    Or<(With<BacklogRoot>, With<SaveLoadRoot>, With<SettingsRoot>)>,
>;

#[derive(Component)]
struct SavePreviewCapture {
    camera: Entity,
    slot: u32,
}

#[derive(SystemParam)]
pub(crate) struct QuickSaveContext<'w, 's> {
    state: ResMut<'w, crate::runtime::resources::GameState>,
    project_root: Res<'w, crate::runtime::resources::ProjectRoot>,
    preview: ResMut<'w, QuickSavePreview>,
    images: ResMut<'w, Assets<Image>>,
    windows: Query<'w, 's, &'static Window>,
    save_load: ResMut<'w, crate::ui::save_load::SaveLoadUi>,
    settings_ui: ResMut<'w, crate::ui::settings_panel::SettingsUi>,
    backlog_ui: ResMut<'w, crate::ui::backlog::BacklogUiState>,
}

#[derive(SystemParam)]
struct SavePreviewContext<'w, 's> {
    targets: Query<'w, 's, &'static SavePreviewCapture>,
    commands: Commands<'w, 's>,
    images: ResMut<'w, Assets<Image>>,
    preview: ResMut<'w, QuickSavePreview>,
    save_previews: ResMut<'w, crate::ui::save_load::SavePreviewCache>,
    save_load: ResMut<'w, crate::ui::save_load::SaveLoadUi>,
    project_root: Res<'w, crate::runtime::resources::ProjectRoot>,
}

/// Spawn the dialog overlay + centred box when DialogRequest is present.
pub fn spawn_dialog(
    mut commands: Commands,
    dialog_q: Query<Entity, With<DialogRoot>>,
    request: Option<Res<DialogRequest>>,
    fonts: Res<UiFonts>,
    dialog_camera_q: Query<Entity, With<DialogCamera>>,
) {
    // Remove existing dialog when request is gone
    if request
        .as_ref()
        .is_some_and(|request| !request.is_changed())
        && !dialog_q.is_empty()
    {
        return;
    }

    // Clear old dialog
    for e in dialog_q.iter() {
        commands.entity(e).despawn();
    }

    let Some(req) = request else { return };
    let Ok(dialog_camera) = dialog_camera_q.single() else {
        return;
    };

    let font = fonts.text.clone();

    commands
        .spawn((
            Name::new("dialog_overlay"),
            DialogRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(0.0),
                top: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::NONE),
            DialogFade(0.0),
            DialogBackground {
                alpha: OVERLAY_ALPHA,
            },
            FocusPolicy::Block,
            GlobalZIndex(200),
            UiTargetCamera(dialog_camera),
            RenderLayers::layer(2),
        ))
        .with_children(|p| {
            p.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(20.0),
                    border: UiRect::top(Val::Px(15.0)),
                    ..default()
                },
                BorderColor::all(Color::NONE),
                DialogBorder { alpha: 0.19 },
            ))
            .with_child((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::SpaceBetween,
                    align_items: AlignItems::Center,
                    padding: UiRect::axes(Val::Px(80.0), Val::Px(24.0)),
                    ..default()
                },
                BackgroundColor(Color::NONE),
                DialogBackground {
                    alpha: PANEL_ALPHA,
                },
                children![
                    // Title
                    dialog_text(req.title.clone(), font.clone(), 64.0, 0.9),
                    // Button row — wide spacing
                    (
                        Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(80.0),
                            ..default()
                        },
                        children![
                            spawn_dialog_button(
                                DialogButton::Confirm,
                                req.left_text.clone(),
                                font.clone(),
                            ),
                            spawn_dialog_button(
                                DialogButton::Cancel,
                                req.right_text.clone(),
                                font,
                            ),
                        ],
                    ),
                ],
            ));
        });
}

/// Full-screen menus normally render after their own backdrop blur. When a
/// confirmation dialog opens, temporarily render those menus on the UI camera
/// so the dialog's full-screen post-process also blurs the menu beneath it.
pub fn sync_modal_backdrop_layer(
    request: Option<Res<DialogRequest>>,
    ui_camera: Query<Entity, With<UiBlurCamera>>,
    dialog_camera: Query<Entity, (With<DialogCamera>, Without<UiBlurCamera>)>,
    mut roots: ModalBackdropQuery,
) {
    let target = if request.is_some() {
        ui_camera.single().ok().map(|entity| (entity, 1))
    } else {
        dialog_camera.single().ok().map(|entity| (entity, 2))
    };
    let Some((target, layer)) = target else {
        return;
    };
    for (mut current, mut layers) in &mut roots {
        if current.0 != target {
            *current = UiTargetCamera(target);
            *layers = RenderLayers::layer(layer);
        }
    }
}

fn spawn_dialog_button(action: DialogButton, text: String, font: Handle<Font>) -> impl Bundle {
    (
        Button,
        action,
        DialogButtonVisual::default(),
        Node {
            padding: UiRect::axes(Val::Px(32.0), Val::Px(8.0)),
            ..default()
        },
        BackgroundColor(Color::NONE),
        children![dialog_text(text, font, 42.0, 0.67)],
    )
}

fn dialog_text(
    content: impl Into<String>,
    font: Handle<Font>,
    size: f32,
    alpha: f32,
) -> impl Bundle {
    (
        Text::new(content.into()),
        TextFont {
            font: font.into(),
            font_size: FontSize::from(size),
            ..default()
        },
        TextColor(Color::NONE),
        DialogText { alpha },
    )
}

pub fn animate_dialog(
    time: Res<Time>,
    mut fade_query: Query<&mut DialogFade>,
    mut backgrounds: Query<(&DialogBackground, &mut BackgroundColor)>,
    mut borders: Query<(&DialogBorder, &mut BorderColor)>,
    mut texts: Query<(&DialogText, &mut TextColor)>,
) {
    let Ok(mut fade) = fade_query.single_mut() else {
        return;
    };
    fade.0 = (fade.0 + time.delta_secs() / FADE_DURATION).min(1.0);

    for (visual, mut color) in &mut backgrounds {
        color.0 = Color::srgba(0.0, 0.0, 0.0, visual.alpha * fade.0);
    }
    for (visual, mut color) in &mut borders {
        *color = BorderColor::all(Color::srgba(0.0, 0.0, 0.0, visual.alpha * fade.0));
    }
    for (visual, mut color) in &mut texts {
        color.0 = Color::srgba(1.0, 1.0, 1.0, visual.alpha * fade.0);
    }
}

pub fn update_dialog_buttons(
    time: Res<Time>,
    mut buttons: Query<(&Interaction, &mut DialogButtonVisual, &mut BackgroundColor)>,
) {
    for (interaction, mut visual, mut color) in &mut buttons {
        visual.target = match interaction {
            Interaction::None => 0.0,
            Interaction::Hovered => BUTTON_HOVER_ALPHA,
            Interaction::Pressed => BUTTON_HOVER_ALPHA * 0.5,
        };
        visual.current += (visual.target - visual.current) * exp_lerp(time.delta_secs(), 12.0);
        color.0 = Color::srgba(1.0, 1.0, 1.0, visual.current);
    }
}

/// Handle dialog button clicks: execute the action and remove the request.
pub fn handle_dialog_click(
    mut commands: Commands,
    buttons: Query<(&Interaction, &DialogButton), Changed<Interaction>>,
    request: Option<Res<DialogRequest>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut context: QuickSaveContext,
) {
    let left_clicked = buttons.iter().any(|(interaction, button)| {
        matches!(interaction, Interaction::Pressed) && *button == DialogButton::Confirm
    }) || keys.just_pressed(KeyCode::Enter);
    let right_clicked = buttons.iter().any(|(interaction, button)| {
        matches!(interaction, Interaction::Pressed) && *button == DialogButton::Cancel
    }) || keys.just_pressed(KeyCode::Escape);

    if !left_clicked && !right_clicked {
        return;
    }
    let Some(req) = request else { return };
    commands.remove_resource::<DialogRequest>();

    if left_clicked {
        match &req.action {
            DialogAction::QuickSave => {
                if let Err(error) = crate::storage::save::save_game(
                    &context.state,
                    QUICK_SAVE_SLOT,
                    &context.project_root,
                ) {
                    log::error!("quick save failed: {error:#}");
                } else {
                    context.preview.state = Some((**context.state).clone());
                    context.preview.image = None;
                    if let Ok(window) = context.windows.single() {
                        let size = Vec2::new(window.width(), window.height());
                        capture_save_preview(
                            &mut commands,
                            &mut context.images,
                            size,
                            QUICK_SAVE_SLOT,
                        );
                    }
                }
            }
            DialogAction::QuickLoad => {
                match crate::storage::save::load_game(QUICK_SAVE_SLOT, &context.project_root) {
                    Ok(loaded) => **context.state = loaded,
                    Err(error) => log::error!("quick load failed: {error:#}"),
                }
            }
            DialogAction::BackToTitle => {
                commands.insert_resource(crate::ui::title::ReturnToTitleTransition::default());
                context.save_load.mode = None;
                context.settings_ui.open = false;
                context.backlog_ui.open = false;
            }
            DialogAction::SaveSlot(slot) => {
                if let Err(error) =
                    crate::storage::save::save_game(&context.state, *slot, &context.project_root)
                {
                    log::error!("save slot {slot} failed: {error:#}");
                } else {
                    context.save_load.set_changed();
                    if let Ok(window) = context.windows.single() {
                        let size = Vec2::new(window.width(), window.height());
                        capture_save_preview(&mut commands, &mut context.images, size, *slot);
                    }
                }
            }
            DialogAction::LoadSlot(slot) => {
                match crate::storage::save::load_game(*slot, &context.project_root) {
                    Ok(loaded) => {
                        **context.state = loaded;
                        context.save_load.mode = None;
                    }
                    Err(error) => log::error!("load slot {slot} failed: {error:#}"),
                }
            }
            DialogAction::DeleteSlot(slot) => {
                match crate::storage::save::delete_game(*slot, &context.project_root) {
                    Ok(()) => context.save_load.set_changed(),
                    Err(error) => log::error!("delete slot {slot} failed: {error:#}"),
                }
            }
            DialogAction::Noop => {}
            DialogAction::ExitGame => {
                commands.write_message(bevy::app::AppExit::Success);
            }
        }
    }
    // right button = cancel, do nothing
}

pub(crate) fn capture_save_preview(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    size: Vec2,
    slot: u32,
) {
    let width = size.x.round().max(1.0) as u32;
    let height = size.y.round().max(1.0) as u32;
    let target = images.add(Image::new_target_texture(
        width,
        height,
        TextureFormat::Rgba8UnormSrgb,
        None,
    ));
    let camera = commands
        .spawn((
            Name::new("save_preview_camera"),
            Camera2d,
            Camera { ..default() },
            RenderTarget::Image(target.clone().into()),
            RenderLayers::layer(0),
        ))
        .id();
    commands
        .spawn((
            Screenshot::image(target),
            SavePreviewCapture { camera, slot },
        ))
        .observe(store_save_preview);
}

fn store_save_preview(capture: On<ScreenshotCaptured>, mut context: SavePreviewContext) {
    let Ok(target) = context.targets.get(capture.entity) else {
        return;
    };
    context.commands.entity(target.camera).despawn();
    let mut display_image = capture.image.clone();
    display_image.asset_usage = bevy::asset::RenderAssetUsages::RENDER_WORLD;
    let captured = context.images.add(display_image);
    if target.slot == QUICK_SAVE_SLOT {
        context.preview.image = Some(captured);
    } else {
        context.save_previews.insert_live(target.slot, captured);
        context.save_load.set_changed();
    }
    let path = crate::storage::save::preview_path(&context.project_root, target.slot);
    let image = capture.image.clone();
    bevy::tasks::AsyncComputeTaskPool::get()
        .spawn(async move {
            let result = image
                .try_into_dynamic()
                .map(|image| image.thumbnail(480, 270).to_rgb8())
                .map_err(anyhow::Error::from)
                .and_then(|image| {
                    crate::scene::images::encode_preview(
                        image.as_raw(),
                        image.width(),
                        image.height(),
                    )
                    .map_err(anyhow::Error::from)
                })
                .and_then(|bytes| std::fs::write(&path, bytes).map_err(anyhow::Error::from));
            if let Err(error) = result {
                log::error!("failed to save slot preview: {error:#}");
            }
        })
        .detach();
}
