// Text box with blurred background + speaker name bar.
use std::collections::HashMap;

use ggez::graphics::{self, Canvas, Color, DrawParam, Image, Mesh, Rect, ShaderBuilder};
use ggez::{Context, GameResult};

use crabgal_core::state::{Dialogue, Sprite};
use crabgal_render::shaders::{HBLUR_WGSL, VBLUR_WGSL};
use crabgal_render::text::{get_text_tex, TexCache};

pub fn draw_textbox(
    ctx: &mut Context,
    c: &mut Canvas,
    textures: &HashMap<String, Image>,
    text_cache: &mut TexCache,
    font: &fontdue::Font,
    fallback: &fontdue::Font,
    dialogue: &Option<Dialogue>,
    bg_name: &Option<String>,
    sprites: &Vec<(String, Sprite)>,
    mini_avatar: &Option<String>,
    mini_avatar_progress: f32,
    tw: f64,
    textbox_visible: bool,
    sc: f32,
    ox: f32,
    oy: f32,
    ds: &dyn Fn(f32) -> f32,
    dx: &dyn Fn(f32) -> f32,
    dy: &dyn Fn(f32) -> f32,
) -> GameResult {
    let di = match dialogue {
        Some(d) => d,
        None => return Ok(()),
    };
    if !textbox_visible { return Ok(()); }

    let a = tw as f32;
    let bw = ds(2560.0 - 50.0);
    let bh = ds(350.0);
    let by = dy(1440.0 - 350.0 - 20.0);
    let ny = by - ds(90.0);
    let nh = ds(80.0);

    // Mini avatar + text box dodge animation (WebGAL style, 250px shift)
    let dodge = mini_avatar_progress * ds(250.0);
    let avatar_size = ds(300.0);

    let tb_x = dx(25.0) + dodge;
    let tb_w = bw;
    let tb_h = bh; // text box only, name bar has its own background
    // Downscale offscreen to 1/2 for softer blur (reduces block artifacts)
    let down = 2u32;
    let tw_ = (tb_w.ceil() as u32 / down).max(1);
    let th = (tb_h.ceil() as u32 / down).max(1);
    if tw_ > 0 && th > 0 {
        let src = Image::new_canvas_image(ctx, tw_, th, 1);
        {
            let mut tc = Canvas::from_image(ctx, src.clone(), Color::BLACK);
            if let Some(bg) = bg_name {
                let n = std::path::Path::new(bg)
                    .file_name().map(|x| x.to_string_lossy().to_string()).unwrap_or_default();
                if let Some(tex) = textures.get(&format!("bg/{}", n)) {
                    let tiw = tex.width() as f32; let tih = tex.height() as f32;
                    let sw = tw_ as f32 / tb_w; let sh = th as f32 / tb_h;
                    tc.draw(tex, DrawParam::new()
                        .dest([(ox - tb_x) * sw, (oy - by) * sh])
                        .scale([ds(2560.0) / tiw * sw, ds(1440.0) / tih * sh]));
                }
            }
            let mut sp: Vec<_> = sprites.iter().collect();
            sp.sort_by(|a, b| a.1.position.y.partial_cmp(&b.1.position.y)
                .unwrap_or(std::cmp::Ordering::Equal));
            for (_, spr) in &sp {
                let n = std::path::Path::new(&spr.image)
                    .file_name().map(|x| x.to_string_lossy().to_string()).unwrap_or_default();
                if let Some(tex) = textures.get(&format!("figure/{}", n)) {
                    let p = spr.transition_progress;
                    let alpha = if spr.entering { p } else { 1.0 - p };
                    let sh_ds = 960.0f32;
                    let sw_ds = sh_ds * tex.width() as f32 / tex.height() as f32;
                    let xd_ds = spr.position.x.resolve(sw_ds) + spr.y_offset * (1.0 - p);
                    let tiw = tex.width() as f32; let tih = tex.height() as f32;
                    let sw = tw_ as f32 / tb_w; let sh = th as f32 / tb_h;
                    let mut dp = DrawParam::new()
                        .dest([(dx(xd_ds) - tb_x) * sw, (dy(480.0) - by) * sh])
                        .scale([ds(sw_ds) / tiw * sw, ds(sh_ds) / tih * sh]);
                    dp.color.a = alpha;
                    tc.draw(tex, dp);
                }
            }
            tc.finish(ctx)?;
        }
        let hb = Image::new_canvas_image(ctx, tw_, th, 1);
        {
            let mut hc = Canvas::from_image(ctx, hb.clone(), Color::BLACK);
            let shader = ShaderBuilder::from_code(HBLUR_WGSL).build(ctx)?;
            hc.set_shader(&shader);
            hc.draw(&src, DrawParam::new().dest([0.0, 0.0]).scale([1.0, 1.0]));
            hc.set_default_shader();
            hc.finish(ctx)?;
        }
        let vshader = ShaderBuilder::from_code(VBLUR_WGSL).build(ctx)?;
        c.set_shader(&vshader);
        let mut bg_dp = DrawParam::new()
            .dest([tb_x, by])
            .scale([tb_w / tw_ as f32, tb_h / th as f32]);
        bg_dp.color = Color::new(0.15, 0.15, 0.15, 0.72 * a);
        c.draw(&hb, bg_dp);
        c.set_default_shader();
    }

    // Draw mini avatar image
    if let Some(av_path) = mini_avatar {
        if mini_avatar_progress > 0.01 {
            let av_n = std::path::Path::new(av_path)
                .file_name().map(|x| x.to_string_lossy().to_string()).unwrap_or_default();
            if let Some(av_tex) = textures.get(&format!("figure/{}", av_n)) {
                let av_x = dx(0.0) - avatar_size + dodge;
                let av_y = dy(0.0);
                let av_w = avatar_size;
                let av_h = avatar_size * av_tex.height() as f32 / av_tex.width() as f32;
                c.draw(av_tex, DrawParam::new()
                    .dest([av_x, av_y])
                    .scale([av_w / av_tex.width() as f32, av_h / av_tex.height() as f32])
                    .color(Color::new(1.0, 1.0, 1.0, mini_avatar_progress * a)));
            }
        }
    }

    // Speaker name bar — compact width, with gap below
    let (ni, nw, _nh2) = get_text_tex(ctx, text_cache, font, fallback,
        &di.speaker, 44.0, 0.0);
    let name_w = nw as f32 * sc + ds(80.0);
    let name_x = dx(25.0) + dodge;
    let nm = Mesh::new_rectangle(ctx, graphics::DrawMode::fill(),
        Rect::new(name_x, ny, name_w, nh),
        Color::new(0.0, 0.0, 0.0, 0.6 * a))?;
    c.draw(&nm, DrawParam::default());
    c.draw(&ni, DrawParam::new()
        .dest([name_x + ds(40.0), ny + ds(14.0)])
        .scale([sc, sc])
        .color(Color::new(1.0, 1.0, 1.0, a)));

    let (ti, _, _) = get_text_tex(ctx, text_cache, font, fallback,
        &di.text, 52.0, 2200.0);
    let text_x = name_x + name_w + ds(30.0);
    c.draw(&ti, DrawParam::new()
        .dest([text_x, by + ds(40.0)])
        .scale([sc, sc])
        .color(Color::new(1.0, 1.0, 1.0, a)));
    Ok(())
}
