//! Rasterizado CPU de texto y stickers para la exportación (sustituye al
//! `fillText` del `<canvas>` web). Usa ab_glyph con las fuentes del sistema
//! (`%WINDIR%\Fonts`), por lo que no necesita red ni permisos.

use ab_glyph::{Font, FontArc, PxScale, ScaleFont};
use resvg::tiny_skia::{self, Color, FillRule, Paint, PathBuilder, Pixmap, PremultipliedColorU8, Transform};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

fn fonts_dir() -> std::path::PathBuf {
    std::env::var("WINDIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("C:\\Windows"))
        .join("Fonts")
}

fn load_font(candidates: &[&str]) -> Option<FontArc> {
    static CACHE: OnceLock<Mutex<HashMap<String, Option<FontArc>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let key = candidates.join("|");
    if let Some(f) = cache.lock().ok()?.get(&key) {
        return f.clone();
    }
    let dir = fonts_dir();
    let font = candidates.iter().find_map(|c| {
        let bytes = std::fs::read(dir.join(c)).ok()?;
        FontArc::try_from_vec(bytes).ok()
    });
    cache.lock().ok()?.insert(key, font.clone());
    font
}

/// Fuente del sistema más parecida a la familia/peso del estilo original.
pub fn font_for(family: &str, weight: f64) -> Option<FontArc> {
    let bold = weight >= 600.0;
    let f = family.to_lowercase();
    let list: &[&str] = if f.contains("serif") && !f.contains("sans") || f.contains("georgia") || f.contains("playfair") {
        if bold { &["georgiab.ttf", "timesbd.ttf"] } else { &["georgia.ttf", "times.ttf"] }
    } else if f.contains("mono") || f.contains("consol") || f.contains("courier") {
        if bold { &["consolab.ttf", "courbd.ttf"] } else { &["consola.ttf", "cour.ttf"] }
    } else if weight >= 850.0 {
        &["seguibl.ttf", "ariblk.ttf", "segoeuib.ttf", "arialbd.ttf"]
    } else if bold {
        &["segoeuib.ttf", "arialbd.ttf", "segoeui.ttf", "arial.ttf"]
    } else {
        &["segoeui.ttf", "arial.ttf"]
    };
    load_font(list).or_else(|| load_font(&["arial.ttf", "segoeui.ttf"]))
}

pub fn emoji_font() -> Option<FontArc> {
    load_font(&["seguiemj.ttf", "seguisym.ttf"])
}

pub fn parse_color(s: &str, alpha: f32) -> Color {
    let c = crate::paint::hex(s);
    let [r, g, b, a] = c.to_array();
    Color::from_rgba8(r, g, b, ((a as f32) * alpha.clamp(0.0, 1.0)).round() as u8)
}

fn num(v: &Value, k: &str, d: f64) -> f64 {
    v.get(k).and_then(Value::as_f64).unwrap_or(d)
}
fn st<'a>(v: &'a Value, k: &str, d: &'a str) -> &'a str {
    v.get(k).and_then(Value::as_str).unwrap_or(d)
}

struct Glyphs {
    /// (fuente, glifo posicionado) — se mezcla fuente principal y emoji.
    items: Vec<(FontArc, ab_glyph::Glyph, bool)>,
    width: f32,
    height: f32,
}

fn layout(text: &str, font: &FontArc, px: f32, letter_spacing: f32, align: &str) -> Glyphs {
    let emoji = emoji_font();
    let scale = PxScale::from(px);
    let sf = font.as_scaled(scale);
    let line_h = (sf.ascent() - sf.descent() + sf.line_gap()).max(px);
    let lines: Vec<&str> = text.split('\n').collect();
    let mut widths = Vec::new();
    let mut per_line: Vec<Vec<(FontArc, ab_glyph::Glyph, bool)>> = Vec::new();
    for line in &lines {
        let mut x = 0.0;
        let mut out = Vec::new();
        let mut prev: Option<ab_glyph::GlyphId> = None;
        for ch in line.chars() {
            // Los emoji se dibujan siempre con la fuente de emoji para obtener sus
            // capas de color COLR; selectores de variación y ZWJ no se pintan.
            let (f, id, is_emoji) = {
                let id = font.glyph_id(ch);
                let wants_emoji = is_emoji_char(ch) || (id.0 == 0 && !ch.is_whitespace());
                match &emoji {
                    Some(e) if wants_emoji && e.glyph_id(ch).0 != 0 => (e.clone(), e.glyph_id(ch), true),
                    _ if is_invisible_joiner(ch) => continue,
                    _ => (font.clone(), id, false),
                }
            };
            let fs = f.as_scaled(scale);
            if let Some(p) = prev {
                x += sf.kern(p, id);
            }
            out.push((f.clone(), id.with_scale_and_position(scale, ab_glyph::point(x, 0.0)), is_emoji));
            x += fs.h_advance(id) + letter_spacing;
            prev = Some(id);
        }
        widths.push(x);
        per_line.push(out);
    }
    let block_w = widths.iter().cloned().fold(0.0, f32::max);
    let mut items = Vec::new();
    for (i, mut line) in per_line.into_iter().enumerate() {
        let dx = match align {
            "left" => 0.0,
            "right" => block_w - widths[i],
            _ => (block_w - widths[i]) * 0.5,
        };
        let y = sf.ascent() + i as f32 * line_h;
        for (_, g, _) in line.iter_mut() {
            g.position.x += dx;
            g.position.y = y;
        }
        items.extend(line);
    }
    Glyphs { items, width: block_w, height: line_h * lines.len() as f32 }
}

fn is_emoji_char(ch: char) -> bool {
    matches!(ch as u32,
        0x1F000..=0x1FAFF | 0x2600..=0x27BF | 0x2B00..=0x2BFF | 0x2300..=0x23FF | 0x3030 | 0x303D | 0x3297 | 0x3299
        | 0x00A9 | 0x00AE | 0x203C | 0x2049 | 0x2122 | 0x2139 | 0x2194..=0x21AA)
}

/// ZWJ, selectores de variación y etiquetas: no deben dibujarse como «tofu».
fn is_invisible_joiner(ch: char) -> bool {
    matches!(ch as u32, 0x200D | 0xFE0E | 0xFE0F | 0xE0020..=0xE007F)
}

// ── Tablas de color COLR v0 + CPAL de Segoe UI Emoji ──

struct ColorFont {
    /// glifo base → (primera capa, nº de capas)
    bases: HashMap<u16, (usize, usize)>,
    /// capas: (glifo, índice de paleta; 0xFFFF = color del texto)
    layers: Vec<(u16, u16)>,
    /// paleta 0 en RGBA
    palette: Vec<[u8; 4]>,
}

fn be16(b: &[u8], o: usize) -> Option<u16> {
    Some(u16::from_be_bytes([*b.get(o)?, *b.get(o + 1)?]))
}
fn be32(b: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_be_bytes([*b.get(o)?, *b.get(o + 1)?, *b.get(o + 2)?, *b.get(o + 3)?]))
}

