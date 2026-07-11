// Lightweight region-based Gaussian blur post-processing for Bevy 0.19.
use crate::components::SpriteNode;
use crate::resources::GameState;
use crate::ui::control_bar::{
    AutoHideTiming, BlurSource, BlurStrength, ToggleStates, UiBlurSource,
};
use crate::ui::dialog::DialogRequest;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::ui::{ComputedNode, UiGlobalTransform};
use bevy::{
    asset::{embedded_asset, load_embedded_asset},
    core_pipeline::{Core2d, Core2dSystems, FullscreenShader},
    render::{
        RenderApp, RenderStartup,
        extract_component::{ExtractComponent, ExtractComponentPlugin},
        render_resource::{binding_types::*, *},
        renderer::{RenderContext, RenderDevice, ViewQuery},
        view::ViewTarget,
    },
};
use std::borrow::Cow;

// ── BlurCamera ──
#[derive(Component, Clone, ExtractComponent, ShaderType)]
pub struct BlurCamera {
    pub count: u32,
    pub coc: f32,
    pub _pad: Vec2,
    pub rects: [BlurRect; 16],
}
#[derive(Clone, Copy, Default, ShaderType)]
pub(crate) struct BlurRect {
    pub min_x: f32,
    pub max_x: f32,
    pub min_y: f32,
    pub max_y: f32,
}

/// The layer-0 camera applies regional blur behind the regular UI.
#[derive(Component, Clone, ExtractComponent)]
pub struct SceneBlurCamera;

/// The layer-1 camera applies full-screen blur behind a modal dialog.
#[derive(Component, Clone, ExtractComponent)]
pub struct UiBlurCamera;

/// The layer-2 camera renders modal UI after all backdrop processing.
#[derive(Component)]
pub struct DialogCamera;

impl Default for BlurCamera {
    fn default() -> Self {
        Self {
            count: 0,
            coc: 30.0,
            _pad: Vec2::ZERO,
            rects: [BlurRect::default(); 16],
        }
    }
}

// ── Plugin ──
pub struct BlurPlugin;
impl Plugin for BlurPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "blur.wgsl");
        app.add_plugins(ExtractComponentPlugin::<BlurCamera>::default());
        app.add_plugins(ExtractComponentPlugin::<SceneBlurCamera>::default());
        app.add_plugins(ExtractComponentPlugin::<UiBlurCamera>::default());
    }
    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        let shader_handle: Handle<Shader> =
            load_embedded_asset!(render_app.world_mut(), "blur.wgsl");
        render_app.insert_resource(BlurShader(shader_handle));
        render_app.add_systems(
            Core2d,
            do_scene_blur.in_set(Core2dSystems::EarlyPostProcess),
        );
        render_app.add_systems(
            Core2d,
            do_ui_blur
                .after(bevy::ui_render::render_pass::ui_pass)
                .before(bevy::core_pipeline::upscaling::upscaling),
        );
        render_app.add_systems(RenderStartup, setup_blur_pipeline.ambiguous_with_all());
    }
}

// ── Pipeline resource ──
#[derive(Resource)]
struct BlurShader(Handle<Shader>);
#[derive(Resource)]
struct BlurPipeline {
    layout: BindGroupLayout,
    sampler: Sampler,
    h_pipe: CachedRenderPipelineId,
    v_pipe: CachedRenderPipelineId,
}

fn setup_blur_pipeline(
    device: Res<RenderDevice>,
    pipeline_cache: ResMut<PipelineCache>,
    fullscreen_shader: Res<FullscreenShader>,
    shader: Res<BlurShader>,
    mut commands: Commands,
) {
    let entries = &BindGroupLayoutEntries::sequential(
        ShaderStages::FRAGMENT,
        (
            texture_2d(TextureSampleType::Float { filterable: true }),
            sampler(SamplerBindingType::Filtering),
            uniform_buffer::<BlurCamera>(false),
        ),
    );
    let layout_desc = BindGroupLayoutDescriptor::new("blur_layout", entries);
    let layout = device.create_bind_group_layout("blur_layout", entries);
    let sampler = device.create_sampler(&SamplerDescriptor {
        address_mode_u: AddressMode::ClampToEdge,
        address_mode_v: AddressMode::ClampToEdge,
        mag_filter: FilterMode::Linear,
        min_filter: FilterMode::Linear,
        ..default()
    });
    let h_pipe = queue_blur_pipeline(
        &pipeline_cache,
        &fullscreen_shader,
        &shader.0,
        layout_desc.clone(),
        "horizontal",
    );
    let v_pipe = queue_blur_pipeline(
        &pipeline_cache,
        &fullscreen_shader,
        &shader.0,
        layout_desc,
        "vertical",
    );
    commands.insert_resource(BlurPipeline {
        layout,
        sampler,
        h_pipe,
        v_pipe,
    });
}

