// crabgal CLI — dev / check
// GPU 2D rendering via notan (Metal backend). fontdue glyph rasterization.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock, mpsc};

use crabgal_core::state::State;
use crabgal_core::step;
use crabgal_core::Action;
use crabgal_core::types::Transition;
use crabgal_script::parser::parse_script;

use notan::draw::*;
use notan::prelude::*;

#[derive(AppState)]
struct Ctx {
    scripts_dir: PathBuf,
    state: Arc<RwLock<State>>,
    textures: HashMap<String, Texture>,
    font: fontdue::Font,
    fallback_font: fontdue::Font,
    text_cache: HashMap<(String, u32, u32), (Texture, u32, u32)>,
    watcher_rx: mpsc::Receiver<PathBuf>,
    tw: f64,
    auto: bool,
    auto_timer: f64,
    last_size: (u32, u32),
    config_path: PathBuf,
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
            let p = a.get(2).map(PathBuf::from).unwrap_or_else(|| { eprintln!("missing file"); std::process::exit(1); });
            if let Err(e) = run_check(&p) { eprintln!("Error: {e}"); std::process::exit(1); }
        }
        _ => std::process::exit(1),
    }
}

fn run_dev(dir: PathBuf) -> anyhow::Result<()> {
    let sd = dir.join("scripts");
    let ar = dir.join("assets");
    if !sd.exists() { std::fs::create_dir_all(&sd)?; }
    let _ = std::fs::create_dir_all(ar.join("fonts"));

    // ── Scripts ──
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

    // ── Pre-load images: decode to RGBA via image crate ──
    let mut raw_images: HashMap<String, (Vec<u8>, u32, u32)> = HashMap::new();
    for (d, prefix) in &[("background","bg"),("figure","figure")] {
        if let Ok(e) = std::fs::read_dir(ar.join(d)) {
            for entry in e.flatten() {
                let p = entry.path();
                if p.extension().map_or(false, |e| e == "png" || e == "webp" || e == "jpg") {
                    if let Ok(img) = image::load_from_memory(&std::fs::read(&p)?) {
                        let rgba = img.to_rgba8();
                        let (w, h) = rgba.dimensions();
                        let key = format!("{}/{}", prefix, p.file_name().unwrap().to_string_lossy());
                        raw_images.insert(key, (rgba.into_raw(), w, h));
                    }
                }
            }
        }
    }

    // ── Load font + window config ──
    let (primary_font, fallback_font) = load_fonts(ar.join("fonts"))?;
    let cfg_path = dir.join("crabgal.cfg");
    let (win_w, win_h) = load_window_size(&cfg_path);

    let win = WindowConfig::new()
        .set_size(win_w, win_h)
        .set_title("crabgal")
        .set_resizable(true)
        .set_vsync(true);

    notan::init_with(move |gfx: &mut Graphics| -> Ctx {
        let mut textures = HashMap::new();
        for (key, (rgba, w, h)) in &raw_images {
            match gfx.create_texture().from_bytes(rgba, *w, *h).build() {
                Ok(tex) => { textures.insert(key.clone(), tex); }
                Err(e) => { log::error!("Texture failed [{}]: {e:?}", key); }
            }
        }
        log::info!("Loaded {} textures", textures.len());
        Ctx { scripts_dir: sd, config_path: cfg_path, state, textures, font: primary_font, fallback_font, text_cache: HashMap::new(), watcher_rx: wrx, tw: 0.0, auto: false, auto_timer: 0.0, last_size: (win_w, win_h) }
    })
    .add_config(win)
    .add_config(DrawConfig)
    .update(update)
    .draw(draw)
    .build()
    .map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(())
}

