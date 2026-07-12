//! Fixed UI primitives for the dedicated crabgal shell.
//!
//! This is deliberately not a theme system: the engine has one visual
//! language. Keeping the two fonts and tiny animation helpers here removes
//! boilerplate without turning every screen into configurable framework code.

use bevy::prelude::*;
use bevy::text::FontWeight;

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
        width: Val::Percent(100.0),
        height: Val::Percent(100.0),
        ..default()
    }
}

pub(crate) fn exp_lerp(dt: f32, rate: f32) -> f32 {
    1.0 - (-dt * rate).exp()
}

pub(crate) fn smoothstep(value: f32) -> f32 {
    value * value * (3.0 - 2.0 * value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn animation_helpers_are_bounded() {
        assert_eq!(smoothstep(0.0), 0.0);
        assert_eq!(smoothstep(1.0), 1.0);
        assert!((0.0..=1.0).contains(&exp_lerp(1.0 / 60.0, 12.0)));
    }
}
