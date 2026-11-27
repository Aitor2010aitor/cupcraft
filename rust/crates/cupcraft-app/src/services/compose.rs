//! Compositor CPU de fotogramas (exportación). Reproduce el orden de capas y
//! las transformaciones de `renderer.js` (`activeLayers`, `clipAlpha`,
//! `drawContain`): medios ajustados en modo *contain*, desplazamiento x/y en
//! % del lienzo, rotación, escala, opacidad × fundidos, filtros de color y
//! ajustes; texto y stickers encima.

use super::raster;
use super::video::{fit_dims, FrameStream};
use rayon::prelude::*;
use cupcraft_core::{Clip, ClipKind, Project, TrackType};
use eframe::egui::Color32;
use resvg::tiny_skia::{
    BlendMode, Color, FillRule, GradientStop, LinearGradient, Mask, Paint, Path, PathBuilder,
    Pixmap, PixmapPaint, Point, RadialGradient, Rect, Shader, SpreadMode, Stroke, StrokeDash,
    Transform,
};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::path::PathBuf;

/// Rango de pintado de cada tipo de clip (igual que `activeLayers`).
pub fn rank(kind: ClipKind) -> i32 {
    match kind {
        ClipKind::Text => 3,
        ClipKind::Sticker => 2,
        ClipKind::Audio => -1,
        _ => 1,
    }
}

// ── Fotogramas clave (`keyframes.js`: `keyframeValue`/`transformAt`) ──
/// Valor de una propiedad en el instante `time` (tiempo absoluto del proyecto),
/// interpolando linealmente sus fotogramas clave. `fallback` si no tiene.
fn kf_value(clip: &Clip, prop: &str, time: f64, fallback: f64) -> f64 {
    let Some(list) = clip
        .extra
        .get("keyframes")
        .and_then(Value::as_object)
        .and_then(|k| k.get(prop))
        .and_then(Value::as_array)
    else {
        return fallback;
    };
    if list.is_empty() {
        return fallback;
    }
    let pt = |i: usize| -> Option<(f64, f64)> {
        let o = list.get(i)?;
        Some((o.get("t")?.as_f64()?, o.get("v")?.as_f64()?))
    };
    let Some((t0, v0)) = pt(0) else { return fallback };
    if time <= t0 {
        return v0;
    }
    let Some((tl, vl)) = pt(list.len() - 1) else { return fallback };
    if time >= tl {
        return vl;
    }
    for i in 1..list.len() {
        let Some((tn, vn)) = pt(i) else { continue };
        if time <= tn {
            let (tp, vp) = pt(i - 1).unwrap_or((tn, vn));
            let span = tn - tp;
            let prog = if span > 0.0 { (time - tp) / span } else { 1.0 };
            return vp + (vn - vp) * prog;
        }
    }
    vl
}

/// Transformación completa del clip en `time` (`transformAt`): respeta la
/// escala uniforme (X e Y comparten valor salvo que `uniform` sea `false`).
pub struct Tr {
    pub x: f64,
    pub y: f64,
    pub rotate: f64,
    pub scale_x: f64,
    pub scale_y: f64,
}

fn transform_at(clip: &Clip, time: f64) -> Tr {
    let uniform = clip.extra.get("uniform").and_then(Value::as_bool) != Some(false);
    let base = clip.scale;
    let (scale_x, scale_y) = if uniform {
        let s = kf_value(clip, "scale", time, base);
        (s, s)
    } else {
        let bx = clip.extra.get("scaleX").and_then(Value::as_f64).unwrap_or(base);
        let by = clip.extra.get("scaleY").and_then(Value::as_f64).unwrap_or(base);
        (kf_value(clip, "scaleX", time, bx), kf_value(clip, "scaleY", time, by))
    };
    Tr {
        x: kf_value(clip, "x", time, clip.x),
        y: kf_value(clip, "y", time, clip.y),
        rotate: kf_value(clip, "rotate", time, clip.rotate),
        scale_x,
        scale_y,
    }
}

/// Opacidad efectiva del clip en el instante local (opacidad × fundidos).
pub fn clip_alpha(clip: &Clip, local: f64) -> f32 {
    let mut a = kf_value(clip, "opacity", clip.start + local, clip.opacity);
    let has_tr = clip.extra.get("transition").and_then(|v| v.as_str()).map(|s| s != "none").unwrap_or(false);
    if clip.fade_in > 0.0 && local < clip.fade_in && !has_tr {
        a *= (local / clip.fade_in).clamp(0.0, 1.0);
    }
    let rem = clip.duration - local;
    if clip.fade_out > 0.0 && rem < clip.fade_out {
        a *= (rem / clip.fade_out).clamp(0.0, 1.0);
    }
    a.clamp(0.0, 1.0) as f32
}

/// Capas activas en `t`, ordenadas para pintar.
pub fn active_layers(project: &Project, t: f64) -> Vec<(&Clip, f64)> {
    let mut out = Vec::new();
    // La pista de arriba de la línea de tiempo se pinta encima (como en CapCut).
    for track in project.tracks.iter().rev() {
        if track.hidden {
            continue;
        }
        for clip in &track.clips {
            if t >= clip.start - 0.0001 && t < clip.start + clip.duration {
                out.push((clip, (t - clip.start).max(0.0)));
            }
        }
    }
    out.sort_by_key(|(c, _)| rank(c.kind));
    out
}

/// Tamaño del lienzo para una proporción y una altura base.
pub fn canvas_size(ratio: &str, short_side: u32) -> (u32, u32) {
    let (rw, rh) = ratio
        .split_once(':')
        .and_then(|(a, b)| Some((a.trim().parse::<f64>().ok()?, b.trim().parse::<f64>().ok()?)))
        .filter(|(a, b)| *a > 0.0 && *b > 0.0)
        .unwrap_or((16.0, 9.0));
    let s = short_side as f64;
    let (w, h) = if rw >= rh { (s * rw / rh, s) } else { (s, s * rh / rw) };
    (((w.round() as u32) & !1).max(2), ((h.round() as u32) & !1).max(2))
}

/// Convierte un fotograma RGBA de origen (`w`×`h`) a un pixmap premultiplicado
/// aplicando, en el mismo orden que `drawMediaSource`+`drawVignette` del
/// original: cadena de color (filtros/ajuste), chroma key por píxel, viñeta y
/// —antes de todo— el desenfoque de zona (`aiRemove`). `cw`/`ch` son las
/// dimensiones del lienzo, necesarias para escalar el radio del desenfoque.
fn rgba_to_pixmap(
    w: u32,
    h: u32,
    rgba: &[u8],
    clip: Option<&Clip>,
    cw: u32,
    ch: u32,
    gpu: Option<&super::gpu::GpuCtx>,
    local: f64,
) -> Option<Pixmap> {
    let mut pm = Pixmap::new(w, h)?;
    // Cadena de color del clip. Para un filtro GL de textura pura se omite su
    // `css` de reserva (lo pinta `apply_gl_texture` como textura real sobre el
    // fotograma final), conservando el resto de etapas (ajuste, HSL, curvas…).
    let chain = clip
        .map(|c| {
            let base = if c
                .gl_filter
                .as_deref()
                .is_some_and(cupcraft_core::filters::is_pure_texture_gl)
            {
                String::new()
            } else {
                crate::paint::clip_css(c)
            };
            crate::paint::clip_chain_from(c, base)
        })
        .unwrap_or_default();
    // Viñeta del panel Ajustar (`drawVignette`): degradado radial negro
    // desde el 42 % del radio hasta la esquina.
    let vig = clip.map(crate::paint::vignette_strength).unwrap_or(0.0);
    let chroma = clip.and_then(chroma_params);
    // Desenfoque de zona (`drawAiRemove`): se calcula sobre una copia de la
    // fuente antes de procesar el color, igual que el original difumina la
    // región del lienzo del clip.
    let zone = clip.and_then(|c| zone_blur_region(c, w, h, cw));
    // Deformación geométrica real (remolino, pellizco, ondas…) del efecto
    // activo. Se aplica a la fuente ANTES del desenfoque y el color, igual que
    // la vista previa (`services::video::texture_fx`), para que export == preview.
    let warp = clip.and_then(|c| crate::paint::clip_warp_at(c, local));
    // Desenfoque de `ctx.filter` (estático): px de lienzo → px de la fuente.
    let fb_sigma = clip.map(filter_blur_px).unwrap_or(0.0) * w as f32 / cw.max(1) as f32;
    // Interruptores locales de Video → Básico (estabilizar, ruido, flujo…).
    let ai = clip.and_then(|c| {
        super::ai_local::AiParams::from_clip(c).map(|p| super::ai_local::AiCtx {
            id: format!("e:{}", c.id),
            local,
            dur: c.duration,
            p,
        })
    });
    let owned;
    let src: &[u8] = if warp.is_some() || zone.is_some() || fb_sigma >= 0.05 || ai.is_some() {
        let mut v = if let Some(wp) = warp {
            crate::paint::warp_rgba(rgba, w as usize, h as usize, &wp)
        } else {
            rgba.to_vec()
        };
        if let Some(a) = &ai {
            super::ai_local::process(a, &mut v, w as usize, h as usize);
        }
        if let Some(z) = zone {
            box_blur_region(&mut v, w as usize, h as usize, z);
        }
        if fb_sigma >= 0.05 {
            gaussian_blur(&mut v, w as usize, h as usize, fb_sigma);
        }
        owned = v;
        &owned
    } else {
        rgba
    };
    let _ = ch;
    let (cx, cy) = (w as f32 / 2.0, h as f32 / 2.0);
    let radius = (cx * cx + cy * cy).sqrt().max(1.0);
    // Fase B2 — cadena de color + viñeta + premultiplicado en GPU (compute). Solo
    // cuando no hay chroma ni giro de tono (`hue`), que conservan la ruta CPU
    // exacta (HSV de egui / chroma key), y cuando hay trabajo real que hacer.
    // Reserva en CPU (`rayon`, abajo) si no hay GPU o la pasada falla.
    if let Some(g) = gpu {
        let hue_free = chain.iter().all(|fx| fx.hue.abs() < 1e-4);
        if chroma.is_none() && hue_free && (!chain.is_empty() || vig > 0.0) {
            let recs: Vec<[f32; 8]> = chain
                .iter()
                .map(|fx| {
                    [
                        fx.invert,
                        fx.brightness,
                        fx.contrast,
                        fx.grayscale,
                        fx.saturate,
                        fx.sepia,
                        fx.opacity,
                        0.0,
                    ]
                })
                .collect();
            if g.run_color(w, h, src, pm.data_mut(), &recs, vig) {
                return Some(pm);
            }
        }
    }
    let dst = pm.data_mut();
    // Compositing por filas en paralelo (rayon): cada píxel es independiente, así
    // que repartir las filas entre núcleos da un resultado idéntico bit a bit y
    // acelera la exportación en equipos multinúcleo.
    let wq = w as usize;
    dst.par_chunks_mut(wq * 4)
        .zip(src.par_chunks(wq * 4))
        .enumerate()
        .for_each(|(y, (dst_row, src_row))| {
            let row_base = y * wq;
            for (x, (dpx, px)) in dst_row
                .chunks_exact_mut(4)
                .zip(src_row.chunks_exact(4))
                .enumerate()
            {
                let i = row_base + x;
                let (mut r, mut g, mut b, mut a) = (px[0], px[1], px[2], px[3]);
                if !chain.is_empty() {
                    let c = chain.iter().fold(Color32::from_rgba_unmultiplied(r, g, b, 255), |c, fx| crate::paint::apply_fx(c, fx));
                    let [r2, g2, b2, _] = c.to_srgba_unmultiplied();
                    r = r2;
                    g = g2;
                    b = b2;
                }
                if let Some(ck) = &chroma {
                    let (r2, g2, b2, a2) = apply_chroma(ck, r, g, b, a);
                    r = r2;
                    g = g2;
                    b = b2;
                    a = a2;
                }
                if vig > 0.0 {
                    let (px_, py_) = ((i as u32 % w) as f32 - cx, (i as u32 / w) as f32 - cy);
                    let t = (((px_ * px_ + py_ * py_).sqrt() / radius - 0.42) / 0.58).clamp(0.0, 1.0);
                    let k = 1.0 - vig * t;
                    r = (r as f32 * k) as u8;
                    g = (g as f32 * k) as u8;
                    b = (b as f32 * k) as u8;
                }
                let af = a as u16;
                dpx[0] = ((r as u16 * af) / 255) as u8;
                dpx[1] = ((g as u16 * af) / 255) as u8;
                dpx[2] = ((b as u16 * af) / 255) as u8;
                dpx[3] = a;
            }
        });
    Some(pm)
}

// ── Chroma key (`applyChromaKeyPixels` de renderer.js) ──
struct Chroma {
    kr: f32,
    kg: f32,
    kb: f32,
    threshold: f32,
    feather_band: f32,
    shadow: f32,
    despill: bool,
    despill_strength: f32,
}

fn mnum(m: &Map<String, Value>, k: &str, d: f64) -> f64 {
    m.get(k).and_then(Value::as_f64).unwrap_or(d)
}

/// ¿El clip tiene el recorte IA activado y una placa de fondo definida?
/// (`drawMattingSource` de la web solo recorta con placa de imagen o color.)
fn matte_active(clip: &Clip) -> bool {
    if !clip.extra.get("autoCutout").and_then(Value::as_bool).unwrap_or(false) {
        return false;
    }
    let has_bg = clip.extra.get("autoCutoutBg").and_then(Value::as_str).is_some_and(|s| !s.is_empty());
    let has_color =
        clip.extra.get("autoCutoutBgColor").and_then(Value::as_str).is_some_and(|s| !s.is_empty());
    has_bg || has_color
}

fn chroma_params(clip: &Clip) -> Option<Chroma> {
    let ck = clip.extra.get("chromaKey")?.as_object()?;
    if !ck.get("active").and_then(Value::as_bool).unwrap_or(false) {
        return None;
    }
    let color = ck.get("color").and_then(Value::as_str).unwrap_or("#00ff00");
    let [kr, kg, kb, _] = crate::paint::hex(color).to_array();
    let strength = (mnum(ck, "strength", 20.0) / 100.0) as f32;
    let feather = (mnum(ck, "feather", 0.0) / 100.0) as f32;
    let shadow = (mnum(ck, "shadow", 0.0) / 100.0) as f32;
    let cleanup = (mnum(ck, "cleanup", 0.0) / 100.0) as f32;
    Some(Chroma {
        kr: kr as f32,
        kg: kg as f32,
        kb: kb as f32,
        threshold: 0.15 + strength * 0.6 + cleanup * 0.15,
        feather_band: (feather + 0.05).max(0.02),
        shadow,
        despill: ck.get("despill").and_then(Value::as_bool) != Some(false),
        despill_strength: 0.3 + strength * 0.4,
    })
}

fn apply_chroma(ck: &Chroma, r: u8, g: u8, b: u8, a: u8) -> (u8, u8, u8, u8) {
    let (mut rf, mut gf, mut bf, mut af) = (r as f32, g as f32, b as f32, a as f32);
    let dist = ((rf - ck.kr).powi(2) + (gf - ck.kg).powi(2) + (bf - ck.kb).powi(2)).sqrt() / 441.67;
    if dist < ck.threshold {
        let alpha = ((ck.threshold - dist) / ck.feather_band).min(1.0);
        af = (af * (1.0 - alpha)).round();
    } else if ck.shadow > 0.0 {
        let k = 1.0 - ck.shadow * 0.25;
        rf *= k;
        gf *= k;
        bf *= k;
    }
    if ck.despill && af > 0.0 {
        let d2 = ((rf - ck.kr).powi(2) + (gf - ck.kg).powi(2) + (bf - ck.kb).powi(2)).sqrt() / 441.67;
        let band = ck.threshold + 0.15;
        if d2 < band {
            let ds = (1.0 - (d2 / band).min(1.0)) * ck.despill_strength;
            if ck.kg > ck.kr && ck.kg > ck.kb {
                gf = gf * (1.0 - ds) + rf.max(bf) * ds;
            } else if ck.kb > ck.kr && ck.kb > ck.kg {
                bf = bf * (1.0 - ds) + rf.max(gf) * ds;
            } else if ck.kr > ck.kg && ck.kr > ck.kb {
                rf = rf * (1.0 - ds) + gf.max(bf) * ds;
            }
        }
    }
    (
        rf.round().clamp(0.0, 255.0) as u8,
        gf.round().clamp(0.0, 255.0) as u8,
        bf.round().clamp(0.0, 255.0) as u8,
        af.clamp(0.0, 255.0) as u8,
    )
}

// ── Desenfoque de zona (`drawAiRemove` de renderer.js) ──
struct Zone {
    x0: usize,
    y0: usize,
    x1: usize,
    y1: usize,
    radius: usize,
}

fn zone_blur_region(clip: &Clip, w: u32, h: u32, cw: u32) -> Option<Zone> {
    if !clip.extra.get("aiRemove").and_then(Value::as_bool).unwrap_or(false) {
        return None;
    }
    let def = Map::new();
    let area = clip.extra.get("aiRemoveArea").and_then(Value::as_object).unwrap_or(&def);
    let (wf, hf) = (w as f32, h as f32);
    let aw = (mnum(area, "w", 30.0) / 100.0).clamp(0.02, 1.0) as f32 * wf;
    let ah = (mnum(area, "h", 30.0) / 100.0).clamp(0.02, 1.0) as f32 * hf;
    let cx = wf / 2.0 + mnum(area, "x", 0.0) as f32 / 100.0 * wf;
    let cy = hf / 2.0 + mnum(area, "y", 0.0) as f32 / 100.0 * hf;
    let x0 = (cx - aw / 2.0).clamp(0.0, wf) as usize;
    let y0 = (cy - ah / 2.0).clamp(0.0, hf) as usize;
    let x1 = (cx + aw / 2.0).clamp(0.0, wf) as usize;
    let y1 = (cy + ah / 2.0).clamp(0.0, hf) as usize;
    if x1 <= x0 + 1 || y1 <= y0 + 1 {
        return None;
    }
    let blur = clip.extra.get("aiRemoveStrength").and_then(Value::as_f64).unwrap_or(28.0).clamp(4.0, 60.0) as f32;
    // El radio del original está en px de lienzo; se escala a px de la fuente.
    let radius = (blur * wf / cw.max(1) as f32).round().clamp(1.0, 120.0) as usize;
    Some(Zone { x0, y0, x1, y1, radius })
}

/// Desenfoque de caja separable dentro de una región del búfer RGBA.
fn box_blur_region(buf: &mut [u8], w: usize, _h: usize, z: Zone) {
    let Zone { x0, y0, x1, y1, radius } = z;
    if radius == 0 {
        return;
    }
    let (rw, rh) = (x1 - x0, y1 - y0);
    if rw < 2 || rh < 2 {
        return;
    }
    let mut cur = vec![0f32; rw * rh * 4];
    for yy in 0..rh {
        for xx in 0..rw {
            let o = ((y0 + yy) * w + (x0 + xx)) * 4;
            let d = (yy * rw + xx) * 4;
            cur[d] = buf[o] as f32;
            cur[d + 1] = buf[o + 1] as f32;
            cur[d + 2] = buf[o + 2] as f32;
            cur[d + 3] = buf[o + 3] as f32;
        }
    }
    let mut tmp = vec![0f32; rw * rh * 4];
    let win = (2 * radius + 1) as f32;
    let span = radius as isize;
    for yy in 0..rh {
        for xx in 0..rw {
            let mut acc = [0f32; 4];
            for k in -span..=span {
                let sx = (xx as isize + k).clamp(0, rw as isize - 1) as usize;
                let s = (yy * rw + sx) * 4;
                acc[0] += cur[s];
                acc[1] += cur[s + 1];
                acc[2] += cur[s + 2];
                acc[3] += cur[s + 3];
            }
            let d = (yy * rw + xx) * 4;
            tmp[d] = acc[0] / win;
            tmp[d + 1] = acc[1] / win;
            tmp[d + 2] = acc[2] / win;
            tmp[d + 3] = acc[3] / win;
        }
    }
    for yy in 0..rh {
        for xx in 0..rw {
            let mut acc = [0f32; 4];
            for k in -span..=span {
                let sy = (yy as isize + k).clamp(0, rh as isize - 1) as usize;
                let s = (sy * rw + xx) * 4;
                acc[0] += tmp[s];
                acc[1] += tmp[s + 1];
                acc[2] += tmp[s + 2];
                acc[3] += tmp[s + 3];
            }
            let o = ((y0 + yy) * w + (x0 + xx)) * 4;
            buf[o] = (acc[0] / win).round().clamp(0.0, 255.0) as u8;
            buf[o + 1] = (acc[1] / win).round().clamp(0.0, 255.0) as u8;
            buf[o + 2] = (acc[2] / win).round().clamp(0.0, 255.0) as u8;
            buf[o + 3] = (acc[3] / win).round().clamp(0.0, 255.0) as u8;
        }
    }
}

// ── Desenfoque de `ctx.filter` (`clipFilter`/transiciones de renderer.js) ──
/// Desenfoque gaussiano separable sobre un búfer RGBA de 8 bits (4 canales).
/// Aproxima el `blur(σpx)` de `ctx.filter` del original. Opera igual sobre los
/// cuatro canales, así vale tanto para RGBA directo como premultiplicado.
fn gaussian_blur(buf: &mut [u8], w: usize, h: usize, sigma: f32) {
    if sigma < 0.05 || w < 2 || h < 2 {
        return;
    }
    let radius = (sigma * 3.0).ceil().clamp(1.0, 200.0) as usize;
    // Núcleo gaussiano 1D normalizado.
    let mut kernel = vec![0f32; 2 * radius + 1];
    let inv = 1.0 / (2.0 * sigma * sigma);
    let mut sum = 0.0;
    for (i, k) in kernel.iter_mut().enumerate() {
        let d = i as f32 - radius as f32;
        *k = (-d * d * inv).exp();
        sum += *k;
    }
    for k in kernel.iter_mut() {
        *k /= sum;
    }
    let n = w * h * 4;
    let src: Vec<f32> = buf[..n].iter().map(|&v| v as f32).collect();
    let mut tmp = vec![0f32; n];
    let span = radius as isize;
    // Pase horizontal.
    for y in 0..h {
        let row = y * w;
        for x in 0..w {
            let mut acc = [0f32; 4];
            for k in -span..=span {
                let sx = (x as isize + k).clamp(0, w as isize - 1) as usize;
                let s = (row + sx) * 4;
                let wk = kernel[(k + span) as usize];
                acc[0] += src[s] * wk;
                acc[1] += src[s + 1] * wk;
                acc[2] += src[s + 2] * wk;
                acc[3] += src[s + 3] * wk;
            }
            let d = (row + x) * 4;
            tmp[d] = acc[0];
            tmp[d + 1] = acc[1];
            tmp[d + 2] = acc[2];
            tmp[d + 3] = acc[3];
        }
    }
    // Pase vertical.
    for y in 0..h {
        for x in 0..w {
            let mut acc = [0f32; 4];
            for k in -span..=span {
                let sy = (y as isize + k).clamp(0, h as isize - 1) as usize;
                let s = (sy * w + x) * 4;
                let wk = kernel[(k + span) as usize];
                acc[0] += tmp[s] * wk;
                acc[1] += tmp[s + 1] * wk;
                acc[2] += tmp[s + 2] * wk;
                acc[3] += tmp[s + 3] * wk;
            }
            let o = (y * w + x) * 4;
            buf[o] = acc[0].round().clamp(0.0, 255.0) as u8;
            buf[o + 1] = acc[1].round().clamp(0.0, 255.0) as u8;
            buf[o + 2] = acc[2].round().clamp(0.0, 255.0) as u8;
            buf[o + 3] = acc[3].round().clamp(0.0, 255.0) as u8;
        }
    }
}

/// Radio total de `blur(px)` del `ctx.filter` estático del clip, en px de
/// lienzo, combinando las fuentes de `clipFilter`/`beautyCss` del original en
/// cuadratura (desenfoques gaussianos encadenados). No incluye el de la
/// transición, que depende del tiempo y se aplica aparte.
fn filter_blur_px(clip: &Clip) -> f32 {
    let mut sq = 0.0f32;
    let mut add = |px: f32| {
        if px > 0.0 {
            sq += px * px;
        }
    };
    // Desenfoque del filtro activo (`FILTERS[clip.filter].css`), ya resuelto por
    // `paint::clip_css` (p. ej. `blur` → blur(2.4px), `cut_watercolor` → 0.4px).
    for tok in crate::paint::clip_css(clip).split_whitespace() {
        if let Some(rest) = tok.strip_prefix("blur(") {
            if let Ok(v) = rest.trim_end_matches(')').trim_end_matches("px").parse::<f32>() {
                add(v);
            }
        }
    }
    // Suavizado de piel (`beautyCss`: blur(smooth*0.022), máx 2.2px).
    if let Some(b) = clip.extra.get("beauty").and_then(Value::as_object) {
        let smooth = b.get("smooth").and_then(Value::as_f64).unwrap_or(0.0);
        if smooth > 0.0 {
            add((smooth as f32 * 0.022).clamp(0.0, 2.2));
        }
    }
    // Reducción de ruido / flujo óptico (`clipFilter`).
    if flag(clip, "denoise") {
        add(0.4);
    }
    if flag(clip, "opticalFlow") {
        add(0.6);
    }
    // Desenfoque del filtro de efecto (`spec.css` de `library.js`), resuelto
    // por `paint::effect_css` (p. ej. esp_dreamy/mov_blur → 1.1px,
    // cut_watercolor → 0.4px).
    if let Some(id) = clip.extra.get("effect").and_then(Value::as_str) {
        let css = crate::paint::fx_effect_css(id, clip.extra.get("effectParams"));
        for tok in css.split_whitespace() {
            if let Some(rest) = tok.strip_prefix("blur(") {
                if let Ok(v) = rest.trim_end_matches(')').trim_end_matches("px").parse::<f32>() {
                    add(v);
                }
            }
        }
    }
    sq.sqrt()
}

