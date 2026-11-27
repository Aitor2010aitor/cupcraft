//! Motor de efectos real por píxel (CPU, paralelo con rayon).
//!
//! Sustituye a las aproximaciones de color/desenfoque de `fx_library` por
//! algoritmos de verdad para la mayoría del catálogo de AbsentMotion:
//! remapeos geométricos, desenfoques direccionales/radiales, bordes, brillos,
//! tramas, ruido procedural, claves (alfa) y sólidos 3D proyectados.
//!
//! Toda función recibe un fotograma RGBA sin premultiplicar (`src`, `w`×`h`) y
//! devuelve otro del mismo tamaño. `a` es la cantidad (0..1) y `d` el segundo
//! parámetro del efecto (`detail`: ángulo, divisiones o tamaño según el efecto).
//! La vista previa y la exportación llaman a la misma [`run`], así que lo que se
//! ve es lo que se exporta.

use rayon::prelude::*;
use std::f32::consts::{PI, TAU};

pub use crate::fx_table::{index_of, op_at};

/// Operación del motor. Cada variante la fija la tabla `fx_table::TABLE`
/// (varios ids de la biblioteca pueden compartir operación).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Op {
    // ---- Desenfoques ----
    Blur, InnerBlur, Directional, Motion, Smear, WarpBlur, ZoomBlur, ChromZoom,
    ZoomStreak, SpinStreak, LinStreak, StreakStrips, SpinBlur, VortexBlur, LensBlur,
    MaskBlur, FillBehind, Denoise, TrueMotionBlur,
    // ---- Distorsión / movimiento / repetición (remapeo de coordenadas) ----
    Puppet, InnerPinch, NoiseDisp, TurbDisp, FractalWarp, Tiles, HexTiles, TileRot,
    TileShift, HexRot, HexShift, Squeeze, View360, Reorient, StretchSeg, Bend, Curl,
    Magnify, Glass, DispMap, PolarDisp, OminoGlass, LiquidGlass, PageCurl,
    Shake, Jitter, Oscillate, Swing, Spin, PulseSize, StretchAxis, Offset, Raster,
    MovePath, Echo, RepTile, RepLinear, RepRadial, RepScatter, RepField,
    RepPath, Feedback, Spherize, Tunnel, FlipLayer, BeatZoom,
    // ---- Estilizar ----
    Glitch, Cartoon, OminoDiff, HexArray, Outline, Glow, LightGlow, SoftGlow, DeepGlow,
    InnerGlow, EdgeGlow, DarkGlow, ContourLines, ContourGrad, ElectricEdges, ContourStrips,
    GlowScan, Ribbon, FindEdges, Emboss, Bevel, BumpMap, Roughen, HalftoneDots,
    HalftoneLines, CmykDots, Ascii, LensFlare, LongShadow, DropShadow, RadialShadow,
    PixelTiles, RgbSplit, Scanlines, Stripes, Voronoi, StarMask, Checker, GridOv,
    DotsOv, RadialRays, Lightning, EnergyBeam, Iridescence, GodRays, Starfield,
    StarfieldSimple, Particles, Sharpen, Unsharp, Vignette, Grunge, Blink, Flicker,
    Flash, Vhs, Datamosh, DrawProgress, StrokeTaper,
    // ---- Color ----
    SolidTint, GradOver, Grad4, GradMap, Palette, Dither, SpotColor, ReplaceColor,
    RemapRgb, RemapHsv, Mixer, Difference,
    // ---- Ruido ----
    FilmGrain, Turbulence, Clouds, FractalRidges, Dissolve, BlockDissolve, BlockNoise,
    NoiseOv, StaticTv,
    // ---- Claves (alfa) ----
    Chroma, Spill, LumaKey, DiffMatte, LumaStamp, SolidMatte, Crop, Wipe, Feather,
    RadialWipe, Choker, Fringe, SmoothEdges,
    // ---- Sólidos 3D ----
    Cube, Box, HollowBox, Cylinder, Ellipsoid, Torus, Pyramid, Octahedron, HexPrism,
    StarPrism, StarPoly, Heart, Cross3, Fracture, RasterExtrude, Ring, Scatter3,
    // ---- Añadidos: color, opacidad animada, trazo ----
    HotColor, SpectralMap, StrokeColor, Fade, PulseOpacity, AdvChroma,
}

// ---------------------------------------------------------------------------
// Utilidades de imagen
// ---------------------------------------------------------------------------

pub struct Img<'a> {
    pub d: &'a [u8],
    pub w: usize,
    pub h: usize,
}

impl<'a> Img<'a> {
    #[inline]
    pub fn at(&self, x: i32, y: i32) -> [f32; 4] {
        let x = x.clamp(0, self.w as i32 - 1) as usize;
        let y = y.clamp(0, self.h as i32 - 1) as usize;
        let i = (y * self.w + x) * 4;
        [self.d[i] as f32, self.d[i + 1] as f32, self.d[i + 2] as f32, self.d[i + 3] as f32]
    }

