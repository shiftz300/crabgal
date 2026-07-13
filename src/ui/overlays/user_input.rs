use bevy::camera::visibility::RenderLayers;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::prelude::*;
use crabgal_core::{DESIGN_HEIGHT, DESIGN_WIDTH};

use crate::render::blur::DialogCamera;
use crate::runtime::resources::GameState;
use crate::ui::foundation::UiFonts;

#[derive(Component)]
pub(crate) struct UserInputRoot;
#[derive(Component)]
pub(crate) struct UserInputTitle;
#[derive(Component)]
pub(crate) struct UserInputValue;
#[derive(Component)]
pub(crate) struct UserInputConfirm;

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
            Name::new("user_input_overlay"),
            UserInputRoot,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(DESIGN_WIDTH),
                height: Val::Px(DESIGN_HEIGHT),
                display: Display::None,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.72)),
            UiTargetCamera(camera),
            RenderLayers::layer(2),
            GlobalZIndex(980),
        ))
        .with_children(|root| {
            root.spawn((
                Node {
                    width: Val::Px(940.0),
                    padding: UiRect::all(Val::Px(48.0)),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(28.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.035, 0.04, 0.055, 0.94)),
            ))
            .with_children(|panel| {
                panel.spawn((
                    UserInputTitle,
                    Text::new("INPUT"),
                    TextFont {
                        font: fonts.text.clone().into(),
                        font_size: FontSize::Px(38.0),
                        ..default()
                    },
                    TextColor(Color::srgba(1.0, 1.0, 1.0, 0.88)),
                ));
                panel.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        min_height: Val::Px(92.0),
                        padding: UiRect::axes(Val::Px(28.0), Val::Px(18.0)),
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.09)),
                    children![(
                        UserInputValue,
                        Text::new(""),
                        TextFont {
                            font: fonts.text.clone().into(),
                            font_size: FontSize::Px(40.0),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    )],
                ));
                panel.spawn((
                    UserInputConfirm,
                    Button,
                    Node {
                        align_self: AlignSelf::FlexEnd,
                        min_width: Val::Px(210.0),
                        padding: UiRect::axes(Val::Px(34.0), Val::Px(18.0)),
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.16)),
                    children![(
                        Text::new("OK"),
                        TextFont {
                            font: fonts.text.clone().into(),
                            font_size: FontSize::Px(30.0),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    )],
                ));
            });
        });
}

pub(crate) fn sync(
    state: Res<GameState>,
    mut roots: Query<&mut Node, With<UserInputRoot>>,
    mut titles: Query<&mut Text, (With<UserInputTitle>, Without<UserInputValue>)>,
    mut values: Query<&mut Text, (With<UserInputValue>, Without<UserInputTitle>)>,
    confirm: Query<&Children, With<UserInputConfirm>>,
    mut texts: Query<&mut Text, (Without<UserInputTitle>, Without<UserInputValue>)>,
) {
    if !state.is_changed() {
        return;
    }
    for mut root in &mut roots {
        root.display = if state.user_input.is_some() {
            Display::Flex
        } else {
            Display::None
        };
    }
    let Some(input) = &state.user_input else {
        return;
    };
    for mut title in &mut titles {
        title.0.clone_from(&input.title);
    }
    for mut value in &mut values {
        value.0 = format!("{}▌", input.value);
    }
    for children in &confirm {
        for child in children.iter() {
            if let Ok(mut text) = texts.get_mut(child) {
                text.0.clone_from(&input.button);
            }
        }
    }
}

pub(crate) fn handle(
    mut state: ResMut<GameState>,
    mut keyboard: MessageReader<KeyboardInput>,
    buttons: Query<&Interaction, (With<UserInputConfirm>, Changed<Interaction>)>,
) {
    if state.user_input.is_none() {
        return;
    }
    let mut submit = buttons.iter().any(|value| *value == Interaction::Pressed);
    let Some(input) = state.user_input.as_mut() else {
        return;
    };
    for event in keyboard.read() {
        if !event.state.is_pressed() {
            continue;
        }
        match &event.logical_key {
            Key::Character(value) if input.value.chars().count() < 64 => {
                input.value.push_str(value);
            }
            Key::Backspace => {
                input.value.pop();
            }
            Key::Enter => submit = true,
            _ => {}
        }
    }
    if submit && !input.value.trim().is_empty() {
        crabgal_core::step::submit_user_input(&mut state);
        crabgal_core::step::step(&mut state);
    }
}