// ── Máscara (`applyMask`/`traceMaskPath` de renderer.js) ──
/// Construye la máscara de recorte del clip en el espacio del lienzo,
/// reproduciendo la transformación del original (centro del clip → rotación →
/// escala×volteo → desplazamiento y rotación de la máscara).
/// Trazado de la máscara de forma del clip (`applyMask`), ya transformado al
/// espacio del lienzo. Devuelve `None` cuando el clip no define máscara.
fn mask_shape_path(clip: &Clip, time: f64, cw: u32, ch: u32) -> Option<(Path, FillRule, Transform)> {
    let m = clip.extra.get("mask")?.as_object()?;
    let shape = m.get("shape").and_then(Value::as_str).unwrap_or("none");
    if shape == "none" {
        return None;
    }
    let (w, h) = (cw as f32, ch as f32);
    let scale = mnum(m, "scale", 1.0).max(0.05) as f32;
    let (mw, mh) = (w * scale, h * scale);
    let invert = m.get("invert").and_then(Value::as_bool).unwrap_or(false);
    let mut pb = PathBuilder::new();
    if invert {
        pb.push_rect(Rect::from_xywh(-w * 2.0, -h * 2.0, w * 4.0, h * 4.0)?);
    }
    mask_shape(&mut pb, shape, mw, mh);
    let path = pb.finish()?;
    let flip_h = if flag(clip, "flipH") { -1.0 } else { 1.0 };
    let flip_v = if flag(clip, "flipV") { -1.0 } else { 1.0 };
    // Transformación del clip con fotogramas clave (`transformAt`).
    let tf = transform_at(clip, time);
    let cx = w / 2.0 + tf.x as f32 / 100.0 * w;
    let cy = h / 2.0 + tf.y as f32 / 100.0 * h;
    let mx = mnum(m, "x", 0.0) as f32 / 100.0 * w;
    let my = mnum(m, "y", 0.0) as f32 / 100.0 * h;
    let mrot = mnum(m, "rotate", 0.0) as f32;
    let tr = Transform::from_translate(cx, cy)
        .pre_rotate(tf.rotate as f32)
        .pre_scale(tf.scale_x as f32 * flip_h, tf.scale_y as f32 * flip_v)
        .pre_translate(mx, my)
        .pre_rotate(mrot);
    let rule = if invert { FillRule::EvenOdd } else { FillRule::Winding };
    Some((path, rule, tr))
}

/// Máscara de recorte final: combina la forma del clip (`applyMask`) con el
/// barrido de la transición de entrada (`wipeClip`). `tr_wipe` es la
/// transformación de dibujo hasta el `scale` (igual que en el original, donde
/// `wipeClip` se aplica tras escalar). Devuelve `None` si no hay recorte.
fn clip_mask(clip: &Clip, time: f64, anim: Option<&Anim>, cw: u32, ch: u32, tr_wipe: Transform) -> Option<Mask> {
    let shape = mask_shape_path(clip, time, cw, ch);
    let wipe = anim.and_then(|a| wipe_spec(a, cw as f32, ch as f32));
    if shape.is_none() && wipe.is_none() {
        return None;
    }
    let mut mask = Mask::new(cw, ch)?;
    match &wipe {
        Some((wp, wr, wrot)) => {
            let t = tr_wipe.pre_rotate(*wrot);
            mask.fill_path(wp, *wr, true, t);
            if let Some((sp, sr, st)) = &shape {
                mask.intersect_path(sp, *sr, true, *st);
            }
        }
        None => {
            let (sp, sr, st) = shape.as_ref()?;
            mask.fill_path(sp, *sr, true, *st);
        }
    }
    Some(mask)
}

// ── Espejo y clon fantasma (`drawMediaLayerRaw` de renderer.js) ──
/// Movimiento compuesto del efecto: espejo y clones fantasma. Los datos salen
/// del catálogo `EFFECTS` de `src/filtros/library.js` (`mirror`/`ghost`/`move`).
struct MoveSpec {
    /// Espejo horizontal de la mitad izquierda (`spec.mirror`).
    mirror: bool,
    /// Número de clones fantasma (`spec.ghost` o 2 para `cloneGhost`).
    ghosts: u32,
    /// Patrón de clon lateral de `cloneGhost` (`spec.move === 'cloneGhost'`).
    clone: bool,
}

fn move_spec(clip: &Clip) -> Option<MoveSpec> {
    let id = clip.extra.get("effect").and_then(Value::as_str)?;
    match id {
        // `mirror`: fx(..., { mirror: true }).
        "mirror" => Some(MoveSpec { mirror: true, ghosts: 0, clone: false }),
        // `mov_blur`: fx(..., { move: 'blur', ghost: 3 }). (El desenfoque va aparte.)
        "mov_blur" => Some(MoveSpec { mirror: false, ghosts: 3, clone: false }),
        // `body_clone`: fx(..., { move: 'cloneGhost' }) → 2 clones laterales.
        "body_clone" => Some(MoveSpec { mirror: false, ghosts: 2, clone: true }),
        _ => None,
    }
}

/// Máscara de una mitad del efecto espejo: rectángulo `[-w/2..0] × [-h/2..h/2]`
/// (en el espacio `rect_tr`) intersecado con la forma y el barrido del clip,
/// igual que el `rect`+`clip` dentro del `applyMask` de `drawMediaLayerRaw`.
#[allow(clippy::too_many_arguments)]
fn mirror_mask(
    clip: &Clip,
    time: f64,
    anim: Option<&Anim>,
    cw: u32,
    ch: u32,
    tr_wipe: Transform,
    rect_tr: Transform,
    w: f32,
    h: f32,
) -> Option<Mask> {
    let mut mask = Mask::new(cw, ch)?;
    let r = Rect::from_xywh(-w / 2.0, -h / 2.0, w / 2.0, h)?;
    let mut pb = PathBuilder::new();
    pb.push_rect(r);
    let path = pb.finish()?;
    mask.fill_path(&path, FillRule::Winding, true, rect_tr);
    // Intersección con la forma de máscara del clip (`applyMask`).
    if let Some((sp, sr, st)) = mask_shape_path(clip, time, cw, ch) {
        mask.intersect_path(&sp, sr, true, st);
    }
    // Intersección con el barrido de la transición (`wipeClip`), sin voltear.
    if let Some((wp, wr, wrot)) = anim.and_then(|a| wipe_spec(a, cw as f32, ch as f32)) {
        mask.intersect_path(&wp, wr, true, tr_wipe.pre_rotate(wrot));
    }
    Some(mask)
}

// ── Trazo del recorte (`drawStroke` de renderer.js) ──
/// Contornea la forma de máscara del clip con el color/grosor/discontinuo de
/// `cutoutStroke`. Requiere `cutoutStroke.active` y una máscara activa
/// (`maskActive`: forma distinta de `none`), igual que el original.
fn draw_cutout_stroke(canvas: &mut Pixmap, clip: &Clip, time: f64, cw: u32, ch: u32) {
    let Some(s) = clip.extra.get("cutoutStroke").and_then(Value::as_object) else {
        return;
    };
    if !s.get("active").and_then(Value::as_bool).unwrap_or(false) {
        return;
    }
    let Some(m) = clip.extra.get("mask").and_then(Value::as_object) else {
        return;
    };
    let shape = m.get("shape").and_then(Value::as_str).unwrap_or("none");
    if shape == "none" {
        return;
    }
    let color = s.get("color").and_then(Value::as_str).unwrap_or("#ffffff");
    let size = s.get("size").and_then(Value::as_f64).unwrap_or(10.0).max(1.0) as f32;
    let spacing = s.get("spacing").and_then(Value::as_f64).unwrap_or(0.0) as f32;
    // Trazado de la forma de máscara (sin invertir), como `traceMaskPath`.
    let (w, h) = (cw as f32, ch as f32);
    let scale = mnum(m, "scale", 1.0).max(0.05) as f32;
    let (mw, mh) = (w * scale, h * scale);
    let mut pb = PathBuilder::new();
    mask_shape(&mut pb, shape, mw, mh);
    let Some(path) = pb.finish() else {
        return;
    };
    // Transformación del clip al espacio del lienzo (idéntica a `mask_shape_path`).
    let flip_h = if flag(clip, "flipH") { -1.0 } else { 1.0 };
    let flip_v = if flag(clip, "flipV") { -1.0 } else { 1.0 };
    let tf = transform_at(clip, time);
    let cx = w / 2.0 + tf.x as f32 / 100.0 * w;
    let cy = h / 2.0 + tf.y as f32 / 100.0 * h;
    let mx = mnum(m, "x", 0.0) as f32 / 100.0 * w;
    let my = mnum(m, "y", 0.0) as f32 / 100.0 * h;
    let mrot = mnum(m, "rotate", 0.0) as f32;
    let tr = Transform::from_translate(cx, cy)
        .pre_rotate(tf.rotate as f32)
        .pre_scale(tf.scale_x as f32 * flip_h, tf.scale_y as f32 * flip_v)
        .pre_translate(mx, my)
        .pre_rotate(mrot);
    // `setLineDash([spacing, spacing])` cuando spacing > 0.
    let dash = if spacing > 0.0 { StrokeDash::new(vec![spacing, spacing], 0.0) } else { None };
    let st = Stroke { width: size, dash, ..Default::default() };
    stroke_ex(canvas, &path, raster::parse_color(color, 1.0), &st, tr, None);
}

// ── Transiciones de entrada (`transitionState`/`transitionAlpha`/
// `applyEntrance`/`wipeClip` de renderer.js) ──
/// Estado de la transición de entrada en el instante local del clip.
pub struct Anim {
    kind: String,
    p: f32,
}

fn ease_out_cubic(v: f32) -> f32 {
    1.0 - (1.0 - v).powi(3)
}

/// `hash01` del original (ruido determinista a partir de un índice).
fn hash01(v: f64) -> f64 {
    let x = (v * 127.1 + 311.7).sin() * 43758.5453;
    x - x.floor()
}

// ── Partículas de efectos (`drawParticles` de renderer.js) ──
/// Ficha de partículas de un efecto, copiada de `EFFECTS` en
/// `src/filtros/library.js` (campos `particles`/`glyphs`/`count`/`speed`/`size`/
/// `colors`/`spawn`). Sin inventar: los datos provienen del catálogo original.
struct ParticleSpec {
    kind: &'static str,
    glyphs: &'static [&'static str],
    count: u32,
    speed: f64,
    size: f64,
    colors: &'static [&'static str],
    spawn: &'static str,
}

/// Tabla de partículas por id de efecto (`clip.effect`). Replica las 39 entradas
/// con `particles:` de `src/filtros/library.js`.
fn particle_spec(clip: &Clip) -> Option<ParticleSpec> {
    let id = clip.extra.get("effect").and_then(Value::as_str)?;
    let s = |kind, glyphs: &'static [&'static str], count, speed, size, colors: &'static [&'static str], spawn| {
        Some(ParticleSpec { kind, glyphs, count, speed, size, colors, spawn })
    };
    const NONE: &[&str] = &[];
    match id {
        "tnd_heart" => s("heart", &["❤", "💖", "💕"], 22, 0.55, 40.0, NONE, ""),
        "tnd_laugh" => s("emoji", &["😂", "❤", "✨"], 16, 0.5, 42.0, NONE, ""),
        "pat_feather" => s("emoji", &["🪶", "🍃", "✨"], 18, 0.35, 40.0, NONE, ""),
        "pat_happy" => s("emoji", &["🦆", "🐤", "💛"], 18, 0.45, 44.0, NONE, ""),
        "pat_egg" => s("emoji", &["🥚", "✨", "🐣"], 16, 0.4, 40.0, NONE, ""),
        "pat_rain" => s("emoji", &["🦆", "🐤", "💛"], 28, 0.75, 38.0, NONE, "top"),
        "pat_bubbles" => s("bubble", NONE, 26, 0.35, 34.0, &["#bff4ff", "#ffffff", "#8ee7ff"], ""),
        "hal_pumpkin" => s("emoji", &["🎃", "🍬", "🕸"], 20, 0.35, 40.0, NONE, ""),
        "hal_ghost" => s("emoji", &["👻", "💀", "🌫"], 20, 0.3, 44.0, NONE, ""),
        "hal_bats" => s("emoji", &["🦇", "🌙", "✨"], 20, 0.45, 38.0, NONE, ""),
        "hal_skull" => s("emoji", &["💀", "🦴", "✨"], 16, 0.32, 40.0, NONE, ""),
        "hal_witch" => s("emoji", &["🧙", "✨", "🕯"], 16, 0.4, 42.0, NONE, ""),
        "esp_confetti" => s("confetti", NONE, 40, 0.55, 22.0, &["#ff4d6d", "#ffd166", "#4cc9f0", "#8ac926", "#c77dff"], ""),
        "esp_stars" => s("star", NONE, 26, 0.4, 34.0, &["#fff3a8", "#ffffff", "#ffd166"], ""),
        "esp_glitter" => s("spark", NONE, 48, 0.3, 16.0, &["#ffffff", "#ffe8a3", "#ffc6f2"], ""),
        "esp_fireworks" => s("firework", NONE, 30, 0.6, 26.0, &["#ffd166", "#ff5d8f", "#7bdff2", "#c8ffbe"], ""),
        "esp_hearts" => s("heart", &["❤", "💖", "💘"], 24, 0.45, 38.0, NONE, ""),
        "esp_notes" => s("emoji", &["🎵", "🎶", "♪"], 22, 0.4, 36.0, NONE, ""),
        "esp_petals" => s("emoji", &["🌸", "🌺", "🌼"], 24, 0.35, 36.0, NONE, ""),
        "esp_candles" => s("emoji", &["🕯", "✨"], 14, 0.25, 40.0, NONE, ""),
        "esp_magic" => s("emoji", &["✨", "🪄", "🔮"], 22, 0.5, 38.0, NONE, ""),
        "blk_tags" => s("emoji", &["🏷", "💸", "🛍"], 20, 0.45, 40.0, NONE, ""),
        "blk_money" => s("emoji", &["💵", "💰", "💸"], 26, 0.65, 40.0, NONE, "top"),
        "blk_cart" => s("emoji", &["🛒", "📦", "🎁"], 18, 0.4, 40.0, NONE, ""),
        "blk_arrows" => s("emoji", &["➡", "⬇", "💥"], 18, 0.5, 34.0, NONE, ""),
        "new_particles" => s("spark", NONE, 36, 0.35, 18.0, &["#ffffff", "#7defff", "#ffd166"], ""),
        "bmb_explosion" => s("explosion", NONE, 34, 0.9, 34.0, &["#ffd166", "#ff7b00", "#ff3d00"], ""),
        "bmb_confetti" => s("confetti", NONE, 52, 0.9, 24.0, &["#ff4d6d", "#ffd166", "#4cc9f0", "#8ac926", "#c77dff"], ""),
        "bmb_fire" => s("emoji", &["🔥", "✨"], 30, 0.8, 38.0, NONE, ""),
        "bmb_sparks" => s("spark", NONE, 56, 0.9, 18.0, &["#fff3a8", "#ffd166", "#ffffff"], ""),
        "fst_confetti" => s("confetti", NONE, 44, 0.7, 22.0, &["#ff4d6d", "#ffd166", "#4cc9f0", "#8ac926", "#c77dff"], ""),
        "fst_disco" => s("emoji", &["🪩", "✨"], 18, 0.35, 40.0, NONE, ""),
        "fst_balloons" => s("emoji", &["🎈", "🎉", "✨"], 24, 0.38, 42.0, NONE, ""),
        "fst_streamers" => s("confetti", NONE, 36, 0.5, 18.0, &["#ff4d6d", "#ffd166", "#4cc9f0", "#8ac926"], ""),
        "fst_dj" => s("emoji", &["🎧", "🎵", "🔊"], 20, 0.5, 38.0, NONE, ""),
        "fst_fireworks" => s("firework", NONE, 34, 0.7, 26.0, &["#ffd166", "#ff5d8f", "#7bdff2", "#c8ffbe"], ""),
        "fst_glitter" => s("spark", NONE, 48, 0.4, 16.0, &["#ffffff", "#ffe8a3", "#ffc6f2"], ""),
        "ret_dust" => s("dust", NONE, 36, 0.2, 4.0, &["#ffffff", "#d8c9a8"], ""),
        "body_sticker" => s("emoji", &["✨", "💪", "🔥"], 16, 0.4, 38.0, NONE, ""),
        _ => None,
    }
}

/// Glifos por defecto de cada tipo cuando el efecto no trae `glyphs`
/// (`PARTICLE_GLYPHS`/`particleGlyphs` de renderer.js).
fn particle_glyphs(spec: &ParticleSpec) -> &'static [&'static str] {
    if !spec.glyphs.is_empty() {
        return spec.glyphs;
    }
    match spec.kind {
        "heart" => &["❤", "💖", "💕"],
        "star" => &["★", "✦", "✧"],
        "spark" => &["✦", "✧", "·"],
        "bubble" => &["○", "◯", "●"],
        "dust" => &["·"],
        _ => &["✨"],
    }
}

/// Pinta las partículas de un efecto en espacio de pantalla sobre `canvas`,
/// portando `drawParticles` de renderer.js (burst radial, confeti, chispas,
/// burbujas, polvo y glifos). `alpha` es la opacidad compuesta del clip.
fn draw_particles(canvas: &mut Pixmap, spec: &ParticleSpec, local: f64, width: f32, height: f32, alpha: f32) {
    let kind = spec.kind;
    let count = spec.count.clamp(4, 80);
    let base_size = ((spec.size as f32) * (height / 1080.0)).max(4.0);
    let speed = spec.speed;
    let colors: &[&str] = if spec.colors.is_empty() { &["#ffffff"] } else { spec.colors };
    let glyphs = particle_glyphs(spec);
    let burst = kind == "firework" || kind == "explosion";
    let shapes = kind == "confetti";
    let (w, h) = (width as f64, height as f64);
    for index in 0..count {
        let i = index as f64;
        let seed = hash01(i + 1.0);
        let seed2 = hash01(i + 17.3);
        let seed3 = hash01(i + 41.7);
        let (mut x, y, life);
        if burst {
            let cycle = 1.8;
            let phase = ((local * speed + seed) % cycle) / cycle;
            let angle = seed2 * std::f64::consts::TAU;
            let radius = phase * w.min(h) * (0.28 + seed3 * 0.34);
            x = w / 2.0 + angle.cos() * radius;
            y = h / 2.0 + angle.sin() * radius;
            life = 1.0 - phase;
        } else {
            let drift = (local * speed * 0.075 + seed2) % 1.0;
            let bubbles = kind == "bubble";
            y = if bubbles || spec.spawn == "bottom" { h * (1.0 - drift) } else { h * drift };
            x = seed * w + (local * speed * 2.2 + i * 1.7).sin() * w * 0.035;
            x = ((x % w) + w) % w;
            life = 0.55 + seed3 * 0.45;
        }
        let size = (base_size as f64) * (0.55 + seed3 * 0.75);
        if x < -size || x > w + size || y < -size || y > h + size {
            continue;
        }
        let (xf, yf, sz) = (x as f32, y as f32, size as f32);
        let life = life as f32;
        let color = colors[(index as usize) % colors.len()];
        if shapes {
            let rot = (local * (1.4 + seed) + seed2 * std::f64::consts::PI) as f32;
            let tr = Transform::from_translate(xf, yf).pre_rotate(rot.to_degrees());
            let mut paint = Paint::default();
            paint.set_color(raster::parse_color(color, alpha * life * 0.92));
            paint.anti_alias = true;
            if index % 3 == 0 {
                if let Some(p) = PathBuilder::from_circle(0.0, 0.0, sz * 0.28) {
                    canvas.fill_path(&p, &paint, FillRule::Winding, tr, None);
                }
            } else if let Some(p) = Rect::from_xywh(-sz * 0.24, -sz * 0.5, sz * 0.48, sz * 0.9).map(PathBuilder::from_rect) {
                canvas.fill_path(&p, &paint, FillRule::Winding, tr, None);
            }
            continue;
        }
        if kind == "spark" {
            let rot = (seed * std::f64::consts::PI + local * 1.8) as f32;
            let tr = Transform::from_translate(xf, yf).pre_rotate(rot.to_degrees());
            let arm = sz * 0.52;
            let thin = sz * 0.1;
            let mut pb = PathBuilder::new();
            pb.move_to(0.0, -arm);
            pb.line_to(thin, -thin);
            pb.line_to(arm, 0.0);
            pb.line_to(thin, thin);
            pb.line_to(0.0, arm);
            pb.line_to(-thin, thin);
            pb.line_to(-arm, 0.0);
            pb.line_to(-thin, -thin);
            pb.close();
            if let Some(p) = pb.finish() {
                let mut paint = Paint::default();
                paint.set_color(raster::parse_color(color, alpha * life));
                paint.anti_alias = true;
                canvas.fill_path(&p, &paint, FillRule::Winding, tr, None);
            }
            continue;
        }
        if kind == "bubble" {
            if let Some(p) = PathBuilder::from_circle(xf, yf, sz * 0.32) {
                let mut paint = Paint::default();
                paint.set_color(raster::parse_color(color, alpha * life * 0.5));
                paint.anti_alias = true;
                let stroke = Stroke { width: (sz * 0.06).max(1.0), ..Default::default() };
                canvas.stroke_path(&p, &paint, &stroke, Transform::identity(), None);
            }
            continue;
        }
        if kind == "dust" {
            if let Some(p) = PathBuilder::from_circle(xf, yf, (sz * 0.08).max(0.7)) {
                let mut paint = Paint::default();
                paint.set_color(raster::parse_color(color, alpha * life * 0.7));
                paint.anti_alias = true;
                canvas.fill_path(&p, &paint, FillRule::Winding, Transform::identity(), None);
            }
            continue;
        }
        // Glifo (emoji o símbolo), centrado en (x, y).
        let glyph = glyphs[(index as usize) % glyphs.len()];
        if let Some(pm) = raster::render_glyph(glyph, sz.max(6.0), color) {
            let (gw, gh) = (pm.width() as f32, pm.height() as f32);
            let paint = PixmapPaint { opacity: alpha * life * 0.9, ..Default::default() };
            canvas.draw_pixmap(0, 0, pm.as_ref(), &paint, Transform::from_translate(xf - gw / 2.0, yf - gh / 2.0), None);
        }
    }
}

// ── Marcos de efectos (`drawFrameFx` de renderer.js) ──
/// Ficha de marco de un efecto: estilo (`frame`) + color de acento (`accent`).
/// Copiada de las 19 entradas con `frame:` de `src/filtros/library.js`
/// (el acento por defecto es `#ffffff`, como `spec.accent || '#ffffff'`).
struct FrameSpec {
    style: &'static str,
    accent: &'static str,
}

/// Tabla de marcos por id de efecto (`clip.effect`). Sin inventar: los estilos
/// y acentos provienen del catálogo original.
fn frame_spec(clip: &Clip) -> Option<FrameSpec> {
    let id = clip.extra.get("effect").and_then(Value::as_str)?;
    let f = |style, accent| Some(FrameSpec { style, accent });
    match id {
        "hal_web" => f("web", "#dfe6ef"),
        "esp_rainbow" => f("rainbow", "#ffffff"),
        "blk_coupon" => f("coupon", "#ffd400"),
        "cls_polaroid" => f("polaroid", "#ffffff"),
        "cls_film" => f("film", "#ffffff"),
        "cut_comic" => f("comic", "#ffffff"),
        "new_badge" => f("badge", "#00e5d0"),
        "bmb_boom" => f("comic", "#ffd166"),
        "int_open" => f("open", "#ffffff"),
        "frm_polaroid" => f("polaroid", "#ffffff"),
        "frm_film" => f("film", "#ffffff"),
        "frm_tape" => f("tape", "#ffffff"),
        "frm_neon" => f("neon", "#ff4de1"),
        "frm_floral" => f("floral", "#ffffff"),
        "frm_torn" => f("torn", "#ffffff"),
        "frm_wood" => f("wood", "#ffffff"),
        "frm_dots" => f("dots", "#ffffff"),
        "frm_rainbow" => f("rainbow", "#ffffff"),
        "ret_polaroid" => f("polaroid", "#ffffff"),
        _ => None,
    }
}

