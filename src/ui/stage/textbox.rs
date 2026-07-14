use bevy::camera::visibility::RenderLayers;
use bevy::ecs::hierarchy::ChildSpawnerCommands;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::ui::FocusPolicy;
use crabgal_core::config::GameConfig;
use crabgal_core::{Anchor, DESIGN_HEIGHT, DESIGN_WIDTH};

use crate::render::blur::{DialogCamera, UiBlurCamera};
use crate::runtime::resources::{GameConfigResource, GameState};
use crate::storage::settings::RuntimeSettings;
use crate::ui::control_bar::{
    AutoHideBar, AutoHideText, AutoHideTiming, BOTTOM_ITEMS, BlurSource, BlurStrength,
    ButtonAction, ControlBarBot, ControlBarTop, ControlItem, HideButtonText, HideContentBg,
    HideContentText, HoverAlpha, LockIcon, QuickPreviewContent, QuickPreviewDialogue,
    QuickPreviewEmpty, QuickPreviewFade, QuickPreviewImage, QuickPreviewPanel, QuickPreviewSpeaker,
    QuickPreviewSurface, QuickPreviewVisual, TOP_ITEMS, UiBlurSource,
};
use crate::ui::foundation::{UiFonts, exp_lerp};

#[derive(Component)]
pub(crate) struct SpeakerText;
#[derive(Component)]
pub(crate) struct DialogueText;
#[derive(Component)]
pub(crate) struct DialogueGlyph {
    reveal_at: usize,
}
#[derive(Component)]
pub(crate) struct DialogueBaseGlyph;
#[derive(Component)]
pub(crate) struct DialogueRubyGlyph;
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

const RUBY_FONT_SCALE: f32 = 0.44;
const RUBY_COLLISION_PADDING: f32 = 4.5;

#[derive(Resource)]
pub(crate) struct TextboxOverlayFade {
    pub(crate) alpha: f32,
}

impl Default for TextboxOverlayFade {
    fn default() -> Self {
        Self { alpha: 1.0 }
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum InitialTextboxFadePhase {
    #[default]
    Waiting,
    Fading,
    Complete,
}

#[derive(Resource)]
pub(crate) struct InitialTextboxFade {
    pub(crate) alpha: f32,
    phase: InitialTextboxFadePhase,
    elapsed: f32,
}

impl Default for InitialTextboxFade {
    fn default() -> Self {
        Self {
            alpha: 0.0,
            phase: InitialTextboxFadePhase::Waiting,
            elapsed: 0.0,
        }
    }
}

impl InitialTextboxFade {
    const SECONDS: f32 = 0.12;

    pub(crate) fn is_animating(&self) -> bool {
        self.phase == InitialTextboxFadePhase::Fading
    }
}

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
type TextboxOverlayFilter = Or<(
    With<crate::ui::dialog::DialogRoot>,
    With<crate::ui::backlog::BacklogRoot>,
    With<crate::ui::save_load::SaveLoadRoot>,
    With<crate::ui::settings_panel::SettingsRoot>,
    With<crate::ui::overlays::user_input::UserInputRoot>,
)>;

#[derive(SystemParam)]
pub(crate) struct TextboxUpdateResources<'w> {
    state: Res<'w, GameState>,
    config: Res<'w, GameConfigResource>,
    settings: Res<'w, RuntimeSettings>,
    auto_hide: Res<'w, AutoHideTiming>,
    overlay: Res<'w, TextboxOverlayFade>,
    fonts: Res<'w, UiFonts>,
}

#[derive(Default)]
pub(crate) struct TextboxRenderCache {
    content_hidden: Option<bool>,
    speaker: String,
    dialogue: String,
    visible_chars: usize,
    left: Option<f32>,
    textbox_alpha: Option<f32>,
    dialogue_size: Option<f32>,
}

pub fn setup_textbox(
    mut commands: Commands,
    config: Res<GameConfigResource>,
    fonts: Res<UiFonts>,
    ui_camera_query: Query<Entity, With<UiBlurCamera>>,
    dialog_camera_query: Query<Entity, With<DialogCamera>>,
) {
    let Ok(ui_camera) = ui_camera_query.single() else {
        log::error!("textbox requires exactly one UI camera");
        return;
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
            spawn_name_bar(root, &config, &fonts);
            spawn_text_box(root, &config, &fonts);
            spawn_quick_preview_backdrops(root);
        });
    spawn_quick_preview_layer(&mut commands, dialog_camera, &fonts);
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
            width: Val::Px(210.0),
            height: Val::Px(210.0),
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
    mut previous: Local<Option<(Option<String>, f32)>>,
) {
    if previous.as_ref().is_some_and(|(avatar, progress)| {
        avatar == &state.mini_avatar && *progress == state.mini_avatar_progress
    }) {
        return;
    }
    *previous = Some((state.mini_avatar.clone(), state.mini_avatar_progress));
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

fn spawn_name_bar(root: &mut ChildSpawnerCommands, config: &GameConfig, assets: &UiFonts) {
    let layout = &config.layout;
    root.spawn((
        Name::new("name_bar"),
        NameBarRoot,
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Percent(layout.namebar_bottom),
            left: Val::Percent(layout.textbox_left),
            padding: UiRect::axes(Val::Px(24.0), Val::Px(9.0)),
            min_width: Val::Px(75.0),
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
            font: assets.text.clone().into(),
            font_size: FontSize::from(config.fonts.speaker_size),
            ..default()
        },
        TextColor(Color::WHITE),
        HideContentText::new(1.0),
    ));
}

