use std::collections::HashSet;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use bevy::app::AppExit;
use bevy::ecs::system::NonSendMarker;
use bevy::prelude::*;
use bevy::window::{WindowPosition, WindowResized};
use serde::Deserialize;

const DEFAULT_RESIZE_SETTLE_MS: u64 = 120;
const MIN_RESIZE_SETTLE_MS: u64 = 50;
const MAX_RESIZE_SETTLE_MS: u64 = 500;
const RESIZE_REVEAL_DELAY: Duration = Duration::from_millis(50);
const RESIZE_RENDER_DRAIN_DELAY: Duration = Duration::from_millis(50);
const RESIZE_CONFIRM_FALLBACK: Duration = Duration::from_millis(150);

pub(crate) struct EditorBridgePlugin {
    port: u16,
    embedded: bool,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct InitialEditorFrame {
    pub(crate) position: IVec2,
    pub(crate) size: UVec2,
    pub(crate) scale_factor: f32,
}

#[derive(Resource, Default)]
pub(crate) struct EditorOverlay;

impl EditorBridgePlugin {
    pub(crate) const fn new(port: u16, embedded: bool) -> Self {
        Self { port, embedded }
    }
}

impl Plugin for EditorBridgePlugin {
    fn build(&self, app: &mut App) {
        let initial_size = initial_editor_frame().map(|frame| frame.size);
        let receiver = start_server(self.port)
            .unwrap_or_else(|error| panic!("failed to start editor bridge: {error:#}"));
        app.init_resource::<EditorOverlay>()
            .insert_resource(EditorBridge {
                embedded: self.embedded,
                receiver: Arc::new(Mutex::new(receiver)),
                frame: None,
                connected: false,
                last_seen: Instant::now(),
                restart_requested: None,
                pending_size: initial_size,
                pending_since: Instant::now(),
                committed_size: initial_size,
                confirmed_size: initial_size,
                resize_armed: None,
                awaiting_resize: None,
                awaiting_since: None,
                reveal_after: Instant::now(),
                suspended_cameras: HashSet::new(),
            });
        if self.embedded {
            app.add_systems(Startup, configure_nonactivating_overlay);
        }
        app.add_systems(PreUpdate, (sync_window, restart_preview).chain());
    }
}

fn configure_nonactivating_overlay(_main_thread: NonSendMarker) {
    #[cfg(target_os = "macos")]
    {
        use objc2::MainThreadMarker;
        use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};

        let Some(main_thread) = MainThreadMarker::new() else {
            log::warn!("editor overlay activation policy must be set on the main thread");
            return;
        };
        let application = NSApplication::sharedApplication(main_thread);
        if !application.setActivationPolicy(NSApplicationActivationPolicy::Prohibited) {
            log::warn!("macOS rejected the non-activating editor overlay policy");
        }
    }
}

