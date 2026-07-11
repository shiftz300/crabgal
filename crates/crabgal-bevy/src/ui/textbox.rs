use bevy::camera::visibility::RenderLayers;
use bevy::ecs::hierarchy::ChildSpawnerCommands;
use bevy::prelude::*;
use bevy::ui::FocusPolicy;
use crabgal_core::config::GameConfig;
use crabgal_core::{Anchor, DESIGN_HEIGHT, DESIGN_WIDTH};

use crate::render::blur::{DialogCamera, UiBlurCamera};
use crate::resources::{GameConfigResource, GameState};
use crate::ui::control_bar::{
    AutoHideBar, AutoHideText, AutoHideTiming, BOTTOM_ITEMS, BlurSource, BlurStrength,
    ButtonAction, ControlBarBot, ControlBarTop, ControlItem, HideButtonText, HideContentBg,
    HideContentText, HoverAlpha, LockIcon, QuickPreviewContent, QuickPreviewDialogue,
    QuickPreviewEmpty, QuickPreviewFade, QuickPreviewImage, QuickPreviewPanel, QuickPreviewSpeaker,
    QuickPreviewSurface, QuickPreviewVisual, TOP_ITEMS, UiBlurSource,
};

#[derive(Component)]
pub(crate) struct SpeakerText;
#[derive(Component)]
pub(crate) struct DialogueText;
#[derive(Component)]
pub(crate) struct TextBoxRoot;
#[derive(Component)]
pub(crate) struct NameBarRoot;
#[derive(Component)]
pub(crate) struct ContentRoot;
#[derive(Component)]
pub(crate) struct MiniAvatarNode;
#[derive(Component)]
pub(crate) struct QuickPreviewLayer;

type ContentRootFilter = (
    With<ContentRoot>,
    Without<NameBarRoot>,
    Without<TextBoxRoot>,
);
type NameBarFilter = (
    With<NameBarRoot>,
    Without<TextBoxRoot>,
    Without<ContentRoot>,
);
type TextBoxFilter = (
    With<TextBoxRoot>,
    Without<NameBarRoot>,
    Without<ContentRoot>,
);

#[derive(Clone)]
struct TextboxAssets {
    text_font: Handle<Font>,
    icon_font: Handle<Font>,
}

pub fn setup_textbox(
    mut commands: Commands,
    config: Res<GameConfigResource>,
    asset_server: Res<AssetServer>,
    ui_camera_query: Query<Entity, With<UiBlurCamera>>,
    dialog_camera_query: Query<Entity, With<DialogCamera>>,
) {
    let Ok(ui_camera) = ui_camera_query.single() else {
        log::error!("textbox requires exactly one UI camera");
        return;
    };
    let assets = TextboxAssets {
        text_font: asset_server.load("fonts/MavenPro-CJK.ttf"),
        icon_font: asset_server.load("fonts/bootstrap-icons.ttf"),
    };
    let Ok(dialog_camera) = dialog_camera_query.single() else {
        log::error!("quick preview requires exactly one dialog camera");
        return;
    };

    commands
        .spawn((
            Name::new("content_root"),
            ContentRoot,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(DESIGN_WIDTH),
                height: Val::Px(DESIGN_HEIGHT),
                ..default()
            },
            UiTargetCamera(ui_camera),
            RenderLayers::layer(1),
        ))
        .with_children(|root| {
            spawn_mini_avatar(root, &config);
            spawn_name_bar(root, &config, &assets);
            spawn_text_box(root, &config, &assets);
            spawn_quick_preview_backdrops(root);
        });
    spawn_quick_preview_layer(&mut commands, dialog_camera, &assets);
}

fn spawn_mini_avatar(root: &mut ChildSpawnerCommands, config: &GameConfig) {
    root.spawn((
        Name::new("mini_avatar"),
        MiniAvatarNode,
        ImageNode::default(),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Percent(config.layout.textbox_bottom),
            left: Val::Percent(config.layout.textbox_left),
            width: Val::Px(280.0),
            height: Val::Px(280.0),
            display: Display::None,
            ..default()
        },
        ZIndex(103),
        RenderLayers::layer(1),
    ));
}

