// Loading screen — shown while assets are being loaded.
use bevy::prelude::*;

#[derive(Component)]
pub(crate) struct LoadingText;

pub fn setup_loading(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font: Handle<Font> = asset_server.load("fonts/MavenPro-CJK.ttf");
    commands.spawn((
        Name::new("loading"),
        LoadingText,
        Text::new("Loading..."),
        TextFont {
            font: font.into(),
            font_size: FontSize::from(24.0),
            ..default()
        },
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Percent(50.0),
            left: Val::Percent(50.0),
            ..default()
        },
        ZIndex(200),
    ));
}

pub fn update_loading(
    mut query: Query<(Entity, &mut Visibility), With<LoadingText>>,
) {
    // With lazy loading, hide loading indicator immediately
    for (_entity, mut vis) in query.iter_mut() {
        *vis = Visibility::Hidden;
    }
}
