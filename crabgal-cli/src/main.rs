// crabgal CLI — dev / check
// GPU 2D rendering via ggez (wgpu backend). fontdue glyph rasterization.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock, mpsc};

use crabgal_core::state::State;
use crabgal_core::step;
use crabgal_core::Action;
use crabgal_core::types::Transition;
use crabgal_script::parser::parse_script;

use crabgal_render::text::TexCache;
use crabgal_core::MenuPanel;

use ggez::conf::{WindowMode, WindowSetup};
use ggez::event::{self, EventHandler};
use ggez::graphics::{self, Canvas, Color, DrawParam, Image, ImageFormat, Mesh, Rect};
use ggez::input::keyboard::{Key, KeyCode};
use ggez::input::mouse::MouseButton;
use ggez::winit::keyboard::{NamedKey, PhysicalKey};
use ggez::{Context, ContextBuilder, GameResult};

pub(crate) struct GameState {
    pub scripts_dir: PathBuf,
    pub state: Arc<RwLock<State>>,
    pub textures: HashMap<String, Image>,
    pub font: fontdue::Font,
    pub fallback_font: fontdue::Font,
    pub icon_font: fontdue::Font,
    pub text_cache: TexCache,
    watcher_rx: mpsc::Receiver<PathBuf>,
    pub tw: f64,
    pub auto: bool,
    pub auto_timer: f64,
    last_size: (u32, u32),
    config_path: PathBuf,
    pub menu_open: bool,
    pub menu_panel: Option<MenuPanel>,
    pub menu_page: u32,
    pub controls_visible: bool,
    pub textbox_visible: bool,
    pub history: Vec<State>,
    pub menu_fade: f32,
    pub skip_mode: bool,
    pub hover_btn: Option<usize>,
    pub dialogue_log: Vec<(String, String)>,
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() < 2 {
        eprintln!("crabgal dev <dir> | check <file>");
        std::process::exit(1);
    }
    match a[1].as_str() {
        "dev" => {
            let dir = a.get(2).map(PathBuf::from).unwrap_or_default();
            if let Err(e) = run_dev(dir) { eprintln!("Error: {e}"); std::process::exit(1); }
        }
        "check" => {
            let p = a.get(2).map(PathBuf::from).unwrap_or_else(|| {
                eprintln!("missing file"); std::process::exit(1);
            });
            if let Err(e) = run_check(&p) { eprintln!("Error: {e}"); std::process::exit(1); }
        }
        _ => std::process::exit(1),
    }
}