pub(crate) fn initial_editor_frame() -> Option<InitialEditorFrame> {
    let encoded = std::env::var("CRABGAL_EDITOR_FRAME").ok()?;
    let frame = serde_json::from_str::<BridgeFrame>(&encoded).ok()?;
    Some(InitialEditorFrame {
        position: IVec2::new(frame.x, frame.y),
        size: frame.size(),
        scale_factor: frame.scale_factor(),
    })
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BridgeFrame {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    #[serde(default = "default_scale_factor")]
    scale_factor: f32,
    visible: bool,
    #[serde(default, alias = "studioFocused")]
    host_focused: bool,
    #[serde(default = "default_resize_settle_ms")]
    resize_settle_ms: u64,
}

const fn default_resize_settle_ms() -> u64 {
    DEFAULT_RESIZE_SETTLE_MS
}

const fn default_scale_factor() -> f32 {
    1.0
}

impl BridgeFrame {
    fn size(self) -> UVec2 {
        UVec2::new(self.width.max(1), self.height.max(1))
    }

    fn resize_settle(self) -> Duration {
        Duration::from_millis(
            self.resize_settle_ms
                .clamp(MIN_RESIZE_SETTLE_MS, MAX_RESIZE_SETTLE_MS),
        )
    }

    fn scale_factor(self) -> f32 {
        if self.scale_factor.is_finite() {
            self.scale_factor.clamp(0.5, 8.0)
        } else {
            1.0
        }
    }
}

enum BridgeEvent {
    Connected,
    Frame(BridgeFrame),
    Restart(Option<BridgeCursor>),
    Disconnected,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BridgeCursor {
    scene: String,
    block_index: usize,
}

impl BridgeCursor {
    const fn source_step(&self) -> usize {
        self.block_index.saturating_add(1)
    }
}

#[derive(Debug, Clone)]
struct RestartRequest {
    cursor: Option<BridgeCursor>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
enum BridgeCommand {
    Restart,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum BridgePacket {
    Frame(BridgeFrame),
    Command {
        command: BridgeCommand,
        #[serde(default)]
        cursor: Option<BridgeCursor>,
    },
}

#[derive(Resource)]
struct EditorBridge {
    embedded: bool,
    receiver: Arc<Mutex<mpsc::Receiver<BridgeEvent>>>,
    frame: Option<BridgeFrame>,
    connected: bool,
    last_seen: Instant,
    restart_requested: Option<RestartRequest>,
    pending_size: Option<UVec2>,
    pending_since: Instant,
    committed_size: Option<UVec2>,
    confirmed_size: Option<UVec2>,
    resize_armed: Option<(UVec2, Instant)>,
    awaiting_resize: Option<UVec2>,
    awaiting_since: Option<Instant>,
    reveal_after: Instant,
    suspended_cameras: HashSet<Entity>,
}

impl EditorBridge {
    fn observe_frame(&mut self, frame: BridgeFrame, now: Instant) {
        let size = frame.size();
        if self.pending_size != Some(size) {
            self.pending_size = Some(size);
            self.pending_since = now;
            self.resize_armed = None;
            self.reveal_after = now + frame.resize_settle() + RESIZE_REVEAL_DELAY;
        }
        self.frame = Some(frame);
    }
}

fn start_server(port: u16) -> Result<mpsc::Receiver<BridgeEvent>> {
    let listener = TcpListener::bind(("127.0.0.1", port))
        .with_context(|| format!("127.0.0.1:{port} is already in use"))?;
    let (sender, receiver) = mpsc::channel();
    std::thread::Builder::new()
        .name("crabgal-editor-bridge".into())
        .spawn(move || {
            for stream in listener.incoming() {
                match stream {
                    Ok(stream) => read_client(stream, &sender),
                    Err(error) => log::warn!("editor bridge connection failed: {error}"),
                }
            }
        })
        .context("failed to spawn editor bridge thread")?;
    Ok(receiver)
}

fn read_client(mut stream: TcpStream, sender: &mpsc::Sender<BridgeEvent>) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(3)));
    let reader_stream = match stream.try_clone() {
        Ok(stream) => stream,
        Err(error) => {
            log::warn!("failed to clone editor bridge connection: {error}");
            return;
        }
    };
    let mut reader = BufReader::new(reader_stream);
    let mut first_line = String::new();
    if reader.read_line(&mut first_line).is_err() || first_line.is_empty() {
        return;
    }
    if first_line.contains(" HTTP/1.") {
        read_http_client(&mut stream, &mut reader, &first_line, sender);
        return;
    }

    let _ = sender.send(BridgeEvent::Connected);
    read_bridge_line(first_line.trim(), sender);
    for line in reader.lines() {
        match line {
            Ok(line) if line.trim().is_empty() => continue,
            Ok(line) => read_bridge_line(&line, sender),
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                continue;
            }
            Err(_) => break,
        }
    }
    let _ = sender.send(BridgeEvent::Disconnected);
}

fn read_bridge_line(line: &str, sender: &mpsc::Sender<BridgeEvent>) {
    if line.trim().is_empty() {
        return;
    }
    match serde_json::from_str::<BridgePacket>(line) {
        Ok(BridgePacket::Frame(frame)) => {
            let _ = sender.send(BridgeEvent::Frame(frame));
        }
        Ok(BridgePacket::Command {
            command: BridgeCommand::Restart,
            cursor,
        }) => {
            let _ = sender.send(BridgeEvent::Restart(cursor));
        }
        Err(error) => log::warn!("invalid editor bridge frame: {error}"),
    }
}

