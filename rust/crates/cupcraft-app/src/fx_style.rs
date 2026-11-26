//! Efectos de estilo, color, ruido y clave (alfa) del motor por píxel.
//! Ver `fx_engine` para el contexto; `P.a` es la cantidad 0..1 y `P.d` el
//! segundo parámetro (detalle 1..20, o ángulo en grados según el efecto).

use crate::fx_engine::*;
use std::f32::consts::TAU;

#[inline]
fn sobel(img: &Img, x: i32, y: i32) -> (f32, f32) {
    let l = |dx: i32, dy: i32| img.luma(x + dx, y + dy);
    let gx = -l(-1, -1) - 2.0 * l(-1, 0) - l(-1, 1) + l(1, -1) + 2.0 * l(1, 0) + l(1, 1);
    let gy = -l(-1, -1) - 2.0 * l(0, -1) - l(1, -1) + l(-1, 1) + 2.0 * l(0, 1) + l(1, 1);
    (gx, gy)
}

#[inline]
fn lum(c: [f32; 4]) -> f32 {
    (0.299 * c[0] + 0.587 * c[1] + 0.114 * c[2]) / 255.0
}

fn screen(a: [f32; 4], b: [f32; 4], k: f32) -> [f32; 4] {
    let f = |x: f32, y: f32| 255.0 - (255.0 - x) * (255.0 - y * k) / 255.0;
    [f(a[0], b[0]), f(a[1], b[1]), f(a[2], b[2]), a[3]]
}

fn over(base: [f32; 4], col: [f32; 3], al: f32) -> [f32; 4] {
    [lerp(base[0], col[0], al), lerp(base[1], col[1], al), lerp(base[2], col[2], al), base[3]]
}

fn bl(img: &Img, r: usize) -> Vec<u8> {
    blur(img.d, img.w, img.h, r.max(1))
}