/// Rectángulo redondeado (`roundedRect` de renderer.js) como `Path`; las esquinas
/// circulares de `arcTo` se aproximan con curvas cuadráticas.
fn rr_path(x: f32, y: f32, w: f32, h: f32, radius: f32) -> Option<Path> {
    let r = radius.min(w / 2.0).min(h / 2.0).max(0.0);
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

/// Rellena un rectángulo recto (`ctx.fillRect`) con un color ya resuelto.
fn fill_rect(canvas: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, color: Color) {
    if let Some(r) = Rect::from_xywh(x, y, w, h) {
        let mut paint = Paint::default();
        paint.set_color(color);
        paint.anti_alias = true;
        canvas.fill_rect(r, &paint, Transform::identity(), None);
    }
}

/// Pinta el marco de un efecto en espacio de pantalla sobre `canvas`, portando
/// `drawFrameFx` de renderer.js. `alpha` es la opacidad compuesta del clip; los
/// literales `rgba()` del original se resuelven aquí a color + alfa explícitos.
fn draw_frame(canvas: &mut Pixmap, spec: &FrameSpec, width: f32, height: f32, alpha: f32) {
    let style = spec.style;
    let accent = spec.accent;
    let edge = (6.0_f32).max(width.min(height) * 0.035);
    let a8 = |base: f32| ((base * alpha).clamp(0.0, 1.0) * 255.0).round() as u8;
    let rgba = |r: u8, g: u8, b: u8, base: f32| Color::from_rgba8(r, g, b, a8(base));
    let acc = raster::parse_color(accent, alpha);
    use std::f32::consts::PI;
    match style {
        "polaroid" => {
            let bottom = edge * 3.4;
            let cream = rgba(0xf7, 0xf2, 0xe6, 1.0);
            fill_rect(canvas, 0.0, 0.0, width, edge, cream);
            fill_rect(canvas, 0.0, height - bottom, width, bottom, cream);
            fill_rect(canvas, 0.0, 0.0, edge, height, cream);
            fill_rect(canvas, width - edge, 0.0, edge, height, cream);
            if let Some(r) = Rect::from_xywh(edge * 0.35, edge * 0.35, width - edge * 0.7, height - edge * 0.7) {
                let mut paint = Paint::default();
                paint.set_color(rgba(0, 0, 0, 0.18));
                paint.anti_alias = true;
                let stroke = Stroke { width: (edge * 0.08).max(1.0), ..Default::default() };
                canvas.stroke_path(&PathBuilder::from_rect(r), &paint, &stroke, Transform::identity(), None);
            }
        }
        "film" => {
            let strip = (10.0_f32).max(height * 0.075);
            let dark = rgba(0x0b, 0x0b, 0x0d, 1.0);
            fill_rect(canvas, 0.0, 0.0, width, strip, dark);
            fill_rect(canvas, 0.0, height - strip, width, strip, dark);
            let hole_w = strip * 0.55;
            let hole_h = strip * 0.32;
            let hole = rgba(245, 242, 226, 0.9);
            let mut x = strip * 0.35;
            while x < width {
                for ry in [strip * 0.28, height - strip * 0.6] {
                    if let Some(p) = rr_path(x, ry, hole_w, hole_h, hole_h * 0.25) {
                        let mut paint = Paint::default();
                        paint.set_color(hole);
                        paint.anti_alias = true;
                        canvas.fill_path(&p, &paint, FillRule::Winding, Transform::identity(), None);
                    }
                }
                x += hole_w * 1.7;
            }
        }
        "tape" => {
            let tape = width.min(height) * 0.13;
            let col = rgba(255, 238, 166, 0.72);
            for (cx, cy, sx, sy) in [
                (0.0, 0.0, -1.0_f32, -1.0_f32),
                (width, 0.0, 1.0, -1.0),
                (0.0, height, -1.0, 1.0),
                (width, height, 1.0, 1.0),
            ] {
                let tr = Transform::from_translate(cx, cy).pre_rotate((sx * sy * 0.75).to_degrees());
                if let Some(r) = Rect::from_xywh(-tape * 0.14, -tape * 0.5, tape * 0.28, tape) {
                    let mut paint = Paint::default();
                    paint.set_color(col);
                    paint.anti_alias = true;
                    canvas.fill_rect(r, &paint, tr, None);
                }
            }
        }
        "neon" => {
            if let Some(p) = rr_path(edge * 0.7, edge * 0.7, width - edge * 1.4, height - edge * 1.4, edge * 1.2) {
                // `shadowBlur` no existe en tiny_skia: el resplandor se aproxima con un
                // trazo ancho y tenue del color de acento bajo el trazo principal.
                let mut glow = Paint::default();
                glow.set_color(raster::parse_color(accent, alpha * 0.5));
                glow.anti_alias = true;
                let gs = Stroke { width: (edge * 0.28).max(3.0) + (edge * 1.4).max(10.0), ..Default::default() };
                canvas.stroke_path(&p, &glow, &gs, Transform::identity(), None);
                let mut paint = Paint::default();
                paint.set_color(acc);
                paint.anti_alias = true;
                let st = Stroke { width: (edge * 0.28).max(3.0), ..Default::default() };
                canvas.stroke_path(&p, &paint, &st, Transform::identity(), None);
                let mut white = Paint::default();
                white.set_color(rgba(255, 255, 255, 0.7));
                white.anti_alias = true;
                let ws = Stroke { width: (edge * 0.08).max(1.0), ..Default::default() };
                canvas.stroke_path(&p, &white, &ws, Transform::identity(), None);
            }
        }
        "floral" => {
            let glyphs = ["🌸", "🌺", "🌼", "🍃"];
            let px = (12.0_f32).max(edge * 0.95);
            for index in 0..12usize {
                let side = index % 4;
                let t = ((index / 4) as f32 + 0.5) / 3.0;
                let x = if side == 0 || side == 2 { t * width } else if side == 1 { width - edge * 0.6 } else { edge * 0.6 };
                let y = if side == 0 { edge * 0.6 } else if side == 2 { height - edge * 0.6 } else { t * height };
                if let Some(pm) = raster::render_glyph(glyphs[index % glyphs.len()], px, "#ffffff") {
                    let (gw, gh) = (pm.width() as f32, pm.height() as f32);
                    let paint = PixmapPaint { opacity: alpha, ..Default::default() };
                    canvas.draw_pixmap(0, 0, pm.as_ref(), &paint, Transform::from_translate(x - gw / 2.0, y - gh / 2.0), None);
                }
            }
        }
        "torn" => {
            let cream = rgba(0xf2, 0xec, 0xdd, 1.0);
            fill_rect(canvas, 0.0, 0.0, width, edge * 0.7, cream);
            fill_rect(canvas, 0.0, height - edge * 0.7, width, edge * 0.7, cream);
            fill_rect(canvas, 0.0, 0.0, edge * 0.7, height, cream);
            fill_rect(canvas, width - edge * 0.7, 0.0, edge * 0.7, height, cream);
            let dark = rgba(0, 0, 0, 0.18);
            for index in 0..22 {
                let x = hash01(index as f64 + 4.0) as f32 * width;
                let h = edge * (0.15 + hash01(index as f64 + 9.0) as f32 * 0.5);
                let mut pb = PathBuilder::new();
                pb.move_to(x - h * 0.5, edge * 0.7);
                pb.line_to(x, edge * 0.7 + h);
                pb.line_to(x + h * 0.5, edge * 0.7);
                pb.close();
                if let Some(p) = pb.finish() {
                    let mut paint = Paint::default();
                    paint.set_color(dark);
                    paint.anti_alias = true;
                    canvas.fill_path(&p, &paint, FillRule::Winding, Transform::identity(), None);
                }
            }
        }
        "wood" => {
            let brown = rgba(0x5a, 0x38, 0x23, 1.0);
            fill_rect(canvas, 0.0, 0.0, width, edge, brown);
            fill_rect(canvas, 0.0, height - edge, width, edge, brown);
            fill_rect(canvas, 0.0, 0.0, edge, height, brown);
            fill_rect(canvas, width - edge, 0.0, edge, height, brown);
            let grain = rgba(35, 18, 8, 0.55);
            let st = Stroke { width: (edge * 0.08).max(1.0), ..Default::default() };
            for index in 0..14 {
                let y = (index as f32 / 13.0) * height;
                let mut pb = PathBuilder::new();
                pb.move_to(0.0, y);
                pb.cubic_to(width * 0.3, y + edge * 0.3, width * 0.7, y - edge * 0.3, width, y);
                if let Some(p) = pb.finish() {
                    let mut paint = Paint::default();
                    paint.set_color(grain);
                    paint.anti_alias = true;
                    canvas.stroke_path(&p, &paint, &st, Transform::identity(), None);
                }
            }
        }
        "dots" => {
            let step = (10.0_f32).max(width.min(height) * 0.055);
            let r = step * 0.14;
            let mut x = step;
            while x < width {
                for cy in [step * 0.7, height - step * 0.7] {
                    if let Some(p) = PathBuilder::from_circle(x, cy, r) {
                        let mut paint = Paint::default();
                        paint.set_color(acc);
                        paint.anti_alias = true;
                        canvas.fill_path(&p, &paint, FillRule::Winding, Transform::identity(), None);
                    }
                }
                x += step;
            }
        }
        "rainbow" => {
            let stops = vec![
                GradientStop::new(0.0, rgba(0xff, 0x4d, 0x6d, 1.0)),
                GradientStop::new(0.25, rgba(0xff, 0xd1, 0x66, 1.0)),
                GradientStop::new(0.5, rgba(0x8a, 0xc9, 0x26, 1.0)),
                GradientStop::new(0.75, rgba(0x4c, 0xc9, 0xf0, 1.0)),
                GradientStop::new(1.0, rgba(0xc7, 0x7d, 0xff, 1.0)),
            ];
            if let (Some(shader), Some(r)) = (
                LinearGradient::new(Point::from_xy(0.0, 0.0), Point::from_xy(width, height), stops, SpreadMode::Pad, Transform::identity()),
                Rect::from_xywh(edge * 0.5, edge * 0.5, width - edge, height - edge),
            ) {
                let paint = Paint { shader, anti_alias: true, ..Default::default() };
                let st = Stroke { width: (edge * 0.5).max(6.0), ..Default::default() };
                canvas.stroke_path(&PathBuilder::from_rect(r), &paint, &st, Transform::identity(), None);
            }
        }
        "web" => {
            let col = rgba(235, 240, 250, 0.72);
            let st = Stroke { width: (edge * 0.08).max(1.0), ..Default::default() };
            for (cx, cy, base) in [
                (0.0, 0.0, 0.0_f32),
                (width, 0.0, PI / 2.0),
                (0.0, height, -PI / 2.0),
                (width, height, PI),
            ] {
                for index in 0..6 {
                    let angle = base + (index as f32 / 5.0) * (PI / 2.0);
                    let mut pb = PathBuilder::new();
                    pb.move_to(cx, cy);
                    pb.line_to(cx + angle.cos() * edge * 4.0, cy + angle.sin() * edge * 4.0);
                    if let Some(p) = pb.finish() {
                        let mut paint = Paint::default();
                        paint.set_color(col);
                        paint.anti_alias = true;
                        canvas.stroke_path(&p, &paint, &st, Transform::identity(), None);
                    }
                }
                let mut radius = edge * 0.8;
                while radius < edge * 3.4 {
                    let mut pb = PathBuilder::new();
                    let segs = 10;
                    for k in 0..=segs {
                        let ang = base + (k as f32 / segs as f32) * (PI / 2.0);
                        let (px, py) = (cx + ang.cos() * radius, cy + ang.sin() * radius);
                        if k == 0 { pb.move_to(px, py); } else { pb.line_to(px, py); }
                    }
                    if let Some(p) = pb.finish() {
                        let mut paint = Paint::default();
                        paint.set_color(col);
                        paint.anti_alias = true;
                        canvas.stroke_path(&p, &paint, &st, Transform::identity(), None);
                    }
                    radius += edge * 0.62;
                }
            }
        }
        "coupon" => {
            if let Some(p) = rr_path(edge * 0.8, edge * 0.8, width - edge * 1.6, height - edge * 1.6, edge * 0.9) {
                let mut paint = Paint::default();
                paint.set_color(acc);
                paint.anti_alias = true;
                let st = Stroke {
                    width: (edge * 0.16).max(2.0),
                    dash: StrokeDash::new(vec![edge * 0.45, edge * 0.32], 0.0),
                    ..Default::default()
                };
                canvas.stroke_path(&p, &paint, &st, Transform::identity(), None);
            }
        }
        "comic" => {
            if let Some(r) = Rect::from_xywh(edge * 0.5, edge * 0.5, width - edge, height - edge) {
                let mut paint = Paint::default();
                paint.set_color(rgba(0x11, 0x11, 0x11, 1.0));
                paint.anti_alias = true;
                let st = Stroke { width: (edge * 0.38).max(5.0), ..Default::default() };
                canvas.stroke_path(&PathBuilder::from_rect(r), &paint, &st, Transform::identity(), None);
            }
            let mut pb = PathBuilder::new();
            pb.move_to(width - edge * 3.4, 0.0);
            pb.line_to(width, 0.0);
            pb.line_to(width, edge * 3.4);
            pb.close();
            if let Some(p) = pb.finish() {
                let mut paint = Paint::default();
                paint.set_color(acc);
                paint.anti_alias = true;
                canvas.fill_path(&p, &paint, FillRule::Winding, Transform::identity(), None);
            }
        }
        "badge" => {
            let radius = (16.0_f32).max(edge * 1.35);
            let (cx, cy) = (width - radius * 1.2, radius * 1.2);
            if let Some(p) = PathBuilder::from_circle(cx, cy, radius) {
                let mut paint = Paint::default();
                paint.set_color(acc);
                paint.anti_alias = true;
                canvas.fill_path(&p, &paint, FillRule::Winding, Transform::identity(), None);
            }
            let tv = serde_json::json!({
                "content": "NUEVO",
                "font": "Segoe UI",
                "weight": 800.0,
                "size": (radius * 0.55).max(10.0),
                "color": "#071016",
                "align": "center",
            });
            if let Some(pm) = raster::render_text(&tv, 1.0) {
                let (gw, gh) = (pm.width() as f32, pm.height() as f32);
                let paint = PixmapPaint { opacity: alpha, ..Default::default() };
                canvas.draw_pixmap(0, 0, pm.as_ref(), &paint, Transform::from_translate(cx - gw / 2.0, cy - gh / 2.0), None);
            }
        }
        "open" => {
            let stops = vec![
                GradientStop::new(0.0, Color::from_rgba8(255, 214, 153, a8(0.28))),
                GradientStop::new(0.45, Color::from_rgba8(255, 255, 255, 0)),
                GradientStop::new(1.0, Color::from_rgba8(120, 200, 255, a8(0.22))),
            ];
            if let (Some(shader), Some(r)) = (
                LinearGradient::new(Point::from_xy(0.0, 0.0), Point::from_xy(0.0, height), stops, SpreadMode::Pad, Transform::identity()),
                Rect::from_xywh(0.0, 0.0, width, height),
            ) {
                let paint = Paint { shader, ..Default::default() };
                canvas.fill_rect(r, &paint, Transform::identity(), None);
            }
        }
        _ => {}
    }
}

// ── Superposiciones de efectos (`drawOverlayFx` de renderer.js) ──
/// Ficha de superposición: estilo (`overlay`) + color de acento (`accent`).
/// Copiada de las ~90 entradas con `overlay:` de `src/filtros/library.js`
/// (el acento por defecto es `#ffffff`, como `spec.accent || '#ffffff'`).
struct OverlaySpec {
    overlay: &'static str,
    accent: &'static str,
}

/// Tabla de superposiciones por id de efecto. Sin inventar: estilos y acentos
/// provienen del catálogo original (`EFFECTS` de library.js).
fn overlay_spec(clip: &Clip) -> Option<OverlaySpec> {
    let id = clip.extra.get("effect").and_then(Value::as_str)?;
    let o = |overlay, accent| Some(OverlaySpec { overlay, accent });
    match id {
        "tnd_chalk" => o("chalk", "#f4f1e8"),
        "tnd_sepia" => o("disco", "#ffffff"),
        "tnd_action" => o("action", "#ffcf3f"),
        "tnd_beat" => o("pulse", "#ff4d88"),
        "tnd_horror" => o("glitch", "#ff2f4f"),
        "tnd_mosaic" => o("mosaic", "#ffffff"),
        "tnd_doodle" => o("doodle", "#ffffff"),
        "tnd_chroma" => o("chroma", "#ffffff"),
        "tnd_print" => o("print", "#f3e9d2"),
        "tnd_retroShake" => o("grain", "#ffffff"),
        "pat_beak" => o("gold", "#ffffff"),
        "pat_puddle" => o("ripple", "#58d7ff"),
        "pat_flap" => o("breeze", "#ffffff"),
        "hal_fog" => o("fog", "#ffffff"),
        "hal_lightning" => o("lightning", "#ffffff"),
        "hal_eyes" => o("eyes", "#ffffff"),
        "hal_blood" => o("blood", "#a40012"),
        "hal_witch" => o("magic", "#ffffff"),
        "esp_dreamy" => o("dreamy", "#ffffff"),
        "esp_rainbow" => o("rainbow", "#ffffff"),
        "esp_magic" => o("magic", "#ffffff"),
        "blk_sale" => o("sale", "#ffd400"),
        "blk_neon" => o("neon", "#ffd400"),
        "blk_flash" => o("strobe", "#ffffff"),
        "blk_coupon" => o("sale", "#ffd400"),
        "cls_vintage" => o("vignette", "#ffffff"),
        "cls_film" => o("grain", "#ffffff"),
        "cls_flash" => o("strobe", "#ffffff"),
        "cut_neon" => o("neonOutline", "#ff4de1"),
        "cut_silhouette" => o("silhouette", "#ffffff"),
        "cut_outline" => o("outline", "#ffffff"),
        "cut_chalk" => o("chalk", "#f2ede1"),
        "cut_double" => o("doubleBorder", "#ffd166"),
        "cut_shadow" => o("shadow", "#ffffff"),
        "cut_comic" => o("halftone", "#ffffff"),
        "cut_watercolor" => o("watercolor", "#ffffff"),
        "new_aura" => o("aura", "#8ffcff"),
        "new_glass" => o("glass", "#ffffff"),
        "new_holo" => o("hologram", "#ffffff"),
        "new_prism" => o("prism", "#ffffff"),
        "new_pulse" => o("pulse", "#7defff"),
        "new_badge" => o("sale", "#00e5d0"),
        "bmb_flash" => o("strobe", "#ffffff"),
        "bmb_boom" => o("boom", "#ffd166"),
        "bmb_fire" => o("fire", "#ffffff"),
        "bmb_impact" => o("impact", "#ffd166"),
        "int_countdown" => o("countdown", "#ffffff"),
        "int_curtain" => o("curtain", "#ffffff"),
        "int_wipe" => o("wipe", "#ffffff"),
        "int_flash" => o("strobe", "#ffffff"),
        "int_glitch" => o("glitch", "#ffffff"),
        "int_close" => o("close", "#ffffff"),
        "int_open" => o("flare", "#ffffff"),
        "fst_disco" => o("disco", "#ffffff"),
        "fst_lights" => o("party", "#ffffff"),
        "edt_vhs" => o("vhs", "#ffffff"),
        "edt_glitch" => o("glitch", "#ffffff"),
        "scanlines" => o("scanlines", "#ffffff"),
        "edt_pixel" => o("pixel", "#ffffff"),
        "edt_vcr" => o("vcr", "#ffffff"),
        "edt_tracking" => o("tracking", "#ffffff"),
        "edt_datamosh" => o("datamosh", "#ffffff"),
        "d3_holo" => o("hologram", "#ffffff"),
        "d3_depth" => o("depth", "#ffffff"),
        "d3_extrude" => o("extrude", "#ffffff"),
        "glow" => o("glow", "#7defff"),
        "neon" => o("neon", "#ff4de1"),
        "luz_leak" => o("lightLeak", "#ffffff"),
        "luz_flare" => o("flare", "#ffffff"),
        "luz_spot" => o("spotlight", "#ffffff"),
        "strobe" => o("strobe", "#ffffff"),
        "luz_bokeh" => o("bokeh", "#ffffff"),
        "luz_rays" => o("rays", "#ffffff"),
        "luz_aurora" => o("aurora", "#ffffff"),
        "luz_flash" => o("flash", "#ffffff"),
        "oldFilm" => o("oldFilm", "#ffffff"),
        "vignette" => o("vignette", "#ffffff"),
        "grain" => o("grain", "#ffffff"),
        "ret_scan" => o("scanlines", "#ffffff"),
        "ret_vhs" => o("vhs", "#ffffff"),
        "ret_projector" => o("projector", "#ffffff"),
        "chromatic" => o("chroma", "#ffffff"),
        "body_aura" => o("aura", "#8ffcff"),
        "body_slim" => o("slim", "#ffffff"),
        "body_glow" => o("bodyGlow", "#ffffff"),
        "body_comic" => o("halftone", "#ffffff"),
        "body_outline" => o("outline", "#ffffff"),
        "body_thermal" => o("thermal", "#ffffff"),
        "body_xray" => o("xray", "#ffffff"),
        "body_neon" => o("neonOutline", "#00e5ff"),
        _ => None,
    }
}

/// Grano de ruido 160×160 (`noiseTile` de renderer.js): gris aleatorio, alfa 26.
/// Se genera una vez con `hash01` (determinista) en vez de `Math.random`, ya que
/// el original lo cachea en un único mosaico fijo que se reutiliza siempre.
fn noise_tile() -> &'static Pixmap {
    static TILE: std::sync::OnceLock<Pixmap> = std::sync::OnceLock::new();
    TILE.get_or_init(|| {
        let mut pm = Pixmap::new(160, 160).expect("mosaico de ruido 160x160");
        for (index, px) in pm.pixels_mut().iter_mut().enumerate() {
            let value = (hash01(index as f64 * 1.37 + 0.5) * 255.0) as u8;
            // Alfa 26; los canales deben venir premultiplicados (<= alfa).
            let pre = ((value as u16 * 26) / 255) as u8;
            if let Some(c) = resvg::tiny_skia::PremultipliedColorU8::from_rgba(pre, pre, pre, 26) {
                *px = c;
            }
        }
        pm
    })
}

/// `ctx.createLinearGradient` → shader lineal de tiny_skia (bordes `Pad`).
fn linear_shader(x0: f32, y0: f32, x1: f32, y1: f32, stops: &[(f32, Color)]) -> Option<Shader<'static>> {
    let gs: Vec<GradientStop> = stops.iter().map(|(p, c)| GradientStop::new(*p, *c)).collect();
    LinearGradient::new(
        Point::from_xy(x0, y0),
        Point::from_xy(x1, y1),
        gs,
        SpreadMode::Pad,
        Transform::identity(),
    )
}

/// `ctx.createRadialGradient(cx,cy,r0, cx,cy,r1)` → shader radial de dos
/// círculos concéntricos. tiny_skia 0.12 admite el radio interior `r0` de forma
/// nativa (igual que canvas), así que se pasa directamente sin remapear paradas.
fn radial_shader(cx: f32, cy: f32, r0: f32, r1: f32, stops: &[(f32, Color)]) -> Option<Shader<'static>> {
    if r1 <= 0.0 {
        return None;
    }
    let gs: Vec<GradientStop> = stops.iter().map(|(p, c)| GradientStop::new(*p, *c)).collect();
    RadialGradient::new(
        Point::from_xy(cx, cy),
        r0.max(0.0),
        Point::from_xy(cx, cy),
        r1,
        gs,
        SpreadMode::Pad,
        Transform::identity(),
    )
}

fn paint_color(c: Color, blend: Option<BlendMode>) -> Paint<'static> {
    let mut p = Paint::default();
    p.set_color(c);
    p.anti_alias = true;
    if let Some(b) = blend {
        p.blend_mode = b;
    }
    p
}