// ── Update: input + animations ──
fn update(app: &mut App, ctx: &mut Ctx) {
    // Save window size on change
    let (cw, ch) = app.window().size();
    let cs = (cw as u32, ch as u32);
    if cs != ctx.last_size && cs.0 > 0 && cs.1 > 0 {
        ctx.last_size = cs;
        save_window_size(&ctx.config_path, cs.0, cs.1);
    }

    // Hot reload
    while let Ok(_) = ctx.watcher_rx.try_recv() {
        let mut s = ctx.state.write().unwrap();
        load_scenes(&mut s, &ctx.scripts_dir);
        step::index_labels(&mut s);
    }

    // Keyboard + mouse → advance
    let advance = |ctx: &mut Ctx| {
        let mut s = ctx.state.write().unwrap();
        if let Some(ref d) = s.dialogue {
            let total = d.text.chars().count();
            if d.visible_chars < total {
                // Skip fade: show all text instantly
                s.dialogue.as_mut().unwrap().visible_chars = total;
                ctx.tw = 1.0;
            } else {
                step::advance(&mut s);
                step::step(&mut s);
                ctx.tw = 0.0; // Start fade for new dialogue
            }
        } else {
            step::step(&mut s);
            ctx.tw = 0.0;
        }
    };

    // Mouse click → advance or select menu choice
    if app.mouse.left_was_pressed() {
        let s = ctx.state.read().unwrap();
        let has_menu = s.menu.is_some();
        drop(s); // release lock

        if has_menu {
            // Convert screen mouse coords to design coords
            let (mx, my) = (app.mouse.x, app.mouse.y);
            let (ww, wh) = app.window().size();
            let dsc = (ww as f32 / 2560.0).min(wh as f32 / 1440.0);
            let dox = (ww as f32 - 2560.0 * dsc) / 2.0;
            let doy = (wh as f32 - 1440.0 * dsc) / 2.0;
            let dx = (mx - dox) / dsc;
            let dy = (my - doy) / dsc;
            // Check if click hits a menu choice (centered 1280px wide, 80px tall)
            let item_w = 1280.0;
            let item_h = 80.0;
            let item_gap = 14.0;
            if dx >= 640.0 && dx <= 640.0 + item_w {
                let s = ctx.state.read().unwrap();
                if let Some(ref chs) = s.menu {
                    let start_y = 720.0 - (chs.len() as f32 * (item_h + item_gap) - item_gap) / 2.0;
                    for (i, _) in chs.iter().enumerate() {
                        let cy = start_y + i as f32 * (item_h + item_gap);
                        if dy >= cy && dy <= cy + item_h {
                            drop(s);
                            let mut s = ctx.state.write().unwrap();
                            step::select_choice(&mut s, i);
                            step::step(&mut s);
                            ctx.tw = 0.0;
                            break;
                        }
                    }
                }
            }
        } else {
            advance(ctx);
        }
    }
    // Keyboard → advance
    if app.keyboard.was_pressed(KeyCode::Space) || app.keyboard.was_pressed(KeyCode::Enter) {
        advance(ctx);
    }
    if app.keyboard.was_pressed(KeyCode::Escape) { app.exit(); }
    if app.keyboard.was_pressed(KeyCode::KeyA) {
        ctx.auto = !ctx.auto;
        ctx.auto_timer = 0.0;
        log::info!("Auto mode: {}", if ctx.auto { "ON" } else { "OFF" });
    }
    if app.keyboard.was_pressed(KeyCode::F5) {
        let s = ctx.state.read().unwrap();
        let path = ctx.scripts_dir.parent().unwrap().join("saves").join("quicksave.bin");
        let _ = std::fs::create_dir_all(path.parent().unwrap());
        if let Ok(data) = bincode::serialize(&*s) {
            if std::fs::write(&path, data).is_ok() {
                log::info!("Quick saved");
            }
        }
    }
    if app.keyboard.was_pressed(KeyCode::F6) {
        let path = ctx.scripts_dir.parent().unwrap().join("saves").join("quicksave.bin");
        if let Ok(data) = std::fs::read(&path) {
            if let Ok(loaded) = bincode::deserialize::<State>(&data) {
                let mut s = ctx.state.write().unwrap();
                *s = loaded;
                step::index_labels(&mut s);
                ctx.tw = 0.0;
                log::info!("Quick loaded");
            }
        }
    }
    if app.keyboard.was_pressed(KeyCode::F7) {
        let mut s = ctx.state.write().unwrap();
        load_scenes(&mut s, &ctx.scripts_dir);
        step::index_labels(&mut s);
        ctx.tw = 0.0;
    }

    // Animations (frame-rate independent)
    let dt = app.timer.delta().as_secs_f64();
    let skip = app.keyboard.is_down(KeyCode::ControlLeft)
        || app.keyboard.is_down(KeyCode::ControlRight);
    {
        let mut s = ctx.state.write().unwrap();
        // Text fade-in: tw accumulates from 0 to 1 over ~0.5s (instant in skip mode)
        if let Some(ref mut d) = s.dialogue {
            if skip {
                ctx.tw = 1.0; // instant text, then auto-advance
            } else if ctx.tw < 1.0 {
                ctx.tw = (ctx.tw + dt as f64 * 2.0).min(1.0);
            }
            d.visible_chars = d.text.chars().count();
        } else {
            ctx.tw = 0.0;
        }
        // Skip/auto mode: auto-advance after text is fully shown
        if (skip || ctx.auto) && ctx.tw >= 1.0 {
            if skip || ctx.auto {
                ctx.auto_timer += dt;
            }
            if skip || ctx.auto_timer > 2.0 {
                step::advance(&mut s);
                step::step(&mut s);
                ctx.tw = 0.0;
                ctx.auto_timer = 0.0;
            }
        }
        for sp in s.sprites.values_mut() {
            if sp.transition_progress < 1.0 || skip {
                let speed = if sp.transition == Transition::Instant { 999.0 } else { 3.0 };
                sp.transition_progress = (sp.transition_progress + dt as f32 * speed).min(1.0);
            }
        }
        // Remove fully hidden sprites
        s.sprites.retain(|_, sp| sp.entering || sp.transition_progress < 1.0);
        if let Some(ref mut t) = s.bg_transition {
            if t.progress < 1.0 || skip {
                t.progress = (t.progress + dt as f32 * 4.0).min(1.0);
            }
        }
    }
}

