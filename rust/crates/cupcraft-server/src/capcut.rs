//! Lectura de borradores nativos de CapCut y conversión al modelo de cupcraft.
//!
//! Es el equivalente Rust de `server/capcut.mjs`: detecta las carpetas de
//! borradores, resume los proyectos para el diálogo de importación y convierte
//! un borrador completo a un proyecto de cupcraft. No modifica nada de CapCut.

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use percent_encoding::percent_decode_str;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::UNIX_EPOCH;

pub type AllowedMedia = Arc<Mutex<HashSet<String>>>;

const MEDIA_EXTENSIONS: &[&str] = &[
    "mp4", "mov", "m4v", "mkv", "webm", "avi", "wmv", "flv", "mp3", "wav", "m4a", "aac", "ogg",
    "flac", "opus", "jpg", "jpeg", "png", "webp", "gif", "bmp", "heic", "avif",
];

fn mime_for(ext: &str) -> &'static str {
    match ext {
        "mp4" => "video/mp4",
        "mov" => "video/quicktime",
        "m4v" => "video/x-m4v",
        "mkv" => "video/x-matroska",
        "webm" => "video/webm",
        "avi" => "video/x-msvideo",
        "wmv" => "video/x-ms-wmv",
        "flv" => "video/x-flv",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "m4a" => "audio/mp4",
        "aac" => "audio/aac",
        "ogg" | "opus" => "audio/ogg",
        "flac" => "audio/flac",
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        "heic" => "image/heic",
        "avif" => "image/avif",
        _ => "application/octet-stream",
    }
}

pub fn mime(path: &Path) -> &'static str {
    mime_for(&extension(path))
}

fn extension(path: &Path) -> String {
    path.extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
}

fn is_media_extension(ext: &str) -> bool {
    MEDIA_EXTENSIONS.contains(&ext)
}

pub fn now_millis() -> f64 {
    UNIX_EPOCH
        .elapsed()
        .map(|d| d.as_millis() as f64)
        .unwrap_or(0.0)
}

pub fn encode_path(value: &str) -> String {
    URL_SAFE_NO_PAD.encode(value.as_bytes())
}

pub fn decode_path(value: &str) -> Option<String> {
    URL_SAFE_NO_PAD
        .decode(value.as_bytes())
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok())
}

/// Clave normalizada para comparar rutas en Windows (sin distinguir mayúsculas).
pub fn media_key(path: &Path) -> String {
    let absolute = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    absolute
        .to_string_lossy()
        .replace('/', "\\")
        .to_ascii_lowercase()
}

pub fn allow_media(allowed: &AllowedMedia, path: &Path) {
    if let Ok(mut set) = allowed.lock() {
        set.insert(media_key(path));
    }
}

pub fn is_allowed_media(allowed: &AllowedMedia, path: &Path) -> bool {
    allowed
        .lock()
        .map(|set| set.contains(&media_key(path)))
        .unwrap_or(false)
}

/// Raíces candidatas de borradores de CapCut / JianyingPro.
pub fn roots() -> Vec<String> {
    let local = std::env::var("LOCALAPPDATA").unwrap_or_default();
    let roaming = std::env::var("APPDATA").unwrap_or_default();
    let home = std::env::var("USERPROFILE").unwrap_or_default();

    fn push_root(candidates: &mut Vec<PathBuf>, base: PathBuf) {
        let mut path = base;
        for part in ["User Data", "Projects", "com.lveditor.draft"] {
            path = path.join(part);
        }
        candidates.push(path);
    }

    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(custom) = std::env::var("openCUP_CAPCUT_ROOT") {
        if !custom.trim().is_empty() {
            candidates.push(PathBuf::from(custom));
        }
    }
    if !local.is_empty() {
        push_root(&mut candidates, Path::new(&local).join("CapCut"));
        push_root(&mut candidates, Path::new(&local).join("JianyingPro"));
    }
    if !roaming.is_empty() {
        push_root(&mut candidates, Path::new(&roaming).join("CapCut"));
    }
    if !home.is_empty() {
        push_root(
            &mut candidates,
            Path::new(&home).join("Documents").join("CapCut"),
        );
        push_root(
            &mut candidates,
            Path::new(&home).join("Documents").join("JianyingPro"),
        );
    }

    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for path in candidates {
        let absolute = std::path::absolute(&path).unwrap_or(path);
        let key = absolute.to_string_lossy().to_ascii_lowercase();
        if seen.insert(key) {
            out.push(absolute.to_string_lossy().to_string());
        }
    }
    out
}

fn exists(path: &Path) -> bool {
    path.exists()
}

