// crabgal CLI — dev / check
// Render: pixels crate (GPU-backed RGBA, zero shaders)
// Bilinear interpolation for anti-aliased image scaling.
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, atomic::AtomicU32, atomic::Ordering, RwLock};
use winit::event::{Event, KeyEvent, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use crabgal_core::state::State;
use crabgal_core::step;
use crabgal_core::Action;
use crabgal_script::parser::parse_script;

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    let a: Vec<String> = std::env::args().collect();
    if a.len() < 2 { eprintln!("crabgal dev <dir> | check <file>"); std::process::exit(1); }
    match a[1].as_str() {
        "dev" => run_dev(&a.get(2).map(PathBuf::from).unwrap_or_default()),
        "check" => run_check(&a.get(2).map(PathBuf::from).ok_or_else(|| anyhow::anyhow!("missing file"))?),
        _ => std::process::exit(1),
    }
}

#[allow(deprecated)]
fn run_dev(dir: &std::path::Path) -> anyhow::Result<()> {
    let sd = dir.join("scripts");
    let ar = dir.join("assets");
    if !sd.exists() { std::fs::create_dir_all(&sd)?; }

    // Scripts
    let mut s = State::new();
    load_scenes(&mut s, &sd);
    if s.scenes.is_empty() {
        s.scenes.insert("s".into(), vec![Action::ShowBg{image:"bg.webp".into(),transition:Default::default()}, Action::Say{speaker:"?".into(),text:"no script".into()}]);
    }
    s.current_scene = s.scenes.keys().next().cloned().unwrap_or_default();
    step::index_labels(&mut s);
    step::step(&mut s);
    let s = Arc::new(RwLock::new(s));
    let wrx = crabgal_script::watcher::start_watcher(&sd)?;

    // Textures (RGBA buffers)
    let tex: Arc<RwLock<HashMap<String, (Vec<u8>, u32, u32)>>> = Arc::new(RwLock::new(HashMap::new()));
    { let mut t = tex.write().unwrap();
        for (dir, prefix) in &[("background","bg"),("figure","figure")] {
            if let Ok(e) = std::fs::read_dir(ar.join(dir)) { for x in e.flatten() { let p=x.path();
                if p.extension().map_or(false,|e|e=="png"||e=="webp"||e=="jpg") {
                    if let Ok(img)=image::open(&p) { let rgba=img.to_rgba8(); let(w,h)=rgba.dimensions();
                        t.insert(format!("{}/{}",prefix,p.file_name().unwrap().to_string_lossy()),(rgba.into_raw(),w,h)); }
                }
            }}
        }
    }

    // Window + pixels
    let ev = EventLoop::new()?;
    let win_sz = winit::dpi::PhysicalSize::new(1600, 900);
    let w = Arc::new(ev.create_window(winit::window::WindowAttributes::default().with_title("crabgal").with_inner_size(win_sz))?);
    let mut pxs = pixels::Pixels::new(win_sz.width, win_sz.height, pixels::SurfaceTexture::new(win_sz.width, win_sz.height, w.as_ref()))?;
    let ww_a = Arc::new(AtomicU32::new(win_sz.width));
    let wh_a = Arc::new(AtomicU32::new(win_sz.height));
    let w2 = w.clone();
    let mut tw = 0.0f64;

    ev.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Poll);
        match event {
            Event::WindowEvent{event,..} => match event {
                WindowEvent::CloseRequested => elwt.exit(),
                WindowEvent::Resized(sz) => { pxs.resize_surface(sz.width,sz.height).ok(); pxs.resize_buffer(sz.width,sz.height).ok(); ww_a.store(sz.width,Ordering::Relaxed); wh_a.store(sz.height,Ordering::Relaxed); }
                WindowEvent::RedrawRequested => {
                    let sg = s.read().unwrap();
                    let tg = tex.read().unwrap();
                    let frame = pxs.frame_mut();
                    let ww = ww_a.load(Ordering::Relaxed) as usize;
                    let wh = wh_a.load(Ordering::Relaxed) as usize;
                    if ww == 0 || wh == 0 { return; }
                    frame.fill(0);

                    let sc = (ww as f32 / 1600.0).min(wh as f32 / 900.0);
                    let sw = (1600.0 * sc) as usize; let sh = (900.0 * sc) as usize;
                    let ox = (ww - sw) / 2; let oy = (wh - sh) / 2;
                    let ds = |v: f32| (v * sc) as usize;
                    let sx = |v: f32| ox + ds(v);
                    let sy = |v: f32| oy + ds(v);

                    // Background
                    if let Some(bg) = &sg.bg {
                        let fname = std::path::Path::new(bg).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                        if let Some((td, tw_, th_)) = tg.get(&format!("bg/{}", fname)) {
                            bilinear(frame, ww, td, *tw_ as usize, *th_ as usize, ox, oy, sw, sh);
                        }
                    }

                    // Sprites — preserve aspect ratio, scale to ~600px design height
                    let mut sp: Vec<_> = sg.sprites.iter().collect();
                    sp.sort_by(|a,b| a.1.position.y.partial_cmp(&b.1.position.y).unwrap_or(std::cmp::Ordering::Equal));
                    for (_, spr) in &sp {
                        let fname = std::path::Path::new(&spr.image).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                        if let Some((td, tw_, th_)) = tg.get(&format!("figure/{}", fname)) {
                            let p = spr.transition_progress;
                            let a = if spr.entering { p } else { 1.0 - p };
                            let x = spr.position.x.resolve(400.0) + spr.y_offset * (1.0 - p);
                            // Scale to 600px design height, maintain aspect ratio
                            let dh = ds(600.0); let dw = (*tw_ as f32 / *th_ as f32 * 600.0 * sc) as usize;
                            let sw = (*tw_ as f32 / *th_ as f32 * 600.0 * sc) as usize; // screen width
                            let sh = ds(600.0); // screen height
                            // Center the sprite x on its anchor, but keep bottom-aligned at y=300
                            let sprite_y = sy(300.0);
                            bilinear_alpha(frame, ww, td, *tw_ as usize, *th_ as usize, sx(x) - sw/2, sprite_y, sw, sh, a);
                        }
                    }

                    // Text box + dialogue
                    if let Some(d) = &sg.dialogue {
                        fill_aa(frame, ww, ox, sy(700.0), sw, ds(200.0), 0, 0, 0, 0.7);
                        draw_text(frame, ww, ox, oy, sc, &d.speaker, 80.0, 720.0, 32.0);
                        let vtxt = &d.text[..d.visible_chars.min(d.text.len())];
                        draw_text(frame, ww, ox, oy, sc, vtxt, 80.0, 780.0, 28.0);
                    }

                    // Menu
                    if let Some(chs) = &sg.menu {
                        for (i, ch) in chs.iter().enumerate() {
                            let y = 500.0 + i as f32 * 50.0;
                            fill_aa(frame, ww, sx(500.0), sy(y), ds(600.0), ds(40.0), 0, 0, 51, 0.8);
                            draw_text(frame, ww, ox, oy, sc, &ch.text, 520.0, y + 5.0, 24.0);
                        }
                    }

                    pxs.render().ok();
                }
                WindowEvent::KeyboardInput{event: KeyEvent{physical_key: PhysicalKey::Code(k), state: ks, ..}, ..} => if ks.is_pressed() { match k {
                    KeyCode::Space | KeyCode::Enter => { let mut s = s.write().unwrap();
                        if let Some(ref d) = s.dialogue {
                            if d.visible_chars < d.text.len() { s.dialogue.as_mut().unwrap().visible_chars = d.text.len(); }
                            else { step::advance(&mut s); step::step(&mut s); }
                        } else { step::step(&mut s); }
                        tw = 0.0;
                    }
                    KeyCode::Escape => elwt.exit(),
                    KeyCode::F5 => { let mut s = s.write().unwrap(); load_scenes(&mut s, &sd); step::index_labels(&mut s); }
                    _ => {}
                }}
                _ => {}
            },
            Event::AboutToWait => {
                while let Ok(_) = wrx.try_recv() { let mut s = s.write().unwrap(); load_scenes(&mut s, &sd); step::index_labels(&mut s); }
                { let mut s = s.write().unwrap();
                    if let Some(ref mut d) = s.dialogue { if d.visible_chars < d.text.len() { tw += 0.016; if tw > 0.05 { d.visible_chars += 1; tw = 0.0; } } }
                    for sp in s.sprites.values_mut() { if sp.transition_progress < 1.0 { sp.transition_progress = (sp.transition_progress + 0.05).min(1.0); } }
                    if let Some(ref mut t) = s.bg_transition { if t.progress < 1.0 { t.progress = (t.progress + 0.02).min(1.0); } }
                }
                w2.request_redraw();
            }
            _ => {}
        }
    })?;
    Ok(())
}