// ── GPU draw ──
fn draw(gfx: &mut Graphics, ctx: &mut Ctx) {
    let sg = ctx.state.read().unwrap();
    let mut d = gfx.create_draw();
    d.clear(Color::BLACK);

    let (ww, wh) = (gfx.size().0 as f32, gfx.size().1 as f32);
    let sc = (ww / 2560.0).min(wh / 1440.0);
    let ox = (ww - 2560.0 * sc) / 2.0;
    let oy = (wh - 1440.0 * sc) / 2.0;
    let ds = |v: f32| v * sc;
    let dx = |v: f32| ox + v * sc;
    let dy = |v: f32| oy + v * sc;

    // ── Background ──
    if let Some(bg) = &sg.bg {
        let fname = std::path::Path::new(bg).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        if let Some(tex) = ctx.textures.get(&format!("bg/{}", fname)) {
            d.image(tex).position(ox, oy).size(ds(2560.0), ds(1440.0));
        }
    }

    // ── Sprites ──
    let mut sp: Vec<_> = sg.sprites.iter().collect();
    sp.sort_by(|a,b| a.1.position.y.partial_cmp(&b.1.position.y).unwrap_or(std::cmp::Ordering::Equal));
    for (_, spr) in &sp {
        let fname = std::path::Path::new(&spr.image).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        if let Some(tex) = ctx.textures.get(&format!("figure/{}", fname)) {
            let p = spr.transition_progress;
            let alpha = if spr.entering { p } else { 1.0 - p };
            let sh_design = 960.0;
            let sw_design = sh_design * tex.width() as f32 / tex.height() as f32;
            let x_design = spr.position.x.resolve(sw_design) + spr.y_offset * (1.0 - p);
            d.image(tex)
                .position(dx(x_design), dy(480.0))
                .size(ds(sw_design), ds(sh_design))
                .alpha(alpha);
        }
    }

    // ── WebGAL text box (fontdue rasterized) ──
    if let Some(di) = &sg.dialogue {
        let box_w = ds(2560.0 - 50.0);
        let box_h = ds(350.0);
        let box_y = dy(1440.0 - 350.0 - 20.0);
        let name_y = box_y - ds(90.0);
        let name_h = ds(80.0);
        // Name plate bg
        d.rect((dx(25.0), name_y), (box_w, name_h))
            .fill().color(Color::new(0.0, 0.0, 0.0, 0.667));
        let (name_tex, nw, nh) = get_text_tex(gfx, &mut ctx.text_cache, &ctx.font, &ctx.fallback_font, &di.speaker, 44.0, 0.0);
        d.image(&name_tex)
            .position(dx(225.0), name_y + ds(14.0))
            .size(nw as f32 * sc, nh as f32 * sc);
        // Text box bg
        d.rect((dx(25.0), box_y), (box_w, box_h))
            .fill().color(Color::new(0.0, 0.0, 0.0, 0.565));
        let (text_tex, tw, th) = get_text_tex(gfx, &mut ctx.text_cache, &ctx.font, &ctx.fallback_font, &di.text, 52.0, 2200.0);
        d.image(&text_tex)
            .position(dx(225.0), box_y + ds(40.0))
            .size(tw as f32 * sc, th as f32 * sc);
    }

    // ── WebGAL choices (fontdue rasterized) ──
    if let Some(chs) = &sg.menu {
        d.rect((dx(0.0), dy(0.0)), (ds(2560.0), ds(1440.0)))
            .fill().color(Color::new(0.0, 0.0, 0.0, 0.051));
        let item_w = ds(1280.0);
        let item_h = ds(80.0);
        let gap = ds(14.0);
        let start_y = dy(720.0) - (chs.len() as f32 * (item_h + gap) - gap) / 2.0;
        for (i, ch) in chs.iter().enumerate() {
            let y = start_y + i as f32 * (item_h + gap);
            d.rect((dx(640.0), y), (item_w, item_h))
                .fill().color(Color::new(0.0, 0.0, 0.0, 0.188));
            let (ch_tex, cw, ch_h) = get_text_tex(gfx, &mut ctx.text_cache, &ctx.font, &ctx.fallback_font, &ch.text, 42.0, 0.0);
            let ctw = cw as f32 * sc;
            let cth = ch_h as f32 * sc;
            d.image(&ch_tex)
                .position(dx(640.0) + (item_w - ctw) / 2.0, y + (item_h - cth) / 2.0)
                .size(ctw, cth);
        }
    }

    gfx.render(&d);
}

