mod bootstrap;
pub(crate) mod lifecycle;
pub(crate) mod resize;
pub(crate) mod resources;
pub(crate) mod tick;
pub(crate) mod viewport;

use bevy::prelude::*;

use crate::scene::ScenePlugin;
use crate::storage::StoragePlugin;
use crate::ui::GameUiPlugin;

pub use bootstrap::run;

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
pub(crate) enum GameSystemSet {
    Input,
    Sync,
    Layout,
    Ui,
}

pub(crate) struct RuntimePlugin;

impl Plugin for RuntimePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<lifecycle::RuntimeActivity>();
        app.add_systems(Update, tick::tick.in_set(GameSystemSet::Input));
        app.add_systems(Update, resize::on_resize.in_set(GameSystemSet::Layout));
        app.add_systems(Last, lifecycle::update);
    }
}

pub(crate) struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
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
        app.add_plugins((RuntimePlugin, ScenePlugin, StoragePlugin, GameUiPlugin));
    }
}
