//! Efectos que necesitan control total del píxel (alfa, varias muestras o
//! trazado de rayos con SDF): pasar página, fundidos y pulsos de opacidad,
//! retroalimentación, eco, fractura/dispersión 3D, sólidos SDF (caja hueca,
//! estrella, corazón, cruz, anillo, extrusión ráster) y color/clave avanzados.

use crate::fx_engine::*;
use std::f32::consts::{PI, TAU};

const CLEAR: [f32; 4] = [0.0, 0.0, 0.0, 0.0];

#[inline]
fn lum(c: [f32; 4]) -> f32 {
    (0.299 * c[0] + 0.587 * c[1] + 0.114 * c[2]) / 255.0
}

/// Mezcla `top` sobre `base` (alfa no premultiplicado).
fn comp(base: [f32; 4], top: [f32; 4], k: f32) -> [f32; 4] {
    let ta = top[3] / 255.0 * k;
    let ba = base[3] / 255.0;
    let oa = ta + ba * (1.0 - ta);
    if oa <= 1e-5 {
        return CLEAR;
    }
    let ch = |i: usize| (top[i] * ta + base[i] * ba * (1.0 - ta)) / oa;
    [ch(0), ch(1), ch(2), oa * 255.0]
}

pub fn handles(op: Op) -> bool {
    matches!(
        op,
        Op::PageCurl | Op::Fade | Op::PulseOpacity | Op::Feedback | Op::Echo | Op::Fracture | Op::Scatter3
            | Op::HollowBox | Op::StarPrism | Op::StarPoly | Op::Heart | Op::Cross3 | Op::Ring | Op::RasterExtrude
            | Op::HotColor | Op::SpectralMap | Op::StrokeColor | Op::AdvChroma | Op::OminoDiff | Op::HexArray
    )
}