fn paint_shader(sh: Shader<'static>, blend: Option<BlendMode>) -> Paint<'static> {
    let mut p = Paint::default();
    p.shader = sh;
    p.anti_alias = true;
    if let Some(b) = blend {
        p.blend_mode = b;
    }
    p
}

fn fill_shader(canvas: &mut Pixmap, rect: Rect, sh: Shader<'static>, blend: Option<BlendMode>) {
    let paint = paint_shader(sh, blend);
    canvas.fill_rect(rect, &paint, Transform::identity(), None);
}

fn fill_path_shader(canvas: &mut Pixmap, path: &Path, sh: Shader<'static>, blend: Option<BlendMode>) {
    let paint = paint_shader(sh, blend);
    canvas.fill_path(path, &paint, FillRule::Winding, Transform::identity(), None);
}

fn fill_path_ex(canvas: &mut Pixmap, path: &Path, color: Color, tr: Transform, blend: Option<BlendMode>) {
    let paint = paint_color(color, blend);
    canvas.fill_path(path, &paint, FillRule::Winding, tr, None);
}

fn stroke_ex(canvas: &mut Pixmap, path: &Path, color: Color, st: &Stroke, tr: Transform, blend: Option<BlendMode>) {
    let paint = paint_color(color, blend);
    canvas.stroke_path(path, &paint, st, tr, None);
}

/// Dibuja texto (`ctx.fillText`) construyendo un clip de texto sintético para
/// `raster::render_text`. `base` posiciona el ancla; `cx`/`cy` centran el pixmap
/// horizontal/verticalmente (textAlign center / textBaseline middle del original).
#[allow(clippy::too_many_arguments)]
fn blit_text(
    canvas: &mut Pixmap,
    text: &str,
    font: &str,
    weight: f64,
    px: f32,
    color: &str,
    base: Transform,
    cx: bool,
    cy: bool,
    opacity: f32,
) {
    if text.is_empty() || opacity <= 0.0 {
        return;
    }
    let tv = serde_json::json!({
        "content": text,
        "font": font,
        "weight": weight,
        "size": px.max(6.0),
        "color": color,
        "align": "center",
    });
    if let Some(pm) = raster::render_text(&tv, 1.0) {
        let (gw, gh) = (pm.width() as f32, pm.height() as f32);
        let ax = if cx { gw / 2.0 } else { 0.0 };
        let ay = if cy { gh / 2.0 } else { 0.0 };
        let tr = base.pre_translate(-ax, -ay);
        let paint = PixmapPaint {
            opacity: opacity.clamp(0.0, 1.0),
            ..Default::default()
        };
        canvas.draw_pixmap(0, 0, pm.as_ref(), &paint, tr, None);
    }
}

/// Pinta la superposición de un efecto en espacio de pantalla, portando
/// `drawOverlayFx` de renderer.js. Las ramas son `if` independientes (no
/// `else if`) igual que el original: un mismo overlay puede disparar varias.
/// `alpha` es la opacidad compuesta; los literales `rgba()` se resuelven aquí.
fn draw_overlay(canvas: &mut Pixmap, spec: &OverlaySpec, local: f64, width: f32, height: f32, alpha: f32) {
    let overlay = spec.overlay;
    let accent = spec.accent;
    let unit = width.min(height);
    let (w, h) = (width, height);
    let lf = local;
    let a = alpha;
    // rgba() del original con el alfa ya multiplicado por la opacidad compuesta.
    let rgba = |r: u8, g: u8, b: u8, aa: f32| {
        Color::from_rgba8(r, g, b, ((aa * a).clamp(0.0, 1.0) * 255.0).round() as u8)
    };
    let acc_a = |f: f32| raster::parse_color(accent, (f * a).clamp(0.0, 1.0));
    let full = Rect::from_xywh(0.0, 0.0, w, h);

    // Viñeta / silueta / silueta fina / sombra: radial oscuro + barras laterales.
    if matches!(overlay, "vignette" | "silhouette" | "slim" | "shadow") {
        let outer = w.max(h) * 0.72;
        let edge = if overlay == "slim" { 0.42 } else { 0.62 };
        if let (Some(sh), Some(r)) = (
            radial_shader(w / 2.0, h / 2.0, unit * 0.2, outer, &[(0.0, rgba(0, 0, 0, 0.0)), (1.0, rgba(0, 0, 0, edge))]),
            full,
        ) {
            fill_shader(canvas, r, sh, None);
        }
        if overlay == "slim" {
            let c = rgba(0, 0, 0, 0.3);
            fill_rect(canvas, 0.0, 0.0, w * 0.09, h, c);
            fill_rect(canvas, w * 0.91, 0.0, w * 0.09, h, c);
        }
    }

    // Grano: mosaico de ruido estirado (globalAlpha alpha*0.72).
    if matches!(overlay, "grain" | "oldFilm" | "projector" | "chalk" | "print" | "watercolor") {
        let tile = noise_tile();
        let paint = PixmapPaint {
            opacity: (a * 0.72).clamp(0.0, 1.0),
            quality: resvg::tiny_skia::FilterQuality::Bilinear,
            ..Default::default()
        };
        canvas.draw_pixmap(0, 0, tile.as_ref(), &paint, Transform::from_scale(w / 160.0, h / 160.0), None);
    }

    // Líneas de barrido horizontales.
    if matches!(overlay, "scanlines" | "vhs" | "vcr" | "pixel" | "thermal" | "xray") {
        let step = (2.0_f32).max((h / 260.0).round());
        let c = rgba(0, 0, 0, 0.28 * 0.5);
        let mut y = 0.0;
        while y < h {
            fill_rect(canvas, 0.0, y, w, step, c);
            y += step * 2.0;
        }
    }

    // Arañazos verticales del cine viejo / proyector.
    if matches!(overlay, "oldFilm" | "projector") {
        let ga = (0.32 + (lf * 9.0).sin().abs() * 0.12) as f32;
        let col = rgba(237, 219, 178, 0.7 * ga);
        let st = Stroke { width: (1.0_f32).max(w / 900.0), ..Default::default() };
        for index in 0..3 {
            let x = (((lf * 13.0 + index as f64 * 31.7) % 1.0) * w as f64) as f32;
            let mut pb = PathBuilder::new();
            pb.move_to(x, 0.0);
            pb.line_to(x + ((lf * 2.0 + index as f64).sin() * 8.0) as f32, h);
            if let Some(p) = pb.finish() {
                stroke_ex(canvas, &p, col, &st, Transform::identity(), None);
            }
        }
        if overlay == "projector" {
            fill_rect(canvas, 0.0, 0.0, w, h, rgba(25, 15, 6, 0.18));
        }
    }

    // Destello estroboscópico / flash: pulso blanco a pantalla completa.
    if matches!(overlay, "strobe" | "flash") {
        let pulse = if overlay == "flash" {
            ((lf * 8.0).sin() - 0.72).max(0.0) * 2.8
        } else if (lf * 18.0).sin() > 0.55 {
            0.1
        } else {
            0.0
        };
        if pulse > 0.0 {
            if let Some(r) = full {
                let c = rgba(255, 255, 255, pulse.min(0.3) as f32);
                fill_shader(canvas, r, Shader::SolidColor(c), None);
            }
        }
    }

    // Resplandor / aura / niebla: baño de color + radial de acento.
    if matches!(overlay, "glow" | "dreamy" | "aura" | "bodyGlow" | "magic" | "fog") {
        let amount = if overlay == "dreamy" { 0.035 } else if overlay == "fog" { 0.12 } else { 0.09 };
        if matches!(overlay, "glow" | "aura" | "bodyGlow") {
            fill_rect(canvas, 0.0, 0.0, w, h, acc_a(amount));
        } else {
            fill_rect(canvas, 0.0, 0.0, w, h, rgba(255, 255, 255, amount));
        }
        if matches!(overlay, "aura" | "bodyGlow" | "magic") {
            // `${accent}88` = acento con alfa 0x88 (0.533), × globalAlpha 0.75.
            let inner = raster::parse_color(accent, (0.533 * 0.75 * a).clamp(0.0, 1.0));
            if let (Some(sh), Some(r)) = (
                radial_shader(w / 2.0, h / 2.0, unit * 0.08, unit * 0.58, &[(0.0, inner), (1.0, rgba(0, 0, 0, 0.0))]),
                full,
            ) {
                fill_shader(canvas, r, sh, None);
            }
        }
    }

    // Cromático: bordes rojo/cian (screen).
    if overlay == "chroma" {
        if let (Some(sh), Some(r)) = (
            linear_shader(0.0, 0.0, w * 0.25, 0.0, &[(0.0, rgba(255, 0, 60, 0.28)), (1.0, rgba(255, 0, 60, 0.0))]),
            Rect::from_xywh(0.0, 0.0, w * 0.25, h),
        ) {
            fill_shader(canvas, r, sh, Some(BlendMode::Screen));
        }
        if let (Some(sh), Some(r)) = (
            linear_shader(w, 0.0, w * 0.75, 0.0, &[(0.0, rgba(0, 240, 255, 0.28)), (1.0, rgba(0, 240, 255, 0.0))]),
            Rect::from_xywh(w * 0.75, 0.0, w * 0.25, h),
        ) {
            fill_shader(canvas, r, sh, Some(BlendMode::Screen));
        }
    }

    // VHS / VCR / tracking: banda móvil + sello PLAY (+REC en VCR).
    if matches!(overlay, "vhs" | "vcr" | "tracking") {
        let band_y = (((lf * 0.35) % 1.2) * h as f64 - h as f64 * 0.1) as f32;
        if let (Some(sh), Some(r)) = (
            linear_shader(
                0.0,
                band_y - h * 0.08,
                0.0,
                band_y + h * 0.08,
                &[(0.0, rgba(255, 255, 255, 0.0)), (0.5, rgba(255, 255, 255, 0.16)), (1.0, rgba(255, 255, 255, 0.0))],
            ),
            Rect::from_xywh(0.0, band_y - h * 0.08, w, h * 0.16),
        ) {
            fill_shader(canvas, r, sh, None);
        }
        let px = (9.0_f32).max(h * 0.032);
        let stamp = format!(
            "PLAY \u{25b6}  {:02}:{:02}:{:02}",
            (lf / 60.0).floor() as i64,
            (lf % 60.0).floor() as i64,
            ((lf * 30.0) % 30.0).floor() as i64
        );
        blit_text(canvas, &stamp, "Consolas", 600.0, px, "#ffffff", Transform::from_translate(w * 0.035, h * 0.04), false, false, 0.85 * a);
        if overlay == "vcr" {
            if let Some(p) = PathBuilder::from_circle(w * 0.05, h * 0.9, (3.0_f32).max(h * 0.012)) {
                fill_path_ex(canvas, &p, rgba(0xff, 0x3b, 0x3b, 1.0), Transform::identity(), None);
            }
            blit_text(canvas, "REC", "Consolas", 600.0, px, "#ffffff", Transform::from_translate(w * 0.065, h * 0.88), false, false, 0.8 * a);
        }
    }

    // Glitch / datamosh: barras de color desplazadas + franjas negras.
    if matches!(overlay, "glitch" | "datamosh" | "tracking") {
        for index in 0..10 {
            let seed = hash01(index as f64 + (lf * 16.0).floor());
            let y = (seed * h as f64) as f32;
            let bh = (2.0_f32).max((hash01(index as f64 + 3.3) * h as f64 * 0.045) as f32);
            let x = (hash01(index as f64 + 8.8) * w as f64 * 0.28 - w as f64 * 0.14) as f32;
            let aa = (0.1 + seed * 0.22) as f32;
            let c = match index % 3 {
                0 => rgba(0xff, 0x2f, 0x5e, aa),
                1 => rgba(0x28, 0xe7, 0xff, aa),
                _ => rgba(0xff, 0xff, 0xff, aa),
            };
            fill_rect(canvas, x, y, (w as f64 * (0.35 + seed * 0.5)) as f32, bh, c);
        }
        for index in 0..4 {
            let y = (hash01(index as f64 + (lf * 8.0).floor() + 2.0) * h as f64) as f32;
            fill_rect(canvas, 0.0, y, w, (1.0_f32).max(h * 0.004), rgba(0, 0, 0, 0.55 * 0.35));
        }
    }

    // Píxel: celdas aleatorias.
    if overlay == "pixel" {
        let (cols, rows) = (24, 14);
        let c = rgba(255, 255, 255, 0.5 * 0.18);
        for x in 0..cols {
            for y in 0..rows {
                if hash01((x * 17 + y * 31) as f64) > 0.82 {
                    fill_rect(
                        canvas,
                        (x as f32 / cols as f32) * w,
                        (y as f32 / rows as f32) * h,
                        w / cols as f32 * 0.8,
                        h / rows as f32 * 0.8,
                        c,
                    );
                }
            }
        }
    }

    // Fuga de luz (screen).
    if overlay == "lightLeak" {
        let x = w * (0.75 + ((lf * 0.6).sin() as f32) * 0.18);
        if let (Some(sh), Some(r)) = (
            radial_shader(
                x,
                h * 0.08,
                0.0,
                unit * 0.95,
                &[(0.0, rgba(255, 213, 128, 0.65)), (0.35, rgba(255, 114, 94, 0.32)), (1.0, rgba(255, 60, 90, 0.0))],
            ),
            full,
        ) {
            fill_shader(canvas, r, sh, Some(BlendMode::Screen));
        }
    }

    // Destello de lente (screen) + rayos radiales.
    if overlay == "flare" {
        let x = w * (0.68 + ((lf * 0.4).sin() as f32) * 0.1);
        let y = h * (0.28 + ((lf * 0.5).cos() as f32) * 0.08);
        if let (Some(sh), Some(r)) = (
            radial_shader(
                x,
                y,
                0.0,
                unit * 0.55,
                &[(0.0, rgba(255, 255, 255, 0.75)), (0.15, rgba(255, 238, 180, 0.35)), (1.0, rgba(255, 220, 140, 0.0))],
            ),
            full,
        ) {
            fill_shader(canvas, r, sh, Some(BlendMode::Screen));
        }
        let col = rgba(255, 245, 210, 0.6);
        let st = Stroke { width: (1.0_f32).max(unit * 0.003), ..Default::default() };
        for index in 0..6 {
            let angle = (index as f64 / 6.0) * std::f64::consts::TAU + lf * 0.1;
            let mut pb = PathBuilder::new();
            pb.move_to(x + (angle.cos() as f32) * unit * 0.08, y + (angle.sin() as f32) * unit * 0.08);
            pb.line_to(x + (angle.cos() as f32) * unit * 0.46, y + (angle.sin() as f32) * unit * 0.46);
            if let Some(p) = pb.finish() {
                stroke_ex(canvas, &p, col, &st, Transform::identity(), Some(BlendMode::Screen));
            }
        }
    }

    // Foco: oscurecimiento radial móvil.
    if overlay == "spotlight" {
        let x = w * (0.5 + (lf * 0.55).sin() as f32 * 0.26);
        let y = h * (0.5 + (lf * 0.43).cos() as f32 * 0.2);
        if let (Some(sh), Some(r)) = (
            radial_shader(
                x,
                y,
                unit * 0.05,
                unit * 0.62,
                &[(0.0, rgba(0, 0, 0, 0.0)), (0.55, rgba(0, 0, 0, 0.25)), (1.0, rgba(0, 0, 0, 0.78))],
            ),
            full,
        ) {
            fill_shader(canvas, r, sh, None);
        }
    }

    // Bokeh: círculos suaves (screen).
    if overlay == "bokeh" {
        for index in 0..22 {
            let seed = hash01(index as f64 + 3.0);
            let x = (((seed * w as f64 + (lf * 0.3 + index as f64).sin() * 30.0) % w as f64 + w as f64) % w as f64) as f32;
            let y = (((hash01(index as f64 + 9.0) * h as f64 + (lf * 0.24 + index as f64).cos() * 24.0) % h as f64 + h as f64) % h as f64) as f32;
            let rr = unit * (0.015 + hash01(index as f64 + 14.0) as f32 * 0.05);
            let aa = 0.08 + hash01(index as f64 + 21.0) as f32 * 0.16;
            let c = if index % 2 == 1 { rgba(255, 235, 180, aa) } else { rgba(180, 235, 255, aa) };
            if let Some(p) = PathBuilder::from_circle(x, y, rr) {
                fill_path_ex(canvas, &p, c, Transform::identity(), Some(BlendMode::Screen));
            }
        }
    }

    // Rayos de sol: triángulos rotados desde arriba (screen).
    if overlay == "rays" {
        let base = Transform::from_translate(w * 0.5, -h * 0.12).pre_rotate(((lf * 0.25).sin() * 0.12).to_degrees() as f32);
        let col = rgba(255, 238, 180, 0.16);
        for index in 0..9 {
            let angle = (-std::f64::consts::FRAC_PI_2) + (index as f64 - 4.0) * 0.17;
            let spread = 0.035;
            let mut pb = PathBuilder::new();
            pb.move_to(0.0, 0.0);
            pb.line_to(((angle - spread).cos() * h as f64 * 1.25) as f32, ((angle - spread).sin() * h as f64 * 1.25) as f32);
            pb.line_to(((angle + spread).cos() * h as f64 * 1.25) as f32, ((angle + spread).sin() * h as f64 * 1.25) as f32);
            pb.close();
            if let Some(p) = pb.finish() {
                fill_path_ex(canvas, &p, col, base, Some(BlendMode::Screen));
            }
        }
    }

    // Aurora: bandas onduladas (screen).
    if overlay == "aurora" {
        for band in 0..3i32 {
            let r0 = (60 + band * 40) as u8;
            let g0 = (220 - band * 30) as u8;
            let b0 = (200 + band * 20) as u8;
            let r1 = (80 + band * 30) as u8;
            let g1 = (230 - band * 20) as u8;
            let b1 = (180 + band * 30) as u8;
            let a1 = 0.18 - band as f32 * 0.04;
            let stops = [(0.0, rgba(r0, g0, b0, 0.0)), (0.5, rgba(r1, g1, b1, a1)), (1.0, rgba(80, 180, 255, 0.0))];
            let base_y = h * (0.28 + band as f32 * 0.09);
            let mut pb = PathBuilder::new();
            pb.move_to(0.0, base_y);
            let mut x = 0.0;
            let stepx = w / 12.0;
            while x <= w {
                let yy = base_y + ((x / w * 6.0 + lf as f32 * 0.8 + band as f32).sin()) * h * 0.07;
                pb.line_to(x, yy);
                x += stepx;
            }
            pb.line_to(w, h);
            pb.line_to(0.0, h);
            pb.close();
            if let (Some(sh), Some(p)) = (linear_shader(0.0, h * 0.2, w, h * 0.6, &stops), pb.finish()) {
                fill_path_shader(canvas, &p, sh, Some(BlendMode::Screen));
            }
        }
    }

    // Matriz: glifos que caen por columnas.
    if overlay == "matrix" {
        let font_size = (10.0_f32).max(h * 0.032);
        let columns = 18usize;
        let glyphs: Vec<char> = "01\u{30a2}\u{30a4}\u{30a6}\u{30a8}\u{30aa}\u{30ab}\u{30ad}\u{30af}\u{30b1}\u{30b3}\u{30b5}\u{30b7}\u{30b9}\u{30bb}\u{30bd}".chars().collect();
        for column in 0..columns {
            let x = (column as f32 + 0.5) * (w / columns as f32);
            let offset = ((lf * (0.35 + hash01(column as f64 + 2.0) * 0.3) + hash01(column as f64 + 5.0)) % 1.0) as f32;
            for line in 0..9 {
                let y = ((offset + line as f32 / 9.0) % 1.0) * h;
                let aa = 0.12 + (line as f32 / 9.0) * 0.3;
                let color = if line == 0 { "#c8ffd4" } else { "#3dff7a" };
                let ch = glyphs[(column + line) % glyphs.len()].to_string();
                blit_text(canvas, &ch, "Consolas", 600.0, font_size, color, Transform::from_translate(x, y), true, false, aa * a);
            }
        }
    }

    // Semitono / cómic / boom: retícula de puntos (+estrella y BOOM! en boom).
    if matches!(overlay, "halftone" | "comic" | "boom") {
        let step = (5.0_f32).max(unit * 0.018);
        let dot = rgba(0, 0, 0, 0.16);
        let mut y = step;
        while y < h {
            let mut x = step;
            while x < w {
                if let Some(p) = PathBuilder::from_circle(x, y, step * 0.18) {
                    fill_path_ex(canvas, &p, dot, Transform::identity(), None);
                }
                x += step * 1.5;
            }
            y += step * 1.5;
        }
        if overlay == "boom" {
            let tr = Transform::from_translate(w * 0.62, h * 0.36).pre_rotate((-0.12_f32).to_degrees());
            let mut pb = PathBuilder::new();
            for index in 0..14 {
                let angle = (index as f64 / 14.0) * std::f64::consts::TAU;
                let radius = unit as f64 * (if index % 2 == 1 { 0.12 } else { 0.19 });
                let (px, py) = ((angle.cos() * radius) as f32, (angle.sin() * radius) as f32);
                if index == 0 {
                    pb.move_to(px, py);
                } else {
                    pb.line_to(px, py);
                }
            }
            pb.close();
            if let Some(p) = pb.finish() {
                fill_path_ex(canvas, &p, rgba(0xff, 0xd1, 0x66, 1.0), tr, None);
                let st = Stroke { width: (3.0_f32).max(unit * 0.008), ..Default::default() };
                stroke_ex(canvas, &p, rgba(0x11, 0x11, 0x11, 1.0), &st, tr, None);
            }
            blit_text(canvas, "BOOM!", "Arial", 900.0, (14.0_f32).max(unit * 0.055), "#111111", tr, true, true, a);
        }
    }

    // Térmico: degradado de temperatura (screen).
    if overlay == "thermal" {
        if let (Some(sh), Some(r)) = (
            linear_shader(0.0, h, w, 0.0, &[(0.0, rgba(20, 20, 255, 0.42)), (0.5, rgba(255, 190, 0, 0.28)), (1.0, rgba(255, 20, 20, 0.38))]),
            full,
        ) {
            fill_shader(canvas, r, sh, Some(BlendMode::Screen));
        }
    }

    // Rayos X: baño azulado (screen).
    if overlay == "xray" {
        if let Some(r) = full {
            fill_shader(canvas, r, Shader::SolidColor(rgba(110, 210, 255, 0.22)), Some(BlendMode::Screen));
        }
    }

    // Neón / contorno / doble borde / extrusión / profundidad: borde redondeado.
    if matches!(overlay, "neon" | "neonOutline" | "outline" | "doubleBorder" | "extrude" | "depth") {
        let stroke_col = if overlay == "neon" { acc_a(1.0) } else { rgba(255, 255, 255, 0.8) };
        let lw = (2.0_f32).max(unit * 0.006);
        let st = Stroke { width: lw, ..Default::default() };
        if let Some(p) = rr_path(unit * 0.04, unit * 0.04, w - unit * 0.08, h - unit * 0.08, unit * 0.03) {
            if matches!(overlay, "neon" | "neonOutline") {
                // `shadowBlur` aproximado: trazo de acento ancho y tenue debajo
                // (tiny_skia no tiene desenfoque de sombra).
                let glow = raster::parse_color(accent, (0.5 * a).clamp(0.0, 1.0));
                let gs = Stroke { width: lw + unit * 0.035 * 2.0, ..Default::default() };
                stroke_ex(canvas, &p, glow, &gs, Transform::identity(), None);
            }
            stroke_ex(canvas, &p, stroke_col, &st, Transform::identity(), None);
        }
        if overlay == "doubleBorder" {
            if let Some(p2) = rr_path(unit * 0.075, unit * 0.075, w - unit * 0.15, h - unit * 0.15, unit * 0.02) {
                stroke_ex(canvas, &p2, rgba(0xff, 0xd1, 0x66, 1.0), &st, Transform::identity(), None);
            }
        }
    }

    // Disco / fiesta: radiales de color girando (screen).
    if matches!(overlay, "disco" | "party") {
        let colors = [(255u8, 80u8, 140u8), (80, 220, 255), (255, 210, 80), (150, 100, 255)];
        for index in 0..6 {
            let angle = lf * 0.5 + (index as f64 / 6.0) * std::f64::consts::TAU;
            let x = w * 0.5 + (angle.cos() as f32) * w * 0.4;
            let y = h * 0.5 + (angle.sin() as f32) * h * 0.4;
            let (cr, cg, cb) = colors[(index % 4) as usize];
            if let (Some(sh), Some(r)) = (
                radial_shader(x, y, 0.0, unit * 0.35, &[(0.0, rgba(cr, cg, cb, 0.22)), (1.0, rgba(cr, cg, cb, 0.0))]),
                full,
            ) {
                fill_shader(canvas, r, sh, Some(BlendMode::Screen));
            }
        }
    }

    // Oferta: banner rotado + texto OFERTA.
    if overlay == "sale" {
        let tr = Transform::from_translate(w * 0.5, h * 0.18).pre_rotate((-0.06_f32).to_degrees());
        if let Some(r) = Rect::from_xywh(-w * 0.18, -h * 0.045, w * 0.36, h * 0.09) {
            let paint = paint_color(acc_a(1.0), None);
            canvas.fill_rect(r, &paint, tr, None);
        }
        blit_text(canvas, "OFERTA", "Arial", 900.0, (12.0_f32).max(h * 0.045), "#111111", tr, true, true, a);
    }

    // Barra de acción: líneas de velocidad.
    if overlay == "action" {
        let col = rgba(255, 255, 255, 0.55);
        let st = Stroke { width: (1.0_f32).max(unit * 0.003), ..Default::default() };
        for index in 0..18 {
            let y = (hash01(index as f64 + 2.0) * h as f64) as f32;
            let x = (((lf * 900.0 + index as f64 * 91.0) % (w as f64 + 300.0)) - 150.0) as f32;
            let mut pb = PathBuilder::new();
            pb.move_to(x, y);
            pb.line_to(x - w * 0.16, y);
            if let Some(p) = pb.finish() {
                stroke_ex(canvas, &p, col, &st, Transform::identity(), None);
            }
        }
    }

    // Mosaico cinético: tablero.
    if overlay == "mosaic" {
        let (cols, rows) = (14, 8);
        for x in 0..cols {
            let mut y = 0;
            while y < rows {
                let c = if (x + y) % 3 == 0 { rgba(255, 255, 255, 0.16) } else { rgba(0, 0, 0, 0.16) };
                fill_rect(canvas, (x as f32 / cols as f32) * w, (y as f32 / rows as f32) * h, w / cols as f32, h / rows as f32, c);
                y += 2;
            }
        }
    }

    // Garabato: líneas onduladas.
    if overlay == "doodle" {
        let col = rgba(255, 255, 255, 0.7);
        let st = Stroke { width: (1.5_f32).max(unit * 0.004), ..Default::default() };
        for index in 0..8 {
            let x = (hash01(index as f64 + 3.0) * w as f64) as f32;
            let y = (hash01(index as f64 + 11.0) * h as f64) as f32;
            let mut pb = PathBuilder::new();
            for step in 0..5 {
                let px = x + ((lf * 0.8 + step as f64 + index as f64).sin() * unit as f64 * 0.05) as f32 + step as f32 * unit * 0.02;
                let py = y + ((step as f64 * 2.0 + index as f64).cos() * unit as f64 * 0.03) as f32;
                if step == 0 {
                    pb.move_to(px, py);
                } else {
                    pb.line_to(px, py);
                }
            }
            if let Some(p) = pb.finish() {
                stroke_ex(canvas, &p, col, &st, Transform::identity(), None);
            }
        }
    }

    // Cuenta atrás: número grande centrado.
    if overlay == "countdown" {
        let value = (3.0 - (lf % 4.0)).ceil().max(1.0) as i64;
        blit_text(canvas, &value.to_string(), "Arial", 900.0, (36.0_f32).max(unit * 0.22), "#ffffff", Transform::from_translate(w / 2.0, h / 2.0), true, true, 0.9 * a);
    }

    // Cortinilla: degradados rojos laterales simétricos.
    if overlay == "curtain" {
        let amount = ((lf * 1.2).sin().max(0.0)) as f32 * w * 0.18;
        let bw = amount + unit * 0.08;
        if let (Some(sh), Some(r)) = (
            linear_shader(0.0, 0.0, bw, 0.0, &[(0.0, rgba(90, 0, 20, 0.9)), (1.0, rgba(90, 0, 20, 0.0))]),
            Rect::from_xywh(0.0, 0.0, bw, h),
        ) {
            fill_shader(canvas, r, sh, None);
        }
        if let (Some(sh), Some(r)) = (
            linear_shader(w, 0.0, w - bw, 0.0, &[(0.0, rgba(90, 0, 20, 0.9)), (1.0, rgba(90, 0, 20, 0.0))]),
            Rect::from_xywh(w - bw, 0.0, bw, h),
        ) {
            fill_shader(canvas, r, sh, None);
        }
    }

    // Barrido: banda blanca móvil.
    if overlay == "wipe" {
        let x = (((lf * 0.8) % 1.0) * w as f64) as f32;
        if let (Some(sh), Some(r)) = (
            linear_shader(
                x - w * 0.08,
                0.0,
                x + w * 0.08,
                0.0,
                &[(0.0, rgba(255, 255, 255, 0.0)), (0.5, rgba(255, 255, 255, 0.65)), (1.0, rgba(255, 255, 255, 0.0))],
            ),
            Rect::from_xywh(x - w * 0.08, 0.0, w * 0.16, h),
        ) {
            fill_shader(canvas, r, sh, None);
        }
    }

    // Cierre: barras negras arriba y abajo.
    if overlay == "close" {
        let bar = h * 0.08;
        let c = rgba(0, 0, 0, 1.0);
        fill_rect(canvas, 0.0, 0.0, w, bar, c);
        fill_rect(canvas, 0.0, h - bar, w, bar, c);
    }

    // Impacto: ráfaga radial de trazos.
    if overlay == "impact" {
        let col = acc_a(1.0);
        let st = Stroke { width: (2.0_f32).max(unit * 0.006), ..Default::default() };
        for index in 0..20 {
            let angle = (index as f64 / 20.0) * std::f64::consts::TAU;
            let r1 = unit as f64 * (0.2 + hash01(index as f64 + 2.0) * 0.22);
            let mut pb = PathBuilder::new();
            pb.move_to(w / 2.0 + (angle.cos() * unit as f64 * 0.12) as f32, h / 2.0 + (angle.sin() * unit as f64 * 0.12) as f32);
            pb.line_to(w / 2.0 + (angle.cos() * r1) as f32, h / 2.0 + (angle.sin() * r1) as f32);
            if let Some(p) = pb.finish() {
                stroke_ex(canvas, &p, col, &st, Transform::identity(), None);
            }
        }
    }

    // Fuego: degradado inferior cálido (screen).
    if overlay == "fire" {
        if let (Some(sh), Some(r)) = (
            linear_shader(0.0, h, 0.0, h * 0.55, &[(0.0, rgba(255, 90, 0, 0.5)), (1.0, rgba(255, 180, 0, 0.0))]),
            full,
        ) {
            fill_shader(canvas, r, sh, Some(BlendMode::Screen));
        }
    }

    // Niebla: nubes radiales móviles.
    if overlay == "fog" {
        for index in 0..5 {
            let x = (((lf * (12.0 + index as f64 * 3.0) + index as f64 * 300.0) % (w as f64 + 400.0)) - 200.0) as f32;
            let cy = h * (0.55 + index as f32 * 0.06);
            if let (Some(sh), Some(r)) = (
                radial_shader(x, cy, 0.0, unit * 0.32, &[(0.0, rgba(220, 230, 240, 0.14)), (1.0, rgba(220, 230, 240, 0.0))]),
                full,
            ) {
                fill_shader(canvas, r, sh, None);
            }
        }
    }

    // Relámpago: rayo ocasional.
    if overlay == "lightning" && (lf * 7.0).sin() > 0.86 {
        let col = rgba(235, 245, 255, 0.9);
        let st = Stroke { width: (2.0_f32).max(unit * 0.005), ..Default::default() };
        let mut x = w * 0.58;
        let mut pb = PathBuilder::new();
        pb.move_to(x, 0.0);
        let mut y = 0.0;
        while y < h {
            x += ((hash01(y as f64 + (lf * 10.0).floor()) - 0.5) * unit as f64 * 0.08) as f32;
            pb.line_to(x, y);
            y += h * 0.08;
        }
        if let Some(p) = pb.finish() {
            stroke_ex(canvas, &p, col, &st, Transform::identity(), None);
        }
    }

    // Ojos: oscuridad + ojos rojos brillantes (elipses).
    if overlay == "eyes" {
        fill_rect(canvas, 0.0, 0.0, w, h, rgba(0, 0, 0, 0.55));
        let glow = (lf * 1.4).sin().abs().max(0.25) as f32;
        let col = rgba(255, 60, 60, glow);
        for off in [-1.0f32, 1.0] {
            let (cx, cy) = (w / 2.0 + off * unit * 0.07, h * 0.46);
            let (rx, ry) = (unit * 0.022, unit * 0.014);
            if let Some(p) = PathBuilder::from_circle(0.0, 0.0, 1.0) {
                // Resplandor de sombra aproximado con una elipse mayor y tenue.
                fill_path_ex(canvas, &p, rgba(255, 59, 59, glow * 0.4), Transform::from_translate(cx, cy).pre_scale(rx * 1.8, ry * 1.8), None);
                fill_path_ex(canvas, &p, col, Transform::from_translate(cx, cy).pre_scale(rx, ry), None);
            }
        }
    }

    // Sangre: goteos verticales.
    if overlay == "blood" {
        let col = rgba(150, 0, 18, 0.55);
        for index in 0..9 {
            let x = (hash01(index as f64 + 4.0) * w as f64) as f32;
            let length = h * (0.08 + hash01(index as f64 + 8.0) as f32 * 0.3);
            let ww = unit * (0.006 + hash01(index as f64 + 12.0) as f32 * 0.018);
            let mut pb = PathBuilder::new();
            pb.move_to(x - ww, 0.0);
            pb.line_to(x + ww, 0.0);
            pb.line_to(x + ww * 0.6, length);
            let r = ww * 0.6;
            for k in 0..=8 {
                let ang = (k as f32 / 8.0) * std::f32::consts::PI;
                pb.line_to(x + ang.cos() * r, length + ang.sin() * r);
            }
            pb.close();
            if let Some(p) = pb.finish() {
                fill_path_ex(canvas, &p, col, Transform::identity(), None);
            }
        }
    }

    // Charco: anillos concéntricos.
    if overlay == "ripple" {
        let st = Stroke { width: (1.0_f32).max(unit * 0.003), ..Default::default() };
        for index in 0..5 {
            let p = ((lf * 0.35 + index as f64 / 5.0) % 1.0) as f32;
            let aa = (1.0 - p) * 0.55;
            if let Some(path) = PathBuilder::from_circle(w / 2.0, h / 2.0, p * unit * 0.7) {
                stroke_ex(canvas, &path, raster::parse_color(accent, (aa * a).clamp(0.0, 1.0)), &st, Transform::identity(), None);
            }
        }
    }

    // Aleteo: curvas de brisa.
    if overlay == "breeze" {
        let col = rgba(255, 255, 255, 0.5);
        let st = Stroke { width: (1.0_f32).max(unit * 0.003), ..Default::default() };
        for index in 0..7 {
            let y = (hash01(index as f64 + 6.0) * h as f64) as f32;
            let x = (((lf * 220.0 + index as f64 * 160.0) % (w as f64 + 300.0)) - 150.0) as f32;
            let mut pb = PathBuilder::new();
            pb.move_to(x, y);
            pb.cubic_to(x + 60.0, y - 30.0, x + 120.0, y + 30.0, x + 180.0, y);
            if let Some(p) = pb.finish() {
                stroke_ex(canvas, &p, col, &st, Transform::identity(), None);
            }
        }
    }

    // Pico dorado: baño diagonal.
    if overlay == "gold" {
        if let (Some(sh), Some(r)) = (
            linear_shader(0.0, 0.0, w, h, &[(0.0, rgba(255, 205, 110, 0.3)), (0.5, rgba(255, 240, 190, 0.12)), (1.0, rgba(180, 120, 20, 0.22))]),
            full,
        ) {
            fill_shader(canvas, r, sh, None);
        }
    }

    // Impresión efímera: papel + líneas onduladas.
    if overlay == "print" {
        fill_rect(canvas, 0.0, 0.0, w, h, rgba(245, 235, 214, 0.1));
        let col = rgba(60, 45, 30, 0.3);
        let st = Stroke { width: (1.0_f32).max(unit * 0.002), ..Default::default() };
        for index in 0..40 {
            let y = (index as f32 / 40.0) * h;
            let mut pb = PathBuilder::new();
            pb.move_to(0.0, y);
            pb.line_to(w, y + (hash01(index as f64 + 3.0) as f32 - 0.5) * 6.0);
            if let Some(p) = pb.finish() {
                stroke_ex(canvas, &p, col, &st, Transform::identity(), None);
            }
        }
    }

    // Cristal: vetas diagonales (screen).
    if overlay == "glass" {
        for index in 0..4 {
            let x = (((index as f64 * 0.3 + lf * 0.04) % 1.0) * w as f64) as f32;
            if let (Some(sh), Some(r)) = (
                linear_shader(
                    x - w * 0.04,
                    0.0,
                    x + w * 0.04,
                    h,
                    &[(0.0, rgba(255, 255, 255, 0.0)), (0.5, rgba(255, 255, 255, 0.16)), (1.0, rgba(255, 255, 255, 0.0))],
                ),
                Rect::from_xywh(x - w * 0.04, 0.0, w * 0.08, h),
            ) {
                fill_shader(canvas, r, sh, Some(BlendMode::Screen));
            }
        }
    }

    // Holograma: bandas de barrido coloreadas (screen).
    if overlay == "hologram" {
        for index in 0..7 {
            let y = (index as f32 / 7.0) * h + (((lf * 40.0) % (h as f64 / 7.0)) as f32);
            if let (Some(sh), Some(r)) = (
                linear_shader(0.0, y, w, y, &[(0.0, rgba(255, 80, 180, 0.12)), (0.5, rgba(80, 255, 230, 0.1)), (1.0, rgba(120, 120, 255, 0.12))]),
                Rect::from_xywh(0.0, y, w, h / 28.0),
            ) {
                fill_shader(canvas, r, sh, Some(BlendMode::Screen));
            }
        }
    }

    // Prisma: triángulos de color (screen).
    if overlay == "prism" {
        let cols = [(255u8, 60u8, 120u8), (255, 200, 60), (60, 255, 180), (60, 180, 255), (180, 80, 255)];
        for (index, (cr, cg, cb)) in cols.iter().enumerate() {
            let i = index as f32;
            let mut pb = PathBuilder::new();
            pb.move_to(w * (0.1 + i * 0.16), h);
            pb.line_to(w * (0.45 + i * 0.1), 0.0);
            pb.line_to(w * (0.62 + i * 0.08), 0.0);
            pb.close();
            if let Some(p) = pb.finish() {
                fill_path_ex(canvas, &p, rgba(*cr, *cg, *cb, 0.1), Transform::identity(), Some(BlendMode::Screen));
            }
        }
    }

    // Pulso: anillo que se expande.
    if overlay == "pulse" {
        let p = ((lf * 1.3) % 1.0) as f32;
        let aa = (1.0 - p) * 0.6;
        let st = Stroke { width: (2.0_f32).max(unit * 0.01), ..Default::default() };
        if let Some(path) = PathBuilder::from_circle(w / 2.0, h / 2.0, unit * (0.18 + p * 0.42)) {
            stroke_ex(canvas, &path, raster::parse_color(accent, (aa * a).clamp(0.0, 1.0)), &st, Transform::identity(), None);
        }
    }

    // Magia: resplandor radial violáceo (screen).
    if overlay == "magic" {
        if let (Some(sh), Some(r)) = (
            radial_shader(w / 2.0, h / 2.0, 0.0, unit * 0.7, &[(0.0, rgba(180, 120, 255, 0.14)), (1.0, rgba(80, 180, 255, 0.0))]),
            full,
        ) {
            fill_shader(canvas, r, sh, Some(BlendMode::Screen));
        }
    }
}

// ── Movimiento procedural de efectos (`effectSpec`/`effectMotion`) ──
/// Ficha de movimiento de un efecto: tipo (`move`) + `speed`/`amount`.
struct EffMove {
    kind: &'static str,
    speed: f64,
    amount: f64,
}

/// `effectSpec`: devuelve el movimiento del efecto del clip (sólo los ids del
/// catálogo con campo `move`; el resto son filtros/partículas sin geometría).
fn effect_move(clip: &Clip) -> Option<EffMove> {
    let id = clip.extra.get("effect").and_then(Value::as_str)?;
    if id == "none" || id.is_empty() {
        return None;
    }
    let m = |kind, speed, amount| Some(EffMove { kind, speed, amount });
    match id {
        "tnd_beat" | "new_pulse" => m("heartbeat", 1.0, 1.0),
        "tnd_retroShake" => m("shake", 1.0, 0.35),
        "shake" => m("shake", 1.0, 1.0),
        "bmb_quake" => m("shake", 1.0, 1.6),
        "pat_flap" => m("flap", 1.0, 1.0),
        "new_3d" | "d3_flip" => m("flip", 1.0, 1.0),
        "bmb_explosion" => m("impact", 0.9, 1.0),
        "bmb_impact" => m("impact", 1.0, 1.0),
        "int_zoom" => m("zoomIn", 1.0, 1.0),
        "zoomPulse" => m("zoom", 1.0, 1.0),
        "mov_wave" => m("wave", 1.0, 1.0),
        "mov_spin" => m("spin", 1.0, 1.0),
        "mov_drift" => m("drift", 1.0, 1.0),
        "mov_swing" => m("swing", 1.0, 1.0),
        "mov_float" => m("float", 1.0, 1.0),
        "mv2_pan" => m("pan", 1.0, 1.0),
        "mv2_tilt" => m("tilt", 1.0, 1.0),
        "mv2_roll" => m("roll", 1.0, 1.0),
        "mv2_orbit" => m("orbit", 1.0, 1.0),
        "mv2_jelly" => m("jelly", 1.0, 1.0),
        "mv2_rebound" => m("rebound", 1.0, 1.0),
        "mv2_parallax" | "d3_parallax" => m("parallax", 1.0, 1.0),
        "edt_datamosh" => m("glitch", 1.0, 1.0),
        "d3_cube" => m("cube", 1.0, 1.0),
        "d3_tilt" => m("tilt3d", 1.0, 1.0),
        _ => None,
    }
}

/// Geometría del movimiento: desplazamiento (px), rotación (grados) y escalas.
struct Motion {
    tx: f32,
    ty: f32,
    rot: f32,
    scale: f32,
    sx: f32,
    sy: f32,
}

impl Default for Motion {
    fn default() -> Self {
        Motion { tx: 0.0, ty: 0.0, rot: 0.0, scale: 1.0, sx: 1.0, sy: 1.0 }
    }
}

/// `effectMotion`: evalúa el movimiento procedural del efecto en `local`.
/// La rotación del original está en radianes; aquí se guarda en grados para
/// `Transform::pre_rotate`.
fn effect_motion(clip: &Clip, local: f64, w: f32, h: f32) -> Motion {
    let mut mo = Motion::default();
    let Some(e) = effect_move(clip) else { return mo };
    let (l, speed, amount) = (local, e.speed, e.amount);
    let (wf, hf) = (w as f64, h as f64);
    let deg = |rad: f64| (rad * 180.0 / std::f64::consts::PI) as f32;
    match e.kind {
        "zoom" => mo.scale = (1.0 + (l * 6.2 * speed).sin() * 0.06) as f32,
        "heartbeat" => {
            let beat = (l * 7.4 * speed).sin().max(0.0);
            mo.scale = (1.0 + beat * beat * beat * 0.1) as f32;
        }
        "shake" => {
            mo.tx = ((l * 43.0).sin() * wf * 0.006 * amount) as f32;
            mo.ty = ((l * 37.0).cos() * hf * 0.006 * amount) as f32;
        }
        "wave" => {
            mo.tx = ((l * 4.1 * speed).sin() * wf * 0.028) as f32;
            mo.ty = ((l * 4.6 * speed).cos() * hf * 0.018) as f32;
            mo.rot = deg((l * 4.1 * speed).sin() * 0.012);
        }
        "spin" => mo.rot = deg(l * 0.55 * speed),
        "drift" => {
            mo.tx = ((l * 2.1 * speed).sin() * wf * 0.03) as f32;
            mo.ty = ((l * 1.7 * speed).cos() * hf * 0.02) as f32;
        }
        "swing" => {
            mo.rot = deg((l * 3.2 * speed).sin() * 0.04);
            mo.tx = ((l * 3.2 * speed).sin() * wf * 0.01) as f32;
        }
        "float" => {
            mo.ty = ((l * 2.7 * speed).sin() * hf * 0.02) as f32;
            mo.tx = ((l * 2.1 * speed).cos() * wf * 0.01) as f32;
        }
        "pan" => mo.tx = ((l * 1.8 * speed).sin() * wf * 0.04) as f32,
        "tilt" => mo.ty = ((l * 1.8 * speed).sin() * hf * 0.04) as f32,
        "roll" => mo.rot = deg((l * 2.2 * speed).sin() * 0.055),
        "orbit" => {
            mo.tx = ((l * 3.1 * speed).sin() * wf * 0.022) as f32;
            mo.ty = ((l * 3.1 * speed).cos() * hf * 0.022) as f32;
        }
        "jelly" => {
            mo.sx = (1.0 + (l * 6.4 * speed).sin() * 0.035) as f32;
            mo.sy = (1.0 - (l * 6.4 * speed).sin() * 0.035) as f32;
        }
        "rebound" => mo.ty = (-(l * 5.2 * speed).sin().abs() * hf * 0.04) as f32,
        "parallax" => {
            mo.tx = ((l * 1.2 * speed).sin() * wf * 0.03) as f32;
            mo.ty = ((l * 1.2 * speed).cos() * hf * 0.016) as f32;
            mo.scale = 1.04;
        }
        "flip" => mo.sx = (l * 1.7 * speed).cos().abs().max(0.12) as f32,
        "cube" => {
            mo.sx = (l * 1.9 * speed).cos().abs().max(0.2) as f32;
            mo.tx = ((l * 1.9 * speed).sin() * wf * 0.035) as f32;
        }
        "tilt3d" => {
            mo.sy = (0.84 + (l * 2.1 * speed).sin() * 0.08) as f32;
            mo.rot = deg((l * 2.1 * speed).sin() * 0.02);
        }
        "zoomIn" => mo.scale = (1.0 + (1.0 - l / 1.4).max(0.0) * 0.32) as f32,
        "impact" => {
            mo.scale = (1.0 + (l * 9.4).sin().max(0.0) * 0.06) as f32;
            mo.tx = ((l * 51.0).sin() * wf * 0.003) as f32;
            mo.ty = ((l * 47.0).cos() * hf * 0.003) as f32;
        }
        "glitch" => mo.tx = ((hash01((l * 18.0).floor()) - 0.5) * wf * 0.022) as f32,
        _ => {}
    }
    mo
}

/// `transitionState`: `None` cuando no hay transición o ya terminó.
fn transition_state(clip: &Clip, local: f64) -> Option<Anim> {
    let kind = clip.extra.get("transition").and_then(Value::as_str).unwrap_or("none");
    if kind == "none" || kind.is_empty() {
        return None;
    }
    let dur = clip.extra.get("transitionDuration").and_then(Value::as_f64).unwrap_or(0.6);
    let duration = dur.min(clip.duration * 0.62).max(0.05);
    // Transición «libre»: empieza `lead` s ANTES del corte (el clip anterior
    // participa en la primera parte, éste en la segunda). `local` puede ser < 0.
    let lead = trans_lead(clip).min(duration);
    if fade_kind(kind) && lead > 0.0 {
        // Oscurecer/aclarar: el anterior se apaga hasta el corte y éste se enciende.
        let rem = (duration - lead).max(0.05);
        if local > rem + 0.0001 {
            return None;
        }
        return Some(Anim { kind: kind.to_string(), p: (local / rem).clamp(0.0, 1.0) as f32 });
    }
    let el = local + lead;
    if el > duration + 0.0001 || el < -0.0001 {
        return None;
    }
    Some(Anim { kind: kind.to_string(), p: (el / duration).clamp(0.0, 1.0) as f32 })
}

/// Segundos que la transición del clip empieza antes del corte (0 = en el corte).
fn trans_lead(clip: &Clip) -> f64 {
    clip.extra.get("transitionLead").and_then(Value::as_f64).unwrap_or(0.0).max(0.0)
}

fn fade_kind(kind: &str) -> bool {
    matches!(kind, "black" | "white" | "fade")
}

/// Clips cuya transición ya ha empezado en `t` aunque su clip aún no (local < 0).
fn early_layers(project: &Project, t: f64) -> Vec<(&Clip, f64)> {
    let mut out = Vec::new();
    for track in &project.tracks {
        if track.hidden {
            continue;
        }
        for clip in &track.clips {
            let lead = trans_lead(clip);
            if lead > 0.0
                && clip.extra.get("transition").and_then(Value::as_str).is_some_and(|k| k != "none" && !k.is_empty())
                && t >= clip.start - lead - 0.0001
                && t < clip.start - 0.0001
            {
                out.push((clip, t - clip.start));
            }
        }
    }
    out
}

/// ¿Hay un filtro GL de textura pura (viñeta, grano, posterizar, división RGB)
/// activo en `t`? El visor lo necesita componer igual que la exportación.
pub fn texture_filter_active(project: &Project, t: f64) -> bool {
    active_gl_texture(project, t).is_some()
}

/// ¿Algún clip visible en `t` usa máscara de forma o suavizado de piel? El
/// visor rápido no los dibuja, así que se compone con la tubería del export.
pub fn shape_or_retouch_active(project: &Project, t: f64) -> bool {
    active_layers(project, t).into_iter().any(|(c, _)| {
        let mask = c
            .extra
            .get("mask")
            .and_then(|m| m.get("shape"))
            .and_then(Value::as_str)
            .is_some_and(|s| s != "none" && !s.is_empty());
        let smooth = c.extra.get("beauty").and_then(|b| b.get("smooth")).and_then(Value::as_f64).unwrap_or(0.0) > 0.0;
        mask || smooth
    })
}

/// ¿Hay una transición de entrada en curso en `t`? El visor usa el compositor
/// completo (igual que el export) mientras dure, para ver barridos, empujes,
/// destellos y el clip anterior por debajo.
pub fn transition_active(project: &Project, t: f64) -> bool {
    active_layers(project, t).into_iter().any(|(c, l)| transition_state(c, l).is_some()) || !early_layers(project, t).is_empty()
}

/// `transitionAlpha`: multiplicador de opacidad según el tipo y el progreso.
fn transition_alpha(anim: &Anim) -> f32 {
    let p = anim.p;
    match anim.kind.as_str() {
        "flash" => (p * 1.8).clamp(0.0, 1.0),
        "fire-burst" | "lightning-flash" | "lens-flare" | "strobe" | "smoke-cloud" | "glitch-rgb"
        | "pixelate" | "rgb-pulse" | "drift-plasma" | "drift-liquid" | "drift-matrix" | "drift-ink"
        | "drift-interference" | "drift-bubbles" => (0.25 + p * 0.9).clamp(0.0, 1.0),
        "zoom-in" | "zoom-out" | "spin" => (0.25 + p * 1.4).clamp(0.0, 1.0),
        "crosszoom" | "dolly" => (0.15 + p * 1.6).clamp(0.0, 1.0),
        "rotate-in" | "warp" | "pulse" => (p * 1.35).clamp(0.0, 1.0),
        "glitch" => (p * 1.5).clamp(0.0, 1.0),
        "film-burn" | "grain" => (0.35 + p * 0.9).clamp(0.0, 1.0),
        // Deslizamientos, barridos y efectos geométricos mantienen opacidad plena.
        "slide-left" | "slide-right" | "slide-up" | "slide-down" | "push-left" | "push-right"
        | "push-up" | "push-down" | "diagonal" | "swing" | "wipe-left" | "wipe-right" | "wipe-up"
        | "wipe-down" | "iris" | "barn" | "blinds" | "clock" | "split" | "curtain" | "blocks"
        | "vignette" | "slam-hit" | "shake-punch" | "earthquake" | "whip-left" | "whip-right"
        | "zoom-snap" | "spin-slam" | "swirl-warp" | "heart-iris" | "star-iris" | "page-curl"
        | "kaleido" | "drift-voronoi" | "drift-swirl" | "drift-honeycomb" | "drift-vhs" => 1.0,
        _ => p,
    }
}

/// `previousClipOnTrack`: clip de la misma pista cuyo final coincide (±0.08 s)
/// con el inicio de `clip`; es decir, el inmediatamente anterior y pegado. Se
/// elige el de arranque más tardío. Como las capas activas llegan clonadas (sin
/// la pista), se localiza la pista que contiene el `id` de `clip` y se busca ahí.
fn previous_clip_on_track<'a>(project: &'a Project, clip: &Clip) -> Option<&'a Clip> {
    let mut best: Option<&Clip> = None;
    for track in &project.tracks {
        if !track.clips.iter().any(|c| c.id == clip.id) {
            continue;
        }
        for cand in &track.clips {
            if cand.id == clip.id {
                continue;
            }
            let end = cand.start + cand.duration;
            if (end - clip.start).abs() <= 0.08 && best.map(|b| cand.start > b.start).unwrap_or(true) {
                best = Some(cand);
            }
        }
    }
    best
}

