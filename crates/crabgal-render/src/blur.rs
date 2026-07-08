// Gaussian blur overlay — renders scene to offscreen, applies H+V blur, draws back.
use std::collections::HashMap;

use ggez::graphics::{Canvas, Color, DrawParam, Image, ShaderBuilder};
use ggez::{Context, GameResult};

use crabgal_core::state::Sprite;
use super::shaders::{HBLUR_WGSL, VBLUR_WGSL};

pub fn draw_blur_overlay(
    ctx: &mut Context,
    c: &mut Canvas,
    textures: &HashMap<String, Image>,
    bg_name: &Option<String>,
    sprites: &Vec<(String, Sprite)>,
    _sc: f32,
    ox: f32,
    oy: f32,
    ds: &dyn Fn(f32) -> f32,
    dx: &dyn Fn(f32) -> f32,
    dy: &dyn Fn(f32) -> f32,
    alpha: f32,
) -> GameResult {
    let down = 2u32;
    let w = (ds(2560.0) / down as f32).ceil() as u32;
    let h = (ds(1440.0) / down as f32).ceil() as u32;
    if w == 0 || h == 0 { return Ok(()); }

    let src = Image::new_canvas_image(ctx, w, h, 1);
    {
        let mut tc = Canvas::from_image(ctx, src.clone(), Color::BLACK);
        if let Some(bg) = bg_name {
            let n = std::path::Path::new(bg)
                .file_name().map(|x| x.to_string_lossy().to_string()).unwrap_or_default();
            if let Some(tex) = textures.get(&format!("bg/{}", n)) {
                tc.draw(tex, DrawParam::new()
                    .dest([0.0, 0.0])
                    .scale([w as f32 / tex.width() as f32, h as f32 / tex.height() as f32]));
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
                let alpha_s = if spr.entering { p } else { 1.0 - p };
                let sh_ds = 960.0f32;
                let sw_ds = sh_ds * tex.width() as f32 / tex.height() as f32;
                let xd = spr.position.x.resolve(sw_ds) + spr.y_offset * (1.0 - p);
                let mut dp = DrawParam::new()
                    .dest([dx(xd) - ox, dy(480.0) - oy])
                    .scale([ds(sw_ds) / tex.width() as f32, ds(sh_ds) / tex.height() as f32]);
                dp.color.a = alpha_s;
                tc.draw(tex, dp);
            }
        }
        tc.finish(ctx)?;
    }

    let hb = Image::new_canvas_image(ctx, w, h, 1);
    {
        let mut hc = Canvas::from_image(ctx, hb.clone(), Color::BLACK);
        let shader = ShaderBuilder::from_code(HBLUR_WGSL).build(ctx)?;
        hc.set_shader(&shader);
        hc.draw(&src, DrawParam::new().dest([0.0, 0.0]).scale([1.0, 1.0]));
        hc.set_default_shader();
        hc.finish(ctx)?;
    }

    let vs = ShaderBuilder::from_code(VBLUR_WGSL).build(ctx)?;
    c.set_shader(&vs);
    let mut bg_dp = DrawParam::new()
        .dest([ox, oy])
        .scale([ds(2560.0) / w as f32, ds(1440.0) / h as f32]);
    bg_dp.color = Color::new(0.08, 0.08, 0.08, 0.85 * alpha);
    c.draw(&hb, bg_dp);
    c.set_default_shader();
    Ok(())
}
