// Lightweight region-based Gaussian blur post-processing for Bevy 0.19.
use std::borrow::Cow;
use bevy::prelude::*;
use bevy::{
    asset::{embedded_asset, load_embedded_asset},
    core_pipeline::{Core2d, Core2dSystems, FullscreenShader},
    render::{
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        render_resource::{binding_types::*, *},
        renderer::{RenderContext, RenderDevice, ViewQuery},
        view::ViewTarget,
        RenderApp, RenderStartup,
    },
};
use bevy::ui::{ComputedNode, UiGlobalTransform};
use crate::ui::control_bar::{AutoHideTiming, BlurSource, ToggleStates};
use crate::ui::dialog::DialogRequest;

// ── BlurCamera ──
#[derive(Component, Clone, ExtractComponent, ShaderType)]
pub struct BlurCamera { pub count: u32, pub coc: f32, pub _pad: Vec2, pub rects: [BlurRect; 2] }
#[derive(Clone, Copy, Default, ShaderType)]
pub(crate) struct BlurRect { pub min_x: f32, pub max_x: f32, pub min_y: f32, pub max_y: f32 }

impl Default for BlurCamera {
    fn default() -> Self { Self { count: 0, coc: 30.0, _pad: Vec2::ZERO, rects: [BlurRect::default(); 2] } }
}

// ── Plugin ──
pub struct BlurPlugin;
impl Plugin for BlurPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "blur.wgsl");
        app.add_plugins(ExtractComponentPlugin::<BlurCamera>::default());
    }
    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else { return };
        let shader_handle: Handle<Shader> = load_embedded_asset!(render_app.world_mut(), "blur.wgsl");
        render_app.insert_resource(BlurShader(shader_handle));
        render_app
            .add_systems(Core2d, (do_blur).in_set(Core2dSystems::EarlyPostProcess));
        render_app.add_systems(RenderStartup, setup_blur_pipeline.ambiguous_with_all());
    }
}

// ── Pipeline resource ──
#[derive(Resource)]
struct BlurShader(Handle<Shader>);
#[derive(Resource)]
struct BlurPipeline { layout: BindGroupLayout, sampler: Sampler, h_pipe: CachedRenderPipelineId, v_pipe: CachedRenderPipelineId }

fn setup_blur_pipeline(
    device: Res<RenderDevice>,
    pipeline_cache: ResMut<PipelineCache>,
    fullscreen_shader: Res<FullscreenShader>,
    shader: Res<BlurShader>,
    mut commands: Commands,
) {
    let entries = &BindGroupLayoutEntries::sequential(ShaderStages::FRAGMENT, (
        texture_2d(TextureSampleType::Float { filterable: true }),
        sampler(SamplerBindingType::Filtering),
        uniform_buffer::<BlurCamera>(false),
    ));
    let layout_desc = BindGroupLayoutDescriptor::new("blur_layout", entries);
    let layout = device.create_bind_group_layout("blur_layout", entries);
    let sampler = device.create_sampler(&SamplerDescriptor {
        address_mode_u: AddressMode::ClampToEdge, address_mode_v: AddressMode::ClampToEdge,
        mag_filter: FilterMode::Linear, min_filter: FilterMode::Linear, ..default()
    });
    let h_pipe = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
        label: Some(Cow::Borrowed("blur")), layout: vec![layout_desc.clone()], immediate_size: 0,
        vertex: fullscreen_shader.to_vertex_state(),
        fragment: Some(FragmentState {
            shader: shader.0.clone(), shader_defs: vec![], entry_point: Some(Cow::Borrowed("horizontal")),
            targets: vec![Some(ColorTargetState {
                format: TextureFormat::Rgba8UnormSrgb, blend: None, write_mask: ColorWrites::ALL,
            })],
        }),
        primitive: PrimitiveState::default(), depth_stencil: None,
        multisample: MultisampleState::default(), zero_initialize_workgroup_memory: false,
    });
    let v_pipe = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
        label: Some(Cow::Borrowed("blur")), layout: vec![layout_desc.clone()], immediate_size: 0,
        vertex: fullscreen_shader.to_vertex_state(),
        fragment: Some(FragmentState {
            shader: shader.0.clone(), shader_defs: vec![], entry_point: Some(Cow::Borrowed("vertical")),
            targets: vec![Some(ColorTargetState {
                format: TextureFormat::Rgba8UnormSrgb, blend: None, write_mask: ColorWrites::ALL,
            })],
        }),
        primitive: PrimitiveState::default(), depth_stencil: None,
        multisample: MultisampleState::default(), zero_initialize_workgroup_memory: false,
    });
    commands.insert_resource(BlurPipeline { layout, sampler, h_pipe, v_pipe });
}

