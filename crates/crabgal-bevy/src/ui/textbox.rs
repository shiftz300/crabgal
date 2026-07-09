// Textbox — WebGAL-style: dialog box with floating name bar, top-right icon row,
// and dialogue text. Dodge offsets the entire group on sprite/mini-avatar enter.
use bevy::prelude::*;
use bevy::camera::visibility::RenderLayers;

use crate::resources::*;
use crate::ui::control_bar::{ControlBarTop, ControlBarBot, TOP_ICONS, BOT_ITEMS, HoverAlpha, ButtonAction, AutoHideBar, AutoHideText, AutoHideTiming, HideContentText, HideContentBg, HideButtonText, LockIcon, BlurSource};

#[derive(Component)] pub(crate) struct SpeakerText;
#[derive(Component)] pub(crate) struct DialogueText;
#[derive(Component)] pub(crate) struct TextBoxRoot;
#[derive(Component)] pub(crate) struct NameBarRoot;
/// Root UI container for the 16:9 letterbox area. All game UI is children of this.
#[derive(Component)] pub(crate) struct ContentRoot;

pub fn setup_textbox(mut commands: Commands, cfg: Res<Cfg>, asset_server: Res<AssetServer>) {
    // ── Fixed 16:9 design canvas (2560x1440) — UiScale handles window mapping ──
    commands.spawn((
        Name::new("content_root"), ContentRoot,
        Node {
            position_type: PositionType::Absolute,
            width: Val::Px(2560.0),
            height: Val::Px(1440.0),
            ..default()
        },
        RenderLayers::layer(1),
    )).with_children(|root| {

    let font: Handle<Font> = asset_server.load("fonts/MavenPro-CJK.ttf");
    let icon_font: Handle<Font> = asset_server.load("fonts/bootstrap-icons.ttf");
    let label_font: Handle<Font> = asset_server.load("fonts/MavenPro-CJK.ttf");
    let alpha = cfg.0.styles.textbox_alpha;
    let l = &cfg.0.layout;
    let f = &cfg.0.fonts;

    root.spawn((
        Name::new("name_bar"), NameBarRoot,
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Percent(l.namebar_bottom), left: Val::Percent(l.textbox_left),
            padding: UiRect { left: Val::Px(20.0), right: Val::Px(20.0), top: Val::Px(6.0), bottom: Val::Px(6.0) },
            min_width: Val::Px(100.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
        HideContentBg::new(0.7),
        BlurSource,
        ZIndex(102),
        RenderLayers::layer(1),
    )).with_children(|nb| {
        nb.spawn((
            Name::new("speaker"), SpeakerText, Text::new(""),
            TextFont { font: font.clone().into(), font_size: FontSize::from(f.speaker_size), ..default() },
            TextColor(Color::WHITE),
            HideContentText::new(1.0),
        ));
    });

    root.spawn((
        Name::new("textbox"), TextBoxRoot,
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Percent(l.textbox_bottom),
            left: Val::Percent(l.textbox_left),
            width: Val::Percent(100.0 - l.textbox_left),
            height: Val::Percent(l.textbox_height),
            padding: UiRect { left: Val::Px(56.0), right: Val::Px(56.0), top: Val::Px(72.0), bottom: Val::Px(40.0) },
            ..default()
        },
        BackgroundColor(Color::srgba(0.05, 0.05, 0.08, alpha)),
        HideContentBg::new(alpha),
        BlurSource,
        ZIndex(101),
        RenderLayers::layer(1),
    )).with_children(|p| {
        // ── Top-right icon row: relative to textbox inner top-right ──
        p.spawn((
            Name::new("ctrl_top"), ControlBarTop, AutoHideBar,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(0.0), right: Val::Px(24.0),
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(4.0),
                ..default()
            },
            ZIndex(110),
        )).with_children(|icons| {
            let top_actions = [ButtonAction::Backlog, ButtonAction::Replay, ButtonAction::Auto, ButtonAction::Skip, ButtonAction::Hide, ButtonAction::Lock];
            for (i, &(ch, _name)) in TOP_ICONS.iter().enumerate() {
                let text_entity = (
                    Text::new(ch.to_string()), AutoHideText::new(0.85),
                    TextFont { font: icon_font.clone().into(), font_size: FontSize::from(f.icon_size), ..default() },
                    TextColor(Color::srgba(0.96, 0.96, 0.97, 0.85)),
                );
                if matches!(top_actions[i], ButtonAction::Hide) {
                    icons.spawn((
                        Button,
                        HoverAlpha::default(),
                        top_actions[i],
                        Node {
                            width: Val::Px(72.0), height: Val::Px(68.0),
                            padding: UiRect::all(Val::Px(8.0)),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                    )).with_child((text_entity, HideButtonText));
                } else if matches!(top_actions[i], ButtonAction::Lock) {
                    icons.spawn((
                        Button,
                        HoverAlpha::default(),
                        top_actions[i],
                        Node {
                            width: Val::Px(72.0), height: Val::Px(68.0),
                            padding: UiRect::all(Val::Px(8.0)),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                    )).with_child((text_entity, LockIcon));
                } else {
                    icons.spawn((
                        Button,
                        HoverAlpha::default(),
                        top_actions[i],
                        Node {
                            width: Val::Px(72.0), height: Val::Px(68.0),
                            padding: UiRect::all(Val::Px(8.0)),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                    )).with_child(text_entity);
                }
            }
        });

        // ── Bottom quick menu: icon+label row, bottom-right inside textbox ──
        p.spawn((
            Name::new("ctrl_bot"), ControlBarBot, AutoHideBar,
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(0.0), right: Val::Px(24.0),
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(2.0),
                ..default()
            },
            ZIndex(110),
        )).with_children(|menu| {
            let bot_actions = [ButtonAction::QuickSave, ButtonAction::QuickLoad, ButtonAction::Save, ButtonAction::Load, ButtonAction::System, ButtonAction::Title];
            for (i, &(ch, label)) in BOT_ITEMS.iter().enumerate() {
                menu.spawn((
                    Button,
                    HoverAlpha::default(),
                    bot_actions[i],
                    Node {
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(16.0),
                        padding: UiRect { left: Val::Px(24.0), right: Val::Px(24.0), top: Val::Px(12.0), bottom: Val::Px(12.0) },
                        ..default()
                    },
                )).with_children(|btn| {
                    btn.spawn((
                        Text::new(ch.to_string()), AutoHideText::new(0.85),
                        TextFont { font: icon_font.clone().into(), font_size: FontSize::from(f.icon_size), ..default() },
                        TextColor(Color::srgba(0.88, 0.88, 0.92, 0.85)),
                    ));
                    btn.spawn((
                        Text::new(label), AutoHideText::new(0.75),
                        TextFont { font: label_font.clone().into(), font_size: FontSize::from(f.label_size), ..default() },
                        TextColor(Color::srgba(0.78, 0.78, 0.84, 0.75)),
                    ));
                });
            }
        });

        // ── Dialogue text ──
        p.spawn((
            Name::new("dialogue"), DialogueText, Text::new(""),
            TextFont { font: font.into(), font_size: FontSize::from(f.dialogue_size), ..default() },
            TextColor(Color::WHITE),
            HideContentText::new(1.0),
            Node { width: Val::Percent(100.0), ..default() },
        ));
    });

    }); // with_children root
}

pub fn update_textbox(
    state: Res<AppState>,
    cfg: Res<Cfg>,
    mut text_q: ParamSet<(
        Query<(&Name, &mut Text), With<SpeakerText>>,
        Query<(&Name, &mut Text), With<DialogueText>>,
    )>,
    mut node_q: ParamSet<(
        Query<&mut Node, With<NameBarRoot>>,
        Query<&mut Node, With<TextBoxRoot>>,
    )>,
) {
    let s = state.0.read().unwrap();

    let (speaker, dialogue) = if let Some(ref d) = s.dialogue {
        let chars: Vec<char> = d.text.chars().collect();
        let vis: String = chars.iter().take(d.visible_chars).collect();
        (d.speaker.clone(), vis)
    } else { (String::new(), String::new()) };

    // Dodge: when left sprite is entering, shift textbox right by config offset.
    let has_left = s.sprites.iter().any(|(_, sp)|
        sp.entering && matches!(sp.position.x, crabgal_core::Anchor::Left(_))
    );
    let l = &cfg.0.layout;
    let left_pct = if has_left { l.textbox_dodge_left } else { l.textbox_left };
    let width_pct = 100.0 - left_pct;
    for mut n in node_q.p0().iter_mut() {
        n.left = Val::Percent(left_pct);
        n.width = Val::Percent(width_pct);
    }
    for mut n in node_q.p1().iter_mut() {
        n.left = Val::Percent(left_pct);
        n.width = Val::Percent(width_pct);
    }

    for (name, mut t) in text_q.p0().iter_mut() { if name.as_str() == "speaker" { t.0 = speaker.clone(); } }
    for (name, mut t) in text_q.p1().iter_mut() { if name.as_str() == "dialogue" { t.0 = dialogue.clone(); } }
}

/// Hide/show textbox content, background, and name bar; control bars stay visible.
pub fn apply_hide_toggle(
    timing: Res<AutoHideTiming>,
    mut text_q: Query<(&mut TextColor, &HideContentText)>,
    mut bg_q: Query<(&mut BackgroundColor, &HideContentBg)>,
) {
    let a = timing.hide_alpha;
    for (mut tc, ht) in text_q.iter_mut() {
        tc.0 = tc.0.with_alpha(ht.base_alpha * a);
    }
    for (mut bg, hb) in bg_q.iter_mut() {
        bg.0 = bg.0.with_alpha(hb.base_alpha * a);
    }
}
