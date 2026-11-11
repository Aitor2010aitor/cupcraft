//! Formateo de tiempo y tamaño, con la misma semántica que `src/utils.js`
//! y `formatTime`/`timeAgo` de la app web.

use cupcraft_core::now_millis;

/// `mm:ss` (o `hh:mm:ss` si supera la hora). Réplica de `formatTime`.
pub fn format_time(seconds: f64) -> String {
    let safe = if seconds.is_finite() { seconds.max(0.0) } else { 0.0 };
    let total = safe.floor() as i64;
    let h = total / 3600;
    let m = (total % 3600) / 60;
    let s = total % 60;
    if h > 0 {
        format!("{h:02}:{m:02}:{s:02}")
    } else {
        format!("{m:02}:{s:02}")
    }
}

/// `hh:mm:ss:ff` con fotogramas, como el timecode del reproductor.
pub fn format_timecode(seconds: f64, fps: u32) -> String {
    let safe = if seconds.is_finite() { seconds.max(0.0) } else { 0.0 };
    let total = safe.floor() as i64;
    let h = total / 3600;
    let m = (total % 3600) / 60;
    let s = total % 60;
    let f = ((safe - total as f64) * fps.max(1) as f64).floor() as i64;
    format!("{h:02}:{m:02}:{s:02}:{f:02}")
}

/// Texto relativo de la portada: "hace un momento", "hace 5 min"…
pub fn time_ago(updated_at_ms: i64) -> String {
    let diff = ((now_millis() - updated_at_ms).max(0)) / 1000;
    if diff < 60 {
        "hace un momento".to_string()
    } else if diff < 3600 {
        format!("hace {} min", diff / 60)
    } else if diff < 86_400 {
        format!("hace {} h", diff / 36_000)
    } else if diff < 604_800 {
        format!("hace {} d", diff / 86_400)
    } else {
        let (y, m, d) = civil_from_days(diff / 86_400);
        let _ = y;
        format!("{d} {}", MONTHS[(m - 1) as usize])
    }
}

const MONTHS: [&str; 12] = [
    "ene", "feb", "mar", "abr", "may", "jun", "jul", "ago", "sep", "oct", "nov", "dic",
];

/// Días desde la epoch a fecha civil (algoritmo de Howard Hinnant).
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// Tamaño legible en binario ("1.2 MB"), como `formatBytes`.
pub fn format_bytes(bytes: u64) -> String {
    if bytes == 0 {
        return "0 B".to_string();
    }
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let i = ((bytes as f64).log(1024.0).floor() as usize).min(UNITS.len() - 1);
    let value = bytes as f64 / 1024f64.powi(i as i32);
    if i == 0 {
        format!("{value:.0} {}", UNITS[i])
    } else {
        format!("{value:.1} {}", UNITS[i])
    }
}