fn find_table<'a>(font: &'a [u8], tag: &[u8; 4]) -> Option<&'a [u8]> {
    let n = be16(font, 4)? as usize;
    for i in 0..n {
        let rec = 12 + i * 16;
        if font.get(rec..rec + 4)? == tag {
            let off = be32(font, rec + 8)? as usize;
            let len = be32(font, rec + 12)? as usize;
            return font.get(off..off + len);
        }
    }
    None
}

fn parse_color_font(font: &[u8]) -> Option<ColorFont> {
    let colr = find_table(font, b"COLR")?;
    let cpal = find_table(font, b"CPAL")?;
    let nbase = be16(colr, 2)? as usize;
    let base_off = be32(colr, 4)? as usize;
    let layer_off = be32(colr, 8)? as usize;
    let nlayers = be16(colr, 12)? as usize;
    let mut bases = HashMap::with_capacity(nbase);
    for i in 0..nbase {
        let o = base_off + i * 6;
        bases.insert(be16(colr, o)?, (be16(colr, o + 2)? as usize, be16(colr, o + 4)? as usize));
    }
    let mut layers = Vec::with_capacity(nlayers);
    for i in 0..nlayers {
        let o = layer_off + i * 4;
        layers.push((be16(colr, o)?, be16(colr, o + 2)?));
    }
    let nentries = be16(cpal, 2)? as usize;
    let rec_off = be32(cpal, 8)? as usize;
    let first = be16(cpal, 12)? as usize;
    let mut palette = Vec::with_capacity(nentries);
    for i in 0..nentries {
        let o = rec_off + (first + i) * 4;
        let (b, g, r, a) = (*cpal.get(o)?, *cpal.get(o + 1)?, *cpal.get(o + 2)?, *cpal.get(o + 3)?);
        palette.push([r, g, b, a]);
    }
    Some(ColorFont { bases, layers, palette })
}

