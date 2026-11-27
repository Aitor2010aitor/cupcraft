//! Subtítulos «karaoke» al estilo TikTok: el párrafo se ve entero y la palabra
//! que se está diciendo cambia de color (y, opcionalmente, «salta»).
//!
//! Un clip de texto guarda dos cosas en su objeto `text`:
//! * `words`: `[{ "s": inicio, "e": fin }]` en segundos **relativos al clip**,
//!   una entrada por palabra del `content` (separadas por espacios).
//! * `karaoke`: `{ "color": "#ffe600", "mode": "color" | "pop" }`.
//!
//! En cada fotograma [`apply`] calcula la palabra activa y deja una orden
//! `_karaoke` que consume `raster::render_text` al pintar los glifos.

use serde_json::{json, Value};

/// Colores de resaltado que ofrece el panel.
pub const COLORS: &[(&str, &str)] = &[("#ffe600", "Amarillo"), ("#00e5d0", "Turquesa"), ("#ff4d6d", "Rosa"), ("#7CFF6B", "Verde"), ("#ff9f1c", "Naranja")];

/// Modos de resaltado: id y etiqueta.
pub const MODES: &[(&str, &str)] = &[("color", "Color"), ("pop", "Color + salto")];

/// Efectos que ofrece el menú de transcribir: id, etiqueta, ¿karaoke?, color, modo.
pub const PRESETS: &[(&str, &str, bool, &str, &str)] = &[
    ("none", "Sin efecto", false, "#ffe600", "color"),
    ("yellow", "Amarillo", true, "#ffe600", "color"),
    ("yellow-pop", "Amarillo + salto", true, "#ffe600", "pop"),
    ("cyan", "Turquesa", true, "#00e5d0", "color"),
    ("cyan-pop", "Turquesa + salto", true, "#00e5d0", "pop"),
    ("pink-pop", "Rosa + salto", true, "#ff4d6d", "pop"),
    ("green", "Verde", true, "#7CFF6B", "color"),
    ("orange-pop", "Naranja + salto", true, "#ff9f1c", "pop"),
];

/// Índice de la palabra activa en `local` (segundos del clip): la última que
/// ya empezó. Antes de la primera se resalta la primera.
pub fn active_index(starts: &[f64], local: f64) -> usize {
    starts.iter().filter(|&&s| s <= local).count().saturating_sub(1)
}

/// Añade `_karaoke` al texto si el clip tiene `karaoke` y `words`.
pub fn apply(val: &Value, local: f64) -> Value {
    let (Some(k), Some(words)) = (val.get("karaoke"), val.get("words").and_then(Value::as_array)) else {
        return val.clone();
    };
    let starts: Vec<f64> = words.iter().map(|w| w.get("s").and_then(Value::as_f64).unwrap_or(0.0)).collect();
    if starts.is_empty() {
        return val.clone();
    }
    let mut out = val.clone();
    if let Some(o) = out.as_object_mut() {
        o.insert(
            "_karaoke".into(),
            json!({
                "idx": active_index(&starts, local),
                "color": k.get("color").and_then(Value::as_str).unwrap_or("#ffe600"),
                "pop": k.get("mode").and_then(Value::as_str) == Some("pop"),
            }),
        );
    }
    out
}

/// Reparte `duration` entre las palabras de `text` en proporción a su longitud
/// (para subtítulos manuales, SRT y voz generada, que no traen tiempos por palabra).
pub fn even_words(text: &str, duration: f64) -> Vec<Value> {
    let words: Vec<&str> = text.split_whitespace().collect();
    let total: f64 = words.iter().map(|w| w.chars().count() as f64 + 1.0).sum::<f64>().max(1.0);
    let mut t = 0.0;
    words
        .iter()
        .map(|w| {
            let d = duration * (w.chars().count() as f64 + 1.0) / total;
            let v = json!({ "s": t, "e": t + d });
            t += d;
            v
        })
        .collect()
}

