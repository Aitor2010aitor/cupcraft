//! Animaciones de clip (Entrada / Salida / Combinado) del inspector.
//!
//! El inspector guarda en el clip el id CapCut elegido (`animIn`, `animOut`,
//! `animCombo`) y opcionalmente su duración (`animInDur`, `animOutDur`). Aquí se
//! traduce ese id a un movimiento procedural según el nombre del catálogo y se
//! evalúa en el tiempo local; la vista previa y el export usan la misma función.

use crate::anim_catalog::{ANIM_COMBO, ANIM_IN, ANIM_OUT};
use cupcraft_core::Clip;

/// Resultado de evaluar las animaciones del clip en un instante.
#[derive(Clone, Copy, Debug)]
pub struct ClipAnim {
    pub alpha: f32,
    /// Desplazamiento como fracción del ancho / alto del lienzo.
    pub dx: f32,
    pub dy: f32,
    pub scale: f32,
    /// Grados.
    pub rot: f32,
}

impl ClipAnim {
    pub const NONE: ClipAnim = ClipAnim { alpha: 1.0, dx: 0.0, dy: 0.0, scale: 1.0, rot: 0.0 };
    pub fn is_identity(&self) -> bool {
        (self.alpha - 1.0).abs() < 1e-4 && self.dx == 0.0 && self.dy == 0.0 && (self.scale - 1.0).abs() < 1e-4 && self.rot == 0.0
    }
}

fn name_of(list: &'static [(&'static str, &'static str, &'static str)], id: &str) -> Option<String> {
    list.iter().find(|(i, _, _)| *i == id).map(|(_, n, _)| n.to_lowercase())
}

fn selected(clip: &Clip, key: &str, list: &'static [(&'static str, &'static str, &'static str)]) -> Option<String> {
    let id = clip.extra.get(key).and_then(|v| v.as_str()).filter(|s| !s.is_empty())?;
    // Ids nuevos; si el proyecto guardó un nombre (versión anterior), también vale.
    name_of(list, id).or_else(|| list.iter().find(|(_, n, _)| n.eq_ignore_ascii_case(id)).map(|(_, n, _)| n.to_lowercase()))
}

fn has(n: &str, keys: &[&str]) -> bool {
    keys.iter().any(|k| n.contains(k))
}

fn ease_out(p: f32) -> f32 {
    1.0 - (1.0 - p).powi(3)
}

/// Estado de entrada con `p` en 0 (inicio) → 1 (reposo). `out` invierte el
/// sentido de los desplazamientos (sale hacia donde indica el nombre).
fn pose(n: &str, p: f32, out: bool) -> ClipAnim {
    let q = 1.0 - ease_out(p.clamp(0.0, 1.0)); // 1 = lejos del reposo
    let mut a = ClipAnim::NONE;
    let toward = n.contains("hacia");
    // Signo del desplazamiento respecto al reposo en el extremo "fuera".
    let vec = |side: f32| -> f32 { if out || !toward { side } else { -side } };
    let mut moved = false;
    if has(n, &["izquierda"]) {
        a.dx = vec(-1.0) * q * 0.6;
        moved = true;
    } else if has(n, &["derecha"]) {
        a.dx = vec(1.0) * q * 0.6;
        moved = true;
    }
    if has(n, &["arriba"]) {
        a.dy = vec(-1.0) * q * 0.6;
        moved = true;
    } else if has(n, &["abajo"]) {
        a.dy = vec(1.0) * q * 0.6;
        moved = true;
    }
    if has(n, &["rotaci", "giro", "espiral", "lavadora", "remolino", "gira"]) {
        a.rot = q * 180.0 * if out { 1.0 } else { -1.0 };
        a.scale = 1.0 - q * 0.7;
        a.alpha = 1.0 - q;
    } else if has(n, &["rebote", "resorte", "resortera", "primavera", "salt", "búm", "bum", "golpe"]) {
        // Sobreimpulso elástico.
        let e = 1.0 - (1.0 - p.clamp(0.0, 1.0)).powi(2) * (p * 9.0).cos();
        a.scale = if out { 1.0 - q * 0.9 } else { e.clamp(0.0, 1.35) };
        a.alpha = 1.0 - q;
    } else if has(n, &["alej"]) {
        a.scale = 1.0 + q * 0.8;
        a.alpha = 1.0 - q;
    } else if has(n, &["zoom", "acerca", "ampli"]) {
        a.scale = 1.0 - q * 0.7;
        a.alpha = 1.0 - q;
    } else if has(n, &["disoluci", "aparici", "fundido", "mezclar", "fusi", "amanecer", "crepúsculo"]) {
        a.alpha = 1.0 - q;
    } else if !moved {
        a.alpha = 1.0 - q;
        a.scale = 1.0 - q * 0.2;
    } else {
        a.alpha = 1.0 - q * 0.6;
    }
    a
}

