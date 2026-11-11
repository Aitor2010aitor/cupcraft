//! Pintura auxiliar: colores hexadecimales, gradientes redondeados, la
//! escena de miniatura (`--scene` de `styles.css`), la aproximación de los
//! filtros CSS de la línea y el dibujado de texto de la vista previa.

use crate::theme;
use cupcraft_core::{filter_by_key, Adjust, Clip, ClipKind, FILTERS};
use eframe::egui::{
    Color32, CornerRadius, FontId, Painter, Pos2, Rect, Stroke, StrokeKind, Vec2,
};
use serde_json::Value;

// ---------------------------------------------------------------------------
// Colores
// ---------------------------------------------------------------------------

/// `#rgb`, `#rrggbb` o `#rrggbbaa` → [`Color32`].
pub fn hex(s: &str) -> Color32 {
    let s = s.trim().trim_start_matches('#');
    let pair = |i: usize| {
        s.get(i..i + 2)
            .and_then(|v| u8::from_str_radix(v, 16).ok())
            .unwrap_or(0)
    };
    match s.len() {
        3 if s.is_ascii() => {
            let d = |i: usize| (s.as_bytes()[i] as char).to_digit(16).unwrap_or(0) as u8;
            Color32::from_rgb(d(0) * 17, d(1) * 17, d(2) * 17)
        }
        6 => Color32::from_rgb(pair(0), pair(2), pair(4)),
        8 => Color32::from_rgba_unmultiplied(pair(0), pair(2), pair(4), pair(6)),
        _ => Color32::WHITE,
    }
}

pub fn with_alpha(c: Color32, a: f32) -> Color32 {
    let [r, g, b, _] = c.to_array();
    Color32::from_rgba_unmultiplied(r, g, b, (a.clamp(0.0, 1.0) * 255.0).round() as u8)
}

pub fn lerp(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let [ar, ag, ab, aa] = a.to_array();
    let [br, bg, bb, ba] = b.to_array();
    let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_unmultiplied(m(ar, br), m(ag, bg), m(ab, bb), m(aa, ba))
}

/// Rectángulo redondeado de un solo color.
pub fn round_fill(p: &Painter, rect: Rect, radius: u8, fill: Color32) {
    p.rect_filled(rect, CornerRadius::from(radius), fill);
}

/// Rectángulo redondeado con degradado vertical.
///
/// Se pinta por bandas: la primera y la última llevan los redondeos de su
/// lado, de modo que el conjunto conserva las esquinas curvas.
pub fn rounded_gradient(p: &Painter, rect: Rect, radius: u8, top: Color32, bot: Color32) {
    let h = rect.height();
    let r = radius as f32;
    if rect.width() <= 0.0 || h <= 0.0 {
        return;
    }
    if radius == 0 || h < r * 2.0 + 1.0 {
        p.rect_filled(rect, CornerRadius::from(radius), lerp(top, bot, 0.5));
        return;
    }
    let mut bands: Vec<(Rect, f32, u8)> = Vec::with_capacity(12);
    bands.push((
        Rect::from_min_max(rect.min, Pos2::new(rect.max.x, rect.min.y + r)),
        r * 0.5 / h,
        1,
    ));
    let steps = 8;
    let mid_h = h - 2.0 * r;
    for i in 0..steps {
        let y0 = rect.min.y + r + mid_h * (i as f32 / steps as f32);
        let y1 = rect.min.y + r + mid_h * ((i + 1) as f32 / steps as f32);
        let t = ((y0 + y1) * 0.5 - rect.min.y) / h;
        bands.push((
            Rect::from_min_max(Pos2::new(rect.min.x, y0 - 0.5), Pos2::new(rect.max.x, y1 + 0.5)),
            t,
            0,
        ));
    }
    bands.push((
        Rect::from_min_max(Pos2::new(rect.min.x, rect.max.y - r), rect.max),
        (h - r * 0.5) / h,
        2,
    ));
    for (band, t, mode) in bands {
        let corners = match mode {
            1 => CornerRadius { nw: radius, ne: radius, sw: 0, se: 0 },
            2 => CornerRadius { nw: 0, ne: 0, sw: radius, se: radius },
            _ => CornerRadius::ZERO,
        };
        p.rect_filled(band, corners, lerp(top, bot, t));
    }
}