fn emoji_colors() -> Option<&'static ColorFont> {
    static CF: OnceLock<Option<ColorFont>> = OnceLock::new();
    CF.get_or_init(|| {
        let bytes = std::fs::read(fonts_dir().join("seguiemj.ttf")).ok()?;
        parse_color_font(&bytes)
    })
    .as_ref()
}

fn blend_outline(data: &mut [PremultipliedColorU8], w: i32, h: i32, font: &FontArc, glyph: ab_glyph::Glyph, rgba: [u8; 4]) {
    let Some(og) = font.outline_glyph(glyph) else { return };
    let b = og.px_bounds();
    let c = Color::from_rgba8(rgba[0], rgba[1], rgba[2], rgba[3]).premultiply().to_color_u8();
    og.draw(|gx, gy, cov| {
        let x = b.min.x as i32 + gx as i32;
        let y = b.min.y as i32 + gy as i32;
        if x < 0 || y < 0 || x >= w || y >= h || cov <= 0.0 {
            return;
        }
        let idx = (y * w + x) as usize;
        let a = cov.min(1.0);
        let src = [c.red() as f32 * a, c.green() as f32 * a, c.blue() as f32 * a, c.alpha() as f32 * a];
        let dst = data[idx];
        let inv = 1.0 - src[3] / 255.0;
        let px = PremultipliedColorU8::from_rgba(
            (src[0] + dst.red() as f32 * inv).round().min(255.0) as u8,
            (src[1] + dst.green() as f32 * inv).round().min(255.0) as u8,
            (src[2] + dst.blue() as f32 * inv).round().min(255.0) as u8,
            (src[3] + dst.alpha() as f32 * inv).round().min(255.0) as u8,
        );
        if let Some(px) = px {
            data[idx] = px;
        }
    });
}

/// Glifos monocromos (contornos, sombras, brillos).
fn draw_glyphs(pm: &mut Pixmap, g: &Glyphs, ox: f32, oy: f32, color: Color) {
    draw_glyphs_impl(pm, g, ox, oy, color, false);
}

/// Relleno final: los emoji usan sus capas de color (como `fillText` en el navegador).
fn draw_glyphs_color(pm: &mut Pixmap, g: &Glyphs, ox: f32, oy: f32, color: Color) {
    draw_glyphs_impl(pm, g, ox, oy, color, true);
}

fn draw_glyphs_impl(pm: &mut Pixmap, g: &Glyphs, ox: f32, oy: f32, color: Color, colored: bool) {
    let (w, h) = (pm.width() as i32, pm.height() as i32);
    let fg = color.to_color_u8();
    let fg = [fg.red(), fg.green(), fg.blue(), fg.alpha()];
    let colors = if colored { emoji_colors() } else { None };
    let data = pm.pixels_mut();
    for (font, glyph, is_emoji) in &g.items {
        let mut glyph = glyph.clone();
        glyph.position.x += ox;
        glyph.position.y += oy;
        if let (true, Some(cf)) = (*is_emoji, colors) {
            if let Some(&(first, n)) = cf.bases.get(&glyph.id.0) {
                for &(lg, pi) in cf.layers.iter().skip(first).take(n) {
                    let col = if pi == 0xFFFF {
                        fg
                    } else {
                        let [r, gg, b, a] = cf.palette.get(pi as usize).copied().unwrap_or([255, 255, 255, 255]);
                        [r, gg, b, ((a as f32) * fg[3] as f32 / 255.0).round() as u8]
                    };
                    let layer = ab_glyph::GlyphId(lg).with_scale_and_position(glyph.scale, glyph.position);
                    blend_outline(data, w, h, font, layer, col);
                }
                continue;
            }
        }
        blend_outline(data, w, h, font, glyph, fg);
    }
}