fn run_dev(dir: PathBuf) -> anyhow::Result<()> {
    env_logger::init();
    let sd = dir.join("scripts");
    let ar = dir.join("assets");
    if !sd.exists() { std::fs::create_dir_all(&sd)?; }
    let _ = std::fs::create_dir_all(ar.join("fonts"));

    let mut s = State::new();
    load_scenes(&mut s, &sd);
    if s.scenes.is_empty() {
        s.scenes.insert("s".into(), vec![
            Action::ShowBg { image: "bg.webp".into(), transition: Default::default() },
            Action::Say { speaker: "?".into(), text: "no script".into() },
        ]);
    }
    s.current_scene = s.scenes.keys().next().cloned().unwrap_or_default();
    step::index_labels(&mut s);
    step::step(&mut s);
    let state = Arc::new(RwLock::new(s));
    let wrx = crabgal_script::watcher::start_watcher(&sd)?;

    let (primary_font, fallback_font) = load_fonts(ar.join("fonts"))?;

    // Embedded Bootstrap Icons font (woff2)
    let icon_font = {
        let icon_woff2 = include_bytes!("../assets/bootstrap-icons.woff2");
        let ttf = woofwoof::decompress(icon_woff2)
            .ok_or_else(|| anyhow::anyhow!("Failed to decompress Bootstrap Icons woff2"))?;
        fontdue::Font::from_bytes(ttf, fontdue::FontSettings::default())
            .map_err(|e| anyhow::anyhow!("Bootstrap Icons font parse: {e}"))?
    };
    log::info!("Loaded Bootstrap Icons font");
    let cfg_path = dir.join("crabgal.cfg");
    let (win_w, win_h) = load_window_size(&cfg_path);

    let cb = ContextBuilder::new("crabgal", "crabgal")
        .window_setup(WindowSetup::default().title("crabgal"))
        .window_mode(WindowMode::default()
            .dimensions(win_w as f32, win_h as f32)
            .resizable(true));
    let (ctx, event_loop) = cb.build()?;

    let mut textures = HashMap::new();
    for (d, prefix) in &[("background", "bg"), ("figure", "figure")] {
        if let Ok(e) = std::fs::read_dir(ar.join(d)) {
            for entry in e.flatten() {
                let p = entry.path();
                if p.extension().map_or(false, |e| e == "png" || e == "webp" || e == "jpg") {
                    let key = format!("{}/{}", prefix, p.file_name().unwrap().to_string_lossy());
                    match std::fs::read(&p) {
                        Ok(data) => match image::load_from_memory(&data) {
                            Ok(img) => {
                                let rgba = img.to_rgba8();
                                let (w, h) = rgba.dimensions();
                                log::info!("Loaded image {} ({}x{})", key, w, h);
                                let gfx_img = Image::from_pixels(
                                    &ctx, &rgba, ImageFormat::Rgba8UnormSrgb, w, h,
                                );
                                textures.insert(key, gfx_img);
                            }
                            Err(e) => log::warn!("Image decode failed {}: {}", key, e),
                        },
                        Err(e) => log::warn!("Read failed {}: {}", key, e),
                    }
                }
            }
        }
    }

    let gs = GameState {
        scripts_dir: sd, config_path: cfg_path, state, textures,
        font: primary_font, fallback_font, icon_font, text_cache: HashMap::new(),
        watcher_rx: wrx, tw: 0.0, auto: false, auto_timer: 0.0,
        last_size: (win_w, win_h), menu_open: false, menu_panel: None,
        menu_page: 1, controls_visible: true, textbox_visible: true,
        history: Vec::new(), menu_fade: 0.0,
        skip_mode: false, hover_btn: None,
        dialogue_log: Vec::new(),
    };

    let _ = event::run(ctx, event_loop, gs);
    Ok(())
}

// ── EventHandler ──

