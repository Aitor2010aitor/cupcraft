//! Integración de síntesis de voz (TTS) abierta y local con Piper (TTS-Abierto).
//! Ejecuta los modelos ONNX 100% en local sin enviar datos a la nube.

use serde_json::{json, Value};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn find_piper_dir(root: &Path) -> Option<PathBuf> {
    let candidate = root.join("tools").join("piper");
    if candidate.join("piper.exe").is_file() {
        return Some(candidate);
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            let candidate2 = parent.join("tools").join("piper");
            if candidate2.join("piper.exe").is_file() {
                return Some(candidate2);
            }
            // In case of workspace target/debug
            if let Some(up) = parent.parent().and_then(|p| p.parent()) {
                let candidate3 = up.join("tools").join("piper");
                if candidate3.join("piper.exe").is_file() {
                    return Some(candidate3);
                }
            }
        }
    }
    None
}

/// Carpeta donde viven los modelos de voz (`tools/piper/models`).
pub fn models_dir(root: &Path) -> Option<PathBuf> {
    find_piper_dir(root).map(|d| d.join("models"))
}

pub fn is_available(root: &Path) -> bool {
    find_piper_dir(root).is_some()
}

pub fn list_voices(root: &Path) -> Value {
    let Some(piper_dir) = find_piper_dir(root) else {
        return json!({ "available": false, "voices": [] });
    };

    let models_dir = piper_dir.join("models");
    let mut voices = Vec::new();

    if let Ok(entries) = fs::read_dir(&models_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("onnx") {
                let stem = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string();

                let (label, gender, quality) = if stem.contains("davefx") {
                    ("Davefx (Español)".to_string(), "Masculina", "Alta calidad")
                } else if stem.contains("sharvard") {
                    ("Sharvard (Español)".to_string(), "Neutral", "Alta calidad")
                } else if stem.contains("carlfm") {
                    ("Carlfm (Español)".to_string(), "Femenina", "Ultrarrápida")
                } else {
                    // `es_MX-claude-high` → «Claude (es_MX)».
                    let mut p = stem.split('-');
                    let region = p.next().unwrap_or("");
                    let who = p.next().unwrap_or(stem.as_str());
                    let nice: String = who.chars().enumerate().map(|(i, c)| if i == 0 { c.to_ascii_uppercase() } else { c }).collect();
                    let quality = if p.next() == Some("high") { "Alta calidad" } else { "Estándar" };
                    (format!("{nice} ({region})"), "Voz", quality)
                };

                voices.push(json!({
                    "id": stem,
                    "name": format!("{} · {}", label, gender),
                    "description": quality,
                    "gender": gender,
                    "lang": if stem.starts_with("es_") { "es" } else { stem.split('_').next().unwrap_or("en") }
                }));
            }
        }
    }

    json!({
        "available": true,
        "voices": voices
    })
}

/// Algunas voces nuevas traen en `phoneme_id_map` fonemas de varios caracteres
/// («aɪ», «eɪ»…) que Piper 2023.11 no acepta («is not a single codepoint»).
/// espeak solo emite fonemas de un carácter, así que se pueden quitar sin riesgo.
pub fn sanitize_voice_config(onnx: &Path) {
    let json = PathBuf::from(format!("{}.json", onnx.display()));
    let Ok(text) = fs::read_to_string(&json) else { return };
    let Ok(mut v) = serde_json::from_str::<Value>(&text) else { return };
    let Some(map) = v.get_mut("phoneme_id_map").and_then(Value::as_object_mut) else { return };
    let bad: Vec<String> = map.keys().filter(|k| k.chars().count() > 1).cloned().collect();
    if bad.is_empty() {
        return;
    }
    for k in bad {
        map.remove(&k);
    }
    if let Ok(out) = serde_json::to_string(&v) {
        let _ = fs::write(&json, out);
    }
}

pub fn generate(root: &Path, text: &str, voice_id: &str, speed: f32) -> Result<Vec<u8>, String> {
    let text = text.trim();
    if text.is_empty() {
        return Err("El texto no puede estar vacío".to_string());
    }

    let piper_dir = find_piper_dir(root)
        .ok_or_else(|| "No se encontró el motor Piper TTS en tools/piper".to_string())?;

    let piper_exe = piper_dir.join("piper.exe");
    if !piper_exe.is_file() {
        return Err("No se encontró el ejecutable piper.exe".to_string());
    }

    let clean_voice = voice_id
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
        .collect::<String>();

    let voice_name = if clean_voice.is_empty() {
        "es_ES-davefx-medium"
    } else {
        &clean_voice
    };

    let model_path = piper_dir.join("models").join(format!("{}.onnx", voice_name));
    if !model_path.is_file() {
        return Err(format!("No se encontró el modelo de voz {}", voice_name));
    }

    sanitize_voice_config(&model_path);
    let speed_clamped = speed.clamp(0.5, 2.5);
    let length_scale = 1.0 / speed_clamped;

    let temp_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let temp_wav = std::env::temp_dir().join(format!("cupcraft_tts_{}.wav", temp_id));

    let mut piper_cmd = Command::new(&piper_exe);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        piper_cmd.creation_flags(0x0800_0000);
    }
    let mut child = piper_cmd
        .current_dir(&piper_dir)
        .arg("--model")
        .arg(&model_path)
        .arg("--output_file")
        .arg(&temp_wav)
        .arg("--length_scale")
        .arg(format!("{:.3}", length_scale))
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Error al ejecutar Piper TTS: {}", e))?;

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(text.as_bytes());
        let _ = stdin.flush();
    }

    let output = child
        .wait_with_output()
        .map_err(|e| format!("Error esperando a Piper TTS: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let _ = fs::remove_file(&temp_wav);
        return Err(format!("Piper TTS falló: {}", stderr.trim()));
    }

    if !temp_wav.is_file() {
        let _ = fs::remove_file(&temp_wav);
        return Err("Piper TTS no generó el archivo de audio".to_string());
    }

    let bytes = fs::read(&temp_wav).map_err(|e| format!("Error leyendo audio generado: {}", e))?;
    let _ = fs::remove_file(&temp_wav);

    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_voices_with_nonexistent_root() {
        let res = list_voices(Path::new("non_existent_folder_xyz"));
        assert!(res.is_object());
    }

    #[test]
    fn test_generate_empty_text_fails() {
        let err = generate(Path::new("."), "", "es_ES-davefx-medium", 1.0);
        assert!(err.is_err());
        assert_eq!(err.unwrap_err(), "El texto no puede estar vacío");
    }
}