fn get_num(object: &Value, key: &str, fallback: f64) -> f64 {
    match object.get(key) {
        Some(value) => value_number(value, fallback),
        None => fallback,
    }
}

fn value_number(value: &Value, fallback: f64) -> f64 {
    match value {
        Value::Number(n) => n.as_f64().unwrap_or(fallback),
        Value::String(s) => s.trim().parse::<f64>().unwrap_or(fallback),
        Value::Bool(b) => {
            if *b {
                1.0
            } else {
                0.0
            }
        }
        Value::Null => 0.0,
        _ => fallback,
    }
}

fn finite(value: &Value) -> Option<f64> {
    match value {
        Value::Number(n) => n.as_f64().filter(|n| n.is_finite()),
        Value::String(s) => s.trim().parse::<f64>().ok().filter(|n| n.is_finite()),
        Value::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
        _ => None,
    }
}

fn get_str<'a>(object: &'a Value, key: &str) -> &'a str {
    object.get(key).and_then(|v| v.as_str()).unwrap_or("")
}

fn get_bool(object: &Value, key: &str) -> bool {
    match object.get(key) {
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0) != 0.0,
        Some(Value::String(s)) => !s.is_empty() && s != "false" && s != "0",
        _ => false,
    }
}

fn seconds_from_micros(value: f64) -> f64 {
    (value / 1_000_000.0).max(0.0)
}

fn ratio_for(width: f64, height: f64) -> &'static str {
    if width <= 0.0 || height <= 0.0 {
        return "16:9";
    }
    let value = width / height;
    let options: [(&str, f64); 6] = [
        ("16:9", 16.0 / 9.0),
        ("9:16", 9.0 / 16.0),
        ("1:1", 1.0),
        ("4:3", 4.0 / 3.0),
        ("21:9", 21.0 / 9.0),
        ("4:5", 4.0 / 5.0),
    ];
    let mut best = options[0];
    for option in options {
        if (option.1 - value).abs() < (best.1 - value).abs() {
            best = option;
        }
    }
    best.0
}

fn is_absolute_windows(value: &str) -> bool {
    let bytes = value.as_bytes();
    (bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/'))
        || value.starts_with("\\\\")
}

fn lexical_normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => out.push(prefix.as_os_str()),
            Component::RootDir => out.push(Component::RootDir.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            Component::Normal(part) => out.push(part),
        }
    }
    out
}

fn normalize_path(raw: &str, draft_dir: &Path) -> Option<PathBuf> {
    let mut value = raw.replace('\\', "/").trim().to_string();
    if value.is_empty() {
        return None;
    }
    if value.to_ascii_lowercase().starts_with("file:") {
        let mut rest = &value[5..];
        if rest.starts_with("//") {
            rest = &rest[2..];
        }
        let decoded = percent_decode_str(rest).decode_utf8_lossy().to_string();
        value = if decoded.len() >= 3
            && decoded.as_bytes()[0] == b'/'
            && decoded.as_bytes()[1].is_ascii_alphabetic()
            && decoded.as_bytes()[2] == b':'
        {
            decoded[1..].to_string()
        } else {
            decoded
        };
    }
    if value.is_empty() {
        return None;
    }
    if is_absolute_windows(&value) {
        Some(lexical_normalize(&PathBuf::from(value.replace('/', "\\"))))
    } else {
        Some(lexical_normalize(&draft_dir.join(value.replace('/', "\\"))))
    }
}

fn safe_media_name(name: &str, path: &Path) -> String {
    let value = name.trim();
    if !value.is_empty() && !value.contains('\\') && !value.contains('/') {
        return value.to_string();
    }
    path.file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("archivo")
        .to_string()
}

