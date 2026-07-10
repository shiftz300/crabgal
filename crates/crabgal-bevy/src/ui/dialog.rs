// GlobalDialog — WebGAL-style confirmation overlay with title + two buttons.
use crate::render::blur::DialogCamera;
use crate::save::QUICK_SAVE_SLOT;
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;

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
#[derive(Component)]
pub(crate) struct DialogLeftBtn;
#[derive(Component)]
pub(crate) struct DialogRightBtn;

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
            BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.06)),
            ZIndex(200),
            UiTargetCamera(dialog_camera),
            RenderLayers::layer(2),
        ))
        .with_children(|p| {
            // Inner box — full width, dark, top border accent
            p.spawn((
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(22.0),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(32.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.67)),
            ))
            .with_children(|inner| {
                // Title
                inner.spawn((
                    Text::new(req.title.clone()),
                    TextFont {
                        font: font.clone().into(),
                        font_size: FontSize::from(42.0),
                        ..default()
                    },
                    TextColor(Color::srgba(0.95, 0.95, 1.0, 0.9)),
                ));
                // Button row — wide spacing
                inner
                    .spawn((Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(80.0),
                        margin: UiRect::top(Val::Px(16.0)),
                        ..default()
                    },))
                    .with_children(|btns| {
                        btns.spawn((
                            Button,
                            DialogLeftBtn,
                            Node {
                                padding: UiRect {
                                    left: Val::Px(40.0),
                                    right: Val::Px(40.0),
                                    top: Val::Px(10.0),
                                    bottom: Val::Px(10.0),
                                },
                                ..default()
                            },
                        ))
                        .with_child((
                            Text::new(req.left_text.clone()),
                            TextFont {
                                font: font.clone().into(),
                                font_size: FontSize::from(28.0),
                                ..default()
                            },
                            TextColor(Color::srgba(0.85, 0.85, 0.90, 1.0)),
                        ));
                        btns.spawn((
                            Button,
                            DialogRightBtn,
                            Node {
                                padding: UiRect {
                                    left: Val::Px(40.0),
                                    right: Val::Px(40.0),
                                    top: Val::Px(10.0),
                                    bottom: Val::Px(10.0),
                                },
                                ..default()
                            },
                        ))
                        .with_child((
                            Text::new(req.right_text.clone()),
                            TextFont {
                                font: font.into(),
                                font_size: FontSize::from(28.0),
                                ..default()
                            },
                            TextColor(Color::srgba(0.85, 0.85, 0.90, 1.0)),
                        ));
                    });
            });
        });
}

/// Handle dialog button clicks: execute the action and remove the request.
pub fn handle_dialog_click(
    mut commands: Commands,
    left_q: Query<&Interaction, (Changed<Interaction>, With<DialogLeftBtn>)>,
    right_q: Query<&Interaction, (Changed<Interaction>, With<DialogRightBtn>)>,
    request: Option<Res<DialogRequest>>,
    mut state: ResMut<crate::resources::GameState>,
    project_root: Res<crate::resources::ProjectRoot>,
) {
    let left_clicked = left_q.iter().any(|i| matches!(i, Interaction::Pressed));
    let right_clicked = right_q.iter().any(|i| matches!(i, Interaction::Pressed));

    if !left_clicked && !right_clicked {
        return;
    }
    let Some(req) = request else { return };
    commands.remove_resource::<DialogRequest>();

    if left_clicked {
        match &req.action {
            DialogAction::QuickSave => {
                if let Err(error) = crate::save::save_game(&state, QUICK_SAVE_SLOT, &project_root) {
                    log::error!("quick save failed: {error:#}");
                }
            }
            DialogAction::QuickLoad => {
                match crate::save::load_game(QUICK_SAVE_SLOT, &project_root) {
                    Ok(loaded) => **state = loaded,
                    Err(error) => log::error!("quick load failed: {error:#}"),
                }
            }
            DialogAction::BackToTitle => {
                log::info!("back to title (not yet implemented)");
            }
        }
    }
    // right button = cancel, do nothing
}
