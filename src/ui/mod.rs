pub(crate) mod activity;
pub mod backlog;
pub mod choice;
pub mod control_bar;
pub mod dialog;
pub(crate) mod foundation;
pub(crate) mod input_scope;
pub mod loading;
pub(crate) mod locale;
pub(crate) mod menu;
pub mod performance;
pub mod save_load;
pub mod settings_panel;
pub mod text_style;
pub mod textbox;
pub mod title;

use bevy::prelude::*;

use crate::render::blur;
use crate::runtime::GameSystemSet;

pub(crate) const FULLSCREEN_BLUR_STRENGTH: f32 = 48.0;
pub(crate) const MENU_BACKDROP_ALPHA: f32 = 0.8;
pub(crate) const BACKLOG_BACKDROP_ALPHA: f32 = 0.82;

pub(crate) struct GameUiPlugin;

impl Plugin for GameUiPlugin {
    fn build(&self, app: &mut App) {
        text_style::install_renderer(app);
        init_resources(app);
        app.add_systems(PreUpdate, input_scope::sync);
        add_startup_systems(app);
        app.add_systems(
            Update,
            control_bar::auto_hide_tick.in_set(GameSystemSet::Input),
        );
        add_stage_systems(app);
        add_overlay_systems(app);
        add_menu_systems(app);
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
        app.add_systems(
            Last,
            activity::update.before(crate::runtime::lifecycle::update),
        );
    }
}

fn init_resources(app: &mut App) {
    app.init_resource::<activity::UiAnimationActivity>()
        .init_resource::<foundation::UiFonts>()
        .init_resource::<control_bar::ToggleStates>()
        .init_resource::<control_bar::AutoHideTiming>()
        .init_resource::<control_bar::QuickSavePreview>()
        .init_resource::<textbox::TextboxOverlayFade>()
        .init_resource::<backlog::BacklogUiState>()
        .init_resource::<save_load::SaveLoadUi>()
        .init_resource::<save_load::SavePreviewCache>()
        .init_resource::<save_load::SaveLoadPageTransition>()
        .init_resource::<menu::MenuRouteTransition>()
        .init_resource::<settings_panel::SettingsUi>()
        .init_resource::<settings_panel::PendingWindowMode>()
        .init_resource::<settings_panel::ActiveSettingSlider>()
        .init_resource::<input_scope::UiInputScope>()
        .init_resource::<crate::storage::settings::RuntimeSettings>();
}

fn add_startup_systems(app: &mut App) {
    app.add_systems(
        Startup,
        (
            control_bar::load_quick_save_preview,
            textbox::setup_textbox,
            loading::setup_loading,
            performance::setup_performance_overlay,
        )
            .chain(),
    );
}

fn add_stage_systems(app: &mut App) {
    app.add_systems(
        Update,
        (
            (
                textbox::update_textbox,
                textbox::update_mini_avatar,
                textbox::animate_overlay_fade,
                textbox::apply_hide_toggle,
            )
                .chain(),
            control_bar::set_hover_target,
            control_bar::animate_hover,
            control_bar::handle_button_click.run_if(loading::assets_ready),
            (
                control_bar::show_quick_preview,
                control_bar::animate_quick_previews,
            )
                .chain(),
            control_bar::sync_quick_preview,
            control_bar::auto_hide_apply,
            control_bar::sync_toggle_highlights,
            control_bar::update_lock_icon,
            loading::update_loading,
            text_style::apply_text_shadows,
            (
                choice::sync_choice,
                choice::handle_choice_input
                    .run_if(loading::assets_ready)
                    .run_if(input_scope::stage_allowed),
                choice::animate_choice_buttons,
            )
                .chain(),
        )
            .in_set(GameSystemSet::Ui),
    );
}

fn add_overlay_systems(app: &mut App) {
    app.add_systems(
        Update,
        (
            (
                backlog::toggle_backlog
                    .run_if(loading::assets_ready)
                    .run_if(input_scope::backlog_allowed),
                backlog::sync_backlog,
                backlog::handle_backlog_action.run_if(loading::assets_ready),
                backlog::animate_backlog,
                backlog::animate_backlog_buttons,
                backlog::scroll_backlog,
            )
                .chain(),
            (
                dialog::sync_modal_backdrop_layer
                    .after(save_load::handle_save_load_slot)
                    .after(control_bar::handle_button_click),
                dialog::handle_dialog_click
                    .run_if(loading::assets_ready)
                    .run_if(input_scope::dialog_allowed),
                dialog::animate_dialog,
                dialog::update_dialog_buttons,
            )
                .chain(),
            (
                title::sync_title,
                title::handle_title_input
                    .run_if(loading::assets_ready)
                    .run_if(input_scope::title_allowed),
                title::animate_title_buttons,
            )
                .chain(),
            (
                performance::toggle_performance_overlay.run_if(loading::assets_ready),
                performance::update_performance_overlay,
            )
                .chain(),
        )
            .in_set(GameSystemSet::Ui),
    );
}

fn add_menu_systems(app: &mut App) {
    app.add_systems(
        Update,
        (
            (
                save_load::toggle_save_load.run_if(loading::assets_ready),
                settings_panel::toggle_settings.run_if(loading::assets_ready),
                save_load::handle_save_load_page
                    .run_if(loading::assets_ready)
                    .run_if(menu::route_settled),
                save_load::animate_page_transition,
                save_load::handle_save_load_slot
                    .run_if(loading::assets_ready)
                    .run_if(menu::route_settled),
                save_load::handle_save_delete
                    .run_if(loading::assets_ready)
                    .run_if(menu::route_settled),
                settings_panel::handle_setting_action
                    .run_if(loading::assets_ready)
                    .run_if(menu::route_settled),
                settings_panel::apply_pending_window_mode,
                settings_panel::handle_settings_page
                    .run_if(loading::assets_ready)
                    .run_if(menu::route_settled),
                settings_panel::handle_setting_sliders
                    .run_if(loading::assets_ready)
                    .run_if(menu::route_settled),
                save_load::sync_save_load,
            )
                .chain(),
            (
                save_load::poll_preview_tasks,
                settings_panel::sync_settings,
                menu::sync_tabs,
                settings_panel::update_setting_visuals.run_if(settings_panel::settings_open),
                settings_panel::update_setting_bubbles.run_if(settings_panel::settings_open),
                settings_panel::update_setting_preview.run_if(settings_panel::settings_open),
                settings_panel::update_settings_pages.run_if(settings_panel::settings_open),
                settings_panel::animate_watermark,
                settings_panel::fade_settings_visuals,
                save_load::animate_save_load_grid_track,
                save_load::animate_save_load_pages,
                save_load::animate_save_load_slots,
                save_load::animate_save_load_content,
                menu::animate,
                menu::animate_route_transition,
            )
                .chain(),
        )
            .chain()
            .in_set(GameSystemSet::Ui),
    );
}
