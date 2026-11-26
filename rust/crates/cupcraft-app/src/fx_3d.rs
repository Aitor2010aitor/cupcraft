//! Sólidos 3D proyectados: el fotograma se usa como textura en la cara
//! frontal/superior/lateral de poliedros convexos (rayos contra planos) y
//! elipsoide/toro analíticos. El fondo queda transparente (alfa 0).

use crate::fx_engine::*;
use std::f32::consts::{PI, TAU};

type V3 = [f32; 3];

fn dot(a: V3, b: V3) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn norm(a: V3) -> V3 {
    let l = dot(a, a).sqrt().max(1e-6);
    [a[0] / l, a[1] / l, a[2] / l]
}
/// Rota alrededor de Y y luego de X.
fn rot_yx(v: V3, ay: f32, ax: f32) -> V3 {
    let (sy, cy) = ay.sin_cos();
    let (x, z) = (v[0] * cy + v[2] * sy, -v[0] * sy + v[2] * cy);
    let (sx, cx) = ax.sin_cos();
    [x, v[1] * cx - z * sx, v[1] * sx + z * cx]
}

/// Planos (normal unitaria, distancia) de un poliedro convexo.
fn planes(op: Op) -> Vec<(V3, f32)> {
    let ax = |n: V3, d: f32| (norm(n), d);
    match op {
        Op::Cube => vec![ax([1., 0., 0.], 0.55), ax([-1., 0., 0.], 0.55), ax([0., 1., 0.], 0.55), ax([0., -1., 0.], 0.55), ax([0., 0., 1.], 0.55), ax([0., 0., -1.], 0.55)],
        Op::Box => vec![ax([1., 0., 0.], 0.85), ax([-1., 0., 0.], 0.85), ax([0., 1., 0.], 0.5), ax([0., -1., 0.], 0.5), ax([0., 0., 1.], 0.35), ax([0., 0., -1.], 0.35)],
        Op::Octahedron => {
            let mut v = vec![];
            for sx in [-1., 1.] {
                for sy in [-1., 1.] {
                    for sz in [-1., 1.] {
                        v.push(ax([sx, sy, sz], 0.62));
                    }
                }
            }
            v
        }
        Op::Pyramid => {
            let s = 0.894;
            vec![ax([0., -1., 0.], 0.5), ax([s, 0.447, 0.], 0.35), ax([-s, 0.447, 0.], 0.35), ax([0., 0.447, s], 0.35), ax([0., 0.447, -s], 0.35)]
        }
        Op::HexPrism => {
            let mut v = vec![ax([0., 0., 1.], 0.45), ax([0., 0., -1.], 0.45)];
            for i in 0..6 {
                let a = i as f32 * PI / 3.0;
                v.push(ax([a.cos(), a.sin(), 0.], 0.62));
            }
            v
        }
        Op::Cylinder => {
            let mut v = vec![ax([0., 1., 0.], 0.7), ax([0., -1., 0.], 0.7)];
            for i in 0..28 {
                let a = i as f32 * TAU / 28.0;
                v.push(ax([a.cos(), 0., a.sin()], 0.5));
            }
            v
        }
        Op::StarPoly | Op::Cross3 | Op::StarPrism | Op::HollowBox => vec![ax([1., 0., 0.], 0.6), ax([-1., 0., 0.], 0.6), ax([0., 1., 0.], 0.6), ax([0., -1., 0.], 0.6), ax([0., 0., 1.], 0.6), ax([0., 0., -1.], 0.6)],
        _ => vec![],
    }
}

/// Ángulos (guiño, cabeceo) a partir de cantidad y detalle.
fn angles(p: &P) -> (f32, f32) {
    (p.a * PI * 0.9 + 0.5, 0.45 * p.a + (p.d - 8.0) * 0.03)
}

