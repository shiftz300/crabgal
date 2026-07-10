use bevy::camera::visibility::RenderLayers;
use bevy::ecs::hierarchy::ChildSpawnerCommands;
use bevy::prelude::*;
use crabgal_core::config::GameConfig;
use crabgal_core::{Anchor, DESIGN_HEIGHT, DESIGN_WIDTH};

use crate::render::blur::UiBlurCamera;
use crate::resources::{GameConfigResource, GameState};
use crate::ui::control_bar::{
    AutoHideBar, AutoHideText, AutoHideTiming, BOTTOM_ITEMS, BlurSource, ButtonAction,
    ControlBarBot, ControlBarTop, ControlItem, HideButtonText, HideContentBg, HideContentText,
    HoverAlpha, LockIcon, TOP_ITEMS,
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
) {
    let Ok(ui_camera) = ui_camera_query.single() else {
        log::error!("textbox requires exactly one UI camera");
        return;
    };
    let assets = TextboxAssets {
        text_font: asset_server.load("fonts/MavenPro-CJK.ttf"),
        icon_font: asset_server.load("fonts/bootstrap-icons.ttf"),
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
            spawn_name_bar(root, &config, &assets);
            spawn_text_box(root, &config, &assets);
        });
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
            padding: UiRect::axes(Val::Px(20.0), Val::Px(6.0)),
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
                column_gap: Val::Px(4.0),
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
                column_gap: Val::Px(2.0),
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

pub fn update_textbox(
    state: Res<GameState>,
    config: Res<GameConfigResource>,
    mut speaker_text: Query<&mut Text, (With<SpeakerText>, Without<DialogueText>)>,
    mut dialogue_text: Query<&mut Text, (With<DialogueText>, Without<SpeakerText>)>,
    mut name_bar: Query<&mut Node, (With<NameBarRoot>, Without<TextBoxRoot>)>,
    mut text_box: Query<&mut Node, (With<TextBoxRoot>, Without<NameBarRoot>)>,
) {
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
        node.width = Val::Percent(width);
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
