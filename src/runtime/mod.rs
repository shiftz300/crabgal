pub(crate) mod asset_reader;
pub(crate) mod audio;
mod bootstrap;
pub(crate) mod input;
pub(crate) mod lifecycle;
mod logging;
pub(crate) mod resources;
pub(crate) mod tick;
pub(crate) mod viewport;

use bevy::prelude::*;

use crate::scene::ScenePlugin;
use crate::storage::StoragePlugin;
use crate::ui::GameUiPlugin;

pub use bootstrap::{run, run_with_loader};

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
        app.init_resource::<lifecycle::RuntimeActivity>()
            .init_resource::<input::InputActions>();
        app.add_systems(PreUpdate, input::collect);
        app.add_systems(Update, tick::tick.in_set(GameSystemSet::Input));
        app.add_systems(Update, viewport::on_resize.in_set(GameSystemSet::Layout));
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
        #[cfg(feature = "audio-opus")]
        app.add_plugins(audio::OpusAudioPlugin);
        app.add_plugins((RuntimePlugin, ScenePlugin, StoragePlugin, GameUiPlugin));
    }
}