/// Parte `text` en líneas de hasta `per_line` palabras y las reparte sobre
/// `[start, start + duration]` según su longitud (voz generada sin tiempos reales).
pub fn chunk_text(text: &str, per_line: usize, start: f64, duration: f64) -> Vec<Value> {
    let words: Vec<&str> = text.split_whitespace().collect();
    let weight = |w: &str| w.chars().count() as f64 + 1.0;
    let total: f64 = words.iter().map(|w| weight(w)).sum::<f64>().max(1.0);
    let mut t = start.max(0.0);
    words
        .chunks(per_line.max(1))
        .map(|c| {
            let line = c.join(" ");
            let d = duration * c.iter().map(|w| weight(w)).sum::<f64>() / total;
            let v = json!({ "text": line, "start": t, "duration": d, "words": even_words(&line, d) });
            t += d;
            v
        })
        .collect()
}

/// Agrupa palabras sueltas (`text`, `start`, `end`) en líneas de hasta
/// `per_line` palabras con sus tiempos por palabra relativos a la línea.
/// También corta la línea tras un punto, una coma o un hueco largo.
pub fn group_words(items: &[Value], per_line: usize) -> Vec<Value> {
    let per_line = per_line.max(1);
    let mut out = Vec::new();
    let mut cur: Vec<(String, f64, f64)> = Vec::new();
    let mut flush = |cur: &mut Vec<(String, f64, f64)>| {
        if cur.is_empty() {
            return;
        }
        let start = cur[0].1;
        let end = cur[cur.len() - 1].2.max(start + 0.1);
        let words: Vec<Value> = cur.iter().map(|(_, s, e)| json!({ "s": s - start, "e": e - start })).collect();
        let text = cur.iter().map(|(t, _, _)| t.as_str()).collect::<Vec<_>>().join(" ");
        out.push(json!({ "text": text, "start": start, "end": end, "words": words }));
        cur.clear();
    };
    for it in items {
        let text = it.get("text").and_then(Value::as_str).unwrap_or("").split_whitespace().collect::<Vec<_>>().join(" ");
        let (Some(s), Some(e)) = (it.get("start").and_then(Value::as_f64), it.get("end").and_then(Value::as_f64)) else { continue };
        if text.is_empty() {
            continue;
        }
        if let Some(last) = cur.last() {
            if s - last.2 > 0.8 {
                flush(&mut cur);
            }
        }
        let stop = text.ends_with(['.', '!', '?', ',', ';', ':']);
        cur.push((text, s, e.max(s + 0.05)));
        if cur.len() >= per_line || stop {
            flush(&mut cur);
        }
    }
    flush(&mut cur);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_palabra_activa_avanza_con_el_tiempo() {
        let s = [0.0, 0.5, 1.0];
        assert_eq!(active_index(&s, -1.0), 0);
        assert_eq!(active_index(&s, 0.2), 0);
        assert_eq!(active_index(&s, 0.5), 1);
        assert_eq!(active_index(&s, 9.0), 2);
    }

    #[test]
    fn apply_solo_actua_con_karaoke_y_palabras() {
        let plain = json!({ "content": "hay una avispa" });
        assert!(apply(&plain, 0.3).get("_karaoke").is_none());
        let v = json!({
            "content": "hay una avispa",
            "karaoke": { "color": "#ffe600", "mode": "pop" },
            "words": even_words("hay una avispa", 3.0),
        });
        let k = apply(&v, 1.2);
        assert_eq!(k["_karaoke"]["idx"], 1);
        assert_eq!(k["_karaoke"]["pop"], true);
    }

    #[test]
    fn even_words_cubre_toda_la_duracion() {
        let w = even_words("uno dos tres", 3.0);
        assert_eq!(w.len(), 3);
        assert!((w[2]["e"].as_f64().unwrap() - 3.0).abs() < 1e-9);
    }

    #[test]
    fn group_words_corta_por_cantidad_y_puntuacion() {
        let mk = |t: &str, s: f64, e: f64| json!({ "text": t, "start": s, "end": e });
        let items = [mk("Hola", 0.0, 0.3), mk("a", 0.3, 0.4), mk("todos.", 0.4, 0.9), mk("Hoy", 1.0, 1.3), mk("vamos", 1.3, 1.6)];
        let g = group_words(&items, 4);
        assert_eq!(g.len(), 2);
        assert_eq!(g[0]["text"], "Hola a todos.");
        assert!((g[1]["words"][1]["s"].as_f64().unwrap() - 0.3).abs() < 1e-9);
    }
}
