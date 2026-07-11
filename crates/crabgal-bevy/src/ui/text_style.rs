use bevy::prelude::*;

#[derive(Component)]
pub struct TextBackdrop;

/// Applies the fixed MainCore text treatment to every UI text entity, including
/// text spawned later by dialogs, choices, previews, and the title screen.
pub fn apply_text_shadows(
    texts: Query<Entity, (With<Text>, Without<TextBackdrop>)>,
    mut commands: Commands,
) {
    for entity in &texts {
        commands.entity(entity).insert((
            TextBackdrop,
            TextShadow {
                offset: Vec2::splat(2.0),
                color: Color::srgba(0.0, 0.0, 0.0, 0.9),
            },
        ));
    }
}
