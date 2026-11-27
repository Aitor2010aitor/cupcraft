//! Más voces para el texto a voz, todo en local:
//!
//! * [`CATALOG`] / [`install_voice`] — voces Piper extra (español de México y
//!   Argentina, inglés…) que se descargan **cuando el usuario lo pide** desde el
//!   repositorio público `rhasspy/piper-voices` y se guardan en `tools\piper\models`.
//! * [`STYLES`] / [`apply_style`] — estilos de voz «de vídeo corto» (ardilla,
//!   grave, robot, radio…) aplicados con FFmpeg sobre el WAV que genera Piper.

use cupcraft_media::ffmpeg::command;
use std::fs;
use std::path::Path;
use std::process::Stdio;

const BASE: &str = "https://huggingface.co/rhasspy/piper-voices/resolve/main";

/// Voz descargable: id (nombre del modelo), ruta en el repositorio, etiqueta y peso aproximado.
pub struct CatalogVoice {
    pub id: &'static str,
    pub path: &'static str,
    pub label: &'static str,
    pub size: &'static str,
}

pub const CATALOG: &[CatalogVoice] = &[
    CatalogVoice { id: "es_MX-claude-high", path: "es/es_MX/claude/high", label: "Claude · Español (México)", size: "~60 MB" },
    CatalogVoice { id: "es_MX-ald-medium", path: "es/es_MX/ald/medium", label: "Ald · Español (México)", size: "~60 MB" },
    CatalogVoice { id: "es_AR-daniela-high", path: "es/es_AR/daniela/high", label: "Daniela · Español (Argentina)", size: "~60 MB" },
    CatalogVoice { id: "en_US-ryan-high", path: "en/en_US/ryan/high", label: "Ryan · Inglés (EE. UU.), narrador", size: "~115 MB" },
    CatalogVoice { id: "en_US-joe-medium", path: "en/en_US/joe/medium", label: "Joe · Inglés (EE. UU.)", size: "~60 MB" },
    CatalogVoice { id: "en_US-amy-medium", path: "en/en_US/amy/medium", label: "Amy · Inglés (EE. UU.)", size: "~60 MB" },
    CatalogVoice { id: "en_GB-alan-medium", path: "en/en_GB/alan/medium", label: "Alan · Inglés (Reino Unido)", size: "~60 MB" },
];

/// ¿Está ya instalada esa voz de [`CATALOG`]?
pub fn installed(models: &Path, id: &str) -> bool {
    models.join(format!("{id}.onnx")).is_file() && models.join(format!("{id}.onnx.json")).is_file()
}

/// Descarga el modelo `.onnx` y su `.onnx.json` a `models`.
pub fn install_voice(models: &Path, v: &CatalogVoice, progress: &dyn Fn(String, f32)) -> Result<(), String> {
    fs::create_dir_all(models).map_err(|e| e.to_string())?;
    let json = models.join(format!("{}.onnx.json", v.id));
    if !json.is_file() {
        super::whisper::download(&format!("{BASE}/{}/{}.onnx.json", v.path, v.id), &json, "configuración de la voz", 0.0, 0.02, progress)?;
    }
    let onnx = models.join(format!("{}.onnx", v.id));
    if !onnx.is_file() {
        super::whisper::download(&format!("{BASE}/{}/{}.onnx", v.path, v.id), &onnx, v.label, 0.02, 0.98, progress)?;
    }
    progress("Listo".into(), 1.0);
    Ok(())
}

/// Estilo de voz: id, etiqueta y factor de tono (1.0 = sin cambio) + filtro extra.
pub struct Style {
    pub id: &'static str,
    pub label: &'static str,
    pub pitch: f64,
    pub extra: &'static str,
}

pub const STYLES: &[Style] = &[
    Style { id: "none", label: "Original", pitch: 1.0, extra: "" },
    Style { id: "ardilla", label: "Ardilla (aguda)", pitch: 1.45, extra: "" },
    Style { id: "joven", label: "Joven", pitch: 1.18, extra: "" },
    Style { id: "grave", label: "Grave", pitch: 0.82, extra: "" },
    Style { id: "gigante", label: "Gigante", pitch: 0.65, extra: "aecho=0.8:0.6:40:0.3" },
    Style { id: "robot", label: "Robot", pitch: 1.0, extra: "afftfilt=real='hypot(re,im)':imag='0':win_size=512:overlap=0.75" },
    Style { id: "radio", label: "Radio / teléfono", pitch: 1.0, extra: "highpass=f=400,lowpass=f=3200,acompressor=threshold=0.1:ratio=6" },
    Style { id: "eco", label: "Eco de cueva", pitch: 1.0, extra: "aecho=0.8:0.88:70|140:0.5|0.3" },
];

/// Filtro FFmpeg del estilo para un WAV de `rate` Hz (`None` = sin cambios).
fn filter_for(style: &Style, rate: u32) -> Option<String> {
    let mut parts = Vec::new();
    if (style.pitch - 1.0).abs() > 0.001 {
        // Cambiar el tono sin cambiar la duración: remuestrear y recuperar el tempo.
        parts.push(format!("asetrate={:.0},aresample={rate},atempo={:.4}", rate as f64 * style.pitch, 1.0 / style.pitch));
    }
    if !style.extra.is_empty() {
        parts.push(style.extra.to_string());
    }
    (!parts.is_empty()).then(|| parts.join(","))
}

/// Aplica el estilo `id` al WAV. Con `none` (o un id desconocido) lo devuelve igual.
pub fn apply_style(wav: Vec<u8>, id: &str) -> Result<Vec<u8>, String> {
    let Some(style) = STYLES.iter().find(|s| s.id == id) else { return Ok(wav) };
    let rate = wav.get(24..28).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]])).filter(|r| (8000..=96000).contains(r)).unwrap_or(22050);
    let Some(filter) = filter_for(style, rate) else { return Ok(wav) };
    let ff = cupcraft_media::FfmpegLocation::find().ok_or("FFmpeg no encontrado (hace falta para los estilos de voz)")?;
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|t| t.as_nanos()).unwrap_or(0);
    let tmp = std::env::temp_dir();
    let (src, dst) = (tmp.join(format!("cupcraft-vox-in-{stamp}.wav")), tmp.join(format!("cupcraft-vox-out-{stamp}.wav")));
    fs::write(&src, &wav).map_err(|e| e.to_string())?;
    let st = command(&ff.ffmpeg_path)
        .args(["-y", "-v", "error", "-i"])
        .arg(&src)
        .args(["-af", &filter, "-c:a", "pcm_s16le"])
        .arg(&dst)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    let out = fs::read(&dst);
    let _ = fs::remove_file(&src);
    let _ = fs::remove_file(&dst);
    match (st, out) {
        (Ok(s), Ok(bytes)) if s.success() && !bytes.is_empty() => Ok(bytes),
        _ => Err(format!("no se pudo aplicar el estilo «{}»", style.label)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn el_filtro_de_tono_conserva_la_duracion() {
        let s = STYLES.iter().find(|s| s.id == "ardilla").unwrap();
        let f = filter_for(s, 22050).unwrap();
        assert!(f.starts_with("asetrate=31972,aresample=22050,atempo=0.6897"), "{f}");
        assert!(filter_for(&STYLES[0], 22050).is_none());
    }

    #[test]
    fn los_ids_del_catalogo_son_unicos() {
        let mut ids: Vec<_> = CATALOG.iter().map(|v| v.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), CATALOG.len());
    }
}