/// Colores de los bloques de la línea de tiempo (por tipo de clip).
pub fn clip_colors(kind: ClipKind) -> (Color32, Color32, Color32) {
    match kind {
        ClipKind::Video => (hex("#194647"), hex("#133033"), hex("#2c7474")),
        ClipKind::Image => (hex("#2b3038"), hex("#1b2026"), hex("#46535f")),
        ClipKind::Audio => (hex("#2a2a52"), hex("#191a2e"), hex("#4a4a7d")),
        ClipKind::Text => (hex("#3b2a46"), hex("#241a2d"), hex("#6f4d86")),
        ClipKind::Sticker => (hex("#3b2a46"), hex("#241a2d"), hex("#6f4d86")),
    }
}

/// Recorta `text` para que quepa en `max_w` añadiendo "…".
pub fn ellipsize(p: &Painter, text: &str, font: &FontId, max_w: f32) -> String {
    if max_w <= 0.0 {
        return String::new();
    }
    let full = p.layout_no_wrap(text.to_owned(), font.clone(), Color32::WHITE);
    if full.size().x <= max_w {
        return text.to_string();
    }
    let mut out = String::new();
    for ch in text.chars() {
        let candidate = format!("{out}{ch}…");
        let g = p.layout_no_wrap(candidate, font.clone(), Color32::WHITE);
        if g.size().x > max_w {
            break;
        }
        out.push(ch);
    }
    out
}

// ---------------------------------------------------------------------------
// Escena de miniatura (`--scene`)
// ---------------------------------------------------------------------------

fn quad(a: f32, c: f32, b: f32, t: f32) -> f32 {
    let u = 1.0 - t;
    u * u * a + 2.0 * u * t * c + t * t * b
}

fn hill1(x: f32) -> f32 {
    if x <= 80.0 {
        quad(62.0, 40.0, 56.0, (x / 80.0).clamp(0.0, 1.0))
    } else {
        quad(56.0, 72.0, 48.0, ((x - 80.0) / 80.0).clamp(0.0, 1.0))
    }
}

fn hill2(x: f32) -> f32 {
    if x <= 100.0 {
        quad(74.0, 60.0, 72.0, (x / 100.0).clamp(0.0, 1.0))
    } else {
        quad(72.0, 84.0, 66.0, ((x - 100.0) / 60.0).clamp(0.0, 1.0))
    }
}

/// Dibuja la escena de `styles.css` dentro de `rect` con encuadre *slice*
/// (centrado, cubre todo el rectángulo). `map` transforma cada color, lo que
/// permite previsualizar filtros y ajustes sin shaders.
pub fn scene(p: &Painter, rect: Rect, map: &dyn Fn(Color32) -> Color32) {
    if rect.width() <= 2.0 || rect.height() <= 2.0 {
        return;
    }
    let scale = (rect.width() / 160.0).max(rect.height() / 96.0);
    let ox = rect.min.x + (rect.width() - 160.0 * scale) * 0.5;
    let oy = rect.min.y + (rect.height() - 96.0 * scale) * 0.5;
    let m = |x: f32, y: f32| Pos2::new(ox + x * scale, oy + y * scale);
    let sz = |w: f32, h: f32| Vec2::new(w * scale, h * scale);

    let p = p.with_clip_rect(rect);

    // Cielo: dos rectángulos planos, igual que el SVG original.
    p.rect_filled(
        Rect::from_min_size(m(0.0, 0.0), sz(160.0, 58.0)),
        CornerRadius::ZERO,
        map(hex("#8fd0f2")),
    );
    p.rect_filled(
        Rect::from_min_size(m(0.0, 58.0), sz(160.0, 38.0)),
        CornerRadius::ZERO,
        map(hex("#4f9ad6")),
    );

    // Sol y halo (el halo va encima, como en el SVG).
    let sun = m(126.0, 22.0);
    p.circle_filled(sun, 13.0 * scale, map(hex("#fff2b0")));
    p.circle_filled(sun, 20.0 * scale, with_alpha(map(hex("#fff6cf")), 0.35));

    // Colinas: bandas verticales que siguen la curva.
    for (hill, color) in [(hill1 as fn(f32) -> f32, "#3f7f4b"), (hill2, "#2f6239")] {
        let fill = map(hex(color));
        let mut x = 0.0f32;
        while x < 160.0 {
            let x1 = (x + 2.0).min(160.0);
            let top = hill(x).min(hill(x1));
            p.rect_filled(
                Rect::from_min_size(m(x, top), sz(x1 - x + 0.05, 96.0 - top)),
                CornerRadius::ZERO,
                fill,
            );
            x = x1;
        }
    }

    // Árbol.
    p.rect_filled(
        Rect::from_min_size(m(30.0, 58.0), sz(3.0, 20.0)),
        CornerRadius::ZERO,
        map(hex("#7a5230")),
    );
    p.circle_filled(m(31.5, 55.0), 11.0 * scale, map(hex("#2c7a45")));
    p.circle_filled(m(24.0, 59.0), 7.0 * scale, map(hex("#36894f")));
    p.circle_filled(m(39.0, 59.0), 7.0 * scale, map(hex("#36894f")));
}

