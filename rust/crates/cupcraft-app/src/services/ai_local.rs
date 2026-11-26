//! Interruptores «locales» de Video → Básico (estabilización, mejora, ruido,
//! flujo óptico, iluminación, encuadre, parpadeo, movimiento, seguimiento y
//! movimiento difuminado). Todo se calcula en CPU sobre el fotograma RGBA, sin
//! nube. Vista previa y exportación comparten esta función.
//!
//! Los efectos temporales (estabilizar, flujo, seguimiento…) guardan un estado
//! por clip y analizan el movimiento global entre fotogramas consecutivos con
//! coincidencia de bloques sobre una miniatura en gris. Si el tiempo salta
//! (búsqueda en la línea de tiempo) el estado se reinicia.

use cupcraft_core::Clip;
use rayon::prelude::*;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

#[derive(Clone, Copy, Default, PartialEq)]
pub struct AiParams {
    pub enhance: bool,
    pub expand: bool,
    pub denoise: bool,
    pub stabilize: bool,
    pub optical: bool,
    pub lighting: bool,
    pub auto_frame: bool,
    pub flicker: bool,
    pub motion: bool,
    pub tracking: bool,
    pub blur_motion: bool,
}

impl AiParams {
    pub fn from_clip(c: &Clip) -> Option<Self> {
        let f = |k: &str| c.extra.get(k).and_then(Value::as_bool).unwrap_or(false);
        let p = AiParams {
            enhance: f("enhanceQuality"),
            expand: f("aiExpand"),
            denoise: f("denoise"),
            stabilize: f("stabilize"),
            optical: f("opticalFlow"),
            lighting: f("lighting"),
            auto_frame: f("autoFrame"),
            flicker: f("flickerRemove"),
            motion: f("aiMotion"),
            tracking: f("cameraTracking"),
            blur_motion: f("blurMotion"),
        };
        (p != AiParams::default()).then_some(p)
    }

    fn bits(&self) -> u64 {
        [
            self.enhance, self.expand, self.denoise, self.stabilize, self.optical, self.lighting,
            self.auto_frame, self.flicker, self.motion, self.tracking, self.blur_motion,
        ]
        .iter()
        .enumerate()
        .fold(0u64, |a, (i, b)| a | ((*b as u64) << i))
    }

    fn temporal(&self) -> bool {
        self.stabilize || self.optical || self.tracking || self.blur_motion || self.denoise || self.flicker
    }

    fn needs_prev(&self) -> bool {
        self.optical || self.blur_motion || self.denoise
    }
}

/// Contexto de un fotograma: identifica el clip (estado temporal) y su instante.
pub struct AiCtx {
    pub id: String,
    pub local: f64,
    pub dur: f64,
    pub p: AiParams,
}

