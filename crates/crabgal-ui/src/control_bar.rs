// Control bar — WebGAL-style two-row right-aligned buttons.
use ggez::graphics::{Canvas, Color, DrawParam, Mesh, Rect};
use ggez::{Context, GameResult};

use crabgal_render::text::{get_text_tex, icon_tex, TexCache};
use crabgal_render::text::{
    ICON_ARROW_CLOCKWISE, ICON_CHEVRON_DOUBLE_DOWN, ICON_CHEVRON_DOUBLE_UP,
    ICON_EYE, ICON_EYE_SLASH, ICON_FAST_FORWARD, ICON_FILE_TEXT,
    ICON_FLOPPY2, ICON_FOLDER2_OPEN, ICON_HOUSE,
    ICON_LOCK, ICON_PLAY, ICON_SLIDERS2, ICON_UNLOCK,
};

pub fn draw_control_bar(
    ctx: &mut Context,
    c: &mut Canvas,
    text_cache: &mut TexCache,
    font: &fontdue::Font,
    fallback: &fontdue::Font,
    icon_font: &fontdue::Font,
    controls_visible: bool,
    textbox_visible: bool,
    menu_open: bool,
    menu_panel_is_some: bool,
    hover_btn: &mut Option<usize>,
    sc: f32,
    ds: &dyn Fn(f32) -> f32,
    dx: &dyn Fn(f32) -> f32,
    dy: &dyn Fn(f32) -> f32,
) -> GameResult {
    if !controls_visible || menu_panel_is_some || menu_open {
        *hover_btn = None;
        return Ok(());
    }

    let bar_h = ds(70.0);
    let upper_design_y = 1440.0 - bar_h / sc + 4.0;
    let lower_design_y = upper_design_y - bar_h / sc - 14.0;
    let upper_bar_y = dy(upper_design_y);
    let lower_bar_y = dy(lower_design_y);

    let mpos = ctx.mouse.position();
    let icon_px = 32.0f32; let label_px = 18.0f32;
    let pad_x = ds(12.0);
    let gap = ds(4.0);
    let mut hi: Option<usize> = None;

    // Upper row
    let hide_icon = if textbox_visible { ICON_EYE_SLASH } else { ICON_EYE };
    let lock_icon = if controls_visible { ICON_LOCK } else { ICON_UNLOCK };
    let upper_items: [(char, &str); 6] = [
        (ICON_FILE_TEXT, "Backlog"),
        (ICON_ARROW_CLOCKWISE, "Replay"),
        (ICON_PLAY, "Auto"),
        (ICON_FAST_FORWARD, "Skip"),
        (hide_icon, "Hide"),
        (lock_icon, "Lock"),
    ];
    let upper_w: f32 = upper_items.iter().map(|(icon, label)| {
        let (_, iw, _) = icon_tex(ctx, text_cache, icon_font, *icon, icon_px);
        let (_, lw, _) = get_text_tex(ctx, text_cache, font, fallback, label, label_px, 0.0);
        iw as f32 * sc + ds(6.0) + lw as f32 * sc + pad_x * 2.0
    }).sum::<f32>() + gap * (upper_items.len() - 1) as f32;
    let mut x = dx(2560.0 - 40.0) - upper_w;

    for (idx, (icon, label)) in upper_items.iter().enumerate() {
        let (it, iw, _) = icon_tex(ctx, text_cache, icon_font, *icon, icon_px);
        let (lt, lw, lh) = get_text_tex(ctx, text_cache, font, fallback, label, label_px, 0.0);
        let bw = iw as f32 * sc + ds(6.0) + lw as f32 * sc + pad_x * 2.0;
        let hover = mpos.x >= x && mpos.x <= x + bw
            && mpos.y >= upper_bar_y && mpos.y <= upper_bar_y + bar_h;
        if hover { hi = Some(idx); }
        if hover {
            let hb = Mesh::new_rectangle(ctx, ggez::graphics::DrawMode::fill(),
                Rect::new(x, upper_bar_y + ds(4.0), bw, bar_h - ds(8.0)),
                Color::new(1.0, 1.0, 1.0, 0.06))?;
            c.draw(&hb, DrawParam::default());
        }
        c.draw(&it, DrawParam::new()
            .dest([x + pad_x, upper_bar_y + (bar_h - iw as f32 * sc) / 2.0])
            .scale([sc, sc]).color(Color::new(1.0, 1.0, 1.0, 0.67)));
        c.draw(&lt, DrawParam::new()
            .dest([x + pad_x + iw as f32 * sc + ds(6.0), upper_bar_y + (bar_h - lh as f32 * sc) / 2.0])
            .scale([sc, sc]).color(Color::new(1.0, 1.0, 1.0, 0.67)));
        x += bw + gap;
    }

    // Lower row
    let lower_items: [(char, &str); 6] = [
        (ICON_CHEVRON_DOUBLE_DOWN, "Q.Save"),
        (ICON_CHEVRON_DOUBLE_UP, "Q.Load"),
        (ICON_FLOPPY2, "Save"),
        (ICON_FOLDER2_OPEN, "Load"),
        (ICON_SLIDERS2, "Config"),
        (ICON_HOUSE, "Title"),
    ];
    let lower_w: f32 = lower_items.iter().map(|(icon, label)| {
        let (_, iw, _) = icon_tex(ctx, text_cache, icon_font, *icon, icon_px);
        let (_, lw, _) = get_text_tex(ctx, text_cache, font, fallback, label, label_px, 0.0);
        iw as f32 * sc + ds(6.0) + lw as f32 * sc + pad_x * 2.0
    }).sum::<f32>() + gap * (lower_items.len() - 1) as f32;
    let mut x = dx(2560.0 - 40.0) - lower_w;

    for (idx, (icon, label)) in lower_items.iter().enumerate() {
        let (it, iw, _) = icon_tex(ctx, text_cache, icon_font, *icon, icon_px);
        let (lt, lw, lh) = get_text_tex(ctx, text_cache, font, fallback, label, label_px, 0.0);
        let bw = iw as f32 * sc + ds(6.0) + lw as f32 * sc + pad_x * 2.0;
        let btn_idx = upper_items.len() + idx;
        let hover = mpos.x >= x && mpos.x <= x + bw
            && mpos.y >= lower_bar_y && mpos.y <= lower_bar_y + bar_h;
        if hover { hi = Some(btn_idx); }
        if hover {
            let hb = Mesh::new_rectangle(ctx, ggez::graphics::DrawMode::fill(),
                Rect::new(x, lower_bar_y + ds(4.0), bw, bar_h - ds(8.0)),
                Color::new(1.0, 1.0, 1.0, 0.06))?;
            c.draw(&hb, DrawParam::default());
        }
        c.draw(&it, DrawParam::new()
            .dest([x + pad_x, lower_bar_y + (bar_h - iw as f32 * sc) / 2.0])
            .scale([sc, sc]).color(Color::new(1.0, 1.0, 1.0, 0.67)));
        c.draw(&lt, DrawParam::new()
            .dest([x + pad_x + iw as f32 * sc + ds(6.0), lower_bar_y + (bar_h - lh as f32 * sc) / 2.0])
            .scale([sc, sc]).color(Color::new(1.0, 1.0, 1.0, 0.67)));
        x += bw + gap;
    }
    *hover_btn = hi;
    Ok(())
}