/// `transitionPreviousOffset`: desplazamiento en píxeles del clip previo durante
/// los `push` (el resto de transiciones lo dejan quieto debajo).
fn push_offset(anim: &Anim, w: f32, h: f32) -> (f32, f32) {
    let p = anim.p;
    match anim.kind.as_str() {
        "push-left" => (-w * p, 0.0),
        "push-right" => (w * p, 0.0),
        "push-up" => (0.0, -h * p),
        "push-down" => (0.0, h * p),
        _ => (0.0, 0.0),
    }
}

/// `applyEntrance`: transformación geométrica de entrada (traslación/escala/
/// rotación). Se inserta tras `rotate(clip.rotate)` y antes del `scale`.
fn entrance_transform(anim: &Anim, w: f32, h: f32) -> Transform {
    let p = anim.p;
    let gap = 1.0 - p;
    let t = Transform::identity();
    match anim.kind.as_str() {
        "slide-left" | "push-left" => t.pre_translate(w * gap, 0.0),
        "slide-right" | "push-right" => t.pre_translate(-w * gap, 0.0),
        "slide-up" | "push-up" => t.pre_translate(0.0, h * gap),
        "slide-down" | "push-down" => t.pre_translate(0.0, -h * gap),
        "diagonal" => t.pre_translate(w * gap, h * gap),
        "swing" => t
            .pre_translate(-w * gap, 0.0)
            .pre_rotate(-gap * 16.0)
            .pre_scale(0.9 + p * 0.1, 0.9 + p * 0.1),
        "zoom-in" => t.pre_scale(1.0 + gap * 0.34, 1.0 + gap * 0.34),
        "zoom-out" => t.pre_scale(0.68 + p * 0.32, 0.68 + p * 0.32),
        "spin" => t.pre_rotate(gap * -14.0).pre_scale(0.9 + p * 0.1, 0.9 + p * 0.1),
        "rotate-in" => t.pre_rotate(gap * -90.0).pre_scale(0.55 + p * 0.45, 0.55 + p * 0.45),
        "crosszoom" => t.pre_scale(1.0 + gap * 1.1, 1.0 + gap * 1.1),
        "pulse" => {
            let overshoot = (p * std::f32::consts::PI).sin() * 0.14;
            let s = 0.8 + ease_out_cubic(p) * 0.2 + overshoot;
            t.pre_scale(s, s)
        }
        "warp" => t.pre_scale(0.2 + p * 0.8, 1.3 - p * 0.3),
        "dolly" => t.pre_scale(1.4 - p * 0.4, 1.4 - p * 0.4).pre_rotate(gap * 6.0),
        "slam-hit" => {
            let shake = (p * 40.0).sin() * (gap * gap) * 24.0;
            let s = 1.0 + (gap * gap) * 0.68;
            t.pre_translate(shake, (p * 30.0).cos() * (gap * gap) * 14.0).pre_scale(s, s)
        }
        "shake-punch" => {
            let punch = (p * std::f32::consts::PI).sin() * 0.22;
            t.pre_translate((p * 45.0).sin() * gap * 26.0, (p * 35.0).cos() * gap * 16.0)
                .pre_scale(1.0 + punch, 1.0 + punch)
        }
        "earthquake" => {
            let decay = gap * gap;
            let jx = ((p * 55.0).sin() + (p * 80.0).cos()) * decay * 18.0;
            let jy = ((p * 65.0).cos() + (p * 90.0).sin()) * decay * 16.0;
            let rot = (p * 45.0).sin() * decay * 0.05;
            t.pre_translate(jx, jy).pre_rotate(rot.to_degrees())
        }
        "whip-left" => {
            let fg = gap.powf(2.8);
            t.pre_translate(w * fg, 0.0).pre_scale(1.0 + fg * 0.45, 1.0)
        }
        "whip-right" => {
            let fg = gap.powf(2.8);
            t.pre_translate(-w * fg, 0.0).pre_scale(1.0 + fg * 0.45, 1.0)
        }
        "zoom-snap" => {
            let spring = 1.0 + (p * std::f32::consts::PI * 2.5).sin() * gap * 0.28;
            t.pre_scale(spring, spring)
        }
        "spin-slam" => {
            let s = 0.4 + p * 0.6;
            t.pre_rotate((-gap * std::f32::consts::PI * 2.0).to_degrees()).pre_scale(s, s)
        }
        "swirl-warp" => t
            .pre_rotate((gap * std::f32::consts::PI * 1.5).to_degrees())
            .pre_scale(0.3 + p * 0.7, 0.3 + p * 0.7),
        "rgb-pulse" => {
            let pulse = 1.0 + (p * std::f32::consts::PI * 3.0).sin() * gap * 0.16;
            t.pre_scale(pulse, pulse)
        }
        "kaleido" => {
            let s = 0.6 + ease_out_cubic(p) * 0.4;
            t.pre_scale(s, s)
        }
        "drift-swirl" => t
            .pre_rotate((gap * std::f32::consts::PI * 2.5).to_degrees())
            .pre_scale(0.25 + p * 0.75, 0.25 + p * 0.75),
        "drift-vhs" => {
            let shift = (p * 50.0).sin() * gap * 16.0;
            let jitter = (hash01((p * 20.0).floor() as f64) as f32 - 0.5) * gap * 10.0;
            t.pre_translate(shift, jitter)
        }
        _ => t,
    }
}

