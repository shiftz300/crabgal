// Text and icon rendering via fontdue.
use std::collections::HashMap;

use ggez::graphics::{Image, ImageFormat};
use ggez::Context;

// ── Bootstrap Icons codepoints ──

pub const ICON_FILE_TEXT: char = '\u{F3B9}';
pub const ICON_ARROW_CLOCKWISE: char = '\u{F116}';
pub const ICON_PLAY: char = '\u{F4F5}';
pub const ICON_FAST_FORWARD: char = '\u{F7F4}';
pub const ICON_EYE_SLASH: char = '\u{F340}';
pub const ICON_EYE: char = '\u{F341}';
pub const ICON_LOCK: char = '\u{F47B}';
pub const ICON_UNLOCK: char = '\u{F600}';
pub const ICON_CHEVRON_DOUBLE_DOWN: char = '\u{F27E}';
pub const ICON_CHEVRON_DOUBLE_UP: char = '\u{F281}';
pub const ICON_FLOPPY2: char = '\u{F7E4}';
pub const ICON_FOLDER2_OPEN: char = '\u{F3D8}';
pub const ICON_SLIDERS2: char = '\u{F789}';
pub const ICON_HOUSE: char = '\u{F425}';

pub type TexCache = HashMap<(String, u32, u32), (Image, u32, u32)>;

pub fn icon_tex(
    ctx: &mut Context,
    cache: &mut TexCache,
    icon_font: &fontdue::Font,
    codepoint: char,
    px: f32,
) -> (Image, u32, u32) {
    let key = (format!("icon_{}", codepoint as u32), px as u32, 0);
    if let Some(cached) = cache.get(&key) {
        return cached.clone();
    }
    let (metrics, bitmap) = icon_font.rasterize(codepoint, px);
    let w = metrics.width.max(1) as u32;
    let h = metrics.height.max(1) as u32;
    let mut rgba = vec![0u8; w as usize * h as usize * 4];
    for row in 0..metrics.height {
        for col in 0..metrics.width {
            let bi = (row * metrics.width + col) as usize;
            if bi < bitmap.len() {
                let si = ((row as u32 * w + col as u32) * 4) as usize;
                rgba[si] = 255;
                rgba[si + 1] = 255;
                rgba[si + 2] = 255;
                rgba[si + 3] = bitmap[bi];
            }
        }
    }
    let img = Image::from_pixels(ctx, &rgba, ImageFormat::Rgba8UnormSrgb, w, h);
    cache.insert(key, (img.clone(), w, h));
    (img, w, h)
}

pub fn rasterize_text(
    font: &fontdue::Font,
    fallback: &fontdue::Font,
    text: &str,
    px: f32,
    max_width: f32,
) -> (Vec<u8>, u32, u32) {
    use fontdue::layout::{CoordinateSystem, Layout, LayoutSettings, TextStyle};
    let same_font = std::ptr::eq(font as *const _, fallback as *const _);
    let fonts: [&fontdue::Font; 2] = [font, fallback];
    let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
    let mw = if max_width > 0.0 { Some(max_width) } else { None };
    layout.reset(&LayoutSettings { x: 0.0, y: 0.0, max_width: mw, ..Default::default() });

    if same_font {
        layout.append(&[font], &TextStyle::new(text, px, 0));
    } else {
        let mut seg_start = 0;
        let chars: Vec<char> = text.chars().collect();
        let mut current_is_latin = !chars.is_empty() && fallback.lookup_glyph_index(chars[0]) != 0;
        for (i, &ch) in chars.iter().enumerate().skip(1) {
            let is_latin = fallback.lookup_glyph_index(ch) != 0;
            if is_latin != current_is_latin {
                let seg_text: String = chars[seg_start..i].iter().collect();
                let fi = if current_is_latin { 1 } else { 0 };
                layout.append(&fonts, &TextStyle::new(&seg_text, px, fi));
                seg_start = i;
                current_is_latin = is_latin;
            }
        }
        if seg_start < chars.len() {
            let seg_text: String = chars[seg_start..].iter().collect();
            let fi = if current_is_latin { 1 } else { 0 };
            layout.append(&fonts, &TextStyle::new(&seg_text, px, fi));
        }
    }

    let glyphs = layout.glyphs();
    if glyphs.is_empty() { return (vec![0u8; 4], 1, 1); }
    let w = (glyphs.iter().map(|g| g.x + g.width as f32)
        .max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap_or(1.0)).ceil() as u32;
    let h = (glyphs.iter().map(|g| g.y + g.height as f32)
        .max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap_or(1.0)).ceil() as u32;
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

pub fn get_text_tex(
    ctx: &mut Context,
    cache: &mut TexCache,
    font: &fontdue::Font,
    fallback: &fontdue::Font,
    text: &str,
    px: f32,
    max_width: f32,
) -> (Image, u32, u32) {
    let key = (text.to_string(), px as u32, max_width as u32);
    if let Some((img, w, h)) = cache.get(&key) {
        return (img.clone(), *w, *h);
    }
    let (rgba, w, h) = rasterize_text(font, fallback, text, px, max_width);
    let img = Image::from_pixels(ctx, &rgba, ImageFormat::Rgba8UnormSrgb, w, h);
    cache.insert(key, (img.clone(), w, h));
    (img, w, h)
}
