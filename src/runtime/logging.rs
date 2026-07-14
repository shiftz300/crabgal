use std::fmt;
use std::time::Instant;

use anyhow::Error;
use bevy::log::{BoxedFmtLayer, Level, LogPlugin, tracing_subscriber};
use bevy::prelude::*;
use bevy::render::batching::gpu_preprocessing::{GpuPreprocessingMode, GpuPreprocessingSupport};
use bevy::render::renderer::RenderAdapterInfo;
use bevy::render::{Render, RenderApp};
use bevy::window::PrimaryWindow;

// Keep the default console focused on the game and its content. RUST_LOG still
// overrides this completely when deeper engine or renderer diagnostics are
// needed.
const DEFAULT_FILTER: &str = concat!(
    "warn,",
    "crabgal=info,",
    "crabgal_core=info,",
    "crabgal_loader=info,",
    "wgpu=error,",
    "naga=warn"
);

pub(super) fn plugin() -> LogPlugin {
    LogPlugin {
        filter: DEFAULT_FILTER.into(),
        level: Level::INFO,
        fmt_layer: compact_layer,
        ..Default::default()
    }
}

pub(super) fn install_runtime_diagnostics(app: &mut App) {
    app.add_systems(PostStartup, log_window);
    if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
        render_app.add_systems(Render, log_renderer.run_if(run_once));
    }
}

fn compact_layer(_: &mut App) -> Option<BoxedFmtLayer> {
    let layer = tracing_subscriber::fmt::layer()
        .with_timer(ShortUptime::now())
        .compact()
        .with_target(true)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_writer(std::io::stderr);
    Some(Box::new(layer))
}

struct ShortUptime(Instant);

impl ShortUptime {
    fn now() -> Self {
        Self(Instant::now())
    }
}

impl tracing_subscriber::fmt::time::FormatTime for ShortUptime {
    fn format_time(&self, writer: &mut tracing_subscriber::fmt::format::Writer<'_>) -> fmt::Result {
        write!(writer, "{:>8.3}s", self.0.elapsed().as_secs_f64())
    }
}

pub(super) fn startup_error(stage: &str, error: &Error) {
    eprintln!("ERROR  crabgal::startup: {stage}");
    for (index, cause) in error.chain().enumerate() {
        eprintln!("       {:>2}. {cause}", index + 1);
    }
}

fn log_window(window: Single<&Window, With<PrimaryWindow>>) {
    let width = window.resolution.width().round() as u32;
    let height = window.resolution.height().round() as u32;
    let scale = window.resolution.scale_factor();
    let resize = if window.resizable {
        "resizable"
    } else {
        "fixed"
    };
    log::info!(
        target: "crabgal::platform",
        "WINDOW   │ {} · {width}×{height} @{scale:.1}× · {resize}",
        window.title,
    );
}

fn log_renderer(adapter: Res<RenderAdapterInfo>, preprocessing: Res<GpuPreprocessingSupport>) {
    let transient_memory = if adapter.transient_saves_memory {
        " · transient memory ✓"
    } else {
        ""
    };
    log::info!(
        target: "crabgal::platform",
        "GPU      │ {} · {:?} · {:?} · subgroup {}–{}{transient_memory}",
        adapter.name,
        adapter.device_type,
        adapter.backend,
        adapter.subgroup_min_size,
        adapter.subgroup_max_size,
    );

    let mode = match preprocessing.max_supported_mode {
        GpuPreprocessingMode::None => "CPU fallback",
        GpuPreprocessingMode::PreprocessingOnly => "GPU preprocessing ✓",
        GpuPreprocessingMode::Culling => "GPU preprocessing + culling ✓",
    };
    log::info!(target: "crabgal::platform", "PIPELINE │ {mode}");
}