fn queue_blur_pipeline(
    pipeline_cache: &PipelineCache,
    fullscreen_shader: &FullscreenShader,
    shader: &Handle<Shader>,
    layout: BindGroupLayoutDescriptor,
    entry_point: &'static str,
) -> CachedRenderPipelineId {
    pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
        label: Some(Cow::Owned(format!("blur_{entry_point}"))),
        layout: vec![layout],
        immediate_size: 0,
        vertex: fullscreen_shader.to_vertex_state(),
        fragment: Some(FragmentState {
            shader: shader.clone(),
            shader_defs: vec![],
            entry_point: Some(Cow::Borrowed(entry_point)),
            targets: vec![Some(ColorTargetState {
                format: TextureFormat::Rgba8UnormSrgb,
                blend: None,
                write_mask: ColorWrites::ALL,
            })],
        }),
        primitive: PrimitiveState::default(),
        depth_stencil: None,
        multisample: MultisampleState::default(),
        zero_initialize_workgroup_memory: false,
    })
}

// ── Render function ──

fn do_scene_blur(
    view: ViewQuery<(&ViewTarget, &BlurCamera, Option<&SceneBlurCamera>)>,
    pipeline_cache: Res<PipelineCache>,
    blur_pipeline: Res<BlurPipeline>,
    mut ctx: RenderContext,
) {
    let (vt, bc, marker) = view.into_inner();
    if marker.is_none() {
        return;
    }
    run_blur(vt, bc, &pipeline_cache, &blur_pipeline, &mut ctx);
}

fn do_ui_blur(
    view: ViewQuery<(&ViewTarget, &BlurCamera, Option<&UiBlurCamera>)>,
    pipeline_cache: Res<PipelineCache>,
    blur_pipeline: Res<BlurPipeline>,
    mut ctx: RenderContext,
) {
    let (vt, bc, marker) = view.into_inner();
    if marker.is_none() {
        return;
    }
    run_blur(vt, bc, &pipeline_cache, &blur_pipeline, &mut ctx);
}

