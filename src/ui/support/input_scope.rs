use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::runtime::resources::{AssetLoadingGate, GameState};
use crate::ui::backlog::BacklogUiState;
use crate::ui::dialog::DialogRequest;
use crate::ui::extra::ExtraUi;
use crate::ui::save_load::SaveLoadUi;
use crate::ui::settings_panel::SettingsUi;

#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum UiInputScope {
    Loading,
    UserInput,
    Dialog,
    Menu,
    Backlog,
    Extra,
    Title,
    #[default]
    Stage,
}

#[derive(SystemParam)]
pub(crate) struct InputScopeContext<'w> {
    loading: Res<'w, AssetLoadingGate>,
    dialog: Option<Res<'w, DialogRequest>>,
    settings: Res<'w, SettingsUi>,
    save_load: Res<'w, SaveLoadUi>,
    backlog: Res<'w, BacklogUiState>,
    extra: Res<'w, ExtraUi>,
    state: Res<'w, GameState>,
    scope: ResMut<'w, UiInputScope>,
}

pub(crate) fn sync(mut context: InputScopeContext) {
    *context.scope = if context.loading.blocked {
        UiInputScope::Loading
    } else if context.state.user_input.is_some() {
        UiInputScope::UserInput
    } else if context.dialog.is_some() {
        UiInputScope::Dialog
    } else if context.settings.open || context.save_load.mode.is_some() {
        UiInputScope::Menu
    } else if context.backlog.open {
        UiInputScope::Backlog
    } else if context.extra.open {
        UiInputScope::Extra
    } else if context.state.ended {
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
