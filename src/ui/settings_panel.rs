use std::collections::HashMap;

use bevy::camera::visibility::RenderLayers;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::text::FontWeight;
use bevy::ui::FocusPolicy;
use bevy::window::{MonitorSelection, WindowMode};

use crate::render::blur::{DialogCamera, UiBlurCamera};
use crate::runtime::resources::ProjectRoot;
use crate::storage::settings::RuntimeSettings;
use crate::ui::control_bar::{BlurStrength, ButtonAction, SkipMode, ToggleStates, UiBlurSource};
use crate::ui::foundation::{UiFonts, exp_lerp, fill_node, smoothstep, text_weight};
use crate::ui::menu::{
    MenuBack, MenuBlur, MenuFade, MenuHeaderActive, MenuRouteTransition, MenuSurface,
    PersistentMenu, active_route, root_node, spawn_header, surface_transform,
};
use crate::ui::save_load::SaveLoadUi;

const OPTION_TRANSITION_RATE: f32 = 18.0;
const OPTION_TEXT_IDLE: f32 = 0.376;
const OPTION_TEXT_ACTIVE: f32 = 0.667;
const OPTION_FILL_ALPHA: f32 = 0.188;
const PAGE_TEXT_IDLE: f32 = 0.175;
const PAGE_TEXT_HOVER: f32 = 0.5;
const PAGE_TEXT_ACTIVE: f32 = 0.8;

#[derive(Resource)]
pub(crate) struct SettingsUi {
    pub(crate) open: bool,
    pub(crate) page: SettingsPage,
}

#[derive(Resource, Default)]
pub(crate) struct PendingWindowMode {
    target: Option<bool>,
    delay_frames: u8,
}

impl PendingWindowMode {
    pub(crate) fn is_pending(&self) -> bool {
        self.target.is_some()
    }
}

#[derive(Resource, Default)]
pub(crate) struct ActiveSettingSlider {
    kind: Option<SettingKind>,
    dirty: bool,
}

impl ActiveSettingSlider {
    pub(crate) fn is_active(&self) -> bool {
        self.kind.is_some()
    }
}

impl Default for SettingsUi {
    fn default() -> Self {
        Self {
            open: false,
            page: SettingsPage::System,
        }
    }
}

#[derive(Component)]
pub(crate) struct SettingsRoot;

#[derive(Component)]
pub(crate) struct SettingsContent;

#[derive(Component)]
pub(crate) struct SettingsBlurProxy;

#[derive(Component)]
pub(crate) struct SettingsWatermark {
    current: f32,
    target: f32,
}

impl SettingsWatermark {
    fn entering() -> Self {
        Self {
            current: 0.0,
            target: 1.0,
        }
    }

    pub(crate) fn show(&mut self) {
        self.target = 1.0;
    }

    pub(crate) fn hide(&mut self) {
        self.target = 0.0;
    }

    pub(crate) fn is_animating(&self) -> bool {
        (self.current - self.target).abs() > 0.001
    }
}

pub(crate) fn spawn_menu_watermark(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    font: &Handle<Font>,
) {
    parent.spawn((
        Name::new("menu_watermark"),
        SettingsWatermark::entering(),
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(52.0),
            bottom: Val::Px(24.0),
            ..default()
        },
        FocusPolicy::Pass,
        crate::ui::text_style::NoTextShadow,
        Text::new(label),
        TextFont {
            font: font.clone().into(),
            font_size: FontSize::from(320.0),
            weight: FontWeight::BOLD,
            ..default()
        },
        TextColor(Color::srgba(1.0, 1.0, 1.0, 0.0)),
    ));
}

#[derive(Default)]
pub(crate) struct SettingsVisualFadeCache {
    text_alpha: HashMap<Entity, f32>,
    background_alpha: HashMap<Entity, f32>,
    outline_alpha: HashMap<Entity, f32>,
    settled: bool,
}

type SettingsRootVisibilityQuery<'w, 's> = Query<
    'w,
    's,
    (Entity, &'static mut Visibility),
    (
        With<SettingsRoot>,
        Without<SettingsBlurProxy>,
        Without<crate::ui::save_load::SaveLoadRoot>,
    ),
>;
type SettingsProxyVisibilityQuery<'w, 's> = Query<
    'w,
    's,
    (Entity, &'static mut Visibility),
    (
        With<SettingsBlurProxy>,
        Without<SettingsRoot>,
        Without<crate::ui::save_load::SaveLoadRoot>,
    ),
>;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum SettingsPage {
    #[default]
    System,
    Display,
    Audio,
}

#[derive(Component)]
pub(crate) struct SettingsPageButton(pub(crate) SettingsPage);

#[derive(Component)]
pub(crate) struct SettingsPageLabel;

#[derive(Component)]
pub(crate) struct SettingsPageButtonVisual(f32);

impl SettingsPageButtonVisual {
    pub(crate) fn is_animating(
        &self,
        interaction: Interaction,
        page: SettingsPage,
        active: SettingsPage,
    ) -> bool {
        let target = if page == active {
            PAGE_TEXT_ACTIVE
        } else if matches!(interaction, Interaction::Hovered | Interaction::Pressed) {
            PAGE_TEXT_HOVER
        } else {
            PAGE_TEXT_IDLE
        };
        (self.0 - target).abs() > 0.001
    }
}

#[derive(Component)]
pub(crate) struct SettingsPagePanel {
    page: SettingsPage,
    progress: f32,
}

impl SettingsPagePanel {
    pub(crate) fn is_animating(&self, active: SettingsPage) -> bool {
        self.page == active && self.progress < 1.0
    }
}

#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SettingAction {
    SetSkip(bool),
    SetFullscreen(bool),
    SetTextSize(u8),
    Noop,
}

#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SettingKind {
    MasterVolume,
    VocalVolume,
    BgmVolume,
    SeVolume,
    UiSeVolume,
    TextSpeed,
    AutoDelay,
    TextboxOpacity,
}