fn read_json(path: &Path) -> Option<Value> {
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

fn parse_text_color(value: &Value, fallback: &str) -> String {
    if let Some(array) = value.as_array() {
        if array.len() >= 3 {
            let has_over_one = array.iter().any(|v| value_number(v, 0.0) > 1.0);
            let scale = if has_over_one { 1.0 } else { 255.0 };
            let mut parts = Vec::new();
            for entry in array.iter().take(3) {
                let channel = (value_number(entry, 0.0) * scale).round().clamp(0.0, 255.0) as u8;
                parts.push(format!("{:02x}", channel));
            }
            return format!("#{}", parts.join(""));
        }
    }
    if let Some(text) = value.as_str() {
        if text.len() == 7
            && text.starts_with('#')
            && text[1..].chars().all(|c| c.is_ascii_hexdigit())
        {
            return text.to_string();
        }
    }
    fallback.to_string()
}

fn parse_text_material(material: &Value) -> Value {
    let raw_content = material
        .get("content")
        .cloned()
        .unwrap_or_else(|| json!(""));
    let parsed = match raw_content.as_str() {
        Some(text) => {
            serde_json::from_str::<Value>(text).unwrap_or_else(|_| json!({ "text": text }))
        }
        None => json!({ "text": "" }),
    };
    let styles = parsed
        .get("styles")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let first = styles.first().cloned().unwrap_or_else(|| json!({}));

    let fill = first
        .get("fill")
        .and_then(|v| v.get("content"))
        .and_then(|v| v.get("solid"))
        .and_then(|v| v.get("color"))
        .cloned()
        .unwrap_or(Value::Null);
    let fallback_color = parse_text_color(
        material.get("text_color").unwrap_or(&Value::Null),
        "#ffffff",
    );
    let color = parse_text_color(&fill, &fallback_color);

    let base_size = match first.get("size") {
        Some(v) => value_number(v, get_num(material, "font_size", 15.0)),
        None => get_num(material, "font_size", 15.0),
    };
    let size = (base_size * 6.0).clamp(24.0, 240.0);
    let bold = get_num(material, "bold_width", 0.0) > 0.0 || get_num(&first, "bold", 0.0) > 0.0;
    let italic = get_num(material, "italic_degree", 0.0) > 0.0;
    let weight = if bold { 700.0 } else { 500.0 };
    let content = parsed
        .get("text")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| raw_content.as_str().unwrap_or("").to_string())
        .replace("\\n", "\n");

    let mut letter_spacing = get_num(material, "letter_spacing", 0.0);
    if letter_spacing.abs() > 1.0 {
        letter_spacing /= 100.0;
    }
    let letter_spacing = letter_spacing.clamp(-0.2, 0.6);

    let mut raw_line_spacing = get_num(material, "line_spacing", 0.0);
    if raw_line_spacing.abs() > 1.0 {
        raw_line_spacing /= 100.0;
    }
    let line_spacing = (1.0 + raw_line_spacing).clamp(0.7, 3.0);

    let mut curve = get_num(material, "text_curve", 0.0);
    if curve.abs() <= 2.0 {
        curve *= 100.0;
    }
    let curve = curve.clamp(-100.0, 100.0);

    let border_color = parse_text_color(material.get("border_color").unwrap_or(&Value::Null), "");
    let border_width = if border_color.is_empty() {
        0.0
    } else {
        get_num(material, "border_width", 0.0).clamp(0.0, 0.3)
    };
    let background_style = get_num(material, "background_style", 0.0);
    let background_color = parse_text_color(
        material.get("background_color").unwrap_or(&Value::Null),
        "#000000",
    );
    let has_background = get_bool(material, "background_fill")
        || background_style > 0.0
        || (!background_color.is_empty() && background_color.to_lowercase() != "#000000");
    let background_width = get_num(material, "background_width", 0.22);
    let background_height = get_num(material, "background_height", 0.22);

    let alignment = get_num(material, "alignment", 1.0);
    let align = if alignment == 0.0 {
        "left"
    } else if alignment == 2.0 {
        "right"
    } else {
        "center"
    };
    let border_mode = get_num(material, "border_mode", 0.0);
    let stroke_mode = if border_mode == 2.0 {
        "inside"
    } else if border_mode == 1.0 {
        "center"
    } else {
        "outer"
    };
    let background_style_name = if background_style == 2.0 {
        "pill"
    } else if background_style == 1.0 {
        "rounded"
    } else {
        "square"
    };

    json!({
        "content": content,
        "size": size,
        "weight": weight,
        "bold": bold,
        "italic": italic,
        "underline": get_bool(material, "underline"),
        "color": color,
        "align": align,
        "font": "Inter",
        "x": 0.0,
        "y": 0.0,
        "letterSpacing": letter_spacing,
        "lineSpacing": line_spacing,
        "textCase": "none",
        "opacity": (get_num(material, "global_alpha", 1.0) * get_num(material, "text_alpha", 1.0)).clamp(0.0, 1.0),
        "shadow": get_bool(material, "has_shadow"),
        "shadowColor": parse_text_color(material.get("shadow_color").unwrap_or(&Value::Null), "#000000"),
        "shadowAlpha": get_num(material, "shadow_alpha", 0.9).clamp(0.0, 1.0),
        "shadowBlur": get_num(material, "shadow_smoothing", 0.45).clamp(0.0, 1.0),
        "shadowDistance": (get_num(material, "shadow_distance", 5.0) / 100.0).clamp(0.0, 0.4),
        "shadowAngle": get_num(material, "shadow_angle", 90.0),
        "strokeColor": if border_color.is_empty() { "#000000".to_string() } else { border_color },
        "strokeWidth": border_width,
        "strokeAlpha": get_num(material, "border_alpha", 1.0).clamp(0.0, 1.0),
        "strokeMode": stroke_mode,
        "background": if has_background { background_color } else { "transparent".to_string() },
        "backgroundAlpha": get_num(material, "background_alpha", 1.0).clamp(0.0, 1.0),
        "backgroundStyle": background_style_name,
        "backgroundRadius": get_num(material, "background_round_radius", 0.0).clamp(0.0, 0.5),
        "backgroundPadding": background_width.max(background_height).clamp(0.0, 0.6),
        "backgroundBorderColor": "#000000",
        "backgroundBorderWidth": 0.0,
        "glow": false,
        "glowColor": "#00e5d0",
        "glowBlur": 0.35,
        "curve": curve,
        "flipX": get_bool(material, "flip_x") || get_bool(material, "flipX"),
        "flipY": get_bool(material, "flip_y") || get_bool(material, "flipY"),
        "animation": "none",
        "animationPhase": "in",
        "animationDuration": 0.6,
    })
}

