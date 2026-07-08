// Menu overlay: blurred background, menu bar, save/load, backlog panels.
use std::collections::HashMap;

use ggez::graphics::{self, Canvas, Color, DrawParam, Image, Mesh, Rect};
use ggez::{Context, GameResult};

use crabgal_core::state::Sprite;
use crabgal_core::MenuPanel;
use crabgal_render::blur::draw_blur_overlay;
use crabgal_render::text::{get_text_tex, TexCache};

pub fn draw_menu_overlay(
    ctx: &mut Context,
    c: &mut Canvas,
    textures: &HashMap<String, Image>,
    text_cache: &mut TexCache,
    font: &fontdue::Font,
    fallback: &fontdue::Font,
    bg_name: &Option<String>,
    sprites: &Vec<(String, Sprite)>,
    menu_fade: f32,
    menu_panel: Option<MenuPanel>,
    menu_page: u32,
    dialogue_log: &Vec<(String, String)>,
    sc: f32,
    ox: f32,
    oy: f32,
    ds: &dyn Fn(f32) -> f32,
    dx: &dyn Fn(f32) -> f32,
    dy: &dyn Fn(f32) -> f32,
) -> GameResult {
    let fade = menu_fade;
    if fade < 0.001 { return Ok(()); }

    draw_blur_overlay(ctx, c, textures, bg_name, sprites, sc, ox, oy, ds, dx, dy, fade)?;

    let bar_y = dy(1440.0 * 0.9); let bar_h = ds(144.0);
    let bm = Mesh::new_rectangle(ctx, graphics::DrawMode::fill(),
        Rect::new(dx(0.0), bar_y, ds(2560.0), bar_h),
        Color::new(0.0, 0.0, 0.0, 0.65 * fade))?;
    c.draw(&bm, DrawParam::default());

    for (lbl, bx) in [("Save", 460.0f32), ("Load", 810.0), ("Options", 1160.0), ("Close", 2300.0)] {
        let (bt, _, _) = get_text_tex(ctx, text_cache, font, fallback, lbl, 44.0, 0.0);
        c.draw(&bt, DrawParam::new()
            .dest([dx(bx), bar_y + ds(30.0)])
            .scale([sc, sc])
            .color(Color::new(1.0, 1.0, 1.0, 0.9 * fade)));
    }

    match menu_panel {
        Some(MenuPanel::Save) | Some(MenuPanel::Load) => {
            draw_save_load_panel(ctx, c, text_cache, font, fallback, menu_page, sc, ds, dx, dy)?;
        }
        Some(MenuPanel::Backlog) => {
            draw_backlog_panel(ctx, c, text_cache, font, fallback, dialogue_log, sc, ds, dx, dy)?;
        }
        _ => {}
    }
    Ok(())
}

fn draw_save_load_panel(
    ctx: &mut Context,
    c: &mut Canvas,
    text_cache: &mut TexCache,
    font: &fontdue::Font,
    fallback: &fontdue::Font,
    menu_page: u32,
    sc: f32,
    ds: &dyn Fn(f32) -> f32,
    dx: &dyn Fn(f32) -> f32,
    dy: &dyn Fn(f32) -> f32,
) -> GameResult {
    let ps = (menu_page - 1) * 10 + 1;
    for p in 1..=20u32 {
        let px = dx(150.0 + (p - 1) as f32 * 80.0);
        let hl = p == menu_page;
        let cl = if hl { Color::new(1.0, 1.0, 1.0, 0.7) } else { Color::new(1.0, 1.0, 1.0, 0.3) };
        let pm = Mesh::new_rectangle(ctx, ggez::graphics::DrawMode::fill(),
            Rect::new(px - ds(10.0), dy(40.0), ds(60.0), ds(50.0)), cl)?;
        c.draw(&pm, DrawParam::default());
        let lbl = format!("{}", p);
        let (lt, lw, _) = get_text_tex(ctx, text_cache, font, fallback, &lbl, 36.0, 0.0);
        c.draw(&lt, DrawParam::new()
            .dest([px + ds(20.0) - lw as f32 * sc / 2.0, dy(48.0)])
            .scale([sc, sc]));
    }
    let cw = ds(448.0); let ch = ds(648.0);
    let sx0 = dx(120.0); let sy0 = dy(130.0);
    for i in 0..10u32 {
        let sl = ps + i;
        let col = i % 5; let row = i / 5;
        let sx = sx0 + col as f32 * (cw + ds(52.0));
        let sy = sy0 + row as f32 * (ch + ds(30.0));
        let cm = Mesh::new_rectangle(ctx, ggez::graphics::DrawMode::fill(),
            Rect::new(sx, sy, cw, ch), Color::new(0.0, 0.0, 0.0, 0.2))?;
        c.draw(&cm, DrawParam::default());
        let idx = format!("{}", sl);
        let (it, _, _) = get_text_tex(ctx, text_cache, font, fallback, &idx, 28.0, 0.0);
        c.draw(&it, DrawParam::new()
            .dest([sx + ds(10.0), sy + ds(8.0)])
            .scale([sc, sc])
            .color(Color::new(1.0, 1.0, 1.0, 0.9)));
    }
    Ok(())
}

fn draw_backlog_panel(
    ctx: &mut Context,
    c: &mut Canvas,
    text_cache: &mut TexCache,
    font: &fontdue::Font,
    fallback: &fontdue::Font,
    dialogue_log: &Vec<(String, String)>,
    sc: f32,
    ds: &dyn Fn(f32) -> f32,
    dx: &dyn Fn(f32) -> f32,
    dy: &dyn Fn(f32) -> f32,
) -> GameResult {
    let log_entries: Vec<_> = dialogue_log.iter().rev().take(20).collect();
    let box_x = dx(80.0); let box_w = ds(2400.0);
    let box_y = dy(100.0); let box_h = ds(1300.0);
    let box_bg = Mesh::new_rectangle(ctx, ggez::graphics::DrawMode::fill(),
        Rect::new(box_x, box_y, box_w, box_h), Color::new(0.0, 0.0, 0.0, 0.4))?;
    c.draw(&box_bg, DrawParam::default());
    let mut cy = box_y + box_h - ds(80.0);
    for (speaker, text) in &log_entries {
        let line = if speaker.is_empty() { text.clone() } else { format!("{}: {}", speaker, text) };
        let (lt, _, lh) = get_text_tex(ctx, text_cache, font, fallback, &line, 36.0, box_w / sc - ds(60.0));
        cy -= lh as f32 * sc + ds(18.0);
        if cy < box_y + ds(20.0) { break; }
        c.draw(&lt, DrawParam::new()
            .dest([box_x + ds(30.0), cy])
            .scale([sc, sc])
            .color(Color::new(1.0, 1.0, 1.0, 0.85)));
    }
    Ok(())
}