fn run_blur(
    vt: &ViewTarget,
    bc: &BlurCamera,
    pipeline_cache: &PipelineCache,
    blur_pipeline: &BlurPipeline,
    ctx: &mut RenderContext,
) {
    // Avoid two full-screen pass-through draws when no blur is requested.
    if bc.count == 0 {
        return;
    }

    let mut uniform = encase::UniformBuffer::new(Vec::new());
    if let Err(error) = uniform.write(bc) {
        log::error!("failed to encode blur uniform: {error}");
        return;
    }
    let bytes = uniform.into_inner();
    let buffer = ctx
        .render_device()
        .create_buffer_with_data(&BufferInitDescriptor {
            label: Some("blur_ubo"),
            contents: &bytes,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

    for pipe in [&blur_pipeline.h_pipe, &blur_pipeline.v_pipe] {
        let Some(p) = pipeline_cache.get_render_pipeline(*pipe) else {
            continue;
        };
        let post = vt.post_process_write();
        let bg = ctx.render_device().create_bind_group(
            "blur_bg",
            &blur_pipeline.layout,
            &BindGroupEntries::sequential((
                post.source,
                &blur_pipeline.sampler,
                buffer.as_entire_binding(),
            )),
        );
        let mut pass = ctx.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("blur"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: post.destination,
                resolve_target: None,
                ops: Operations::default(),
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
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
type SceneBlurQuery<'w, 's> =
    Query<'w, 's, &'static mut BlurCamera, (With<SceneBlurCamera>, Without<UiBlurCamera>)>;
type UiBlurQuery<'w, 's> =
    Query<'w, 's, &'static mut BlurCamera, (With<UiBlurCamera>, Without<SceneBlurCamera>)>;

#[derive(SystemParam)]
pub struct BlurBehavior<'w> {
    dialog: Option<Res<'w, DialogRequest>>,
    timing: Res<'w, AutoHideTiming>,
    toggles: Res<'w, ToggleStates>,
    state: Res<'w, GameState>,
}

pub fn update_blur_regions(
    window_query: Query<&Window>,
    mut scene_blur_query: SceneBlurQuery,
    mut ui_blur_query: UiBlurQuery,
    behavior: BlurBehavior,
    blur_nodes: Query<(&ComputedNode, &UiGlobalTransform, Option<&BlurStrength>), With<BlurSource>>,
    ui_blur_nodes: Query<
        (&ComputedNode, &UiGlobalTransform, Option<&BlurStrength>),
        With<UiBlurSource>,
    >,
    world_sprites: Query<(&SpriteNode, &Sprite, &Transform)>,
) {
    let Ok(w) = window_query.single() else { return };
    let sf = w.scale_factor();
    let sh = w.height() * sf;

    // Camera 1 runs after the regular UI has been composited, so a full-screen
    // pass here blurs the scene, textbox, and control bar together. Camera 2
    // then draws the dialog itself without post-processing.
    if behavior.dialog.is_some() {
        if let Ok(mut bc) = scene_blur_query.single_mut() {
            bc.count = 0;
        }
        if let Ok(mut bc) = ui_blur_query.single_mut() {
            bc.rects[0] = BlurRect {
                min_x: 0.0,
                max_x: w.width() * sf,
                min_y: 0.0,
                max_y: sh,
            };
            bc.coc = 36.0;
            bc.count = 1;
        }
        return;
    }

    if let Ok(mut bc) = ui_blur_query.single_mut() {
        bc.count = 0;
        bc.coc = 0.0;
        for (node, transform, strength) in &ui_blur_nodes {
            let size = node.size();
            if size.x <= 0.0 || size.y <= 0.0 {
                continue;
            }
            let index = bc.count as usize;
            if index >= bc.rects.len() {
                break;
            }
            let position = transform.translation;
            let half = size * 0.5;
            bc.rects[index] = BlurRect {
                min_x: position.x - half.x,
                max_x: position.x + half.x,
                min_y: position.y - half.y,
                max_y: position.y + half.y,
            };
            bc.count += 1;
            bc.coc = bc.coc.max(strength.map_or(30.0, |strength| strength.0));
        }
    }

    let Ok(mut bc) = scene_blur_query.single_mut() else {
        return;
    };

    // Skip blur when hide content is mostly gone
    if behavior.toggles.hide && behavior.timing.hide_alpha < 0.5 {
        bc.count = 0;
        return;
    }

    bc.coc = 0.0;
    bc.count = 0;
    if behavior.state.bg_transform.blur > 0.0 {
        bc.rects[0] = BlurRect {
            min_x: 0.0,
            max_x: w.width() * sf,
            min_y: 0.0,
            max_y: sh,
        };
        bc.count = 1;
        bc.coc = behavior.state.bg_transform.blur;
    }
    for (node, sprite, transform) in &world_sprites {
        let Some(effect) = behavior
            .state
            .sprites
            .get(&node.0)
            .map(|sprite| sprite.transform)
        else {
            continue;
        };
        if effect.blur <= 0.0 {
            continue;
        }
        let Some(size) = sprite.custom_size else {
            continue;
        };
        let index = bc.count as usize;
        if index >= bc.rects.len() {
            break;
        }
        let center =
            (transform.translation.truncate() + Vec2::new(w.width(), w.height()) * 0.5) * sf;
        let half = size.abs() * sf * 0.5;
        bc.rects[index] = BlurRect {
            min_x: center.x - half.x,
            max_x: center.x + half.x,
            min_y: center.y - half.y,
            max_y: center.y + half.y,
        };
        bc.count += 1;
        bc.coc = bc.coc.max(effect.blur);
    }
    for (node, transform, strength) in &blur_nodes {
        let size = node.size(); // physical pixels
        if size.x <= 0.0 || size.y <= 0.0 {
            continue;
        }
        let index = bc.count as usize;
        if index >= bc.rects.len() {
            break;
        }
        let pos = transform.translation; // Vec2, screen-space center in physical px
        let half = size * 0.5;
        bc.rects[index] = BlurRect {
            min_x: pos.x - half.x,
            max_x: pos.x + half.x,
            min_y: pos.y - half.y,
            max_y: pos.y + half.y,
        };
        bc.count += 1;
        bc.coc = bc.coc.max(strength.map_or(30.0, |strength| strength.0));
    }
}
