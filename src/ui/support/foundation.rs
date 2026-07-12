//! Fixed UI primitives for the dedicated crabgal shell.
//!
//! This is deliberately not a theme system: the engine has one visual
//! language. Keeping the two fonts and tiny animation helpers here removes
//! boilerplate without turning every screen into configurable framework code.

use bevy::prelude::*;
use bevy::text::FontWeight;
use crabgal_core::{DESIGN_HEIGHT, DESIGN_WIDTH};

pub(crate) const TEXT_FONT_PATH: &str = "fonts/MavenPro-CJK.ttf";
pub(crate) const ICON_FONT_PATH: &str = "fonts/bootstrap-icons.ttf";

#[derive(Resource, Clone)]
pub(crate) struct UiFonts {
    pub(crate) text: Handle<Font>,
    pub(crate) icons: Handle<Font>,
}

impl FromWorld for UiFonts {
    fn from_world(world: &mut World) -> Self {
        let assets = world.resource::<AssetServer>();
        Self {
            text: assets.load(TEXT_FONT_PATH),
            icons: assets.load(ICON_FONT_PATH),
        }
    }
}

pub(crate) fn text(
    content: impl Into<String>,
    font: &Handle<Font>,
    size: f32,
    alpha: f32,
) -> impl Bundle {
    text_weight(content, font, size, alpha, FontWeight::NORMAL)
}

pub(crate) fn text_weight(
    content: impl Into<String>,
    font: &Handle<Font>,
    size: f32,
    alpha: f32,
    weight: FontWeight,
) -> impl Bundle {
    (
        Text::new(content.into()),
        TextFont {
            font: font.clone().into(),
            font_size: FontSize::from(size),
            weight,
            ..default()
        },
        TextColor(Color::srgba(1.0, 1.0, 1.0, alpha)),
    )
}

pub(crate) fn fill_node() -> Node {
    Node {
        position_type: PositionType::Absolute,
        width: Val::Px(DESIGN_WIDTH),
        height: Val::Px(DESIGN_HEIGHT),
        ..default()
    }
}

pub(crate) fn exp_lerp(dt: f32, rate: f32) -> f32 {
    1.0 - (-dt * rate).exp()
}

pub(crate) fn smoothstep(value: f32) -> f32 {
    value * value * (3.0 - 2.0 * value)
}

pub(crate) fn ease_in_out_cubic(value: f32) -> f32 {
    let value = value.clamp(0.0, 1.0);
    if value < 0.5 {
        4.0 * value * value * value
    } else {
        1.0 - (-2.0 * value + 2.0).powi(3) / 2.0
    }
}

#[derive(Component)]
pub(crate) struct ButtonPressFeedback {
    scale: f32,
}

impl Default for ButtonPressFeedback {
    fn default() -> Self {
        Self { scale: 1.0 }
    }
}

impl ButtonPressFeedback {
    pub(crate) fn is_animating(&self, interaction: Interaction) -> bool {
        interaction == Interaction::Pressed || (self.scale - 1.0).abs() > 0.001
    }
}

type ButtonFeedbackFilter = (
    Without<crate::ui::title::TitleButtonMotion>,
    Without<crate::ui::save_load::SaveLoadSlotMotion>,
    Without<crate::ui::save_load::SaveLoadPageVisual>,
    Without<crate::ui::settings_panel::SettingSlider>,
);

type NewButtonQuery<'w, 's> =
    Query<'w, 's, (Entity, Option<&'static UiTransform>), (Added<Button>, ButtonFeedbackFilter)>;

pub(crate) fn attach_button_feedback(buttons: NewButtonQuery, mut commands: Commands) {
    for (entity, transform) in &buttons {
        let mut entity = commands.entity(entity);
        entity.insert(ButtonPressFeedback::default());
        if transform.is_none() {
            entity.insert(UiTransform::default());
        }
    }
}

pub(crate) fn animate_button_feedback(
    time: Res<Time>,
    mut buttons: Query<
        (&Interaction, &mut ButtonPressFeedback, &mut UiTransform),
        ButtonFeedbackFilter,
    >,
) {
    for (interaction, mut feedback, mut transform) in &mut buttons {
        let pressed = *interaction == Interaction::Pressed;
        if !feedback.is_animating(*interaction) {
            continue;
        }

        let target = if pressed { 0.965 } else { 1.0 };
        feedback.scale += (target - feedback.scale)
            * exp_lerp(time.delta_secs(), if pressed { 28.0 } else { 18.0 });
        if !pressed && (feedback.scale - 1.0).abs() < 0.001 {
            feedback.scale = 1.0;
        }
        transform.scale = Vec2::splat(feedback.scale);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn animation_helpers_are_bounded() {
        assert_eq!(smoothstep(0.0), 0.0);
        assert_eq!(smoothstep(1.0), 1.0);
        assert_eq!(ease_in_out_cubic(0.0), 0.0);
        assert_eq!(ease_in_out_cubic(1.0), 1.0);
        assert!((0.0..=1.0).contains(&exp_lerp(1.0 / 60.0, 12.0)));
    }
}