pub fn update_mini_avatar(
    state: Res<GameState>,
    config: Res<GameConfigResource>,
    asset_server: Res<AssetServer>,
    mut avatars: Query<(&mut ImageNode, &mut Node), With<MiniAvatarNode>>,
) {
    if !state.is_changed() {
        return;
    }
    for (mut image, mut node) in &mut avatars {
        let Some(avatar) = &state.mini_avatar else {
            node.display = Display::None;
            continue;
        };
        image.image = asset_server.load(config.figure_path(avatar));
        image.color = Color::srgba(1.0, 1.0, 1.0, state.mini_avatar_progress);
        node.display = Display::Flex;
    }
}

fn spawn_name_bar(root: &mut ChildSpawnerCommands, config: &GameConfig, assets: &TextboxAssets) {
    let layout = &config.layout;
    root.spawn((
        Name::new("name_bar"),
        NameBarRoot,
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Percent(layout.namebar_bottom),
            left: Val::Percent(layout.textbox_left),
            padding: UiRect::axes(Val::Px(32.0), Val::Px(12.0)),
            min_width: Val::Px(100.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
        HideContentBg::new(0.7),
        BlurSource,
        ZIndex(102),
        RenderLayers::layer(1),
    ))
    .with_child((
        Name::new("speaker"),
        SpeakerText,
        Text::new(""),
        TextFont {
            font: assets.text_font.clone().into(),
            font_size: FontSize::from(config.fonts.speaker_size),
            ..default()
        },
        TextColor(Color::WHITE),
        HideContentText::new(1.0),
    ));
}

fn spawn_text_box(root: &mut ChildSpawnerCommands, config: &GameConfig, assets: &TextboxAssets) {
    let layout = &config.layout;
    let alpha = config.styles.textbox_alpha;
    root.spawn((
        Name::new("textbox"),
        TextBoxRoot,
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Percent(layout.textbox_bottom),
            left: Val::Percent(layout.textbox_left),
            width: Val::Percent(100.0 - layout.textbox_left),
            height: Val::Percent(layout.textbox_height),
            padding: UiRect {
                left: Val::Px(56.0),
                right: Val::Px(56.0),
                top: Val::Px(72.0),
                bottom: Val::Px(40.0),
            },
            ..default()
        },
        BackgroundColor(Color::srgba(0.05, 0.05, 0.08, alpha)),
        HideContentBg::new(alpha),
        BlurSource,
        ZIndex(101),
        RenderLayers::layer(1),
    ))
    .with_children(|text_box| {
        spawn_top_controls(text_box, config, assets);
        spawn_bottom_controls(text_box, config, assets);
        text_box.spawn((
            Name::new("dialogue"),
            DialogueText,
            Text::new(""),
            TextFont {
                font: assets.text_font.clone().into(),
                font_size: FontSize::from(config.fonts.dialogue_size),
                ..default()
            },
            TextColor(Color::WHITE),
            HideContentText::new(1.0),
            Node {
                width: Val::Percent(100.0),
                ..default()
            },
        ));
    });
}

fn spawn_top_controls(
    text_box: &mut ChildSpawnerCommands,
    config: &GameConfig,
    assets: &TextboxAssets,
) {
    text_box
        .spawn((
            Name::new("control_bar_top"),
            ControlBarTop,
            AutoHideBar,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(0.0),
                right: Val::Px(24.0),
                flex_direction: FlexDirection::Row,
                // Adjacent hit boxes prevent hover from dropping while moving
                // horizontally across the control bar.
                column_gap: Val::ZERO,
                ..default()
            },
            ZIndex(110),
        ))
        .with_children(|controls| {
            for item in TOP_ITEMS {
                spawn_top_control(controls, *item, config, assets);
            }
        });
}