/// Animación combinada: movimiento continuo a lo largo del clip.
fn combo(n: &str, local: f64, dur: f64) -> ClipAnim {
    let t = local as f32;
    let mut a = ClipAnim::NONE;
    let p = (local / dur.max(0.05)).clamp(0.0, 1.0) as f32;
    if has(n, &["rotaci", "giro", "espiral", "lavadora", "remolino"]) {
        a.rot = p * 360.0;
    } else if has(n, &["zoom", "acerca", "ampli", "pulso", "latido"]) {
        a.scale = 1.0 + 0.12 * (t * 5.0).sin().abs();
    } else if has(n, &["temblor", "vibra", "agita", "shake"]) {
        a.dx = (t * 40.0).sin() * 0.008;
        a.dy = (t * 53.0).cos() * 0.008;
    } else if has(n, &["izquierda", "derecha", "balanceo", "péndulo", "pendulo"]) {
        a.dx = (t * 3.0).sin() * 0.04;
        a.rot = (t * 3.0).sin() * 4.0;
    } else if has(n, &["arriba", "abajo", "rebote", "salt"]) {
        a.dy = -(t * 6.0).sin().abs() * 0.05;
    } else {
        a.scale = 1.0 + 0.05 * (t * 3.0).sin();
        a.rot = (t * 2.0).sin() * 2.0;
    }
    a
}

/// Evalúa las animaciones de entrada, salida y combinada del clip en `local`.
pub fn eval(clip: &Clip, local: f64) -> ClipAnim {
    let mut r = ClipAnim::NONE;
    let dur = clip.duration.max(0.05);
    let get_d = |k: &str| clip.extra.get(k).and_then(|v| v.as_f64()).unwrap_or(0.6).clamp(0.1, dur / 2.0);

    if let Some(n) = selected(clip, "animCombo", ANIM_COMBO) {
        let c = combo(&n, local, dur);
        r = mul(r, c);
    }
    if let Some(n) = selected(clip, "animIn", ANIM_IN) {
        let d = get_d("animInDur");
        if local < d {
            r = mul(r, pose(&n, (local / d) as f32, false));
        }
    }
    if let Some(n) = selected(clip, "animOut", ANIM_OUT) {
        let d = get_d("animOutDur");
        let rem = dur - local;
        if rem < d {
            r = mul(r, pose(&n, (rem / d) as f32, true));
        }
    }
    r
}

fn mul(a: ClipAnim, b: ClipAnim) -> ClipAnim {
    ClipAnim { alpha: a.alpha * b.alpha, dx: a.dx + b.dx, dy: a.dy + b.dy, scale: a.scale * b.scale, rot: a.rot + b.rot }
}

/// Muestra en bucle para la vista previa al pasar el ratón por la tarjeta:
/// `sub` es "in" | "out" | otro (combinado), `t` el reloj de la UI en segundos.
pub fn preview(sub: &str, id: &str, t: f64) -> ClipAnim {
    const CYCLE: f64 = 1.8;
    const D: f64 = 0.6;
    let l = t.rem_euclid(CYCLE);
    match sub {
        "in" => name_of(ANIM_IN, id).map(|n| pose(&n, (l / D).min(1.0) as f32, false)).unwrap_or(ClipAnim::NONE),
        "out" => name_of(ANIM_OUT, id)
            .map(|n| pose(&n, ((CYCLE - l) / D).min(1.0) as f32, true))
            .unwrap_or(ClipAnim::NONE),
        _ => name_of(ANIM_COMBO, id).map(|n| combo(&n, l, CYCLE)).unwrap_or(ClipAnim::NONE),
    }
}