#[derive(Clone)]
struct MaterialEntry {
    item: Value,
    kind: String,
    bucket: String,
}

fn collect_materials(content: &Value) -> HashMap<String, MaterialEntry> {
    let mut map = HashMap::new();
    let Some(buckets) = content.get("materials").and_then(|v| v.as_object()) else {
        return map;
    };
    let pairs = [
        ("videos", "video"),
        ("images", "image"),
        ("audios", "audio"),
        ("stickers", "sticker"),
        ("texts", "text"),
    ];
    for (bucket, kind) in pairs {
        if let Some(list) = buckets.get(bucket).and_then(|v| v.as_array()) {
            for item in list {
                if let Some(id) = item.get("id").and_then(|v| v.as_str()) {
                    map.insert(
                        id.to_string(),
                        MaterialEntry {
                            item: item.clone(),
                            kind: kind.to_string(),
                            bucket: bucket.to_string(),
                        },
                    );
                }
            }
        }
    }
    map
}

/// Los borradores de CapCut no siempre guardan el archivo en `path`: los stickers
/// animados pueden traerlo en `resource_path`, `file_path` o dentro de `extra_info`.
fn material_path(material: &Value) -> String {
    const KEYS: &[&str] = &[
        "path",
        "resource_path",
        "file_path",
        "local_path",
        "image_path",
        "cover_path",
    ];
    for key in KEYS {
        let value = get_str(material, key);
        if !value.trim().is_empty() {
            return value.to_string();
        }
    }
    let Some(extra) = material.get("extra_info") else {
        return String::new();
    };
    let parsed = match extra {
        Value::String(text) => serde_json::from_str::<Value>(text).ok(),
        Value::Object(_) => Some(extra.clone()),
        _ => None,
    };
    if let Some(extra) = parsed {
        for key in KEYS {
            if let Some(value) = extra.get(*key).and_then(|v| v.as_str()) {
                if !value.trim().is_empty() {
                    return value.to_string();
                }
            }
        }
    }
    String::new()
}

fn material_to_media(
    material: &Value,
    kind: &str,
    draft_dir: &Path,
    warnings: &mut Vec<String>,
) -> Option<Value> {
    let raw_path = material_path(material);
    if raw_path.trim().is_empty() {
        if kind == "sticker" {
            let label = {
                let name = get_str(material, "material_name");
                let name = if name.is_empty() {
                    get_str(material, "name")
                } else {
                    name
                };
                if name.is_empty() {
                    get_str(material, "id").to_string()
                } else {
                    name.to_string()
                }
            };
            warnings.push(format!(
                "Sticker sin archivo local: {}",
                if label.is_empty() {
                    "sin nombre".to_string()
                } else {
                    label
                }
            ));
        }
        return None;
    }
    let path = normalize_path(&raw_path, draft_dir)?;
    let ext = extension(&path);
    if !is_media_extension(&ext) {
        warnings.push(format!(
            "Formato no soportado: {}",
            path.file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("archivo")
        ));
        return None;
    }
    let media_kind = if kind == "sticker" { "image" } else { kind };
    let path_text = path.to_string_lossy().to_string();
    let raw_name = {
        let material_name = get_str(material, "material_name");
        if material_name.is_empty() {
            get_str(material, "name")
        } else {
            material_name
        }
    };
    Some(json!({
        "capcutId": get_str(material, "id"),
        "name": safe_media_name(raw_name, &path),
        "kind": media_kind,
        "path": path_text,
        "fileUrl": format!("/api/capcut/media?path={}", encode_path(&path_text)),
        "duration": get_num(material, "duration", 0.0) / 1_000_000.0,
        "width": get_num(material, "width", 0.0),
        "height": get_num(material, "height", 0.0),
        "size": get_num(material, "size", 0.0),
        "mime": mime_for(&ext),
        "missing": false,
    }))
}