impl EventHandler for GameState {
    fn update(&mut self, ctx: &mut Context) -> GameResult {
        let (cw, ch) = ctx.gfx.drawable_size();
        let cs = (cw as u32, ch as u32);
        if cs != self.last_size && cs.0 > 0 && cs.1 > 0 {
            self.last_size = cs;
            save_window_size(&self.config_path, cs.0, cs.1);
        }

        while let Ok(_) = self.watcher_rx.try_recv() {
            let mut s = self.state.write().unwrap();
            load_scenes(&mut s, &self.scripts_dir);
            step::index_labels(&mut s);
        }

        let dt = ctx.time.delta().as_secs_f64();
        let skip = ctx.keyboard.is_physical_key_pressed(&PhysicalKey::Code(KeyCode::ControlLeft))
            || ctx.keyboard.is_physical_key_pressed(&PhysicalKey::Code(KeyCode::ControlRight))
            || self.skip_mode;
        let ma = self.menu_open || self.menu_panel.is_some();

        let target: f32 = if ma { 1.0 } else { 0.0 };
        let speed: f32 = 6.67;
        let dt_f32 = dt as f32;
        if (self.menu_fade - target).abs() < 0.01 {
            self.menu_fade = target;
        } else if self.menu_fade < target {
            self.menu_fade = (self.menu_fade + dt_f32 * speed).min(target);
        } else {
            self.menu_fade = (self.menu_fade - dt_f32 * speed).max(target);
        }

        if ctx.keyboard.is_physical_key_just_pressed(&PhysicalKey::Code(KeyCode::PageUp)) && !ma {
            if let Some(prev) = self.history.pop() {
                let mut s = self.state.write().unwrap();
                *s = prev; step::index_labels(&mut s); self.tw = 0.0;
            }
        }

        if ctx.mouse.button_just_pressed(MouseButton::Left) {
            self.handle_click(ctx);
        }

        if ctx.keyboard.is_logical_key_just_pressed(&Key::Named(NamedKey::Space))
            || ctx.keyboard.is_logical_key_just_pressed(&Key::Named(NamedKey::Enter))
        { if !ma { self.advance_action(); } }

        if ctx.keyboard.is_logical_key_just_pressed(&Key::Named(NamedKey::Escape)) {
            if self.menu_panel.is_some() { self.menu_panel = None; }
            else if self.menu_open { self.menu_open = false; }
            else { self.controls_visible = !self.controls_visible; }
        }

        if ctx.keyboard.is_logical_key_just_pressed(&Key::Named(NamedKey::Tab)) {
            self.menu_open = !self.menu_open;
        }

        for (i, ch) in ["1","2","3","4","5","6","7","8","9"].iter().enumerate() {
            if ctx.keyboard.is_logical_key_just_pressed(&Key::Character((*ch).into())) {
                let sh = ctx.keyboard.is_physical_key_pressed(&PhysicalKey::Code(KeyCode::ShiftLeft))
                    || ctx.keyboard.is_physical_key_pressed(&PhysicalKey::Code(KeyCode::ShiftRight));
                let save_dir = self.scripts_dir.parent().unwrap().join("saves");
                let slot = (i + 1) as u32;
                if sh {
                    if let Some(l) = load_from_slot(&save_dir, slot) {
                        let mut s = self.state.write().unwrap();
                        *s = l; step::index_labels(&mut s);
                        self.tw = 0.0; self.history.clear();
                    }
                } else {
                    save_to_slot(&self.state, &save_dir, slot);
                }
            }
        }

        {
            let mut s = self.state.write().unwrap();
            if let Some(ref mut d) = s.dialogue {
                if skip { self.tw = 1.0; }
                else if self.tw < 1.0 { self.tw = (self.tw + dt * 2.0).min(1.0); }
                d.visible_chars = d.text.chars().count();
            } else { self.tw = 0.0; }

            if (skip || self.auto) && self.tw >= 1.0 {
                if skip || self.auto { self.auto_timer += dt; }
                if skip || self.auto_timer > 2.0 {
                    step::advance(&mut s); step::step(&mut s);
                    self.tw = 0.0; self.auto_timer = 0.0;
                }
            }

            for sp in s.sprites.values_mut() {
                if sp.transition_progress < 1.0 || skip {
                    let speed = if sp.transition == Transition::Instant { 999.0 } else { 3.0 };
                    sp.transition_progress = (sp.transition_progress + dt as f32 * speed).min(1.0);
                }
            }
            s.sprites.retain(|_, sp| sp.entering || sp.transition_progress < 1.0);
            if let Some(ref mut t) = s.bg_transition {
                if t.progress < 1.0 || skip {
                    t.progress = (t.progress + dt as f32 * 4.0).min(1.0);
                }
            }
            // Mini avatar enter/exit animation (0.33s, like WebGAL CSS)
            if s.mini_avatar.is_some() && s.mini_avatar_progress < 1.0 {
                s.mini_avatar_progress = (s.mini_avatar_progress + dt as f32 * 3.0).min(1.0);
            }
            if s.mini_avatar.is_none() && s.mini_avatar_progress > 0.0 {
                s.mini_avatar_progress = (s.mini_avatar_progress - dt as f32 * 3.0).max(0.0);
            }
        }
        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        let (ww, wh) = ctx.gfx.drawable_size();
        if ww < 1.0 || wh < 1.0 { return Ok(()); }
        let sg = self.state.read().unwrap();
        let sc = (ww / 2560.0).min(wh / 1440.0);
        let ox = (ww - 2560.0 * sc) / 2.0;
        let oy = (wh - 1440.0 * sc) / 2.0;
        let ds = |v: f32| v * sc;
        let dx = |v: f32| ox + v * sc;
        let dy = |v: f32| oy + v * sc;

        let mut c = Canvas::from_frame(ctx, Color::BLACK);

        if let Some(bg) = &sg.bg {
            let n = std::path::Path::new(bg).file_name()
                .map(|x| x.to_string_lossy().to_string()).unwrap_or_default();
            if let Some(img) = self.textures.get(&format!("bg/{}", n)) {
                let iw = img.width() as f32; let ih = img.height() as f32;
                c.draw(img, DrawParam::new()
                    .dest([ox, oy])
                    .scale([ds(2560.0) / iw, ds(1440.0) / ih]));
            } else {
                let fb = Mesh::new_rectangle(ctx, graphics::DrawMode::fill(),
                    Rect::new(ox, oy, ds(2560.0), ds(1440.0)),
                    Color::new(1.0, 0.0, 0.0, 1.0))?;
                c.draw(&fb, DrawParam::default());
            }
        }

        let mut sp: Vec<_> = sg.sprites.iter().collect();
        sp.sort_by(|a, b| a.1.position.y.partial_cmp(&b.1.position.y)
            .unwrap_or(std::cmp::Ordering::Equal));
        for (_, spr) in &sp {
            let n = std::path::Path::new(&spr.image).file_name()
                .map(|x| x.to_string_lossy().to_string()).unwrap_or_default();
            if let Some(img) = self.textures.get(&format!("figure/{}", n)) {
                let p = spr.transition_progress;
                let alpha = if spr.entering { p } else { 1.0 - p };
                let sh = 960.0; let sw = sh * img.width() as f32 / img.height() as f32;
                let xd = spr.position.x.resolve(sw) + spr.y_offset * (1.0 - p);
                let iw = img.width() as f32; let ih = img.height() as f32;
                let mut dp = DrawParam::new()
                    .dest([dx(xd), dy(480.0)])
                    .scale([ds(sw) / iw, ds(sh) / ih]);
                dp.color.a = alpha;
                c.draw(img, dp);
            }
        }

        let dialogue_text = sg.dialogue.clone();
        let menu_choices = sg.menu.clone();
        let bg_ref = sg.bg.clone();
        let sprites_clone: Vec<_> = sg.sprites.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        let mini_avatar = sg.mini_avatar.clone();
        let mini_avatar_progress = sg.mini_avatar_progress;
        let menu_fade = self.menu_fade;
        drop(sg);

        if dialogue_text.is_some() {
            crabgal_ui::textbox::draw_textbox(
                ctx, &mut c, &self.textures, &mut self.text_cache,
                &self.font, &self.fallback_font,
                &dialogue_text, &bg_ref, &sprites_clone,
                &mini_avatar, mini_avatar_progress,
                self.tw, self.textbox_visible,
                sc, ox, oy, &ds, &dx, &dy,
            )?;
            crabgal_ui::control_bar::draw_control_bar(
                ctx, &mut c, &mut self.text_cache,
                &self.font, &self.fallback_font, &self.icon_font,
                self.controls_visible, self.textbox_visible,
                self.menu_open, self.menu_panel.is_some(),
                &mut self.hover_btn,
                sc, &ds, &dx, &dy,
            )?;
        }

        crabgal_ui::choices::draw_choices(
            ctx, &mut c, &mut self.text_cache,
            &self.font, &self.fallback_font,
            &menu_choices,
            sc, &ds, &dx, &dy,
        )?;

        if menu_fade > 0.001 {
            crabgal_ui::menu::draw_menu_overlay(
                ctx, &mut c, &self.textures, &mut self.text_cache,
                &self.font, &self.fallback_font,
                &bg_ref, &sprites_clone,
                menu_fade, self.menu_panel, self.menu_page,
                &self.dialogue_log,
                sc, ox, oy, &ds, &dx, &dy,
            )?;
        }
        if let Err(e) = c.finish(ctx) {
            log::warn!("Canvas finish failed (window occluded?): {e}");
        }
        Ok(())
    }
}