fn rrect(pb: &mut PathBuilder, x: f32, y: f32, w: f32, h: f32) {
    if w > 0.0 && h > 0.0 {
        if let Some(r) = Rect::from_xywh(x, y, w, h) {
            pb.push_rect(r);
        }
    }
}

/// Sector circular (relleno tipo reloj), muestreado como polígono.
fn pie(pb: &mut PathBuilder, r: f32, a0: f32, a1: f32) {
    pb.move_to(0.0, 0.0);
    let steps = 72;
    for i in 0..=steps {
        let a = a0 + (a1 - a0) * i as f32 / steps as f32;
        pb.line_to(a.cos() * r, a.sin() * r);
    }
    pb.close();
}

/// `wipeClip`: trazado de recorte de la transición (en el espacio ya escalado,
/// con las dimensiones del lienzo). Devuelve también una rotación extra en
/// grados para las formas que el original rota antes de recortar. `None` para
/// los tipos sin recorte geométrico.
fn wipe_spec(anim: &Anim, w: f32, h: f32) -> Option<(Path, FillRule, f32)> {
    use std::f32::consts::{FRAC_PI_2, PI};
    let p = anim.p;
    let e = ease_out_cubic(p);
    let (hw, hh) = (w / 2.0, h / 2.0);
    let diag = w.hypot(h);
    let mut pb = PathBuilder::new();
    let mut extra_rot = 0.0f32;
    match anim.kind.as_str() {
        "wipe-left" => rrect(&mut pb, -hw, -hh, w * p, h),
        "wipe-right" => {
            let s = w * p;
            rrect(&mut pb, hw - s, -hh, s, h);
        }
        "wipe-up" => {
            let s = h * p;
            rrect(&mut pb, -hw, hh - s, w, s);
        }
        "wipe-down" => rrect(&mut pb, -hw, -hh, w, h * p),
        "iris" => {
            let r = (diag * 0.55 * e).max(0.01);
            pb.push_circle(0.0, 0.0, r);
        }
        "barn" => {
            let half = hw * e;
            rrect(&mut pb, -hw, -hh, half, h);
            rrect(&mut pb, hw - half, -hh, half, h);
        }
        "blinds" => {
            let rows = 8;
            let rh = h / rows as f32;
            for i in 0..rows {
                rrect(&mut pb, -hw, -hh + i as f32 * rh, w, rh * p);
            }
        }
        "clock" => pie(&mut pb, diag * 0.75, -FRAC_PI_2, -FRAC_PI_2 + PI * 2.0 * e),
        "split" => {
            extra_rot = 45.0;
            let reach = diag * 0.75;
            rrect(&mut pb, -reach, -reach, reach * 2.0 * e, reach * 2.0);
        }
        "curtain" => {
            let s = w * e;
            rrect(&mut pb, -s / 2.0, -hh, s, h);
        }
        "blocks" => {
            let (cols, rows) = (8, 5);
            let (cell_w, cell_h) = (w / cols as f32, h / rows as f32);
            for row in 0..rows {
                for col in 0..cols {
                    let order = (col as f32 + row as f32 * 0.6) / (cols as f32 + rows as f32 * 0.6);
                    let threshold = order * 0.55 + hash01(col as f64 * 13.7 + row as f64 * 7.3) as f32 * 0.45;
                    if e >= threshold {
                        rrect(&mut pb, -hw + col as f32 * cell_w, -hh + row as f32 * cell_h, cell_w + 0.5, cell_h + 0.5);
                    }
                }
            }
        }
        "heart-iris" => {
            let size = (diag * 0.042 * e).max(0.01);
            let mut theta = 0.0f32;
            let mut first = true;
            while theta <= PI * 2.0 + 0.05 {
                let hx = 16.0 * theta.sin().powi(3);
                let hy = -(13.0 * theta.cos() - 5.0 * (2.0 * theta).cos() - 2.0 * (3.0 * theta).cos() - (4.0 * theta).cos());
                if first {
                    pb.move_to(hx * size, hy * size);
                    first = false;
                } else {
                    pb.line_to(hx * size, hy * size);
                }
                theta += 0.06;
            }
            pb.close();
        }
        "star-iris" => {
            let outer = (diag * 0.72 * e).max(0.01);
            let inner = outer * 0.44;
            let points = 5;
            let rot = (1.0 - p) * 0.5;
            for i in 0..points * 2 {
                let r = if i % 2 == 0 { outer } else { inner };
                let angle = i as f32 * PI / points as f32 - FRAC_PI_2 + rot;
                let (sx, sy) = (angle.cos() * r, angle.sin() * r);
                if i == 0 {
                    pb.move_to(sx, sy);
                } else {
                    pb.line_to(sx, sy);
                }
            }
            pb.close();
        }
        "page-curl" => {
            extra_rot = 30.0;
            let reach = diag * 1.5;
            let prog = e * reach;
            rrect(&mut pb, -reach / 2.0, -reach / 2.0 + reach - prog, reach, prog);
        }
        "drift-honeycomb" => {
            let hex = (w / 14.0).max(16.0);
            let cols = (w / (hex * 1.5)).ceil() as i32 + 2;
            let rows = (h / (hex * 1.732)).ceil() as i32 + 2;
            let norm = (hw.hypot(hh)).max(1.0);
            let (c_lo, c_hi) = (-(cols / 2), (cols as f32 / 2.0).ceil() as i32);
            let (r_lo, r_hi) = (-(rows / 2), (rows as f32 / 2.0).ceil() as i32);
            for c in c_lo..=c_hi {
                for r in r_lo..=r_hi {
                    let hx = c as f32 * hex * 1.5;
                    let hy = r as f32 * hex * 1.732 + ((c.abs() % 2) as f32) * (hex * 0.866);
                    let dist = hx.hypot(hy) / norm;
                    if e >= dist * 0.7 {
                        let s = ((e - dist * 0.7) / 0.3).min(1.0) * hex * 0.95;
                        pb.move_to(hx + s, hy);
                        for a in 1..6 {
                            let ang = a as f32 * PI / 3.0;
                            pb.line_to(hx + ang.cos() * s, hy + ang.sin() * s);
                        }
                        pb.close();
                    }
                }
            }
        }
        _ => return None,
    }
    Some((pb.finish()?, FillRule::Winding, extra_rot))
}

fn mask_shape(pb: &mut PathBuilder, shape: &str, mw: f32, mh: f32) {
    let (hw, hh) = (mw / 2.0, mh / 2.0);
    match shape {
        "circle" => {
            if let Some(r) = Rect::from_xywh(-hw, -hh, mw, mh) {
                pb.push_oval(r);
            }
        }
        "rounded" => rounded_into(pb, -hw, -hh, mw, mh, mw.min(mh) * 0.12),
        "triangle" => {
            pb.move_to(0.0, -hh);
            pb.line_to(hw, hh);
            pb.line_to(-hw, hh);
            pb.close();
        }
        "heart" => {
            let s = mw.min(mh) / 2.0;
            pb.move_to(0.0, s * 0.9);
            pb.cubic_to(s * 1.2, s * 0.1, s * 0.6, -s, 0.0, -s * 0.35);
            pb.cubic_to(-s * 0.6, -s, -s * 1.2, s * 0.1, 0.0, s * 0.9);
            pb.close();
        }
        "star" => {
            let outer = mw.min(mh) / 2.0;
            let inner = outer * 0.45;
            for index in 0..10 {
                let radius = if index % 2 == 0 { outer } else { inner };
                let angle = std::f32::consts::PI / 5.0 * index as f32 - std::f32::consts::FRAC_PI_2;
                let (px, py) = (angle.cos() * radius, angle.sin() * radius);
                if index == 0 {
                    pb.move_to(px, py);
                } else {
                    pb.line_to(px, py);
                }
            }
            pb.close();
        }
        "linear" => {
            pb.move_to(-hw, -hh);
            pb.line_to(mw * 0.1, -hh);
            pb.line_to(-mw * 0.1, hh);
            pb.line_to(-hw, hh);
            pb.close();
        }
        "mirror" => {
            if let Some(r) = Rect::from_xywh(-hw, -hh, hw, mh) {
                pb.push_rect(r);
            }
        }
        // `rect` y cualquier forma desconocida: rectángulo completo.
        _ => {
            if let Some(r) = Rect::from_xywh(-hw, -hh, mw, mh) {
                pb.push_rect(r);
            }
        }
    }
}

fn rounded_into(pb: &mut PathBuilder, x: f32, y: f32, w: f32, h: f32, r: f32) {
    let r = r.min(w / 2.0).min(h / 2.0).max(0.0);
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
}

fn flag(clip: &Clip, key: &str) -> bool {
    clip.extra.get(key).and_then(|v| v.as_bool()).unwrap_or(false)
}

// ── Superposiciones de transición a pantalla completa ──
// Despachador de `renderFrame` (renderer.js 3674-3734): adornos decorativos que
// se pintan sobre el fotograma ya compuesto, iguales en vista previa y
// exportación, localizados con `animFor(tipo)` (primera capa cuyo
// `transitionState.type` coincide). Quedan fuera `glitch`/`film-burn`/`grain`:
// requieren relectura del lienzo (`drawImage` del propio canvas) o textura de
// ruido (`noiseTile`), un rasgo aparte.

/// Color RGBA con alfa en 0..1 (helper local de esta pasada).
#[inline]
fn ov_col(r: u8, g: u8, b: u8, a: f32) -> Color {
    Color::from_rgba8(r, g, b, (a.clamp(0.0, 1.0) * 255.0).round() as u8)
}

/// `ctx.fillRect` de color plano (acepta x/y negativos; recorta al lienzo).
fn ov_fill_rect(canvas: &mut Pixmap, x: f32, y: f32, w: f32, h: f32, color: Color, blend: Option<BlendMode>) {
    if w <= 0.0 || h <= 0.0 || color.alpha() <= 0.0 {
        return;
    }
    if let Some(r) = Rect::from_xywh(x, y, w, h) {
        canvas.fill_rect(r, &paint_color(color, blend), Transform::identity(), None);
    }
}

/// `ctx.arc(...); ctx.fill()` con degradado (círculo relleno por shader).
fn ov_fill_circle(canvas: &mut Pixmap, cx: f32, cy: f32, r: f32, shader: Shader<'static>, blend: Option<BlendMode>) {
    if r <= 0.0 {
        return;
    }
    if let Some(path) = PathBuilder::from_circle(cx, cy, r) {
        fill_path_shader(canvas, &path, shader, blend);
    }
}

/// `ctx.arc(...); ctx.fill()` con color plano.
fn ov_fill_circle_col(canvas: &mut Pixmap, cx: f32, cy: f32, r: f32, color: Color, blend: Option<BlendMode>) {
    if r <= 0.0 || color.alpha() <= 0.0 {
        return;
    }
    if let Some(path) = PathBuilder::from_circle(cx, cy, r) {
        fill_path_ex(canvas, &path, color, Transform::identity(), blend);
    }
}

/// `animFor(tipo)`: primera capa activa cuyo `transition_state().kind` coincide.
fn anim_for(layers: &[(Clip, f64)], kind: &str) -> Option<Anim> {
    layers.iter().find_map(|(c, l)| transition_state(c, *l).filter(|a| a.kind == kind))
}

/// `drawFireBurstOverlay`: resplandor radial cálido (screen) + 16 chispas.
fn ov_fire_burst(canvas: &mut Pixmap, anim: &Anim, w: f32, h: f32) {
    let p = anim.p;
    if p > 0.85 {
        return;
    }
    let intensity = ((p / 0.85) * std::f32::consts::PI).sin();
    if intensity <= 0.01 {
        return;
    }
    let (cx, cy) = (w / 2.0, h / 2.0);
    let max_r = (w * w + h * h).sqrt() * 0.65;
    if let (Some(sh), Some(full)) = (
        radial_shader(cx, cy, 0.0, max_r, &[
            (0.0, ov_col(255, 255, 240, intensity * 0.95)),
            (0.2, ov_col(255, 175, 10, intensity * 0.88)),
            (0.5, ov_col(255, 55, 0, intensity * 0.72)),
            (0.8, ov_col(160, 10, 0, intensity * 0.42)),
            (1.0, ov_col(0, 0, 0, 0.0)),
        ]),
        Rect::from_xywh(0.0, 0.0, w, h),
    ) {
        fill_shader(canvas, full, sh, Some(BlendMode::Screen));
    }
    // Chispas: `globalAlpha = intensity*0.7` se pliega en los alfas de las paradas.
    let ga = intensity * 0.7;
    for i in 0..16 {
        let fi = i as f32;
        let ang = (fi / 16.0) * std::f32::consts::PI * 2.0 + p * 3.0;
        let dist = (0.2 + ((i % 4) as f32) * 0.18) * max_r * (0.4 + p * 0.8);
        let fx = cx + ang.cos() * dist;
        let fy = cy + ang.sin() * dist - p * 50.0;
        let spark_r = 12.0 + ((i % 3) as f32) * 10.0;
        if let Some(sh) = radial_shader(fx, fy, 0.0, spark_r, &[
            (0.0, ov_col(255, 255, 220, 0.95 * ga)),
            (0.5, ov_col(255, 110, 0, 0.65 * ga)),
            (1.0, ov_col(255, 0, 0, 0.0)),
        ]) {
            ov_fill_circle(canvas, fx, fy, spark_r, sh, Some(BlendMode::Screen));
        }
    }
}

/// `drawLightningOverlay`: destello azul/blanco + rayo descendente. El original
/// usa `Math.random` (parpadeo no determinista); aquí se usa `hash01` para una
/// sacudida reproducible, y el resplandor `shadowBlur` se aproxima con un trazo
/// cian ancho translúcido (tiny_skia no tiene sombras).
fn ov_lightning(canvas: &mut Pixmap, anim: &Anim, w: f32, h: f32) {
    let p = anim.p;
    if p > 0.6 {
        return;
    }
    let flash_phase = (p * 32.0).sin();
    if flash_phase < 0.25 {
        return;
    }
    let alpha = (1.0 - p / 0.6) * 0.85;
    let fill_col = if flash_phase > 0.7 {
        ov_col(255, 255, 255, alpha)
    } else {
        ov_col(178, 232, 255, alpha)
    };
    ov_fill_rect(canvas, 0.0, 0.0, w, h, fill_col, None);
    let mut lx = w * (0.35 + hash01((p * 18.0).floor() as f64) as f32 * 0.3);
    let mut ly = 0.0f32;
    let mut pb = PathBuilder::new();
    pb.move_to(lx, ly);
    let mut step = 0u32;
    while ly < h && step < 400 {
        let r1 = hash01(step as f64 * 2.7 + p as f64 * 50.0) as f32;
        let r2 = hash01(step as f64 * 5.1 + p as f64 * 90.0 + 11.0) as f32;
        ly += 25.0 + r1 * 30.0;
        lx += (r2 - 0.5) * 50.0;
        pb.line_to(lx, ly);
        step += 1;
    }
    if let Some(path) = pb.finish() {
        let lw = 3.0 + (1.0 - p) * 4.0;
        let glow = Stroke { width: lw + 10.0, ..Default::default() };
        stroke_ex(canvas, &path, ov_col(0, 195, 255, alpha * 0.4), &glow, Transform::identity(), Some(BlendMode::Screen));
        let st = Stroke { width: lw, ..Default::default() };
        stroke_ex(canvas, &path, ov_col(255, 255, 255, alpha), &st, Transform::identity(), None);
    }
}

/// `drawSmokeCloudOverlay`: 9 bocanadas radiales (`globalAlpha = intensity`).
fn ov_smoke_cloud(canvas: &mut Pixmap, anim: &Anim, w: f32, h: f32) {
    let p = anim.p;
    let intensity = (p * std::f32::consts::PI).sin() * 0.75;
    if intensity < 0.02 {
        return;
    }
    let (cx, cy) = (w / 2.0, h / 2.0);
    let max_r = (w * w + h * h).sqrt() * 0.55;
    for i in 0..9 {
        let fi = i as f32;
        let ox = cx + (fi * 1.7 + p * 2.0).sin() * max_r * 0.45;
        let oy = cy + (fi * 2.3 + p * 2.0).cos() * max_r * 0.35;
        let rad = max_r * (0.35 + ((i % 3) as f32) * 0.15);
        if let Some(sh) = radial_shader(ox, oy, 0.0, rad, &[
            (0.0, ov_col(235, 235, 240, 0.7 * intensity)),
            (0.5, ov_col(200, 205, 215, 0.35 * intensity)),
            (1.0, ov_col(200, 205, 215, 0.0)),
        ]) {
            ov_fill_circle(canvas, ox, oy, rad, sh, None);
        }
    }
}

/// `drawGlitchRgbOverlay`: bandas rojas/cian desplazadas en horizontal (screen).
fn ov_glitch_rgb(canvas: &mut Pixmap, anim: &Anim, w: f32, h: f32) {
    let p = anim.p;
    if p > 0.8 {
        return;
    }
    let intensity = 1.0 - p / 0.8;
    let shift = intensity * 26.0 * if (p * 45.0).sin() > 0.0 { 1.0 } else { -1.0 };
    let red = ov_col(255, 0, 60, intensity * 0.35);
    let cyan = ov_col(0, 240, 255, intensity * 0.35);
    let mut y = 0.0;
    while y < h {
        if hash01(y as f64 + p as f64 * 100.0) > 0.4 {
            ov_fill_rect(canvas, shift, y, w, 12.0, red, Some(BlendMode::Screen));
        }
        y += 18.0;
    }
    let mut y = 0.0;
    while y < h {
        if hash01(y as f64 + p as f64 * 100.0 + 7.0) > 0.4 {
            ov_fill_rect(canvas, -shift, y, w, 12.0, cyan, Some(BlendMode::Screen));
        }
        y += 18.0;
    }
}