fn load_scenes(s: &mut State, d: &std::path::Path) {
    s.scenes.clear();
    if let Ok(e) = std::fs::read_dir(d) { for x in e.flatten() { let p = x.path();
        if p.extension().map_or(false, |e| e == "crab") { let n = p.file_stem().unwrap().to_string_lossy().to_string();
            if let Ok(c) = std::fs::read_to_string(&p) { s.scenes.insert(n.clone(), parse_script(&c)); if s.current_scene.is_empty() { s.current_scene = n; } }
        }
    }}
}

fn run_check(p: &std::path::Path) -> anyhow::Result<()> {
    let c = std::fs::read_to_string(p)?;
    for (i, a) in parse_script(&c).iter().enumerate() { println!("{:3}: {:?}", i + 1, a); }
    Ok(())
}

// ── Anti-aliased rendering (bilinear interpolation) ──

#[inline]
fn blerp(a: u8, b: u8, t: f32) -> u8 { (a as f32 + (b as f32 - a as f32) * t) as u8 }

fn bilinear_sample(tex: &[u8], tw: usize, th: usize, u: f32, v: f32) -> (u8, u8, u8, u8) {
    let x = u * (tw as f32 - 1.0); let y = v * (th as f32 - 1.0);
    let x0 = x.floor() as usize; let y0 = y.floor() as usize;
    let x1 = (x0 + 1).min(tw - 1); let y1 = (y0 + 1).min(th - 1);
    let fx = x - x0 as f32; let fy = y - y0 as f32;
    let at = |px: usize, py: usize| { let o = (py * tw + px) * 4; (tex[o], tex[o+1], tex[o+2], tex[o+3]) };
    let (r0, g0, b0, a0) = at(x0, y0); let (r1, g1, b1, a1) = at(x1, y0);
    let (r2, g2, b2, a2) = at(x0, y1); let (r3, g3, b3, a3) = at(x1, y1);
    (blerp(blerp(r0, r1, fx), blerp(r2, r3, fx), fy),
     blerp(blerp(g0, g1, fx), blerp(g2, g3, fx), fy),
     blerp(blerp(b0, b1, fx), blerp(b2, b3, fx), fy),
     blerp(blerp(a0, a1, fx), blerp(a2, a3, fx), fy))
}