// ── GameState methods ──

impl GameState {
    fn advance_action(&mut self) {
        let mut s = self.state.write().unwrap();
        if let Some(ref d) = s.dialogue {
            let total = d.text.chars().count();
            if d.visible_chars < total {
                s.dialogue.as_mut().unwrap().visible_chars = total;
                self.tw = 1.0;
            } else {
                self.dialogue_log.push((d.speaker.clone(), d.text.clone()));
                self.history.push(s.clone());
                step::advance(&mut s); step::step(&mut s); self.tw = 0.0;
            }
        } else {
            self.history.push(s.clone());
            step::step(&mut s); self.tw = 0.0;
        }
    }

    fn handle_click(&mut self, ctx: &mut Context) {
        let (ww, wh) = ctx.gfx.drawable_size();
        if ww < 1.0 || wh < 1.0 { return; }
        let mpos = ctx.mouse.position();
        let dsc = (ww / 2560.0).min(wh / 1440.0);
        let dox = (ww - 2560.0 * dsc) / 2.0;
        let doy = (wh - 1440.0 * dsc) / 2.0;
        let dxc = (mpos.x - dox) / dsc;
        let dyc = (mpos.y - doy) / dsc;

        if self.menu_open || self.menu_panel.is_some() {
            let bar_y = 1440.0 * 0.9;
            if dyc >= bar_y {
                if dxc < 650.0 { self.menu_panel = Some(MenuPanel::Save); self.menu_page = 1; }
                else if dxc < 1000.0 { self.menu_panel = Some(MenuPanel::Load); self.menu_page = 1; }
                else if dxc < 1400.0 { self.menu_panel = Some(MenuPanel::Options); }
                else { self.menu_panel = None; self.menu_open = false; }
                return;
            }
            if let Some(panel) = self.menu_panel {
                if panel == MenuPanel::Save || panel == MenuPanel::Load {
                    if dyc < 100.0 {
                        let page = ((dxc - 150.0) / 80.0) as u32 + 1;
                        if page >= 1 && page <= 20 { self.menu_page = page; }
                        return;
                    }
                    let cw = 448.0; let ch = 648.0;
                    for i in 0..10 {
                        let col = i % 5; let row = i / 5;
                        let sx = 120.0 + col as f32 * 500.0;
                        let sy = 130.0 + row as f32 * 678.0;
                        if dxc >= sx && dxc <= sx + cw && dyc >= sy && dyc <= sy + ch {
                            let slot = ((self.menu_page - 1) * 10 + i + 1) as u32;
                            let save_dir = self.scripts_dir.parent().unwrap().join("saves");
                            if panel == MenuPanel::Save {
                                save_to_slot(&self.state, &save_dir, slot);
                            } else if let Some(loaded) = load_from_slot(&save_dir, slot) {
                                let mut s = self.state.write().unwrap();
                                *s = loaded; step::index_labels(&mut s);
                                self.tw = 0.0; self.history.clear();
                            }
                            self.menu_panel = None; self.menu_open = false;
                            return;
                        }
                    }
                }
                self.menu_panel = None;
                return;
            }
            self.menu_open = false;
            return;
        }

        if let Some(idx) = self.hover_btn {
            let sd = self.scripts_dir.parent().unwrap().join("saves");
            match idx {
                0 => { self.menu_panel = Some(MenuPanel::Backlog); self.menu_open = true; }
                1 => { /* Replay */ }
                2 => { self.auto = !self.auto; self.auto_timer = 0.0; }
                3 => { self.skip_mode = !self.skip_mode; }
                4 => { self.textbox_visible = !self.textbox_visible; }
                5 => { self.controls_visible = !self.controls_visible; }
                6 => save_to_slot(&self.state, &sd, 0),
                7 => {
                    if let Some(l) = load_from_slot(&sd, 0) {
                        let mut s = self.state.write().unwrap();
                        *s = l; step::index_labels(&mut s);
                        self.tw = 0.0; self.history.clear();
                    }
                }
                8 => { self.menu_panel = Some(MenuPanel::Save); self.menu_page = 1; self.menu_open = true; }
                9 => { self.menu_panel = Some(MenuPanel::Load); self.menu_page = 1; self.menu_open = true; }
                10 => { self.menu_panel = Some(MenuPanel::Options); self.menu_open = true; }
                11 => { /* Title */ }
                _ => {}
            }
            return;
        }

        let s = self.state.read().unwrap();
        let hm = s.menu.is_some();
        drop(s);
        if hm {
            let iw = 1280.0; let ih = 80.0; let ig = 14.0;
            if dxc >= 640.0 && dxc <= 640.0 + iw {
                let s = self.state.read().unwrap();
                if let Some(ref chs) = s.menu {
                    let sy = 720.0 - (chs.len() as f32 * (ih + ig) - ig) / 2.0;
                    for (i, _) in chs.iter().enumerate() {
                        let cy = sy + i as f32 * (ih + ig);
                        if dyc >= cy && dyc <= cy + ih {
                            drop(s);
                            let mut s = self.state.write().unwrap();
                            step::select_choice(&mut s, i);
                            step::step(&mut s); self.tw = 0.0;
                            return;
                        }
                    }
                }
            }
            return;
        }

        {
            let s = self.state.read().unwrap();
            let hd = s.dialogue.is_some();
            drop(s);
            if hd && self.controls_visible {
                self.advance_action();
            }
        }
    }
}