fn lookup_refs(
    segment: &Value,
    materials: &HashMap<String, MaterialEntry>,
    content: &Value,
) -> Vec<(Value, String)> {
    let mut refs = Vec::new();
    let Some(ids) = segment
        .get("extra_material_refs")
        .and_then(|v| v.as_array())
    else {
        return refs;
    };
    for id_value in ids {
        let Some(id) = id_value.as_str() else {
            continue;
        };
        if let Some(entry) = materials.get(id) {
            refs.push((entry.item.clone(), entry.bucket.clone()));
            continue;
        }
        'search: for list in content
            .get("materials")
            .and_then(|v| v.as_object())
            .map(|m| m.values())
            .into_iter()
            .flatten()
        {
            if let Some(items) = list.as_array() {
                for item in items {
                    if item.get("id").and_then(|v| v.as_str()) == Some(id) {
                        refs.push((item.clone(), String::new()));
                        break 'search;
                    }
                }
            }
        }
    }
    refs
}

fn ref_number(refs: &[(Value, String)], keys: &[&str], fallback: f64) -> f64 {
    for (item, _) in refs {
        for key in keys {
            if let Some(value) = item.get(*key) {
                if let Some(number) = finite(value) {
                    return number;
                }
            }
        }
    }
    fallback
}

fn normalize_imported_clip(
    segment: &Value,
    material: Option<&MaterialEntry>,
    refs: &[(Value, String)],
    draft_dir: &Path,
    warnings: &mut Vec<String>,
) -> Option<Value> {
    let target = segment
        .get("target_timerange")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let source = segment
        .get("source_timerange")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let start = seconds_from_micros(get_num(&target, "start", 0.0));
    let mut duration = seconds_from_micros(get_num(&target, "duration", 0.0));
    if duration <= 0.0 {
        duration = seconds_from_micros(get_num(&source, "duration", 0.0));
    }
    if duration <= 0.0 {
        return None;
    }

    let segment_id = get_str(segment, "id");
    let clip_id = if segment_id.is_empty() {
        format!("cc_{}_{}", start, duration)
    } else {
        format!("cc_{}", segment_id)
    };

    let speed = {
        let speed = get_num(segment, "speed", 1.0);
        if speed == 0.0 {
            1.0
        } else {
            speed
        }
    };
    let clip_data = segment.get("clip").unwrap_or(&Value::Null);
    let scale = {
        let scale = get_num(clip_data.get("scale").unwrap_or(&Value::Null), "x", 1.0);
        if scale == 0.0 {
            1.0
        } else {
            scale.max(0.05)
        }
    };
    let transform = clip_data.get("transform").unwrap_or(&Value::Null);

    let mut clip = json!({
        "id": clip_id,
        "kind": "video",
        "mediaId": Value::Null,
        "name": "",
        "start": start,
        "duration": duration,
        "in": seconds_from_micros(get_num(&source, "start", 0.0)),
        "speed": speed,
        "volume": get_num(segment, "volume", 1.0),
        "opacity": get_num(clip_data, "alpha", 1.0).clamp(0.0, 1.0),
        "scale": scale,
        "x": get_num(transform, "x", 0.0) * 50.0,
        "y": get_num(transform, "y", 0.0) * 50.0,
        "rotate": get_num(clip_data, "rotation", 0.0),
        "filter": "none",
        "adjust": { "brightness": 100.0, "contrast": 100.0, "saturate": 100.0 },
        "fadeIn": 0.0,
        "fadeOut": 0.0,
    });

    match material {
        Some(entry) if entry.kind == "text" => {
            let text = parse_text_material(&entry.item);
            let name: String = text
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("Texto")
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            let name = if name.is_empty() {
                "Texto".to_string()
            } else {
                name.chars().take(24).collect()
            };
            clip["kind"] = json!("text");
            clip["text"] = text;
            clip["name"] = json!(name);
        }
        Some(entry) => {
            let Some(media) = material_to_media(&entry.item, &entry.kind, draft_dir, warnings)
            else {
                return None;
            };
            let media_kind = media
                .get("kind")
                .and_then(|v| v.as_str())
                .unwrap_or("video");
            clip["kind"] = json!(media_kind);
            clip["mediaId"] = media.get("capcutId").cloned().unwrap_or(Value::Null);
            clip["name"] = media.get("name").cloned().unwrap_or_else(|| json!(""));
            if media_kind == "image" || media_kind == "sticker" {
                clip["in"] = json!(0.0);
            }
        }
        None => return None,
    }

    let fade_in = ref_number(
        refs,
        &["fade_in", "fadeIn", "audio_fade_in", "video_fade_in"],
        0.0,
    );
    let fade_out = ref_number(
        refs,
        &["fade_out", "fadeOut", "audio_fade_out", "video_fade_out"],
        0.0,
    );
    let duration_now = clip
        .get("duration")
        .and_then(|v| v.as_f64())
        .unwrap_or(duration);
    if fade_in > 0.0 {
        clip["fadeIn"] = json!(seconds_from_micros(fade_in).min(duration_now));
    }
    if fade_out > 0.0 {
        clip["fadeOut"] = json!(seconds_from_micros(fade_out).min(duration_now));
    }

    let brightness = ref_number(refs, &["brightness", "brightness_value"], -1.0);
    let contrast = ref_number(refs, &["contrast", "contrast_value"], -1.0);
    let saturation = ref_number(refs, &["saturate", "saturation", "saturation_value"], -1.0);
    if brightness >= 0.0 {
        clip["adjust"]["brightness"] = json!(if brightness > 2.0 {
            brightness
        } else {
            100.0 + brightness * 100.0
        });
    }
    if contrast >= 0.0 {
        clip["adjust"]["contrast"] = json!(if contrast > 2.0 {
            contrast
        } else {
            100.0 + contrast * 100.0
        });
    }
    if saturation >= 0.0 {
        clip["adjust"]["saturate"] = json!(if saturation > 2.0 {
            saturation
        } else {
            100.0 + saturation * 100.0
        });
    }

    for (item, bucket) in refs {
        let filter_name = get_str(item, "name").to_string()
            + get_str(item, "filter_name")
            + get_str(item, "resource_name");
        let is_filter = bucket == "filters"
            || get_str(item, "type") == "filter"
            || item.get("filter_id").is_some();
        if is_filter {
            let key = filter_name.to_lowercase();
            let table = [
                ("mono", "mono"),
                ("blanco y negro", "mono"),
                ("noir", "noir"),
                ("vintage", "vintage"),
                ("retro", "vintage"),
                ("vivid", "vivid"),
                ("calido", "warm"),
                ("cálido", "warm"),
                ("warm", "warm"),
                ("frio", "cool"),
                ("frío", "cool"),
                ("cool", "cool"),
                ("fade", "fade"),
                ("desvanecido", "fade"),
            ];
            if let Some((_, filter)) = table.iter().find(|(label, _)| key.contains(label)) {
                clip["filter"] = json!(filter);
            }
        }
    }

    Some(clip)
}

