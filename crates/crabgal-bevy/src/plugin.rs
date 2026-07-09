use bevy::prelude::*;

use crate::game::{tick, resize};
use crate::render::blur;
use crate::resources::DesignScale;
use crate::scene::{background, sprites};
use crate::ui::{textbox, control_bar, loading, dialog};

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
        app.insert_resource(control_bar::ToggleStates::default());
        app.insert_resource(control_bar::AutoHideTiming::default());
        app.insert_resource(DesignScale::default());
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
        app.add_systems(Update, control_bar::auto_hide_tick.in_set(GameSystemSet::Input));
        app.add_systems(
            Update,
            (
                background::sync_bg,
                sprites::sync_sprites,
            )
                .in_set(GameSystemSet::Sync),
        );
        app.add_systems(Update, (resize::on_resize).in_set(GameSystemSet::Render));
        app.add_systems(Update, (textbox::update_textbox, textbox::apply_hide_toggle, control_bar::set_hover_target, control_bar::animate_hover, control_bar::handle_button_click, control_bar::auto_hide_apply, control_bar::sync_toggle_highlights, control_bar::update_lock_icon, dialog::handle_dialog_click, loading::update_loading).in_set(GameSystemSet::Ui));
        app.add_systems(PostUpdate, blur::update_blur_regions.after(bevy::ui::UiSystems::Layout));
        app.add_systems(PostUpdate, dialog::spawn_dialog);
    }
}
