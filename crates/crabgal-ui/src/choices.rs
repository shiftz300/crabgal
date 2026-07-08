// Choice menu rendering.
use ggez::graphics::{Canvas, Color, DrawParam, Mesh, Rect};
use ggez::{Context, GameResult};

use crabgal_core::action::Choice;
use crabgal_render::text::{get_text_tex, TexCache};

pub fn draw_choices(
    ctx: &mut Context,
    c: &mut Canvas,
    text_cache: &mut TexCache,
    font: &fontdue::Font,
    fallback: &fontdue::Font,
    choices: &Option<Vec<Choice>>,
    sc: f32,
    ds: &dyn Fn(f32) -> f32,
    dx: &dyn Fn(f32) -> f32,
    dy: &dyn Fn(f32) -> f32,
) -> GameResult {
    let chs = match choices {
        Some(chs) => chs,
        None => return Ok(()),
    };
    let ov = Mesh::new_rectangle(ctx, ggez::graphics::DrawMode::fill(),
        Rect::new(dx(0.0), dy(0.0), ds(2560.0), ds(1440.0)),
        Color::new(0.0, 0.0, 0.0, 0.08))?;
    c.draw(&ov, DrawParam::default());
    let iw = ds(1280.0); let ih = ds(80.0); let gp = ds(14.0);
    let sy = dy(720.0) - (chs.len() as f32 * (ih + gp) - gp) / 2.0;
    for (i, ch) in chs.iter().enumerate() {
        let y = sy + i as f32 * (ih + gp);
        let cm = Mesh::new_rectangle(ctx, ggez::graphics::DrawMode::fill(),
            Rect::new(dx(640.0), y, iw, ih),
            Color::new(0.0, 0.0, 0.0, 0.25))?;
        c.draw(&cm, DrawParam::default());
        let (ci, cw, ch_h) = get_text_tex(ctx, text_cache, font, fallback,
            &ch.text, 42.0, 0.0);
        c.draw(&ci, DrawParam::new()
            .dest([dx(640.0) + (iw - cw as f32 * sc) / 2.0, y + (ih - ch_h as f32 * sc) / 2.0])
            .scale([sc, sc]));
    }
    Ok(())
}
