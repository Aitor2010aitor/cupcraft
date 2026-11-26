//! Efectos de texto (categoría "Texto"): modifican el contenido/estilo del
//! texto en función del tiempo local del clip. Los que mueven cada letra
//! (sacudida, transformar, destellos, a lo largo de ruta) dejan una orden
//! `_glyphFx` que consume `raster::render_text` al colocar los glifos.

use cupcraft_core::Clip;
use serde_json::{json, Value};

fn prm(clip: &Clip, key: &str, def: f64) -> f64 {
    clip.extra
        .get("effectParams")
        .and_then(|p| p.get(key))
        .and_then(Value::as_f64)
        .unwrap_or(def)
}

fn h(i: u64, s: u64) -> f64 {
    let mut x = i.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ s.wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    x ^= x >> 31;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 29;
    (x >> 11) as f64 / (1u64 << 53) as f64
}

/// Devuelve el texto modificado por el efecto de texto activo del clip, o
/// `None` si el clip no tiene efecto de texto.
pub fn apply(val: &Value, clip: &Clip, local: f64) -> Option<Value> {
    let id = clip.extra.get("effect").and_then(Value::as_str)?;
    if !id.starts_with("txt_") {
        return None;
    }
    let content = val.get("content").and_then(Value::as_str).unwrap_or("").to_string();
    let dur = clip.duration.max(0.001);
    let mut out = val.clone();
    let set = |out: &mut Value, k: &str, v: Value| {
        if let Some(o) = out.as_object_mut() {
            o.insert(k.to_string(), v);
        }
    };
    match id {
        "txt_progress" => {
            let span = dur * prm(clip, "amount", 40.0).clamp(1.0, 100.0) / 100.0;
            let f = (local / span).clamp(0.0, 1.0) as f32;
            if f < 0.999 {
                out = crate::services::raster::reveal_text(val, f);
            }
        }
        "txt_randomizer" => {
            const CH: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789#$%&@*+=?";
            let span = dur * prm(clip, "amount", 40.0).clamp(1.0, 100.0) / 100.0;
            let chars: Vec<char> = content.chars().collect();
            let n = chars.len().max(1) as f64;
            let frame = (local * 20.0).floor() as u64;
            let s: String = chars
                .iter()
                .enumerate()
                .map(|(i, &c)| {
                    let settle = span * (i as f64 + 1.0) / n;
                    if c.is_whitespace() || local >= settle {
                        c
                    } else {
                        let k = (h(i as u64, frame) * CH.len() as f64) as usize % CH.len();
                        let r = CH[k] as char;
                        if c.is_lowercase() { r.to_ascii_lowercase() } else { r }
                    }
                })
                .collect();
            set(&mut out, "content", json!(s));
        }
        "txt_countupdown" => {
            let from = prm(clip, "from", 0.0);
            let to = prm(clip, "to", 100.0);
            let dec = prm(clip, "decimals", 0.0).clamp(0.0, 3.0) as usize;
            let f = (local / dur).clamp(0.0, 1.0);
            let e = 1.0 - (1.0 - f).powi(3);
            let v = from + (to - from) * e;
            set(&mut out, "content", json!(format!("{:.*}", dec, v)));
        }
        "txt_timecode" => {
            let fps = prm(clip, "fps", 30.0).clamp(1.0, 60.0);
            let total = (local.max(0.0) * fps).floor() as u64;
            let f = fps.round() as u64;
            let ff = total % f.max(1);
            let secs = total / f.max(1);
            let s = format!("{:02}:{:02}:{:02}:{:02}", secs / 3600, (secs / 60) % 60, secs % 60, ff);
            set(&mut out, "content", json!(s));
        }
        "txt_fontweight" => {
            set(&mut out, "weight", json!(prm(clip, "weight", 800.0).clamp(100.0, 900.0)));
        }
        "txt_spacing" => {
            set(&mut out, "letterSpacing", json!(prm(clip, "spacing", 12.0) * 0.01));
        }
        "txt_shake" | "txt_transform" | "txt_blips" | "txt_alongpath" => {
            let kind = &id[4..];
            // Tiempo cuantizado a 60 fps para que la caché de texturas no crezca sin fin.
            let t = (local * 60.0).round() / 60.0;
            set(
                &mut out,
                "_glyphFx",
                json!({ "kind": kind, "amount": prm(clip, "amount", 50.0), "detail": prm(clip, "detail", 8.0), "t": t }),
            );
        }
        _ => return None,
    }
    Some(out)
}