pub fn run(op: Op, img: &Img, p: &P) -> Vec<u8> {
    let (w, h) = (img.w, img.h);
    let (ay, ax) = angles(p);
    let cam = [0.0f32, 0.0, -2.6];
    let focal = 1.9;
    let aspect = p.w / p.h;
    let planes_v = planes(op);
    let is_ell = op == Op::Ellipsoid;
    let is_tor = op == Op::Torus;
    if planes_v.is_empty() && !is_ell && !is_tor {
        return img.d.to_vec();
    }
    gen(w, h, |x, y| {
        let u = (x as f32 + 0.5) / p.w * 2.0 - 1.0;
        let v = 1.0 - (y as f32 + 0.5) / p.h * 2.0;
        // Rayo en el espacio del objeto: aplicamos la rotación inversa a cámara y dirección.
        let dir = norm([u * aspect, v, focal]);
        let o = rot_yx(cam, -ay, -ax);
        let dr = rot_yx(dir, -ay, -ax);
        let sky = [0.0, 0.0, 0.0, 0.0];
        if is_ell {
            // Elipsoide con semiejes (1.0*aspect, 1.0, 1.0)*0.7.
            let r = [0.75 * aspect.min(1.4), 0.75, 0.75];
            let oo = [o[0] / r[0], o[1] / r[1], o[2] / r[2]];
            let dd = [dr[0] / r[0], dr[1] / r[1], dr[2] / r[2]];
            let (a2, b2, c2) = (dot(dd, dd), dot(oo, dd), dot(oo, oo) - 1.0);
            let disc = b2 * b2 - a2 * c2;
            if disc < 0.0 {
                return sky;
            }
            let t = (-b2 - disc.sqrt()) / a2;
            let hit = [oo[0] + dd[0] * t, oo[1] + dd[1] * t, oo[2] + dd[2] * t];
            let lon = hit[0].atan2(-hit[2]);
            let lat = hit[1].clamp(-1.0, 1.0).asin();
            let c = img.bil((lon / PI * 0.5 + 0.5) * (p.w - 1.0), (0.5 - lat / PI) * (p.h - 1.0));
            let light = 0.45 + 0.55 * dot(norm(hit), norm([-0.4, 0.6, -0.7])).max(0.0);
            return [c[0] * light, c[1] * light, c[2] * light, c[3]];
        }
        if is_tor {
            let (big, small) = (0.62f32, 0.26f32);
            let mut t = 0.8f32;
            for _ in 0..70 {
                let q = [o[0] + dr[0] * t, o[1] + dr[1] * t, o[2] + dr[2] * t];
                let ring = (q[0] * q[0] + q[2] * q[2]).sqrt() - big;
                let d = (ring * ring + q[1] * q[1]).sqrt() - small;
                if d < 0.002 {
                    let ang = q[2].atan2(q[0]);
                    let tub = q[1].atan2(ring);
                    let c = img.bil(((ang / TAU + 0.5) * 2.0).fract() * (p.w - 1.0), ((tub / TAU + 0.5) * 1.0).fract() * (p.h - 1.0));
                    let nrm = norm([q[0] - big * q[0] / (q[0] * q[0] + q[2] * q[2]).sqrt().max(1e-5), q[1], q[2] - big * q[2] / (q[0] * q[0] + q[2] * q[2]).sqrt().max(1e-5)]);
                    let light = 0.4 + 0.6 * dot(nrm, norm([-0.4, 0.6, -0.7])).max(0.0);
                    return [c[0] * light, c[1] * light, c[2] * light, c[3]];
                }
                t += d * 0.9;
                if t > 6.0 {
                    break;
                }
            }
            return sky;
        }
        // Poliedro convexo: intersección de semiespacios (método de las losas).
        let (mut t0, mut t1) = (0.0f32, 1e9f32);
        let mut face = usize::MAX;
        for (i, (n, d)) in planes_v.iter().enumerate() {
            let denom = dot(*n, dr);
            let num = d - dot(*n, o);
            if denom.abs() < 1e-6 {
                if num < 0.0 {
                    return sky;
                }
                continue;
            }
            let t = num / denom;
            if denom < 0.0 {
                if t > t0 {
                    t0 = t;
                    face = i;
                }
            } else if t < t1 {
                t1 = t;
            }
            if t0 > t1 {
                return sky;
            }
        }
        if face == usize::MAX {
            return sky;
        }
        let hit = [o[0] + dr[0] * t0, o[1] + dr[1] * t0, o[2] + dr[2] * t0];
        let (n, _) = planes_v[face];
        // Base ortonormal de la cara para proyectar la textura.
        let up = if n[1].abs() > 0.9 { [0.0, 0.0, 1.0] } else { [0.0, 1.0, 0.0] };
        let tx = norm([up[1] * n[2] - up[2] * n[1], up[2] * n[0] - up[0] * n[2], up[0] * n[1] - up[1] * n[0]]);
        let ty = norm([n[1] * tx[2] - n[2] * tx[1], n[2] * tx[0] - n[0] * tx[2], n[0] * tx[1] - n[1] * tx[0]]);
        let rel = sub(hit, [0.0, 0.0, 0.0]);
        let (su, sv) = (dot(rel, tx) / 1.1 + 0.5, 0.5 - dot(rel, ty) / 1.1);
        let c = img.bil(su.rem_euclid(1.0) * (p.w - 1.0), sv.rem_euclid(1.0) * (p.h - 1.0));
        let light = 0.45 + 0.55 * dot(n, norm(rot_yx([-0.4, 0.6, -0.7], -ay, -ax))).max(0.0);
        [c[0] * light, c[1] * light, c[2] * light, c[3]]
    })
}