impl SettingKind {
    fn ratio(self, settings: &RuntimeSettings) -> f32 {
        match self {
            Self::MasterVolume => settings.master_volume,
            Self::VocalVolume => settings.vocal_volume,
            Self::BgmVolume => settings.bgm_volume,
            Self::SeVolume => settings.se_volume,
            Self::UiSeVolume => settings.ui_se_volume,
            Self::TextSpeed => ((settings.typewriter_speed - 10.0) / 110.0) as f32,
            Self::AutoDelay => ((settings.auto_delay - 0.5) / 4.5) as f32,
            Self::TextboxOpacity => settings.textbox_opacity,
        }
    }

    fn set_ratio(self, settings: &mut RuntimeSettings, ratio: f32) {
        match self {
            Self::MasterVolume => settings.master_volume = ratio,
            Self::VocalVolume => settings.vocal_volume = ratio,
            Self::BgmVolume => settings.bgm_volume = ratio,
            Self::SeVolume => settings.se_volume = ratio,
            Self::UiSeVolume => settings.ui_se_volume = ratio,
            Self::TextSpeed => settings.typewriter_speed = 10.0 + f64::from(ratio) * 110.0,
            Self::AutoDelay => settings.auto_delay = 0.5 + f64::from(ratio) * 4.5,
            Self::TextboxOpacity => settings.textbox_opacity = ratio,
        }
    }

    fn value_text(self, ratio: f32) -> String {
        match self {
            Self::TextSpeed => format!("{:.0}", 10.0 + ratio * 110.0),
            Self::AutoDelay => format!("{:.1}", 0.5 + ratio * 4.5),
            _ => format!("{}", (ratio * 100.0).round()),
        }
    }
}

#[derive(Clone, Copy)]
struct SliderSpec {
    label: &'static str,
    kind: SettingKind,
}

const SYSTEM_SLIDERS: &[SliderSpec] = &[SliderSpec {
    label: "AUTO PLAY SPEED",
    kind: SettingKind::AutoDelay,
}];

const DISPLAY_SLIDERS: &[SliderSpec] = &[
    SliderSpec {
        label: "TEXT SPEED",
        kind: SettingKind::TextSpeed,
    },
    SliderSpec {
        label: "TEXTBOX OPACITY",
        kind: SettingKind::TextboxOpacity,
    },
];

const AUDIO_SLIDERS: &[SliderSpec] = &[
    SliderSpec {
        label: "MASTER VOLUME",
        kind: SettingKind::MasterVolume,
    },
    SliderSpec {
        label: "VOICE VOLUME",
        kind: SettingKind::VocalVolume,
    },
    SliderSpec {
        label: "BGM VOLUME",
        kind: SettingKind::BgmVolume,
    },
    SliderSpec {
        label: "SOUND EFFECT VOLUME",
        kind: SettingKind::SeVolume,
    },
    SliderSpec {
        label: "UI SOUND VOLUME",
        kind: SettingKind::UiSeVolume,
    },
];

#[derive(Component)]
pub(crate) struct SettingSlider(pub(crate) SettingKind);

#[derive(Component)]
pub(crate) struct SettingSliderThumb(pub(crate) SettingKind);

#[derive(Component)]
pub(crate) struct SettingSliderThumbVisual(pub(crate) f32);

#[derive(Component)]
pub(crate) struct SettingValueText(pub(crate) SettingKind);

#[derive(Component)]
pub(crate) struct SettingValueBubble(pub(crate) SettingKind);

#[derive(Component)]
pub(crate) struct SettingChoice(pub(crate) SettingAction);

#[derive(Component)]
pub(crate) struct SettingChoiceVisual {
    selected: bool,
    hovered: bool,
    fill: f32,
    text_alpha: f32,
}

impl SettingChoiceVisual {
    pub(crate) fn is_animating(
        &self,
        interaction: Interaction,
        action: SettingAction,
        settings: &RuntimeSettings,
    ) -> bool {
        let selected = choice_is_selected(settings, action);
        let hovered = matches!(interaction, Interaction::Hovered | Interaction::Pressed);
        let target_fill = if selected || hovered { 100.0 } else { 0.0 };
        let target_text = if selected || hovered {
            OPTION_TEXT_ACTIVE
        } else {
            OPTION_TEXT_IDLE
        };
        (self.fill - target_fill).abs() > 0.001 || (self.text_alpha - target_text).abs() > 0.001
    }
}

#[derive(Component)]
pub(crate) struct SettingChoiceFill;

type SettingChoiceFillQuery<'w, 's> = Query<
    'w,
    's,
    (&'static mut Node, &'static mut BackgroundColor),
    (With<SettingChoiceFill>, Without<SettingSliderThumb>),
>;

#[derive(Component)]
pub(crate) struct SettingPreviewSurface;

#[derive(Component)]
pub(crate) struct SettingPreviewText;