// ---------------------------------------------------------------------------
// Filtros CSS (aproximación por color sólido)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
pub struct Fx {
    pub brightness: f32,
    pub contrast: f32,
    pub saturate: f32,
    pub grayscale: f32,
    pub sepia: f32,
    pub hue: f32,
    pub opacity: f32,
    pub blur: f32,
}

impl Default for Fx {
    fn default() -> Self {
        Self {
            brightness: 1.0,
            contrast: 1.0,
            saturate: 1.0,
            grayscale: 0.0,
            sepia: 0.0,
            hue: 0.0,
            opacity: 1.0,
            blur: 0.0,
        }
    }
}

/// Interpreta una cadena CSS de filtro (`saturate(1.4) contrast(1.08)…`).
pub fn parse_css(css: &str) -> Fx {
    let mut fx = Fx::default();
    for token in css.split_whitespace() {
        let Some(open) = token.find('(') else { continue };
        let name = &token[..open];
        let inner = token[open + 1..].trim_end_matches(')');
        let raw = inner.trim_end_matches("deg").trim_end_matches("px").trim();
        let Ok(val) = raw.parse::<f32>() else { continue };
        match name {
            "brightness" => fx.brightness = val,
            "contrast" => fx.contrast = val,
            "saturate" => fx.saturate = val,
            "grayscale" => fx.grayscale = val.clamp(0.0, 1.0),
            "sepia" => fx.sepia = val.clamp(0.0, 1.0),
            "hue-rotate" => fx.hue = val,
            "opacity" => fx.opacity = val.clamp(0.0, 1.0),
            "blur" => fx.blur = val.max(0.0),
            _ => {}
        }
    }
    fx
}

/// Ajustes del panel Ajustar → mismo modelo de filtro (valores sobre 100).
pub fn fx_from_adjust(adj: &Adjust) -> Fx {
    let extra = |key: &str| adj.extra.get(key).and_then(Value::as_f64).unwrap_or(0.0) as f32;
    Fx {
        brightness: (adj.brightness / 100.0) as f32,
        contrast: (adj.contrast / 100.0) as f32,
        saturate: (adj.saturate / 100.0) as f32,
        grayscale: (extra("grayscale") / 100.0).clamp(0.0, 1.0),
        sepia: (extra("sepia") / 100.0).clamp(0.0, 1.0),
        hue: extra("hue"),
        opacity: 1.0,
        blur: 0.0,
    }
}

/// CSS del filtro activo del clip (color o GL; los LUT antiguos no tienen
/// equivalente propio y se ignoran en la previsualización).
pub fn clip_css(clip: &Clip) -> String {
    if let Some(shader) = &clip.gl_filter {
        if let Some(f) = FILTERS.iter().find(|f| f.gl == Some(shader.as_str())) {
            return f.css.to_string();
        }
    }
    if clip.filter != "none" {
        if let Some(f) = filter_by_key(&clip.filter) {
            return f.css.to_string();
        }
    }
    String::new()
}

