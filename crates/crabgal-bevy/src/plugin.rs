use bevy::prelude::*;

use crate::game::{resize, tick};
use crate::render::blur;
use crate::scene::{assets, audio, background, sprites};
use crate::ui::{choice, control_bar, dialog, loading, text_style, textbox, title};

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
        app.insert_resource(control_bar::QuickSavePreview::default());
        app.insert_resource(audio::VocalPlayback::default());
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
            (
                control_bar::load_quick_save_preview,
                textbox::setup_textbox,
                loading::setup_loading,
            )
                .chain(),
        );
        app.add_systems(Update, tick::tick.in_set(GameSystemSet::Input));
        app.add_systems(
            Update,
            control_bar::auto_hide_tick.in_set(GameSystemSet::Input),
        );
        app.add_systems(
            Update,
            (
                assets::prefetch_local_assets,
                background::sync_bg,
                sprites::sync_sprites,
                audio::sync_vocal,
            )
                .in_set(GameSystemSet::Sync),
        );
        app.add_systems(Update, resize::on_resize.in_set(GameSystemSet::Layout));
        app.add_systems(
            Update,
            (
                textbox::update_textbox,
                textbox::update_mini_avatar,
                textbox::apply_hide_toggle,
                control_bar::set_hover_target,
                control_bar::animate_hover,
                control_bar::handle_button_click,
                (
                    control_bar::show_quick_preview,
                    control_bar::animate_quick_previews,
                )
                    .chain(),
                control_bar::sync_quick_preview,
                control_bar::auto_hide_apply,
                control_bar::sync_toggle_highlights,
                control_bar::update_lock_icon,
                dialog::handle_dialog_click,
                dialog::animate_dialog,
                dialog::update_dialog_buttons,
                loading::update_loading,
                title::sync_title,
                title::handle_title_input,
                text_style::apply_text_shadows,
            )
                .in_set(GameSystemSet::Ui),
        );
        app.add_systems(
            Update,
            (
                choice::sync_choice,
                choice::handle_choice_input,
                choice::animate_choice_buttons,
            )
                .chain()
                .in_set(GameSystemSet::Ui),
        );
        app.add_systems(
            PostUpdate,
            (
                control_bar::position_quick_previews,
                blur::update_blur_regions,
            )
                .chain()
                .after(bevy::ui::UiSystems::Layout),
        );
        app.add_systems(PostUpdate, dialog::spawn_dialog);
    }
}
