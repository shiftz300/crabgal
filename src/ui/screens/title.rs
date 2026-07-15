use bevy::camera::visibility::RenderLayers;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::ui::FocusPolicy;

use crate::render::blur::{DialogCamera, UiBlurCamera};
use crate::runtime::input::InputActions;
use crate::runtime::resources::{GameConfigResource, GameState, ProjectRoot};
use crate::runtime::viewport::DesignViewport;
use crate::ui::control_bar::{BlurSource, BlurStrength, HoverAlpha, QuickSavePreview};
use crate::ui::dialog::{DialogAction, DialogRequest};
use crate::ui::foundation::{UiFonts, exp_lerp, text};
use crabgal_core::{DESIGN_HEIGHT, DESIGN_WIDTH};

#[derive(Component)]
pub struct TitleRoot;

#[derive(Component)]
pub struct TitleBackground;

#[derive(Component, Clone, Copy)]
pub enum TitleAction {
    Start,
    Continue,
    Load,
    Extra,
    Options,
    Exit,
}

#[derive(Resource)]
pub struct PendingTitleAction {
    action: TitleAction,
    remaining: f32,
}

#[derive(Resource, Default)]
pub(crate) struct ReturnToTitleTransition {
    elapsed: f32,
    switched: bool,
}

#[derive(Component)]
pub(crate) struct ReturnToTitleOverlay;

pub(crate) fn animate_return_to_title(
    transition: Option<ResMut<ReturnToTitleTransition>>,
    time: Res<Time>,
    mut state: ResMut<GameState>,
    camera: Query<Entity, With<DialogCamera>>,
    mut overlays: Query<(Entity, &mut BackgroundColor), With<ReturnToTitleOverlay>>,
    mut commands: Commands,
) {
    const COVER_SECONDS: f32 = 0.18;
    const REVEAL_SECONDS: f32 = 0.28;
    let Some(mut transition) = transition else {
        return;
    };
    if overlays.is_empty()
        && let Ok(camera) = camera.single()
    {
        commands.spawn((
            ReturnToTitleOverlay,
            Node {
                position_type: PositionType::Absolute,
                width: Val::Px(DESIGN_WIDTH),
                height: Val::Px(DESIGN_HEIGHT),
                ..default()
            },
            BackgroundColor(Color::NONE),
            FocusPolicy::Block,
            GlobalZIndex(1000),
            UiTargetCamera(camera),
            RenderLayers::layer(2),
        ));
    }
    transition.elapsed += time.delta_secs();
    if !transition.switched && transition.elapsed >= COVER_SECONDS {
        crabgal_core::step::end_game(&mut state);
        transition.switched = true;
    }
    let alpha = if transition.elapsed < COVER_SECONDS {
        crate::ui::foundation::smoothstep(transition.elapsed / COVER_SECONDS)
    } else {
        1.0 - crate::ui::foundation::smoothstep(
            (transition.elapsed - COVER_SECONDS) / REVEAL_SECONDS,
        )
    }
    .clamp(0.0, 1.0);
    for (_, mut background) in &mut overlays {
        background.0 = Color::srgba(0.0, 0.0, 0.0, alpha);
    }
    if transition.elapsed >= COVER_SECONDS + REVEAL_SECONDS {
        for (entity, _) in &mut overlays {
            commands.entity(entity).despawn();
        }
        commands.remove_resource::<ReturnToTitleTransition>();
    }
}

#[derive(Component)]
pub struct TitleButtonMotion {
    width: f32,
    padding: f32,
    press: f32,
}

impl TitleButtonMotion {
    pub(crate) fn is_animating(&self, interaction: Interaction) -> bool {
        let (target_width, target_padding) = match interaction {
            Interaction::None => (100.0, 22.5),
            Interaction::Hovered => (96.25, 22.5),
            Interaction::Pressed => (92.0, 20.25),
        };
        (self.width - target_width).abs() > 0.001
            || (self.padding - target_padding).abs() > 0.001
            || self.press > 0.001
    }
}