fn bilinear(fb: &mut [u8], fbw: usize, tex: &[u8], tw: usize, th: usize, dx: usize, dy: usize, dw: usize, dh: usize) {
    for ty in 0..dh { let v = ty as f32 / dh as f32; for tx in 0..dw { let u = tx as f32 / dw as f32;
        let (r, g, b, a) = bilinear_sample(tex, tw, th, u, v);
        let di = ((dy + ty) * fbw + dx + tx) * 4; if di + 3 < fb.len() { fb[di] = r; fb[di+1] = g; fb[di+2] = b; fb[di+3] = a; }
    }}
}

fn bilinear_alpha(fb: &mut [u8], fbw: usize, tex: &[u8], tw: usize, th: usize, dx: usize, dy: usize, dw: usize, dh: usize, alpha: f32) {
    for ty in 0..dh { let v = ty as f32 / dh as f32; for tx in 0..dw { let u = tx as f32 / dw as f32;
        let (sr, sg, sb, sa) = bilinear_sample(tex, tw, th, u, v);
        let sa = sa as f32 * alpha; if sa < 1.0 { continue; }
        let di = ((dy + ty) * fbw + dx + tx) * 4; if di + 3 < fb.len() {
            let a = sa / 255.0; let ia = 1.0 - a;
            fb[di] = (sr as f32 * a + fb[di] as f32 * ia) as u8;
            fb[di+1] = (sg as f32 * a + fb[di+1] as f32 * ia) as u8;
            fb[di+2] = (sb as f32 * a + fb[di+2] as f32 * ia) as u8;
            fb[di+3] = (sa + fb[di+3] as f32 * ia) as u8;
        }
    }}
}