impl AiCtx {
    /// Huella para la caché de texturas de la vista previa.
    pub fn hash(&self) -> u64 {
        let mut h = self.p.bits().wrapping_mul(0x9E37_79B9_7F4A_7C15);
        if self.p.motion {
            h ^= ((self.local * 30.0) as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
        }
        h
    }
}

#[derive(Default, Clone, Copy)]
struct Step {
    d: (f32, f32),
    corr: (f32, f32),
    track: (f32, f32),
    gain: f32,
}

#[derive(Default)]
struct State {
    inited: bool,
    dims: (usize, usize),
    last_local: f64,
    last_small: Vec<u8>,
    last: Vec<u8>,
    prev: Vec<u8>,
    has_prev: bool,
    path: (f32, f32),
    smooth: (f32, f32),
    ema: f32,
    step: Step,
}

static STATE: OnceLock<Mutex<HashMap<String, State>>> = OnceLock::new();

pub fn process(c: &AiCtx, buf: &mut Vec<u8>, w: usize, h: usize) {
    let p = &c.p;
    if w < 8 || h < 8 || buf.len() != w * h * 4 {
        return;
    }
    let mut step = Step { gain: 1.0, ..Default::default() };
    let mut prev: Option<Vec<u8>> = None;

    if p.temporal() {
        let mut map = STATE
            .get_or_init(|| Mutex::new(HashMap::new()))
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let st = map.entry(c.id.clone()).or_default();
        let dt = c.local - st.last_local;
        let fresh = !st.inited || st.dims != (w, h) || !(-1e-6..0.35).contains(&dt);
        if fresh {
            *st = State { inited: true, dims: (w, h), step: Step { gain: 1.0, ..Default::default() }, ..Default::default() };
        }
        if fresh || dt > 1e-6 {
            let (small, sw, sh) = gray_small(buf, w, h);
            let luma = small.iter().map(|&v| v as f32).sum::<f32>() / small.len().max(1) as f32;
            if fresh {
                st.ema = luma;
                if p.needs_prev() {
                    st.last = buf.clone();
                }
            } else {
                let (dx, dy) = match_blocks(&st.last_small, &small, sw, sh);
                let k = w as f32 / sw as f32;
                let d = (dx as f32 * k, dy as f32 * k);
                st.path = (st.path.0 + d.0, st.path.1 + d.1);
                st.smooth = (st.smooth.0 + (st.path.0 - st.smooth.0) * 0.1, st.smooth.1 + (st.path.1 - st.smooth.1) * 0.1);
                let (mx, my) = (w as f32 * 0.05, h as f32 * 0.05);
                let corr = ((st.path.0 - st.smooth.0).clamp(-mx, mx), (st.path.1 - st.smooth.1).clamp(-my, my));
                st.path = (st.smooth.0 + corr.0, st.smooth.1 + corr.1);
                // Seguimiento: centroide de los píxeles que cambian.
                let (mut sx, mut sy, mut wt) = (0f32, 0f32, 0f32);
                for y in 0..sh {
                    for x in 0..sw {
                        let i = y * sw + x;
                        let df = (small[i] as i32 - st.last_small[i] as i32).unsigned_abs();
                        if df > 18 {
                            sx += (x as f32 - sw as f32 / 2.0) * df as f32;
                            sy += (y as f32 - sh as f32 / 2.0) * df as f32;
                            wt += df as f32;
                        }
                    }
                }
                let target = if wt > 400.0 { (sx / wt * k, sy / wt * k) } else { (0.0, 0.0) };
                let tr = st.step.track;
                let track = (tr.0 + (target.0 - tr.0) * 0.12, tr.1 + (target.1 - tr.1) * 0.12);
                st.ema = st.ema * 0.9 + luma * 0.1;
                st.step = Step { d, corr, track, gain: (st.ema / luma.max(1.0)).clamp(0.85, 1.18) };
                if p.needs_prev() {
                    st.prev = std::mem::replace(&mut st.last, buf.clone());
                    st.has_prev = true;
                }
            }
            st.last_small = small;
            st.last_local = c.local;
        }
        step = st.step;
        if p.needs_prev() && st.has_prev {
            prev = Some(st.prev.clone());
        }
    }

    // 1) Parpadeo: iguala la luminosidad a su media móvil.
    if p.flicker && (step.gain - 1.0).abs() > 0.002 {
        let g = step.gain;
        buf.par_chunks_mut(4).for_each(|px| {
            for ch in px.iter_mut().take(3) {
                *ch = (*ch as f32 * g).round().clamp(0.0, 255.0) as u8;
            }
        });
    }
    // 2) Mezcla temporal (ruido, flujo) y desenfoque de movimiento.
    if let Some(pv) = prev.as_deref() {
        if p.denoise {
            temporal_blend(buf, pv, w, h, step.d, 0.35, true);
        }
        if p.optical {
            temporal_blend(buf, pv, w, h, step.d, 0.4, false);
        }
        if p.blur_motion {
            temporal_blend(buf, pv, w, h, step.d, 0.25, false);
        }
    }
    if p.blur_motion {
        directional_blur(buf, w, h, step.d);
    }
    // 3) Ruido espacial.
    if p.denoise {
        bilateral3(buf, w, h);
    }
    // 4) Geometría combinada: estabilizar + encuadre + movimiento + seguimiento.
    let (mut zoom, mut ox, mut oy) = (1.0f32, 0.0f32, 0.0f32);
    if p.stabilize {
        zoom *= 1.07;
        ox -= step.corr.0;
        oy -= step.corr.1;
    }
    if p.auto_frame {
        zoom *= 1.15;
    }
    if p.motion {
        let t = (c.local / c.dur.max(0.1)).clamp(0.0, 1.0) as f32;
        zoom *= 1.0 + 0.14 * t;
    }
    if p.tracking {
        zoom *= 1.2;
        ox += step.track.0;
        oy += step.track.1;
    }
    let (mx, my) = (w as f32 * (1.0 - 1.0 / zoom) / 2.0, h as f32 * (1.0 - 1.0 / zoom) / 2.0);
    ox = ox.clamp(-mx, mx);
    oy = oy.clamp(-my, my);
    if zoom > 1.001 || ox.abs() > 0.3 || oy.abs() > 0.3 {
        *buf = resample(buf, w, h, zoom, ox, oy);
    }
    // 5) Nitidez (mejorar calidad / ampliación).
    let amount = if p.enhance { 0.8 } else { 0.0 } + if p.expand { 0.4 } else { 0.0 };
    if amount > 0.0 {
        unsharp(buf, w, h, amount);
    }
    // 6) Iluminación.
    if p.lighting {
        add_light(buf, w, h);
    }
}

fn gray_small(buf: &[u8], w: usize, h: usize) -> (Vec<u8>, usize, usize) {
    let sw = w.min(96);
    let sh = (h * sw / w).max(8);
    let mut out = vec![0u8; sw * sh];
    for y in 0..sh {
        for x in 0..sw {
            let mut acc = 0u32;
            for (fx, fy) in [(0.25, 0.25), (0.75, 0.25), (0.25, 0.75), (0.75, 0.75)] {
                let px = (((x as f32 + fx) / sw as f32) * w as f32) as usize;
                let py = (((y as f32 + fy) / sh as f32) * h as f32) as usize;
                let i = (py.min(h - 1) * w + px.min(w - 1)) * 4;
                acc += (buf[i] as u32 * 77 + buf[i + 1] as u32 * 150 + buf[i + 2] as u32 * 29) >> 8;
            }
            out[y * sw + x] = (acc / 4) as u8;
        }
    }
    (out, sw, sh)
}

/// Traslación global (dx, dy) tal que `cur(x) ≈ prev(x + d)`.
fn match_blocks(prev: &[u8], cur: &[u8], sw: usize, sh: usize) -> (i32, i32) {
    if prev.len() != cur.len() || prev.len() != sw * sh {
        return (0, 0);
    }
    const R: i32 = 5;
    let sad = |dx: i32, dy: i32| -> u64 {
        let mut s = 0u64;
        for y in R..(sh as i32 - R) {
            for x in R..(sw as i32 - R) {
                let a = cur[y as usize * sw + x as usize] as i32;
                let b = prev[(y + dy) as usize * sw + (x + dx) as usize] as i32;
                s += (a - b).unsigned_abs() as u64;
            }
        }
        s
    };
    let zero = sad(0, 0);
    let (mut best, mut bd) = ((zero as f64 * 0.97) as u64, (0, 0));
    for dy in -R..=R {
        for dx in -R..=R {
            if dx == 0 && dy == 0 {
                continue;
            }
            let s = sad(dx, dy);
            if s < best {
                best = s;
                bd = (dx, dy);
            }
        }
    }
    bd
}

#[inline]
fn px_at(buf: &[u8], w: usize, h: usize, x: i32, y: i32) -> usize {
    (y.clamp(0, h as i32 - 1) as usize * w + x.clamp(0, w as i32 - 1) as usize) * 4
}

/// Mezcla el fotograma previo (alineado con `d`) sobre el actual. Con `gate`,
/// solo donde la diferencia es pequeña (evita fantasmas en zonas con movimiento).
fn temporal_blend(buf: &mut [u8], prev: &[u8], w: usize, h: usize, d: (f32, f32), alpha: f32, gate: bool) {
    if prev.len() != buf.len() {
        return;
    }
    let (dx, dy) = (d.0.round() as i32, d.1.round() as i32);
    buf.par_chunks_mut(w * 4).enumerate().for_each(|(y, row)| {
        for (x, px) in row.chunks_exact_mut(4).enumerate() {
            let i = px_at(prev, w, h, x as i32 + dx, y as i32 + dy);
            let mut a = alpha;
            if gate {
                let diff = (0..3).map(|c| (px[c] as i32 - prev[i + c] as i32).abs()).sum::<i32>() as f32 / 3.0;
                a *= (1.0 - diff / 40.0).clamp(0.0, 1.0);
            }
            for c in 0..3 {
                px[c] = (px[c] as f32 * (1.0 - a) + prev[i + c] as f32 * a).round() as u8;
            }
        }
    });
}

/// Desenfoque direccional a lo largo del movimiento (mínimo horizontal visible).
fn directional_blur(buf: &mut [u8], w: usize, h: usize, d: (f32, f32)) {
    let mag = (d.0 * d.0 + d.1 * d.1).sqrt();
    let (dir, len) = if mag > 0.5 { ((d.0 / mag, d.1 / mag), (mag * 2.0).clamp(3.0, 28.0)) } else { ((1.0, 0.0), 3.0) };
    let src = buf.to_vec();
    const TAPS: i32 = 9;
    buf.par_chunks_mut(w * 4).enumerate().for_each(|(y, row)| {
        for (x, px) in row.chunks_exact_mut(4).enumerate() {
            let mut acc = [0f32; 3];
            for t in 0..TAPS {
                let o = (t as f32 / (TAPS - 1) as f32 - 0.5) * len;
                let i = px_at(&src, w, h, (x as f32 + dir.0 * o).round() as i32, (y as f32 + dir.1 * o).round() as i32);
                for c in 0..3 {
                    acc[c] += src[i + c] as f32;
                }
            }
            for c in 0..3 {
                px[c] = (acc[c] / TAPS as f32).round() as u8;
            }
        }
    });
}

/// Filtro bilateral 3×3 (suaviza el ruido conservando los bordes).
fn bilateral3(buf: &mut [u8], w: usize, h: usize) {
    let src = buf.to_vec();
    let lut: Vec<f32> = (0..256).map(|d| (-(d as f32).powi(2) / (2.0 * 22.0f32.powi(2))).exp()).collect();
    buf.par_chunks_mut(w * 4).enumerate().for_each(|(y, row)| {
        for (x, px) in row.chunks_exact_mut(4).enumerate() {
            let (cx, cy) = (x as i32, y as i32);
            let mut acc = [0f32; 3];
            let mut wsum = 0f32;
            for oy in -1..=1 {
                for ox in -1..=1 {
                    let i = px_at(&src, w, h, cx + ox, cy + oy);
                    let diff = (0..3).map(|c| (px[c] as i32 - src[i + c] as i32).abs()).sum::<i32>() / 3;
                    let wt = lut[diff.min(255) as usize];
                    wsum += wt;
                    for c in 0..3 {
                        acc[c] += src[i + c] as f32 * wt;
                    }
                }
            }
            for c in 0..3 {
                px[c] = (acc[c] / wsum).round() as u8;
            }
        }
    });
}

/// Zoom centrado en `(w/2+ox, h/2+oy)` con interpolación bilineal.
fn resample(src: &[u8], w: usize, h: usize, zoom: f32, ox: f32, oy: f32) -> Vec<u8> {
    let mut out = vec![0u8; src.len()];
    let (cx, cy) = (w as f32 / 2.0 + ox, h as f32 / 2.0 + oy);
    out.par_chunks_mut(w * 4).enumerate().for_each(|(y, row)| {
        let v = ((y as f32 + 0.5 - h as f32 / 2.0) / zoom + cy - 0.5).clamp(0.0, h as f32 - 1.0);
        let (y0, fy) = (v.floor() as usize, v.fract());
        let y1 = (y0 + 1).min(h - 1);
        for (x, px) in row.chunks_exact_mut(4).enumerate() {
            let u = ((x as f32 + 0.5 - w as f32 / 2.0) / zoom + cx - 0.5).clamp(0.0, w as f32 - 1.0);
            let (x0, fx) = (u.floor() as usize, u.fract());
            let x1 = (x0 + 1).min(w - 1);
            for c in 0..4 {
                let a = src[(y0 * w + x0) * 4 + c] as f32 * (1.0 - fx) + src[(y0 * w + x1) * 4 + c] as f32 * fx;
                let b = src[(y1 * w + x0) * 4 + c] as f32 * (1.0 - fx) + src[(y1 * w + x1) * 4 + c] as f32 * fx;
                px[c] = (a * (1.0 - fy) + b * fy).round() as u8;
            }
        }
    });
    out
}

/// Máscara de enfoque con media 3×3.
fn unsharp(buf: &mut [u8], w: usize, h: usize, amount: f32) {
    let src = buf.to_vec();
    buf.par_chunks_mut(w * 4).enumerate().for_each(|(y, row)| {
        for (x, px) in row.chunks_exact_mut(4).enumerate() {
            let mut acc = [0f32; 3];
            for oy in -1..=1 {
                for ox in -1..=1 {
                    let i = px_at(&src, w, h, x as i32 + ox, y as i32 + oy);
                    for c in 0..3 {
                        acc[c] += src[i + c] as f32;
                    }
                }
            }
            for c in 0..3 {
                let mean = acc[c] / 9.0;
                px[c] = (px[c] as f32 + amount * (px[c] as f32 - mean)).round().clamp(0.0, 255.0) as u8;
            }
        }
    });
}

/// Luz suave radial con realce de sombras.
fn add_light(buf: &mut [u8], w: usize, h: usize) {
    buf.par_chunks_mut(w * 4).enumerate().for_each(|(y, row)| {
        let ny = (y as f32 / h as f32 - 0.42) * 2.0;
        for (x, px) in row.chunks_exact_mut(4).enumerate() {
            let nx = (x as f32 / w as f32 - 0.5) * 2.0;
            let fall = (1.0 - (nx * nx + ny * ny) / 2.0).clamp(0.0, 1.0);
            let gain = 1.0 + 0.28 * fall;
            let lift = 10.0 * fall;
            for c in 0..3 {
                px[c] = (px[c] as f32 * gain + lift).round().clamp(0.0, 255.0) as u8;
            }
        }
    });
}
