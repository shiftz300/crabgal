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
        Ctx { scripts_dir: sd, state, textures, font, watcher_rx: wrx, tw: 0.0 }
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

    if app.keyboard.was_pressed(KeyCode::Space) || app.keyboard.was_pressed(KeyCode::Enter)
        || app.mouse.left_was_pressed()
    {
        advance(ctx);
    }
    if app.keyboard.was_pressed(KeyCode::Escape) { app.exit(); }
    if app.keyboard.was_pressed(KeyCode::F5) {
        let mut s = ctx.state.write().unwrap();
        load_scenes(&mut s, &ctx.scripts_dir);
        step::index_labels(&mut s);
    }

    // Animations (frame-rate independent)
    let dt = app.timer.delta().as_secs_f64();
    {
        let mut s = ctx.state.write().unwrap();
        // Text fade-in: tw accumulates from 0 to 1 over ~0.5s
        if let Some(ref mut d) = s.dialogue {
            if ctx.tw < 1.0 {
                ctx.tw = (ctx.tw + dt as f64 * 2.0).min(1.0);
            }
            d.visible_chars = d.text.chars().count();
        } else {
            ctx.tw = 0.0;
        }
        for sp in s.sprites.values_mut() {
            if sp.transition_progress < 1.0 {
                sp.transition_progress = (sp.transition_progress + dt as f32 * 3.0).min(1.0);
            }
        }
        if let Some(ref mut t) = s.bg_transition {
            if t.progress < 1.0 {
                t.progress = (t.progress + dt as f32 * 2.0).min(1.0);
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

    // ── Text box with fade-in ──
    if let Some(di) = &sg.dialogue {
        let alpha = ctx.tw as f32;
        d.rect((dx(0.0), dy(700.0)), (ds(1600.0), ds(200.0)))
            .fill().color(Color::new(0.0, 0.0, 0.0, 0.7 * alpha));
        d.text(&ctx.font, &di.speaker)
            .position(dx(80.0), dy(720.0)).size(ds(32.0))
            .color(Color::new(1.0, 1.0, 1.0, alpha));
        d.text(&ctx.font, &di.text)
            .position(dx(80.0), dy(780.0)).size(ds(28.0))
            .color(Color::new(1.0, 1.0, 1.0, alpha));
    }

    // ── Menu ──
    if let Some(chs) = &sg.menu {
        for (i, ch) in chs.iter().enumerate() {
            let y = 500.0 + i as f32 * 50.0;
            d.rect((dx(500.0), dy(y)), (ds(600.0), ds(40.0)))
                .fill().color(Color::new(0.0, 0.0, 0.2, 0.8));
            d.text(&ctx.font, &ch.text)
                .position(dx(520.0), dy(y + 5.0)).size(ds(24.0)).color(Color::WHITE);
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