    #[inline]
    pub fn bil(&self, fx: f32, fy: f32) -> [f32; 4] {
        let fx = fx.clamp(0.0, (self.w - 1) as f32);
        let fy = fy.clamp(0.0, (self.h - 1) as f32);
        let x0 = fx.floor() as i32;
        let y0 = fy.floor() as i32;
        let dx = fx - x0 as f32;
        let dy = fy - y0 as f32;
        let p00 = self.at(x0, y0);
        let p10 = self.at(x0 + 1, y0);
        let p01 = self.at(x0, y0 + 1);
        let p11 = self.at(x0 + 1, y0 + 1);
        let mut o = [0.0; 4];
        for c in 0..4 {
            let t = p00[c] * (1.0 - dx) + p10[c] * dx;
            let b = p01[c] * (1.0 - dx) + p11[c] * dx;
            o[c] = t * (1.0 - dy) + b * dy;
        }
        o
    }

    #[inline]
    pub fn luma(&self, x: i32, y: i32) -> f32 {
        let p = self.at(x, y);
        (0.299 * p[0] + 0.587 * p[1] + 0.114 * p[2]) / 255.0
    }
}

#[inline]
pub fn cl(v: f32) -> u8 {
    (v + 0.5).clamp(0.0, 255.0) as u8
}

/// Genera un fotograma evaluando `f(x, y)` por píxel (filas en paralelo).
pub fn gen<F: Fn(usize, usize) -> [f32; 4] + Sync>(w: usize, h: usize, f: F) -> Vec<u8> {
    let mut dst = vec![0u8; w * h * 4];
    dst.par_chunks_mut(w * 4).enumerate().for_each(|(y, row)| {
        for x in 0..w {
            let p = f(x, y);
            let o = x * 4;
            row[o] = cl(p[0]);
            row[o + 1] = cl(p[1]);
            row[o + 2] = cl(p[2]);
            row[o + 3] = cl(p[3]);
        }
    });
    dst
}

#[inline]
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

#[inline]
pub fn mix4(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    [lerp(a[0], b[0], t), lerp(a[1], b[1], t), lerp(a[2], b[2], t), lerp(a[3], b[3], t)]
}