pub fn run(op: Op, img: &Img, p: &P) -> Vec<u8> {
    let (w, h) = (img.w, img.h);
    let a = p.a;
    let det = p.d.max(1.0);
    match op {
        // ------------------------------------------------------ pasar página
        Op::PageCurl => {
            // Pliegue perpendicular a la diagonal desde la esquina inferior
            // derecha. La parte levantada se enrolla en un cilindro de radio r:
            // por delante se ve el dorso (aclarado), detrás queda transparente.
            let (dx, dy) = {
                let l = p.w.hypot(p.h);
                (p.w / l, p.h / l)
            };
            let len = p.w * dx + p.h * dy;
            let pos = len * (1.0 - a * 0.9);
            let r = len * 0.06;
            gen(w, h, |x, y| {
                let (xf, yf) = (x as f32 + 0.5, y as f32 + 0.5);
                let d = xf * dx + yf * dy; // distancia a lo largo de la diagonal
                let front = img.bil(xf - 0.5, yf - 0.5);
                if d <= pos {
                    // Dorso del cilindro que cae encima de la página.
                    let k = pos - d;
                    if k < r {
                        let ang = (k / r).clamp(-1.0, 1.0).asin();
                        let arc = PI - ang;
                        let back_d = pos + arc * r;
                        if back_d < len {
                            let (sx, sy) = (xf + (back_d - d) * dx, yf + (back_d - d) * dy);
                            if sx < p.w && sy < p.h {
                                let c = img.bil(sx - 0.5, sy - 0.5);
                                let sh = 0.75 + 0.25 * ang.cos();
                                let g = lum(c) * 255.0;
                                let back = [lerp(c[0], g, 0.6) * sh + 40.0, lerp(c[1], g, 0.6) * sh + 40.0, lerp(c[2], g, 0.6) * sh + 40.0, c[3]];
                                return back;
                            }
                        }
                    }
                    // Sombra suave del rizo sobre la página.
                    let s = 1.0 - (0.35 * (1.0 - ((pos - d) / (r * 2.5)).clamp(0.0, 1.0))) * a.min(1.0);
                    return [front[0] * s, front[1] * s, front[2] * s, front[3]];
                }
                // Zona levantada: parte frontal del cilindro (curvándose).
                let k = d - pos;
                if k < r {
                    let ang = (k / r).asin();
                    let src_d = pos + ang * r;
                    let (sx, sy) = (xf + (src_d - d) * dx, yf + (src_d - d) * dy);
                    let c = img.bil(sx - 0.5, sy - 0.5);
                    let sh = 0.6 + 0.4 * ang.cos();
                    return [c[0] * sh, c[1] * sh, c[2] * sh, c[3]];
                }
                CLEAR
            })
        }
        // --------------------------------------------- opacidad en el tiempo
        Op::Fade => {
            // `cantidad` = duración del fundido (0..2 s) en entrada y salida.
            let fi = (a * 2.0).max(0.02);
            let k = if p.dur > 0.0 {
                ((p.t / fi).min(1.0) * ((p.dur - p.t) / fi).min(1.0)).clamp(0.0, 1.0)
            } else {
                0.5
            };
            let k = k * k * (3.0 - 2.0 * k);
            gen(w, h, |x, y| {
                let c = img.at(x as i32, y as i32);
                [c[0], c[1], c[2], c[3] * k]
            })
        }
        Op::PulseOpacity => {
            let f = det / 4.0; // pulsos por segundo
            let s = 0.5 + 0.5 * (p.t * TAU * f).cos();
            let k = 1.0 - a * (if p.t == 0.0 && p.dur == 0.0 { 0.5 } else { 1.0 - s });
            gen(w, h, |x, y| {
                let c = img.at(x as i32, y as i32);
                [c[0], c[1], c[2], c[3] * k]
            })
        }
        // -------------------------------------------- retroalimentación / eco
        Op::Feedback => {
            // Copias recursivas cada vez más pequeñas y giradas hacia el centro.
            let n = 7;
            let sc = 1.0 - (0.08 + a * 0.18);
            let spin = (det - 8.0) * 0.02 + p.t * 0.15 * a;
            gen(w, h, |x, y| {
                let mut out = img.at(x as i32, y as i32);
                let (fx, fy) = (x as f32 + 0.5 - p.cx, y as f32 + 0.5 - p.cy);
                let mut s = 1.0f32;
                for k in 1..=n {
                    s *= sc;
                    let ang = spin * k as f32;
                    let (cs, sn) = ang.cos_sin_pair();
                    let (rx, ry) = ((fx * cs + fy * sn) / s, (-fx * sn + fy * cs) / s);
                    let (sx, sy) = (rx + p.cx, ry + p.cy);
                    if sx < 0.0 || sy < 0.0 || sx >= p.w || sy >= p.h {
                        break;
                    }
                    let c = img.bil(sx - 0.5, sy - 0.5);
                    out = comp(out, c, 0.88);
                }
                out
            })
        }
        Op::Echo => {
            // Ecos: copias anteriores desplazadas siguiendo un movimiento
            // oscilante, cada vez más tenues (lo que dejaría el fotograma previo).
            let n = (3.0 + det * 0.4) as i32;
            gen(w, h, |x, y| {
                let mut acc = [0.0f32; 4];
                let mut wsum = 0.0f32;
                for k in (0..=n).rev() {
                    let lag = k as f32 * 0.06;
                    let tt = p.t - lag;
                    let ox = (tt * 2.1).sin() * a * p.w * 0.04 + k as f32 * a * p.w * 0.012;
                    let oy = (tt * 1.3).cos() * a * p.h * 0.02;
                    let wt = 0.62f32.powi(k);
                    let c = img.bil(x as f32 - ox, y as f32 - oy);
                    for i in 0..4 {
                        acc[i] += c[i] * wt;
                    }
                    wsum += wt;
                }
                let c0 = img.at(x as i32, y as i32);
                let e = [acc[0] / wsum, acc[1] / wsum, acc[2] / wsum, acc[3] / wsum];
                [e[0].max(c0[0] * 0.0 + e[0]), e[1], e[2], e[3].max(c0[3])]
            })
        }
        // ---------------------------------------------------- fragmentos 3D
        Op::Fracture => {
            // Voronoi: cada fragmento se separa del centro, gira y encoge.
            let n = det * 0.6 + 3.0;
            let cell = p.m / n;
            let push = a * p.m * 0.35;
            gen(w, h, |x, y| {
                let (xf, yf) = (x as f32, y as f32);
                let mut best = CLEAR;
                let mut bz = f32::MAX;
                let (ci, cj) = ((xf / cell).floor() as i32, (yf / cell).floor() as i32);
                for j in -2..=2 {
                    for i in -2..=2 {
                        let (gi, gj) = (ci + i, cj + j);
                        let site = ((gi as f32 + h2(gi, gj)) * cell, (gj as f32 + h2(gj + 17, gi)) * cell);
                        let (vx, vy) = (site.0 - p.cx, site.1 - p.cy);
                        let dist = vx.hypot(vy).max(1.0);
                        let rnd = h2(gi * 7 + 3, gj * 13 + 1);
                        let mv = push * (0.4 + rnd) * (dist / p.rmax + 0.3);
                        let (ox, oy) = (vx / dist * mv, vy / dist * mv);
                        let rot = (rnd - 0.5) * a * 1.4;
                        let scale = 1.0 - a * 0.25 * rnd;
                        // Punto en el espacio original del fragmento.
                        let (lx, ly) = (xf - site.0 - ox, yf - site.1 - oy);
                        let (cs, sn) = rot.cos_sin_pair();
                        let (qx, qy) = ((lx * cs + ly * sn) / scale + site.0, (-lx * sn + ly * cs) / scale + site.1);
                        // ¿Pertenece q a la celda de este sitio?
                        let (qi, qj) = ((qx / cell).floor() as i32, (qy / cell).floor() as i32);
                        let mut own = (gi, gj);
                        let mut md = f32::MAX;
                        for b in -1..=1 {
                            for c in -1..=1 {
                                let (si, sj) = (qi + c, qj + b);
                                let s = ((si as f32 + h2(si, sj)) * cell, (sj as f32 + h2(sj + 17, si)) * cell);
                                let dd = (s.0 - qx).powi(2) + (s.1 - qy).powi(2);
                                if dd < md {
                                    md = dd;
                                    own = (si, sj);
                                }
                            }
                        }
                        if own == (gi, gj) && qx >= 0.0 && qy >= 0.0 && qx < p.w && qy < p.h {
                            let z = rnd;
                            if z < bz {
                                bz = z;
                                let c = img.bil(qx, qy);
                                let sh = 1.0 - rot.abs() * 0.3;
                                best = [c[0] * sh, c[1] * sh, c[2] * sh, c[3]];
                            }
                        }
                    }
                }
                best
            })
        }
        Op::Scatter3 => {
            // Teselas cuadradas que salen volando en profundidad.
            let n = (det * 0.8 + 4.0).round();
            let cell = p.w / n;
            gen(w, h, |x, y| {
                let (xf, yf) = (x as f32, y as f32);
                let mut best = CLEAR;
                let mut bz = f32::MAX;
                let (ci, cj) = ((xf / cell).floor() as i32, (yf / cell).floor() as i32);
                let rr = (2.0 + a * 4.0) as i32;
                for j in -rr..=rr {
                    for i in -rr..=rr {
                        let (gi, gj) = (ci + i, cj + j);
                        let c0 = ((gi as f32 + 0.5) * cell, (gj as f32 + 0.5) * cell);
                        if c0.0 < 0.0 || c0.1 < 0.0 || c0.0 > p.w || c0.1 > p.h {
                            continue;
                        }
                        let r1 = h2(gi, gj);
                        let r2 = h2(gj + 5, gi + 9);
                        let depth = a * (0.3 + r1 * 0.7);
                        let persp = 1.0 / (1.0 + depth * 1.6);
                        let (vx, vy) = (c0.0 - p.cx, c0.1 - p.cy);
                        let (tx, ty) = (p.cx + vx * (1.0 + depth * 0.9) + (r2 - 0.5) * a * cell * 2.0, p.cy + vy * (1.0 + depth * 0.9) + (r1 - 0.5) * a * cell * 2.0);
                        let rot = (r2 - 0.5) * a * PI;
                        let (lx, ly) = ((xf - tx) / persp, (yf - ty) / persp);
                        let (cs, sn) = rot.cos_sin_pair();
                        let (ux, uy) = (lx * cs + ly * sn, -lx * sn + ly * cs);
                        if ux.abs() <= cell * 0.5 && uy.abs() <= cell * 0.5 && depth < bz {
                            bz = depth;
                            let c = img.bil(c0.0 + ux, c0.1 + uy);
                            let sh = 1.0 - depth * 0.35;
                            best = [c[0] * sh, c[1] * sh, c[2] * sh, c[3]];
                        }
                    }
                }
                best
            })
        }
        // ---------------------------------------------------- sólidos SDF
        Op::HollowBox | Op::StarPrism | Op::StarPoly | Op::Heart | Op::Cross3 | Op::Ring | Op::RasterExtrude => sdf_solid(op, img, p),
        // ---------------------------------------------------- color
        Op::HotColor => gen(w, h, |x, y| {
            let c = img.at(x as i32, y as i32);
            let l = lum(c);
            // negro → rojo → naranja → amarillo → blanco
            let r = (l * 3.0).min(1.0);
            let g = (l * 3.0 - 1.0).clamp(0.0, 1.0);
            let b = (l * 3.0 - 2.0).clamp(0.0, 1.0);
            mix4(c, [r * 255.0, g * 255.0, b * 255.0, c[3]], a.max(0.05))
        }),
        Op::SpectralMap => gen(w, h, |x, y| {
            let c = img.at(x as i32, y as i32);
            let l = lum(c);
            let g = hsv((1.0 - l) * 270.0 + (det - 8.0) * 10.0, 0.9, 0.35 + l * 0.65);
            mix4(c, [g[0], g[1], g[2], c[3]], a.max(0.05))
        }),
        Op::StrokeColor => {
            // Trazo de color alrededor de la silueta (alfa) o de los bordes.
            let col = hsv(det * 18.0, 0.85, 1.0);
            let r = a * p.m * 0.03 + 1.0;
            gen(w, h, |x, y| {
                let c = img.at(x as i32, y as i32);
                let mut mx = 0.0f32;
                for k in 0..16 {
                    let ang = k as f32 / 16.0 * TAU;
                    let s = img.bil(x as f32 + ang.cos() * r, y as f32 + ang.sin() * r);
                    mx = mx.max(s[3]);
                }
                let outer = (mx / 255.0) * (1.0 - c[3] / 255.0);
                let e = {
                    let l = |dx: i32, dy: i32| lum(img.at(x as i32 + dx, y as i32 + dy));
                    let gx = l(1, 0) - l(-1, 0);
                    let gy = l(0, 1) - l(0, -1);
                    (gx.hypot(gy) * 3.0).clamp(0.0, 1.0) * a.max(0.1)
                };
                let base = comp([col[0], col[1], col[2], 255.0 * outer], c, 1.0);
                [lerp(base[0], col[0], e), lerp(base[1], col[1], e), lerp(base[2], col[2], e), base[3]]
            })
        }
        Op::AdvChroma => {
            // Clave en espacio CbCr con tolerancia, suavizado y supresión de derrame.
            let target = hsv(60.0 + (det - 1.0) / 19.0 * 180.0, 1.0, 1.0);
            let cbcr = |c: [f32; 4]| {
                let (r, g, b) = (c[0] / 255.0, c[1] / 255.0, c[2] / 255.0);
                (-0.169 * r - 0.331 * g + 0.5 * b, 0.5 * r - 0.419 * g - 0.081 * b)
            };
            let (tb, tr) = cbcr([target[0], target[1], target[2], 255.0]);
            let tol = 0.05 + a * 0.25;
            gen(w, h, |x, y| {
                let c = img.at(x as i32, y as i32);
                let (cb, cr) = cbcr(c);
                let d = (cb - tb).hypot(cr - tr);
                let alpha = smooth(tol, tol + 0.12, d);
                // Despill: limita el canal dominante del fondo al promedio de los otros.
                let mut o = c;
                let tmax = if target[1] >= target[0] && target[1] >= target[2] { 1 } else if target[2] >= target[0] { 2 } else { 0 };
                let (i1, i2) = match tmax { 0 => (1, 2), 1 => (0, 2), _ => (0, 1) };
                let lim = (c[i1] + c[i2]) * 0.5;
                if o[tmax] > lim {
                    o[tmax] = lerp(o[tmax], lim, 1.0 - alpha * 0.5);
                }
                [o[0], o[1], o[2], c[3] * alpha]
            })
        }
        Op::OminoDiff => {
            // Diferencia con una copia desplazada por poliominós aleatorios.
            let cell = (p.w / (det * 2.0 + 6.0)).max(3.0);
            gen(w, h, |x, y| {
                let c = img.at(x as i32, y as i32);
                let (ix, iy) = ((x as f32 / cell).floor() as i32, (y as f32 / cell).floor() as i32);
                // Fusiona celdas vecinas en piezas de 1-4 celdas.
                let gx = if h2(ix / 2, iy) < 0.5 { ix / 2 * 2 } else { ix };
                let gy = if h2(gx, iy / 2 + 50) < 0.5 { iy / 2 * 2 } else { iy };
                let fr = (p.t * 6.0).floor() as i32;
                let ox = (h2(gx + fr, gy) - 0.5) * cell * 3.0 * a;
                let oy = (h2(gy + 31, gx + fr) - 0.5) * cell * 3.0 * a;
                let s = img.bil(x as f32 + ox, y as f32 + oy);
                let dif = [(c[0] - s[0]).abs(), (c[1] - s[1]).abs(), (c[2] - s[2]).abs(), c[3]];
                let boost = [c[0] * 0.5 + dif[0] * 1.5, c[1] * 0.5 + dif[1] * 1.5, c[2] * 0.5 + dif[2] * 1.5, c[3]];
                mix4(c, boost, a.max(0.05))
            })
        }
        Op::HexArray => {
            // Cada hexágono muestra una miniatura de la imagen completa.
            let s = p.w / (det * 0.8 + 3.0);
            let hh = s * 3.0f32.sqrt();
            gen(w, h, |x, y| {
                let c = img.at(x as i32, y as i32);
                let (xf, yf) = (x as f32, y as f32);
                let col = (xf / (s * 1.5)).floor();
                let mut best = (0.0f32, 0.0f32, f32::MAX);
                for ci in [col - 1.0, col, col + 1.0] {
                    let off = if (ci as i32).rem_euclid(2) == 0 { 0.0 } else { hh * 0.5 };
                    let cx = (ci + 0.5) * s * 1.5;
                    let cyi = ((yf - off) / hh).floor();
                    for cj in [cyi - 1.0, cyi, cyi + 1.0] {
                        let cy = (cj + 0.5) * hh + off;
                        let d = (xf - cx).hypot(yf - cy);
                        if d < best.2 {
                            best = (cx, cy, d);
                        }
                    }
                }
                let (lx, ly) = ((xf - best.0) / (s * 2.0), (yf - best.1) / (s * 2.0));
                let m = img.bil((lx + 0.5) * p.w, (ly * p.w / p.h * 1.0 + 0.5) * p.h);
                // Borde del hexágono oscurecido.
                let q = ((xf - best.0).abs(), (yf - best.1).abs());
                let hexd = (q.0 * 0.866 + q.1 * 0.5).max(q.1) / (hh * 0.5);
                let edge = smooth(0.9, 1.0, hexd);
                let out = [m[0] * (1.0 - edge * 0.8), m[1] * (1.0 - edge * 0.8), m[2] * (1.0 - edge * 0.8), m[3]];
                mix4(c, out, a.max(0.05))
            })
        }
        _ => img.d.to_vec(),
    }
}

