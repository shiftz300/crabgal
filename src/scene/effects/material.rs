use bevy::asset::{AssetPath, embedded_asset, embedded_path};
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, BlendComponent, BlendFactor, BlendOperation, BlendState, RenderPipelineDescriptor,
    SpecializedMeshPipelineError,
};
use bevy::shader::ShaderRef;
use bevy::sprite_render::{AlphaMode2d, Material2d, Material2dKey, Material2dPlugin};
use crabgal_core::{BlendMode, VisualFilter};

pub(crate) struct StageMaterialPlugin;

impl Plugin for StageMaterialPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "stage_material.wgsl");
        app.add_plugins(Material2dPlugin::<StageMaterial>::default())
            .add_systems(Startup, setup_quad);
    }
}

#[derive(Resource, Clone)]
pub(crate) struct StageQuad(pub(crate) Handle<Mesh>);

fn setup_quad(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    commands.insert_resource(StageQuad(meshes.add(Rectangle::new(1.0, 1.0))));
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
#[bind_group_data(StageMaterialKey)]
pub(crate) struct StageMaterial {
    #[uniform(0)]
    pub(crate) tint: Vec4,
    #[uniform(1)]
    pub(crate) filter: Vec4,
    #[uniform(2)]
    pub(crate) transition: Vec4,
    #[texture(3)]
    #[sampler(4)]
    pub(crate) image: Handle<Image>,
    pub(crate) blend: BlendMode,
}

impl StageMaterial {
    pub(crate) fn new(
        image: Handle<Image>,
        alpha: f32,
        filter: VisualFilter,
        blend: BlendMode,
        transition: Vec4,
    ) -> Self {
        Self {
            tint: Vec4::new(1.0, 1.0, 1.0, alpha.clamp(0.0, 1.0)),
            filter: Vec4::new(
                filter.blur.max(0.0),
                filter.brightness.clamp(0.0, 4.0),
                filter.contrast.clamp(0.0, 4.0),
                filter.saturation.clamp(0.0, 4.0),
            ),
            transition,
            image,
            blend,
        }
    }
}

pub(crate) fn animation_uniform(animation: Option<&crabgal_core::state::PresetAnimation>) -> Vec4 {
    use crabgal_core::AnimationPreset;
    let Some(animation) = animation else {
        return Vec4::ZERO;
    };
    let kind = match animation.preset {
        AnimationPreset::OldFilm => 1.0,
        AnimationPreset::DotFilm => 2.0,
        AnimationPreset::ReflectionFilm => 3.0,
        AnimationPreset::GlitchFilm => 4.0,
        AnimationPreset::RgbFilm => 5.0,
        AnimationPreset::GodrayFilm => 6.0,
        _ => 0.0,
    };
    Vec4::new(
        0.0,
        0.0,
        kind,
        (animation.elapsed / animation.duration).clamp(0.0, 1.0),
    )
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct StageMaterialKey(u8);

impl From<&StageMaterial> for StageMaterialKey {
    fn from(material: &StageMaterial) -> Self {
        Self(match material.blend {
            BlendMode::Alpha => 0,
            BlendMode::Add => 1,
            BlendMode::Multiply => 2,
            BlendMode::Screen => 3,
        })
    }
}

impl Material2d for StageMaterial {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Path(
            AssetPath::from_path_buf(embedded_path!("stage_material.wgsl")).with_source("embedded"),
        )
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }

    fn specialize(
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        key: Material2dKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let blend = match key.bind_group_data.0 {
            1 => BlendState {
                color: BlendComponent {
                    src_factor: BlendFactor::SrcAlpha,
                    dst_factor: BlendFactor::One,
                    operation: BlendOperation::Add,
                },
                alpha: BlendComponent::OVER,
            },
            2 => BlendState {
                color: BlendComponent {
                    src_factor: BlendFactor::Dst,
                    dst_factor: BlendFactor::OneMinusSrcAlpha,
                    operation: BlendOperation::Add,
                },
                alpha: BlendComponent::OVER,
            },
            3 => BlendState {
                color: BlendComponent {
                    src_factor: BlendFactor::One,
                    dst_factor: BlendFactor::OneMinusSrc,
                    operation: BlendOperation::Add,
                },
                alpha: BlendComponent::OVER,
            },
            _ => BlendState::ALPHA_BLENDING,
        };
        if let Some(fragment) = descriptor.fragment.as_mut() {
            match key.bind_group_data.0 {
                2 => fragment.shader_defs.push("BLEND_MULTIPLY".into()),
                3 => fragment.shader_defs.push("BLEND_SCREEN".into()),
                _ => {}
            }
            if let Some(target) = fragment.targets.first_mut().and_then(Option::as_mut) {
                target.blend = Some(blend);
            }
        }
        Ok(())
    }
}
