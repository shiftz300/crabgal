//! Shared chrome for the fixed SAVE / LOAD / CONFIG shell.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::ui::control_bar::{BlurStrength, ButtonAction, HoverAlpha};
use crate::ui::foundation::{exp_lerp, smoothstep, text};
use crate::ui::save_load::{SaveLoadContent, SaveLoadMode, SaveLoadRoot, SaveLoadUi};
use crate::ui::settings_panel::{SettingsContent, SettingsRoot, SettingsUi};
use crate::ui::{FULLSCREEN_BLUR_STRENGTH, MENU_BACKDROP_ALPHA};

/// Below this point the blur is no longer perceptible, while opaque child UI
/// would otherwise remain visible during the tail of the exponential fade.
/// Finish both layers on the same frame instead of waiting for numerical zero.
const EXIT_VISUAL_EPSILON: f32 = 0.05;

#[derive(Component)]
pub(crate) struct MenuFade {
    pub(crate) current: f32,
    pub(crate) target: f32,
}

impl MenuFade {
    pub(crate) fn entering() -> Self {
        Self {
            current: 0.0,
            target: 1.0,
        }
    }

    pub(crate) fn visible() -> Self {
        Self {
            current: 1.0,
            target: 1.0,
        }
    }
}

#[derive(Component)]
pub(crate) struct MenuSurface {
    start_scale: f32,
    start_translation: Vec2,
}

impl MenuSurface {
    pub(crate) fn standard() -> Self {
        Self {
            start_scale: 0.99,
            start_translation: Vec2::new(0.0, 12.0),
        }
    }

    pub(crate) fn config() -> Self {
        Self {
            start_scale: 1.0,
            start_translation: Vec2::new(42.0, 0.0),
        }
    }
}

#[derive(Component)]
pub(crate) struct MenuBlur;

#[derive(Component)]
pub(crate) struct PersistentMenu;

type MenuSurfaceQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static mut MenuFade,
        &'static MenuSurface,
        &'static mut BackgroundColor,
        &'static mut UiTransform,
        &'static mut Visibility,
        Option<&'static PersistentMenu>,
    ),
    Without<MenuBlur>,
>;

type MenuBlurQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static mut MenuFade,
        &'static mut BlurStrength,
        &'static mut Visibility,
        Option<&'static PersistentMenu>,
    ),
    (With<MenuBlur>, Without<MenuSurface>),
>;

#[derive(Component)]
pub(crate) struct MenuBack;

#[derive(Component)]
pub(crate) struct MenuHeader;

#[derive(Component)]
pub(crate) struct MenuTab(ButtonAction);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MenuHeaderActive {
    Save,
    Load,
    Config,
}

pub(crate) fn active_route(
    save_load: &SaveLoadUi,
    settings: &SettingsUi,
) -> Option<MenuHeaderActive> {
    if settings.open {
        Some(MenuHeaderActive::Config)
    } else {
        save_load.mode.map(|mode| match mode {
            SaveLoadMode::Save => MenuHeaderActive::Save,
            SaveLoadMode::Load => MenuHeaderActive::Load,
        })
    }
}

#[derive(Resource, Default)]
pub(crate) struct MenuRouteTransition {
    from: Option<MenuHeaderActive>,
    to: Option<MenuHeaderActive>,
    elapsed: f32,
}

impl MenuRouteTransition {
    const SECONDS: f32 = 0.26;

    pub(crate) fn begin(&mut self, from: MenuHeaderActive, to: MenuHeaderActive) {
        if from == to {
            return;
        }
        self.from = Some(from);
        self.to = Some(to);
        self.elapsed = 0.0;
    }

    pub(crate) fn is_animating(&self) -> bool {
        self.from.is_some() && self.to.is_some()
    }

    pub(crate) fn involves(&self, route: MenuHeaderActive) -> bool {
        self.is_animating() && (self.from == Some(route) || self.to == Some(route))
    }
}

pub(crate) fn route_settled(transition: Res<MenuRouteTransition>) -> bool {
    !transition.is_animating()
}