#[derive(SystemParam)]
pub(crate) struct SettingsSyncContext<'w, 's> {
    commands: Commands<'w, 's>,
    roots: SettingsRootVisibilityQuery<'w, 's>,
    proxies: SettingsProxyVisibilityQuery<'w, 's>,
    camera: Query<'w, 's, Entity, With<DialogCamera>>,
    blur_camera: Query<'w, 's, Entity, With<UiBlurCamera>>,
    fonts: Res<'w, UiFonts>,
    fades: Query<'w, 's, &'static mut MenuFade>,
    watermarks: Query<'w, 's, (&'static mut SettingsWatermark, &'static mut Text)>,
    save_roots:
        Query<'w, 's, (Entity, &'static mut Visibility), With<crate::ui::save_load::SaveLoadRoot>>,
    save_proxies: Query<'w, 's, Entity, With<crate::ui::save_load::SaveLoadBlurProxy>>,
    panels: Query<'w, 's, &'static mut SettingsPagePanel>,
    route_transition: Res<'w, MenuRouteTransition>,
}

pub fn toggle_settings(
    keys: Res<ButtonInput<KeyCode>>,
    controls: Query<(&Interaction, &ButtonAction), Changed<Interaction>>,
    back: Query<&Interaction, (With<MenuBack>, Changed<Interaction>)>,
    mut ui: ResMut<SettingsUi>,
    mut save_load: ResMut<SaveLoadUi>,
    mut route_transition: ResMut<MenuRouteTransition>,
) {
    let previous_route = active_route(&save_load, &ui);
    if controls.iter().any(|(interaction, action)| {
        *interaction == Interaction::Pressed && *action == ButtonAction::System
    }) {
        ui.open = !ui.open;
        if ui.open {
            save_load.mode = None;
        }
    }
    let next_route = active_route(&save_load, &ui);
    if let (Some(from), Some(to)) = (previous_route, next_route)
        && (from == MenuHeaderActive::Config || to == MenuHeaderActive::Config)
    {
        route_transition.begin(from, to);
    }
    if ui.open
        && (keys.just_pressed(KeyCode::Escape)
            || back
                .iter()
                .any(|interaction| *interaction == Interaction::Pressed))
    {
        ui.open = false;
    }
}

pub fn settings_open(ui: Res<SettingsUi>) -> bool {
    ui.open
}

pub fn handle_settings_page(
    buttons: Query<(&Interaction, &SettingsPageButton), Changed<Interaction>>,
    mut ui: ResMut<SettingsUi>,
) {
    for (interaction, page) in &buttons {
        if *interaction == Interaction::Pressed && ui.page != page.0 {
            ui.page = page.0;
        }
    }
}

pub fn sync_settings(
    ui: Res<SettingsUi>,
    settings: Res<RuntimeSettings>,
    save_load: Res<SaveLoadUi>,
    mut context: SettingsSyncContext,
) {
    if !ui.is_changed() {
        return;
    }
    for (entity, _) in &mut context.roots {
        if !ui.open
            && !context.route_transition.involves(MenuHeaderActive::Config)
            && let Ok(mut fade) = context.fades.get_mut(entity)
        {
            fade.target = 0.0;
        }
    }
    if !ui.open {
        if save_load.mode.is_none() && !context.route_transition.involves(MenuHeaderActive::Config)
        {
            for (mut watermark, _) in &mut context.watermarks {
                watermark.hide();
            }
        }
        for (entity, _) in &mut context.proxies {
            if let Ok(mut fade) = context.fades.get_mut(entity) {
                fade.target = f32::from(save_load.mode.is_some());
            }
        }
        return;
    }
    if !context.roots.is_empty() {
        let switching = context.route_transition.is_animating();
        if switching {
            for mut panel in &mut context.panels {
                panel.progress = if panel.page == ui.page { 0.0 } else { 1.0 };
            }
        }
        if !switching {
            for (_, mut visibility) in &mut context.save_roots {
                *visibility = Visibility::Hidden;
            }
        }
        for (mut watermark, mut label) in &mut context.watermarks {
            watermark.show();
            **label = "CONFIG".into();
        }
        for (entity, mut visibility) in &mut context.roots {
            *visibility = Visibility::Inherited;
            if let Ok(mut fade) = context.fades.get_mut(entity) {
                if switching {
                    fade.current = 1.0;
                }
                fade.target = 1.0;
            }
        }
        for (entity, mut visibility) in &mut context.proxies {
            *visibility = Visibility::Inherited;
            if let Ok(mut fade) = context.fades.get_mut(entity) {
                fade.target = 1.0;
            }
        }
        return;
    }
    let (Ok(camera), Ok(blur_camera)) = (context.camera.single(), context.blur_camera.single())
    else {
        return;
    };
    let switching = context.route_transition.is_animating();
    if !switching {
        for (_, mut visibility) in &mut context.save_roots {
            *visibility = Visibility::Hidden;
        }
    }
    let font = context.fonts.text.clone();
    let icon_font = context.fonts.icons.clone();
    if context.proxies.is_empty() {
        context
            .commands
            .spawn((
                Name::new("settings_blur"),
                SettingsBlurProxy,
                crate::ui::save_load::SaveLoadBlurProxy,
                MenuBlur,
                PersistentMenu,
                if switching {
                    MenuFade::visible()
                } else {
                    MenuFade::entering()
                },
                UiBlurSource,
                BlurStrength(if switching {
                    crate::ui::FULLSCREEN_BLUR_STRENGTH
                } else {
                    0.0
                }),
                fill_node(),
                GlobalZIndex(179),
                UiTargetCamera(blur_camera),
                RenderLayers::layer(1),
            ))
            .with_children(|proxy| spawn_menu_watermark(proxy, "CONFIG", &font));
    } else {
        for (entity, mut visibility) in &mut context.proxies {
            *visibility = Visibility::Inherited;
            if let Ok(mut fade) = context.fades.get_mut(entity) {
                fade.target = 1.0;
            }
        }
        if context.watermarks.is_empty()
            && let Some(proxy) = context.save_proxies.iter().next()
        {
            context
                .commands
                .entity(proxy)
                .with_children(|proxy| spawn_menu_watermark(proxy, "CONFIG", &font));
        }
    }
    context
        .commands
        .spawn((
            Name::new("system_settings"),
            SettingsRoot,
            MenuSurface::config(),
            PersistentMenu,
            if switching {
                MenuFade::visible()
            } else {
                MenuFade::entering()
            },
            surface_transform(&MenuSurface::config(), switching),
            root_node(),
            BackgroundColor(Color::NONE),
            FocusPolicy::Block,
            GlobalZIndex(180),
            UiTargetCamera(camera),
            RenderLayers::layer(2),
        ))
        .with_children(|root| {
            spawn_header(root, MenuHeaderActive::Config, &font, &icon_font);
            spawn_options_content(root, &ui, &settings, &font);
        });
}

fn spawn_options_content(
    root: &mut ChildSpawnerCommands,
    ui: &SettingsUi,
    settings: &RuntimeSettings,
    font: &Handle<Font>,
) {
    root.spawn((
        SettingsContent,
        UiTransform::default(),
        Node {
            position_type: PositionType::Relative,
            width: Val::Percent(100.0),
            flex_grow: 1.0,
            flex_direction: FlexDirection::Column,
            overflow: Overflow::clip(),
            ..default()
        },
    ))
    .with_children(|options| {
        options
            .spawn((Node {
                width: Val::Percent(100.0),
                flex_grow: 1.0,
                padding: UiRect::axes(Val::Px(36.0), Val::Px(18.0)),
                ..default()
            },))
            .with_children(|body| {
                body.spawn((Node {
                    width: Val::Percent(18.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(12.0),
                    ..default()
                },))
                    .with_children(|pages| {
                        spawn_page_button(pages, "SYSTEM", SettingsPage::System, ui, font);
                        spawn_page_button(pages, "DISPLAY", SettingsPage::Display, ui, font);
                        spawn_page_button(pages, "AUDIO", SettingsPage::Audio, ui, font);
                    });
                body.spawn((Node {
                    position_type: PositionType::Relative,
                    flex_grow: 1.0,
                    height: Val::Percent(100.0),
                    padding: UiRect::left(Val::Px(48.0)),
                    ..default()
                },))
                    .with_children(|content| {
                        for page in [
                            SettingsPage::System,
                            SettingsPage::Display,
                            SettingsPage::Audio,
                        ] {
                            spawn_settings_page(content, page, ui, settings, font);
                        }
                    });
            });
    });
}

pub fn animate_watermark(
    time: Res<Time>,
    mut watermarks: Query<(&mut SettingsWatermark, &mut TextColor)>,
) {
    let amount = exp_lerp(time.delta_secs(), 16.0);
    for (mut watermark, mut color) in &mut watermarks {
        if (watermark.current - watermark.target).abs() < 0.001 {
            continue;
        }
        watermark.current += (watermark.target - watermark.current) * amount;
        if (watermark.target - watermark.current).abs() < 0.001 {
            watermark.current = watermark.target;
        }
        color.0 = Color::srgba(1.0, 1.0, 1.0, 0.075 * watermark.current);
    }
}

#[derive(SystemParam)]
pub(crate) struct SettingsVisualFadeContext<'w, 's> {
    roots: Query<'w, 's, (Entity, &'static MenuFade, &'static Visibility), With<SettingsRoot>>,
    parents: Query<'w, 's, &'static ChildOf>,
    texts: Query<'w, 's, (Entity, &'static mut TextColor)>,
    backgrounds: Query<'w, 's, (Entity, &'static mut BackgroundColor)>,
    outlines: Query<'w, 's, (Entity, &'static mut Outline)>,
}

pub fn fade_settings_visuals(
    ui: Res<SettingsUi>,
    route_transition: Res<MenuRouteTransition>,
    mut context: SettingsVisualFadeContext,
    mut cache: Local<SettingsVisualFadeCache>,
) {
    let Ok((root, fade, visibility)) = context.roots.single() else {
        return;
    };
    let belongs_to_settings = |entity: Entity| {
        let mut current = entity;
        while let Ok(parent) = context.parents.get(current) {
            current = parent.parent();
            if current == root {
                return true;
            }
        }
        false
    };

    if *visibility == Visibility::Hidden {
        return;
    }
    let fading =
        (!ui.open && !route_transition.involves(MenuHeaderActive::Config)) || fade.current < 0.999;
    let alpha = smoothstep(fade.current);

    if fading {
        cache.settled = false;
        for (entity, mut color) in &mut context.texts {
            if belongs_to_settings(entity) {
                let base = *cache
                    .text_alpha
                    .entry(entity)
                    .or_insert_with(|| color.0.alpha());
                color.0 = color.0.with_alpha(base * alpha);
            }
        }
        for (entity, mut background) in &mut context.backgrounds {
            if belongs_to_settings(entity) {
                let base = *cache
                    .background_alpha
                    .entry(entity)
                    .or_insert_with(|| background.0.alpha());
                background.0 = background.0.with_alpha(base * alpha);
            }
        }
        for (entity, mut outline) in &mut context.outlines {
            if belongs_to_settings(entity) {
                let base = *cache
                    .outline_alpha
                    .entry(entity)
                    .or_insert_with(|| outline.color.alpha());
                outline.color = outline.color.with_alpha(base * alpha);
            }
        }
        return;
    }

    for (entity, mut color) in &mut context.texts {
        if !belongs_to_settings(entity) {
            continue;
        }
        if cache.settled {
            cache.text_alpha.insert(entity, color.0.alpha());
        } else if let Some(base) = cache.text_alpha.get(&entity) {
            color.0 = color.0.with_alpha(*base);
        }
    }
    for (entity, mut background) in &mut context.backgrounds {
        if !belongs_to_settings(entity) {
            continue;
        }
        if cache.settled {
            cache.background_alpha.insert(entity, background.0.alpha());
        } else if let Some(base) = cache.background_alpha.get(&entity) {
            background.0 = background.0.with_alpha(*base);
        }
    }
    for (entity, mut outline) in &mut context.outlines {
        if !belongs_to_settings(entity) {
            continue;
        }
        if cache.settled {
            cache.outline_alpha.insert(entity, outline.color.alpha());
        } else if let Some(base) = cache.outline_alpha.get(&entity) {
            outline.color = outline.color.with_alpha(*base);
        }
    }
    cache.settled = true;
}

fn spawn_page_button(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    page: SettingsPage,
    ui: &SettingsUi,
    font: &Handle<Font>,
) {
    let active = ui.page == page;
    parent.spawn((
        Button,
        SettingsPageButton(page),
        SettingsPageButtonVisual(if active {
            PAGE_TEXT_ACTIVE
        } else {
            PAGE_TEXT_IDLE
        }),
        Node {
            height: Val::Px(100.0),
            padding: UiRect::axes(Val::Px(8.0), Val::Px(6.0)),
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(Color::NONE),
        children![(
            SettingsPageLabel,
            Text::new(label),
            TextFont {
                font: font.clone().into(),
                font_size: FontSize::from(48.0),
                weight: FontWeight::BOLD,
                ..default()
            },
            TextColor(Color::srgba(
                1.0,
                1.0,
                1.0,
                if active {
                    PAGE_TEXT_ACTIVE
                } else {
                    PAGE_TEXT_IDLE
                },
            )),
        )],
    ));
}

fn spawn_settings_page(
    parent: &mut ChildSpawnerCommands,
    page: SettingsPage,
    ui: &SettingsUi,
    settings: &RuntimeSettings,
    font: &Handle<Font>,
) {
    let active = ui.page == page;
    parent
        .spawn((
            SettingsPagePanel {
                page,
                progress: 0.0,
            },
            UiTransform::default(),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                padding: UiRect::axes(Val::Px(28.0), Val::Px(12.0)),
                flex_direction: FlexDirection::Row,
                flex_wrap: FlexWrap::Wrap,
                align_items: AlignItems::FlexStart,
                align_content: AlignContent::FlexStart,
                overflow: Overflow::scroll_y(),
                display: if active { Display::Flex } else { Display::None },
                ..default()
            },
        ))
        .with_children(|content| match page {
            SettingsPage::System => {
                spawn_sliders(content, SYSTEM_SLIDERS, settings, font);
                spawn_skip_row(content, settings, font);
                spawn_static_row(content, "LANGUAGE", &["简体中文"], font, 30.0);
                spawn_static_row(
                    content,
                    "CLEAR OR RESTORE DATA",
                    &["CLEAR SAVES", "RESET SETTINGS", "CLEAR ALL"],
                    font,
                    54.0,
                );
                spawn_static_row(
                    content,
                    "IMPORT OR EXPORT SAVES AND OPTIONS",
                    &["EXPORT", "IMPORT"],
                    font,
                    38.0,
                );
                spawn_about(content, font);
            }
            SettingsPage::Display => {
                spawn_fullscreen_row(content, settings, font);
                spawn_text_size_row(content, settings, font);
                spawn_sliders(content, DISPLAY_SLIDERS, settings, font);
                spawn_text_preview(content, settings, font);
            }
            SettingsPage::Audio => spawn_sliders(content, AUDIO_SLIDERS, settings, font),
        });
}

fn spawn_sliders(
    content: &mut ChildSpawnerCommands,
    specs: &[SliderSpec],
    settings: &RuntimeSettings,
    font: &Handle<Font>,
) {
    for spec in specs {
        spawn_row(
            content,
            font,
            spec.label,
            spec.kind,
            spec.kind.ratio(settings),
            30.0,
        );
    }
}

fn spawn_skip_row(
    content: &mut ChildSpawnerCommands,
    settings: &RuntimeSettings,
    font: &Handle<Font>,
) {
    spawn_choice_row(
        content,
        "SKIP MODE",
        &[
            ("READ", SettingAction::SetSkip(false)),
            ("ALL", SettingAction::SetSkip(true)),
        ],
        usize::from(settings.skip_all),
        font,
        30.0,
    );
}

fn spawn_fullscreen_row(
    content: &mut ChildSpawnerCommands,
    settings: &RuntimeSettings,
    font: &Handle<Font>,
) {
    spawn_choice_row(
        content,
        "FULLSCREEN",
        &[
            ("ON", SettingAction::SetFullscreen(true)),
            ("OFF", SettingAction::SetFullscreen(false)),
        ],
        if settings.fullscreen { 0 } else { 1 },
        font,
        30.0,
    );
}

fn spawn_text_size_row(
    content: &mut ChildSpawnerCommands,
    settings: &RuntimeSettings,
    font: &Handle<Font>,
) {
    spawn_choice_row(
        content,
        "TEXT SIZE",
        &[
            ("SMALL", SettingAction::SetTextSize(0)),
            ("MEDIUM", SettingAction::SetTextSize(1)),
            ("LARGE", SettingAction::SetTextSize(2)),
        ],
        usize::from(settings.text_size),
        font,
        30.0,
    );
}

fn spawn_choice_row(
    content: &mut ChildSpawnerCommands,
    label: &str,
    choices: &[(&str, SettingAction)],
    selected: usize,
    font: &Handle<Font>,
    width: f32,
) {
    content
        .spawn((Node {
            width: Val::Percent(width),
            min_height: Val::Px(112.0),
            margin: UiRect::axes(Val::Px(16.0), Val::Px(6.0)),
            padding: UiRect::all(Val::Px(6.0)),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::FlexStart,
            ..default()
        },))
        .with_children(|row| {
            row.spawn(setting_text(label, font, 32.0, true));
            row.spawn((Node {
                margin: UiRect::top(Val::Px(10.0)),
                column_gap: Val::Px(8.0),
                ..default()
            },))
                .with_children(|buttons| {
                    for (index, (text, action)) in choices.iter().copied().enumerate() {
                        spawn_choice(buttons, text, action, index == selected, font);
                    }
                });
        });
}

fn spawn_static_row(
    content: &mut ChildSpawnerCommands,
    label: &str,
    choices: &[&str],
    font: &Handle<Font>,
    width: f32,
) {
    let choices: Vec<_> = choices
        .iter()
        .map(|text| (*text, SettingAction::Noop))
        .collect();
    spawn_choice_row(content, label, &choices, usize::MAX, font, width);
}

fn spawn_about(content: &mut ChildSpawnerCommands, font: &Handle<Font>) {
    content.spawn((
        Node {
            width: Val::Percent(100.0),
            margin: UiRect::axes(Val::Px(22.0), Val::Px(6.0)),
            padding: UiRect::all(Val::Px(6.0)),
            ..default()
        },
        children![setting_text("ABOUT CRABGAL", font, 25.0, true)],
    ));
}

fn spawn_text_preview(
    content: &mut ChildSpawnerCommands,
    settings: &RuntimeSettings,
    font: &Handle<Font>,
) {
    let size = match settings.text_size {
        0 => 31.0,
        2 => 42.0,
        _ => 36.0,
    };
    content
        .spawn((Node {
            width: Val::Percent(94.0),
            min_height: Val::Px(330.0),
            margin: UiRect::axes(Val::Px(16.0), Val::Px(8.0)),
            padding: UiRect::all(Val::Px(6.0)),
            flex_direction: FlexDirection::Column,
            ..default()
        },))
        .with_children(|preview| {
            preview.spawn(setting_text("TEXT PREVIEW", font, 32.0, true));
            preview
                .spawn((
                    Node {
                        width: Val::Percent(100.0),
                        flex_grow: 1.0,
                        margin: UiRect::top(Val::Px(12.0)),
                        padding: UiRect::axes(Val::Px(36.0), Val::Px(28.0)),
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(10.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(
                        0.0,
                        0.0,
                        0.03,
                        settings.textbox_opacity * 0.72,
                    )),
                    SettingPreviewSurface,
                ))
                .with_children(|box_| {
                    box_.spawn((
                        Node {
                            width: Val::Px(300.0),
                            height: Val::Px(58.0),
                            margin: UiRect::new(
                                Val::Px(-18.0),
                                Val::ZERO,
                                Val::Px(-12.0),
                                Val::ZERO,
                            ),
                            padding: UiRect::horizontal(Val::Px(22.0)),
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.0, 0.0, 0.02, 0.7)),
                    ))
                    .with_child(setting_text(
                        "TEXT PREVIEW",
                        font,
                        34.0,
                        true,
                    ));
                    box_.spawn((
                        SettingPreviewText,
                        setting_text(
                            "Preview the dialogue size, speed and textbox opacity here.",
                            font,
                            size,
                            false,
                        ),
                    ));
                });
        });
}

fn spawn_row(
    root: &mut ChildSpawnerCommands,
    font: &Handle<Font>,
    label: &str,
    kind: SettingKind,
    ratio: f32,
    width: f32,
) {
    root.spawn((Node {
        width: Val::Percent(width),
        min_height: Val::Px(128.0),
        margin: UiRect::axes(Val::Px(16.0), Val::Px(6.0)),
        padding: UiRect::all(Val::Px(6.0)),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::FlexStart,
        ..default()
    },))
        .with_children(|row| {
            row.spawn(setting_text(label, font, 32.0, true));
            row.spawn((
                Button,
                SettingSlider(kind),
                Node {
                    position_type: PositionType::Relative,
                    width: Val::Px(500.0),
                    height: Val::Px(50.0),
                    margin: UiRect::top(Val::Px(10.0)),
                    ..default()
                },
                BackgroundColor(Color::NONE),
            ))
            .with_children(|slider| {
                slider.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(0.0),
                        top: Val::Px(20.0),
                        width: Val::Percent(100.0),
                        height: Val::Px(10.0),
                        ..default()
                    },
                    BackgroundColor(Color::BLACK),
                    Outline::new(Val::Px(5.0), Val::ZERO, Color::srgba(1.0, 1.0, 1.0, 0.19)),
                ));
                slider.spawn((
                    SettingSliderThumb(kind),
                    SettingSliderThumbVisual(10.0),
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Percent(ratio.clamp(0.0, 1.0) * 90.0),
                        top: Val::Px(5.0),
                        width: Val::Percent(10.0),
                        height: Val::Px(40.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.5)),
                ));
                slider
                    .spawn((
                        SettingValueBubble(kind),
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Percent(ratio.clamp(0.0, 1.0) * 90.0),
                            top: Val::Px(-42.0),
                            width: Val::Percent(10.0),
                            height: Val::Px(36.0),
                            display: Display::None,
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border_radius: BorderRadius::all(Val::Px(6.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.85)),
                    ))
                    .with_child((
                        SettingValueText(kind),
                        Text::new(kind.value_text(ratio)),
                        TextFont {
                            font: font.clone().into(),
                            font_size: FontSize::from(22.0),
                            weight: FontWeight::BOLD,
                            ..default()
                        },
                        TextColor(Color::WHITE),
                    ));
            });
        });
}

fn spawn_choice(
    parent: &mut ChildSpawnerCommands,
    text: &str,
    action: SettingAction,
    selected: bool,
    font: &Handle<Font>,
) {
    parent.spawn((
        Button,
        action,
        SettingChoice(action),
        SettingChoiceVisual {
            selected,
            hovered: false,
            fill: if selected { 100.0 } else { 0.0 },
            text_alpha: if selected {
                OPTION_TEXT_ACTIVE
            } else {
                OPTION_TEXT_IDLE
            },
        },
        Node {
            min_width: Val::Px(110.0),
            height: Val::Px(58.0),
            padding: UiRect::horizontal(Val::Px(20.0)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(Color::NONE),
        children![
            (
                SettingChoiceFill,
                Node {
                    position_type: PositionType::Absolute,
                    left: Val::ZERO,
                    top: Val::ZERO,
                    width: Val::Percent(if selected { 100.0 } else { 0.0 }),
                    height: Val::Percent(100.0),
                    ..default()
                },
                BackgroundColor(Color::srgba(1.0, 1.0, 1.0, OPTION_FILL_ALPHA)),
                FocusPolicy::Pass,
            ),
            text_weight(
                text,
                font,
                26.0,
                if selected {
                    OPTION_TEXT_ACTIVE
                } else {
                    OPTION_TEXT_IDLE
                },
                if selected {
                    FontWeight::BOLD
                } else {
                    FontWeight::NORMAL
                },
            )
        ],
    ));
}

fn setting_text(
    text: impl Into<String>,
    font: &Handle<Font>,
    size: f32,
    bold: bool,
) -> impl Bundle {
    text_weight(
        text,
        font,
        size,
        0.78,
        if bold {
            FontWeight::BOLD
        } else {
            FontWeight::NORMAL
        },
    )
}

pub fn handle_setting_action(
    actions: Query<(&Interaction, &SettingAction), Changed<Interaction>>,
    mut settings: ResMut<RuntimeSettings>,
    mut toggles: ResMut<ToggleStates>,
    mut pending_window: ResMut<PendingWindowMode>,
    project_root: Res<ProjectRoot>,
) {
    let Some(action) = actions.iter().find_map(|(interaction, action)| {
        (*interaction == Interaction::Pressed).then_some(*action)
    }) else {
        return;
    };
    match action {
        SettingAction::SetSkip(value) => {
            settings.skip_all = value;
            toggles.skip_mode = if settings.skip_all {
                SkipMode::All
            } else {
                SkipMode::Read
            };
            toggles.skip = false;
        }
        SettingAction::SetFullscreen(value) => {
            settings.fullscreen = value;
            pending_window.target = Some(value);
            pending_window.delay_frames = 1;
        }
        SettingAction::SetTextSize(value) => settings.text_size = value.min(2),
        SettingAction::Noop => return,
    }
    if let Err(error) = crate::storage::settings::persist(&settings, &project_root) {
        log::error!("failed to persist settings: {error:#}");
    }
}

pub fn apply_pending_window_mode(
    mut pending: ResMut<PendingWindowMode>,
    mut windows: Query<&mut Window>,
) {
    let Some(value) = pending.target else {
        return;
    };
    if pending.delay_frames > 0 {
        pending.delay_frames -= 1;
        return;
    }
    if let Ok(mut window) = windows.single_mut() {
        window.mode = if value {
            WindowMode::BorderlessFullscreen(MonitorSelection::Current)
        } else {
            WindowMode::Windowed
        };
    }
    pending.target = None;
}

pub fn handle_setting_sliders(
    sliders: Query<(
        &Interaction,
        &SettingSlider,
        &ComputedNode,
        &UiGlobalTransform,
    )>,
    windows: Query<&Window>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut settings: ResMut<RuntimeSettings>,
    project_root: Res<ProjectRoot>,
    mut drag: ResMut<ActiveSettingSlider>,
) {
    let Ok(window) = windows.single() else { return };
    // UI layout and UiGlobalTransform are expressed in physical pixels.
    // Using the logical cursor position on HiDPI displays offsets the hit
    // calculation and commonly clamps the slider to zero.
    if mouse.just_pressed(MouseButton::Left) {
        drag.kind = sliders
            .iter()
            .find(|(interaction, _, _, _)| {
                matches!(interaction, Interaction::Hovered | Interaction::Pressed)
            })
            .map(|(_, slider, _, _)| slider.0);
    }
    if mouse.pressed(MouseButton::Left)
        && let (Some(kind), Some(cursor)) = (drag.kind, window.physical_cursor_position())
    {
        for (_, slider, node, transform) in &sliders {
            if slider.0 != kind {
                continue;
            }
            let size = node.size();
            if size.x <= 0.0 {
                continue;
            }
            let left = transform.translation.x - size.x * 0.5;
            let ratio = ((cursor.x - left) / size.x).clamp(0.0, 1.0);
            if (slider.0.ratio(&settings) - ratio).abs() > 0.0005 {
                slider.0.set_ratio(&mut settings, ratio);
                drag.dirty = true;
            }
        }
    }
    if mouse.just_released(MouseButton::Left) {
        drag.kind = None;
        if drag.dirty {
            if let Err(error) = crate::storage::settings::persist(&settings, &project_root) {
                log::error!("failed to persist settings: {error:#}");
            }
            drag.dirty = false;
        }
    }
}

pub fn update_setting_visuals(
    time: Res<Time>,
    settings: Res<RuntimeSettings>,
    sliders: Query<(&Interaction, &SettingSlider)>,
    mut thumbs: Query<
        (
            &SettingSliderThumb,
            &mut SettingSliderThumbVisual,
            &mut Node,
            &mut BackgroundColor,
        ),
        Without<SettingChoice>,
    >,
    mut choices: Query<
        (
            &Interaction,
            &SettingChoice,
            &mut SettingChoiceVisual,
            &Children,
        ),
        Without<SettingSliderThumb>,
    >,
    mut fills: SettingChoiceFillQuery,
    mut choice_text: Query<(&mut TextColor, &mut TextFont)>,
) {
    let amount = exp_lerp(time.delta_secs(), OPTION_TRANSITION_RATE);
    for (kind, mut visual, mut node, mut background) in &mut thumbs {
        let hovered = sliders.iter().any(|(interaction, slider)| {
            slider.0 == kind.0 && matches!(interaction, Interaction::Hovered | Interaction::Pressed)
        });
        let target_width = if hovered { 12.0 } else { 10.0 };
        if (visual.0 - target_width).abs() < 0.001
            && node.left
                == Val::Percent(kind.0.ratio(&settings).clamp(0.0, 1.0) * (100.0 - visual.0))
        {
            continue;
        }
        visual.0 += (target_width - visual.0) * amount;
        node.width = Val::Percent(visual.0);
        node.left = Val::Percent(kind.0.ratio(&settings).clamp(0.0, 1.0) * (100.0 - visual.0));
        background.0 = Color::srgba(1.0, 1.0, 1.0, if hovered { 0.67 } else { 0.5 });
    }
    for (interaction, choice, mut visual, children) in &mut choices {
        let selected = choice_is_selected(&settings, choice.0);
        let hovered = matches!(interaction, Interaction::Hovered | Interaction::Pressed);
        visual.selected = selected;
        visual.hovered = hovered;
        let target_fill = if selected || hovered { 100.0 } else { 0.0 };
        let target_text = if selected || hovered {
            OPTION_TEXT_ACTIVE
        } else {
            OPTION_TEXT_IDLE
        };
        visual.fill += (target_fill - visual.fill) * amount;
        visual.text_alpha += (target_text - visual.text_alpha) * amount;
        for child in children.iter() {
            if let Ok((mut node, mut background)) = fills.get_mut(child) {
                node.width = Val::Percent(visual.fill);
                background.0 = Color::srgba(1.0, 1.0, 1.0, OPTION_FILL_ALPHA);
            }
            if let Ok((mut color, mut font)) = choice_text.get_mut(child) {
                color.0 = Color::srgba(1.0, 1.0, 1.0, visual.text_alpha);
                font.weight = if selected {
                    FontWeight::BOLD
                } else {
                    FontWeight::NORMAL
                };
            }
        }
    }
}

pub fn update_setting_bubbles(
    settings: Res<RuntimeSettings>,
    sliders: Query<(&Interaction, &SettingSlider)>,
    mut values: Query<(&SettingValueText, &mut Text)>,
    mut bubbles: Query<(&SettingValueBubble, &mut Node), Without<SettingSliderThumb>>,
) {
    for (kind, mut text) in &mut values {
        let value = kind.0.value_text(kind.0.ratio(&settings));
        if text.0 != value {
            text.0 = value;
        }
    }
    for (kind, mut node) in &mut bubbles {
        let active = sliders.iter().any(|(interaction, slider)| {
            slider.0 == kind.0 && matches!(interaction, Interaction::Hovered | Interaction::Pressed)
        });
        let display = if active { Display::Flex } else { Display::None };
        let left = Val::Percent(kind.0.ratio(&settings).clamp(0.0, 1.0) * 90.0);
        if node.display != display {
            node.display = display;
        }
        if node.left != left {
            node.left = left;
        }
    }
}

pub fn update_setting_preview(
    settings: Res<RuntimeSettings>,
    mut surfaces: Query<&mut BackgroundColor, With<SettingPreviewSurface>>,
    mut texts: Query<&mut TextFont, With<SettingPreviewText>>,
) {
    if !settings.is_changed() {
        return;
    }
    for mut background in &mut surfaces {
        background.0 = Color::srgba(0.0, 0.0, 0.03, settings.textbox_opacity * 0.72);
    }
    let size = match settings.text_size {
        0 => 31.0,
        2 => 42.0,
        _ => 36.0,
    };
    for mut font in &mut texts {
        font.font_size = FontSize::from(size);
    }
}

fn choice_is_selected(settings: &RuntimeSettings, action: SettingAction) -> bool {
    match action {
        SettingAction::SetSkip(value) => settings.skip_all == value,
        SettingAction::SetFullscreen(value) => settings.fullscreen == value,
        SettingAction::SetTextSize(value) => settings.text_size == value,
        SettingAction::Noop => false,
    }
}

pub fn update_settings_pages(
    time: Res<Time>,
    ui: Res<SettingsUi>,
    mut buttons: Query<(
        &Interaction,
        &SettingsPageButton,
        &mut SettingsPageButtonVisual,
        &Children,
    )>,
    mut labels: Query<&mut TextColor, With<SettingsPageLabel>>,
    mut panels: Query<(&mut SettingsPagePanel, &mut Node, &mut UiTransform)>,
) {
    let amount = exp_lerp(time.delta_secs(), OPTION_TRANSITION_RATE);
    for (interaction, page, mut visual, children) in &mut buttons {
        let active = ui.page == page.0;
        let hovered = matches!(interaction, Interaction::Hovered | Interaction::Pressed);
        let target = if active {
            PAGE_TEXT_ACTIVE
        } else if hovered {
            PAGE_TEXT_HOVER
        } else {
            PAGE_TEXT_IDLE
        };
        visual.0 += (target - visual.0) * amount;
        for child in children.iter() {
            if let Ok(mut color) = labels.get_mut(child) {
                color.0 = Color::srgba(1.0, 1.0, 1.0, visual.0);
            }
        }
    }

    for (mut panel, mut node, mut transform) in &mut panels {
        if panel.page != ui.page {
            if panel.progress != 0.0 {
                panel.progress = 0.0;
            }
            if node.display != Display::None {
                node.display = Display::None;
            }
            continue;
        }
        if panel.progress >= 1.0 {
            continue;
        }
        if node.display == Display::None {
            node.display = Display::Flex;
            panel.progress = 0.0;
        }
        panel.progress = (panel.progress + time.delta_secs() / 0.2).min(1.0);
        let eased = smoothstep(panel.progress);
        transform.translation = Val2::px(-140.0 * (1.0 - eased), 0.0);
        transform.scale = Vec2::ONE;
    }
}