// ── save/load ──

fn save_to_slot(state: &Arc<RwLock<State>>, save_dir: &std::path::Path, slot: u32) {
    let s = state.read().unwrap();
    let _ = std::fs::create_dir_all(save_dir);
    let path = save_dir.join(format!("slot_{}.bin", slot));
    if let Ok(data) = bincode::serialize(&*s) {
        if let Err(e) = std::fs::write(&path, data) {
            log::error!("Save failed slot {}: {}", slot, e);
        } else {
            log::info!("Saved slot {}", slot);
        }
    }
}

fn load_from_slot(save_dir: &std::path::Path, slot: u32) -> Option<State> {
    let path = save_dir.join(format!("slot_{}.bin", slot));
    if let Ok(data) = std::fs::read(&path) {
        if let Ok(loaded) = bincode::deserialize::<State>(&data) {
            log::info!("Loaded slot {}", slot);
            return Some(loaded);
        }
    }
    None
}

// ── helpers ──

fn load_scenes(s: &mut State, d: &std::path::Path) {
    s.scenes.clear();
    if let Ok(e) = std::fs::read_dir(d) {
        for entry in e.flatten() {
            let p = entry.path();
            let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("");
            if ext == "crab" || ext == "txt" {
                let n = p.file_stem().unwrap().to_string_lossy().to_string();
                if let Ok(c) = std::fs::read_to_string(&p) {
                    let actions = if ext == "txt" {
                        crabgal_script::parse_webgal(&c)
                    } else {
                        parse_script(&c)
                    };
                    s.scenes.insert(n.clone(), actions);
                    if s.current_scene.is_empty() { s.current_scene = n; }
                }
            }
        }
    }
}