fn validate_draft_id(id: &str, roots: &[String]) -> Result<PathBuf, String> {
    let decoded = decode_path(id).ok_or_else(|| "Proyecto no válido".to_string())?;
    let resolved = lexical_normalize(&PathBuf::from(decoded.replace('/', "\\")));
    let resolved_key = resolved
        .to_string_lossy()
        .replace('/', "\\")
        .to_ascii_lowercase();
    let inside = roots.iter().any(|root| {
        let root = lexical_normalize(Path::new(root));
        let root_key = root
            .to_string_lossy()
            .replace('/', "\\")
            .to_ascii_lowercase();
        resolved_key == root_key || resolved_key.starts_with(&(root_key.clone() + "\\"))
    });
    if !inside {
        return Err("La carpeta no pertenece a CapCut".to_string());
    }
    let last = resolved
        .components()
        .filter_map(|c| match c {
            Component::Normal(part) => Some(part.to_string_lossy().to_string()),
            _ => None,
        })
        .last()
        .unwrap_or_default();
    if resolved.file_name().and_then(|s| s.to_str()).unwrap_or("") != last.as_str() {
        return Err("Ruta no válida".to_string());
    }
    Ok(resolved)
}

/// Resume los borradores encontrados para el diálogo de importación.
pub fn scan_projects(allowed: &AllowedMedia) -> Vec<Value> {
    let mut projects = Vec::new();
    for root in roots() {
        let root_path = PathBuf::from(&root);
        if !exists(&root_path) {
            continue;
        }
        let Ok(entries) = std::fs::read_dir(&root_path) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if !file_type.is_dir() {
                continue;
            }
            let dir = entry.path();
            let content_path = dir.join("draft_content.json");
            if !exists(&content_path) {
                continue;
            }
            let Some(content) = read_json(&content_path) else {
                continue;
            };
            let meta = read_json(&dir.join("draft_meta_info.json")).unwrap_or_else(|| json!({}));
            let folder = entry.file_name().to_string_lossy().to_string();
            let name = {
                let value = get_str(&meta, "draft_name");
                if value.is_empty() {
                    folder.clone()
                } else {
                    value.to_string()
                }
            };
            let width = get_num(
                content.get("canvas_config").unwrap_or(&Value::Null),
                "width",
                1920.0,
            );
            let height = get_num(
                content.get("canvas_config").unwrap_or(&Value::Null),
                "height",
                1080.0,
            );

            let mut cover = String::new();
            let mut cover_url = String::new();
            let mut candidates: Vec<PathBuf> = Vec::new();
            let draft_cover = get_str(&meta, "draft_cover");
            if !draft_cover.is_empty() {
                if let Some(path) = normalize_path(draft_cover, &dir) {
                    candidates.push(path);
                }
            }
            candidates.push(dir.join("draft_cover.jpg"));
            candidates.push(dir.join("draft_cover.png"));
            candidates.push(dir.join("Resources").join("cover.jpg"));
            for candidate in candidates {
                if exists(&candidate) {
                    cover = candidate.to_string_lossy().to_string();
                    cover_url = format!("/api/capcut/cover?path={}", encode_path(&cover));
                    allow_media(allowed, &candidate);
                    break;
                }
            }

            let counts = json!({
                "video": content.get("materials").and_then(|m| m.get("videos")).and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0),
                "image": content.get("materials").and_then(|m| m.get("images")).and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0),
                "audio": content.get("materials").and_then(|m| m.get("audios")).and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0),
                "text": content.get("materials").and_then(|m| m.get("texts")).and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0),
            });
            let tracks = content
                .get("tracks")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            let clips: usize = tracks
                .iter()
                .map(|track| {
                    track
                        .get("segments")
                        .and_then(|v| v.as_array())
                        .map(|a| a.len())
                        .unwrap_or(0)
                })
                .sum();
            let modified = std::fs::metadata(&content_path)
                .and_then(|m| m.modified())
                .ok()
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as f64)
                .unwrap_or(0.0);

            projects.push(json!({
                "id": encode_path(&dir.to_string_lossy()),
                "folder": folder,
                "name": name,
                "duration": get_num(&content, "duration", 0.0) / 1_000_000.0,
                "width": width,
                "height": height,
                "ratio": ratio_for(width, height),
                "fps": get_num(&content, "fps", 30.0),
                "modified": modified,
                "tracks": tracks.len(),
                "clips": clips,
                "counts": counts,
                "cover": cover,
                "coverUrl": cover_url,
                "root": root,
            }));
        }
    }
    projects.sort_by(|a, b| {
        let ma = a.get("modified").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let mb = b.get("modified").and_then(|v| v.as_f64()).unwrap_or(0.0);
        mb.partial_cmp(&ma).unwrap_or(std::cmp::Ordering::Equal)
    });
    projects
}