trait CosSin {
    fn cos_sin_pair(self) -> (f32, f32);
}
impl CosSin for f32 {
    fn cos_sin_pair(self) -> (f32, f32) {
        let (s, c) = self.sin_cos();
        (c, s)
    }
}

// ---------------------------------------------------------------------------
// Sólidos por campo de distancia (raymarching)
// ---------------------------------------------------------------------------

type V3 = [f32; 3];

fn dot(a: V3, b: V3) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn norm(a: V3) -> V3 {
    let l = dot(a, a).sqrt().max(1e-6);
    [a[0] / l, a[1] / l, a[2] / l]
}
fn rot_yx(v: V3, ay: f32, ax: f32) -> V3 {
    let (sy, cy) = ay.sin_cos();
    let (x, z) = (v[0] * cy + v[2] * sy, -v[0] * sy + v[2] * cy);
    let (sx, cx) = ax.sin_cos();
    [x, v[1] * cx - z * sx, v[1] * sx + z * cx]
}

fn sd_box(q: V3, b: V3) -> f32 {
    let d = [q[0].abs() - b[0], q[1].abs() - b[1], q[2].abs() - b[2]];
    let o = [d[0].max(0.0), d[1].max(0.0), d[2].max(0.0)];
    dot(o, o).sqrt() + d[0].max(d[1]).max(d[2]).min(0.0)
}