pub fn apply_fx(c: Color32, fx: &Fx) -> Color32 {
    let [r, g, b, a] = c.to_array();
    let mut col = [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0];
    for v in &mut col {
        *v *= fx.brightness;
    }
    for v in &mut col {
        *v = (v - 0.5) * fx.contrast + 0.5;
    }
    if fx.grayscale > 0.0 {
        let l = 0.2126 * col[0] + 0.7152 * col[1] + 0.0722 * col[2];
        for v in &mut col {
            *v += (l - *v) * fx.grayscale;
        }
    }
    if fx.saturate != 1.0 {
        let l = 0.2126 * col[0] + 0.7152 * col[1] + 0.0722 * col[2];
        for v in &mut col {
            *v = l + (*v - l) * fx.saturate;
        }
    }
    if fx.sepia > 0.0 {
        let s = [
            0.393 * col[0] + 0.769 * col[1] + 0.189 * col[2],
            0.349 * col[0] + 0.686 * col[1] + 0.168 * col[2],
            0.272 * col[0] + 0.534 * col[1] + 0.131 * col[2],
        ];
        for i in 0..3 {
            col[i] += (s[i] - col[i]) * fx.sepia;
        }
    }
    if fx.hue != 0.0 {
        let base = Color32::from_rgb(
            (col[0].clamp(0.0, 1.0) * 255.0).round() as u8,
            (col[1].clamp(0.0, 1.0) * 255.0).round() as u8,
            (col[2].clamp(0.0, 1.0) * 255.0).round() as u8,
        );
        let mut hsva = eframe::egui::ecolor::Hsva::from(base);
        hsva.h = (hsva.h + fx.hue / 360.0).rem_euclid(1.0);
        let out: Color32 = hsva.into();
        let [r2, g2, b2, _] = out.to_array();
        col = [r2 as f32 / 255.0, g2 as f32 / 255.0, b2 as f32 / 255.0];
    }
    let q = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    let alpha = (a as f32 * fx.opacity).round().clamp(0.0, 255.0) as u8;
    Color32::from_rgba_unmultiplied(q(col[0]), q(col[1]), q(col[2]), alpha)
}

/// Ajuste + filtro del clip, aplicados en cadena (como en el renderizador).
pub fn clip_map(clip: &Clip) -> impl Fn(Color32) -> Color32 + '_ {
    let adjust = fx_from_adjust(&clip.adjust);
    let css = parse_css(&clip_css(clip));
    move |c| apply_fx(apply_fx(c, &adjust), &css)
}

// ---------------------------------------------------------------------------
// Texto de la vista previa
// ---------------------------------------------------------------------------

pub fn num_of(v: &Value, key: &str, def: f64) -> f64 {
    v.get(key).and_then(Value::as_f64).unwrap_or(def)
}

pub fn str_of<'a>(v: &'a Value, key: &str, def: &'a str) -> &'a str {
    v.get(key).and_then(Value::as_str).unwrap_or(def)
}

pub fn bool_of(v: &Value, key: &str, def: bool) -> bool {
    v.get(key).and_then(Value::as_bool).unwrap_or(def)
}

const DIRS: [(f32, f32); 12] = [
    (1.0, 0.0),
    (-1.0, 0.0),
    (0.0, 1.0),
    (0.0, -1.0),
    (0.707, 0.707),
    (-0.707, 0.707),
    (0.707, -0.707),
    (-0.707, -0.707),
    (0.707, 0.0),
    (-0.707, 0.0),
    (0.0, 0.707),
    (0.0, -0.707),
];