#[derive(Component)]
pub struct TitleContinuePreview;

type TitleButtonAnimationQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static Interaction,
        &'static mut TitleButtonMotion,
        &'static mut Node,
        &'static mut UiTransform,
        Option<&'static TitleAction>,
    ),
    Without<TitleContinuePreview>,
>;

#[derive(SystemParam)]
pub struct TitleSyncContext<'w, 's> {
    config: Res<'w, GameConfigResource>,
    roots: Query<'w, 's, (Entity, &'static mut Node), With<TitleRoot>>,
    backgrounds:
        Query<'w, 's, (Entity, &'static mut Sprite, &'static mut Transform), With<TitleBackground>>,
    camera: Query<'w, 's, Entity, With<UiBlurCamera>>,
    fonts: Res<'w, UiFonts>,
    asset_server: Res<'w, AssetServer>,
    preview: Res<'w, QuickSavePreview>,
    windows: Query<'w, 's, &'static Window>,
    commands: Commands<'w, 's>,
}

#[derive(SystemParam)]
pub struct TitleInputContext<'w, 's> {
    buttons: Query<'w, 's, (&'static Interaction, &'static TitleAction), Changed<Interaction>>,
    keys: ResMut<'w, ButtonInput<KeyCode>>,
    actions: ResMut<'w, InputActions>,
    state: ResMut<'w, GameState>,
    project_root: Res<'w, ProjectRoot>,
    store: Res<'w, crate::runtime::resources::StoreCodec>,
    time: Res<'w, Time>,
    pending: Option<ResMut<'w, PendingTitleAction>>,
    save_load: ResMut<'w, crate::ui::save_load::SaveLoadUi>,
    settings: ResMut<'w, crate::ui::settings_panel::SettingsUi>,
    runtime_settings: Res<'w, crate::storage::settings::RuntimeSettings>,
    extra: ResMut<'w, crate::ui::extra::ExtraUi>,
    commands: Commands<'w, 's>,
}

pub fn sync_title(state: Res<GameState>, mut context: TitleSyncContext) {
    let Ok(window) = context.windows.single() else {
        return;
    };
    let viewport = DesignViewport::from_window(window);
    for (_, mut node) in &mut context.roots {
        layout_title_root(&mut node, viewport);
    }
    for (_, mut sprite, mut transform) in &mut context.backgrounds {
        layout_title_background(&mut sprite, &mut transform, viewport);
    }
    if state.ended != context.roots.is_empty() {
        return;
    }
    for (entity, _) in &context.roots {
        context.commands.entity(entity).despawn();
    }
    for (entity, _, _) in &context.backgrounds {
        context.commands.entity(entity).despawn();
    }
    if !state.ended {
        return;
    }
    let Ok(camera) = context.camera.single() else {
        return;
    };
    let font = context.fonts.text.clone();
    let background: Handle<Image> = context
        .asset_server
        .load(context.config.bg_path(&context.config.title_background));
    context.commands.spawn((
        Name::new("title_background"),
        TitleBackground,
        Sprite {
            image: background,
            custom_size: Some(Vec2::new(DESIGN_WIDTH, DESIGN_HEIGHT) * viewport.scale),
            ..default()
        },
        Transform::from_translation(viewport.content_center().extend(0.0)),
        RenderLayers::layer(0),
    ));
    context
        .commands
        .spawn((
            Name::new("title"),
            TitleRoot,
            Node {
                position_type: PositionType::Absolute,
                left: Val::ZERO,
                top: Val::ZERO,
                width: Val::Px(DESIGN_WIDTH),
                height: Val::Px(DESIGN_HEIGHT),
                ..default()
            },
            BackgroundColor(Color::NONE),
            FocusPolicy::Block,
            GlobalZIndex(160),
            UiTargetCamera(camera),
            RenderLayers::layer(1),
        ))
        .with_children(|title| {
            title.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.01, 0.015, 0.04, 0.2)),
            ));
            title
                .spawn((Node {
                    position_type: PositionType::Absolute,
                    right: Val::Percent(10.0),
                    top: Val::Percent(17.0),
                    width: Val::Percent(20.5),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(30.0),
                    ..default()
                },))
                .with_children(|menu| {
                    spawn_title_button(menu, "START", Some(TitleAction::Start), &font, None);
                    if context.preview.is_compatible(state.program_fingerprint) {
                        spawn_title_button(
                            menu,
                            "CONTINUE",
                            Some(TitleAction::Continue),
                            &font,
                            Some(&context.preview),
                        );
                    } else {
                        spawn_disabled_button(menu, "CONTINUE", &font);
                    }
                    spawn_title_button(menu, "LOAD", Some(TitleAction::Load), &font, None);
                    spawn_title_button(menu, "EXTRA", Some(TitleAction::Extra), &font, None);
                    spawn_title_button(menu, "OPTIONS", Some(TitleAction::Options), &font, None);
                    spawn_title_button(menu, "EXIT", Some(TitleAction::Exit), &font, None);
                });
        });
}