fn spawn_text_box(root: &mut ChildSpawnerCommands, config: &GameConfig, assets: &UiFonts) {
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
                left: Val::Px(42.0),
                right: Val::Px(42.0),
                top: Val::Px(54.0),
                bottom: Val::Px(30.0),
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
            Node {
                width: Val::Percent(100.0),
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
                align_items: AlignItems::FlexEnd,
                align_content: AlignContent::FlexStart,
                // Ruby is overlaid inside each base glyph's line box. This gap
                // only keeps annotations on adjacent wrapped lines apart; it
                // does not create a separate ruby line.
                row_gap: Val::Px(10.5),
                ..default()
            },
        ));
    });
}

fn spawn_top_controls(text_box: &mut ChildSpawnerCommands, config: &GameConfig, assets: &UiFonts) {
    text_box
        .spawn((
            Name::new("control_bar_top"),
            ControlBarTop,
            AutoHideBar,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(0.0),
                right: Val::Px(18.0),
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
    assets: &UiFonts,
) {
    controls
        .spawn((
            Name::new(format!("control::{}", item.label)),
            Button,
            HoverAlpha::default(),
            item.action,
            Node {
                width: Val::Px(54.0),
                height: Val::Px(51.0),
                padding: UiRect::all(Val::Px(6.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
        ))
        .with_children(|button| {
            let mut icon = button.spawn((
                Text::new(item.icon.to_string()),
                AutoHideText::new(0.94),
                TextFont {
                    font: assets.icons.clone().into(),
                    font_size: FontSize::from(config.fonts.icon_size),
                    ..default()
                },
                TextColor(Color::srgba(0.98, 0.98, 0.99, 0.94)),
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
    assets: &UiFonts,
) {
    text_box
        .spawn((
            Name::new("control_bar_bottom"),
            ControlBarBot,
            AutoHideBar,
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(0.0),
                right: Val::Px(18.0),
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
                            column_gap: Val::Px(12.0),
                            padding: UiRect::axes(Val::Px(18.0), Val::Px(9.0)),
                            ..default()
                        },
                    ))
                    .with_children(|button| {
                        button.spawn((
                            Text::new(item.icon.to_string()),
                            AutoHideText::new(0.94),
                            TextFont {
                                font: assets.icons.clone().into(),
                                font_size: FontSize::from(config.fonts.icon_size),
                                ..default()
                            },
                            TextColor(Color::srgba(0.96, 0.96, 0.98, 0.94)),
                        ));
                        button.spawn((
                            Text::new(item.label),
                            AutoHideText::new(0.88),
                            TextFont {
                                font: assets.text.clone().into(),
                                font_size: FontSize::from(config.fonts.label_size),
                                ..default()
                            },
                            TextColor(Color::srgba(0.92, 0.92, 0.95, 0.88)),
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
                right: Val::Px(30.0),
                bottom: Val::Px(217.5),
                width: Val::Px(787.5),
                height: Val::Px(202.5),
                display: Display::None,
                ..default()
            },
            FocusPolicy::Pass,
            GlobalZIndex(139),
        ));
    }
}

fn spawn_quick_preview_layer(commands: &mut Commands, dialog_camera: Entity, assets: &UiFonts) {
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

fn spawn_quick_preview(layer: &mut ChildSpawnerCommands, owner: ButtonAction, assets: &UiFonts) {
    layer
        .spawn((
            Name::new(format!("quick_preview::{owner:?}")),
            QuickPreviewPanel { owner },
            QuickPreviewFade::default(),
            QuickPreviewSurface,
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(30.0),
                bottom: Val::Px(217.5),
                width: Val::Px(787.5),
                height: Val::Px(202.5),
                display: Display::None,
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                padding: UiRect::all(Val::Px(12.0)),
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
                    width: Val::Px(318.0),
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
                        row_gap: Val::Px(9.0),
                        padding: UiRect::left(Val::Px(15.0)),
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
                            font: assets.text.clone().into(),
                            font_size: FontSize::from(24.0),
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
                            font: assets.text.clone().into(),
                            font_size: FontSize::from(19.5),
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
                    font: assets.text.clone().into(),
                    font_size: FontSize::from(22.5),
                    ..default()
                },
                TextColor(Color::srgba(1.0, 1.0, 1.0, 0.67)),
            ));
        });
}

#[allow(
    clippy::too_many_arguments,
    reason = "independent ECS queries keep overlapping UI component access explicit"
)]
pub fn update_textbox(
    resources: TextboxUpdateResources,
    mut cache: Local<TextboxRenderCache>,
    mut commands: Commands,
    mut content_root: Query<&mut Node, ContentRootFilter>,
    mut speaker_text: Query<&mut Text, (With<SpeakerText>, Without<DialogueText>)>,
    dialogue_root: Query<Entity, With<DialogueText>>,
    mut glyphs: Query<(&DialogueGlyph, &mut Visibility)>,
    mut name_bar: Query<&mut Node, NameBarFilter>,
    mut text_box: Query<(&mut Node, &mut BackgroundColor, &mut HideContentBg), TextBoxFilter>,
) {
    let state = &resources.state;
    let config = &resources.config;
    let content_hidden = state.ended || state.textbox_hidden;
    if cache.content_hidden != Some(content_hidden)
        && let Ok(mut root) = content_root.single_mut()
    {
        root.display = if content_hidden {
            Display::None
        } else {
            Display::Flex
        };
        cache.content_hidden = Some(content_hidden);
    }

    let (speaker, markup, visible_chars) =
        state.dialogue.as_ref().map_or(("", "", 0), |dialogue| {
            (
                dialogue.speaker.as_str(),
                dialogue.markup.as_str(),
                dialogue.visible_chars,
            )
        });

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
    let speaker_changed = cache.speaker != speaker;
    let dialogue_changed = cache.dialogue != markup;
    let visibility_changed = cache.visible_chars != visible_chars;
    let layout_changed = cache.left != Some(left);
    if layout_changed || speaker_changed {
        for mut node in &mut name_bar {
            if layout_changed {
                node.left = Val::Percent(left);
                node.width = Val::Auto;
            }
            if speaker_changed {
                node.display = if speaker.is_empty() {
                    Display::None
                } else {
                    Display::Flex
                };
            }
        }
    }
    let textbox_alpha =
        (config.styles.textbox_alpha * (resources.settings.textbox_opacity / 0.75)).clamp(0.0, 1.0);
    let alpha_changed = cache.textbox_alpha != Some(textbox_alpha);
    if layout_changed || alpha_changed {
        for (mut node, mut background, mut hidden) in &mut text_box {
            if layout_changed {
                node.left = Val::Percent(left);
                node.width = Val::Percent(width);
            }
            if alpha_changed {
                hidden.base_alpha = textbox_alpha;
                background.0 = background.0.with_alpha(
                    textbox_alpha * resources.auto_hide.hide_alpha * resources.overlay.alpha,
                );
            }
        }
    }

    if speaker_changed && let Ok(mut text) = speaker_text.single_mut() {
        text.0.clear();
        text.0.push_str(speaker);
    }
    let scale = match resources.settings.text_size {
        0 => 0.86,
        2 => 1.16,
        _ => 1.0,
    };
    let dialogue_size = config.fonts.dialogue_size * scale;
    let dialogue_size_changed = cache.dialogue_size != Some(dialogue_size);
    if (dialogue_changed || dialogue_size_changed)
        && let Ok(root) = dialogue_root.single()
    {
        commands.entity(root).despawn_related::<Children>();
        spawn_rich_dialogue(
            &mut commands,
            root,
            markup,
            visible_chars,
            dialogue_size,
            &resources.fonts.text,
        );
    } else if visibility_changed {
        for (glyph, mut visibility) in &mut glyphs {
            *visibility = if glyph.reveal_at <= visible_chars {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
    }
    if speaker_changed {
        cache.speaker.clear();
        cache.speaker.push_str(speaker);
    }
    if dialogue_changed {
        cache.dialogue.clear();
        cache.dialogue.push_str(markup);
    }
    cache.visible_chars = visible_chars;
    cache.left = Some(left);
    cache.textbox_alpha = Some(textbox_alpha);
    cache.dialogue_size = Some(dialogue_size);
}

#[derive(Clone)]
struct RichStyle {
    color: Color,
    scale: f32,
    weight: FontWeight,
    font_style: FontStyle,
}

impl Default for RichStyle {
    fn default() -> Self {
        Self {
            color: Color::WHITE,
            scale: 1.0,
            weight: FontWeight::NORMAL,
            font_style: FontStyle::Normal,
        }
    }
}

struct RichRun {
    base: String,
    ruby: Option<String>,
    style: RichStyle,
}

fn spawn_rich_dialogue(
    commands: &mut Commands,
    root: Entity,
    markup: &str,
    visible_chars: usize,
    font_size: f32,
    font: &Handle<Font>,
) {
    let runs = parse_rich_markup(markup);
    commands.entity(root).with_children(|content| {
        let mut character_index = 0;
        for run in runs {
            if run.base == "\n" {
                character_index += 1;
                content.spawn((
                    DialogueGlyph {
                        reveal_at: character_index,
                    },
                    Node {
                        width: Val::Percent(100.0),
                        height: Val::Px(0.0),
                        ..default()
                    },
                    glyph_visibility(character_index, visible_chars),
                ));
                continue;
            }
            if let Some(ruby) = run.ruby {
                character_index += run.base.chars().count();
                spawn_ruby_cluster(
                    content,
                    run.base,
                    ruby,
                    run.style,
                    character_index,
                    visible_chars,
                    font_size,
                    font,
                );
            } else {
                for value in run.base.chars() {
                    character_index += 1;
                    spawn_plain_cluster(
                        content,
                        value,
                        run.style.clone(),
                        character_index,
                        visible_chars,
                        font_size,
                        font,
                    );
                }
            }
        }
    });
}

#[allow(clippy::too_many_arguments)]
fn spawn_plain_cluster(
    content: &mut ChildSpawnerCommands,
    value: char,
    style: RichStyle,
    reveal_at: usize,
    visible_chars: usize,
    font_size: f32,
    font: &Handle<Font>,
) {
    let alpha = style.color.alpha();
    content
        .spawn((
            DialogueGlyph { reveal_at },
            Node {
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::FlexEnd,
                ..default()
            },
            glyph_visibility(reveal_at, visible_chars),
        ))
        .with_child((
            DialogueBaseGlyph,
            Text::new(value.to_string()),
            TextFont {
                font: font.clone().into(),
                font_size: FontSize::Px(font_size * style.scale),
                weight: style.weight,
                style: style.font_style,
                ..default()
            },
            TextColor(style.color),
            TextLayout::no_wrap(),
            HideContentText::new(alpha),
        ));
}

#[allow(clippy::too_many_arguments)]
fn spawn_ruby_cluster(
    content: &mut ChildSpawnerCommands,
    base: String,
    ruby: String,
    style: RichStyle,
    reveal_at: usize,
    visible_chars: usize,
    font_size: f32,
    font: &Handle<Font>,
) {
    let alpha = style.color.alpha();
    let cluster_width = ruby_cluster_width(&base, &ruby, font_size, style.scale);
    content
        .spawn((
            DialogueGlyph { reveal_at },
            Node {
                min_width: Val::Px(cluster_width),
                flex_shrink: 0.0,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::FlexEnd,
                ..default()
            },
            glyph_visibility(reveal_at, visible_chars),
        ))
        .with_children(|cluster| {
            cluster.spawn((
                DialogueRubyGlyph,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::Percent(50.0),
                    // Keep ruby in the base line's upper leading instead of
                    // letting it increase the flex line height.
                    bottom: Val::Percent(80.0),
                    ..default()
                },
                UiTransform::from_xy(Val::Percent(-50.0), Val::Px(-1.5)),
                Text::new(ruby),
                TextFont {
                    font: font.clone().into(),
                    font_size: FontSize::Px(font_size * RUBY_FONT_SCALE),
                    weight: FontWeight::MEDIUM,
                    ..default()
                },
                TextColor(style.color.with_alpha(alpha * 0.88)),
                TextLayout::no_wrap(),
                HideContentText::new(alpha * 0.88),
            ));
            cluster.spawn((
                DialogueBaseGlyph,
                Text::new(base),
                TextFont {
                    font: font.clone().into(),
                    font_size: FontSize::Px(font_size * style.scale),
                    weight: style.weight,
                    style: style.font_style,
                    ..default()
                },
                TextColor(style.color),
                TextLayout::no_wrap(),
                HideContentText::new(alpha),
            ));
        });
}

fn ruby_cluster_width(base: &str, ruby: &str, font_size: f32, base_scale: f32) -> f32 {
    let base_width = estimated_text_width(base) * font_size * base_scale;
    let ruby_width = estimated_text_width(ruby) * font_size * RUBY_FONT_SCALE;
    base_width.max(ruby_width) + RUBY_COLLISION_PADDING
}

fn estimated_text_width(text: &str) -> f32 {
    text.chars()
        .map(|value| {
            if value.is_ascii_whitespace() {
                0.34
            } else if value.is_ascii_punctuation() {
                0.52
            } else if value.is_ascii() {
                0.62
            } else {
                1.0
            }
        })
        .sum()
}

fn glyph_visibility(reveal_at: usize, visible_chars: usize) -> Visibility {
    if reveal_at <= visible_chars {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    }
}

fn parse_rich_markup(source: &str) -> Vec<RichRun> {
    let chars = source.chars().collect::<Vec<_>>();
    let mut runs = Vec::new();
    let mut plain = String::new();
    let mut cursor = 0;
    while cursor < chars.len() {
        if chars[cursor] == '\n' {
            push_plain(&mut runs, &mut plain);
            runs.push(RichRun {
                base: "\n".into(),
                ruby: None,
                style: RichStyle::default(),
            });
            cursor += 1;
            continue;
        }
        if chars[cursor] != '[' {
            plain.push(chars[cursor]);
            cursor += 1;
            continue;
        }
        let Some(label_offset) = chars[cursor + 1..].iter().position(|value| *value == ']') else {
            plain.push(chars[cursor]);
            cursor += 1;
            continue;
        };
        let label_end = cursor + 1 + label_offset;
        if chars.get(label_end + 1) != Some(&'(') {
            plain.push(chars[cursor]);
            cursor += 1;
            continue;
        }
        let Some(argument_offset) = chars[label_end + 2..]
            .iter()
            .position(|value| *value == ')')
        else {
            plain.push(chars[cursor]);
            cursor += 1;
            continue;
        };
        push_plain(&mut runs, &mut plain);
        let argument_end = label_end + 2 + argument_offset;
        let base = chars[cursor + 1..label_end].iter().collect::<String>();
        let argument = chars[label_end + 2..argument_end]
            .iter()
            .collect::<String>();
        let styled = argument.contains('=')
            || matches!(argument.as_str(), "bold" | "italic" | "bold,italic");
        runs.push(RichRun {
            base,
            ruby: (!styled && !argument.is_empty()).then_some(argument.clone()),
            style: if styled {
                parse_rich_style(&argument)
            } else {
                RichStyle::default()
            },
        });
        cursor = argument_end + 1;
    }
    push_plain(&mut runs, &mut plain);
    runs
}

fn push_plain(runs: &mut Vec<RichRun>, plain: &mut String) {
    if plain.is_empty() {
        return;
    }
    runs.push(RichRun {
        base: std::mem::take(plain),
        ruby: None,
        style: RichStyle::default(),
    });
}

fn parse_rich_style(source: &str) -> RichStyle {
    let mut style = RichStyle::default();
    for part in source.split([',', ';']) {
        let part = part.trim();
        let (key, value) = part.split_once('=').unwrap_or((part, "true"));
        match key.trim() {
            "color" => {
                if let Some(color) = parse_hex_color(value.trim()) {
                    style.color = color;
                }
            }
            "size" | "fontSize" => {
                let value = value.trim().trim_end_matches("px");
                if let Ok(value) = value.parse::<f32>() {
                    style.scale = if value > 4.0 { value / 60.0 } else { value };
                    style.scale = style.scale.clamp(0.5, 2.0);
                }
            }
            "weight" if value.eq_ignore_ascii_case("bold") => style.weight = FontWeight::BOLD,
            "bold" => style.weight = FontWeight::BOLD,
            "italic" | "style"
                if value.eq_ignore_ascii_case("true") || value.eq_ignore_ascii_case("italic") =>
            {
                style.font_style = FontStyle::Italic;
            }
            "opacity" | "alpha" => {
                if let Ok(alpha) = value.parse::<f32>() {
                    style.color = style.color.with_alpha(alpha.clamp(0.0, 1.0));
                }
            }
            _ => {}
        }
    }
    style
}

fn parse_hex_color(value: &str) -> Option<Color> {
    let value = value.trim().strip_prefix('#').unwrap_or(value.trim());
    let parse = |range| u8::from_str_radix(&value[range], 16).ok();
    match value.len() {
        6 => Some(Color::srgb_u8(parse(0..2)?, parse(2..4)?, parse(4..6)?)),
        8 => Some(Color::srgba_u8(
            parse(0..2)?,
            parse(2..4)?,
            parse(4..6)?,
            parse(6..8)?,
        )),
        _ => None,
    }
}

pub fn animate_overlay_fade(
    time: Res<Time>,
    overlays: Query<(&Visibility, &Node), TextboxOverlayFilter>,
    mut fade: ResMut<TextboxOverlayFade>,
) {
    let target = if overlays
        .iter()
        .any(|(visibility, node)| overlay_is_displayed(*visibility, node.display))
    {
        0.0
    } else {
        1.0
    };
    fade.alpha += (target - fade.alpha) * exp_lerp(time.delta_secs(), 18.0);
    if (target - fade.alpha).abs() < 0.001 {
        fade.alpha = target;
    }
}

pub fn animate_initial_fade(
    time: Res<Time>,
    state: Res<GameState>,
    mut fade: ResMut<InitialTextboxFade>,
) {
    if fade.phase == InitialTextboxFadePhase::Waiting {
        if state.ended || state.textbox_hidden || state.dialogue.is_none() {
            return;
        }
        fade.phase = InitialTextboxFadePhase::Fading;
    }
    if fade.phase != InitialTextboxFadePhase::Fading {
        return;
    }
    fade.elapsed = (fade.elapsed + time.delta_secs()).min(InitialTextboxFade::SECONDS);
    let progress = fade.elapsed / InitialTextboxFade::SECONDS;
    fade.alpha = 1.0 - (1.0 - progress).powi(3);
    if fade.elapsed >= InitialTextboxFade::SECONDS {
        fade.alpha = 1.0;
        fade.phase = InitialTextboxFadePhase::Complete;
    }
}

fn overlay_is_displayed(visibility: Visibility, display: Display) -> bool {
    visibility != Visibility::Hidden && display != Display::None
}

#[allow(
    clippy::too_many_arguments,
    reason = "the hide pass updates independent text, background, and image component families"
)]
pub fn apply_hide_toggle(
    timing: Res<AutoHideTiming>,
    overlay: Res<TextboxOverlayFade>,
    initial_fade: Res<InitialTextboxFade>,
    state: Res<GameState>,
    mut text_query: Query<(&mut TextColor, &HideContentText, Option<&mut TextShadow>)>,
    mut background_query: Query<(&mut BackgroundColor, &HideContentBg)>,
    mut avatars: Query<&mut ImageNode, With<MiniAvatarNode>>,
    added_text: Query<(), Added<HideContentText>>,
    mut last: Local<Option<(f32, f32)>>,
) {
    let alpha = timing.hide_alpha * overlay.alpha * initial_fade.alpha;
    let current = (alpha, state.mini_avatar_progress);
    if added_text.is_empty()
        && last.is_some_and(|last| {
            (last.0 - current.0).abs() < 0.001 && (last.1 - current.1).abs() < 0.001
        })
    {
        return;
    }
    *last = Some(current);
    for (mut color, hidden, shadow) in &mut text_query {
        let text_alpha = hidden.base_alpha * alpha;
        color.0 = color.0.with_alpha(text_alpha);
        if let Some(mut shadow) = shadow {
            shadow.color = shadow.color.with_alpha(0.9 * text_alpha);
        }
    }
    for (mut color, hidden) in &mut background_query {
        color.0 = color.0.with_alpha(hidden.base_alpha * alpha);
    }
    for mut avatar in &mut avatars {
        avatar.color = avatar.color.with_alpha(state.mini_avatar_progress * alpha);
    }
}

#[cfg(test)]
mod rich_text_tests {
    use super::*;

    #[test]
    fn separates_ruby_and_style_without_polluting_plain_text() {
        let runs = parse_rich_markup("読む[蟹](かに)と[桜](color=#ffb7c5,bold)");
        assert_eq!(runs.len(), 4);
        assert_eq!(runs[1].base, "蟹");
        assert_eq!(runs[1].ruby.as_deref(), Some("かに"));
        assert_eq!(runs[3].base, "桜");
        assert!(runs[3].ruby.is_none());
        assert_eq!(runs[3].style.weight, FontWeight::BOLD);
    }

    #[test]
    fn rejects_invalid_color_without_panicking() {
        assert!(parse_hex_color("#xyz").is_none());
        assert!(parse_hex_color("#12345").is_none());
    }

    #[test]
    fn ignores_overlay_roots_hidden_by_display() {
        assert!(!overlay_is_displayed(Visibility::Inherited, Display::None));
        assert!(!overlay_is_displayed(Visibility::Hidden, Display::Flex));
        assert!(overlay_is_displayed(Visibility::Inherited, Display::Flex));
    }

    #[test]
    fn ruby_cluster_reserves_horizontal_collision_space() {
        let base_only = estimated_text_width("物") * 60.0;
        let cluster = ruby_cluster_width("物", "ものがたり", 60.0, 1.0);
        assert!(cluster > base_only);
    }
}