fn fill_aa(fb: &mut [u8], fbw: usize, x: usize, y: usize, w: usize, h: usize, r: u8, g: u8, b: u8, alpha: f32) {
    for ty in y..y+h { for tx in x..x+w { let i = (ty * fbw + tx) * 4; if i + 3 < fb.len() {
        let ia = 1.0 - alpha;
        fb[i] = (r as f32 * alpha + fb[i] as f32 * ia) as u8;
        fb[i+1] = (g as f32 * alpha + fb[i+1] as f32 * ia) as u8;
        fb[i+2] = (b as f32 * alpha + fb[i+2] as f32 * ia) as u8;
        fb[i+3] = ((255.0 * alpha + fb[i+3] as f32 * ia) as u8).max(fb[i+3]);
    }}}
}

fn draw_text(fb: &mut [u8], fbw: usize, ox: usize, oy: usize, sc: f32, text: &str, x: f32, y: f32, fs: f32) {
    let dx = ox + (x * sc) as usize; let dy = oy + (y * sc) as usize;
    let cw = (fs * 0.6 * sc) as usize; let ch = (fs * sc) as usize;
    for (i, c) in text.chars().enumerate() { let bx = dx + i * (cw + 2); let bm = char_bits(c);
        for row in 0..7 { let mut bits = bm[row]; for col in 0..5 { if (bits & 0x8000_0000) != 0 {
            let px = bx + col * cw / 5; let py = dy + row * ch / 7;
            let di = (py * fbw + px) * 4; if di + 3 < fb.len() { fb[di] = 255; fb[di+1] = 255; fb[di+2] = 255; fb[di+3] = 255; }
        } bits <<= 1; }}
    }
}