// ── Render function ──

fn do_blur(
    view: ViewQuery<(&ViewTarget, &BlurCamera)>,
    pipeline_cache: Res<PipelineCache>,
    blur_pipeline: Res<BlurPipeline>,
    mut ctx: RenderContext,
) {
    let (vt, bc) = view.into_inner();
    for pipe in [&blur_pipeline.h_pipe, &blur_pipeline.v_pipe] {
        let Some(p) = pipeline_cache.get_render_pipeline(*pipe) else { continue };
        let post = vt.post_process_write();
        let mut buf = [0u8; 48];
        buf[0..4].copy_from_slice(&bc.count.to_le_bytes());
        buf[4..8].copy_from_slice(&bc.coc.to_le_bytes());
        for (i, r) in bc.rects.iter().enumerate() {
            let off = 16 + i * 16;
            buf[off..off+4].copy_from_slice(&r.min_x.to_le_bytes());
            buf[off+4..off+8].copy_from_slice(&r.max_x.to_le_bytes());
            buf[off+8..off+12].copy_from_slice(&r.min_y.to_le_bytes());
            buf[off+12..off+16].copy_from_slice(&r.max_y.to_le_bytes());
        }
        let buffer = ctx.render_device().create_buffer_with_data(&BufferInitDescriptor {
            label: Some("blur_ubo"), contents: &buf, usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });
        let bg = ctx.render_device().create_bind_group("blur_bg", &blur_pipeline.layout,
            &BindGroupEntries::sequential((post.source, &blur_pipeline.sampler, buffer.as_entire_binding())),
        );
        let mut pass = ctx.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("blur"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: post.destination, resolve_target: None, ops: Operations::default(), depth_slice: None,
            })],
            depth_stencil_attachment: None, timestamp_writes: None, occlusion_query_set: None, multiview_mask: None,
        });
        pass.set_render_pipeline(p);
        pass.set_bind_group(0, &bg, &[]);
        pass.draw(0..3, 0..1);
    }
}

// ── Main-world system ──
/// Reads UI node positions from ComputedNode + UiGlobalTransform (Bevy 0.19 UI layout).
/// BlurSource markers tell the system which nodes need blur behind them.
/// No more manual coordinate sync — just tag a node and blur follows.
pub fn update_blur_regions(
    window_query: Query<&Window>,
    mut blur_q: Query<&mut BlurCamera>,
    dialog: Option<Res<DialogRequest>>,
    timing: Res<AutoHideTiming>,
    toggles: Res<ToggleStates>,
    blur_nodes: Query<(&ComputedNode, &UiGlobalTransform), With<BlurSource>>,
) {
    let Ok(mut bc) = blur_q.single_mut() else { return };
    let Ok(w) = window_query.single() else { return };
    let sf = w.scale_factor() as f32;
    let sh = w.height() * sf;

    // Full-screen blur when dialog is active
    if dialog.is_some() {
        bc.rects[0] = BlurRect { min_x: 0.0, max_x: w.width() * sf, min_y: 0.0, max_y: sh };
        bc.coc = 15.0;
        bc.count = 1;
        return;
    }

    // Skip blur when hide content is mostly gone
    if toggles.hide && timing.hide_alpha < 0.5 {
        bc.count = 0;
        return;
    }

    bc.coc = 30.0;
    bc.count = 0;
    for (i, (node, transform)) in blur_nodes.iter().enumerate() {
        let size = node.size(); // physical pixels
        let pos = transform.translation; // Vec2, screen-space center in physical px
        let half = size * 0.5;
        bc.rects[i] = BlurRect {
            min_x: pos.x - half.x,
            max_x: pos.x + half.x,
            min_y: pos.y - half.y,
            max_y: pos.y + half.y,
        };
        bc.count = (i + 1) as u32;
        if bc.count >= 2 { break; }
    }
}