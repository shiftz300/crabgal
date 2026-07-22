pub(crate) mod material;
pub(crate) mod particles;

use bevy::prelude::*;

pub(crate) struct StageEffectsPlugin;

impl Plugin for StageEffectsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            material::StageMaterialPlugin,
            particles::ParticleMaterialPlugin,
        ))
        .init_resource::<particles::ParticleRuntime>()
        .init_resource::<particles::ParticleClock>()
        .add_systems(
            Update,
            (particles::sync, particles::animate)
                .chain()
                .in_set(crate::runtime::GameSystemSet::Sync),
        );
    }
}
