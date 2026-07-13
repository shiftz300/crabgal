use bevy::prelude::*;

use crate::runtime::resources::{AssetLoadingGate, GameState};
use crate::ui::backlog::BacklogUiState;
use crate::ui::dialog::DialogRequest;
use crate::ui::save_load::SaveLoadUi;
use crate::ui::settings_panel::SettingsUi;

#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum UiInputScope {
    Loading,
    UserInput,
    Dialog,
    Menu,
    Backlog,
    Title,
    #[default]
    Stage,
}

pub(crate) fn sync(
    loading: Res<AssetLoadingGate>,
    dialog: Option<Res<DialogRequest>>,
    settings: Res<SettingsUi>,
    save_load: Res<SaveLoadUi>,
    backlog: Res<BacklogUiState>,
    state: Res<GameState>,
    mut scope: ResMut<UiInputScope>,
) {
    *scope = if loading.blocked {
        UiInputScope::Loading
    } else if state.user_input.is_some() {
        UiInputScope::UserInput
    } else if dialog.is_some() {
        UiInputScope::Dialog
    } else if settings.open || save_load.mode.is_some() {
        UiInputScope::Menu
    } else if backlog.open {
        UiInputScope::Backlog
    } else if state.ended {
        UiInputScope::Title
    } else {
        UiInputScope::Stage
    };
}

pub(crate) fn backlog_allowed(scope: Res<UiInputScope>) -> bool {
    matches!(*scope, UiInputScope::Stage | UiInputScope::Backlog)
}

pub(crate) fn stage_allowed(scope: Res<UiInputScope>) -> bool {
    *scope == UiInputScope::Stage
}

pub(crate) fn title_allowed(scope: Res<UiInputScope>) -> bool {
    *scope == UiInputScope::Title
}

pub(crate) fn dialog_allowed(scope: Res<UiInputScope>) -> bool {
    *scope == UiInputScope::Dialog
}
