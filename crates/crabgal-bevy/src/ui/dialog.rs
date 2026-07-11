// GlobalDialog — WebGAL-style confirmation overlay with title + two buttons.
use crate::render::blur::DialogCamera;
use crate::save::QUICK_SAVE_SLOT;
use crate::ui::control_bar::QuickSavePreview;
use bevy::camera::{RenderTarget, visibility::RenderLayers};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy::render::view::screenshot::{Screenshot, ScreenshotCaptured};

const FADE_DURATION: f32 = 0.2;
const OVERLAY_ALPHA: f32 = 0.0625;
const PANEL_ALPHA: f32 = 2.0 / 3.0;
const BUTTON_HOVER_ALPHA: f32 = 0.0625;

/// Which action to perform when the user confirms.
#[derive(Clone, Copy, Debug)]
pub(crate) enum DialogAction {
    QuickSave,
    QuickLoad,
    BackToTitle,
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
            left_text: crate::locale::dialog::CONFIRM.into(),
            right_text: crate::locale::dialog::CANCEL.into(),
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

#[derive(Component)]
struct QuickPreviewCapture {
    camera: Entity,
}

#[derive(SystemParam)]
pub(crate) struct QuickSaveContext<'w, 's> {
    state: ResMut<'w, crate::resources::GameState>,
    project_root: Res<'w, crate::resources::ProjectRoot>,
    preview: ResMut<'w, QuickSavePreview>,
    images: ResMut<'w, Assets<Image>>,
    windows: Query<'w, 's, &'static Window>,
}

/// Spawn the dialog overlay + centred box when DialogRequest is present.
pub fn spawn_dialog(
    mut commands: Commands,
    dialog_q: Query<Entity, With<DialogRoot>>,
    request: Option<Res<DialogRequest>>,
    asset_server: Res<AssetServer>,
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

    let font: Handle<Font> = asset_server.load("fonts/MavenPro-CJK.ttf");

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
            ZIndex(200),
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
                    (
                        Text::new(req.title.clone()),
                        TextFont {
                            font: font.clone().into(),
                            font_size: FontSize::from(64.0),
                            ..default()
                        },
                        TextColor(Color::NONE),
                        DialogText { alpha: 0.9 },
                    ),
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
        children![(
            Text::new(text),
            TextFont {
                font: font.into(),
                font_size: FontSize::from(42.0),
                ..default()
            },
            TextColor(Color::NONE),
            DialogText { alpha: 0.67 },
        )],
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
        visual.current += (visual.target - visual.current) * (time.delta_secs() * 12.0).min(1.0);
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
                if let Err(error) =
                    crate::save::save_game(&context.state, QUICK_SAVE_SLOT, &context.project_root)
                {
                    log::error!("quick save failed: {error:#}");
                } else {
                    context.preview.state = Some((**context.state).clone());
                    context.preview.image = None;
                    if let Ok(window) = context.windows.single() {
                        let size = Vec2::new(window.width(), window.height());
                        capture_quick_preview(&mut commands, &mut context.images, size);
                    }
                }
            }
            DialogAction::QuickLoad => {
                match crate::save::load_game(QUICK_SAVE_SLOT, &context.project_root) {
                    Ok(loaded) => **context.state = loaded,
                    Err(error) => log::error!("quick load failed: {error:#}"),
                }
            }
            DialogAction::BackToTitle => {
                crabgal_core::step::end_game(&mut context.state);
            }
        }
    }
    // right button = cancel, do nothing
}

fn capture_quick_preview(commands: &mut Commands, images: &mut Assets<Image>, size: Vec2) {
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
            Name::new("quick_save_preview_camera"),
            Camera2d,
            Camera { ..default() },
            RenderTarget::Image(target.clone().into()),
            RenderLayers::layer(0),
        ))
        .id();
    commands
        .spawn((Screenshot::image(target), QuickPreviewCapture { camera }))
        .observe(store_quick_preview);
}

fn store_quick_preview(
    capture: On<ScreenshotCaptured>,
    targets: Query<&QuickPreviewCapture>,
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut preview: ResMut<QuickSavePreview>,
    project_root: Res<crate::resources::ProjectRoot>,
) {
    if let Ok(target) = targets.get(capture.entity) {
        commands.entity(target.camera).despawn();
    }

    preview.image = Some(images.add(capture.image.clone()));
    let path = crate::save::preview_path(&project_root, QUICK_SAVE_SLOT);
    let image = capture.image.clone();
    bevy::tasks::AsyncComputeTaskPool::get()
        .spawn(async move {
            let result = image
                .try_into_dynamic()
                .map(|image| image.thumbnail(480, 270).to_rgb8())
                .map_err(anyhow::Error::from)
                .and_then(|image| image.save(&path).map_err(anyhow::Error::from));
            if let Err(error) = result {
                log::error!("failed to save quick-save preview: {error:#}");
            }
        })
        .detach();
}
