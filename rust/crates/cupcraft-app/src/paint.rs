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
        ClipKind::Effect => (hex("#7b46d6"), hex("#5a2fa8"), hex("#a678ff")),
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
    pub invert: f32,
    pub opacity: f32,
    pub blur: f32,
}

impl Fx {
    pub fn is_identity(&self) -> bool {
        (self.brightness - 1.0).abs() < 1e-3
            && (self.contrast - 1.0).abs() < 1e-3
            && (self.saturate - 1.0).abs() < 1e-3
            && self.grayscale <= 0.0
            && self.sepia <= 0.0
            && self.hue.abs() < 1e-3
            && self.invert <= 0.0
            && (self.opacity - 1.0).abs() < 1e-3
    }
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
            invert: 0.0,
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
            "invert" => fx.invert = val.clamp(0.0, 1.0),
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
        invert: 0.0,
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

/// CSS del efecto activo (`EFFECTS[id].css` de `src/filtros/library.js`),
/// portado 1:1. Solo entradas con color o desenfoque; las de solo
/// partículas/marcos/movimiento (`css: 'none'`) y las de solo `drop-shadow`
/// —que este motor no reproduce— devuelven cadena vacía. La parte de color la
/// consume `clip_chain`; la de `blur`, `filter_blur_px` del compositor.
pub fn effect_css(id: &str) -> &'static str {
    match id {
        // Tendencias
        "tnd_chalk" => "contrast(1.12) saturate(0.82)",
        "tnd_heart" => "brightness(1.06) saturate(1.14)",
        "tnd_sepia" => "sepia(0.42) saturate(1.5) hue-rotate(-16deg) contrast(1.06)",
        "tnd_action" => "contrast(1.25) saturate(1.24)",
        "tnd_beat" => "contrast(1.08)",
        "tnd_horror" => "contrast(1.3) saturate(0.68) brightness(0.92)",
        "tnd_mosaic" => "contrast(1.08) saturate(1.15)",
        "tnd_doodle" => "saturate(1.18)",
        "tnd_laugh" => "brightness(1.08) saturate(1.1)",
        "tnd_chroma" => "saturate(1.5) contrast(1.06)",
        "tnd_print" => "contrast(1.2) saturate(0.9)",
        "tnd_retroShake" => "sepia(0.3) contrast(1.12)",
        // Pato
        "pat_feather" => "brightness(1.04)",
        "pat_beak" => "sepia(0.24) saturate(1.3) brightness(1.06)",
        "pat_happy" => "saturate(1.2) brightness(1.05)",
        "pat_puddle" => "hue-rotate(175deg) saturate(1.25)",
        "pat_egg" => "brightness(1.07)",
        "pat_rain" => "saturate(1.12)",
        "pat_flap" => "brightness(1.03)",
        "pat_bubbles" => "brightness(1.08) saturate(1.08)",
        // Halloween
        "hal_pumpkin" => "saturate(1.24) contrast(1.05)",
        "hal_ghost" => "brightness(1.05) contrast(0.98)",
        "hal_web" => "contrast(1.1)",
        "hal_bats" => "brightness(0.96) contrast(1.08)",
        "hal_fog" => "contrast(1.04) saturate(0.85)",
        "hal_lightning" => "brightness(1.1) contrast(1.16)",
        "hal_eyes" => "brightness(0.8) contrast(1.2)",
        "hal_blood" => "saturate(1.42) contrast(1.08)",
        "hal_skull" => "grayscale(0.35) contrast(1.16)",
        "hal_witch" => "hue-rotate(-12deg) saturate(1.24)",
        // Especiales
        "esp_dreamy" => "blur(1.1px) brightness(1.12) saturate(1.18)",
        "esp_confetti" => "saturate(1.15)",
        "esp_stars" => "brightness(1.06)",
        "esp_glitter" => "brightness(1.08) saturate(1.1)",
        "esp_rainbow" => "saturate(1.35)",
        "esp_fireworks" => "brightness(1.05)",
        "esp_hearts" => "saturate(1.18)",
        "esp_notes" => "brightness(1.04)",
        "esp_petals" => "saturate(1.2)",
        "esp_candles" => "brightness(1.04)",
        "esp_magic" => "saturate(1.2) brightness(1.05)",
        // Black Friday
        "blk_sale" => "brightness(1.1) contrast(1.06)",
        "blk_tags" => "saturate(1.14)",
        "blk_money" => "saturate(1.16)",
        "blk_cart" => "brightness(1.05)",
        "blk_neon" => "saturate(1.85) contrast(1.12)",
        "blk_flash" => "brightness(1.2) contrast(1.15)",
        "blk_arrows" => "contrast(1.1)",
        "blk_coupon" => "saturate(1.1)",
        // Clásica
        "blur" => "blur(2.4px)",
        "cls_vintage" => "sepia(0.35) contrast(1.05) saturate(0.9)",
        "cls_polaroid" => "brightness(1.04) contrast(0.98)",
        "cls_film" => "contrast(1.04) saturate(0.94)",
        "cls_bw" => "grayscale(1) contrast(1.2)",
        "cls_sepia" => "sepia(0.8) contrast(1.05)",
        "cls_flash" => "brightness(1.45) contrast(1.2)",
        // Recorte mágico
        "cut_neon" => "contrast(1.1) saturate(1.16)",
        "cut_silhouette" => "brightness(0.72) contrast(1.28)",
        "cut_outline" => "contrast(1.1)",
        "cut_chalk" => "contrast(1.16) saturate(0.86)",
        "cut_double" => "contrast(1.08)",
        "cut_shadow" => "brightness(0.98) contrast(1.08)",
        "cut_comic" => "contrast(1.3) saturate(1.5)",
        "cut_watercolor" => "saturate(1.18) blur(0.4px) brightness(1.04)",
        // NUEVO
        "new_3d" => "saturate(1.1)",
        "new_aura" => "brightness(1.06) saturate(1.12)",
        "new_glass" => "brightness(1.08) saturate(1.08)",
        "new_holo" => "hue-rotate(20deg) saturate(1.4)",
        "new_prism" => "saturate(1.24)",
        "new_pulse" => "contrast(1.08)",
        "new_particles" => "brightness(1.06)",
        "new_badge" => "brightness(1.05)",
        // Bombazos
        "bmb_explosion" => "contrast(1.18) saturate(1.3)",
        "bmb_confetti" => "saturate(1.3)",
        "bmb_flash" => "brightness(1.5) contrast(1.2)",
        "bmb_boom" => "contrast(1.35) saturate(1.4)",
        "bmb_fire" => "saturate(1.35) contrast(1.08)",
        "bmb_sparks" => "brightness(1.12)",
        "bmb_impact" => "contrast(1.25)",
        "bmb_quake" => "contrast(1.12) saturate(1.18)",
        // Intro y cierre
        "int_countdown" => "contrast(1.1)",
        "int_curtain" => "brightness(0.98)",
        "int_wipe" => "contrast(1.06)",
        "int_flash" => "brightness(1.4)",
        "int_zoom" => "contrast(1.06)",
        "int_glitch" => "contrast(1.14)",
        "int_close" => "brightness(0.92)",
        "int_open" => "brightness(1.08)",
        // Marcos aleatorios
        "frm_polaroid" => "contrast(0.99) brightness(1.04)",
        "frm_film" => "saturate(0.94)",
        "frm_tape" => "brightness(1.03)",
        "frm_neon" => "saturate(1.2)",
        "frm_floral" => "saturate(1.12)",
        "frm_torn" => "contrast(1.02)",
        "frm_wood" => "sepia(0.2) contrast(1.04)",
        "frm_dots" => "brightness(1.04)",
        "frm_rainbow" => "saturate(1.25)",
        // Fiesta
        "fst_confetti" => "saturate(1.24)",
        "fst_disco" => "saturate(1.3) brightness(1.06)",
        "fst_lights" => "saturate(1.25)",
        "fst_balloons" => "saturate(1.22)",
        "fst_streamers" => "saturate(1.2)",
        "fst_dj" => "saturate(1.28) contrast(1.06)",
        "fst_fireworks" => "brightness(1.06)",
        "fst_glitter" => "brightness(1.08)",
        // Movimiento
        "mov_blur" => "blur(1.1px)",
        // Ediciones
        "edt_vhs" => "contrast(1.1) saturate(0.9)",
        "edt_glitch" => "contrast(1.16) saturate(1.08)",
        "scanlines" => "contrast(1.12) saturate(0.85)",
        "edt_pixel" => "contrast(1.3) saturate(1.1)",
        "edt_vcr" => "contrast(1.08)",
        "edt_tracking" => "contrast(1.12)",
        "edt_datamosh" => "contrast(1.14) saturate(1.16)",
        // 3D
        "d3_cube" => "saturate(1.08)",
        "d3_flip" => "saturate(1.08)",
        "d3_holo" => "hue-rotate(24deg) saturate(1.4)",
        "d3_depth" => "brightness(1.04) contrast(1.1)",
        "d3_extrude" => "contrast(1.08)",
        // Luz
        "glow" => "brightness(1.16) saturate(1.2)",
        "neon" => "saturate(1.9) contrast(1.08) hue-rotate(-12deg)",
        "luz_leak" => "brightness(1.06) saturate(1.1)",
        "luz_flare" => "brightness(1.08)",
        "luz_spot" => "contrast(1.08)",
        "strobe" => "brightness(1.45) contrast(1.2)",
        "luz_bokeh" => "brightness(1.05) saturate(1.08)",
        "luz_rays" => "brightness(1.08) saturate(1.1)",
        "luz_aurora" => "saturate(1.3) brightness(1.04)",
        "luz_flash" => "brightness(1.5) contrast(1.16)",
        // Retro
        "oldFilm" => "sepia(0.45) contrast(1.02) brightness(0.94)",
        "vignette" => "contrast(1.06) brightness(0.98)",
        "grain" => "contrast(1.08) saturate(0.9)",
        "ret_scan" => "contrast(1.1) saturate(0.86)",
        "ret_sepia" => "sepia(0.85) contrast(1.05)",
        "ret_vhs" => "contrast(1.08) saturate(0.88)",
        "ret_polaroid" => "brightness(1.04) contrast(0.98)",
        "ret_projector" => "sepia(0.3) contrast(1.08)",
        "ret_dust" => "contrast(1.06) saturate(0.9)",
        "chromatic" => "saturate(1.4)",
        // Efectos corporales
        "body_aura" => "brightness(1.06) saturate(1.12)",
        "body_slim" => "contrast(1.1)",
        "body_glow" => "brightness(1.1) saturate(1.1)",
        "body_comic" => "contrast(1.3) saturate(1.4)",
        "body_outline" => "contrast(1.1)",
        "body_thermal" => "invert(1) hue-rotate(180deg) saturate(2.4) contrast(1.4)",
        "body_xray" => "invert(1) grayscale(1) contrast(1.6)",
        "body_sticker" => "saturate(1.2)",
        "body_neon" => "saturate(1.3)",
        _ => "",
    }
}