fn read_http_client(
    stream: &mut TcpStream,
    reader: &mut BufReader<TcpStream>,
    request_line: &str,
    sender: &mpsc::Sender<BridgeEvent>,
) {
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default();
    let path = parts.next().unwrap_or_default();
    let mut content_length = 0usize;
    loop {
        let mut header = String::new();
        match reader.read_line(&mut header) {
            Ok(0) | Err(_) => return,
            Ok(_) if header == "\r\n" || header == "\n" => break,
            Ok(_) => {
                if let Some((name, value)) = header.split_once(':')
                    && name.eq_ignore_ascii_case("content-length")
                {
                    content_length = value.trim().parse().unwrap_or(0);
                }
            }
        }
    }

    if method == "OPTIONS" {
        write_http_response(stream, "204 No Content", "", "text/plain");
        return;
    }

    const MAX_HTTP_BODY: usize = 64 * 1024;
    if content_length > MAX_HTTP_BODY {
        write_http_response(stream, "413 Payload Too Large", "", "text/plain");
        return;
    }
    let mut body = vec![0; content_length];
    if content_length > 0 && reader.read_exact(&mut body).is_err() {
        write_http_response(stream, "400 Bad Request", "", "text/plain");
        return;
    }

    match (method, path) {
        ("GET", "/v1/status") => {
            write_http_response(stream, "200 OK", r#"{"ok":true}"#, "application/json");
        }
        ("POST", "/v1/heartbeat") => {
            let _ = sender.send(BridgeEvent::Connected);
            write_http_response(stream, "204 No Content", "", "text/plain");
        }
        ("POST", "/v1/restart") => {
            let cursor = serde_json::from_slice::<BridgeCursor>(&body).ok();
            let _ = sender.send(BridgeEvent::Connected);
            let _ = sender.send(BridgeEvent::Restart(cursor));
            write_http_response(stream, "204 No Content", "", "text/plain");
        }
        _ => write_http_response(stream, "404 Not Found", "", "text/plain"),
    }
}

fn write_http_response(stream: &mut TcpStream, status: &str, body: &str, content_type: &str) {
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type\r\nAccess-Control-Allow-Private-Network: true\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
}

fn sync_window(
    mut bridge: ResMut<EditorBridge>,
    mut window: Single<&mut Window>,
    mut resized: MessageReader<WindowResized>,
    mut cameras: Query<(Entity, &mut Camera)>,
    mut exit: MessageWriter<AppExit>,
) {
    let now = Instant::now();
    let events = bridge
        .receiver
        .lock()
        .map(|receiver| receiver.try_iter().collect::<Vec<_>>())
        .unwrap_or_default();
    for event in events {
        bridge.last_seen = now;
        match event {
            BridgeEvent::Connected => bridge.connected = true,
            BridgeEvent::Frame(frame) if bridge.embedded => {
                bridge.connected = true;
                bridge.observe_frame(frame, now);
            }
            BridgeEvent::Frame(_) => bridge.connected = true,
            BridgeEvent::Restart(cursor) => {
                bridge.restart_requested = Some(RestartRequest { cursor });
            }
            BridgeEvent::Disconnected => bridge.connected = false,
        }
    }

    if !bridge.embedded {
        if bridge.last_seen.elapsed() > Duration::from_secs(10) {
            bridge.connected = false;
            exit.write(AppExit::Success);
        }
        return;
    }

    // A requested resize only becomes render-safe after the window backend has
    // acknowledged it. WindowResolution changes immediately in the ECS world,
    // while macOS may deliver its WindowResized event on the next event-loop
    // turn. Rendering in between lets the new swapchain race the previous
    // camera depth attachment.
    if resized.read().next().is_some() {
        let actual = UVec2::new(
            window.resolution.physical_width(),
            window.resolution.physical_height(),
        );
        if bridge.awaiting_resize == Some(actual) {
            bridge.awaiting_resize = None;
            bridge.awaiting_since = None;
            bridge.confirmed_size = Some(actual);
            bridge.reveal_after = now + RESIZE_REVEAL_DELAY;
            log::debug!(
                "Editor preview resize confirmed · {}×{}",
                actual.x,
                actual.y
            );
        }
    }

    if let (Some(target), Some(since)) = (bridge.awaiting_resize, bridge.awaiting_since) {
        let actual = UVec2::new(
            window.resolution.physical_width(),
            window.resolution.physical_height(),
        );
        if actual == target && now.saturating_duration_since(since) >= RESIZE_CONFIRM_FALLBACK {
            bridge.awaiting_resize = None;
            bridge.awaiting_since = None;
            bridge.confirmed_size = Some(target);
            bridge.reveal_after = now + RESIZE_REVEAL_DELAY;
            log::debug!(
                "Editor preview resize confirmed by fallback · {}×{}",
                target.x,
                target.y
            );
        }
    }

    if bridge.resize_armed.is_some()
        || bridge.awaiting_resize.is_some()
        || !bridge.suspended_cameras.is_empty()
    {
        suspend_cameras(&mut cameras, &mut bridge.suspended_cameras);
    }

    if let Some(frame) = bridge.frame {
        let size = frame.size();
        let position = IVec2::new(frame.x, frame.y);
        if window.position != WindowPosition::At(position) {
            window.position = WindowPosition::At(position);
        }

        // An editor can change the preview rectangle several times while opening or
        // rearranging panes. Applying every intermediate size can make winit's
        // surface advance one frame ahead of Bevy's depth target. Coalesce the
        // burst and expose the overlay only after one stable resize has landed.
        let resize_ready = bridge.pending_size == Some(size)
            && now.saturating_duration_since(bridge.pending_since) >= frame.resize_settle();
        if resize_ready && bridge.committed_size != Some(size) && bridge.awaiting_resize.is_none() {
            let current = UVec2::new(
                window.resolution.physical_width(),
                window.resolution.physical_height(),
            );
            if current == size && bridge.awaiting_resize.is_none() {
                bridge.committed_size = Some(size);
                bridge.confirmed_size = Some(size);
                bridge.resize_armed = None;
                bridge.reveal_after = now + RESIZE_REVEAL_DELAY;
            } else if matches!(bridge.resize_armed, Some((target, ready_at)) if target == size && now >= ready_at)
            {
                suspend_cameras(&mut cameras, &mut bridge.suspended_cameras);
                window.visible = false;
                window.resolution.set_physical_resolution(size.x, size.y);
                bridge.committed_size = Some(size);
                bridge.confirmed_size = None;
                bridge.resize_armed = None;
                bridge.awaiting_resize = Some(size);
                bridge.awaiting_since = Some(now);
                bridge.reveal_after = now + frame.resize_settle() + RESIZE_REVEAL_DELAY;
                log::debug!("Editor preview resize requested · {}×{}", size.x, size.y);
            } else if !matches!(bridge.resize_armed, Some((target, _)) if target == size) {
                suspend_cameras(&mut cameras, &mut bridge.suspended_cameras);
                window.visible = false;
                bridge.confirmed_size = None;
                bridge.resize_armed = Some((size, now + RESIZE_RENDER_DRAIN_DELAY));
            }
        }

        let resize_landed = bridge.committed_size == Some(size)
            && bridge.confirmed_size == Some(size)
            && bridge.resize_armed.is_none()
            && bridge.awaiting_resize.is_none()
            && now >= bridge.reveal_after;
        if resize_landed {
            restore_cameras(&mut cameras, &mut bridge.suspended_cameras);
        }
        let visible = bridge.connected && frame.visible && frame.host_focused && resize_landed;
        if window.visible != visible {
            window.visible = visible;
        }
    }

    if bridge.last_seen.elapsed() > Duration::from_secs(10) {
        bridge.connected = false;
        exit.write(AppExit::Success);
    }
}

fn suspend_cameras(cameras: &mut Query<(Entity, &mut Camera)>, suspended: &mut HashSet<Entity>) {
    for (entity, mut camera) in cameras.iter_mut() {
        if camera.is_active {
            camera.is_active = false;
            suspended.insert(entity);
        }
    }
}

fn restore_cameras(cameras: &mut Query<(Entity, &mut Camera)>, suspended: &mut HashSet<Entity>) {
    for (entity, mut camera) in cameras.iter_mut() {
        if suspended.remove(&entity) {
            camera.is_active = true;
        }
    }
    suspended.clear();
}

fn restart_preview(
    mut bridge: ResMut<EditorBridge>,
    content: Res<crate::runtime::resources::ContentProjectResource>,
    manifest: Res<crate::runtime::resources::LocalAssetManifest>,
    mut state: ResMut<crate::runtime::resources::GameState>,
) {
    let Some(request) = bridge.restart_requested.take() else {
        return;
    };
    let restarted = match request.cursor.as_ref() {
        Some(cursor) => crate::runtime::tick::restart_editor_position(
            &mut state,
            &manifest,
            &cursor.scene,
            cursor.source_step(),
        ),
        None => crate::runtime::tick::restart_editor_cursor(&content, &mut state, &manifest),
    };
    if restarted {
        log::info!("Editor host requested a native preview restart");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bridge_frame_is_a_small_stable_json_contract() {
        let packet: BridgePacket = serde_json::from_str(
            r#"{"x":100,"y":200,"width":1280,"height":720,"visible":true,"hostFocused":false}"#,
        )
        .unwrap();
        let BridgePacket::Frame(frame) = packet else {
            panic!("expected frame packet");
        };
        assert_eq!(
            (frame.x, frame.y, frame.width, frame.height),
            (100, 200, 1280, 720)
        );
        assert!(frame.visible);
        assert!(!frame.host_focused);
        assert_eq!(frame.resize_settle_ms, DEFAULT_RESIZE_SETTLE_MS);
    }

    #[test]
    fn bridge_resize_delay_is_bounded() {
        let mut frame: BridgeFrame = serde_json::from_str(
            r#"{"x":0,"y":0,"width":1280,"height":720,"visible":true,"hostFocused":true,"resizeSettleMs":1}"#,
        )
        .unwrap();
        assert_eq!(frame.resize_settle(), Duration::from_millis(50));
        frame.resize_settle_ms = 10_000;
        assert_eq!(frame.resize_settle(), Duration::from_millis(500));
    }

    #[test]
    fn bridge_restart_command_is_backwards_compatible_with_plain_frames() {
        let packet: BridgePacket = serde_json::from_str(r#"{"command":"restart"}"#).unwrap();
        assert!(matches!(
            packet,
            BridgePacket::Command {
                command: BridgeCommand::Restart,
                cursor: None,
            }
        ));
    }

    #[test]
    fn bridge_restart_can_carry_the_live_studio_cursor() {
        let packet: BridgePacket = serde_json::from_str(
            r#"{"command":"restart","cursor":{"scene":"fragment-1","blockIndex":7}}"#,
        )
        .unwrap();
        let BridgePacket::Command {
            command: BridgeCommand::Restart,
            cursor: Some(cursor),
        } = packet
        else {
            panic!("expected restart cursor");
        };
        assert_eq!(cursor.scene, "fragment-1");
        assert_eq!(cursor.block_index, 7);
        assert_eq!(cursor.source_step(), 8);
    }

    #[test]
    fn initial_frame_uses_the_same_packet_shape_as_live_updates() {
        let frame: BridgeFrame = serde_json::from_str(
            r#"{"x":12,"y":34,"width":1600,"height":900,"visible":false,"hostFocused":true}"#,
        )
        .unwrap();
        assert_eq!(frame.size(), UVec2::new(1600, 900));
        assert_eq!(IVec2::new(frame.x, frame.y), IVec2::new(12, 34));
    }

    #[test]
    fn sdk_http_restart_maps_to_the_small_editor_protocol() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let (sender, receiver) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            read_client(stream, &sender);
        });

        let mut client = TcpStream::connect(address).unwrap();
        client
            .write_all(
                b"POST /v1/restart HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: text/plain\r\nContent-Length: 37\r\n\r\n{\"scene\":\"fragment-1\",\"blockIndex\":7}",
            )
            .unwrap();
        let mut response = String::new();
        client.read_to_string(&mut response).unwrap();
        worker.join().unwrap();

        assert!(response.starts_with("HTTP/1.1 204 No Content"));
        assert!(matches!(receiver.recv().unwrap(), BridgeEvent::Connected));
        let BridgeEvent::Restart(Some(cursor)) = receiver.recv().unwrap() else {
            panic!("expected HTTP restart cursor");
        };
        assert_eq!(cursor.scene, "fragment-1");
        assert_eq!(cursor.block_index, 7);
    }
}
