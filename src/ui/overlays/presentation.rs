use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy::ui::FocusPolicy;
use crabgal_core::{DESIGN_HEIGHT, DESIGN_WIDTH};

use crate::render::blur::DialogCamera;
use crate::runtime::resources::GameState;
use crate::ui::foundation::UiFonts;

#[derive(Component)]
pub(crate) struct IntroRoot;

#[derive(Component)]
pub(crate) struct IntroText;

#[derive(Component)]
pub(crate) struct FilmBar;

pub(crate) fn setup(
    mut commands: Commands,
    fonts: Res<UiFonts>,
    cameras: Query<Entity, With<DialogCamera>>,
) {
    let Ok(camera) = cameras.single() else {
        return;
    };
    commands
        .spawn((
            Name::new("intro_overlay"),
            IntroRoot,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(DESIGN_WIDTH),
                height: Val::Px(DESIGN_HEIGHT),
                display: Display::None,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                padding: UiRect::horizontal(Val::Px(240.0)),
                ..default()
            },
            BackgroundColor(Color::BLACK),
            FocusPolicy::Pass,
            UiTargetCamera(camera),
            RenderLayers::layer(2),
            ZIndex(900),
        ))
        .with_child((
            IntroText,
            Text::new(""),
            TextFont {
                font: FontSource::Handle(fonts.text.clone()),
                font_size: FontSize::Px(54.0),
                ..default()
            },
            TextColor(Color::WHITE),
            TextLayout::justify(Justify::Center),
        ));

    for (name, top) in [("film_bar_top", true), ("film_bar_bottom", false)] {
        commands.spawn((
            Name::new(name),
            FilmBar,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(DESIGN_WIDTH),
                height: Val::Px(92.0),
                top: if top { Val::Px(0.0) } else { Val::Auto },
                bottom: if top { Val::Auto } else { Val::Px(0.0) },
                display: Display::None,
                ..default()
            },
            BackgroundColor(Color::BLACK),
            FocusPolicy::Pass,
            UiTargetCamera(camera),
            RenderLayers::layer(2),
            ZIndex(800),
        ));
    }
}

pub(crate) fn sync(
    state: Res<GameState>,
    mut intro_roots: Query<&mut Node, (With<IntroRoot>, Without<FilmBar>)>,
    mut intro_texts: Query<(&mut Text, &mut TextColor), With<IntroText>>,
    mut film_bars: Query<&mut Node, (With<FilmBar>, Without<IntroRoot>)>,
) {
    if !state.is_changed() {
        return;
    }
    let intro = state.intro.as_ref();
    for mut node in &mut intro_roots {
        node.display = if intro.is_some() {
            Display::Flex
        } else {
            Display::None
        };
    }
    for (mut text, mut color) in &mut intro_texts {
        let Some(intro) = intro else {
            text.0.clear();
            continue;
        };
        text.0 = intro.pages.get(intro.page).cloned().unwrap_or_default();
        let alpha = (intro.elapsed / 0.22).clamp(0.0, 1.0);
        color.0 = Color::srgba(1.0, 1.0, 1.0, alpha);
    }
    for mut node in &mut film_bars {
        node.display = if state.film_mode {
            Display::Flex
        } else {
            Display::None
        };
    }
}