/// Estrella de 5 puntas 2D (radio exterior r, interior ri).
fn sd_star2(px: f32, py: f32, r: f32, ri: f32) -> f32 {
    let n = 5.0;
    let ang = py.atan2(px) + PI * 0.5;
    let seg = TAU / n;
    let a = (ang.rem_euclid(seg) - seg * 0.5).abs();
    let rr = px.hypot(py);
    let (x, y) = (rr * a.cos(), rr * a.sin());
    // Segmento desde la punta (r, 0) hasta el valle (ri·cos, ri·sin) de seg/2.
    let (ax, ay) = (r, 0.0);
    let (bx, by) = (ri * (seg * 0.5).cos(), ri * (seg * 0.5).sin());
    let (ex, ey) = (bx - ax, by - ay);
    let (wx, wy) = (x - ax, y - ay);
    let t = ((wx * ex + wy * ey) / (ex * ex + ey * ey)).clamp(0.0, 1.0);
    let d = (wx - ex * t).hypot(wy - ey * t);
    let side = ex * wy - ey * wx;
    if side > 0.0 { -d } else { d }
}

fn sd_heart2(px: f32, py: f32) -> f32 {
    // Corazón de Inigo Quilez (unidad ~1), con la punta hacia abajo.
    let x = px.abs();
    let y = py + 0.5;
    if y + x > 1.0 {
        return ((x - 0.25).powi(2) + (y - 0.75).powi(2)).sqrt() - 2f32.sqrt() / 4.0;
    }
    let d1 = (x * x + (y - 1.0).powi(2)).sqrt();
    let m = 0.5 * (x + y).max(0.0);
    let d2 = ((x - m).powi(2) + (y - m).powi(2)).sqrt();
    d1.min(d2) * (x - y).signum()
}