fn spawn_top_control(
    controls: &mut ChildSpawnerCommands,
    item: ControlItem,
    config: &GameConfig,
    assets: &TextboxAssets,
) {
    controls
        .spawn((
            Name::new(format!("control::{}", item.label)),
            Button,
            HoverAlpha::default(),
            item.action,
            Node {
                width: Val::Px(72.0),
                height: Val::Px(68.0),
                padding: UiRect::all(Val::Px(8.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
        ))
        .with_children(|button| {
            let mut icon = button.spawn((
                Text::new(item.icon.to_string()),
                AutoHideText::new(0.85),
                TextFont {
                    font: assets.icon_font.clone().into(),
                    font_size: FontSize::from(config.fonts.icon_size),
                    ..default()
                },
                TextColor(Color::srgba(0.96, 0.96, 0.97, 0.85)),
            ));
            match item.action {
                ButtonAction::Hide => {
                    icon.insert(HideButtonText);
                }
                ButtonAction::Lock => {
                    icon.insert(LockIcon);
                }
                _ => {}
            }
        });
}

fn spawn_bottom_controls(
    text_box: &mut ChildSpawnerCommands,
    config: &GameConfig,
    assets: &TextboxAssets,
) {
    text_box
        .spawn((
            Name::new("control_bar_bottom"),
            ControlBarBot,
            AutoHideBar,
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(0.0),
                right: Val::Px(24.0),
                flex_direction: FlexDirection::Row,
                // Keep Q·SAVE/Q·LOAD and the remaining controls contiguous so
                // quick-preview transitions are not interrupted by dead space.
                column_gap: Val::ZERO,
                ..default()
            },
            ZIndex(110),
        ))
        .with_children(|controls| {
            for item in BOTTOM_ITEMS {
                controls
                    .spawn((
                        Name::new(format!("control::{}", item.label)),
                        Button,
                        HoverAlpha::default(),
                        item.action,
                        Node {
                            flex_direction: FlexDirection::Row,
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            column_gap: Val::Px(16.0),
                            padding: UiRect::axes(Val::Px(24.0), Val::Px(12.0)),
                            ..default()
                        },
                    ))
                    .with_children(|button| {
                        button.spawn((
                            Text::new(item.icon.to_string()),
                            AutoHideText::new(0.85),
                            TextFont {
                                font: assets.icon_font.clone().into(),
                                font_size: FontSize::from(config.fonts.icon_size),
                                ..default()
                            },
                            TextColor(Color::srgba(0.88, 0.88, 0.92, 0.85)),
                        ));
                        button.spawn((
                            Text::new(item.label),
                            AutoHideText::new(0.75),
                            TextFont {
                                font: assets.text_font.clone().into(),
                                font_size: FontSize::from(config.fonts.label_size),
                                ..default()
                            },
                            TextColor(Color::srgba(0.78, 0.78, 0.84, 0.75)),
                        ));
                    });
            }
        });
}

fn spawn_quick_preview_backdrops(root: &mut ChildSpawnerCommands) {
    for owner in [ButtonAction::QuickSave, ButtonAction::QuickLoad] {
        root.spawn((
            Name::new(format!("quick_preview_blur::{owner:?}")),
            QuickPreviewPanel { owner },
            QuickPreviewFade::default(),
            UiBlurSource,
            BlurStrength(0.0),
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(40.0),
                bottom: Val::Px(290.0),
                width: Val::Px(1050.0),
                height: Val::Px(270.0),
                display: Display::None,
                ..default()
            },
            FocusPolicy::Pass,
            GlobalZIndex(139),
        ));
    }
}

fn spawn_quick_preview_layer(
    commands: &mut Commands,
    dialog_camera: Entity,
    assets: &TextboxAssets,
) {
    commands
        .spawn((
            Name::new("quick_preview_layer"),
            QuickPreviewLayer,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(DESIGN_WIDTH),
                height: Val::Px(DESIGN_HEIGHT),
                ..default()
            },
            FocusPolicy::Pass,
            UiTargetCamera(dialog_camera),
            RenderLayers::layer(2),
        ))
        .with_children(|layer| {
            for owner in [ButtonAction::QuickSave, ButtonAction::QuickLoad] {
                spawn_quick_preview(layer, owner, assets);
            }
        });
}