pub fn apply_fx(c: Color32, fx: &Fx) -> Color32 {
    let [r, g, b, a] = c.to_array();
    let mut col = [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0];
    if fx.invert > 0.0 {
        for v in &mut col {
            *v += fx.invert * (1.0 - 2.0 * *v);
        }
    }
    for v in &mut col {
        *v *= fx.brightness;
    }
    for v in &mut col {
        *v = (*v - 0.5) * fx.contrast + 0.5;
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

/// Etapas de color del clip; vacía = identidad. Cada función CSS es una etapa
/// propia (como `clipFilter` del renderizador original: filtro → ajuste → HSL →
/// retoque → curvas → extras), así dos `hue-rotate` se suman igual que en el
/// navegador. La previsualización y el export las aplican con [`apply_fx`].
pub fn clip_chain(clip: &Clip) -> Vec<Fx> {
    clip_chain_from(clip, clip_css(clip))
}

/// Igual que [`clip_chain`] pero partiendo de una cadena CSS base explícita.
/// El compositor la usa para omitir el `css` de reserva de un filtro GL de
/// textura pura (grano, viñeta…) que ya se pinta como textura real a pantalla
/// completa, pasando `String::new()` en su lugar.
pub fn clip_chain_from(clip: &Clip, mut css: String) -> Vec<Fx> {
    // Mismo orden que `clipFilter` (renderer.js): filtro → ajuste → HSL →
    // retoque → toggles de IA → curvas → CSS del efecto.
    for part in [
        adjust_css(&clip.adjust),
        hsl_css(clip.extra.get("hsl")),
        beauty_css(clip.extra.get("beauty")),
    ] {
        if !part.is_empty() && part != "none" {
            css.push(' ');
            css.push_str(&part);
        }
    }
    let flag = |k: &str| clip.extra.get(k).and_then(Value::as_bool).unwrap_or(false);
    // Toggles de IA (solo los términos de color; `denoise`/`opticalFlow` son
    // desenfoque puro y los resuelve `filter_blur_px` del compositor).
    if flag("enhanceQuality") {
        css.push_str(" contrast(1.08) saturate(1.05)");
    }
    if flag("stabilize") {
        css.push_str(" contrast(1.03) saturate(1.02)");
    }
    if flag("aiExpand") {
        css.push_str(" brightness(1.05) saturate(1.08)");
    }
    if flag("aiMotion") {
        css.push_str(" brightness(1.03)");
    }
    if flag("cameraTracking") {
        css.push_str(" contrast(1.04)");
    }
    if flag("eyeContact") {
        css.push_str(" brightness(1.04) saturate(1.03)");
    }
    if flag("lipSync") {
        css.push_str(" contrast(1.02) saturate(1.02)");
    }
    if flag("aiRemix") {
        css.push_str(" hue-rotate(3deg) saturate(1.05)");
    }
    let curves = curve_css(clip);
    if !curves.is_empty() && curves != "none" {
        css.push(' ');
        css.push_str(&curves);
    }
    // CSS de color del efecto activo (`spec.css` de library.js). El término
    // `blur` del efecto lo aplica `filter_blur_px`; aquí solo cuenta el color
    // (los Fx de solo desenfoque se descartan como identidad).
    if let Some(id) = clip.extra.get("effect").and_then(Value::as_str) {
        // Efectos parametrizables de la biblioteca (vista previa == export);
        // si el id no pertenece a ella, se usa el catálogo fijo `effect_css`.
        let ec = fx_effect_css(id, clip.extra.get("effectParams"));
        if !ec.is_empty() {
            css.push(' ');
            css.push_str(&ec);
        }
    }
    css.split_whitespace()
        .map(parse_css)
        .filter(|fx| !fx.is_identity())
        .collect()
}

/// CSS de filtros del efecto activo: prefiere la biblioteca parametrizable
/// (`fx_library`) y, si el id no pertenece a ella, usa el catálogo fijo
/// `effect_css`. Fuente única para vista previa (`clip_chain_from`) y
/// exportación (`compose::filter_blur_px`).
pub fn fx_effect_css(id: &str, params: Option<&Value>) -> String {
    crate::fx_library::css_for(id, params).unwrap_or_else(|| effect_css(id).to_string())
}

fn clamp1(v: f64, lo: f64, hi: f64) -> f64 {
    (v.clamp(lo, hi) * 10.0).round() / 10.0
}

/// `adjustCss` de `renderer.js` (valores en %, como CSS → se pasan a factor).
pub fn adjust_css(a: &Adjust) -> String {
    let num = |k: &str, d: f64| a.extra.get(k).and_then(Value::as_f64).unwrap_or(d);
    let brightness = a.brightness + num("exposure", 0.0) * 0.9 + (num("whites", 0.0) - num("blacks", 0.0)) * 0.18 + num("light", 0.0);
    let contrast = a.contrast + num("clarity", 0.0) * 0.35 + num("sharpen", 0.0) * 0.12 - num("fade", 0.0) * 0.45;
    let saturate = a.saturate;
    let temperature = num("temperature", 0.0);
    let tint = num("tint", 0.0);
    let mut p: Vec<String> = Vec::new();
    if (brightness - 100.0).abs() > 0.5 {
        p.push(format!("brightness({})", clamp1(brightness, 0.0, 300.0) / 100.0));
    }
    if (contrast - 100.0).abs() > 0.5 {
        p.push(format!("contrast({})", clamp1(contrast, 0.0, 300.0) / 100.0));
    }
    if (saturate - 100.0).abs() > 0.5 {
        p.push(format!("saturate({})", clamp1(saturate, 0.0, 400.0) / 100.0));
    }
    if temperature > 0.0 {
        p.push(format!("sepia({})", clamp1(temperature * 0.45, 0.0, 60.0) / 100.0));
    } else if temperature < 0.0 {
        p.push(format!("sepia({})", clamp1(-temperature * 0.45, 0.0, 60.0) / 100.0));
        p.push(format!("hue-rotate({}deg)", clamp1(150.0 - temperature * 0.35, 0.0, 220.0)));
    }
    if tint != 0.0 {
        p.push(format!("hue-rotate({}deg)", clamp1(tint * 0.15, -45.0, 45.0)));
    }
    if num("grayscale", 0.0) > 0.0 {
        p.push(format!("grayscale({})", num("grayscale", 0.0).min(100.0) / 100.0));
    }
    if num("sepia", 0.0) > 0.0 {
        p.push(format!("sepia({})", num("sepia", 0.0).min(100.0) / 100.0));
    }
    if num("hue", 0.0) != 0.0 {
        p.push(format!("hue-rotate({}deg)", num("hue", 0.0)));
    }
    p.join(" ")
}

/// `hslCss`: aproximación global de la familia de color elegida.
pub fn hsl_css(hsl: Option<&Value>) -> String {
    let Some(h) = hsl else { return String::new() };
    let num = |k: &str| h.get(k).and_then(Value::as_f64).unwrap_or(0.0);
    let mut p = Vec::new();
    if num("hue") != 0.0 {
        p.push(format!("hue-rotate({}deg)", clamp1(num("hue") * 0.5, -180.0, 180.0)));
    }
    if num("sat") != 0.0 {
        p.push(format!("saturate({})", clamp1(100.0 + num("sat") * 0.6, 0.0, 300.0) / 100.0));
    }
    if num("light") != 0.0 {
        p.push(format!("brightness({})", clamp1(100.0 + num("light") * 0.5, 0.0, 300.0) / 100.0));
    }
    p.join(" ")
}

/// `beautyCss` (el desenfoque de piel no se aplica píxel a píxel).
pub fn beauty_css(beauty: Option<&Value>) -> String {
    let Some(b) = beauty else { return String::new() };
    let num = |k: &str| b.get(k).and_then(Value::as_f64).unwrap_or(0.0);
    let mut p = Vec::new();
    if num("whiten") > 0.0 {
        p.push(format!("brightness({})", clamp1(100.0 + num("whiten") * 0.22, 100.0, 135.0) / 100.0));
    }
    if num("rosy") > 0.0 {
        p.push(format!("saturate({})", clamp1(100.0 + num("rosy") * 0.25, 100.0, 140.0) / 100.0));
    }
    p.join(" ")
}

/// `curveCss`: brillo/contraste aproximados a partir de los puntos del canal.
pub fn curve_css(clip: &Clip) -> String {
    let Some(curves) = clip.extra.get("curves") else { return String::new() };
    let channel = clip.extra.get("curveChannel").and_then(Value::as_str).unwrap_or("luma");
    let pts = curves.get(channel).or_else(|| curves.get("luma")).and_then(Value::as_array);
    let mut pts: Vec<(f64, f64)> = pts
        .map(|a| {
            a.iter()
                .filter_map(|p| Some((p.get(0)?.as_f64()?, p.get(1)?.as_f64()?)))
                .collect()
        })
        .unwrap_or_default();
    if pts.len() < 2 {
        return String::new();
    }
    pts.sort_by(|a, b| a.0.total_cmp(&b.0));
    let (first, last) = (pts[0], pts[pts.len() - 1]);
    let slope = (last.1 - first.1) / (last.0 - first.0).max(0.01);
    let mid = pts[pts.len() / 2];
    let b = (0.6 + (mid.1 - 0.5) * 0.8).clamp(0.6, 1.6);
    let c = (0.6 + slope * 0.4).clamp(0.6, 1.6);
    if (b - 1.0).abs() < 0.02 && (c - 1.0).abs() < 0.02 {
        return String::new();
    }
    format!("brightness({b:.2}) contrast({c:.2})")
}

/// Fuerza de la viñeta del panel Ajustar (`drawVignette`): 0 = sin viñeta.
pub fn vignette_strength(clip: &Clip) -> f32 {
    let v = clip.adjust.extra.get("vignette").and_then(Value::as_f64).unwrap_or(0.0);
    if v <= 0.0 { 0.0 } else { (v / 100.0).min(0.85) as f32 }
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

// ---------------------------------------------------------------------------
// Deformación geométrica real (Fase B de efectos)
//
// Algunos efectos de la biblioteca (`fx_library`, `approx == false`) no se
// pueden aproximar con color/desenfoque porque mueven píxeles: remolino,
// pellizco, ondas, coordenadas polares, pixelado, espejo y caleidoscopio.
// Se resuelven con un remapeo inverso de coordenadas + muestreo bilineal,
// paralelizado con rayon. La misma función la usan la vista previa
// (`services::video::texture_fx`) y la exportación (`services::compose`),
// de modo que lo que se ve es lo que se renderiza.
// ---------------------------------------------------------------------------

use rayon::prelude::*;

const TAU_F: f32 = std::f32::consts::TAU;

/// Tipo de deformación geométrica. Cada variante corresponde a uno o varios
/// ids de `fx_library` (ver [`warp_for`]).
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum WarpKind {
    Swirl,
    Pinch,
    Ripple,
    Polar,
    Pixelate,
    Mirror,
    Kaleidoscope,
    /// Operación del motor `fx_engine` (índice en `fx_table::TABLE`).
    Engine,
}

/// Deformación geométrica activa de un clip, con sus parámetros resueltos
/// (leídos de `effectParams`). `Copy` para plegarla en la clave de caché de la
/// textura de vista previa sin asignar memoria.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Warp {
    pub kind: WarpKind,
    /// 0..1 (slider `amount` 0..100 %).
    pub amount: f32,
    /// Repeticiones/segmentos (slider `detail`); 1 si el efecto no lo usa.
    pub detail: f32,
    /// Índice en `fx_table::TABLE` cuando `kind == Engine`.
    pub engine: u16,
    /// Tiempo local del clip (s) para efectos animados.
    pub time: f32,
    /// Duración del clip (s).
    pub dur: f32,
    /// Pulsaciones por minuto de los efectos de ritmo (0 = no es de ritmo).
    pub bpm: f32,
}

impl Warp {
    /// Huella estable para la clave de caché de `texture_fx` (la deformación
    /// debe invalidar la textura igual que la cadena de color).
    pub fn hash(&self) -> u64 {
        let k = self.kind as u64;
        let a = (self.amount * 1000.0).round() as i64 as u64;
        let d = (self.detail * 1000.0).round() as i64 as u64;
        let k = k ^ ((self.engine as u64) << 16);
        // El tiempo solo invalida la caché en los efectos animados.
        let anim = self.kind == WarpKind::Engine
            && crate::fx_engine::op_at(self.engine).is_some_and(crate::fx_engine::animated);
        let tq = if anim || self.bpm > 0.0 { ((self.time * 1000.0).round() as i64 as u64).wrapping_mul(0x27D4EB2F165667C5) } else { 0 };
        let k = k ^ tq;
        k.wrapping_mul(0x9E3779B97F4A7C15)
            ^ a.wrapping_mul(0xC2B2AE3D27D4EB4F)
            ^ d.wrapping_mul(0x165667B19E3779F9)
    }
}

/// Mapea un id de efecto a su deformación, leyendo `amount`/`detail` de los
/// parámetros guardados (`effectParams`). Devuelve `None` si el id no es un
/// efecto geométrico real (entonces no hay nada que remapear).
pub fn warp_for(id: &str, params: Option<&Value>) -> Option<Warp> {
    let get = |key: &str, def: f32| -> f32 {
        params
            .and_then(|p| p.get(key))
            .and_then(Value::as_f64)
            .map(|v| v as f32)
            .unwrap_or(def)
    };
    // amount del slider llega en 0..100 (%) → 0..1.
    let amount = (get("amount", 50.0) / 100.0).clamp(0.0, 1.0);
    if let Some(i) = crate::fx_engine::index_of(id) {
        if amount <= 0.0005 {
            return None;
        }
        // El segundo parámetro puede ser un ángulo (0 por defecto) o un detalle.
        let def = match crate::fx_table::second_of(id) {
            Some(crate::fx_table::Second::Angle) => 0.0,
            _ => 8.0,
        };
        // Ritmo: el segundo parámetro es el tempo (BPM) y el motor recibe el
        // detalle por defecto; la cantidad late con la envolvente del pulso.
        if id.starts_with("bet_") {
            return Some(Warp { kind: WarpKind::Engine, amount, detail: 8.0, engine: i, time: 0.0, dur: 0.0, bpm: get("bpm", 120.0).max(20.0) });
        }
        return Some(Warp { kind: WarpKind::Engine, amount, detail: get("detail", def), engine: i, time: 0.0, dur: 0.0, bpm: 0.0 });
    }
    let detail = get("detail", 8.0).max(1.0);
    let kind = match id {
        "dst_swirl" => WarpKind::Swirl,
        "dst_pinch" => WarpKind::Pinch,
        "dst_ripple" | "dst_circripple" | "dst_wavewarp" => WarpKind::Ripple,
        "dst_polar" => WarpKind::Polar,
        "sty_pixelate" | "sty_mosaic" => WarpKind::Pixelate,
        "rep_mirror" => WarpKind::Mirror,
        "rep_kaleidoscope" => WarpKind::Kaleidoscope,
        _ => return None,
    };
    // amount == 0 → identidad; no vale la pena deformar.
    if amount <= 0.0005 {
        return None;
    }
    Some(Warp { kind, amount, detail, engine: u16::MAX, time: 0.0, dur: 0.0, bpm: 0.0 })
}

/// Deformación activa de un clip, si su efecto es geométrico real.
pub fn clip_warp(clip: &Clip) -> Option<Warp> {
    clip_warp_at(clip, 0.0)
}

/// Deformación del clip en el instante local `local` (s desde su inicio).
pub fn clip_warp_at(clip: &Clip, local: f64) -> Option<Warp> {
    let id = clip.extra.get("effect").and_then(Value::as_str)?;
    let mut w = warp_for(id, clip.extra.get("effectParams"))?;
    w.time = effect_local(clip, local) as f32;
    w.dur = clip.duration as f32;
    Some(w)
}

/// Tiempo local efectivo: con "Cuantización de tiempo" (`mov_framerate`) se
/// redondea hacia abajo a la cadencia elegida, así el vídeo y los efectos
/// avanzan a saltos como a menos fotogramas por segundo.
pub fn effect_local(clip: &Clip, local: f64) -> f64 {
    if clip.extra.get("effect").and_then(Value::as_str) == Some("mov_framerate") {
        let fps = clip
            .extra
            .get("effectParams")
            .and_then(|p| p.get("fps"))
            .and_then(Value::as_f64)
            .unwrap_or(8.0)
            .clamp(1.0, 60.0);
        return (local * fps).floor() / fps;
    }
    local
}

/// Muestreo bilineal de un buffer RGBA en una coordenada flotante.
#[inline]
fn bilinear(src: &[u8], w: usize, h: usize, fx: f32, fy: f32) -> [u8; 4] {
    if w == 0 || h == 0 {
        return [0, 0, 0, 0];
    }
    let fx = fx.clamp(0.0, (w - 1) as f32);
    let fy = fy.clamp(0.0, (h - 1) as f32);
    let x0 = fx.floor() as usize;
    let y0 = fy.floor() as usize;
    let x1 = (x0 + 1).min(w - 1);
    let y1 = (y0 + 1).min(h - 1);
    let dx = fx - x0 as f32;
    let dy = fy - y0 as f32;
    let at = |x: usize, y: usize| -> &[u8] {
        let i = (y * w + x) * 4;
        &src[i..i + 4]
    };
    let p00 = at(x0, y0);
    let p10 = at(x1, y0);
    let p01 = at(x0, y1);
    let p11 = at(x1, y1);
    let mut out = [0u8; 4];
    for c in 0..4 {
        let top = p00[c] as f32 * (1.0 - dx) + p10[c] as f32 * dx;
        let bot = p01[c] as f32 * (1.0 - dx) + p11[c] as f32 * dx;
        out[c] = (top * (1.0 - dy) + bot * dy).round().clamp(0.0, 255.0) as u8;
    }
    out
}

/// Aplica una deformación geométrica a un fotograma RGBA (`src`, `w`×`h`) con
/// remapeo inverso: por cada píxel de destino se calcula de qué coordenada de
/// origen proviene y se muestrea por bilineal. Paralelo por filas (rayon).
pub fn warp_rgba(src: &[u8], w: usize, h: usize, warp: &Warp) -> Vec<u8> {
    if w == 0 || h == 0 || src.len() < w * h * 4 {
        return src.to_vec();
    }
    if warp.kind == WarpKind::Engine {
        return match crate::fx_engine::op_at(warp.engine) {
            Some(op) => {
                // Ritmo: la cantidad sigue una envolvente que salta en cada
                // pulso y cae exponencialmente; el tiempo se congela por pulso
                // (cada golpe da una pose nueva).
                let (a, t) = if warp.bpm > 0.0 {
                    let beats = warp.time * warp.bpm / 60.0;
                    let env = (-beats.fract() * 5.0).exp();
                    (warp.amount * env, beats.floor() * 0.37 + 0.25)
                } else {
                    (warp.amount, warp.time)
                };
                crate::fx_engine::run_t(op, src, w, h, a, warp.detail, t, warp.dur)
            }
            None => src.to_vec(),
        };
    }
    let mut dst = vec![0u8; w * h * 4];
    let wf = w as f32;
    let hf = h as f32;
    let cx = wf / 2.0;
    let cy = hf / 2.0;
    let rad_max = cx.hypot(cy).max(1.0);
    let amt = warp.amount;
    let detail = warp.detail;
    let kind = warp.kind;

    dst.par_chunks_mut(w * 4)
        .enumerate()
        .for_each(|(y, row)| {
            let yf = y as f32;
            for x in 0..w {
                let xf = x as f32;
                // Coordenada de origen (sx, sy) para este píxel de destino.
                let (sx, sy) = match kind {
                    WarpKind::Swirl => {
                        let dx = xf - cx;
                        let dy = yf - cy;
                        let r = dx.hypot(dy);
                        let falloff = (1.0 - r / rad_max).clamp(0.0, 1.0);
                        let ang = amt * 3.0 * falloff;
                        let (s, c) = ang.sin_cos();
                        (cx + dx * c - dy * s, cy + dx * s + dy * c)
                    }
                    WarpKind::Pinch => {
                        let dx = xf - cx;
                        let dy = yf - cy;
                        let r = dx.hypot(dy) / rad_max;
                        if r <= 0.0001 {
                            (xf, yf)
                        } else {
                            let nr = r.powf(1.0 + amt * 1.5);
                            let k = nr / r;
                            (cx + dx * k, cy + dy * k)
                        }
                    }
                    WarpKind::Ripple => {
                        let a = amt * 0.08 * rad_max;
                        (
                            xf + a * (yf / hf * detail * TAU_F).sin(),
                            yf + a * (xf / wf * detail * TAU_F).sin(),
                        )
                    }
                    WarpKind::Polar => {
                        let ang = (xf / wf) * TAU_F;
                        let r = (yf / hf) * rad_max;
                        let (s, c) = ang.sin_cos();
                        let tx = cx + r * c;
                        let ty = cy + r * s;
                        // Mezcla con la identidad según la intensidad.
                        (xf + (tx - xf) * amt, yf + (ty - yf) * amt)
                    }
                    WarpKind::Pixelate => {
                        let block = 2.0 + amt * 40.0;
                        let bx = ((xf / block).floor() + 0.5) * block;
                        let by = ((yf / block).floor() + 0.5) * block;
                        (bx, by)
                    }
                    WarpKind::Mirror => {
                        let half = if xf < cx { xf } else { wf - 1.0 - xf };
                        (xf + (half - xf) * amt, yf)
                    }
                    WarpKind::Kaleidoscope => {
                        let dx = xf - cx;
                        let dy = yf - cy;
                        let r = dx.hypot(dy);
                        let seg = TAU_F / detail;
                        let mut ang = dy.atan2(dx);
                        ang = (ang.rem_euclid(seg) - seg * 0.5).abs();
                        let tx = cx + r * ang.cos();
                        let ty = cy + r * ang.sin();
                        (xf + (tx - xf) * amt, yf + (ty - yf) * amt)
                    }
                    // Atendido arriba (retorno temprano); identidad por completitud.
                    WarpKind::Engine => (xf, yf),
                };
                let px = bilinear(src, w, h, sx, sy);
                let o = x * 4;
                row[o..o + 4].copy_from_slice(&px);
            }
        });
    dst
}