pub(crate) fn root_node() -> Node {
    Node {
        position_type: PositionType::Absolute,
        width: Val::Percent(100.0),
        height: Val::Percent(100.0),
        padding: UiRect::axes(Val::Percent(2.5), Val::Percent(2.0)),
        flex_direction: FlexDirection::Column,
        row_gap: Val::Percent(1.0),
        ..default()
    }
}

pub(crate) fn surface_transform(surface: &MenuSurface, visible: bool) -> UiTransform {
    if visible {
        UiTransform::default()
    } else {
        UiTransform {
            translation: Val2::px(surface.start_translation.x, surface.start_translation.y),
            scale: Vec2::splat(surface.start_scale),
            ..default()
        }
    }
}

pub(crate) fn spawn_header(
    root: &mut ChildSpawnerCommands,
    active: MenuHeaderActive,
    font: &Handle<Font>,
    icon_font: &Handle<Font>,
) {
    root.spawn((
        MenuHeader,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(7.0),
            padding: UiRect::horizontal(Val::Px(12.0)),
            flex_shrink: 0.0,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            ..default()
        },
    ))
    .with_children(|header| {
        header
            .spawn((Node {
                height: Val::Percent(100.0),
                ..default()
            },))
            .with_children(|left| {
                spawn_button(
                    left,
                    "\u{f7d8}",
                    "SAVE",
                    ButtonAction::Save,
                    active == MenuHeaderActive::Save,
                    font,
                    icon_font,
                );
                spawn_button(
                    left,
                    "\u{f3d8}",
                    "LOAD",
                    ButtonAction::Load,
                    active == MenuHeaderActive::Load,
                    font,
                    icon_font,
                );
                spawn_button(
                    left,
                    "\u{f56b}",
                    "CONFIG",
                    ButtonAction::System,
                    active == MenuHeaderActive::Config,
                    font,
                    icon_font,
                );
            });
        header
            .spawn((Node {
                height: Val::Percent(100.0),
                ..default()
            },))
            .with_children(|right| {
                spawn_button(
                    right,
                    "\u{f423}",
                    "TITLE",
                    ButtonAction::Title,
                    false,
                    font,
                    icon_font,
                );
                right.spawn((
                    Button,
                    MenuBack,
                    HoverAlpha::default(),
                    Node {
                        min_width: Val::Px(150.0),
                        height: Val::Percent(100.0),
                        padding: UiRect::horizontal(Val::Px(28.0)),
                        column_gap: Val::Px(10.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::NONE),
                    children![
                        text("\u{f1c3}", icon_font, 28.0, 0.82),
                        text("BACK", font, 28.0, 0.82),
                    ],
                ));
            });
    });
}

