//! Importación y caché de medios (nativo, sin navegador).

use cupcraft_core::Media;
use cupcraft_persist::{MediaRecord, Store};
use serde_json::json;
use std::path::{Path, PathBuf};

fn app_dir() -> PathBuf {
    crate::app::app_dir()
}

/// Carpeta de caché de medios del usuario (no requiere administrador).
pub fn cache_dir() -> PathBuf {
    let dir = app_dir().join("media");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// Carpeta de trabajo temporal (exportación, audio intermedio…).
pub fn work_dir() -> PathBuf {
    let dir = app_dir().join("work");
    let _ = std::fs::create_dir_all(&dir);
    dir
}

pub fn ext_of(media: &Media) -> String {
    if let Some(e) = media.extra.get("ext").and_then(|v| v.as_str()) {
        return e.to_string();
    }
    if let Some(m) = media.extra.get("mime").and_then(|v| v.as_str()) {
        if let Some(e) = ext_from_mime(m) {
            return e.to_string();
        }
    }
    media
        .name
        .as_deref()
        .and_then(|n| Path::new(n).extension())
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_else(|| match media.kind.as_deref() {
            Some("audio") => "mp3".into(),
            Some("video") => "mp4".into(),
            _ => "png".into(),
        })
}

fn ext_from_mime(m: &str) -> Option<&'static str> {
    Some(match m {
        "audio/mpeg" => "mp3",
        "audio/wav" | "audio/x-wav" => "wav",
        "audio/ogg" => "ogg",
        "audio/mp4" => "m4a",
        "video/mp4" => "mp4",
        "video/webm" => "webm",
        "video/quicktime" => "mov",
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/webp" => "webp",
        "image/gif" => "gif",
        _ => return None,
    })
}

/// Devuelve la ruta en disco del medio, escribiéndola desde el almacén si
/// todavía no existe en la caché.
pub fn ensure_file(store: Option<&Store>, media: &Media) -> Option<PathBuf> {
    let path = cache_dir().join(format!("{}.{}", media.id, ext_of(media)));
    if path.exists() {
        return Some(path);
    }
    let rec = store?.get_media(&media.id).ok()??;
    std::fs::write(&path, &rec.bytes).ok()?;
    Some(path)
}

pub fn kind_and_mime(ext: &str) -> (&'static str, &'static str) {
    match ext {
        "mp3" => ("audio", "audio/mpeg"),
        "wav" => ("audio", "audio/wav"),
        "ogg" | "oga" | "opus" => ("audio", "audio/ogg"),
        "m4a" | "aac" => ("audio", "audio/mp4"),
        "flac" => ("audio", "audio/flac"),
        "mp4" | "m4v" => ("video", "video/mp4"),
        "mov" => ("video", "video/quicktime"),
        "mkv" => ("video", "video/x-matroska"),
        "webm" => ("video", "video/webm"),
        "avi" => ("video", "video/x-msvideo"),
        "png" => ("image", "image/png"),
        "jpg" | "jpeg" => ("image", "image/jpeg"),
        "webp" => ("image", "image/webp"),
        "gif" => ("image", "image/gif"),
        "bmp" => ("image", "image/bmp"),
        _ => ("", ""),
    }
}

pub const IMPORT_EXTS: &[&str] = &[
    "mp4", "m4v", "mov", "mkv", "webm", "avi", "mp3", "wav", "ogg", "oga", "opus", "m4a", "aac",
    "flac", "png", "jpg", "jpeg", "webp", "gif", "bmp",
];

/// Importa un archivo local: lo guarda en el almacén, lo copia a la caché y
/// rellena duración/tamaño con ffprobe (o con el decodificador de imágenes).
pub fn import_file(store: Option<&mut Store>, path: &Path) -> Result<Media, String> {
    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
    let (kind, _) = kind_and_mime(&ext);
    if kind.is_empty() {
        return Err(format!("Formato no compatible: .{ext}"));
    }
    let bytes = std::fs::read(path).map_err(|e| format!("No se pudo leer el archivo: {e}"))?;
    let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("media").to_string();
    import_bytes(store, &name, &ext, bytes)
}