fn rounded_rect(x: f32, y: f32, w: f32, h: f32, r: f32) -> Option<tiny_skia::Path> {
    let r = r.min(w / 2.0).min(h / 2.0).max(0.0);
    let mut pb = PathBuilder::new();
    pb.move_to(x + r, y);
    pb.line_to(x + w - r, y);
    pb.quad_to(x + w, y, x + w, y + r);
    pb.line_to(x + w, y + h - r);
    pb.quad_to(x + w, y + h, x + w - r, y + h);
    pb.line_to(x + r, y + h);
    pb.quad_to(x, y + h, x, y + h - r);
    pb.line_to(x, y + r);
    pb.quad_to(x, y, x + r, y);
    pb.close();
    pb.finish()
}

const DIRS: [(f32, f32); 12] = [
    (1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0),
    (0.707, 0.707), (-0.707, 0.707), (0.707, -0.707), (-0.707, -0.707),
    (0.707, 0.0), (-0.707, 0.0), (0.0, 0.707), (0.0, -0.707),
];

/// Rasteriza el bloque de texto de un clip. `scale` = alto del lienzo / 1080.
/// Devuelve el pixmap y su desplazamiento relativo al centro (x, y en %).
pub fn render_text(text: &Value, scale: f32) -> Option<Pixmap> {
    let mut body = st(text, "content", "").to_string();
    if body.trim().is_empty() {
        return None;
    }
    match st(text, "textCase", "none") {
        "upper" => body = body.to_uppercase(),
        "lower" => body = body.to_lowercase(),
        _ => {}
    }
    let px = (num(text, "size", 72.0) as f32 * scale).max(6.0);
    let font = font_for(st(text, "font", "Inter"), num(text, "weight", 800.0))?;
    let spacing = num(text, "letterSpacing", 0.0) as f32 * px;
    let mut g = layout(&body, &font, px, spacing, st(text, "align", "center"));
    // Efectos de texto por letra (`fx_text`): sacudida, transformar, destellos, ruta.
    let gfx_margin = match text.get("_glyphFx") {
        Some(fx) => {
            let n = g.items.len();
            let mut kept = Vec::with_capacity(n);
            for (i, (f, mut gl, e)) in std::mem::take(&mut g.items).into_iter().enumerate() {
                let (dx, dy, sc, on) = crate::fx_text::glyph_motion(fx, i, n, gl.position.x, px);
                if !on {
                    continue;
                }
                gl.position.x += dx;
                gl.position.y += dy;
                if (sc - 1.0).abs() > 0.001 {
                    gl.scale = PxScale { x: gl.scale.x * sc, y: gl.scale.y * sc };
                }
                kept.push((f, gl, e));
            }
            g.items = kept;
            crate::fx_text::glyph_margin(fx, px)
        }
        None => 0.0,
    };

    let stroke_w = num(text, "strokeWidth", 0.0) as f32 * px * 0.6;
    let glow = text.get("glow").and_then(Value::as_bool).unwrap_or(false);
    let glow_r = if glow { (num(text, "glowBlur", 0.35) as f32 * px).max(1.0) } else { 0.0 };
    let bg = st(text, "background", "transparent");
    let has_bg = bg != "transparent" && !bg.is_empty();
    let pad = if has_bg { num(text, "backgroundPadding", 0.22) as f32 * px } else { 0.0 };
    let margin = pad + stroke_w + glow_r + px * 0.15 + gfx_margin;
    let w = (g.width + margin * 2.0).ceil().max(1.0) as u32;
    let h = (g.height + margin * 2.0).ceil().max(1.0) as u32;
    let mut pm = Pixmap::new(w, h)?;
    let ox = margin;
    let oy = margin;

    if has_bg {
        let alpha = num(text, "backgroundAlpha", 1.0) as f32;
        let radius = num(text, "backgroundRadius", 0.16) as f32 * px;
        if let Some(path) = rounded_rect(ox - pad, oy - pad, g.width + pad * 2.0, g.height + pad * 2.0, radius) {
            let mut paint = Paint::default();
            paint.set_color(parse_color(bg, alpha));
            paint.anti_alias = true;
            pm.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
        }
    }
    if glow {
        let c = parse_color(st(text, "glowColor", "#00e5d0"), 0.4);
        for (dx, dy) in DIRS.iter().take(8) {
            draw_glyphs(&mut pm, &g, ox + dx * glow_r * 0.6, oy + dy * glow_r * 0.6, c);
        }
    }
    if text.get("shadow").and_then(Value::as_bool).unwrap_or(false) {
        let c = parse_color(st(text, "shadowColor", "#000000"), num(text, "shadowAlpha", 0.58) as f32);
        let dist = num(text, "shadowDistance", 0.05) as f32 * px;
        let ang = (num(text, "shadowAngle", 90.0) as f32).to_radians();
        draw_glyphs(&mut pm, &g, ox + ang.cos() * dist, oy + ang.sin() * dist, c);
    }
    if stroke_w > 0.0 {
        let c = parse_color(st(text, "strokeColor", "#000000"), num(text, "strokeAlpha", 1.0) as f32);
        let r = stroke_w.max(1.0);
        for (dx, dy) in DIRS.iter() {
            draw_glyphs(&mut pm, &g, ox + dx * r, oy + dy * r, c);
        }
    }
    let opacity = num(text, "opacity", 1.0) as f32;
    let fill = parse_color(st(text, "color", "#ffffff"), opacity);
    // Karaoke (`karaoke::apply`): la palabra activa lleva otro color y, si se
    // pide, crece un poco; el resto del párrafo se queda como está.
    let kara = text.get("_karaoke").filter(|_| text.get("_glyphFx").is_none());
    let word_of = kara.and_then(|_| word_indices(&body, g.items.len()));
    match (kara, word_of) {
        (Some(k), Some(word_of)) => {
            let active = k.get("idx").and_then(Value::as_u64).unwrap_or(0) as usize;
            let pop = k.get("pop").and_then(Value::as_bool).unwrap_or(false);
            let hot = parse_color(k.get("color").and_then(Value::as_str).unwrap_or("#ffe600"), opacity);
            let (mut rest, mut lit) = (Vec::new(), Vec::new());
            for (item, w) in g.items.iter().zip(&word_of) {
                if *w == active {
                    let mut it = item.clone();
                    if pop {
                        it.1.scale = PxScale { x: it.1.scale.x * 1.12, y: it.1.scale.y * 1.12 };
                        it.1.position.y -= px * 0.05;
                    }
                    lit.push(it);
                } else {
                    rest.push(item.clone());
                }
            }
            draw_glyphs_color(&mut pm, &Glyphs { items: rest, width: g.width, height: g.height }, ox, oy, fill);
            draw_glyphs_color(&mut pm, &Glyphs { items: lit, width: g.width, height: g.height }, ox, oy, hot);
        }
        _ => draw_glyphs_color(&mut pm, &g, ox, oy, fill),
    }
    Some(pm)
}

