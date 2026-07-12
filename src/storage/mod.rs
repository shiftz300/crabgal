pub(crate) mod read_history;
pub(crate) mod save;
pub(crate) mod settings;

use bevy::prelude::*;

use crate::runtime::GameSystemSet;

pub(crate) struct StoragePlugin;

impl Plugin for StoragePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, settings::load_settings);
        app.add_systems(
            Update,
            read_history::persist_read_history.in_set(GameSystemSet::Sync),
        );
    }
}
