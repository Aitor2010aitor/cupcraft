//! Evaluación e interpolación lineal de fotogramas clave (keyframes).
//!
//! Equivalente exacto a `src/keyframes.js`.
//! Permite animar propiedades continuas de los clips en el tiempo.

use crate::model::Clip;
use serde::{Deserialize, Serialize};

pub const EPSILON: f64 = 1.0 / 120.0;

/// Propiedades soportadas para fotogramas clave.
pub const KEYFRAME_PROPS: &[&str] = &[
    "x",
    "y",
    "scale",
    "scaleX",
    "scaleY",
    "rotate",
    "opacity",
    "text.opacity",
];

/// Un punto de fotograma clave con tiempo absoluto y valor numérico.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct KeyframePoint {
    pub t: f64,
    pub v: f64,
}

/// Transformación completa evaluada en un instante de tiempo.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Transform {
    pub x: f64,
    pub y: f64,
    pub rotate: f64,
    pub scale_x: f64,
    pub scale_y: f64,
    pub opacity: f64,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            rotate: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
            opacity: 1.0,
        }
    }
}

/// Obtiene la lista ordenada de fotogramas clave para una propiedad dada.
pub fn keyframes_of(clip: &Clip, prop: &str) -> Option<Vec<KeyframePoint>> {
    let raw = clip.extra.get("keyframes")?;
    let list = raw.get(prop)?.as_array()?;
    let mut points = Vec::with_capacity(list.len());
    for item in list {
        if let (Some(t), Some(v)) = (item.get("t").and_then(|x| x.as_f64()), item.get("v").and_then(|x| x.as_f64())) {
            points.push(KeyframePoint { t, v });
        }
    }
    if points.is_empty() {
        None
    } else {
        points.sort_by(|a, b| a.t.partial_cmp(&b.t).unwrap_or(std::cmp::Ordering::Equal));
        Some(points)
    }
}

/// Comprueba si un clip tiene fotogramas clave para una propiedad específica.
pub fn has_keyframes(clip: &Clip, prop: &str) -> bool {
    keyframes_of(clip, prop).is_some()
}

/// Comprueba si el clip contiene al menos un fotograma clave en cualquier propiedad.
pub fn clip_has_keyframes(clip: &Clip) -> bool {
    let Some(raw) = clip.extra.get("keyframes").and_then(|v| v.as_object()) else {
        return false;
    };
    raw.values().any(|v| v.as_array().map_or(false, |arr| !arr.is_empty()))
}

/// Interpola linealmente sobre una lista de puntos ordenada en el tiempo.
pub fn interpolate(points: &[KeyframePoint], time: f64) -> Option<f64> {
    if points.is_empty() {
        return None;
    }
    if time <= points[0].t {
        return Some(points[0].v);
    }
    let last = points[points.len() - 1];
    if time >= last.t {
        return Some(last.v);
    }
    for i in 1..points.len() {
        let next = points[i];
        if time <= next.t {
            let prev = points[i - 1];
            let span = next.t - prev.t;
            let progress = if span > 0.0 { (time - prev.t) / span } else { 1.0 };
            return Some(prev.v + (next.v - prev.v) * progress);
        }
    }
    Some(last.v)
}

/// Evalúa el valor de una propiedad en el instante `time`, cayendo a `fallback` si no hay keyframes.
pub fn keyframe_value(clip: &Clip, prop: &str, time: f64, fallback: f64) -> f64 {
    match keyframes_of(clip, prop) {
        Some(points) => interpolate(&points, time).unwrap_or(fallback),
        None => fallback,
    }
}