/// Palabra (por orden) a la que pertenece cada glifo que `layout` coloca, o
/// `usize::MAX` para espacios. `None` si no cuadra con los glifos reales.
fn word_indices(body: &str, glyph_count: usize) -> Option<Vec<usize>> {
    let mut out = Vec::with_capacity(glyph_count);
    let (mut next, mut cur) = (0usize, None::<usize>);
    for ch in body.chars() {
        if is_invisible_joiner(ch) {
            continue;
        }
        if ch == '\n' {
            cur = None;
            continue;
        }
        if ch.is_whitespace() {
            cur = None;
            out.push(usize::MAX);
        } else {
            let w = *cur.get_or_insert_with(|| {
                next += 1;
                next - 1
            });
            out.push(w);
        }
    }
    (out.len() == glyph_count).then_some(out)
}

/// Rasteriza un sticker de emoji/texto con las capas de color COLR/CPAL de
/// Segoe UI Emoji (emoji a todo color, igual que en el navegador).
pub fn render_sticker(sticker: &Value, scale: f32) -> Option<Pixmap> {
    let content = sticker
        .get("content")
        .or_else(|| sticker.get("emoji"))
        .or_else(|| sticker.get("text"))
        .and_then(Value::as_str)
        .unwrap_or("");
    if content.is_empty() {
        return None;
    }
    let px = (num(sticker, "size", 220.0) as f32 * scale).max(8.0);
    let font = emoji_font().or_else(|| font_for("sans", 700.0))?;
    let g = layout(content, &font, px, 0.0, "center");
    let bg = st(sticker, "background", "transparent");
    let has_bg = bg != "transparent" && !bg.is_empty();
    let side = (px * if has_bg { 1.45 } else { 1.18 }).max(g.width.max(g.height)).ceil() as u32;
    let mut pm = Pixmap::new(side, side)?;
    if has_bg {
        let shape = if st(sticker, "shape", "none") == "circle" {
            tiny_skia::PathBuilder::from_circle(side as f32 / 2.0, side as f32 / 2.0, side as f32 / 2.0)
        } else {
            rounded_rect(0.0, 0.0, side as f32, side as f32, side as f32 * 0.22)
        };
        if let Some(path) = shape {
            let mut paint = Paint::default();
            paint.set_color(parse_color(bg, 1.0));
            paint.anti_alias = true;
            pm.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
        }
    }
    let color = parse_color(st(sticker, "color", "#ffffff"), 1.0);
    draw_glyphs_color(&mut pm, &g, (side as f32 - g.width) / 2.0, (side as f32 - g.height) / 2.0, color);
    Some(pm)
}