fn extrude(d2: f32, z: f32, hgt: f32) -> f32 {
    let wv = (d2, z.abs() - hgt);
    wv.0.max(wv.1).min(0.0) + (wv.0.max(0.0).powi(2) + wv.1.max(0.0).powi(2)).sqrt()
}

fn sdf(op: Op, q: V3, img: &Img, p: &P) -> f32 {
    match op {
        Op::HollowBox => {
            let outer = sd_box(q, [0.6, 0.6, 0.6]);
            let ix = sd_box(q, [0.7, 0.45, 0.45]);
            let iy = sd_box(q, [0.45, 0.7, 0.45]);
            let iz = sd_box(q, [0.45, 0.45, 0.7]);
            outer.max(-ix).max(-iy).max(-iz)
        }
        Op::StarPrism => extrude(sd_star2(q[0], q[1], 0.75, 0.32), q[2], 0.22),
        Op::StarPoly => {
            // Bipirámide estrellada: la estrella se estrecha con |z|.
            let k = (1.0 - q[2].abs() / 0.45).max(0.02);
            let d = sd_star2(q[0] / k, q[1] / k, 0.75, 0.32) * k;
            d.max(q[2].abs() - 0.45) * 0.7
        }
        Op::Heart => extrude(sd_heart2(q[0] * 1.35, -q[1] * 1.35 + 0.05) / 1.35 - 0.02, q[2], 0.18) - 0.04,
        Op::Cross3 => {
            let t = 0.2;
            sd_box(q, [0.7, t, t]).min(sd_box(q, [t, 0.7, t])).min(sd_box(q, [t, t, 0.7]))
        }
        Op::Ring => {
            let r = q[0].hypot(q[1]);
            extrude((r - 0.58).abs() - 0.14, q[2], 0.12)
        }
        Op::RasterExtrude => {
            // Altura proporcional a la luma (y alfa) del fotograma.
            let aspect = p.w / p.h;
            let (u, v) = (q[0] / (0.8 * aspect.min(1.6)) * 0.5 + 0.5, 0.5 - q[1] / 0.8 * 0.5);
            let base = sd_box(q, [0.8 * aspect.min(1.6), 0.8, 0.02]);
            if !(0.0..=1.0).contains(&u) || !(0.0..=1.0).contains(&v) {
                return base.max(0.02);
            }
            let c = img.bil(u * (p.w - 1.0), v * (p.h - 1.0));
            let hgt = 0.02 + lum(c) * (c[3] / 255.0) * 0.35 * (0.3 + p.a);
            let top = q[2] + hgt; // la cara frontal sobresale hacia la cámara (z negativo)
            let slab = (-top).max(q[2] - 0.02);
            slab.max(base - 0.0).min(base) * 0.5
        }
        _ => 1.0,
    }
}

