use bevy::prelude::*;

use crate::game::{resize, tick};
use crate::render::blur;
use crate::scene::{background, sprites};
use crate::ui::{control_bar, dialog, loading, textbox};

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub enum GameSystemSet {
    Input,
    Sync,
    Layout,
    Ui,
}

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(control_bar::ToggleStates::default());
        app.insert_resource(control_bar::AutoHideTiming::default());
        app.configure_sets(
            Update,
            (
                GameSystemSet::Input,
                GameSystemSet::Sync,
                GameSystemSet::Layout,
                GameSystemSet::Ui,
            )
                .chain(),
        );

        app.add_systems(
            Startup,
            (textbox::setup_textbox, loading::setup_loading).chain(),
        );
        app.add_systems(Update, tick::tick.in_set(GameSystemSet::Input));
        app.add_systems(
            Update,
            control_bar::auto_hide_tick.in_set(GameSystemSet::Input),
        );
        app.add_systems(
            Update,
            (background::sync_bg, sprites::sync_sprites).in_set(GameSystemSet::Sync),
        );
        app.add_systems(Update, resize::on_resize.in_set(GameSystemSet::Layout));
        app.add_systems(
            Update,
            (
                textbox::update_textbox,
                textbox::apply_hide_toggle,
                control_bar::set_hover_target,
                control_bar::animate_hover,
                control_bar::handle_button_click,
                control_bar::auto_hide_apply,
                control_bar::sync_toggle_highlights,
                control_bar::update_lock_icon,
                dialog::handle_dialog_click,
                loading::update_loading,
            )
                .in_set(GameSystemSet::Ui),
        );
        app.add_systems(
            PostUpdate,
            blur::update_blur_regions.after(bevy::ui::UiSystems::Layout),
        );
        app.add_systems(PostUpdate, dialog::spawn_dialog);
    }
}