/// Convierte un borrador completo de CapCut en un proyecto de cupcraft.
pub fn read_draft(id: &str, allowed: &AllowedMedia) -> Result<Value, String> {
    let roots = roots();
    let dir = validate_draft_id(id, &roots)?;
    let content_path = dir.join("draft_content.json");
    if !exists(&content_path) {
        return Err("El borrador no contiene draft_content.json".to_string());
    }
    let content =
        read_json(&content_path).ok_or_else(|| "No se pudo leer draft_content.json".to_string())?;
    let meta = read_json(&dir.join("draft_meta_info.json")).unwrap_or_else(|| json!({}));

    let mut warnings: Vec<String> = Vec::new();
    let width = get_num(
        content.get("canvas_config").unwrap_or(&Value::Null),
        "width",
        1920.0,
    );
    let height = get_num(
        content.get("canvas_config").unwrap_or(&Value::Null),
        "height",
        1080.0,
    );
    let materials = collect_materials(&content);

    let mut media: Vec<Value> = Vec::new();
    let mut media_index: HashMap<String, usize> = HashMap::new();
    for entry in materials.values() {
        if !["video", "image", "audio", "sticker"].contains(&entry.kind.as_str()) {
            continue;
        }
        let Some(imported) = material_to_media(&entry.item, &entry.kind, &dir, &mut warnings)
        else {
            continue;
        };
        if let Some(path) = imported.get("path").and_then(|v| v.as_str()) {
            allow_media(allowed, Path::new(path));
        }
        if let Some(capcut_id) = imported.get("capcutId").and_then(|v| v.as_str()) {
            media_index.insert(capcut_id.to_string(), media.len());
        }
        media.push(imported);
    }

    let mut used_track_names: HashMap<String, usize> = HashMap::new();
    let mut tracks: Vec<Value> = Vec::new();
    let empty_tracks = Vec::new();
    let source_tracks = content
        .get("tracks")
        .and_then(|v| v.as_array())
        .unwrap_or(&empty_tracks);
    for track in source_tracks {
        let segments = track
            .get("segments")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        if segments.is_empty() {
            continue;
        }
        let mut clips = Vec::new();
        for segment in &segments {
            let material = segment
                .get("material_id")
                .and_then(|v| v.as_str())
                .and_then(|id| materials.get(id));
            let refs = lookup_refs(segment, &materials, &content);
            let Some(mut clip) =
                normalize_imported_clip(segment, material, &refs, &dir, &mut warnings)
            else {
                continue;
            };
            if let Some(media_id) = clip.get("mediaId").and_then(|v| v.as_str()) {
                let media_id = media_id.to_string();
                let Some(&index) = media_index.get(&media_id) else {
                    continue;
                };
                let path = media[index]
                    .get("path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                if !exists(Path::new(&path)) {
                    media[index]["missing"] = json!(true);
                    let name = media[index]
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("archivo")
                        .to_string();
                    warnings.push(format!("Archivo no encontrado: {}", name));
                }
            }
            if let Some(text) = clip.get_mut("text") {
                if let Some(color) = text.get("color").and_then(|v| v.as_str()) {
                    if !color.starts_with('#') {
                        text["color"] = json!("#ffffff");
                    }
                }
            }
            clips.push(clip);
        }
        if clips.is_empty() {
            continue;
        }
        let all_text = clips
            .iter()
            .all(|c| c.get("kind").and_then(|v| v.as_str()) == Some("text"));
        let all_audio = clips
            .iter()
            .all(|c| c.get("kind").and_then(|v| v.as_str()) == Some("audio"));
        let kind = if all_text {
            "text"
        } else if all_audio {
            "audio"
        } else {
            "video"
        };
        let base = match kind {
            "text" => "Texto",
            "audio" => "Audio",
            _ => "Video",
        };
        let index = used_track_names.get(kind).copied().unwrap_or(0) + 1;
        used_track_names.insert(kind.to_string(), index);
        let track_id = {
            let id = get_str(track, "id");
            if id.is_empty() {
                format!("cap_track_{}", tracks.len() + 1)
            } else {
                format!("cap_track_{}", id)
            }
        };
        tracks.push(json!({
            "id": track_id,
            "type": kind,
            "name": format!("{} {}", base, index),
            "clips": clips,
        }));
    }

    let has = |kind: &str| {
        tracks
            .iter()
            .any(|track| track.get("type").and_then(|v| v.as_str()) == Some(kind))
    };
    let has_text = has("text");
    let has_video = has("video");
    let has_audio = has("audio");
    if !has_text {
        tracks.insert(
            0,
            json!({ "id": "cap_track_text", "type": "text", "name": "Texto 1", "clips": [] }),
        );
    }
    if !has_video {
        tracks.push(
            json!({ "id": "cap_track_video", "type": "video", "name": "Video 1", "clips": [] }),
        );
    }
    if !has_audio {
        tracks.push(
            json!({ "id": "cap_track_audio", "type": "audio", "name": "Audio 1", "clips": [] }),
        );
    }

    let mut seen = HashSet::new();
    let warnings: Vec<String> = warnings
        .into_iter()
        .filter(|w| seen.insert(w.clone()))
        .take(40)
        .collect();
    let now = now_millis();
    let created = get_num(&meta, "tm_draft_create", 0.0) / 1000.0;
    let updated = get_num(&meta, "tm_draft_modified", 0.0) / 1000.0;
    let app_version = {
        let from_content = content
            .get("platform")
            .and_then(|p| p.get("app_version"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if !from_content.is_empty() {
            from_content.to_string()
        } else {
            get_str(&meta, "draft_new_version").to_string()
        }
    };
    let name = {
        let value = get_str(&meta, "draft_name");
        if value.is_empty() {
            dir.file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("Proyecto CapCut")
                .to_string()
        } else {
            value.to_string()
        }
    };

    Ok(json!({
        "name": name,
        "width": width,
        "height": height,
        "ratio": ratio_for(width, height),
        "fps": get_num(&content, "fps", 30.0),
        "duration": get_num(&content, "duration", 0.0) / 1_000_000.0,
        "createdAt": if created > 0.0 { created } else { now },
        "updatedAt": if updated > 0.0 { updated } else { now },
        "tracks": tracks,
        "media": media,
        "warnings": warnings,
        "source": {
            "type": "capcut",
            "folder": dir.to_string_lossy(),
            "appVersion": app_version,
        },
    }))
}