fn sdf_solid(op: Op, img: &Img, p: &P) -> Vec<u8> {
    let (w, h) = (img.w, img.h);
    let ay = p.a * PI * 0.9 + 0.5 + p.t * 0.0;
    let ax = 0.45 * p.a + (p.d - 8.0) * 0.03;
    let (ay, ax) = if op == Op::RasterExtrude { (p.a * 0.9 - 0.45, 0.35 * p.a) } else { (ay, ax) };
    let cam = [0.0f32, 0.0, -2.6];
    let focal = 1.9;
    let aspect = p.w / p.h;
    let light = norm([-0.4, 0.6, -0.7]);
    gen(w, h, |x, y| {
        let u = (x as f32 + 0.5) / p.w * 2.0 - 1.0;
        let v = 1.0 - (y as f32 + 0.5) / p.h * 2.0;
        let dir = norm([u * aspect, v, focal]);
        let o = rot_yx(cam, -ay, -ax);
        let dr = rot_yx(dir, -ay, -ax);
        let mut t = 1.2f32;
        for _ in 0..96 {
            let q = [o[0] + dr[0] * t, o[1] + dr[1] * t, o[2] + dr[2] * t];
            let d = sdf(op, q, img, p);
            if d < 0.0015 {
                let e = 0.002;
                let n = norm([
                    sdf(op, [q[0] + e, q[1], q[2]], img, p) - sdf(op, [q[0] - e, q[1], q[2]], img, p),
                    sdf(op, [q[0], q[1] + e, q[2]], img, p) - sdf(op, [q[0], q[1] - e, q[2]], img, p),
                    sdf(op, [q[0], q[1], q[2] + e], img, p) - sdf(op, [q[0], q[1], q[2] - e], img, p),
                ]);
                // Proyección planar según el eje dominante de la normal.
                let (su, sv) = if n[2].abs() >= n[0].abs() && n[2].abs() >= n[1].abs() {
                    (q[0], q[1])
                } else if n[0].abs() >= n[1].abs() {
                    (q[2], q[1])
                } else {
                    (q[0], q[2])
                };
                let sx = if op == Op::RasterExtrude { su / (0.8 * aspect.min(1.6)) * 0.5 + 0.5 } else { su / 1.4 + 0.5 };
                let sy = if op == Op::RasterExtrude { 0.5 - sv / 0.8 * 0.5 } else { 0.5 - sv / 1.4 };
                let c = img.bil(sx.rem_euclid(1.0) * (p.w - 1.0), sy.rem_euclid(1.0) * (p.h - 1.0));
                let lw = rot_yx(light, -ay, -ax);
                let k = 0.4 + 0.6 * dot(n, lw).max(0.0);
                return [c[0] * k, c[1] * k, c[2] * k, c[3].max(200.0)];
            }
            t += d.max(0.002);
            if t > 6.0 {
                break;
            }
        }
        CLEAR
    })
}