/// Desplazamiento (dx, dy en px), escala y visibilidad de la letra `i` de `n`
/// situada en `x` (px desde la izquierda del bloque) para la orden `_glyphFx`.
pub fn glyph_motion(fx: &Value, i: usize, n: usize, x: f32, px: f32) -> (f32, f32, f32, bool) {
    let kind = fx.get("kind").and_then(Value::as_str).unwrap_or("");
    let a = (fx.get("amount").and_then(Value::as_f64).unwrap_or(50.0) / 100.0) as f32;
    let d = fx.get("detail").and_then(Value::as_f64).unwrap_or(8.0) as f32;
    let t = fx.get("t").and_then(Value::as_f64).unwrap_or(0.0) as f32;
    let fi = i as f32;
    match kind {
        "shake" => {
            let fr = (t * d * 3.0).floor() as u64;
            let dx = (h(i as u64, fr) as f32 - 0.5) * 2.0;
            let dy = (h(i as u64 + 977, fr) as f32 - 0.5) * 2.0;
            (dx * a * px * 0.18, dy * a * px * 0.18, 1.0, true)
        }
        "transform" => {
            // Cada letra entra escalonada: sube, crece y se asienta.
            let stagger = 0.6 / n.max(1) as f32;
            let p = ((t * d * 0.25) - fi * stagger * d * 0.25).clamp(0.0, 1.0);
            let e = 1.0 - (1.0 - p).powi(3);
            let wave = (t * d * 0.6 + fi * 0.7).sin() * a * px * 0.05;
            ((1.0 - e) * a * px * 0.3, (1.0 - e) * a * px * 1.2 + wave, 0.3 + 0.7 * e, p > 0.0)
        }
        "blips" => {
            let fr = (t * d * 2.0).floor() as u64;
            let on = h(i as u64, fr) as f32 >= a * 0.7;
            let pop = if h(i as u64 + 31, fr) < 0.15 { 1.25 } else { 1.0 };
            (0.0, 0.0, pop, on)
        }
        "alongpath" => {
            let ph = x / px.max(1.0) * 0.55 + t * d * 0.5;
            (0.0, ph.sin() * a * px * 0.9, 1.0, true)
        }
        _ => (0.0, 0.0, 1.0, true),
    }
}

/// Margen extra (px) que necesita el bloque para que las letras no se corten.
pub fn glyph_margin(fx: &Value, px: f32) -> f32 {
    let a = (fx.get("amount").and_then(Value::as_f64).unwrap_or(50.0) / 100.0) as f32;
    px * (0.1 + a * 1.3)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clip(effect: &str, params: Value) -> Clip {
        let mut c: Clip = serde_json::from_value(json!({
            "id": "t", "kind": "text", "name": "t", "start": 0.0, "duration": 10.0,
            "inPoint": 0.0, "speed": 1.0, "volume": 1.0, "opacity": 1.0, "scale": 1.0,
            "x": 0.0, "y": 0.0, "rotate": 0.0, "filter": "none",
            "adjust": { "brightness": 100.0, "contrast": 100.0, "saturate": 100.0 },
            "fadeIn": 0.0, "fadeOut": 0.0
        }))
        .expect("clip mínimo");
        c.extra.insert("effect".into(), json!(effect));
        c.extra.insert("effectParams".into(), params);
        c
    }

    #[test]
    fn efectos_de_texto_cambian_con_el_tiempo() {
        let v = json!({ "content": "HOLA MUNDO" });
        let c = clip("txt_countupdown", json!({ "from": 0, "to": 100, "decimals": 0 }));
        assert_eq!(apply(&v, &c, 0.0).unwrap()["content"], "0");
        assert_eq!(apply(&v, &c, 10.0).unwrap()["content"], "100");
        let c = clip("txt_timecode", json!({ "fps": 30 }));
        assert_eq!(apply(&v, &c, 61.5).unwrap()["content"], "00:01:01:15");
        let c = clip("txt_randomizer", json!({ "amount": 50 }));
        assert_ne!(apply(&v, &c, 0.1).unwrap()["content"], "HOLA MUNDO");
        assert_eq!(apply(&v, &c, 6.0).unwrap()["content"], "HOLA MUNDO");
        let c = clip("txt_shake", json!({}));
        let out = apply(&v, &c, 1.0).unwrap();
        assert!(out.get("_glyphFx").is_some());
        assert!(apply(&v, &clip("col_hotcolor", json!({})), 1.0).is_none());
    }
}
