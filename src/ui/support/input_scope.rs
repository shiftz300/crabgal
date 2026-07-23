use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::runtime::resources::{AssetLoadingGate, EditorSyncSession, GameState};
use crate::ui::backlog::BacklogUiState;
use crate::ui::dialog::DialogRequest;
use crate::ui::extra::ExtraUi;
use crate::ui::save_load::SaveLoadUi;
use crate::ui::settings_panel::SettingsUi;
use crate::ui::title::ReturnToTitleTransition;

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

impl UiInputScope {
    pub(crate) const fn allows_backlog(self) -> bool {
        matches!(self, Self::Stage | Self::Backlog)
    }

    pub(crate) const fn allows_menu(self) -> bool {
        matches!(self, Self::Stage | Self::Menu)
    }
}

#[derive(SystemParam)]
pub(crate) struct InputScopeContext<'w> {
    loading: Res<'w, AssetLoadingGate>,
    dialog: Option<Res<'w, DialogRequest>>,
    settings: Res<'w, SettingsUi>,
    save_load: Res<'w, SaveLoadUi>,
    backlog: Res<'w, BacklogUiState>,
    extra: Res<'w, ExtraUi>,
    return_to_title: Option<Res<'w, ReturnToTitleTransition>>,
    state: Res<'w, GameState>,
    scope: ResMut<'w, UiInputScope>,
}

pub(crate) fn sync(mut context: InputScopeContext) {
    *context.scope = if context.loading.blocked || context.return_to_title.is_some() {
        UiInputScope::Loading
    } else if context.dialog.is_some() {
        UiInputScope::Dialog
    } else if context.state.user_input.is_some() {
        UiInputScope::UserInput
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
    scope.allows_backlog()
}

pub(crate) fn menu_allowed(scope: Res<UiInputScope>) -> bool {
    scope.allows_menu()
}

pub(crate) fn user_input_allowed(scope: Res<UiInputScope>) -> bool {
    *scope == UiInputScope::UserInput
}

pub(crate) fn extra_allowed(scope: Res<UiInputScope>) -> bool {
    *scope == UiInputScope::Extra
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

/// Studio owns the preview cursor and its source project. Native preview UI
/// may still be inspected, but actions that mutate saves/settings stay inert.
pub(crate) fn writable_session(editor_sync: Option<Res<EditorSyncSession>>) -> bool {
    editor_sync.is_none()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_and_backlog_scopes_do_not_accept_modal_input() {
        assert!(UiInputScope::Stage.allows_menu());
        assert!(UiInputScope::Menu.allows_menu());
        assert!(!UiInputScope::Dialog.allows_menu());
        assert!(!UiInputScope::UserInput.allows_menu());

        assert!(UiInputScope::Stage.allows_backlog());
        assert!(UiInputScope::Backlog.allows_backlog());
        assert!(!UiInputScope::Menu.allows_backlog());
        assert!(!UiInputScope::Dialog.allows_backlog());
    }
}