/// Igual que `import_file` pero a partir de bytes ya en memoria (voz TTS…).
pub fn import_bytes(store: Option<&mut Store>, name: &str, ext: &str, bytes: Vec<u8>) -> Result<Media, String> {
    let (kind, mime) = kind_and_mime(ext);
    let id = cupcraft_core::new_id("med");
    let cached = cache_dir().join(format!("{id}.{ext}"));
    std::fs::write(&cached, &bytes).map_err(|e| format!("No se pudo copiar a la caché: {e}"))?;

    let mut extra = serde_json::Map::new();
    extra.insert("ext".into(), json!(ext));
    extra.insert("mime".into(), json!(mime));
    extra.insert("size".into(), json!(bytes.len()));
    let mut duration = None;
    if kind == "image" {
        if let Ok((w, h)) = image::image_dimensions(&cached) {
            extra.insert("width".into(), json!(w));
            extra.insert("height".into(), json!(h));
        }
    } else {
        match cupcraft_media::probe_media(&cached) {
            Ok(info) => {
                if info.duration > 0.0 {
                    duration = Some(info.duration);
                }
                if info.width > 0 {
                    extra.insert("width".into(), json!(info.width));
                    extra.insert("height".into(), json!(info.height));
                }
                if info.fps > 0.0 {
                    extra.insert("fps".into(), json!(info.fps));
                }
                extra.insert("hasAudio".into(), json!(info.has_audio));
                extra.insert("hasVideo".into(), json!(info.has_video));
            }
            Err(_) => {
                // Sin FFmpeg no se puede conocer la duración de vídeo; para
                // audio se intenta con el decodificador nativo.
                let fallback = if kind == "audio" {
                    cupcraft_audio::decode_audio_file(&cached).ok().map(|b| b.duration())
                } else {
                    None
                };
                duration = Some(fallback.unwrap_or(5.0));
                extra.insert("probeFailed".into(), json!(true));
            }
        }
    }
    let media = Media {
        id,
        kind: Some(kind.to_string()),
        name: Some(name.to_string()),
        duration,
        thumb: None,
        extra,
    };
    if let Some(store) = store {
        store
            .save_media(&MediaRecord { meta: media.clone(), mime: mime.to_string(), bytes })
            .map_err(|e| format!("No se pudo guardar en el almacén: {e}"))?;
    }
    Ok(media)
}

/// Decodifica la pista de audio de un medio (audio o vídeo) a un búfer PCM.
/// Se transcodifica con FFmpeg a WAV (48 kHz estéreo) para admitir cualquier
/// contenedor; si FFmpeg no está, se intenta directamente con Symphonia.
pub fn decode_audio(path: &Path, media_id: &str) -> Option<cupcraft_audio::AudioBuffer> {
    let wav = work_dir().join(format!("{media_id}.wav"));
    if !wav.exists() {
        if let Some(loc) = cupcraft_media::FfmpegLocation::find() {
            let ok = cupcraft_media::ffmpeg::command(&loc.ffmpeg_path)
                .args(["-y", "-v", "error", "-i"])
                .arg(path)
                .args(["-vn", "-ac", "2", "-ar", "48000", "-c:a", "pcm_s16le"])
                .arg(&wav)
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
            if !ok {
                let _ = std::fs::remove_file(&wav);
            }
        }
    }
    if wav.exists() {
        if let Ok(buf) = cupcraft_audio::decode_audio_file(&wav) {
            return Some(buf);
        }
    }
    cupcraft_audio::decode_audio_file(path).ok()
}

pub fn media_dims(media: &Media) -> Option<(u32, u32)> {
    let w = media.extra.get("width")?.as_u64()? as u32;
    let h = media.extra.get("height")?.as_u64()? as u32;
    (w > 0 && h > 0).then_some((w, h))
}

/// ¿El medio aporta audio a la mezcla?
pub fn has_audio(media: &Media) -> bool {
    match media.kind.as_deref() {
        Some("audio") => true,
        Some("video") => media.extra.get("hasAudio").and_then(|v| v.as_bool()).unwrap_or(true),
        _ => false,
    }
}