/// Renderiza un único glifo (emoji a color o símbolo monocromo) centrado en un
/// pixmap cuadrado ajustado, para las partículas del compositor (`drawParticles`).
/// Reutiliza la maquinaria de emoji a color COLR/CPAL.
pub fn render_glyph(glyph: &str, px: f32, color: &str) -> Option<Pixmap> {
    if glyph.is_empty() {
        return None;
    }
    let px = px.max(6.0);
    let font = emoji_font().or_else(|| font_for("sans", 700.0))?;
    let g = layout(glyph, &font, px, 0.0, "center");
    let side = (g.width.max(g.height).max(px).ceil() as u32).saturating_add(2).max(1);
    let mut pm = Pixmap::new(side, side)?;
    let col = parse_color(color, 1.0);
    draw_glyphs_color(&mut pm, &g, (side as f32 - g.width) / 2.0, (side as f32 - g.height) / 2.0, col);
    Some(pm)
}

// ── Animaciones de texto (`textAnimationState` de renderer.js) ──

/// Estado de animación de un texto en un instante: desplazamientos en
/// unidades de tamaño de fuente, escala, giro en grados y fracción visible
/// (máquina de escribir).
#[derive(Clone, Copy, Debug)]
pub struct TextAnim {
    pub alpha: f32,
    pub dx: f32,
    pub dy: f32,
    pub scale: f32,
    pub rotate: f32,
    pub reveal: f32,
}

fn anim_spec(id: &str) -> Option<(&'static str, f64)> {
    crate::text_styles::text_animations().iter().find_map(|a| {
        (a.get("id").and_then(Value::as_str) == Some(id)).then(|| {
            let phase = match a.get("phase").and_then(Value::as_str).unwrap_or("in") {
                "out" => "out",
                "loop" => "loop",
                "caption" => "caption",
                _ => "in",
            };
            (phase, a.get("duration").and_then(Value::as_f64).unwrap_or(0.6))
        })
    })
}

fn ease_out_cubic(v: f64) -> f64 {
    1.0 - (1.0 - v).powi(3)
}

fn ease_out_back(v: f64) -> f64 {
    let c1 = 1.70158;
    let c3 = c1 + 1.0;
    1.0 + c3 * (v - 1.0).powi(3) + c1 * (v - 1.0).powi(2)
}

