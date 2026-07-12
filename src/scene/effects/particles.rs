use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use crabgal_core::{DESIGN_HEIGHT, DESIGN_WIDTH};

use crate::runtime::resources::GameState;
use crate::runtime::viewport::DesignViewport;

const PARTICLE_COUNT: usize = 72;

#[derive(Resource, Default)]
pub(crate) struct ParticleRuntime {
    effect: Option<String>,
}

#[derive(Component)]
pub(crate) struct StageParticle {
    position: Vec2,
    velocity: Vec2,
    size: Vec2,
    drift: f32,
}

pub(crate) fn sync(
    state: Res<GameState>,
    mut runtime: ResMut<ParticleRuntime>,
    particles: Query<Entity, With<StageParticle>>,
    mut commands: Commands,
) {
    if runtime.effect == state.particle_effect {
        return;
    }
    for entity in &particles {
        commands.entity(entity).despawn();
    }
    runtime.effect.clone_from(&state.particle_effect);
    let Some(effect) = runtime.effect.as_deref() else {
        return;
    };
    let style = ParticleStyle::from_name(effect);
    for index in 0..PARTICLE_COUNT {
        let x = hash(index as u32 * 3 + 1) * DESIGN_WIDTH;
        let y = hash(index as u32 * 3 + 2) * DESIGN_HEIGHT;
        let speed = style.speed * (0.72 + hash(index as u32 * 3 + 3) * 0.56);
        let size = style.size * (0.65 + hash(index as u32 + 91) * 0.7);
        commands.spawn((
            Name::new(format!("particle::{effect}::{index}")),
            StageParticle {
                position: Vec2::new(x, y),
                velocity: Vec2::new(style.wind, -speed),
                size: Vec2::new(size * style.aspect, size),
                drift: hash(index as u32 + 181) * std::f32::consts::TAU,
            },
            Sprite::from_color(style.color, Vec2::ONE),
            Transform::from_xyz(0.0, 0.0, 0.8),
            RenderLayers::layer(0),
        ));
    }
}

pub(crate) fn animate(
    time: Res<Time>,
    windows: Query<&Window>,
    mut particles: Query<(&mut StageParticle, &mut Transform)>,
) {
    if particles.is_empty() {
        return;
    }
    let Ok(window) = windows.single() else {
        return;
    };
    let viewport = DesignViewport::from_window(window);
    let delta = time.delta_secs();
    let elapsed = time.elapsed_secs();
    for (mut particle, mut transform) in &mut particles {
        let velocity = particle.velocity;
        particle.position += velocity * delta;
        particle.position.x += (elapsed * 1.7 + particle.drift).sin() * 18.0 * delta;
        if particle.position.y < -40.0 {
            particle.position.y = DESIGN_HEIGHT + 40.0;
        }
        if particle.position.x < -40.0 {
            particle.position.x = DESIGN_WIDTH + 40.0;
        } else if particle.position.x > DESIGN_WIDTH + 40.0 {
            particle.position.x = -40.0;
        }
        transform.translation = viewport.world_from_design(particle.position).extend(0.8);
        transform.scale = (particle.size * viewport.scale).extend(1.0);
        transform.rotation = Quat::from_rotation_z(particle.drift + elapsed * 0.15);
    }
}

struct ParticleStyle {
    color: Color,
    speed: f32,
    wind: f32,
    size: f32,
    aspect: f32,
}

impl ParticleStyle {
    fn from_name(name: &str) -> Self {
        let name = name.to_ascii_lowercase();
        if name.contains("snow") {
            Self {
                color: Color::srgba(1.0, 1.0, 1.0, 0.78),
                speed: 90.0,
                wind: 18.0,
                size: 10.0,
                aspect: 1.0,
            }
        } else if name.contains("sakura") || name.contains("petal") {
            Self {
                color: Color::srgba(1.0, 0.68, 0.78, 0.82),
                speed: 125.0,
                wind: 42.0,
                size: 13.0,
                aspect: 1.7,
            }
        } else if name.contains("dust") || name.contains("light") {
            Self {
                color: Color::srgba(1.0, 0.88, 0.54, 0.55),
                speed: 36.0,
                wind: -8.0,
                size: 7.0,
                aspect: 1.0,
            }
        } else {
            Self {
                color: Color::srgba(0.72, 0.86, 1.0, 0.62),
                speed: 720.0,
                wind: -80.0,
                size: 18.0,
                aspect: 0.12,
            }
        }
    }
}

fn hash(value: u32) -> f32 {
    let value = value.wrapping_mul(747_796_405).wrapping_add(2_891_336_453);
    let value = ((value >> ((value >> 28) + 4)) ^ value).wrapping_mul(277_803_737);
    ((value >> 22) ^ value) as f32 / u32::MAX as f32
}
