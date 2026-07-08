use bevy::prelude::*;

use crate::systems::*;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum GameSystemSet {
    Input,
    Sync,
    Render,
    Ui,
}

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            Update,
            (
                GameSystemSet::Input,
                GameSystemSet::Sync,
                GameSystemSet::Render,
                GameSystemSet::Ui,
            )
                .chain(),
        );

        app.add_systems(Startup, (textbox::setup_textbox, loading::setup_loading).chain());
        app.add_systems(Update, tick::tick.in_set(GameSystemSet::Input));
        app.add_systems(
            Update,
            (
                render_bg::sync_bg,
                render_sprites::sync_sprites,
            )
                .in_set(GameSystemSet::Sync),
        );
        app.add_systems(Update, (resize::on_resize, blur_regions::update_blur_regions).in_set(GameSystemSet::Render));
        app.add_systems(Update, render_sprites::update_tex_dims.in_set(GameSystemSet::Render));
        app.add_systems(Update, (textbox::update_textbox, loading::update_loading).in_set(GameSystemSet::Ui));
    }
}