/// `drawRgbPulseOverlay`: dos marcos de borde (magenta/cian) latiendo (screen).
fn ov_rgb_pulse(canvas: &mut Pixmap, anim: &Anim, w: f32, h: f32) {
    let p = anim.p;
    let pulse = (p * std::f32::consts::PI).sin() * 0.65;
    if pulse < 0.05 {
        return;
    }
    if let Some(r) = Rect::from_xywh(0.0, 0.0, w, h) {
        let st = Stroke { width: 22.0 * pulse, ..Default::default() };
        stroke_ex(canvas, &PathBuilder::from_rect(r), ov_col(255, 0, 180, pulse * 0.65), &st, Transform::identity(), Some(BlendMode::Screen));
    }
    if let Some(r) = Rect::from_xywh(10.0, 10.0, w - 20.0, h - 20.0) {
        let st = Stroke { width: 12.0 * pulse, ..Default::default() };
        stroke_ex(canvas, &PathBuilder::from_rect(r), ov_col(0, 230, 255, pulse * 0.65), &st, Transform::identity(), Some(BlendMode::Screen));
    }
}

/// `drawLensFlareOverlay`: franja horizontal con degradado + núcleo radial (screen).
fn ov_lens_flare(canvas: &mut Pixmap, anim: &Anim, w: f32, h: f32) {
    let p = anim.p;
    let intensity = (p * std::f32::consts::PI).sin();
    if intensity < 0.02 {
        return;
    }
    let flare_x = p * w;
    let flare_y = h * 0.45;
    // Las paradas son monótonas crecientes: max(0,p-k) y min(1,p+k) conservan el orden.
    let streak = [
        ((p - 0.35).max(0.0), ov_col(0, 150, 255, 0.0)),
        ((p - 0.1).max(0.0), ov_col(0, 200, 255, intensity * 0.7)),
        (p, ov_col(255, 255, 255, intensity * 0.95)),
        ((p + 0.1).min(1.0), ov_col(255, 200, 80, intensity * 0.7)),
        ((p + 0.35).min(1.0), ov_col(255, 120, 0, 0.0)),
    ];
    if let (Some(sh), Some(band)) = (
        linear_shader(0.0, flare_y, w, flare_y, &streak),
        Rect::from_xywh(0.0, flare_y - 18.0, w, 36.0),
    ) {
        fill_shader(canvas, band, sh, Some(BlendMode::Screen));
    }
    if let Some(sh) = radial_shader(flare_x, flare_y, 0.0, 120.0, &[
        (0.0, ov_col(255, 255, 255, intensity * 0.9)),
        (0.3, ov_col(0, 220, 255, intensity * 0.6)),
        (0.7, ov_col(255, 140, 0, intensity * 0.3)),
        (1.0, ov_col(0, 0, 0, 0.0)),
    ]) {
        ov_fill_circle(canvas, flare_x, flare_y, 120.0, sh, Some(BlendMode::Screen));
    }
}

/// `drawStrobeOverlay`: parpadeo blanco/negro en ciclo de 3.
fn ov_strobe(canvas: &mut Pixmap, anim: &Anim, w: f32, h: f32) {
    let p = anim.p;
    if p > 0.7 {
        return;
    }
    let cycle = ((p * 24.0).floor() as i64).rem_euclid(3);
    if cycle == 0 {
        ov_fill_rect(canvas, 0.0, 0.0, w, h, ov_col(255, 255, 255, (1.0 - p / 0.7) * 0.85), None);
    } else if cycle == 1 {
        ov_fill_rect(canvas, 0.0, 0.0, w, h, ov_col(0, 0, 0, (1.0 - p / 0.7) * 0.75), None);
    }
}

/// `drawPixelateOverlay`: retícula de bloques claros/oscuros según `hash01`.
fn ov_pixelate(canvas: &mut Pixmap, anim: &Anim, w: f32, h: f32) {
    let p = anim.p;
    if p > 0.65 {
        return;
    }
    let intensity = 1.0 - p / 0.65;
    let block = (intensity * 48.0).round().max(8.0);
    let ga = intensity * 0.45;
    let seed = (p as f64 * 8.0).floor();
    let mut x = 0.0;
    while x < w {
        let mut y = 0.0;
        while y < h {
            let hh = hash01(x as f64 * 13.0 + y as f64 * 29.0 + seed);
            let c = if hh > 0.5 {
                ov_col(255, 255, 255, 0.18 * ga)
            } else {
                ov_col(0, 0, 0, 0.18 * ga)
            };
            ov_fill_rect(canvas, x, y, block - 1.0, block - 1.0, c, None);
            y += block;
        }
        x += block;
    }
}

/// `drawVignetteOverlay`: degradado radial oscuro en los bordes.
fn ov_vignette(canvas: &mut Pixmap, anim: &Anim, w: f32, h: f32) {
    let strength = 1.0 - ease_out_cubic(anim.p);
    if strength <= 0.01 {
        return;
    }
    let r0 = w.min(h) * 0.26;
    let r1 = (w * w + h * h).sqrt() * 0.62;
    if let (Some(sh), Some(full)) = (
        radial_shader(w / 2.0, h / 2.0, r0, r1, &[
            (0.0, ov_col(0, 0, 0, 0.0)),
            (1.0, ov_col(0, 0, 0, 0.92 * strength)),
        ]),
        Rect::from_xywh(0.0, 0.0, w, h),
    ) {
        fill_shader(canvas, full, sh, None);
    }
}

/// `drawSlamHitOverlay`: destello blanco + anillo expansivo.
fn ov_slam_hit(canvas: &mut Pixmap, anim: &Anim, w: f32, h: f32) {
    if anim.p > 0.45 {
        return;
    }
    let alpha = 1.0 - anim.p / 0.45;
    ov_fill_rect(canvas, 0.0, 0.0, w, h, ov_col(255, 255, 255, alpha * 0.75), None);
    let radius = anim.p * (w * w + h * h).sqrt() * 0.9;
    if let Some(path) = PathBuilder::from_circle(w / 2.0, h / 2.0, radius) {
        let st = Stroke { width: (14.0 * alpha).max(2.0), ..Default::default() };
        stroke_ex(canvas, &path, ov_col(255, 255, 255, alpha * 0.85), &st, Transform::identity(), None);
    }
}

/// `drawDriftVoronoiOverlay`: 14 radios blancos desde el centro.
fn ov_drift_voronoi(canvas: &mut Pixmap, anim: &Anim, w: f32, h: f32) {
    if anim.p > 0.7 {
        return;
    }
    let intensity = 1.0 - anim.p / 0.7;
    let (cx, cy) = (w / 2.0, h / 2.0);
    let col = ov_col(255, 255, 255, intensity * 0.75);
    let st = Stroke { width: (4.0 * intensity).max(1.0), ..Default::default() };
    let r = (anim.p * 1.2 + 0.1) * (cx * cx + cy * cy).sqrt();
    for i in 0..14 {
        let angle = (i as f32 / 14.0) * std::f32::consts::PI * 2.0;
        let mut pb = PathBuilder::new();
        pb.move_to(cx, cy);
        pb.line_to(cx + angle.cos() * r, cy + angle.sin() * r);
        if let Some(path) = pb.finish() {
            stroke_ex(canvas, &path, col, &st, Transform::identity(), None);
        }
    }
}

/// `drawDriftPlasmaOverlay`: degradado lineal diagonal cálido (screen).
fn ov_drift_plasma(canvas: &mut Pixmap, anim: &Anim, w: f32, h: f32) {
    if anim.p > 0.8 {
        return;
    }
    let intensity = ((anim.p / 0.8) * std::f32::consts::PI).sin();
    if let (Some(sh), Some(full)) = (
        linear_shader(0.0, 0.0, w, h, &[
            (0.0, ov_col(255, 80, 8, intensity * 0.7)),
            (0.5, ov_col(255, 240, 160, intensity * 0.9)),
            (1.0, ov_col(200, 20, 100, intensity * 0.6)),
        ]),
        Rect::from_xywh(0.0, 0.0, w, h),
    ) {
        fill_shader(canvas, full, sh, Some(BlendMode::Screen));
    }
}

/// `drawDriftMatrixOverlay`: 24 columnas de lluvia verde cayendo.
fn ov_drift_matrix(canvas: &mut Pixmap, anim: &Anim, w: f32, h: f32) {
    if anim.p > 0.75 {
        return;
    }
    let intensity = 1.0 - anim.p / 0.75;
    let cols = 24;
    let col_w = w / cols as f32;
    let col = ov_col(0, 255, 100, intensity * 0.8);
    for c in 0..cols {
        let drop_y = ((anim.p as f64 * 3.5 + hash01((c * 17) as f64)) % 1.0) as f32 * h;
        ov_fill_rect(canvas, c as f32 * col_w, drop_y - 30.0, col_w * 0.6, 30.0, col, None);
    }
}

/// `drawDriftInkOverlay`: 7 manchas de tinta oscura expansivas.
fn ov_drift_ink(canvas: &mut Pixmap, anim: &Anim, w: f32, h: f32) {
    if anim.p > 0.8 {
        return;
    }
    let intensity = ((anim.p / 0.8) * std::f32::consts::PI).sin();
    let col = ov_col(10, 7, 16, intensity * 0.88);
    let (cx, cy) = (w / 2.0, h / 2.0);
    let diag = (w * w + h * h).sqrt();
    for i in 0..7 {
        let ox = cx + (hash01((i * 13) as f64) as f32 - 0.5) * w * 0.7;
        let oy = cy + (hash01((i * 29) as f64) as f32 - 0.5) * h * 0.7;
        let r = anim.p * diag * (0.15 + ((i % 3) as f32) * 0.1);
        ov_fill_circle_col(canvas, ox, oy, r, col, None);
    }
}

/// `drawDriftInterferenceOverlay`: barra horizontal con bordes de color.
fn ov_drift_interference(canvas: &mut Pixmap, anim: &Anim, w: f32, h: f32) {
    if anim.p > 0.7 {
        return;
    }
    let intensity = 1.0 - anim.p / 0.7;
    let bar_y = (anim.p * 4.0).rem_euclid(1.0) * h;
    ov_fill_rect(canvas, 0.0, bar_y, w, h * 0.18, ov_col(255, 255, 255, intensity * 0.4), None);
    ov_fill_rect(canvas, 0.0, bar_y - 12.0, w, 4.0, ov_col(0, 255, 255, intensity * 0.3), None);
    ov_fill_rect(canvas, 0.0, bar_y + h * 0.18, w, 4.0, ov_col(255, 0, 150, intensity * 0.3), None);
}

/// `drawDriftBubblesOverlay`: 20 burbujas ascendentes con borde.
fn ov_drift_bubbles(canvas: &mut Pixmap, anim: &Anim, w: f32, h: f32) {
    if anim.p > 0.8 {
        return;
    }
    let intensity = ((anim.p / 0.8) * std::f32::consts::PI).sin();
    let stroke_col = ov_col(255, 255, 255, intensity * 0.8);
    let fill_col = ov_col(255, 255, 255, intensity * 0.15);
    let st = Stroke { width: 2.0, ..Default::default() };
    for i in 0..20 {
        let bx = hash01((i * 31) as f64) as f32 * w;
        let by = h - ((anim.p as f64 * 1.6 + hash01((i * 17) as f64)) % 1.0) as f32 * h;
        let br = 8.0 + ((i % 5) as f32) * 6.0;
        if let Some(path) = PathBuilder::from_circle(bx, by, br) {
            fill_path_ex(canvas, &path, fill_col, Transform::identity(), None);
            stroke_ex(canvas, &path, stroke_col, &st, Transform::identity(), None);
        }
    }
}

/// `drawDriftLiquidOverlay`: 12 columnas líquidas descendentes.
fn ov_drift_liquid(canvas: &mut Pixmap, anim: &Anim, w: f32, h: f32) {
    if anim.p > 0.75 {
        return;
    }
    let intensity = ((anim.p / 0.75) * std::f32::consts::PI).sin();
    let col = ov_col(20, 10, 40, intensity * 0.55);
    let drops = 12;
    let drop_w = w / drops as f32;
    for d in 0..drops {
        let dh = h * (anim.p as f64 * 1.2 + hash01((d * 23) as f64) * 0.3) as f32;
        ov_fill_rect(canvas, d as f32 * drop_w, 0.0, drop_w - 2.0, dh, col, None);
    }
}

/// `drawGlitchOverlay`: desplaza 7 franjas horizontales del fotograma ya
/// compuesto (relectura del lienzo mediante un `Pixmap` instantáneo, igual que
/// `ctx.drawImage(canvas, …)`) y superpone dos barras de color. En exportación
/// no hay transform de contexto, así que `scaleX = scaleY = 1` y `cw/ch`
/// coinciden con el tamaño en píxeles del lienzo.
fn ov_glitch(canvas: &mut Pixmap, anim: &Anim, w: f32, h: f32) {
    let strength = 1.0 - ease_out_cubic(anim.p);
    if strength <= 0.01 {
        return;
    }
    let cw = canvas.width() as usize;
    let ch = canvas.height() as usize;
    if cw == 0 || ch == 0 {
        return;
    }
    let seed = (anim.p * 24.0).floor() as f64;
    let snapshot = canvas.clone();
    let slices = 7usize;
    let paint = PixmapPaint { opacity: 0.9, ..Default::default() };
    for index in 0..slices {
        let y0 = index * ch / slices;
        let y1 = (index + 1) * ch / slices;
        if y1 <= y0 {
            continue;
        }
        let Some(mut band) = Pixmap::new(cw as u32, (y1 - y0) as u32) else {
            continue;
        };
        band.data_mut().copy_from_slice(&snapshot.data()[y0 * cw * 4..y1 * cw * 4]);
        let shift = ((hash01(index as f64 * 3.1 + seed) - 0.5) * cw as f64 * 0.18 * strength as f64)
            .round() as i32;
        canvas.draw_pixmap(shift, y0 as i32, band.as_ref(), &paint, Transform::identity(), None);
    }
    // Dos barras de color (globalAlpha = min(1, strength·1.2) plegado en el alfa).
    let ga = (strength * 1.2).min(1.0);
    let cyan_y = h * (0.15 + (hash01(seed + 1.0) * 0.55) as f32);
    ov_fill_rect(canvas, 0.0, cyan_y, w, (h * 0.02).max(2.0), ov_col(0, 240, 255, 0.24 * ga), None);
    let mag_y = h * (0.1 + (hash01(seed + 4.0) * 0.6) as f32);
    ov_fill_rect(canvas, 0.0, mag_y, w, (h * 0.015).max(2.0), ov_col(255, 0, 120, 0.22 * ga), None);
}

/// `drawFilmBurnOverlay`: degradado radial cálido a pantalla completa + grano.
fn ov_film_burn(canvas: &mut Pixmap, anim: &Anim, w: f32, h: f32) {
    let strength = 1.0 - ease_out_cubic(anim.p);
    if strength <= 0.01 {
        return;
    }
    let r0 = w.min(h) * 0.05;
    let r1 = w.max(h) * 0.72;
    if let (Some(sh), Some(full)) = (
        radial_shader(w * 0.5, h * 0.55, r0, r1, &[
            (0.15, ov_col(255, 186, 74, 0.9 * strength)),
            (0.5, ov_col(226, 92, 22, 0.6 * strength)),
            (1.0, ov_col(36, 6, 0, 0.92 * strength)),
        ]),
        Rect::from_xywh(0.0, 0.0, w, h),
    ) {
        fill_shader(canvas, full, sh, None);
    }
    let paint = PixmapPaint {
        opacity: (strength * 0.4).clamp(0.0, 1.0),
        quality: resvg::tiny_skia::FilterQuality::Bilinear,
        ..Default::default()
    };
    canvas.draw_pixmap(0, 0, noise_tile().as_ref(), &paint, Transform::from_scale(w / 160.0, h / 160.0), None);
}

/// `drawGrainOverlay`: mosaico de ruido estirado (globalAlpha strength·0.5).
fn ov_grain(canvas: &mut Pixmap, anim: &Anim, w: f32, h: f32) {
    let strength = 1.0 - ease_out_cubic(anim.p);
    if strength <= 0.01 {
        return;
    }
    let paint = PixmapPaint {
        opacity: (strength * 0.5).clamp(0.0, 1.0),
        quality: resvg::tiny_skia::FilterQuality::Bilinear,
        ..Default::default()
    };
    canvas.draw_pixmap(0, 0, noise_tile().as_ref(), &paint, Transform::from_scale(w / 160.0, h / 160.0), None);
}

/// Pasada de superposiciones de transición tras componer las capas (todas salvo
/// `white`, que va antes de las capas en `frame`). Replica el orden de
/// `renderFrame`.
fn draw_frame_overlays(canvas: &mut Pixmap, layers: &[(Clip, f64)], w: f32, h: f32) {
    if let Some(a) = anim_for(layers, "black") {
        ov_fill_rect(canvas, 0.0, 0.0, w, h, ov_col(0, 0, 0, (1.0 - a.p) * 0.55), None);
    }
    if let Some(a) = anim_for(layers, "flash") {
        ov_fill_rect(canvas, 0.0, 0.0, w, h, ov_col(255, 255, 255, (1.0 - a.p) * 0.7), None);
    }
    if let Some(a) = anim_for(layers, "glitch") {
        ov_glitch(canvas, &a, w, h);
    }
    if let Some(a) = anim_for(layers, "film-burn") {
        ov_film_burn(canvas, &a, w, h);
    }
    if let Some(a) = anim_for(layers, "vignette") {
        ov_vignette(canvas, &a, w, h);
    }
    if let Some(a) = anim_for(layers, "grain") {
        ov_grain(canvas, &a, w, h);
    }
    if let Some(a) = anim_for(layers, "slam-hit") {
        ov_slam_hit(canvas, &a, w, h);
    }
    if let Some(a) = anim_for(layers, "fire-burst") {
        ov_fire_burst(canvas, &a, w, h);
    }
    if let Some(a) = anim_for(layers, "lightning-flash") {
        ov_lightning(canvas, &a, w, h);
    }
    if let Some(a) = anim_for(layers, "smoke-cloud") {
        ov_smoke_cloud(canvas, &a, w, h);
    }
    if let Some(a) = anim_for(layers, "glitch-rgb") {
        ov_glitch_rgb(canvas, &a, w, h);
    }
    if let Some(a) = anim_for(layers, "rgb-pulse") {
        ov_rgb_pulse(canvas, &a, w, h);
    }
    if let Some(a) = anim_for(layers, "lens-flare") {
        ov_lens_flare(canvas, &a, w, h);
    }
    if let Some(a) = anim_for(layers, "strobe") {
        ov_strobe(canvas, &a, w, h);
    }
    if let Some(a) = anim_for(layers, "pixelate") {
        ov_pixelate(canvas, &a, w, h);
    }
    if let Some(a) = anim_for(layers, "drift-voronoi") {
        ov_drift_voronoi(canvas, &a, w, h);
    }
    if let Some(a) = anim_for(layers, "drift-plasma") {
        ov_drift_plasma(canvas, &a, w, h);
    }
    if let Some(a) = anim_for(layers, "drift-matrix") {
        ov_drift_matrix(canvas, &a, w, h);
    }
    if let Some(a) = anim_for(layers, "drift-ink") {
        ov_drift_ink(canvas, &a, w, h);
    }
    if let Some(a) = anim_for(layers, "drift-interference") {
        ov_drift_interference(canvas, &a, w, h);
    }
    if let Some(a) = anim_for(layers, "drift-bubbles") {
        ov_drift_bubbles(canvas, &a, w, h);
    }
    if let Some(a) = anim_for(layers, "drift-liquid") {
        ov_drift_liquid(canvas, &a, w, h);
    }
}

// ----------------------------------------------------------------------
// Postproceso de textura GL (`gl-post.js` → CPU)
//
// Puerto 1:1 de `applyClipFilters` (`src/herramientas/export.js`): sobre el
// fotograma ya compuesto se aplica el shader del primer clip de vídeo/overlay
// activo, si es un filtro GL de textura pura (viñeta, grano, división RGB,
// posterizar). Los demás shaders GL ya se aproximan por color en `clip_chain`.
// ----------------------------------------------------------------------

/// Shader y cantidad del clip activo cuyo filtro GL es de textura pura, o
/// `None`. Reproduce la selección de `applyClipFilters`: primer clip de
/// vídeo/overlay activo en `t` (en orden de pista), y solo cuenta SU `glFilter`.
fn active_gl_texture(project: &Project, t: f64) -> Option<(&str, f64)> {
    let active = project
        .tracks
        .iter()
        .filter(|tr| matches!(tr.kind, TrackType::Video | TrackType::Overlay))
        .flat_map(|tr| tr.clips.iter())
        .find(|c| t >= c.start && t < c.start + c.duration)?;
    let shader = active.gl_filter.as_deref()?;
    cupcraft_core::filters::is_pure_texture_gl(shader)
        .then(|| (shader, active.gl_filter_amount.unwrap_or(1.0)))
}

#[inline]
fn gl_fract(x: f64) -> f64 {
    x - x.floor()
}
/// `hash(float n) = fract(sin(n) * 43758.5453)` de `gl-post.js`.
#[inline]
fn gl_hash1(n: f64) -> f64 {
    gl_fract(n.sin() * 43758.5453)
}
/// `hash(vec2 p) = fract(sin(dot(p, vec2(12.9898, 78.233))) * 43758.5453)`.
#[inline]
fn gl_hash2(x: f64, y: f64) -> f64 {
    gl_fract((x * 12.9898 + y * 78.233).sin() * 43758.5453)
}
/// `smoothstep` de GLSL (admite `e0 > e1`, como en el shader de viñeta).
#[inline]
fn gl_smoothstep(e0: f64, e1: f64, x: f64) -> f64 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
#[inline]
fn to_u8(v: f64) -> u8 {
    (v * 255.0).round().clamp(0.0, 255.0) as u8
}

/// Reserva en CPU del shader de textura GL sobre el lienzo opaco ya compuesto
/// (RGBA8, premultiplicado == directo porque el alfa es 255). Se usa cuando no
/// hay GPU disponible; en GPU lo ejecuta `services::gpu`.
fn apply_gl_texture_cpu(canvas: &mut Pixmap, shader: &str, time: f64, amount: f64) {
    let (w, h) = (canvas.width() as usize, canvas.height() as usize);
    let buf = canvas.data_mut();
    match shader {
        "vignette" => tex_vignette(buf, w, h, amount),
        "grain" => tex_grain(buf, w, h, time, amount),
        "posterize" => tex_posterize(buf, amount),
        "rgb-split" => tex_rgb_split(buf, w, h, time, amount),
        _ => {}
    }
}

/// `vignette`: oscurece hacia los bordes. `c.rgb * mix(0.45, 1.0, v)` con
/// `v = smoothstep(0.78, 0.32, d*(0.9 + u_amount*0.5))`, `d = dist(uv, 0.5)`.
fn tex_vignette(buf: &mut [u8], w: usize, h: usize, amount: f64) {
    buf.par_chunks_mut(w * 4).enumerate().for_each(|(y, row)| {
        let uy = (y as f64 + 0.5) / h as f64;
        for (x, px) in row.chunks_exact_mut(4).enumerate() {
            let ux = (x as f64 + 0.5) / w as f64;
            let d = ((ux - 0.5).powi(2) + (uy - 0.5).powi(2)).sqrt();
            let v = gl_smoothstep(0.78, 0.32, d * (0.9 + amount * 0.5));
            let m = 0.45 + (1.0 - 0.45) * v; // mix(0.45, 1.0, v)
            for c in &mut px[..3] {
                *c = to_u8(*c as f64 / 255.0 * m);
            }
        }
    });
}

/// `grain`: ruido aditivo por píxel. `c.rgb + (hash(uv*u_res + u_time) - 0.5) *
/// 0.22 * u_amount`. `uv*u_res` = coordenada de píxel + 0.5.
fn tex_grain(buf: &mut [u8], w: usize, _h: usize, time: f64, amount: f64) {
    buf.par_chunks_mut(w * 4).enumerate().for_each(|(y, row)| {
        for (x, px) in row.chunks_exact_mut(4).enumerate() {
            let n = gl_hash2(x as f64 + 0.5 + time, y as f64 + 0.5 + time) - 0.5;
            let add = n * 0.22 * amount;
            for c in &mut px[..3] {
                *c = to_u8(*c as f64 / 255.0 + add);
            }
        }
    });
}

/// `posterize`: cuantiza cada canal. `floor(c*steps + 0.5) / steps` con
/// `steps = max(2.0, 12.0 - u_amount*9.0)`.
fn tex_posterize(buf: &mut [u8], amount: f64) {
    let steps = (12.0 - amount * 9.0).max(2.0);
    buf.par_chunks_exact_mut(4).for_each(|px| {
        for c in &mut px[..3] {
            let v = *c as f64 / 255.0;
            *c = to_u8((v * steps + 0.5).floor() / steps);
        }
    });
}