fn run_check(p: &std::path::Path) -> anyhow::Result<()> {
    let c = std::fs::read_to_string(p)?;
    let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("");
    let actions = if ext == "txt" { crabgal_script::parse_webgal(&c) } else { parse_script(&c) };
    for (i, a) in actions.iter().enumerate() { println!("{:3}: {:?}", i + 1, a); }
    Ok(())
}

fn load_fonts(fonts_dir: std::path::PathBuf) -> anyhow::Result<(fontdue::Font, fontdue::Font)> {
    let mut primary: Option<fontdue::Font> = None;
    let mut fallback: Option<fontdue::Font> = None;
    if let Ok(e) = std::fs::read_dir(&fonts_dir) {
        for entry in e.flatten() {
            let p = entry.path();
            let name = p.file_stem().unwrap_or_default().to_string_lossy().to_lowercase();
            if let Ok(data) = std::fs::read(&p) {
                let ttf = if p.extension().and_then(|s| s.to_str()) == Some("woff2") {
                    match woofwoof::decompress(&data) {
                        Some(d) => d,
                        None => { log::warn!("WOFF2 decode failed: {}", p.display()); continue; }
                    }
                } else { data };
                if let Ok(font) = fontdue::Font::from_bytes(ttf, fontdue::FontSettings::default()) {
                    log::info!("Loaded font: {} ({} chars)", p.display(), font.chars().len());
                    if name.contains("maven") && fallback.is_none() { fallback = Some(font); }
                    else if primary.is_none() { primary = Some(font); }
                }
            }
        }
    }
    match (primary, fallback) {
        (Some(p), Some(f)) => Ok((p, f)),
        (Some(p), None) => { let f = p.clone(); Ok((p, f)) },
        (None, Some(f)) => { let p = f.clone(); Ok((p, f)) },
        (None, None) => {
            let sys = std::path::Path::new("/System/Library/Fonts/Supplemental/Arial Unicode.ttf");
            if sys.exists() {
                log::info!("Using system font");
                let data = std::fs::read(sys)?;
                let font = fontdue::Font::from_bytes(data, fontdue::FontSettings::default())
                    .map_err(|e| anyhow::anyhow!("System font parse: {e}"))?;
                let f2 = font.clone();
                return Ok((font, f2));
            }
            anyhow::bail!("No font found")
        }
    }
}

fn load_window_size(cfg_path: &std::path::Path) -> (u32, u32) {
    if let Ok(data) = std::fs::read_to_string(cfg_path) {
        let parts: Vec<u32> = data.split_whitespace().filter_map(|s| s.parse().ok()).collect();
        if parts.len() == 2 { return (parts[0], parts[1]); }
    }
    (2560, 1440)
}

fn save_window_size(cfg_path: &std::path::Path, w: u32, h: u32) {
    let s = format!("{w} {h}");
    let _ = std::fs::write(cfg_path, s);
}