fn spawn_button(
    parent: &mut ChildSpawnerCommands,
    icon: &str,
    label: &str,
    action: ButtonAction,
    active: bool,
    font: &Handle<Font>,
    icon_font: &Handle<Font>,
) {
    let alpha = if active { 0.18 } else { 0.0 };
    parent.spawn((
        Button,
        action,
        MenuTab(action),
        HoverAlpha {
            target: alpha,
            current: alpha,
            active,
            active_alpha: 0.18,
            hover_alpha: 0.18,
        },
        Node {
            min_width: Val::Px(165.0),
            height: Val::Percent(100.0),
            padding: UiRect::horizontal(Val::Px(28.0)),
            margin: UiRect::right(Val::Px(12.0)),
            column_gap: Val::Px(10.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(Color::srgba(1.0, 1.0, 1.0, alpha)),
        children![
            text(icon, icon_font, 28.0, 0.82),
            text(label, font, 28.0, 0.82),
        ],
    ));
}

pub(crate) fn sync_tabs(
    save_load: Res<SaveLoadUi>,
    settings: Res<SettingsUi>,
    mut tabs: Query<(&MenuTab, &mut HoverAlpha)>,
) {
    if !save_load.is_changed() && !settings.is_changed() {
        return;
    }
    for (tab, mut hover) in &mut tabs {
        let active = match tab.0 {
            ButtonAction::Save => save_load.mode == Some(SaveLoadMode::Save),
            ButtonAction::Load => save_load.mode == Some(SaveLoadMode::Load),
            ButtonAction::System => settings.open,
            _ => false,
        };
        hover.active = active;
        hover.active_alpha = 0.18;
        hover.target = if active { 0.18 } else { 0.0 };
    }
}

pub(crate) fn animate(
    time: Res<Time>,
    mut commands: Commands,
    mut surfaces: MenuSurfaceQuery,
    mut blurs: MenuBlurQuery,
) {
    let amount = exp_lerp(time.delta_secs(), 16.0);
    for (entity, mut fade, motion, mut background, mut transform, mut visibility, persistent) in
        &mut surfaces
    {
        if persistent.is_some()
            && *visibility == Visibility::Hidden
            && fade.current == 0.0
            && fade.target == 0.0
        {
            continue;
        }
        fade.current += (fade.target - fade.current) * amount;
        if fade.target == 0.0 && fade.current <= EXIT_VISUAL_EPSILON
            || (fade.target - fade.current).abs() < 0.001
        {
            fade.current = fade.target;
        }
        let eased = smoothstep(fade.current);
        background.0 = Color::srgba(0.0, 0.0, 0.0, MENU_BACKDROP_ALPHA * eased);
        transform.scale = Vec2::splat(motion.start_scale + (1.0 - motion.start_scale) * eased);
        transform.translation = Val2::px(
            motion.start_translation.x * (1.0 - eased),
            motion.start_translation.y * (1.0 - eased),
        );
        if fade.target == 0.0 && fade.current == 0.0 {
            if persistent.is_some() {
                *visibility = Visibility::Hidden;
            } else {
                commands.entity(entity).despawn();
            }
        }
    }
    for (entity, mut fade, mut strength, mut visibility, persistent) in &mut blurs {
        if persistent.is_some()
            && *visibility == Visibility::Hidden
            && fade.current == 0.0
            && fade.target == 0.0
        {
            continue;
        }
        fade.current += (fade.target - fade.current) * amount;
        if fade.target == 0.0 && fade.current <= EXIT_VISUAL_EPSILON
            || (fade.target - fade.current).abs() < 0.001
        {
            fade.current = fade.target;
        }
        strength.0 = FULLSCREEN_BLUR_STRENGTH * smoothstep(fade.current);
        if fade.target == 0.0 && fade.current == 0.0 {
            if persistent.is_some() {
                *visibility = Visibility::Hidden;
            } else {
                commands.entity(entity).despawn();
            }
        }
    }
}

type SaveRouteRootQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static mut Visibility,
        &'static mut BackgroundColor,
        &'static mut MenuFade,
    ),
    (With<SaveLoadRoot>, Without<SettingsRoot>),
>;
type SettingsRouteRootQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static mut Visibility,
        &'static mut BackgroundColor,
        &'static mut MenuFade,
    ),
    (With<SettingsRoot>, Without<SaveLoadRoot>),
>;
type SaveRouteContentQuery<'w, 's> = Query<
    'w,
    's,
    (&'static mut UiTransform, &'static ComputedNode),
    (With<SaveLoadContent>, Without<SettingsContent>),
>;
type SettingsRouteContentQuery<'w, 's> = Query<
    'w,
    's,
    (&'static mut UiTransform, &'static ComputedNode),
    (With<SettingsContent>, Without<SaveLoadContent>),
>;
type MenuHeaderQuery<'w, 's> = Query<
    'w,
    's,
    (&'static ChildOf, &'static mut Visibility),
    (
        With<MenuHeader>,
        Without<SaveLoadRoot>,
        Without<SettingsRoot>,
    ),
>;

