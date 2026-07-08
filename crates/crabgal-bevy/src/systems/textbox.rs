// Textbox — WebGAL-style: name bar + dialogue area, slide-right dodge on sprite/mini-avatar enter.
use bevy::prelude::*;

use crate::resources::*;

#[derive(Component)] pub(crate) struct SpeakerText;
#[derive(Component)] pub(crate) struct DialogueText;
#[derive(Component)] pub(crate) struct TextBoxRoot;
#[derive(Component)] pub(crate) struct NameBarRoot;

pub fn setup_textbox(mut commands: Commands, cfg: Res<Cfg>, asset_server: Res<AssetServer>) {
    let font: Handle<Font> = asset_server.load("fonts/MavenPro-CJK.ttf");
    let alpha = cfg.0.styles.textbox_alpha;

    commands.spawn((
        Name::new("name_bar"), NameBarRoot,
        Node { position_type: PositionType::Absolute, bottom: Val::Percent(22.0), left: Val::Percent(7.0),
            padding: UiRect { left: Val::Px(28.0), right: Val::Px(28.0), top: Val::Px(10.0), bottom: Val::Px(10.0) },
            min_width: Val::Px(120.0), ..default() },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)), ZIndex(102),
    )).with_children(|p| { p.spawn((Name::new("speaker"), SpeakerText, Text::new(""),
        TextFont { font: font.clone().into(), font_size: 32.0, ..default() },
        TextColor(Color::srgba(1.0, 0.85, 0.6, 1.0)), )); });

    commands.spawn((
        Name::new("textbox"), TextBoxRoot,
        Node { position_type: PositionType::Absolute, bottom: Val::Percent(3.0), left: Val::Percent(7.0),
            width: Val::Percent(86.0), height: Val::Percent(18.0),
            padding: UiRect { left: Val::Px(28.0), right: Val::Px(28.0), top: Val::Px(36.0), bottom: Val::Px(20.0) },
            ..default() },
        BackgroundColor(Color::srgba(0.05, 0.05, 0.08, alpha)), ZIndex(101),
    )).with_children(|p| { p.spawn((Name::new("dialogue"), DialogueText, Text::new(""),
        TextFont { font: font.into(), font_size: 30.0, ..default() },
        TextColor(Color::WHITE), Node { width: Val::Percent(100.0), ..default() }, )); });
}

pub fn update_textbox(
    state: Res<AppState>,
    mut text_q: ParamSet<(
        Query<(&Name, &mut Text), With<SpeakerText>>,
        Query<(&Name, &mut Text), With<DialogueText>>,
    )>,
    mut node_q: ParamSet<(
        Query<&mut Node, With<NameBarRoot>>,
        Query<&mut Node, With<TextBoxRoot>>,
    )>,
    window_query: Query<&Window>,
) {
    let s = state.0.read().unwrap();
    let w = window_query.single();
    let sc = (w.width() / 2560.0).min(w.height() / 1440.0);

    let (speaker, dialogue) = if let Some(ref d) = s.dialogue {
        let chars: Vec<char> = d.text.chars().collect();
        let vis: String = chars.iter().take(d.visible_chars).collect();
        (d.speaker.clone(), vis)
    } else { (String::new(), String::new()) };

    // Dodge offset: slide right when mini-avatar or left-entering sprite appears
    let mut dodge: f32 = 0.0;
    if s.mini_avatar.is_some() {
        dodge = s.mini_avatar_progress * 200.0 * sc;
    }
    for (_, sp) in s.sprites.iter() {
        if sp.entering {
            use crabgal_core::Anchor;
            if let Anchor::Left(_) = sp.position.x {
                dodge = dodge.max(sp.transition_progress * 200.0 * sc);
            }
        }
    }
    let left_px = w.width() * 0.07 + dodge;

    for mut n in node_q.p0().iter_mut() { n.left = Val::Px(left_px); }
    for mut n in node_q.p1().iter_mut() { n.left = Val::Px(left_px); }

    for (name, mut t) in text_q.p0().iter_mut() { if name.as_str() == "speaker" { t.0 = speaker.clone(); } }
    for (name, mut t) in text_q.p1().iter_mut() { if name.as_str() == "dialogue" { t.0 = dialogue.clone(); } }
}