/// `local`: segundos desde el inicio del clip; `duration`: duración del clip.
pub fn text_anim(text: &Value, local: f64, duration: f64) -> Option<TextAnim> {
    let id = st(text, "animation", "none");
    if id.is_empty() || id == "none" {
        return None;
    }
    let (spec_phase, spec_dur) = anim_spec(id)?;
    let phase = text.get("animationPhase").and_then(Value::as_str).unwrap_or(spec_phase);
    let ad = num(text, "animationDuration", spec_dur).max(0.1);
    let progress = match phase {
        "in" => (local / ad).clamp(0.0, 1.0),
        "out" => ((duration.max(0.001) - local) / ad).clamp(0.0, 1.0),
        _ => 1.0,
    };
    let e = ease_out_cubic(progress);
    let c01 = |v: f64| v.clamp(0.0, 1.0);
    let t = local;
    let mut s = TextAnim { alpha: 1.0, dx: 0.0, dy: 0.0, scale: 1.0, rotate: 0.0, reveal: 1.0 };
    let (mut a, mut dx, mut dy, mut sc, mut rot, mut rev) = (1.0, 0.0, 0.0, 1.0, 0.0, 1.0);
    match id {
        "in-fade" | "out-fade" | "out-blur" => a = e,
        "in-slide-up" => { a = e; dy = (1.0 - e) * 0.6; }
        "in-slide-down" => { a = e; dy = -(1.0 - e) * 0.6; }
        "in-slide-left" => { a = e; dx = (1.0 - e) * 0.7; }
        "in-slide-right" => { a = e; dx = -(1.0 - e) * 0.7; }
        "in-zoom" => { a = e; sc = 0.7 + e * 0.3; }
        "in-pop" => { a = c01(e * 3.0); sc = 0.3 + ease_out_back(e) * 0.7; }
        "in-typewriter" => rev = e,
        "in-spin" => { a = e; rot = (1.0 - e) * -22.0; }
        "in-blur" => a = c01(e * 1.5),
        "in-bounce" => { a = c01(e * 1.5); sc = 0.55 + ease_out_back(e) * 0.45; }
        "in-diagonal" => { a = e; dx = (1.0 - e) * 0.45; dy = -(1.0 - e) * 0.35; }
        "in-flip" => { a = e; sc = 0.25 + e * 0.75; rot = (1.0 - e) * 160.0; }
        "out-slide-up" => { a = e; dy = -(1.0 - e) * 0.45; }
        "out-slide-down" => { a = e; dy = (1.0 - e) * 0.45; }
        "out-slide-left" => { a = e; dx = -(1.0 - e) * 0.55; }
        "out-slide-right" => { a = e; dx = (1.0 - e) * 0.55; }
        "out-zoom" => { a = e; sc = 1.0 + (1.0 - e) * 0.5; }
        "out-spin" => { a = e; rot = (1.0 - e) * -150.0; sc = 0.7 + e * 0.3; }
        "loop-wave" => dy = (t * 3.2).sin() * 0.08,
        "loop-pulse" => sc = 1.0 + (t * 4.4).sin() * 0.035,
        "loop-shake" => dx = (t * 38.0).sin() * 0.04 + (t * 23.0).sin() * 0.02,
        "loop-swing" => rot = (t * 2.6).sin() * 3.5,
        "loop-float" => dy = (t * 1.9).sin() * 0.06,
        "loop-blink" => a = 0.55 + ((t * 5.0).sin() + 1.0) * 0.225,
        "loop-orbit" => { dx = (t * 2.8).cos() * 0.08; dy = (t * 2.8).sin() * 0.08; }
        "loop-wiggle" => rot = (t * 7.0).sin() * 4.5,
        _ => {}
    }
    s.alpha = a as f32;
    s.dx = dx as f32;
    s.dy = dy as f32;
    s.scale = sc as f32;
    s.rotate = rot as f32;
    s.reveal = rev as f32;
    Some(s)
}

/// Copia del texto recortada por la máquina de escribir (`reveal` < 1).
pub fn reveal_text(text: &Value, reveal: f32) -> Value {
    let mut v = text.clone();
    if reveal < 0.999 {
        let content = st(text, "content", "");
        let n = content.chars().count();
        let k = ((n as f32) * reveal.clamp(0.0, 1.0)).floor() as usize;
        let cut: String = content.chars().take(k).collect();
        if let Some(o) = v.as_object_mut() {
            o.insert("content".into(), Value::String(cut));
        }
    }
    v
}

/// Tamaño de fuente del texto en píxeles para `scale`.
pub fn text_font_px(text: &Value, scale: f32) -> f32 {
    (num(text, "size", 72.0) as f32 * scale).max(6.0)
}