fn spawn_quick_preview(
    layer: &mut ChildSpawnerCommands,
    owner: ButtonAction,
    assets: &TextboxAssets,
) {
    layer
        .spawn((
            Name::new(format!("quick_preview::{owner:?}")),
            QuickPreviewPanel { owner },
            QuickPreviewFade::default(),
            QuickPreviewSurface,
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(40.0),
                bottom: Val::Px(290.0),
                width: Val::Px(1050.0),
                height: Val::Px(270.0),
                display: Display::None,
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                padding: UiRect::all(Val::Px(16.0)),
                ..default()
            },
            BackgroundColor(Color::NONE),
            FocusPolicy::Pass,
            GlobalZIndex(140),
        ))
        .with_children(|preview| {
            preview.spawn((
                QuickPreviewImage,
                QuickPreviewVisual {
                    owner,
                    base_alpha: 1.0,
                },
                ImageNode::default(),
                Node {
                    width: Val::Px(424.0),
                    height: Val::Percent(100.0),
                    flex_shrink: 0.0,
                    display: Display::None,
                    ..default()
                },
            ));
            preview
                .spawn((
                    QuickPreviewContent,
                    Node {
                        height: Val::Percent(100.0),
                        flex_grow: 1.0,
                        display: Display::None,
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(12.0),
                        padding: UiRect::left(Val::Px(20.0)),
                        overflow: Overflow::clip(),
                        ..default()
                    },
                ))
                .with_children(|content| {
                    content.spawn((
                        QuickPreviewSpeaker,
                        QuickPreviewVisual {
                            owner,
                            base_alpha: 0.85,
                        },
                        Text::new(""),
                        TextFont {
                            font: assets.text_font.clone().into(),
                            font_size: FontSize::from(32.0),
                            ..default()
                        },
                        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.85)),
                    ));
                    content.spawn((
                        QuickPreviewDialogue,
                        QuickPreviewVisual {
                            owner,
                            base_alpha: 0.67,
                        },
                        Text::new(""),
                        TextFont {
                            font: assets.text_font.clone().into(),
                            font_size: FontSize::from(26.0),
                            ..default()
                        },
                        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.67)),
                    ));
                });
            preview.spawn((
                QuickPreviewEmpty,
                QuickPreviewVisual {
                    owner,
                    base_alpha: 0.67,
                },
                Text::new("暂无存档"),
                TextFont {
                    font: assets.text_font.clone().into(),
                    font_size: FontSize::from(30.0),
                    ..default()
                },
                TextColor(Color::srgba(1.0, 1.0, 1.0, 0.67)),
            ));
        });
}

pub fn update_textbox(
    state: Res<GameState>,
    config: Res<GameConfigResource>,
    mut content_root: Query<&mut Node, ContentRootFilter>,
    mut speaker_text: Query<&mut Text, (With<SpeakerText>, Without<DialogueText>)>,
    mut dialogue_text: Query<&mut Text, (With<DialogueText>, Without<SpeakerText>)>,
    mut name_bar: Query<&mut Node, NameBarFilter>,
    mut text_box: Query<&mut Node, TextBoxFilter>,
) {
    if let Ok(mut root) = content_root.single_mut() {
        root.display = if state.ended {
            Display::None
        } else {
            Display::Flex
        };
    }

    let (speaker, dialogue) = state.dialogue.as_ref().map_or_else(
        || (String::new(), String::new()),
        |dialogue| {
            (
                dialogue.speaker.clone(),
                dialogue.text.chars().take(dialogue.visible_chars).collect(),
            )
        },
    );

    let has_left_sprite = state
        .sprites
        .values()
        .any(|sprite| matches!(sprite.position.x, Anchor::Left(_)));
    let layout = &config.layout;
    let left = if has_left_sprite {
        layout.textbox_dodge_left
    } else {
        layout.textbox_left
    };
    let width = 100.0 - left;
    for mut node in &mut name_bar {
        node.left = Val::Percent(left);
        node.width = Val::Auto;
        node.display = if speaker.is_empty() {
            Display::None
        } else {
            Display::Flex
        };
    }
    for mut node in &mut text_box {
        node.left = Val::Percent(left);
        node.width = Val::Percent(width);
    }

    if let Ok(mut text) = speaker_text.single_mut() {
        text.0 = speaker;
    }
    if let Ok(mut text) = dialogue_text.single_mut() {
        text.0 = dialogue;
    }
}

pub fn apply_hide_toggle(
    timing: Res<AutoHideTiming>,
    mut text_query: Query<(&mut TextColor, &HideContentText)>,
    mut background_query: Query<(&mut BackgroundColor, &HideContentBg)>,
) {
    for (mut color, hidden) in &mut text_query {
        color.0 = color.0.with_alpha(hidden.base_alpha * timing.hide_alpha);
    }
    for (mut color, hidden) in &mut background_query {
        color.0 = color.0.with_alpha(hidden.base_alpha * timing.hide_alpha);
    }
}