#[derive(SystemParam)]
pub(crate) struct MenuRouteContext<'w, 's> {
    save_roots: SaveRouteRootQuery<'w, 's>,
    settings_roots: SettingsRouteRootQuery<'w, 's>,
    save_contents: SaveRouteContentQuery<'w, 's>,
    settings_contents: SettingsRouteContentQuery<'w, 's>,
    headers: MenuHeaderQuery<'w, 's>,
    windows: Query<'w, 's, &'static Window>,
}

pub(crate) fn animate_route_transition(
    time: Res<Time>,
    mut transition: ResMut<MenuRouteTransition>,
    mut context: MenuRouteContext,
) {
    let (Some(from), Some(to)) = (transition.from, transition.to) else {
        return;
    };
    let (
        Ok((save_root, mut save_visibility, mut save_background, mut save_fade)),
        Ok((settings_root, mut settings_visibility, mut settings_background, mut settings_fade)),
    ) = (
        context.save_roots.single_mut(),
        context.settings_roots.single_mut(),
    )
    else {
        return;
    };

    transition.elapsed = (transition.elapsed + time.delta_secs()).min(MenuRouteTransition::SECONDS);
    let progress = smoothstep(transition.elapsed / MenuRouteTransition::SECONDS);
    let route_index = |route| -> f32 {
        match route {
            MenuHeaderActive::Save => 0.0,
            MenuHeaderActive::Load => 1.0,
            MenuHeaderActive::Config => 2.0,
        }
    };
    let direction = (route_index(to) - route_index(from)).signum();
    let width = context
        .save_contents
        .iter()
        .map(|(_, node)| node.size().x)
        .chain(
            context
                .settings_contents
                .iter()
                .map(|(_, node)| node.size().x),
        )
        .fold(0.0_f32, f32::max)
        .max(
            context
                .windows
                .single()
                .map_or(1.0, |window| window.width() * 0.95),
        );
    let incoming_x = direction * width * (1.0 - progress);
    let outgoing_x = -direction * width * progress;
    let save_is_incoming = matches!(to, MenuHeaderActive::Save | MenuHeaderActive::Load);

    *save_visibility = Visibility::Inherited;
    *settings_visibility = Visibility::Inherited;
    save_fade.current = 1.0;
    save_fade.target = 1.0;
    settings_fade.current = 1.0;
    settings_fade.target = 1.0;
    save_background.0 = if save_is_incoming {
        Color::srgba(0.0, 0.0, 0.0, MENU_BACKDROP_ALPHA)
    } else {
        Color::NONE
    };
    settings_background.0 = if save_is_incoming {
        Color::NONE
    } else {
        Color::srgba(0.0, 0.0, 0.0, MENU_BACKDROP_ALPHA)
    };

    for (mut transform, _) in &mut context.save_contents {
        transform.translation = Val2::px(
            if save_is_incoming {
                incoming_x
            } else {
                outgoing_x
            },
            0.0,
        );
    }
    for (mut transform, _) in &mut context.settings_contents {
        transform.translation = Val2::px(
            if save_is_incoming {
                outgoing_x
            } else {
                incoming_x
            },
            0.0,
        );
    }
    let incoming_root = if save_is_incoming {
        save_root
    } else {
        settings_root
    };
    for (parent, mut visibility) in &mut context.headers {
        *visibility = if parent.parent() == incoming_root {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }

    if transition.elapsed < MenuRouteTransition::SECONDS {
        return;
    }
    if save_is_incoming {
        *settings_visibility = Visibility::Hidden;
        settings_fade.current = 0.0;
        settings_fade.target = 0.0;
    } else {
        *save_visibility = Visibility::Hidden;
        save_fade.current = 0.0;
        save_fade.target = 0.0;
    }
    for (mut transform, _) in &mut context.save_contents {
        transform.translation = Val2::ZERO;
    }
    for (mut transform, _) in &mut context.settings_contents {
        transform.translation = Val2::ZERO;
    }
    for (_, mut visibility) in &mut context.headers {
        *visibility = Visibility::Inherited;
    }
    transition.from = None;
    transition.to = None;
    transition.elapsed = 0.0;
}