fn char_bits(c: char) -> [u32; 7] { match c {
    'A'|'a'=>[0b01110,0b10001,0b10001,0b11111,0b10001,0b10001,0b10001],'B'|'b'=>[0b11110,0b10001,0b11110,0b10001,0b10001,0b10001,0b11110],
    'C'|'c'=>[0b01110,0b10001,0b10000,0b10000,0b10000,0b10001,0b01110],'D'|'d'=>[0b11110,0b10001,0b10001,0b10001,0b10001,0b10001,0b11110],
    'E'|'e'=>[0b11111,0b10000,0b11110,0b10000,0b10000,0b10000,0b11111],'F'|'f'=>[0b11111,0b10000,0b11110,0b10000,0b10000,0b10000,0b10000],
    'G'|'g'=>[0b01110,0b10001,0b10000,0b10111,0b10001,0b10001,0b01110],'H'|'h'=>[0b10001,0b10001,0b11111,0b10001,0b10001,0b10001,0b10001],
    'I'|'i'=>[0b01110,0b00100,0b00100,0b00100,0b00100,0b00100,0b01110],'J'|'j'=>[0b00111,0b00001,0b00001,0b00001,0b10001,0b10001,0b01110],
    'K'|'k'=>[0b10001,0b10010,0b11100,0b10010,0b10010,0b10001,0b10001],'L'|'l'=>[0b10000,0b10000,0b10000,0b10000,0b10000,0b10000,0b11111],
    'M'|'m'=>[0b10001,0b11011,0b10101,0b10001,0b10001,0b10001,0b10001],'N'|'n'=>[0b10001,0b11001,0b10101,0b10011,0b10001,0b10001,0b10001],
    'O'|'o'=>[0b01110,0b10001,0b10001,0b10001,0b10001,0b10001,0b01110],'P'|'p'=>[0b11110,0b10001,0b10001,0b11110,0b10000,0b10000,0b10000],
    'Q'|'q'=>[0b01110,0b10001,0b10001,0b10001,0b10101,0b10010,0b01101],'R'|'r'=>[0b11110,0b10001,0b10001,0b11110,0b10010,0b10001,0b10001],
    'S'|'s'=>[0b01110,0b10001,0b10000,0b01110,0b00001,0b10001,0b01110],'T'|'t'=>[0b11111,0b00100,0b00100,0b00100,0b00100,0b00100,0b00100],
    'U'|'u'=>[0b10001,0b10001,0b10001,0b10001,0b10001,0b10001,0b01110],'V'|'v'=>[0b10001,0b10001,0b10001,0b10001,0b01010,0b01010,0b00100],
    'W'|'w'=>[0b10001,0b10001,0b10001,0b10101,0b10101,0b11011,0b10001],'X'|'x'=>[0b10001,0b01010,0b00100,0b00100,0b01010,0b10001,0b10001],
    'Y'|'y'=>[0b10001,0b01010,0b00100,0b00100,0b00100,0b00100,0b00100],'Z'|'z'=>[0b11111,0b00001,0b00010,0b00100,0b01000,0b10000,0b11111],
    '0'=>[0b01110,0b10001,0b10011,0b10101,0b11001,0b10001,0b01110],'1'=>[0b00100,0b01100,0b00100,0b00100,0b00100,0b00100,0b01110],
    '2'=>[0b01110,0b10001,0b00001,0b00110,0b01000,0b10000,0b11111],'3'=>[0b01110,0b10001,0b00001,0b00110,0b00001,0b10001,0b01110],
    '4'=>[0b00010,0b00110,0b01010,0b10010,0b11111,0b00010,0b00010],'5'=>[0b11111,0b10000,0b11110,0b00001,0b00001,0b10001,0b01110],
    '6'=>[0b01110,0b10001,0b10000,0b11110,0b10001,0b10001,0b01110],'7'=>[0b11111,0b00001,0b00010,0b00100,0b01000,0b01000,0b01000],
    '8'=>[0b01110,0b10001,0b10001,0b01110,0b10001,0b10001,0b01110],'9'=>[0b01110,0b10001,0b10001,0b01111,0b00001,0b00001,0b01110],
    ' '=>[0;7],'.'=>[0,0,0,0,0,0b01100,0b01100],','=>[0,0,0,0,0,0b01100,0b01000],
    '!'=>[0b00100,0b00100,0b00100,0b00100,0b00100,0,0b00100],'?'=>[0b01110,0b10001,0b00001,0b00110,0b00100,0,0b00100],
    ':'=>[0,0b01100,0b01100,0,0b01100,0b01100,0],';'=>[0,0b01100,0b01100,0,0b00100,0b01100,0b01000],
    '-'=>[0,0,0,0b11111,0,0,0],'\''=>[0b00100,0b00100,0,0,0,0,0],'"'=>[0b01010,0b01010,0,0,0,0,0],
    '('=>[0b00010,0b00100,0b01000,0b01000,0b01000,0b00100,0b00010],')'=>[0b01000,0b00100,0b00010,0b00010,0b00010,0b00100,0b01000],
    '['=>[0b01110,0b01000,0b01000,0b01000,0b01000,0b01000,0b01110],']'=>[0b01110,0b00010,0b00010,0b00010,0b00010,0b00010,0b01110],
    _=>[0;7]
}}