pub fn run(op: Op, img: &Img, p: &P) -> Vec<u8> {
    let (w, h) = (img.w, img.h);
    let a = p.a;
    let det = p.d.max(1.0);
    let at = |x: usize, y: usize| img.at(x as i32, y as i32);
    match op {
        // ---------------------------------------------------------- brillos
        Op::Glow | Op::LightGlow | Op::SoftGlow | Op::DeepGlow | Op::Blink | Op::Flash => {
            if matches!(op, Op::Blink | Op::Flash) {
                return gen(w, h, |x, y| {
                    let c = at(x, y);
                    // Parpadeo: encendido/apagado ~3 veces por segundo (en t=0, encendido).
                    let on = op == Op::Flash || p.t == 0.0 || (p.t * 3.0 * det.max(1.0) / 8.0).fract() < 0.5;
                    let k = if on { 1.0 + a * 1.2 } else { 1.0 - a * 0.6 };
                    [c[0] * k, c[1] * k, c[2] * k, c[3]]
                });
            }
            let (rf, gain) = match op {
                Op::SoftGlow => (0.02, 0.8),
                Op::DeepGlow => (0.06, 1.6),
                Op::LightGlow => (0.03, 1.2),
                _ => (0.035, 1.0),
            };
            let b = bl(img, (a * rf * p.w) as usize + 1);
            let bi = Img { d: &b, w, h };
            gen(w, h, |x, y| {
                let c = at(x, y);
                let g = bi.at(x as i32, y as i32);
                // Solo las zonas claras irradian.
                let t = smooth(0.35, 0.9, lum(g));
                screen(c, [g[0] * t, g[1] * t, g[2] * t, 255.0], a * gain * 1.3)
            })
        }
        Op::InnerGlow | Op::EdgeGlow => {
            let b = bl(img, (a * 0.03 * p.w) as usize + 1);
            let bi = Img { d: &b, w, h };
            gen(w, h, |x, y| {
                let (gx, gy) = sobel(img, x as i32, y as i32);
                let e = (gx.hypot(gy) * 1.5).clamp(0.0, 1.0);
                let g = bi.at(x as i32, y as i32);
                let c = at(x, y);
                let k = e * a * 2.0;
                [c[0] + g[0] * k, c[1] + g[1] * k, c[2] + g[2] * k, c[3]]
            })
        }
        Op::DarkGlow => {
            let b = bl(img, (a * 0.04 * p.w) as usize + 1);
            let bi = Img { d: &b, w, h };
            gen(w, h, |x, y| {
                let c = at(x, y);
                let g = bi.at(x as i32, y as i32);
                let t = (1.0 - lum(g)) * a;
                [c[0] * (1.0 - t * 0.8), c[1] * (1.0 - t * 0.8), c[2] * (1.0 - t * 0.8), c[3]]
            })
        }
        Op::GlowScan => gen(w, h, |x, y| {
            let c = at(x, y);
            let pos = if p.t == 0.0 { det / 20.0 - 0.5 } else { (p.t * det.max(1.0) / 8.0).fract() * 1.6 - 0.8 };
            let d = ((x as f32 / p.w - 0.5 + pos + (y as f32 / p.h - 0.5) * 0.3).abs() * 4.0).min(1.0);
            let k = (1.0 - d).powi(2) * a * 1.4;
            [c[0] + 255.0 * k * 0.6, c[1] + 255.0 * k * 0.8, c[2] + 255.0 * k, c[3]]
        }),
        // ------------------------------------------------ bordes y relieve
        Op::FindEdges | Op::ContourLines | Op::ContourStrips | Op::ContourGrad | Op::Outline | Op::Ribbon => {
            gen(w, h, |x, y| {
                let (gx, gy) = sobel(img, x as i32, y as i32);
                let e = (gx.hypot(gy) * (1.0 + a * 3.0)).clamp(0.0, 1.0);
                let c = at(x, y);
                match op {
                    Op::FindEdges => {
                        let v = 255.0 - e * 255.0;
                        mix4(c, [v, v, v, c[3]], a.max(0.05) * 1.0 + 0.0)
                    }
                    Op::Outline | Op::Ribbon => over(c, [0.0, 0.0, 0.0], e * a.max(0.1)),
                    Op::ContourLines => {
                        let l = lum(c) * det * 0.6;
                        let line = (1.0 - (l.fract() - 0.5).abs() * 2.0).powi(6);
                        over(c, [0.0, 0.0, 0.0], line * a)
                    }
                    Op::ContourStrips => {
                        let l = (lum(c) * det).floor() / det;
                        mix4(c, [l * 255.0, l * 255.0, l * 255.0, c[3]], a)
                    }
                    _ => {
                        let l = lum(c);
                        let g = hsv(l * 300.0, 0.8, 1.0);
                        over(c, g, e * a)
                    }
                }
            })
        }
        Op::ElectricEdges => gen(w, h, |x, y| {
            let (gx, gy) = sobel(img, x as i32, y as i32);
            let n = vnoise(x as f32 * 0.08, y as f32 * 0.08);
            let e = (gx.hypot(gy) * (2.0 + a * 4.0) * (0.6 + n)).clamp(0.0, 1.0);
            let c = at(x, y);
            [c[0] * (1.0 - a * 0.6) + 60.0 * e, c[1] * (1.0 - a * 0.6) + 200.0 * e, c[2] * (1.0 - a * 0.6) + 255.0 * e, c[3]]
        }),
        Op::Emboss | Op::Bevel | Op::BumpMap => gen(w, h, |x, y| {
            let (gx, gy) = sobel(img, x as i32, y as i32);
            let ang = if op == Op::Emboss { det * 0.3 } else { 0.8 };
            let rel = (gx * ang.cos() + gy * ang.sin()) * (1.0 + a * 4.0);
            let c = at(x, y);
            if op == Op::Emboss {
                let v = 128.0 + rel * 128.0;
                mix4(c, [v, v, v, c[3]], a.clamp(0.1, 1.0))
            } else {
                let k = 1.0 + rel * a;
                [c[0] * k, c[1] * k, c[2] * k, c[3]]
            }
        }),
        Op::Cartoon => {
            let b = bl(img, 2);
            let bi = Img { d: &b, w, h };
            let lv = 3.0 + det * 0.4;
            gen(w, h, |x, y| {
                let c = bi.at(x as i32, y as i32);
                let q = |v: f32| (v / 255.0 * lv).round() / lv * 255.0;
                let (gx, gy) = sobel(img, x as i32, y as i32);
                let e = (gx.hypot(gy) * 3.0).clamp(0.0, 1.0);
                let m = [q(c[0]) * (1.0 - e), q(c[1]) * (1.0 - e), q(c[2]) * (1.0 - e), c[3]];
                mix4(at(x, y), m, a.max(0.05))
            })
        }
        Op::Roughen => gen(w, h, |x, y| {
            let s = 18.0 / det.max(1.0);
            let ox = (vnoise(x as f32 / s, y as f32 / s) - 0.5) * a * 14.0;
            let oy = (vnoise(x as f32 / s + 9.0, y as f32 / s + 3.0) - 0.5) * a * 14.0;
            img.bil(x as f32 + ox, y as f32 + oy)
        }),
        // ------------------------------------------------------ semitonos
        Op::HalftoneDots | Op::CmykDots | Op::Ascii => {
            let cell = (p.w / (det * 6.0 + 20.0)).max(3.0);
            gen(w, h, |x, y| {
                let (cx, cy) = (((x as f32 / cell).floor() + 0.5) * cell, ((y as f32 / cell).floor() + 0.5) * cell);
                let c = img.bil(cx, cy);
                let l = lum(c);
                let d = ((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)).sqrt();
                let r = cell * 0.72 * l.sqrt();
                let m = smooth(r + 0.7, r - 0.7, d);
                let out = if op == Op::HalftoneDots { [m * 255.0; 3].map(|v| v) } else { [c[0] * m, c[1] * m, c[2] * m] };
                mix4(at(x, y), [out[0], out[1], out[2], 255.0], a.max(0.05))
            })
        }
        Op::HalftoneLines => {
            let cell = (p.w / (det * 5.0 + 20.0)).max(3.0);
            gen(w, h, |x, y| {
                let cy = ((y as f32 / cell).floor() + 0.5) * cell;
                let l = lum(img.bil(x as f32, cy));
                let m = smooth(cell * 0.5 * l + 0.7, cell * 0.5 * l - 0.7, (y as f32 - cy).abs());
                mix4(at(x, y), [m * 255.0, m * 255.0, m * 255.0, 255.0], a.max(0.05))
            })
        }
        Op::PixelTiles => {
            let cell = (1.0 + a * det * 3.0).max(1.0);
            gen(w, h, |x, y| {
                let cx = ((x as f32 / cell).floor() + 0.5) * cell;
                let cy = ((y as f32 / cell).floor() + 0.5) * cell;
                let c = img.bil(cx, cy);
                let edge = (x as f32 % cell < 1.0 || y as f32 % cell < 1.0) as i32 as f32 * 0.25 * a;
                [c[0] * (1.0 - edge), c[1] * (1.0 - edge), c[2] * (1.0 - edge), c[3]]
            })
        }
        // ------------------------------------------------- nitidez y color
        Op::Sharpen | Op::Unsharp => {
            let b = bl(img, if op == Op::Sharpen { 1 } else { (p.w * 0.004) as usize + 1 });
            let bi = Img { d: &b, w, h };
            gen(w, h, |x, y| {
                let c = at(x, y);
                let g = bi.at(x as i32, y as i32);
                let k = a * 3.0;
                [c[0] + (c[0] - g[0]) * k, c[1] + (c[1] - g[1]) * k, c[2] + (c[2] - g[2]) * k, c[3]]
            })
        }
        Op::RgbSplit => {
            let off = a * p.w * 0.03;
            let ang = det * 0.35;
            let (dx, dy) = (ang.cos() * off, ang.sin() * off);
            gen(w, h, |x, y| {
                let (xf, yf) = (x as f32, y as f32);
                let c = at(x, y);
                [img.bil(xf - dx, yf - dy)[0], c[1], img.bil(xf + dx, yf + dy)[2], c[3]]
            })
        }
        Op::Vignette => gen(w, h, |x, y| {
            let c = at(x, y);
            let (dx, dy) = ((x as f32 - p.cx) / p.cx, (y as f32 - p.cy) / p.cy);
            let k = 1.0 - smooth(0.4, 1.35, dx.hypot(dy)) * a * 1.1;
            [c[0] * k, c[1] * k, c[2] * k, c[3]]
        }),
        Op::Grunge => gen(w, h, |x, y| {
            let c = at(x, y);
            let n = fbm(x as f32 / p.w * 8.0, y as f32 / p.w * 8.0, 5);
            let k = 1.0 - smooth(0.45, 0.8, n) * a * 0.8;
            let g = (h2(x as i32, y as i32) - 0.5) * a * 50.0;
            [c[0] * k + g, c[1] * k + g, c[2] * k + g, c[3]]
        }),
        Op::Flicker => gen(w, h, |x, y| {
            let c = at(x, y);
            let fr = (p.t * det.max(1.0) * 3.0).floor() as i32;
            let k = 1.0 - a * 0.5 * (1.0 + (y as f32 * 0.03 + p.t * 9.0).sin()) * 0.5 + (h2(det as i32 + fr * 7, 7) - 0.5) * a * 0.6;
            [c[0] * k, c[1] * k, c[2] * k, c[3]]
        }),
        Op::Scanlines => {
            let period = (p.h / (det * 12.0 + 40.0)).max(2.0);
            gen(w, h, |x, y| {
                let c = at(x, y);
                let k = 1.0 - a * 0.7 * (0.5 + 0.5 * ((y as f32 / period - p.t * 4.0) * TAU).sin());
                [c[0] * k, c[1] * k, c[2] * k, c[3]]
            })
        }
        Op::Vhs => gen(w, h, |x, y| {
            let fr = (p.t * 12.0).floor() as i32;
            let band = (y as f32 / (p.h / 40.0)) as i32 + fr * 41;
            let jit = (h2(band, det as i32) - 0.5) * a * p.w * 0.04 * (h2(band, 99) > 0.8) as i32 as f32;
            let r = img.bil(x as f32 + jit + a * 4.0, y as f32);
            let g = img.bil(x as f32 + jit, y as f32);
            let b = img.bil(x as f32 + jit - a * 4.0, y as f32);
            let sl = 1.0 - a * 0.25 * (0.5 + 0.5 * (y as f32 * 1.6).sin());
            let n = (h2(x as i32 + fr * 977, y as i32 + det as i32) - 0.5) * a * 40.0;
            [r[0] * sl + n, g[1] * sl + n, b[2] * sl + n, g[3]]
        }),
        Op::Glitch | Op::Datamosh => gen(w, h, |x, y| {
            let fr = (p.t * 10.0).floor() as i32;
            let band = (y as f32 / (p.h / (6.0 + det))).floor() as i32;
            let on = h2(band + fr * 13, 31) < 0.35 + a * 0.4;
            let off = if on { (h2(band + fr * 13, 5) - 0.5) * a * p.w * 0.25 } else { 0.0 };
            let (xf, yf) = (x as f32 + off, y as f32);
            if on && op == Op::Glitch {
                [img.bil(xf + 6.0 * a, yf)[0], img.bil(xf, yf)[1], img.bil(xf - 6.0 * a, yf)[2], 255.0]
            } else if op == Op::Datamosh {
                let (bx, by) = ((x as f32 / 32.0).floor() as i32 + fr * 3, (y as f32 / 32.0).floor() as i32);
                let sm = h2(bx, by) < a * 0.45;
                if sm { img.bil(xf + (h2(bx, by + 4) - 0.5) * 120.0 * a, yf + (h2(by, bx) - 0.5) * 120.0 * a) } else { img.bil(xf, yf) }
            } else {
                img.bil(xf, yf)
            }
        }),
        Op::DrawProgress => gen(w, h, |x, y| {
            let c = at(x, y);
            let (e, _) = sobel(img, x as i32, y as i32);
            let l = lum(c);
            let reveal = a * 1.2;
            let t = smooth(reveal, reveal - 0.1, 1.0 - y as f32 / p.h);
            let line = (e.abs() * 3.0).clamp(0.0, 1.0);
            let ink = 255.0 * (1.0 - line);
            let _ = l;
            mix4([255.0, 255.0, 255.0, c[3]], mix4([ink, ink, ink, c[3]], c, 0.0), 1.0 - t).pipe_reveal(c, t)
        }),
        Op::StrokeTaper => gen(w, h, |x, y| {
            let c = at(x, y);
            let t = x as f32 / p.w;
            let k = 1.0 - a * (1.0 - t) * 0.9;
            [c[0] * k, c[1] * k, c[2] * k, c[3] * k.max(0.0)]
        }),
        // ----------------------------------------------------- patrones
        Op::Stripes | Op::Checker | Op::GridOv | Op::DotsOv | Op::StarMask | Op::RadialRays | Op::GodRays | Op::Voronoi => {
            let n = (det * 1.2 + 4.0).round();
            gen(w, h, |x, y| {
                let c = at(x, y);
                let (u, v) = (x as f32 / p.w * n, y as f32 / p.w * n);
                let m = match op {
                    Op::Stripes => ((u * 2.0).floor() as i32 & 1) as f32,
                    Op::Checker => (((u.floor() + v.floor()) as i32) & 1) as f32,
                    Op::GridOv => (u.fract() < 0.06 || v.fract() < 0.06) as i32 as f32,
                    Op::DotsOv => {
                        let (fx, fy) = (u.fract() - 0.5, v.fract() - 0.5);
                        smooth(0.3, 0.26, fx.hypot(fy))
                    }
                    Op::StarMask => {
                        let (dx, dy) = ((x as f32 - p.cx) / p.m, (y as f32 - p.cy) / p.m);
                        let ang = dy.atan2(dx);
                        let r = 0.2 + 0.2 * (ang * 5.0).cos().abs();
                        smooth(r + 0.02, r - 0.02, dx.hypot(dy) * 1.2)
                    }
                    Op::RadialRays | Op::GodRays => {
                        let ang = (y as f32 - p.cy).atan2(x as f32 - p.cx);
                        let r = (((ang / TAU + 0.5) * n * 2.0).fract() - 0.5).abs() * 2.0;
                        let fall = 1.0 - ((x as f32 - p.cx).hypot(y as f32 - p.cy) / p.rmax).min(1.0);
                        (1.0 - r).powi(3) * fall
                    }
                    _ => {
                        // Voronoi: borde de la celda más cercana.
                        let (iu, iv) = (u.floor(), v.floor());
                        let (mut d1, mut d2) = (9.0f32, 9.0f32);
                        for j in -1..=1 {
                            for i in -1..=1 {
                                let (cx, cy) = (iu + i as f32, iv + j as f32);
                                let (px, py) = (cx + h2(cx as i32, cy as i32), cy + h2(cy as i32 + 7, cx as i32));
                                let d = (px - u).hypot(py - v);
                                if d < d1 { d2 = d1; d1 = d; } else if d < d2 { d2 = d; }
                            }
                        }
                        1.0 - smooth(0.0, 0.08, d2 - d1)
                    }
                };
                match op {
                    Op::Voronoi => over(c, [0.0, 0.0, 0.0], m * a),
                    Op::RadialRays | Op::GodRays => screen(c, [255.0, 240.0, 200.0, 255.0], m * a * 0.9),
                    Op::StarMask => mix4(c, [c[0], c[1], c[2], c[3] * m], a),
                    Op::DotsOv => over(c, [255.0, 255.0, 255.0], m * a * 0.6),
                    _ => over(c, [0.0, 0.0, 0.0], m * a * 0.6),
                }
            })
        }
        Op::Starfield | Op::StarfieldSimple | Op::Particles => {
            let cell = (p.w / (det * 3.0 + 12.0)).max(4.0);
            // Las estrellas se desplazan; las partículas suben.
            let (vx, vy) = match op { Op::Particles => (0.0, -p.h * 0.12), Op::Starfield => (-p.w * 0.08, 0.0), _ => (-p.w * 0.03, 0.0) };
            gen(w, h, |x, y| {
                let c = at(x, y);
                let (x, y) = ((x as f32 - vx * p.t).rem_euclid(p.w) as usize, (y as f32 - vy * p.t).rem_euclid(p.h) as usize);
                let (ix, iy) = ((x as f32 / cell).floor() as i32, (y as f32 / cell).floor() as i32);
                let (sx, sy) = ((ix as f32 + h2(ix, iy)) * cell, (iy as f32 + h2(iy + 3, ix)) * cell);
                let r = ((x as f32 - sx).powi(2) + (y as f32 - sy).powi(2)).sqrt();
                let size = (0.6 + h2(ix + 11, iy + 5) * 1.6) * if op == Op::Particles { 2.2 } else { 1.0 };
                let on = h2(ix + 2, iy + 9) < 0.5 * a + 0.1;
                let tw = 0.65 + 0.35 * (p.t * (2.0 + h2(ix, iy + 1) * 5.0) + h2(iy, ix) * TAU).sin();
                let m = if on { smooth(size, 0.0, r) * tw } else { 0.0 };
                screen(c, [255.0, 255.0, 255.0, 255.0], m)
            })
        }
        Op::Iridescence => gen(w, h, |x, y| {
            let c = at(x, y);
            let t = fbm(x as f32 / p.w * 3.0 + p.t * 0.2, y as f32 / p.w * 3.0, 3) * 360.0 * 2.0 + lum(c) * 200.0 + det * 10.0 + p.t * 60.0;
            let g = hsv(t, 0.55, 1.0);
            [lerp(c[0], c[0] * 0.5 + g[0] * 0.7, a), lerp(c[1], c[1] * 0.5 + g[1] * 0.7, a), lerp(c[2], c[2] * 0.5 + g[2] * 0.7, a), c[3]]
        }),
        Op::Lightning | Op::EnergyBeam => gen(w, h, |x, y| {
            let c = at(x, y);
            let seed = if op == Op::Lightning { (p.t * 12.0).floor() * 3.7 } else { p.t * 1.5 };
            let wob = (fbm(y as f32 / p.h * 6.0 + det, 0.5 + seed, 4) - 0.5) * 0.35 + 0.5;
            let d = ((x as f32 / p.w) - wob).abs();
            let core = smooth(0.012 + a * 0.02, 0.0, d);
            let halo = smooth(0.12, 0.0, d) * 0.6 * a;
            screen(c, [140.0 + 115.0 * core, 190.0 + 65.0 * core, 255.0, 255.0], (core + halo).min(1.0))
        }),
        Op::LensFlare => gen(w, h, |x, y| {
            let c = at(x, y);
            let (fx, fy) = (p.w * 0.7, p.h * 0.3);
            let (dx, dy) = (x as f32 - fx, y as f32 - fy);
            let d = dx.hypot(dy) / p.m;
            let core = smooth(0.18, 0.0, d);
            let ring = smooth(0.02, 0.0, (d - 0.45).abs()) * 0.5;
            let (gx, gy) = ((p.cx - fx) * 2.0 + fx - x as f32, (p.cy - fy) * 2.0 + fy - y as f32);
            let ghost = smooth(0.1, 0.0, gx.hypot(gy) / p.m) * 0.4;
            let m = (core + ring + ghost) * a;
            screen(c, [255.0, 230.0, 180.0, 255.0], m.min(1.0))
        }),
        Op::LongShadow | Op::DropShadow | Op::RadialShadow => {
            let len = if op == Op::LongShadow { a * p.w * 0.25 } else { a * p.w * 0.06 };
            gen(w, h, |x, y| {
                let c = at(x, y);
                if c[3] > 250.0 && op != Op::RadialShadow {
                    // Opaco: oscurece según la luma del vecino proyectado.
                }
                let mut sh = 0.0f32;
                let n = 14;
                for i in 1..=n {
                    let t = i as f32 / n as f32 * len;
                    let (sx, sy) = if op == Op::RadialShadow {
                        let (dx, dy) = (x as f32 - p.cx, y as f32 - p.cy);
                        let d = dx.hypot(dy).max(1.0);
                        (x as f32 - dx / d * t, y as f32 - dy / d * t)
                    } else {
                        (x as f32 - t * 0.7, y as f32 - t * 0.7)
                    };
                    let s = img.bil(sx, sy);
                    sh = sh.max((1.0 - lum(s)) * 0.0 + lum(s) * (1.0 - i as f32 / (n + 1) as f32));
                }
                let k = 1.0 - (sh - lum(c)).max(0.0) * a * 1.2;
                [c[0] * k, c[1] * k, c[2] * k, c[3]]
            })
        }
        // --------------------------------------------------------- ruido
        Op::FilmGrain | Op::NoiseOv | Op::StaticTv => gen(w, h, |x, y| {
            let c = at(x, y);
            let fr = (p.t * 24.0).floor() as i32;
            let n = (h2(x as i32 + det as i32 * 131 + fr * 7919, y as i32 + fr * 104_729) - 0.5) * a * if op == Op::StaticTv { 260.0 } else { 110.0 };
            if op == Op::StaticTv {
                let m = (a * 1.4).min(1.0);
                mix4(c, [128.0 + n, 128.0 + n, 128.0 + n, c[3]], m * 0.8)
            } else {
                [c[0] + n, c[1] + n, c[2] + n, c[3]]
            }
        }),
        Op::Turbulence | Op::Clouds | Op::FractalRidges => gen(w, h, |x, y| {
            let c = at(x, y);
            let (u, v) = (x as f32 / p.w * (det * 0.5 + 1.5) + p.t * 0.15, y as f32 / p.w * (det * 0.5 + 1.5) + p.t * 0.05);
            let n = match op {
                Op::FractalRidges => 1.0 - (fbm(u, v, 5) * 2.0 - 1.0).abs(),
                Op::Turbulence => (fbm(u, v, 5) * 2.0 - 1.0).abs() * 1.6,
                _ => fbm(u, v, 5),
            };
            let g = n * 255.0;
            let blend = if op == Op::Clouds { screen(c, [g, g, g, 255.0], a) } else { mix4(c, [g, g, g, c[3]], a * 0.8) };
            blend
        }),
        Op::Dissolve | Op::BlockDissolve => {
            let blk = if op == Op::BlockDissolve { (p.w / (det * 4.0 + 8.0)).max(2.0) } else { 1.0 };
            gen(w, h, |x, y| {
                let c = at(x, y);
                let n = h2((x as f32 / blk).floor() as i32, (y as f32 / blk).floor() as i32);
                [c[0], c[1], c[2], if n < a { 0.0 } else { c[3] }]
            })
        }
        Op::BlockNoise => {
            let blk = (p.w / (det * 5.0 + 10.0)).max(2.0);
            gen(w, h, |x, y| {
                let c = at(x, y);
                let fr = (p.t * 12.0).floor() as i32;
                let n = (h2((x as f32 / blk).floor() as i32 + fr * 89, (y as f32 / blk).floor() as i32) - 0.5) * a * 160.0;
                [c[0] + n, c[1] + n, c[2] + n, c[3]]
            })
        }
        // ----------------------------------------------------- color
        Op::SolidTint => {
            let col = hsv(det * 18.0, 0.85, 1.0);
            gen(w, h, |x, y| over(at(x, y), col, a))
        }
        Op::GradOver | Op::Grad4 => gen(w, h, |x, y| {
            let c = at(x, y);
            let (u, v) = (x as f32 / p.w, y as f32 / p.h);
            let g = if op == Op::Grad4 {
                let top = [lerp(255.0, 40.0, u), lerp(60.0, 200.0, u), lerp(120.0, 255.0, u)];
                let bot = [lerp(255.0, 60.0, u), lerp(220.0, 40.0, u), lerp(40.0, 220.0, u)];
                [lerp(top[0], bot[0], v), lerp(top[1], bot[1], v), lerp(top[2], bot[2], v)]
            } else {
                hsv(det * 18.0 + v * 120.0, 0.8, 1.0)
            };
            over(c, g, a * 0.65)
        }),
        Op::GradMap | Op::Palette => gen(w, h, |x, y| {
            let c = at(x, y);
            let l = lum(c);
            let g = if op == Op::GradMap {
                [lerp(20.0, 255.0, l) * (1.0 - l * 0.2), lerp(10.0, 190.0, l * l), lerp(80.0, 70.0, l) + l * 100.0]
            } else {
                let q = (l * 4.0).floor().min(3.0);
                [[15.0, 56.0, 15.0], [48.0, 98.0, 48.0], [139.0, 172.0, 15.0], [155.0, 188.0, 15.0]][q as usize]
            };
            over(c, g, a.max(0.05))
        }),
        Op::Dither => {
            const B: [f32; 16] = [0.0, 8.0, 2.0, 10.0, 12.0, 4.0, 14.0, 6.0, 3.0, 11.0, 1.0, 9.0, 15.0, 7.0, 13.0, 5.0];
            gen(w, h, |x, y| {
                let c = at(x, y);
                let t = (B[(y % 4) * 4 + x % 4] + 0.5) / 16.0 - 0.5;
                let lv = 2.0 + det * 0.3;
                let q = |v: f32| ((v / 255.0 + t / lv).clamp(0.0, 1.0) * (lv - 1.0)).round() / (lv - 1.0) * 255.0;
                mix4(c, [q(c[0]), q(c[1]), q(c[2]), c[3]], a.max(0.05))
            })
        }
        Op::SpotColor => gen(w, h, |x, y| {
            let c = at(x, y);
            let (hh, s, _) = to_hsv(c[0], c[1], c[2]);
            let target = det * 18.0;
            let dh = ((hh - target + 540.0) % 360.0 - 180.0).abs();
            let keep = smooth(50.0, 20.0, dh) * s.min(1.0).max(0.0).sqrt();
            let g = lum(c) * 255.0;
            let m = lerp(1.0, keep, a);
            [lerp(g, c[0], m), lerp(g, c[1], m), lerp(g, c[2], m), c[3]]
        }),
        Op::ReplaceColor => gen(w, h, |x, y| {
            let c = at(x, y);
            let (hh, s, v) = to_hsv(c[0], c[1], c[2]);
            let target = 120.0;
            let dh = ((hh - target + 540.0) % 360.0 - 180.0).abs();
            let m = smooth(60.0, 25.0, dh) * s.sqrt();
            let n = hsv(hh + (det - 10.0) * 18.0, s, v);
            [lerp(c[0], n[0], m * a), lerp(c[1], n[1], m * a), lerp(c[2], n[2], m * a), c[3]]
        }),
        Op::RemapRgb => gen(w, h, |x, y| {
            let c = at(x, y);
            let s = (det as usize) % 6;
            let m = match s { 0 => [c[1], c[2], c[0]], 1 => [c[2], c[0], c[1]], 2 => [c[0], c[2], c[1]], 3 => [c[2], c[1], c[0]], 4 => [c[1], c[0], c[2]], _ => [255.0 - c[0], c[1], c[2]] };
            mix4(c, [m[0], m[1], m[2], c[3]], a)
        }),
        Op::RemapHsv => gen(w, h, |x, y| {
            let c = at(x, y);
            let (hh, s, v) = to_hsv(c[0], c[1], c[2]);
            let n = hsv(hh + a * 180.0, (s + det * 0.02).min(1.0), v);
            [n[0], n[1], n[2], c[3]]
        }),
        Op::Mixer => gen(w, h, |x, y| {
            let c = at(x, y);
            let k = a * 0.6;
            [c[0] * (1.0 - k) + c[1] * k, c[1] * (1.0 - k) + c[2] * k, c[2] * (1.0 - k) + c[0] * k, c[3]]
        }),
        Op::Difference => gen(w, h, |x, y| {
            let c = at(x, y);
            let s = img.bil(x as f32 - det * 2.0, y as f32 - det);
            let m = [(c[0] - s[0]).abs() * 2.0, (c[1] - s[1]).abs() * 2.0, (c[2] - s[2]).abs() * 2.0, c[3]];
            mix4(c, m, a)
        }),
        // ----------------------------------------------------- claves
        Op::Chroma | Op::Spill | Op::Fringe => gen(w, h, |x, y| {
            let c = at(x, y);
            let (hh, s, _) = to_hsv(c[0], c[1], c[2]);
            // Clave verde por defecto; `det` desplaza el tono (0..20 → 60..240°).
            let target = 60.0 + (det - 1.0) / 19.0 * 180.0;
            let dh = ((hh - target + 540.0) % 360.0 - 180.0).abs();
            let m = smooth(55.0, 20.0, dh) * smooth(0.15, 0.4, s);
            match op {
                Op::Chroma => [c[0], c[1], c[2], c[3] * (1.0 - m * a.max(0.05))],
                _ => {
                    let g = (c[0] + c[2]) * 0.5;
                    let n = [c[0], c[1].min(g), c[2], c[3]];
                    mix4(c, n, m * a)
                }
            }
        }),
        Op::LumaKey | Op::LumaStamp => gen(w, h, |x, y| {
            let c = at(x, y);
            let l = lum(c);
            let k = smooth(a - 0.05, a + 0.05, l);
            if op == Op::LumaKey { [c[0], c[1], c[2], c[3] * k] } else { [c[0], c[1], c[2], c[3] * (1.0 - k)] }
        }),
        Op::DiffMatte => gen(w, h, |x, y| {
            let c = at(x, y);
            let bg = img.at(0, 0);
            let d = ((c[0] - bg[0]).abs() + (c[1] - bg[1]).abs() + (c[2] - bg[2]).abs()) / 765.0;
            [c[0], c[1], c[2], c[3] * smooth(a * 0.4, a * 0.4 + 0.1, d)]
        }),
        Op::SolidMatte => gen(w, h, |x, y| {
            let c = at(x, y);
            let k = c[3] / 255.0;
            let bg = hsv(det * 18.0, 0.5, 0.4);
            [lerp(bg[0], c[0], k), lerp(bg[1], c[1], k), lerp(bg[2], c[2], k), 255.0]
        }),
        Op::Crop => gen(w, h, |x, y| {
            let c = at(x, y);
            let (mx, my) = (a * 0.5 * p.w * 0.8, a * 0.5 * p.h * 0.8);
            let inside = x as f32 >= mx && (x as f32) < p.w - mx && y as f32 >= my && (y as f32) < p.h - my;
            [c[0], c[1], c[2], if inside { c[3] } else { 0.0 }]
        }),
        Op::Wipe => gen(w, h, |x, y| {
            let c = at(x, y);
            let ang = det * 18.0f32.to_radians();
            let t = ((x as f32 / p.w - 0.5) * ang.cos() + (y as f32 / p.h - 0.5) * ang.sin()) + 0.5;
            [c[0], c[1], c[2], c[3] * smooth(a - 0.02, a + 0.02, 1.0 - t).min(1.0).max(0.0)]
        }),
        Op::RadialWipe => gen(w, h, |x, y| {
            let c = at(x, y);
            let ang = ((y as f32 - p.cy).atan2(x as f32 - p.cx) / TAU + 0.75).fract();
            [c[0], c[1], c[2], if ang <= a { c[3] } else { 0.0 }]
        }),
        Op::Feather | Op::SmoothEdges => {
            let b = bl(img, (a * 0.03 * p.w) as usize + 1);
            let bi = Img { d: &b, w, h };
            gen(w, h, |x, y| {
                let c = at(x, y);
                let al = bi.at(x as i32, y as i32)[3];
                [c[0], c[1], c[2], if op == Op::Feather { al } else { c[3] * smooth(100.0, 200.0, al) }]
            })
        }
        Op::Choker => {
            let r = (a * 0.01 * p.w) as i32 + 1;
            gen(w, h, |x, y| {
                let c = at(x, y);
                let mut mn = 255.0f32;
                for dy in [-r, 0, r] {
                    for dx in [-r, 0, r] {
                        mn = mn.min(img.at(x as i32 + dx, y as i32 + dy)[3]);
                    }
                }
                [c[0], c[1], c[2], mn.min(c[3])]
            })
        }
        _ => crate::fx_3d::run(op, img, p),
    }
}

trait Reveal {
    fn pipe_reveal(self, orig: [f32; 4], t: f32) -> [f32; 4];
}
impl Reveal for [f32; 4] {
    fn pipe_reveal(self, orig: [f32; 4], t: f32) -> [f32; 4] {
        mix4(self, orig, t)
    }
}
