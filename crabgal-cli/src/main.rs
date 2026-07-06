// crabgal CLI — dev / check
// GPU 2D rendering via notan (Metal backend). Zero hand-rolled pixel loops.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock, mpsc};

use crabgal_core::state::State;
use crabgal_core::step;
use crabgal_core::Action;
use crabgal_script::parser::parse_script;

use notan::draw::*;
use notan::prelude::*;

#[derive(AppState)]
struct Ctx {
    scripts_dir: PathBuf,
    state: Arc<RwLock<State>>,
    textures: HashMap<String, Texture>,
    font: Font,
    watcher_rx: mpsc::Receiver<PathBuf>,
    tw: f64,
    auto: bool,
    auto_timer: f64,
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

    // ── Load font ──
    let font_data = load_font(ar.join("fonts"))?;

    let win = WindowConfig::new()
        .set_size(1600, 900)
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
        let font = gfx.create_font(&font_data).expect("Font parse failed");
        Ctx { scripts_dir: sd, state, textures, font, watcher_rx: wrx, tw: 0.0, auto: false, auto_timer: 0.0 }
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
            let dsc = (ww as f32 / 1600.0).min(wh as f32 / 900.0);
            let dox = (ww as f32 - 1600.0 * dsc) / 2.0;
            let doy = (wh as f32 - 900.0 * dsc) / 2.0;
            let dx = (mx - dox) / dsc;
            let dy = (my - doy) / dsc;
            // Check if click hits a menu choice (WebGAL layout: centered, 800px wide, 72px tall + 12px gap)
            let item_w = 800.0;
            let item_h = 72.0;
            let item_gap = 12.0;
            if dx >= 400.0 && dx <= 400.0 + item_w {
                let s = ctx.state.read().unwrap();
                if let Some(ref chs) = s.menu {
                    let start_y = 450.0 - chs.len() as f32 * (item_h + item_gap) / 2.0;
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
                sp.transition_progress = (sp.transition_progress + dt as f32 * 5.0).min(1.0);
            }
        }
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
    let sc = (ww / 1600.0).min(wh / 900.0);
    let ox = (ww - 1600.0 * sc) / 2.0;
    let oy = (wh - 900.0 * sc) / 2.0;
    let ds = |v: f32| v * sc;
    let dx = |v: f32| ox + v * sc;
    let dy = |v: f32| oy + v * sc;

    // ── Background ──
    if let Some(bg) = &sg.bg {
        let fname = std::path::Path::new(bg).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        if let Some(tex) = ctx.textures.get(&format!("bg/{}", fname)) {
            d.image(tex).position(ox, oy).size(ds(1600.0), ds(900.0));
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
            // Design-space sprite dimensions (600px target height, maintain aspect ratio)
            let sh_design = 600.0;
            let sw_design = sh_design * tex.width() as f32 / tex.height() as f32;
            // Resolve anchor to design-space left edge
            let x_design = spr.position.x.resolve(sw_design) + spr.y_offset * (1.0 - p);
            d.image(tex)
                .position(dx(x_design), dy(300.0))
                .size(ds(sw_design), ds(sh_design))
                .alpha(alpha);
        }
    }

    // ── WebGAL-style text box ──
    if let Some(di) = &sg.dialogue {
        let alpha = ctx.tw as f32;
        let box_h = ds(250.0);
        let box_y = dy(900.0 - 250.0);
        // Text box background: #00000090 (WebGAL: black 56% alpha)
        d.rect((dx(0.0), box_y), (ds(1600.0), box_h))
            .fill().color(Color::new(0.0, 0.0, 0.0, 0.56 * alpha));
        // Speaker name plate: floating above text box (WebGAL style)
        let name_h = ds(80.0);
        let name_y = box_y - name_h;
        d.rect((dx(50.0), name_y), (ds(300.0), name_h))
            .fill().color(Color::new(0.0, 0.0, 0.0, 0.67 * alpha));
        d.text(&ctx.font, &di.speaker)
            .position(dx(80.0), name_y + ds(10.0)).size(ds(32.0))
            .color(Color::new(1.0, 1.0, 1.0, alpha));
        // Dialogue text: left-padded, white
        d.text(&ctx.font, &di.text)
            .position(dx(80.0), box_y + ds(40.0)).size(ds(30.0))
            .color(Color::new(1.0, 1.0, 1.0, alpha));
    }

    // ── WebGAL-style choices ──
    if let Some(chs) = &sg.menu {
        // Dim overlay
        d.rect((dx(0.0), dy(0.0)), (ds(1600.0), ds(900.0)))
            .fill().color(Color::new(0.0, 0.0, 0.0, 0.05));
        let item_w = ds(800.0);
        let item_h = ds(72.0);
        let start_y = dy(450.0) - chs.len() as f32 * item_h / 2.0;
        for (i, ch) in chs.iter().enumerate() {
            let y = start_y + i as f32 * (item_h + ds(12.0));
            // WebGAL: bg #00000030, white text #ffffffaa, centered
            d.rect((dx(400.0), y), (item_w, item_h))
                .fill().color(Color::new(0.0, 0.0, 0.0, 0.19));
            d.text(&ctx.font, &ch.text)
                .position(dx(400.0) + item_w / 2.0, y + ds(16.0))
                .size(ds(36.0)).color(Color::new(1.0, 1.0, 1.0, 0.67))
                .h_align_center();
        }
    }

    gfx.render(&d);
}

// ── Helpers ──

fn load_scenes(s: &mut State, d: &std::path::Path) {
    s.scenes.clear();
    if let Ok(e) = std::fs::read_dir(d) {
        for entry in e.flatten() {
            let p = entry.path();
            if p.extension().map_or(false, |e| e == "crab") {
                let n = p.file_stem().unwrap().to_string_lossy().to_string();
                if let Ok(c) = std::fs::read_to_string(&p) {
                    s.scenes.insert(n.clone(), parse_script(&c));
                    if s.current_scene.is_empty() { s.current_scene = n; }
                }
            }
        }
    }
}

fn run_check(p: &std::path::Path) -> anyhow::Result<()> {
    let c = std::fs::read_to_string(p)?;
    for (i, a) in parse_script(&c).iter().enumerate() { println!("{:3}: {:?}", i + 1, a); }
    Ok(())
}

fn load_font(fonts_dir: std::path::PathBuf) -> anyhow::Result<Vec<u8>> {
    // 1. System font first (known-good TTF, always works)
    let sys = std::path::Path::new("/System/Library/Fonts/Supplemental/Arial Unicode.ttf");
    if sys.exists() { log::info!("Using system font"); return Ok(std::fs::read(sys)?); }
    // 2. Project fonts — TTF/OTF only (WOFF2/TTC not supported by glyph-brush)
    if let Ok(e) = std::fs::read_dir(&fonts_dir) {
        for entry in e.flatten() {
            let p = entry.path();
            let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
            if ext == "ttf" || ext == "otf" {
                if let Ok(data) = std::fs::read(&p) {
                    log::info!("Loaded font: {}", p.display());
                    return Ok(data);
                }
            }
        }
    }
    anyhow::bail!("No TTF/OTF font found")
}