fn layout_title_root(node: &mut Node, _viewport: DesignViewport) {
    node.left = Val::ZERO;
    node.top = Val::ZERO;
    node.width = Val::Px(DESIGN_WIDTH);
    node.height = Val::Px(DESIGN_HEIGHT);
}

fn layout_title_background(
    sprite: &mut Sprite,
    transform: &mut Transform,
    viewport: DesignViewport,
) {
    sprite.custom_size = Some(Vec2::new(DESIGN_WIDTH, DESIGN_HEIGHT) * viewport.scale);
    transform.translation = viewport.content_center().extend(0.0);
}

fn spawn_title_button(
    menu: &mut ChildSpawnerCommands,
    label: &str,
    action: Option<TitleAction>,
    font: &Handle<Font>,
    preview: Option<&QuickSavePreview>,
) {
    // The parent only reserves layout space. The visible surface lives inside
    // the animated button so press scaling affects the complete rectangle,
    // rather than only its text and padding.
    menu.spawn((
        Node {
            position_type: PositionType::Relative,
            width: Val::Percent(100.0),
            height: Val::Px(94.5),
            ..default()
        },
        BackgroundColor(Color::NONE),
    ))
    .with_children(|surface| {
        let mut entity = surface.spawn((
            Button,
            HoverAlpha {
                active_alpha: 0.035,
                hover_alpha: 0.035,
                ..default()
            },
            TitleButtonMotion {
                width: 100.0,
                padding: 22.5,
                press: 0.0,
            },
            UiTransform::default(),
            BlurSource,
            BlurStrength(7.5),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                padding: UiRect::left(Val::Px(22.5)),
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::NONE),
            children![
                (
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::ZERO,
                        top: Val::ZERO,
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.64)),
                    FocusPolicy::Pass,
                ),
                text(label, font, 37.5, 0.9)
            ],
        ));
        if let Some(action) = action {
            entity.insert(action);
        }
        if let Some(preview) = preview {
            spawn_continue_preview(surface, preview, font);
        }
    });
}

fn spawn_continue_preview(
    surface: &mut ChildSpawnerCommands,
    preview: &QuickSavePreview,
    font: &Handle<Font>,
) {
    surface
        .spawn((
            TitleContinuePreview,
            Node {
                position_type: PositionType::Absolute,
                right: Val::Percent(105.0),
                top: Val::Px(0.0),
                width: Val::Px(675.0),
                height: Val::Px(172.5),
                padding: UiRect::all(Val::Px(9.0)),
                display: Display::None,
                column_gap: Val::Px(10.5),
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.78)),
            BlurSource,
            BlurStrength(36.0),
        ))
        .with_children(|panel| {
            if let Some(image) = &preview.image {
                panel.spawn((
                    ImageNode::new(image.clone()),
                    Node {
                        width: Val::Px(270.0),
                        height: Val::Percent(100.0),
                        flex_shrink: 0.0,
                        ..default()
                    },
                ));
            }
            let (speaker, dialogue) = preview
                .state
                .as_ref()
                .map_or(("NO SAVE DATA", ""), |state| {
                    (state.speaker.as_str(), state.dialogue.as_str())
                });
            panel
                .spawn((Node {
                    flex_grow: 1.0,
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(9.0),
                    ..default()
                },))
                .with_children(|copy| {
                    copy.spawn(text(speaker, font, 22.5, 0.8));
                    copy.spawn(text(dialogue, font, 18.75, 0.67));
                });
        });
}