/// Dibuja un clip de texto dentro de `rect`.
///
/// `scale` convierte el tamaño de diseño (basado en 1080 px de alto) a
/// píxeles reales: la vista previa usa `alto / 1080` y las tarjetas usan un
/// valor mayor para que la muestra se lea.
pub fn paint_text(p: &Painter, rect: Rect, text: &Value, scale: f32) {
    let content = str_of(text, "content", "");
    if content.is_empty() || rect.width() < 4.0 || rect.height() < 4.0 {
        return;
    }
    let mut body = content.to_string();
    match str_of(text, "textCase", "none") {
        "upper" => body = body.to_uppercase(),
        "lower" => body = body.to_lowercase(),
        "capitalize" => body = capitalize(&body),
        _ => {}
    }

    let font_px = (num_of(text, "size", 72.0) as f32 * scale).max(6.0);
    let font = theme::font_for(num_of(text, "weight", 800.0), font_px);
    let color = hex(str_of(text, "color", "#ffffff"));
    let opacity = num_of(text, "opacity", 1.0) as f32;
    let fill = if opacity < 1.0 { with_alpha(color, opacity) } else { color };

    let tx = num_of(text, "x", 0.0) as f32;
    let ty = num_of(text, "y", 0.0) as f32;
    let center = Pos2::new(
        rect.center().x + (tx / 100.0) * rect.width(),
        rect.center().y + (ty / 100.0) * rect.height(),
    );

    let first = p.layout_no_wrap(body.clone(), font.clone(), fill);
    let gsize = first.size();
    let pos = match str_of(text, "align", "center") {
        "left" => Pos2::new(center.x, center.y - gsize.y * 0.5),
        "right" => Pos2::new(center.x - gsize.x, center.y - gsize.y * 0.5),
        _ => Pos2::new(center.x - gsize.x * 0.5, center.y - gsize.y * 0.5),
    };
    let block = Rect::from_min_size(pos, gsize);
    let draw = |at: Pos2, c: Color32| {
        let g = p.layout_no_wrap(body.clone(), font.clone(), c);
        p.galley(at, g, c);
    };

    // Fondo (burbuja de texto).
    let bg = str_of(text, "background", "transparent");
    if bg != "transparent" && !bg.is_empty() {
        let pad = num_of(text, "backgroundPadding", 0.22) as f32 * font_px;
        let radius = num_of(text, "backgroundRadius", 0.16) as f32 * font_px;
        let alpha = num_of(text, "backgroundAlpha", 1.0) as f32;
        let bg_rect =
            Rect::from_min_max(block.min - Vec2::splat(pad), block.max + Vec2::splat(pad));
        p.rect_filled(
            bg_rect,
            CornerRadius::from(radius.round().clamp(0.0, 255.0) as u8),
            with_alpha(hex(bg), alpha),
        );
        let border = num_of(text, "backgroundBorderWidth", 0.0) as f32 * font_px;
        if border > 0.0 {
            p.rect_stroke(
                bg_rect,
                CornerRadius::from(radius.round().clamp(0.0, 255.0) as u8),
                Stroke::new(border, hex(str_of(text, "backgroundBorderColor", "#000000"))),
                StrokeKind::Inside,
            );
        }
    }

    if bool_of(text, "glow", false) {
        let glow = hex(str_of(text, "glowColor", "#00e5d0"));
        let blur = (num_of(text, "glowBlur", 0.35) as f32 * font_px).max(1.0);
        for (dx, dy) in DIRS.iter().take(8) {
            draw(
                pos + Vec2::new(dx * blur * 0.6, dy * blur * 0.6),
                with_alpha(glow, 0.4),
            );
        }
    }

    if bool_of(text, "shadow", false) {
        let sc = hex(str_of(text, "shadowColor", "#000000"));
        let sa = num_of(text, "shadowAlpha", 0.58) as f32;
        let dist = num_of(text, "shadowDistance", 0.05) as f32 * font_px;
        let angle = (num_of(text, "shadowAngle", 90.0) as f32).to_radians();
        draw(
            pos + Vec2::new(angle.cos() * dist, angle.sin() * dist),
            with_alpha(sc, sa),
        );
    }

    let stroke_w = num_of(text, "strokeWidth", 0.0) as f32 * font_px * 0.6;
    if stroke_w > 0.0 {
        let sc = with_alpha(
            hex(str_of(text, "strokeColor", "#000000")),
            num_of(text, "strokeAlpha", 1.0) as f32,
        );
        let r = stroke_w.max(1.0);
        for (dx, dy) in DIRS.iter() {
            draw(pos + Vec2::new(dx * r, dy * r), sc);
        }
    }

    draw(pos, fill);
}

fn capitalize(s: &str) -> String {
    s.split(' ')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => {
                    let rest: String = chars.collect();
                    format!("{}{}", first.to_uppercase(), rest)
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