/// `rgb-split`: desplaza R/B en horizontal por bandas de fila. `band =
/// floor(uv.y*42)`, `shift = (hash(band + floor(u_time*8)) - 0.5) * u_amount *
/// 0.05` (en uv); R en `uv+shift`, B en `uv-shift`, G sin tocar.
fn tex_rgb_split(buf: &mut [u8], w: usize, h: usize, time: f64, amount: f64) {
    let src = buf.to_vec();
    let tphase = (time * 8.0).floor();
    // Muestreo con borde fijado (CLAMP_TO_EDGE), vecino más próximo.
    let sample = |xf: f64, y: usize, ch: usize| -> u8 {
        let xi = xf.round().clamp(0.0, (w - 1) as f64) as usize;
        src[(y * w + xi) * 4 + ch]
    };
    buf.par_chunks_mut(w * 4).enumerate().for_each(|(y, row)| {
        let uy = (y as f64 + 0.5) / h as f64;
        let band = (uy * 42.0).floor();
        let shift = (gl_hash1(band + tphase) - 0.5) * amount * 0.05; // uv
        let dx = shift * w as f64; // píxeles
        for (x, px) in row.chunks_exact_mut(4).enumerate() {
            px[0] = sample(x as f64 + dx, y, 0); // R en uv + d
            px[2] = sample(x as f64 - dx, y, 2); // B en uv - d
            // G (px[1]) y alfa (px[3]) sin cambios.
        }
    });
}

/// Estado del compositor durante una exportación.
pub struct Compositor {
    pub w: u32,
    pub h: u32,
    paths: HashMap<String, PathBuf>,
    dims: HashMap<String, (u32, u32)>,
    streams: HashMap<String, FrameStream>,
    images: HashMap<String, Option<(u32, u32, Vec<u8>)>>,
    text_cache: HashMap<String, Option<Pixmap>>,
    // Matting IA (`drawMattingSource`): modelo ONNX cargado perezosamente, caché
    // de placas de fondo (id de medio o color → RGBA) y ruta del modelo.
    matte: Option<super::matte::Matte>,
    matte_tried: bool,
    matte_model: Option<PathBuf>,
    matte_bg: HashMap<String, (usize, usize, Vec<u8>)>,
    // Compositing en GPU (Fase B): contexto wgpu para las texturas GL, creado
    // perezosamente la primera vez que un fotograma lleva textura (así no se
    // inicializa la GPU en exportaciones sin filtros GL). `None` = reserva CPU.
    gpu: Option<super::gpu::GpuCtx>,
    gpu_tried: bool,
}

impl Compositor {
    /// `paths`: medio → archivo en la caché (resuelto antes en el hilo de UI).
    pub fn new(w: u32, h: u32, paths: HashMap<String, PathBuf>, dims: HashMap<String, (u32, u32)>) -> Self {
        Self {
            w,
            h,
            paths,
            dims,
            streams: HashMap::new(),
            images: HashMap::new(),
            text_cache: HashMap::new(),
            matte: None,
            matte_tried: false,
            matte_model: super::matte::find_model(),
            matte_bg: HashMap::new(),
            gpu: None,
            gpu_tried: false,
        }
    }

    /// Aplica la textura GL sobre el fotograma ya compuesto: primero intenta la
    /// GPU (wgpu, inicialización perezosa) y, si no hay adaptador o falla la
    /// pasada, recurre a la misma matemática en CPU (`rayon`). El resultado es
    /// equivalente; la GPU es `f32` como el shader GLSL original de la web.
    fn apply_gl_texture(&mut self, canvas: &mut Pixmap, shader: &str, time: f64, amount: f64) {
        if let Some(g) = &self.gpu {
            if g.run_texture(shader, canvas.data_mut(), self.w, self.h, time, amount) {
                return;
            }
        }
        apply_gl_texture_cpu(canvas, shader, time, amount);
    }

    fn media_frame(&mut self, clip: &Clip, local: f64) -> Option<Pixmap> {
        let mid = clip.media_id.as_deref()?;
        let path = self.paths.get(mid)?.clone();
        let (cw, ch) = (self.w, self.h);
        if clip.kind == ClipKind::Image {
            if let Some(anim) = super::gif_anim::load(&path) {
                let t = crate::paint::effect_local(clip, local) * clip.speed.max(0.01);
                let (_, rgba) = anim.frame_at(t);
                return rgba_to_pixmap(anim.w, anim.h, &rgba, Some(clip), cw, ch, self.gpu.as_ref(), local);
            }
            self.images.entry(mid.to_string()).or_insert_with(|| {
                let img = image::open(&path).ok()?;
                let img = if img.width() > 4096 || img.height() > 4096 { img.thumbnail(4096, 4096) } else { img };
                let rgba = img.to_rgba8();
                Some((rgba.width(), rgba.height(), rgba.into_raw()))
            });
            let (w, h) = { let (w, h, _) = self.images.get(mid)?.as_ref()?; (*w, *h) };
            if matte_active(clip) {
                let mut rgba = self.images.get(mid)?.as_ref()?.2.clone();
                self.apply_matte(clip, &mut rgba, w as usize, h as usize, local);
                return rgba_to_pixmap(w, h, &rgba, Some(clip), cw, ch, self.gpu.as_ref(), local);
            }
            let (_, _, data) = self.images.get(mid)?.as_ref()?;
            return rgba_to_pixmap(w, h, data, Some(clip), cw, ch, self.gpu.as_ref(), local);
        }
        let src_t = (clip.in_point + crate::paint::effect_local(clip, local) * clip.speed.max(0.01)).max(0.0);
        let (fw, fh) = fit_dims(self.dims.get(mid).copied(), self.w, self.h);
        let reuse = self
            .streams
            .get(&clip.id)
            .map(|s| !s.ended && src_t + 0.5 / s.fps >= s.next_t - 1.0 / s.fps && src_t <= s.next_t + 2.0)
            .unwrap_or(false);
        if !reuse {
            self.streams.remove(&clip.id);
            let s = FrameStream::open(&path, src_t, fw, fh, 60.0_f64.min(120.0))?;
            self.streams.insert(clip.id.clone(), s);
        }
        let mut buf = {
            let s = self.streams.get_mut(&clip.id)?;
            s.frame_at(src_t)?
        };
        if matte_active(clip) {
            self.apply_matte(clip, &mut buf, fw as usize, fh as usize, local);
        }
        rgba_to_pixmap(fw, fh, &buf, Some(clip), cw, ch, self.gpu.as_ref(), local)
    }

    /// Resuelve la placa de fondo del matting (`autoCutoutBg` = id de imagen, o
    /// `autoCutoutBgColor` = color sólido) a RGBA, con caché. Devuelve `None`
    /// si el clip no define ninguna (la web tampoco recorta sin placa).
    fn matte_plate(&mut self, clip: &Clip) -> Option<(usize, usize, Vec<u8>)> {
        let bg_id = clip.extra.get("autoCutoutBg").and_then(Value::as_str).filter(|s| !s.is_empty());
        if let Some(mid) = bg_id {
            if let Some(p) = self.matte_bg.get(mid) {
                return Some(p.clone());
            }
            let path = self.paths.get(mid)?.clone();
            let img = image::open(&path).ok()?.to_rgba8();
            let plate = (img.width() as usize, img.height() as usize, img.into_raw());
            self.matte_bg.insert(mid.to_string(), plate.clone());
            return Some(plate);
        }
        let color = clip.extra.get("autoCutoutBgColor").and_then(Value::as_str).filter(|s| !s.is_empty())?;
        if let Some(p) = self.matte_bg.get(color) {
            return Some(p.clone());
        }
        let [r, g, b, _] = crate::paint::hex(color).to_array();
        let px = [r, g, b, 255u8];
        let mut data = Vec::with_capacity(4 * 4);
        for _ in 0..4 {
            data.extend_from_slice(&px);
        }
        let plate = (2usize, 2usize, data);
        self.matte_bg.insert(color.to_string(), plate.clone());
        Some(plate)
    }

    /// Aplica el recorte IA (`drawMattingSource`): calcula el alpha del sujeto y
    /// lo escribe en el canal alpha del fotograma `rgba` (`w`×`h`). `intensity`
    /// mezcla con el original: 1 recorta del todo, 0 deja la imagen intacta.
    fn apply_matte(&mut self, clip: &Clip, rgba: &mut [u8], w: usize, h: usize, local: f64) {
        let Some((bw, bh, bg)) = self.matte_plate(clip) else { return };
        if !self.matte_tried {
            self.matte_tried = true;
            if let Some(path) = self.matte_model.clone() {
                self.matte = super::matte::Matte::load(&path);
            }
        }
        let Some(m) = self.matte.as_mut() else { return };
        let edge = clip.extra.get("autoCutoutEdge").and_then(Value::as_f64).unwrap_or(0.35) as f32;
        let temporal = clip.extra.get("autoCutoutTemporal").and_then(Value::as_f64).unwrap_or(0.3) as f32;
        let intensity = clip.extra.get("autoCutoutIntensity").and_then(Value::as_f64).unwrap_or(1.0).clamp(0.0, 1.0) as f32;
        let plate = clip.extra.get("autoCutoutBg").and_then(Value::as_str).unwrap_or_else(|| {
            clip.extra.get("autoCutoutBgColor").and_then(Value::as_str).unwrap_or("")
        });
        let key = format!("{}:{}:{}:{}", clip.id, plate, temporal, edge);
        let time = (clip.start + local).max(0.0);
        let Some(alpha) = m.alpha(rgba, w, h, &bg, bw, bh, edge, temporal, &key, time) else { return };
        for y in 0..h {
            for x in 0..w {
                let am = super::matte::sample(&alpha, x, y, w, h);
                let a_out = (1.0 - intensity * (1.0 - am)).clamp(0.0, 1.0);
                let o = (y * w + x) * 4 + 3;
                rgba[o] = (rgba[o] as f32 * a_out).round() as u8;
            }
        }
    }

    /// Compone el fotograma del instante `t`. Devuelve RGBA (sin premultiplicar,
    /// opaco) listo para el codificador.
    pub fn frame(&mut self, project: &Project, t: f64) -> Vec<u8> {
        let (w, h) = (self.w as f32, self.h as f32);
        // Init perezoso del contexto GPU (Fase B): una sola vez por exportación.
        // Lo usan tanto la cadena de color (`rgba_to_pixmap`) como las texturas
        // GL. `None` = sin adaptador → reserva en CPU (`rayon`).
        if !self.gpu_tried {
            self.gpu_tried = true;
            self.gpu = super::gpu::GpuCtx::new();
        }
        let mut canvas = Pixmap::new(self.w, self.h).expect("lienzo");
        canvas.fill(Color::BLACK);
        let mut layers: Vec<(Clip, f64)> = active_layers(project, t).into_iter().map(|(c, l)| (c.clone(), l)).collect();
        // Transiciones libres que empiezan antes del corte: el clip entrante entra
        // en capa (a su primer fotograma) y el saliente se apaga o se desplaza.
        let early: Vec<(Clip, f64)> = early_layers(project, t).into_iter().map(|(c, l)| (c.clone(), l)).collect();
        let mut prev_fade: std::collections::HashMap<String, f32> = std::collections::HashMap::new();
        let mut prev_skip: std::collections::HashSet<String> = std::collections::HashSet::new();
        for (b, tl) in &early {
            if let Some(prev) = previous_clip_on_track(project, b) {
                let kind = b.extra.get("transition").and_then(Value::as_str).unwrap_or("none");
                if fade_kind(kind) {
                    prev_fade.insert(prev.id.clone(), ((-tl) / trans_lead(b).max(0.001)).clamp(0.0, 1.0) as f32);
                } else {
                    prev_skip.insert(prev.id.clone());
                }
            }
        }
        if !early.is_empty() {
            layers.extend(early);
            layers.sort_by_key(|(c, _)| rank(c.kind));
        }
        // Destello blanco de transición (`renderFrame`): se pinta DETRÁS de las
        // capas, por lo que va antes del bucle.
        if let Some(a) = anim_for(&layers, "white") {
            ov_fill_rect(&mut canvas, 0.0, 0.0, w, h, ov_col(255, 255, 255, 1.0 - a.p), None);
        }
        for (clip, local) in layers.iter().cloned() {
            if prev_skip.contains(&clip.id) {
                continue; // lo dibuja la capa entrante (con su desplazamiento)
            }
            let alpha = clip_alpha(&clip, local.max(0.0)) * prev_fade.get(&clip.id).copied().unwrap_or(1.0);
            if alpha <= 0.001 {
                continue;
            }
            // Clip anterior de la pista bajo una transición (`renderFrame`): se
            // dibuja debajo, a su último fotograma, sin fundidos ni transición;
            // los `push` lo desplazan (`transitionPreviousOffset`).
            if let Some(anim) = transition_state(&clip, local) {
                if !fade_kind(&anim.kind) {
                    if let Some(prev) = previous_clip_on_track(project, &clip) {
                        let prev = prev.clone();
                        // Antes del corte el saliente sigue en movimiento; después, su último fotograma.
                        let plocal = if local < 0.0 {
                            (t - prev.start).clamp(0.0, (prev.duration - 0.001).max(0.0))
                        } else {
                            (prev.duration - 0.001).max(0.0)
                        };
                        let palpha =
                            kf_value(&prev, "opacity", prev.start + plocal, 1.0).clamp(0.0, 1.0) as f32;
                        let off = push_offset(&anim, w, h);
                        self.draw_clip_layer(&mut canvas, prev, plocal, w, h, palpha, false, off);
                    }
                }
            }
            self.draw_clip_layer(&mut canvas, clip, local, w, h, alpha, true, (0.0, 0.0));
        }
        // Adornos de transición a pantalla completa, sobre el fotograma compuesto.
        draw_frame_overlays(&mut canvas, &layers, w, h);
        // Postproceso de textura GL (`applyClipFilters`): tras componer todo,
        // el shader de textura pura del clip activo sobre el fotograma final.
        if let Some((shader, amount)) = active_gl_texture(project, t) {
            let shader = shader.to_string();
            self.apply_gl_texture(&mut canvas, &shader, t, amount);
        }
        // Lienzo opaco: premultiplicado == sin premultiplicar.
        canvas.take()
    }

    /// Dibuja una capa (medio/texto/sticker) en el lienzo. Es el cuerpo del bucle
    /// de `frame` extraído para reutilizarlo con el clip previo de una transición.
    /// Con `with_transition = false` se ignoran la transición de entrada y el
    /// barrido (`ignoreTransition` de `drawVisualLayer`); `offset` desplaza el
    /// centro (para los `push`). `alpha` es la opacidad base ya resuelta.
    #[allow(clippy::too_many_arguments)]
    fn draw_clip_layer(
        &mut self,
        canvas: &mut Pixmap,
        clip: Clip,
        local: f64,
        w: f32,
        h: f32,
        alpha: f32,
        with_transition: bool,
        offset: (f32, f32),
    ) {
        // `tl` (puede ser < 0 en una transición que empieza antes del corte) solo
        // sirve para la transición; el medio se lee desde 0.
        let tl = local;
        let local = local.max(0.0);
        let scale = h / 1080.0;
        match clip.kind {
                ClipKind::Video | ClipKind::Image => {
                    let Some(mut src) = self.media_frame(&clip, local) else { return };
                    // Transición de entrada: opacidad × geometría × barrido.
                    let anim = if with_transition { transition_state(&clip, tl) } else { None };
                    let alpha = alpha * anim.as_ref().map(transition_alpha).unwrap_or(1.0);
                    if alpha <= 0.001 {
                        return;
                    }
                    // Desenfoque de transición (`blur`/`dolly`/`crosszoom`):
                    // `ctx.filter` extra de `(1-p)*(10|6)` px, sobre el fotograma ya
                    // premultiplicado. Se escala a px de la fuente.
                    if let Some(a) = anim.as_ref() {
                        let amt = match a.kind.as_str() {
                            "blur" => (1.0 - a.p) * 10.0,
                            "dolly" | "crosszoom" => (1.0 - a.p) * 6.0,
                            _ => 0.0,
                        };
                        if amt > 0.05 {
                            let sigma = amt * src.width() as f32 / w;
                            let (sw0, sh0) = (src.width() as usize, src.height() as usize);
                            gaussian_blur(src.data_mut(), sw0, sh0, sigma);
                        }
                    }
                    let (sw, sh) = (src.width() as f32, src.height() as f32);
                    let fit = (w / sw).min(h / sh);
                    // Transformación con fotogramas clave (`transformAt`).
                    let tf = transform_at(&clip, clip.start + local);
                    // Movimiento procedural del efecto (`effectMotion`).
                    let mut mo = effect_motion(&clip, local, w, h);
                    let ca = crate::clip_anim::eval(&clip, local);
                    mo.tx += ca.dx * w;
                    mo.ty += ca.dy * h;
                    mo.scale *= ca.scale;
                    mo.rot += ca.rot;
                    let alpha = alpha * ca.alpha;
                    let sx = fit * tf.scale_x as f32 * mo.scale * mo.sx * if flag(&clip, "flipH") { -1.0 } else { 1.0 };
                    let sy = fit * tf.scale_y as f32 * mo.scale * mo.sy * if flag(&clip, "flipV") { -1.0 } else { 1.0 };
                    let cx = w / 2.0 + tf.x as f32 / 100.0 * w + offset.0;
                    let cy = h / 2.0 + tf.y as f32 / 100.0 * h + offset.1;
                    let ent = anim.as_ref().map(|a| entrance_transform(a, w, h)).unwrap_or(Transform::identity());
                    // Transformación hasta el `scale` (espacio donde el original aplica `wipeClip`):
                    // centro → rotación → entrada → desplazamiento/rotación del efecto → escala.
                    let tr_pre = Transform::from_translate(cx, cy)
                        .pre_rotate(tf.rotate as f32)
                        .pre_concat(ent)
                        .pre_translate(mo.tx, mo.ty)
                        .pre_rotate(mo.rot)
                        .pre_scale(sx, sy);
                    let tr = tr_pre.pre_translate(-sw / 2.0, -sh / 2.0);
                    let paint = PixmapPaint { opacity: alpha, quality: resvg::tiny_skia::FilterQuality::Bilinear, ..Default::default() };
                    // Máscara de recorte: forma del clip ∩ barrido de la transición.
                    let mask = clip_mask(&clip, clip.start + local, anim.as_ref(), self.w, self.h, tr_pre);
                    // Espejo y clones fantasma (`drawMediaLayerRaw`): se repite el
                    // dibujo del original con recortes y transformaciones extra.
                    match move_spec(&clip) {
                        Some(ms) if ms.mirror => {
                            // Mitad izquierda original.
                            let m1 = mirror_mask(&clip, clip.start + local, anim.as_ref(), self.w, self.h, tr_pre, tr_pre, w, h);
                            canvas.draw_pixmap(0, 0, src.as_ref(), &paint, tr, m1.as_ref());
                            // Mitad derecha reflejada (`scale(-1, 1)`).
                            let tr_pre_f = tr_pre.pre_scale(-1.0, 1.0);
                            let tr_f = tr_pre_f.pre_translate(-sw / 2.0, -sh / 2.0);
                            let m2 = mirror_mask(&clip, clip.start + local, anim.as_ref(), self.w, self.h, tr_pre, tr_pre_f, w, h);
                            canvas.draw_pixmap(0, 0, src.as_ref(), &paint, tr_f, m2.as_ref());
                        }
                        Some(ms) => {
                            // Original más los clones fantasma, cada uno con menos opacidad.
                            canvas.draw_pixmap(0, 0, src.as_ref(), &paint, tr, mask.as_ref());
                            for i in 0..ms.ghosts {
                                let fi = i as f32;
                                let ga = (alpha * (0.16 + fi * 0.04)).clamp(0.0, 1.0);
                                let tr_pre_i = if ms.clone {
                                    let dir = if i % 2 == 0 { -1.0 } else { 1.0 };
                                    tr_pre.pre_translate(dir * w * (0.16 + fi * 0.04), 0.0).pre_scale(0.92, 0.92)
                                } else {
                                    tr_pre.pre_translate(w * 0.04 * (fi + 1.0), h * 0.006 * (fi + 1.0))
                                };
                                let tr_i = tr_pre_i.pre_translate(-sw / 2.0, -sh / 2.0);
                                let paint_i = PixmapPaint { opacity: ga, quality: resvg::tiny_skia::FilterQuality::Bilinear, ..Default::default() };
                                canvas.draw_pixmap(0, 0, src.as_ref(), &paint_i, tr_i, mask.as_ref());
                            }
                        }
                        None => {
                            canvas.draw_pixmap(0, 0, src.as_ref(), &paint, tr, mask.as_ref());
                        }
                    }
                    // Trazo del recorte (`drawStroke`), sobre la forma de máscara.
                    draw_cutout_stroke(canvas, &clip, clip.start + local, self.w, self.h);
                    // Superposición del efecto, en espacio de pantalla
                    // (`drawEffectOverlay`: overlay → marco → partículas).
                    if let Some(os) = overlay_spec(&clip) {
                        draw_overlay(canvas, &os, local, w, h, alpha);
                    }
                    // Marco del efecto, en espacio de pantalla (`drawEffectOverlay`,
                    // antes de las partículas).
                    if let Some(fs) = frame_spec(&clip) {
                        draw_frame(canvas, &fs, w, h, alpha);
                    }
                    // Partículas del efecto, en espacio de pantalla (`drawEffectOverlay`).
                    if let Some(ps) = particle_spec(&clip) {
                        draw_particles(canvas, &ps, local, w, h, alpha);
                    }
                }
                ClipKind::Text | ClipKind::Sticker => {
                    let (val, is_text) = match clip.kind {
                        ClipKind::Text => (clip.text.clone(), true),
                        _ => (clip.sticker.clone(), false),
                    };
                    let Some(val) = val else { return };
                    let val = if is_text { crate::fx_text::apply(&val, &clip, local).unwrap_or(val) } else { val };
                    let val = if is_text { crate::karaoke::apply(&val, local) } else { val };
                    // Transición de entrada del clip (opacidad × geometría × barrido).
                    let tanim = if with_transition { transition_state(&clip, tl) } else { None };
                    let anim = if is_text { raster::text_anim(&val, local, clip.duration) } else { None };
                    let alpha = alpha
                        * anim.map(|a| a.alpha).unwrap_or(1.0)
                        * tanim.as_ref().map(transition_alpha).unwrap_or(1.0);
                    if alpha <= 0.002 {
                        return;
                    }
                    let base = val.clone();
                    let val = match anim {
                        Some(a) if a.reveal < 0.999 => raster::reveal_text(&val, a.reveal),
                        _ => val,
                    };
                    let key = format!("{}:{}:{}", is_text, scale, val);
                    if self.text_cache.len() > 256 && !self.text_cache.contains_key(&key) {
                        self.text_cache.clear();
                    }
                    let pm = self
                        .text_cache
                        .entry(key)
                        .or_insert_with(|| if is_text { raster::render_text(&val, scale) } else { raster::render_sticker(&val, scale) })
                        .clone();
                    let Some(pm) = pm else { return };
                    let vx = val.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let vy = val.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let vrot = if is_text { 0.0 } else { val.get("rotate").and_then(|v| v.as_f64()).unwrap_or(0.0) };
                    let tf = transform_at(&clip, clip.start + local);
                    let cx = w / 2.0 + ((tf.x + vx) as f32 / 100.0) * w + offset.0;
                    let cy = h / 2.0 + ((tf.y + vy) as f32 / 100.0) * h + offset.1;
                    let (pw, ph) = (pm.width() as f32, pm.height() as f32);
                    let (asc, adx, ady, arot) = anim.map(|a| (a.scale, a.dx, a.dy, a.rotate)).unwrap_or((1.0, 0.0, 0.0, 0.0));
                    let fpx = raster::text_font_px(&base, scale);
                    // Transición de entrada (`applyEntrance`): entre la rotación y la escala.
                    let ent = tanim.as_ref().map(|a| entrance_transform(a, w, h)).unwrap_or(Transform::identity());
                    // Transformación hasta el espacio del barrido (`wipeClip`).
                    let ca = crate::clip_anim::eval(&clip, local);
                    let alpha = alpha * ca.alpha;
                    let tr_wipe = Transform::from_translate(cx + ca.dx * w, cy + ca.dy * h)
                        .pre_rotate((tf.rotate + vrot + ca.rot as f64) as f32)
                        .pre_concat(ent)
                        .pre_scale(tf.scale_x as f32 * asc * ca.scale, tf.scale_y as f32 * asc * ca.scale)
                        .pre_translate(adx * fpx, ady * fpx)
                        .pre_rotate(arot);
                    let tr = tr_wipe.pre_translate(-pw / 2.0, -ph / 2.0);
                    // Máscara de barrido de la transición (el texto/sticker no define forma).
                    let mask = clip_mask(&clip, clip.start + local, tanim.as_ref(), self.w, self.h, tr_wipe);
                    let paint = PixmapPaint { opacity: alpha, quality: resvg::tiny_skia::FilterQuality::Bilinear, ..Default::default() };
                    canvas.draw_pixmap(0, 0, pm.as_ref(), &paint, tr, mask.as_ref());
                }
                _ => {}
            }
    }
}