#[inline]
pub fn smooth(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Hash entero 2D → 0..1.
#[inline]
pub fn h2(x: i32, y: i32) -> f32 {
    let mut n = (x as u32).wrapping_mul(374761393).wrapping_add((y as u32).wrapping_mul(668265263));
    n = (n ^ (n >> 13)).wrapping_mul(1274126177);
    n ^= n >> 16;
    (n & 0x00FF_FFFF) as f32 / 16_777_215.0
}

pub fn vnoise(x: f32, y: f32) -> f32 {
    let xi = x.floor();
    let yi = y.floor();
    let fx = x - xi;
    let fy = y - yi;
    let (xi, yi) = (xi as i32, yi as i32);
    let sx = fx * fx * (3.0 - 2.0 * fx);
    let sy = fy * fy * (3.0 - 2.0 * fy);
    let a = lerp(h2(xi, yi), h2(xi + 1, yi), sx);
    let b = lerp(h2(xi, yi + 1), h2(xi + 1, yi + 1), sx);
    lerp(a, b, sy)
}

pub fn fbm(x: f32, y: f32, oct: u32) -> f32 {
    let (mut s, mut amp, mut f, mut norm) = (0.0, 0.5, 1.0, 0.0);
    for _ in 0..oct {
        s += amp * vnoise(x * f, y * f);
        norm += amp;
        amp *= 0.5;
        f *= 2.0;
    }
    s / norm
}

/// HSV (h en grados) → RGB 0..255.
pub fn hsv(h: f32, s: f32, v: f32) -> [f32; 3] {
    let h = h.rem_euclid(360.0) / 60.0;
    let i = h.floor();
    let f = h - i;
    let (p, q, t) = (v * (1.0 - s), v * (1.0 - s * f), v * (1.0 - s * (1.0 - f)));
    let (r, g, b) = match i as i32 {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };
    [r * 255.0, g * 255.0, b * 255.0]
}

/// RGB 0..255 → (h grados, s, v).
pub fn to_hsv(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    let (r, g, b) = (r / 255.0, g / 255.0, b / 255.0);
    let mx = r.max(g).max(b);
    let mn = r.min(g).min(b);
    let dl = mx - mn;
    let h = if dl < 1e-6 {
        0.0
    } else if mx == r {
        60.0 * ((g - b) / dl).rem_euclid(6.0)
    } else if mx == g {
        60.0 * ((b - r) / dl + 2.0)
    } else {
        60.0 * ((r - g) / dl + 4.0)
    };
    (h, if mx <= 0.0 { 0.0 } else { dl / mx }, mx)
}

// ---------------------------------------------------------------------------
// Desenfoque de caja (3 pasadas ≈ gaussiano), transposición y mediana
// ---------------------------------------------------------------------------

fn box_h(src: &[u8], w: usize, r: usize) -> Vec<u8> {
    if r == 0 {
        return src.to_vec();
    }
    let mut dst = vec![0u8; src.len()];
    let n = (2 * r + 1) as u32;
    dst.par_chunks_mut(w * 4).zip(src.par_chunks(w * 4)).for_each(|(o, s)| {
        let mut acc = [0u32; 4];
        for k in -(r as i32)..=(r as i32) {
            let x = k.clamp(0, w as i32 - 1) as usize;
            for c in 0..4 {
                acc[c] += s[x * 4 + c] as u32;
            }
        }
        for x in 0..w {
            for c in 0..4 {
                o[x * 4 + c] = (acc[c] / n) as u8;
            }
            let add = (x as i32 + r as i32 + 1).min(w as i32 - 1) as usize;
            let sub = (x as i32 - r as i32).max(0) as usize;
            for c in 0..4 {
                acc[c] = acc[c] + s[add * 4 + c] as u32 - s[sub * 4 + c] as u32;
            }
        }
    });
    dst
}

fn transpose(src: &[u8], w: usize, h: usize) -> Vec<u8> {
    let mut dst = vec![0u8; src.len()];
    dst.par_chunks_mut(h * 4).enumerate().for_each(|(x, col)| {
        for y in 0..h {
            let i = (y * w + x) * 4;
            col[y * 4..y * 4 + 4].copy_from_slice(&src[i..i + 4]);
        }
    });
    dst
}

/// Desenfoque ~gaussiano de radio `r` px.
pub fn blur(src: &[u8], w: usize, h: usize, r: usize) -> Vec<u8> {
    if r == 0 {
        return src.to_vec();
    }
    let mut cur = src.to_vec();
    for _ in 0..2 {
        cur = box_h(&cur, w, r);
        let t = transpose(&cur, w, h);
        let t = box_h(&t, h, r);
        cur = transpose(&t, h, w);
    }
    cur
}

fn rad(w: usize, a: f32, frac: f32) -> usize {
    (a * frac * w as f32).round().max(0.0) as usize
}

// ---------------------------------------------------------------------------
// Punto de entrada
// ---------------------------------------------------------------------------

pub fn run(op: Op, src: &[u8], w: usize, h: usize, a: f32, d: f32) -> Vec<u8> {
    run_t(op, src, w, h, a, d, 0.0, 0.0)
}

/// Igual que [`run`] con el tiempo local del clip `t` y su duración `dur`
/// (segundos), para los efectos animados.
#[allow(clippy::too_many_arguments)]
pub fn run_t(op: Op, src: &[u8], w: usize, h: usize, a: f32, d: f32, t: f32, dur: f32) -> Vec<u8> {
    if w == 0 || h == 0 || src.len() < w * h * 4 {
        return src.to_vec();
    }
    let img = Img { d: src, w, h };
    let p = P { w: w as f32, h: h as f32, cx: w as f32 * 0.5, cy: h as f32 * 0.5, a, d, rmax: ((w * w + h * h) as f32).sqrt() * 0.5, m: w.min(h) as f32, t, dur };
    if let Some(f) = remap_fn(op) {
        return gen(w, h, |x, y| {
            let (sx, sy) = f(x as f32 + 0.5, y as f32 + 0.5, &p, &img);
            img.bil(sx - 0.5, sy - 0.5)
        });
    }
    match op {
        // ---- desenfoques ----
        Op::Blur => blur(src, w, h, rad(w, a, 0.03)),
        Op::InnerBlur | Op::MaskBlur | Op::FillBehind | Op::LensBlur | Op::Denoise
        | Op::Directional | Op::Motion | Op::Smear | Op::WarpBlur | Op::ZoomBlur | Op::ChromZoom
        | Op::ZoomStreak | Op::SpinStreak | Op::LinStreak | Op::StreakStrips | Op::SpinBlur
        | Op::VortexBlur | Op::TrueMotionBlur => blur_ops(op, &img, &p),
        _ if crate::fx_extra::handles(op) => crate::fx_extra::run(op, &img, &p),
        _ => crate::fx_style::run(op, &img, &p),
    }
}

/// Operaciones cuyo resultado depende del tiempo (invalidan la caché por fotograma).
pub fn animated(op: Op) -> bool {
    matches!(
        op,
        Op::Shake | Op::Jitter | Op::Oscillate | Op::Swing | Op::Spin | Op::PulseSize | Op::MovePath
            | Op::NoiseDisp | Op::TurbDisp | Op::FractalWarp | Op::Tunnel | Op::Glitch | Op::Datamosh
            | Op::Flicker | Op::Blink | Op::FilmGrain | Op::NoiseOv | Op::StaticTv | Op::GlowScan
            | Op::Starfield | Op::StarfieldSimple | Op::Particles | Op::Lightning | Op::EnergyBeam
            | Op::Iridescence | Op::Scanlines | Op::Turbulence | Op::Clouds | Op::FractalRidges
            | Op::BlockNoise | Op::Fade | Op::PulseOpacity | Op::Echo | Op::Feedback | Op::Vhs
            | Op::OminoDiff | Op::Flash
    )
}

/// Parámetros resueltos de una ejecución.
pub struct P {
    pub w: f32,
    pub h: f32,
    pub cx: f32,
    pub cy: f32,
    pub a: f32,
    pub d: f32,
    pub rmax: f32,
    /// Lado menor.
    pub m: f32,
    /// Tiempo local del clip (s).
    pub t: f32,
    /// Duración del clip (s); 0 = desconocida.
    pub dur: f32,
}

type MapFn = fn(f32, f32, &P, &Img) -> (f32, f32);

fn rot(x: f32, y: f32, ang: f32) -> (f32, f32) {
    let (s, c) = ang.sin_cos();
    (x * c - y * s, x * s + y * c)
}

/// Celda hexagonal (apuntando hacia arriba) de lado `s`: devuelve el centro.
fn hex_center(x: f32, y: f32, s: f32) -> (f32, f32) {
    let hh = s * 3.0f32.sqrt();
    let w2 = s * 1.5;
    let q = x / w2;
    let qi = q.floor();
    let off = if (qi as i32) & 1 == 0 { 0.0 } else { hh * 0.5 };
    // Dos candidatos (columna actual y vecina) → el más cercano.
    let cand = |ci: f32| {
        let o = if (ci as i32) & 1 == 0 { 0.0 } else { hh * 0.5 };
        let cx = (ci + 0.5) * w2;
        let cyi = ((y - o) / hh).floor();
        (cx, (cyi + 0.5) * hh + o)
    };
    let _ = off;
    let c0 = cand(qi);
    let c1 = cand(qi + 1.0);
    let c2 = cand(qi - 1.0);
    let dd = |c: (f32, f32)| (c.0 - x).powi(2) + (c.1 - y).powi(2);
    let mut best = c0;
    for c in [c1, c2] {
        if dd(c) < dd(best) {
            best = c;
        }
    }
    best
}

fn remap_fn(op: Op) -> Option<MapFn> {
    Some(match op {
        Op::InnerPinch => |x, y, p, _| {
            let (dx, dy) = (x - p.cx, y - p.cy);
            let r0 = p.m * 0.42;
            let r = dx.hypot(dy);
            if r >= r0 || r < 0.001 {
                return (x, y);
            }
            let t = r / r0;
            let nr = r0 * t.powf(1.0 + p.a * 1.6);
            (p.cx + dx * nr / r, p.cy + dy * nr / r)
        },
        Op::Puppet => |x, y, p, _| {
            // Tres alfileres que arrastran el contenido con caída gaussiana.
            let pins = [(0.3f32, 0.4f32, 0.18f32, -0.1f32), (0.7, 0.45, -0.15, 0.12), (0.5, 0.75, 0.0, -0.14)];
            let (mut sx, mut sy) = (x, y);
            for (px, py, mx, my) in pins {
                let (dx, dy) = (x - px * p.w, y - py * p.h);
                let g = (-(dx * dx + dy * dy) / (2.0 * (p.m * 0.22).powi(2))).exp();
                sx -= mx * p.w * p.a * g;
                sy -= my * p.h * p.a * g;
            }
            (sx, sy)
        },
        Op::NoiseDisp => |x, y, p, _| {
            let s = p.w / (p.d.max(1.0) * 1.5 + 2.0);
            let tt = p.t * 0.6;
            let (nx, ny) = (vnoise(x / s + tt, y / s), vnoise(x / s + 41.0, y / s + 17.0 + tt));
            (x + (nx - 0.5) * p.a * 0.22 * p.w, y + (ny - 0.5) * p.a * 0.22 * p.w)
        },
        Op::TurbDisp => |x, y, p, _| {
            let s = p.w / (p.d.max(1.0) * 1.2 + 2.0);
            let tt = p.t * 0.5;
            let (nx, ny) = (fbm(x / s + tt, y / s, 5), fbm(x / s + 7.3, y / s + 3.1 + tt, 5));
            (x + (nx - 0.5) * p.a * 0.35 * p.w, y + (ny - 0.5) * p.a * 0.35 * p.w)
        },
        Op::FractalWarp => |x, y, p, _| {
            let s = p.w / (p.d.max(1.0) * 0.8 + 2.0);
            let tt = p.t * 0.25;
            let (qx, qy) = (fbm(x / s + tt, y / s, 4), fbm(x / s + 5.2, y / s + 1.3 - tt, 4));
            let (nx, ny) = (fbm(x / s + 4.0 * qx, y / s + 4.0 * qy, 5), fbm(x / s + 4.0 * qy + 8.3, y / s + 4.0 * qx + 2.8, 5));
            (x + (nx - 0.5) * p.a * 0.45 * p.w, y + (ny - 0.5) * p.a * 0.45 * p.w)
        },
        Op::Tiles | Op::RepTile => |x, y, p, _| {
            let n = 1.0 + p.a * p.d.max(1.0);
            ((x / p.w * n).rem_euclid(1.0) * p.w, (y / p.h * n).rem_euclid(1.0) * p.h)
        },
        Op::RepLinear => |x, y, p, _| {
            let n = 1.0 + p.a * p.d.max(1.0);
            ((x / p.w * n).rem_euclid(1.0) * p.w, y)
        },
        Op::TileRot => |x, y, p, _| {
            let n = p.d.max(2.0);
            let (cw, ch) = (p.w / n, p.h / n);
            let (ix, iy) = ((x / cw).floor(), (y / ch).floor());
            let (cxx, cyy) = ((ix + 0.5) * cw, (iy + 0.5) * ch);
            let ang = (h2(ix as i32, iy as i32) - 0.5) * p.a * TAU * 0.5;
            let (rx, ry) = rot(x - cxx, y - cyy, ang);
            (cxx + rx, cyy + ry)
        },
        Op::TileShift => |x, y, p, _| {
            let n = p.d.max(2.0);
            let ch = p.h / n;
            let iy = (y / ch).floor();
            (x + (h2(iy as i32, 3) - 0.5) * p.a * p.w * 0.6, y)
        },
        Op::HexTiles => |x, y, p, _| {
            let s = p.w / (p.d.max(2.0) * 1.5);
            let (hx, hy) = hex_center(x, y, s);
            let n = p.a;
            // Cada celda muestra el contenido del centro de la imagen escalado.
            let k = lerp(1.0, 0.25, n);
            (p.cx + (x - hx) * 1.0 * k + (hx - p.cx) * 0.0, p.cy + (y - hy) * 1.0 * k)
                .pipe(|(sx, sy)| (lerp(x, sx, n), lerp(y, sy, n)))
        },
        Op::HexRot => |x, y, p, _| {
            let s = p.w / (p.d.max(2.0) * 1.5);
            let (hx, hy) = hex_center(x, y, s);
            let ang = (h2((hx / s) as i32, (hy / s) as i32) - 0.5) * p.a * TAU * 0.5;
            let (rx, ry) = rot(x - hx, y - hy, ang);
            (hx + rx, hy + ry)
        },
        Op::HexShift => |x, y, p, _| {
            let s = p.w / (p.d.max(2.0) * 1.5);
            let (hx, hy) = hex_center(x, y, s);
            let (i, j) = ((hx / s) as i32, (hy / s) as i32);
            (x + (h2(i, j) - 0.5) * p.a * s * 2.0, y + (h2(j, i + 9) - 0.5) * p.a * s * 2.0)
        },
        Op::Squeeze => |x, y, p, _| {
            let t = (x - p.cx) / p.cx;
            let s = t.signum() * t.abs().powf(1.0 + p.a * 1.6);
            (p.cx + s * p.cx, y)
        },
        Op::StretchSeg => |x, y, p, _| {
            let t = (x - p.cx) / p.cx;
            let sin = 1.0 - p.a * 0.85;
            let at = t.abs();
            let ts = if at < 0.5 { at * sin } else { 0.5 * sin + (at - 0.5) * (1.0 - 0.5 * sin) / 0.5 };
            (p.cx + t.signum() * ts * p.cx, y)
        },
        Op::View360 => |x, y, p, _| {
            let fov = (2.2 - p.a * 1.2).max(0.6);
            let (u, v) = ((x - p.cx) / p.cx * fov * 0.5, (y - p.cy) / p.cx * fov * 0.5);
            let (dx, dz) = rot(u, 1.0, (p.d / 20.0) * TAU);
            let lon = dx.atan2(dz);
            let lat = (-v / (u * u + v * v + 1.0).sqrt()).asin();
            ((lon / TAU + 0.5).rem_euclid(1.0) * p.w, (0.5 - lat / PI) * p.h)
        },
        Op::Reorient => |x, y, p, _| {
            let lon = (x / p.w - 0.5) * TAU;
            let lat = (0.5 - y / p.h) * PI;
            let (vx, vy, vz) = (lat.cos() * lon.sin(), lat.sin(), lat.cos() * lon.cos());
            let (vx, vz) = rot(vx, vz, p.a * TAU);
            let (vy, vz) = rot(vy, vz, (p.d - 1.0) / 19.0 * PI * 0.5);
            let lon2 = vx.atan2(vz);
            let lat2 = vy.clamp(-1.0, 1.0).asin();
            ((lon2 / TAU + 0.5).rem_euclid(1.0) * p.w, (0.5 - lat2 / PI) * p.h)
        },
        Op::Bend => |x, y, p, _| {
            let t = (x - p.cx) / p.cx;
            (x, y + p.a * p.h * 0.4 * (1.0 - t * t) - p.a * p.h * 0.2)
        },
        Op::Curl => |x, y, p, _| {
            // Remolino con centro en la esquina inferior derecha.
            let (ox, oy) = (p.w, p.h);
            let (dx, dy) = (x - ox, y - oy);
            let r = dx.hypot(dy);
            let f = (1.0 - r / (p.rmax * 1.6)).clamp(0.0, 1.0);
            let (rx, ry) = rot(dx, dy, p.a * 2.6 * f * f);
            (ox + rx, oy + ry)
        },
        Op::Magnify => |x, y, p, _| {
            let (dx, dy) = (x - p.cx, y - p.cy);
            let r0 = p.m * 0.38;
            let r = dx.hypot(dy);
            if r >= r0 {
                return (x, y);
            }
            let t = r / r0;
            let k = 1.0 / (1.0 + p.a * 1.8 * (1.0 - t * t));
            (p.cx + dx * k, p.cy + dy * k)
        },
        Op::Glass => |x, y, p, _| {
            let n = p.d.max(2.0) + 2.0;
            let cell = p.w / n;
            let (lx, ly) = ((x / cell).rem_euclid(1.0) - 0.5, (y / cell).rem_euclid(1.0) - 0.5);
            (x + lx * cell * p.a * 1.6, y + ly * cell * p.a * 1.6)
        },
        Op::OminoGlass => |x, y, p, _| {
            let f = (p.d.max(1.0) + 2.0) / p.w * TAU;
            let g = p.a * p.w * 0.03;
            (x + (y * f).sin() * g + (x * f * 0.5).cos() * g * 0.5, y + (x * f).sin() * g + (y * f * 0.5).cos() * g * 0.5)
        },
        Op::LiquidGlass => |x, y, p, _| {
            let f = (p.d.max(1.0) + 2.0) / p.w * TAU * 0.5;
            let g = p.a * p.w * 0.04;
            let u = (x * f + (y * f * 1.3).sin() * 2.0).sin();
            let v = (y * f + (x * f * 1.1).sin() * 2.0).cos();
            (x + u * g, y + v * g)
        },
        Op::DispMap => |x, y, p, img| {
            let c = img.at(x as i32, y as i32);
            (x + (c[0] / 255.0 - 0.5) * p.a * 0.25 * p.w, y + (c[1] / 255.0 - 0.5) * p.a * 0.25 * p.w)
        },
        Op::PolarDisp => |x, y, p, img| {
            let (dx, dy) = (x - p.cx, y - p.cy);
            let r = dx.hypot(dy).max(0.001);
            let l = img.luma(x as i32, y as i32);
            let nr = r + (l - 0.5) * p.a * 0.35 * p.rmax;
            (p.cx + dx / r * nr, p.cy + dy / r * nr)
        },
        Op::Shake => |x, y, p, _| {
            // Sacudida: ruido suave en el tiempo; `detalle` = frecuencia.
            let f = p.d.max(1.0) * 0.9;
            let (sx, sy) = (vnoise(p.t * f, 3.7) - 0.5, vnoise(11.3, p.t * f) - 0.5);
            let (sx, sy) = if p.t == 0.0 { (h2(p.d as i32, 1) - 0.5, h2(p.d as i32, 2) - 0.5) } else { (sx * 2.0, sy * 2.0) };
            let ang = (vnoise(p.t * f * 0.7, 27.1) - 0.5) * p.a * 0.12;
            let (rx, ry) = rot(x - p.cx, y - p.cy, ang);
            (p.cx + rx + sx * p.a * 0.12 * p.w, p.cy + ry + sy * p.a * 0.12 * p.w)
        },
        Op::Jitter => |x, y, p, _| {
            // Vibración por línea de escaneo, nueva en cada fotograma.
            let band = (y / (p.h / 60.0)).floor() as i32;
            let fr = (p.t * p.d.max(1.0) * 3.0).floor() as i32;
            (x + (h2(band + fr * 61, p.d as i32) - 0.5) * p.a * 0.08 * p.w, y)
        },
        Op::Oscillate => |x, y, p, _| {
            // Vaivén senoidal; `detalle` = ciclos cada 8 s.
            let s = (p.t * TAU * p.d.max(1.0) / 8.0 + PI * 0.5).sin();
            (x - p.a * 0.2 * p.w * s, y - p.a * 0.06 * p.h * (p.t * TAU * p.d.max(1.0) / 4.0).sin())
        },
        Op::Swing => |x, y, p, _| {
            let (rx, ry) = rot(x - p.cx, y - p.cy, p.a * PI * 0.25 * (p.t * TAU * 0.6).cos());
            (p.cx + rx, p.cy + ry)
        },
        Op::Spin => |x, y, p, _| {
            // Giro continuo: `cantidad` = vueltas por 2 s (más un desfase visible en t=0).
            let (rx, ry) = rot(x - p.cx, y - p.cy, p.a * (PI * 0.25 + TAU * 0.5 * p.t));
            (p.cx + rx, p.cy + ry)
        },
        Op::PulseSize => |x, y, p, _| {
            let k = 1.0 / (1.0 + p.a * 0.6 * (0.5 + 0.5 * (p.t * TAU * 1.2).cos()));
            (p.cx + (x - p.cx) * k, p.cy + (y - p.cy) * k)
        },
        Op::BeatZoom => |x, y, p, _| {
            let k = 1.0 / (1.0 + p.a * 0.5);
            (p.cx + (x - p.cx) * k, p.cy + (y - p.cy) * k)
        },
        Op::StretchAxis => |x, y, p, _| {
            let k = 1.0 / (1.0 + p.a * p.d.max(1.0) * 0.2);
            (p.cx + (x - p.cx) * k, y)
        },
        Op::Offset => |x, y, p, _| ((x + p.a * p.w).rem_euclid(p.w), (y + (p.d - 1.0) / 19.0 * p.a * p.h).rem_euclid(p.h)),
        Op::Raster => |x, y, p, _| {
            let (rx, ry) = rot(x - p.cx, y - p.cy, p.a * PI / 6.0);
            let k = 1.0 / (1.0 - p.a * 0.3);
            (p.cx + rx * k, p.cy + ry * k)
        },
        Op::MovePath => |x, y, p, _| {
            // Recorre una curva en ocho a lo largo del clip.
            let u = if p.dur > 0.0 { (p.t / p.dur).clamp(0.0, 1.0) } else { 0.25 } * TAU;
            (x - p.a * 0.25 * p.w * u.sin(), y - p.a * 0.18 * p.h * (2.0 * u).sin())
        },
        Op::RepRadial => |x, y, p, _| {
            let n = (1.0 + p.a * p.d.max(2.0)).round().max(1.0);
            let (dx, dy) = (x - p.cx, y - p.cy);
            let seg = TAU / n;
            let ang = dy.atan2(dx);
            let k = (ang / seg).floor();
            let (rx, ry) = rot(dx, dy, -k * seg);
            (p.cx + rx, p.cy + ry)
        },
        Op::RepScatter => |x, y, p, _| {
            let n = p.d.max(2.0) * 0.6;
            let (cw, ch) = (p.w / n, p.h / n);
            let (ix, iy) = ((x / cw).floor(), (y / ch).floor());
            let (jx, jy) = (h2(ix as i32, iy as i32) - 0.5, h2(iy as i32 + 5, ix as i32) - 0.5);
            let (lx, ly) = ((x / cw).rem_euclid(1.0), (y / ch).rem_euclid(1.0));
            let _ = (lx, ly);
            (((x + jx * p.a * cw) / cw).rem_euclid(1.0) * p.w, ((y + jy * p.a * ch) / ch).rem_euclid(1.0) * p.h)
        },
        Op::RepField => |x, y, p, _| {
            let n = p.d.max(2.0) * 0.5;
            let (cw, ch) = (p.w / n, p.h / n);
            let (ix, iy) = ((x / cw).floor(), (y / ch).floor());
            let s = 0.5 + h2(ix as i32 + 3, iy as i32 + 1) * 1.2 * p.a;
            let (lx, ly) = ((x / cw).rem_euclid(1.0) - 0.5, (y / ch).rem_euclid(1.0) - 0.5);
            (p.cx + lx / s * p.w, p.cy + ly / s * p.h)
        },
        Op::RepPath => |x, y, p, _| {
            let n = 1.0 + p.a * p.d.max(2.0) * 0.5;
            let yy = y + (x / p.w * TAU * 1.5).sin() * p.h * 0.08 * p.a;
            ((x / p.w * n).rem_euclid(1.0) * p.w, (yy / p.h * n).rem_euclid(1.0) * p.h)
        },
        Op::Spherize => |x, y, p, _| {
            let (dx, dy) = (x - p.cx, y - p.cy);
            let r0 = p.m * 0.5;
            let r = dx.hypot(dy);
            if r >= r0 || r < 0.001 {
                return (x, y);
            }
            let t = r / r0;
            let nr = (t.asin() / (PI * 0.5)) * r0;
            let k = lerp(r, nr, p.a) / r;
            (p.cx + dx * k, p.cy + dy * k)
        },
        Op::Tunnel => |x, y, p, _| {
            let (dx, dy) = (x - p.cx, y - p.cy);
            let r = dx.hypot(dy).max(1.0) / p.rmax;
            let ang = dy.atan2(dx) / TAU + 0.5;
            let tx = (ang * p.d.max(1.0) * 0.5).rem_euclid(1.0) * p.w;
            let ty = ((0.3 / r) * 0.5 + p.t * 0.4).rem_euclid(1.0) * p.h;
            (lerp(x, tx, p.a), lerp(y, ty, p.a))
        },
        Op::FlipLayer => |x, y, p, _| {
            let ang = p.a * PI * 0.45;
            let c = ang.cos().max(0.05);
            let persp = 1.0 + ang.sin() * ((x - p.cx) / p.cx) * 0.35;
            let k = persp.max(0.2);
            (p.cx + (x - p.cx) / c * k, p.cy + (y - p.cy) * k)
        },
        _ => return None,
    })
}

/// Pequeña ayuda para encadenar una transformación sobre una tupla.
trait Pipe: Sized {
    fn pipe<R>(self, f: impl FnOnce(Self) -> R) -> R {
        f(self)
    }
}
impl<T> Pipe for T {}

// ---------------------------------------------------------------------------
// Desenfoques con barrido (lineal, zoom, giro, vórtice) y de disco
// ---------------------------------------------------------------------------

fn blur_ops(op: Op, img: &Img, p: &P) -> Vec<u8> {
    let (w, h) = (img.w, img.h);
    match op {
        Op::InnerBlur => {
            let b = blur(img.d, w, h, rad(w, p.a, 0.03).max(1));
            gen(w, h, |x, y| {
                let (dx, dy) = (x as f32 - p.cx, y as f32 - p.cy);
                let t = 1.0 - smooth(p.m * 0.3, p.m * 0.5, dx.hypot(dy));
                mix4(img.at(x as i32, y as i32), Img { d: &b, w, h }.at(x as i32, y as i32), t)
            })
        }
        Op::MaskBlur => {
            // Enfoque en una elipse central; desenfoque creciente hacia fuera.
            let b = blur(img.d, w, h, rad(w, p.a, 0.035).max(1));
            let bi = Img { d: &b, w, h };
            gen(w, h, |x, y| {
                let (dx, dy) = ((x as f32 - p.cx) / p.cx, (y as f32 - p.cy) / p.cy);
                let t = smooth(0.35, 0.95, dx.hypot(dy));
                mix4(img.at(x as i32, y as i32), bi.at(x as i32, y as i32), t)
            })
        }
        Op::FillBehind => {
            // Fondo = copia ampliada y desenfocada; primer plano = fotograma reducido.
            let b = blur(img.d, w, h, (w / 40).max(2));
            let bi = Img { d: &b, w, h };
            let s = 1.0 - p.a * 0.45;
            gen(w, h, |x, y| {
                let (xf, yf) = (x as f32, y as f32);
                let (fx, fy) = (p.cx + (xf - p.cx) / s, p.cy + (yf - p.cy) / s);
                let inside = fx >= 0.0 && fx < p.w && fy >= 0.0 && fy < p.h;
                let bgc = bi.bil(p.cx + (xf - p.cx) * 0.6, p.cy + (yf - p.cy) * 0.6);
                let bg = [bgc[0] * 0.7, bgc[1] * 0.7, bgc[2] * 0.7, 255.0];
                if inside { img.bil(fx, fy) } else { bg }
            })
        }
        Op::LensBlur => {
            // Disco de bokeh con pesos por brillo (muestreo en espiral áurea).
            let r = p.a * p.w * 0.025;
            let n = 40;
            gen(w, h, |x, y| {
                let mut acc = [0.0f32; 4];
                let mut ws = 0.0;
                for i in 0..n {
                    let t = (i as f32 + 0.5) / n as f32;
                    let rr = t.sqrt() * r;
                    let ang = i as f32 * 2.399963;
                    let c = img.bil(x as f32 + rr * ang.cos(), y as f32 + rr * ang.sin());
                    let lum = (c[0] + c[1] + c[2]) / 765.0;
                    let wgt = 1.0 + lum * lum * 3.0;
                    for k in 0..4 {
                        acc[k] += c[k] * wgt;
                    }
                    ws += wgt;
                }
                [acc[0] / ws, acc[1] / ws, acc[2] / ws, acc[3] / ws]
            })
        }
        Op::Denoise => {
            // Mediana 3×3 ponderada por la cantidad.
            let mut out = vec![0u8; img.d.len()];
            out.par_chunks_mut(w * 4).enumerate().for_each(|(y, row)| {
                for x in 0..w {
                    let mut o = [0.0f32; 4];
                    for c in 0..4 {
                        let mut v = [0.0f32; 9];
                        let mut k = 0;
                        for dy in -1..=1 {
                            for dx in -1..=1 {
                                v[k] = img.at(x as i32 + dx, y as i32 + dy)[c];
                                k += 1;
                            }
                        }
                        v.sort_by(|a, b| a.total_cmp(b));
                        o[c] = v[4];
                    }
                    let s = img.at(x as i32, y as i32);
                    let m = mix4(s, o, p.a);
                    let i = x * 4;
                    for c in 0..4 {
                        row[i + c] = cl(m[c]);
                    }
                }
            });
            // Un pase de desenfoque suave tras la mediana para reducir ruido residual.
            let r = rad(w, p.a, 0.004);
            if r > 0 { blur(&out, w, h, r) } else { out }
        }
        _ => sweep(op, img, p),
    }
}

fn sweep(op: Op, img: &Img, p: &P) -> Vec<u8> {
    let (w, h) = (img.w, img.h);
    let n: i32 = 20;
    let ang_deg = if matches!(op, Op::Directional | Op::LinStreak | Op::Smear | Op::Motion | Op::StreakStrips) { p.d } else { 0.0 };
    let dir = (ang_deg.to_radians().cos(), ang_deg.to_radians().sin());
    let len = p.a * p.w * 0.12;
    let streak = matches!(op, Op::ZoomStreak | Op::SpinStreak | Op::LinStreak | Op::StreakStrips);
    gen(w, h, |x, y| {
        let (xf, yf) = (x as f32 + 0.5, y as f32 + 0.5);
        let (dx, dy) = (xf - p.cx, yf - p.cy);
        let r = dx.hypot(dy);
        let mut acc = [0.0f32; 4];
        let mut mx = [0.0f32; 4];
        let mut ws = 0.0f32;
        // Para StreakStrips solo se estiran franjas alternas.
        let active = if op == Op::StreakStrips { ((y as f32 / (p.h / 14.0)) as i32) & 1 == 0 } else { true };
        let steps = if active { n } else { 1 };
        for i in 0..steps {
            let t = if steps == 1 { 0.0 } else { i as f32 / (steps - 1) as f32 - 0.5 };
            let (sx, sy, ch_off) = match op {
                Op::Directional | Op::Motion | Op::TrueMotionBlur | Op::LinStreak | Op::StreakStrips => (xf + dir.0 * len * t, yf + dir.1 * len * t, 0.0),
                Op::Smear => (xf - dir.0 * len * (t + 0.5), yf - dir.1 * len * (t + 0.5), 0.0),
                Op::ZoomBlur | Op::ZoomStreak | Op::ChromZoom => {
                    let k = 1.0 + t * p.a * 0.3;
                    (p.cx + dx * k, p.cy + dy * k, t)
                }
                Op::SpinBlur | Op::SpinStreak => {
                    let (rx, ry) = rot(dx, dy, t * p.a * 0.6);
                    (p.cx + rx, p.cy + ry, 0.0)
                }
                Op::VortexBlur => {
                    let (rx, ry) = rot(dx, dy, t * p.a * 0.9 * (1.0 - (r / p.rmax).min(1.0)));
                    (p.cx + rx, p.cy + ry, 0.0)
                }
                Op::WarpBlur => {
                    let s = p.w / 6.0;
                    let ang = vnoise(xf / s, yf / s) * TAU;
                    (xf + ang.cos() * len * t, yf + ang.sin() * len * t, 0.0)
                }
                _ => (xf, yf, 0.0),
            };
            let mut c = img.bil(sx - 0.5, sy - 0.5);
            if op == Op::ChromZoom {
                // Canales con escalas distintas → aberración cromática radial.
                let kr = 1.0 + (t + 0.15) * p.a * 0.3;
                let kb = 1.0 + (t - 0.15) * p.a * 0.3;
                c[0] = img.bil(p.cx + dx * kr - 0.5, p.cy + dy * kr - 0.5)[0];
                c[2] = img.bil(p.cx + dx * kb - 0.5, p.cy + dy * kb - 0.5)[2];
                let _ = ch_off;
            }
            let wgt = if streak { 1.0 - (i as f32 / steps as f32) * 0.75 } else { 1.0 };
            for k in 0..4 {
                acc[k] += c[k] * wgt;
                mx[k] = mx[k].max(c[k] * wgt);
            }
            ws += wgt;
        }
        let avg = [acc[0] / ws, acc[1] / ws, acc[2] / ws, acc[3] / ws];
        if streak {
            // Estela = aclarado (máximo) sobre el original.
            let s = img.at(x as i32, y as i32);
            [s[0].max(mx[0]), s[1].max(mx[1]), s[2].max(mx[2]), s[3]]
        } else {
            avg
        }
    })
}