fn spawn_disabled_button(menu: &mut ChildSpawnerCommands, label: &str, font: &Handle<Font>) {
    menu.spawn((
        BlurSource,
        BlurStrength(7.5),
        Node {
            width: Val::Percent(100.0),
            height: Val::Px(94.5),
            padding: UiRect::left(Val::Px(25.5)),
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(Color::srgba(0.32, 0.32, 0.34, 0.2)),
        children![text(label, font, 37.5, 0.36)],
    ));
}

pub fn handle_title_input(mut context: TitleInputContext) {
    let action = context.buttons.iter().find_map(|(interaction, action)| {
        (*interaction == Interaction::Pressed).then_some(*action)
    });
    if !context.state.ended {
        context.commands.remove_resource::<PendingTitleAction>();
        return;
    }
    if start_game_from_keyboard(&mut context.keys, &mut context.actions, &mut context.state) {
        return;
    }
    let action = if let Some(mut pending) = context.pending {
        pending.remaining -= context.time.delta_secs();
        if pending.remaining > 0.0 {
            return;
        }
        let action = pending.action;
        context.commands.remove_resource::<PendingTitleAction>();
        Some(action)
    } else if let Some(action) = action {
        context.commands.insert_resource(PendingTitleAction {
            action,
            remaining: 0.1,
        });
        return;
    } else {
        None
    };
    match action {
        Some(TitleAction::Continue) => {
            if let Ok(loaded) = crate::storage::save::load_game(
                context.store.0.as_ref(),
                crate::storage::save::QUICK_SAVE_SLOT,
                &context.project_root,
            ) {
                if let Err(error) = loaded.restore_into(&mut context.state) {
                    log::error!("continue rejected: {error}");
                    context
                        .commands
                        .insert_resource(DialogRequest::confirmation(
                            crate::ui::support::i18n::tr(
                                context.runtime_settings.locale,
                                crate::ui::support::i18n::UiText::ForeignSave,
                            ),
                            DialogAction::Noop,
                        ));
                }
            } else {
                context
                    .commands
                    .insert_resource(DialogRequest::confirmation(
                        crate::ui::support::i18n::tr(
                            context.runtime_settings.locale,
                            crate::ui::support::i18n::UiText::NoSaveData,
                        ),
                        DialogAction::Noop,
                    ));
            }
        }
        Some(TitleAction::Exit) => {
            context
                .commands
                .insert_resource(DialogRequest::confirmation(
                    crate::ui::support::i18n::tr(
                        context.runtime_settings.locale,
                        crate::ui::support::i18n::UiText::ConfirmExit,
                    ),
                    DialogAction::ExitGame,
                ));
        }
        Some(TitleAction::Load) => {
            context.settings.open = false;
            context.save_load.mode = Some(crate::ui::save_load::SaveLoadMode::Load);
        }
        Some(TitleAction::Options) => {
            context.save_load.mode = None;
            context.settings.open = true;
        }
        Some(TitleAction::Extra) => {
            context.save_load.mode = None;
            context.settings.open = false;
            context.extra.open = true;
        }
        Some(TitleAction::Start) => start_game(&mut context.state),
        _ => {}
    }
}

pub fn animate_title_buttons(
    time: Res<Time>,
    mut buttons: TitleButtonAnimationQuery,
    mut previews: Query<&mut Node, (With<TitleContinuePreview>, Without<TitleButtonMotion>)>,
) {
    let amount = exp_lerp(time.delta_secs(), 12.0);
    for (interaction, mut motion, mut node, mut transform, action) in &mut buttons {
        if *interaction == Interaction::Pressed {
            motion.press = 1.0;
        } else {
            motion.press *= (-time.delta_secs() * 16.0).exp();
            if motion.press < 0.001 {
                motion.press = 0.0;
            }
        }
        let (target_width, target_padding) = match interaction {
            Interaction::None => (100.0, 22.5),
            Interaction::Hovered => (96.25, 22.5),
            Interaction::Pressed => (92.0, 20.25),
        };
        if (motion.width - target_width).abs() < 0.001
            && (motion.padding - target_padding).abs() < 0.001
            && motion.press == 0.0
        {
            continue;
        }
        motion.width += (target_width - motion.width) * amount;
        motion.padding += (target_padding - motion.padding) * amount;
        node.width = Val::Percent(motion.width);
        node.margin.left = Val::Auto;
        node.padding.left = Val::Px(motion.padding);
        transform.scale = Vec2::splat(1.0 - 0.045 * motion.press);
        if matches!(action, Some(TitleAction::Continue))
            && let Ok(mut preview) = previews.single_mut()
        {
            preview.display = if matches!(interaction, Interaction::Hovered | Interaction::Pressed)
            {
                Display::Flex
            } else {
                Display::None
            };
        }
    }
}

fn start_game(state: &mut GameState) {
    state.ended = false;
    state.backlog.clear();
    state.current_scene = crate::scene::entry_scene(state);
    state.cursor = 0;
    crabgal_core::step::step(state);
}

fn start_game_from_keyboard(
    keys: &mut ButtonInput<KeyCode>,
    actions: &mut InputActions,
    state: &mut GameState,
) -> bool {
    const START_KEYS: [KeyCode; 2] = [KeyCode::Enter, KeyCode::Space];
    if !keys.any_just_pressed(START_KEYS) {
        return false;
    }

    // `InputActions` was collected earlier in this frame. Clear both the raw
    // edge and its translated action so the key that leaves the title cannot
    // also advance the first stage presentation. The held state is preserved;
    // releasing and pressing again still creates a normal intro advance edge.
    for key in START_KEYS {
        keys.clear_just_pressed(key);
    }
    actions.advance = false;
    start_game(state);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crabgal_core::{Action, Program, State};

    #[test]
    fn keyboard_start_consumes_only_the_launch_edge_before_a_held_intro() {
        let mut state = GameState(State::new());
        state.install_program(Program::from_scenes([(
            "start".into(),
            vec![Action::Intro {
                pages: vec!["first".into(), "second".into()],
                hold: true,
            }],
        )]));
        state.ended = true;

        let mut keys = ButtonInput::default();
        keys.press(KeyCode::Enter);
        let mut actions = InputActions {
            advance: true,
            ..Default::default()
        };

        assert!(start_game_from_keyboard(
            &mut keys,
            &mut actions,
            &mut state,
        ));
        assert_eq!(state.intro.as_ref().unwrap().page, 0);
        assert!(state.intro.as_ref().unwrap().hold);
        assert!(!state.ended);
        assert!(!actions.advance);
        assert!(!keys.just_pressed(KeyCode::Enter));
        assert!(keys.pressed(KeyCode::Enter));

        actions.advance = keys.any_just_pressed([KeyCode::Space, KeyCode::Enter]);
        assert!(
            !actions.advance,
            "collecting input again must not replay the title launch edge"
        );

        keys.release(KeyCode::Enter);
        keys.clear();
        keys.press(KeyCode::Enter);
        actions.advance = keys.just_pressed(KeyCode::Enter);

        assert!(
            actions.advance,
            "a later Enter press must advance the intro"
        );
        assert_eq!(state.intro.as_ref().unwrap().page, 0);
    }
}
