//! Typed native-shell and open extension dispatch at the engine host boundary.

use std::collections::HashSet;

use bevy::prelude::*;

use crate::runtime::resources::GameState;
use crate::ui::backlog::BacklogUiState;
use crate::ui::control_bar::ToggleStates;
use crate::ui::extra::ExtraUi;
use crate::ui::save_load::{SaveLoadMode, SaveLoadUi};
use crate::ui::settings_panel::SettingsUi;
use crabgal_core::{ShellEvent, SystemUiSlot};

pub(crate) fn dispatch_shell(
    mut state: ResMut<GameState>,
    mut toggles: ResMut<ToggleStates>,
    mut save_load: ResMut<SaveLoadUi>,
    mut settings: ResMut<SettingsUi>,
    mut backlog: ResMut<BacklogUiState>,
    mut extra: ResMut<ExtraUi>,
) {
    for event in std::mem::take(&mut state.shell_events) {
        match event {
            ShellEvent::SetAutoplay(enabled) => toggles.auto = enabled,
            ShellEvent::SetSystemUi { slot, visible } => set_system_ui(
                slot,
                visible,
                &mut state,
                &mut save_load,
                &mut settings,
                &mut backlog,
                &mut extra,
            ),
        }
    }
}

fn set_system_ui(
    slot: SystemUiSlot,
    visible: bool,
    state: &mut crabgal_core::State,
    save_load: &mut SaveLoadUi,
    settings: &mut SettingsUi,
    backlog: &mut BacklogUiState,
    extra: &mut ExtraUi,
) {
    match (slot, visible) {
        (SystemUiSlot::Title, true) => crabgal_core::step::end_game(state),
        (SystemUiSlot::Save, true) => {
            settings.open = false;
            save_load.mode = Some(SaveLoadMode::Save);
        }
        (SystemUiSlot::Load, true) => {
            settings.open = false;
            save_load.mode = Some(SaveLoadMode::Load);
        }
        (SystemUiSlot::Settings, true) => {
            save_load.mode = None;
            settings.open = true;
        }
        (SystemUiSlot::History, true) => backlog.open = true,
        (SystemUiSlot::Gallery, true) => extra.open = true,
        (SystemUiSlot::Save | SystemUiSlot::Load, false) => save_load.mode = None,
        (SystemUiSlot::Settings, false) => settings.open = false,
        (SystemUiSlot::History, false) => backlog.open = false,
        (SystemUiSlot::Gallery, false) => extra.open = false,
        (SystemUiSlot::Input | SystemUiSlot::Title, false) | (SystemUiSlot::Input, true) => {}
    }
}

/// A preserved third-party extension call emitted by a project adapter.
///
/// Built-in engine behavior must use typed core actions. External plugins can
/// read this message without changing an adapter or the script VM.
#[derive(Message, Debug, Clone, PartialEq, Eq)]
pub struct HostCommandMessage(pub crabgal_core::HostCommandEvent);

/// Capability names claimed by installed extension plugins.
#[derive(Resource, Default)]
pub struct HostCapabilityRegistry(HashSet<(String, String)>);

impl HostCapabilityRegistry {
    pub fn claim(&mut self, namespace: impl Into<String>, command: impl Into<String>) {
        self.0.insert((namespace.into(), command.into()));
    }

    fn contains(&self, event: &crabgal_core::HostCommandEvent) -> bool {
        self.0
            .contains(&(event.namespace.clone(), event.command.clone()))
    }
}

#[derive(Resource, Default)]
pub(crate) struct HostCommandDiagnostics(HashSet<(String, String)>);

pub(crate) fn dispatch(
    mut state: ResMut<GameState>,
    mut messages: MessageWriter<HostCommandMessage>,
) {
    for event in std::mem::take(&mut state.host_commands) {
        messages.write(HostCommandMessage(event));
    }
}

pub(crate) fn diagnose_unhandled(
    mut messages: MessageReader<HostCommandMessage>,
    capabilities: Res<HostCapabilityRegistry>,
    mut diagnostics: ResMut<HostCommandDiagnostics>,
) {
    for message in messages.read() {
        let event = &message.0;
        if capabilities.contains(event) {
            continue;
        }
        if diagnostics
            .0
            .insert((event.namespace.clone(), event.command.clone()))
        {
            log::warn!(
                "no extension plugin handled capability {}/{}",
                event.namespace,
                event.command
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_system_slots_route_to_the_native_shell() {
        let mut state = crabgal_core::State::new();
        let mut save_load = SaveLoadUi::default();
        let mut settings = SettingsUi::default();
        let mut backlog = BacklogUiState::default();
        let mut extra = ExtraUi::default();

        set_system_ui(
            SystemUiSlot::Load,
            true,
            &mut state,
            &mut save_load,
            &mut settings,
            &mut backlog,
            &mut extra,
        );
        assert_eq!(save_load.mode, Some(SaveLoadMode::Load));

        set_system_ui(
            SystemUiSlot::Settings,
            true,
            &mut state,
            &mut save_load,
            &mut settings,
            &mut backlog,
            &mut extra,
        );
        assert!(settings.open);
        assert_eq!(save_load.mode, None);
    }
}