// ── fontdue text helpers ──

fn get_text_tex(
    gfx: &mut Graphics,
    cache: &mut HashMap<(String, u32, u32), (Texture, u32, u32)>,
    font: &fontdue::Font,
    fallback: &fontdue::Font,
    text: &str,
    px: f32,
    max_width: f32,
) -> (Texture, u32, u32) {
    let key = (text.to_string(), px as u32, max_width as u32);
    if let Some((tex, w, h)) = cache.get(&key) {
        return (tex.clone(), *w, *h);
    }
    let (rgba, w, h) = rasterize_text(font, fallback, text, px, max_width);
    if let Ok(tex) = gfx.create_texture().from_bytes(&rgba, w, h).build() {
        cache.insert(key, (tex.clone(), w, h));
        return (tex, w, h);
    }
    (gfx.create_texture().from_bytes(&[0u8;4], 1, 1).build().unwrap(), 1, 1)
}

fn rasterize_text(font: &fontdue::Font, fallback: &fontdue::Font, text: &str, px: f32, max_width: f32) -> (Vec<u8>, u32, u32) {
    use fontdue::layout::{CoordinateSystem, Layout, LayoutSettings, TextStyle};
    let same_font = std::ptr::eq(font as *const _, fallback as *const _);

    // Split text into script runs: Latin (MavenPro, font_index=1) vs CJK (HanaMinA, font_index=0)
    let fonts: [&fontdue::Font; 2] = [font, fallback];
    let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
    let mw = if max_width > 0.0 { Some(max_width) } else { None };
    let settings = LayoutSettings { x: 0.0, y: 0.0, max_width: mw, ..Default::default() };
    layout.reset(&settings);

    if same_font {
        layout.append(&[font], &TextStyle::new(text, px, 0));
    } else {
        // Segment text by script: each contiguous run uses one font
        let mut seg_start = 0;
        let chars: Vec<char> = text.chars().collect();
        let mut current_is_latin = !chars.is_empty() && fallback.lookup_glyph_index(chars[0]) != 0;
        for (i, &ch) in chars.iter().enumerate().skip(1) {
            let is_latin = fallback.lookup_glyph_index(ch) != 0;
            if is_latin != current_is_latin {
                // End current segment
                let seg_text: String = chars[seg_start..i].iter().collect();
                let fi = if current_is_latin { 1 } else { 0 };
                layout.append(&fonts, &TextStyle::new(&seg_text, px, fi));
                seg_start = i;
                current_is_latin = is_latin;
            }
        }
        // Final segment
        if seg_start < chars.len() {
            let seg_text: String = chars[seg_start..].iter().collect();
            let fi = if current_is_latin { 1 } else { 0 };
            layout.append(&fonts, &TextStyle::new(&seg_text, px, fi));
        }
    }

    let glyphs = layout.glyphs();
    if glyphs.is_empty() { return (vec![0u8; 4], 1, 1); }
    let w = (glyphs.iter().map(|g| g.x + g.width as f32).max_by(|a,b| a.partial_cmp(b).unwrap()).unwrap_or(1.0)).ceil() as u32;
    let h = (glyphs.iter().map(|g| g.y + g.height as f32).max_by(|a,b| a.partial_cmp(b).unwrap()).unwrap_or(1.0)).ceil() as u32;
    let w = w.max(1); let h = h.max(1);
    let mut buf = vec![0u8; (w * h * 4) as usize];

    for g in glyphs {
        let raster_font = &fonts[g.font_index];
        let (metrics, bitmap) = raster_font.rasterize_config(g.key);
        for row in 0..metrics.height {
            for col in 0..metrics.width {
                let sx = g.x as u32 + col as u32;
                let sy = g.y as u32 + row as u32;
                if sx < w && sy < h {
                    let si = ((sy * w + sx) * 4) as usize;
                    let bi = (row * metrics.width + col) as usize;
                    if bi < bitmap.len() {
                        buf[si] = 255; buf[si+1] = 255; buf[si+2] = 255; buf[si+3] = bitmap[bi];
                    }
                }
            }
        }
    }
    (buf, w, h)
}

// ── Helpers ──

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
    // Load HanaMinA (primary, for layout + CJK) + MavenPro (fallback, for Latin style)
    let mut primary: Option<fontdue::Font> = None;  // HanaMinA
    let mut fallback: Option<fontdue::Font> = None; // MavenPro
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

fn load_window_size(path: &std::path::Path) -> (u32, u32) {
    if let Ok(data) = std::fs::read_to_string(path) {
        let parts: Vec<u32> = data.split_whitespace().filter_map(|s| s.parse().ok()).collect();
        if parts.len() == 2 { return (parts[0], parts[1]); }
    }
    (2560, 1440) // default
}

fn save_window_size(path: &std::path::Path, w: u32, h: u32) {
    let _ = std::fs::write(path, format!("{} {}", w, h));
}