/// Inserta o actualiza un fotograma clave en el clip.
pub fn set_keyframe(clip: &mut Clip, prop: &str, time: f64, value: f64) {
    let keyframes_val = clip
        .extra
        .entry("keyframes".to_string())
        .or_insert_with(|| serde_json::json!({}));
    let Some(kf_map) = keyframes_val.as_object_mut() else {
        return;
    };

    let list_val = kf_map.entry(prop.to_string()).or_insert_with(|| serde_json::json!([]));
    let Some(list) = list_val.as_array_mut() else {
        return;
    };

    let mut points: Vec<KeyframePoint> = list
        .iter()
        .filter_map(|p| {
            let t = p.get("t")?.as_f64()?;
            let v = p.get("v")?.as_f64()?;
            Some(KeyframePoint { t, v })
        })
        .collect();

    if let Some(existing) = points.iter_mut().find(|p| (p.t - time).abs() <= EPSILON) {
        existing.v = value;
    } else {
        points.push(KeyframePoint { t: time, v: value });
    }
    points.sort_by(|a, b| a.t.partial_cmp(&b.t).unwrap_or(std::cmp::Ordering::Equal));

    *list = points
        .into_iter()
        .map(|p| serde_json::json!({ "t": p.t, "v": p.v }))
        .collect();
}

/// Elimina un fotograma clave en el instante `time`.
pub fn clear_keyframe(clip: &mut Clip, prop: &str, time: f64) {
    let Some(keyframes_val) = clip.extra.get_mut("keyframes").and_then(|v| v.as_object_mut()) else {
        return;
    };
    let Some(list_val) = keyframes_val.get_mut(prop).and_then(|v| v.as_array_mut()) else {
        return;
    };
    list_val.retain(|p| {
        p.get("t")
            .and_then(|x| x.as_f64())
            .map_or(true, |t| (t - time).abs() > EPSILON)
    });
}

/// Elimina todos los fotogramas clave del clip.
pub fn clear_all_keyframes(clip: &mut Clip) {
    clip.extra.remove("keyframes");
}

/// Calcula la transformación completa de un clip en el instante `time`.
pub fn transform_at(clip: &Clip, time: f64) -> Transform {
    let scale = keyframe_value(clip, "scale", time, clip.scale);
    let scale_x = keyframe_value(clip, "scaleX", time, scale);
    let scale_y = keyframe_value(clip, "scaleY", time, scale);
    Transform {
        x: keyframe_value(clip, "x", time, clip.x),
        y: keyframe_value(clip, "y", time, clip.y),
        rotate: keyframe_value(clip, "rotate", time, clip.rotate),
        scale_x,
        scale_y,
        opacity: keyframe_value(clip, "opacity", time, clip.opacity),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::timeline::make_clip;
    use crate::model::ClipKind;

    #[test]
    fn test_interpolation_and_keyframes() {
        let mut clip = make_clip(ClipKind::Video, "clip1", 0.0, 10.0);
        assert!(!clip_has_keyframes(&clip));

        set_keyframe(&mut clip, "x", 0.0, 0.0);
        set_keyframe(&mut clip, "x", 10.0, 100.0);
        assert!(clip_has_keyframes(&clip));
        assert!(has_keyframes(&clip, "x"));
        assert!(!has_keyframes(&clip, "y"));

        assert_eq!(keyframe_value(&clip, "x", 0.0, -1.0), 0.0);
        assert_eq!(keyframe_value(&clip, "x", 5.0, -1.0), 50.0);
        assert_eq!(keyframe_value(&clip, "x", 10.0, -1.0), 100.0);
        assert_eq!(keyframe_value(&clip, "x", 15.0, -1.0), 100.0); // Clamped to last
        assert_eq!(keyframe_value(&clip, "x", -5.0, -1.0), 0.0); // Clamped to first

        let tr = transform_at(&clip, 5.0);
        assert_eq!(tr.x, 50.0);
        assert_eq!(tr.scale_x, 1.0);

        clear_keyframe(&mut clip, "x", 5.0); // Doesn't match any point
        assert_eq!(keyframe_value(&clip, "x", 5.0, -1.0), 50.0);

        clear_keyframe(&mut clip, "x", 0.0);
        assert_eq!(keyframe_value(&clip, "x", 0.0, -1.0), 100.0);

        clear_all_keyframes(&mut clip);
        assert!(!clip_has_keyframes(&clip));
    }
}
