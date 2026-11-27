//! Subtítulos automáticos locales con whisper.cpp (`whisper-cli.exe`), igual
//! que Piper para la voz: un proceso hijo, sin nube, sin administrador.
//!
//! * `ready()`      — ¿están el motor y el modelo en `tools\whisper`?
//! * `install()`    — descarga motor (zip) y modelo `ggml-tiny.bin` (acción del
//!   usuario desde el panel; informa del progreso).
//! * `transcribe()` — extrae el audio del clip con FFmpeg (16 kHz mono), ejecuta
//!   whisper y devuelve `{text,start,end}` en segundos del propio clip.

use cupcraft_media::ffmpeg::command;
use serde_json::Value;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Stdio;

const ENGINE_URL: &str = "https://github.com/ggml-org/whisper.cpp/releases/download/b5907/whisper-bin-x64.zip";
const MODEL_URL: &str = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin";
const MODEL_FILE: &str = "ggml-tiny.bin";

/// Carpeta `tools\whisper` (cwd, o junto al ejecutable y sus padres).
pub fn dir() -> PathBuf {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")).join("tools").join("whisper");
    if cwd.is_dir() {
        return cwd;
    }
    if let Ok(exe) = std::env::current_exe() {
        let mut d = exe.parent().map(Path::to_path_buf);
        for _ in 0..5 {
            let Some(cur) = d.clone() else { break };
            let cand = cur.join("tools").join("whisper");
            if cand.is_dir() {
                return cand;
            }
            d = cur.parent().map(Path::to_path_buf);
        }
    }
    cwd
}

fn find_cli(root: &Path) -> Option<PathBuf> {
    let mut stack = vec![root.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in fs::read_dir(&d).ok()?.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.file_name().and_then(|n| n.to_str()).map(|n| n.eq_ignore_ascii_case("whisper-cli.exe")).unwrap_or(false) {
                return Some(p);
            }
        }
    }
    None
}

pub fn ready() -> bool {
    let d = dir();
    find_cli(&d).is_some() && d.join(MODEL_FILE).is_file()
}

pub(crate) fn download(url: &str, dest: &Path, label: &str, base: f32, span: f32, progress: &dyn Fn(String, f32)) -> Result<(), String> {
    let resp = ureq::get(url).call().map_err(|e| format!("descarga fallida ({label}): {e}"))?;
    let total = resp.header("Content-Length").and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
    let mut reader = resp.into_reader();
    let tmp = dest.with_extension("part");
    let mut out = fs::File::create(&tmp).map_err(|e| e.to_string())?;
    let mut buf = vec![0u8; 256 * 1024];
    let mut done = 0u64;
    loop {
        let n = reader.read(&mut buf).map_err(|e| format!("descarga interrumpida ({label}): {e}"))?;
        if n == 0 {
            break;
        }
        out.write_all(&buf[..n]).map_err(|e| e.to_string())?;
        done += n as u64;
        if total > 0 {
            progress(format!("Descargando {label}…"), base + span * (done as f32 / total as f32));
        }
    }
    drop(out);
    fs::rename(&tmp, dest).map_err(|e| e.to_string())
}

/// Descarga e instala motor + modelo en `tools\whisper`.
pub fn install(progress: &dyn Fn(String, f32)) -> Result<(), String> {
    let d = dir();
    fs::create_dir_all(&d).map_err(|e| e.to_string())?;
    if find_cli(&d).is_none() {
        let zip = d.join("whisper-engine.zip");
        download(ENGINE_URL, &zip, "motor Whisper", 0.0, 0.2, progress)?;
        progress("Instalando motor…".into(), 0.2);
        let st = command("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command"])
            .arg(format!("Expand-Archive -LiteralPath '{}' -DestinationPath '{}' -Force", zip.display(), d.display()))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|e| format!("no se pudo descomprimir: {e}"))?;
        let _ = fs::remove_file(&zip);
        if !st.success() || find_cli(&d).is_none() {
            return Err("el paquete descargado no contiene whisper-cli.exe".into());
        }
    }
    if !d.join(MODEL_FILE).is_file() {
        download(MODEL_URL, &d.join(MODEL_FILE), "modelo Whisper (~78 MB)", 0.2, 0.8, progress)?;
    }
    progress("Listo".into(), 1.0);
    Ok(())
}

/// Transcribe `[in_point, in_point+duration]` del medio. Tiempos devueltos
/// relativos al inicio del tramo (segundos). `lang`: "auto", "es", "en"…
/// Con `words`, whisper devuelve una palabra por segmento y se agrupan en
/// líneas de pocas palabras con sus tiempos (`words`) para el karaoke.
pub fn transcribe(media: &Path, in_point: f64, duration: f64, lang: &str, words: bool) -> Result<Vec<Value>, String> {
    let d = dir();
    let cli = find_cli(&d).ok_or("falta el motor Whisper")?;
    let model = d.join(MODEL_FILE);
    if !model.is_file() {
        return Err("falta el modelo Whisper".into());
    }
    let ff = cupcraft_media::FfmpegLocation::find().ok_or("FFmpeg no encontrado")?;
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|t| t.as_millis()).unwrap_or(0);
    let wav = std::env::temp_dir().join(format!("cupcraft-stt-{stamp}.wav"));
    let st = command(&ff.ffmpeg_path)
        .args(["-y", "-v", "error", "-ss", &format!("{:.3}", in_point.max(0.0)), "-t", &format!("{:.3}", duration.max(0.1)), "-i"])
        .arg(media)
        .args(["-vn", "-ac", "1", "-ar", "16000", "-c:a", "pcm_s16le"])
        .arg(&wav)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| format!("FFmpeg: {e}"))?;
    if !st.success() || !wav.is_file() {
        return Err("no se pudo extraer el audio (¿el clip tiene sonido?)".into());
    }
    let base = std::env::temp_dir().join(format!("cupcraft-stt-{stamp}"));
    let mut cmd = command(&cli);
    cmd.arg("-m").arg(&model).arg("-f").arg(&wav).args(["-l", lang, "-osrt", "-of"]).arg(&base).args(["-np"]);
    if words {
        cmd.args(["-ml", "1", "-sow"]);
    }
    let out = cmd
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("whisper-cli: {e}"))?;
    let _ = fs::remove_file(&wav);
    let srt_path = base.with_extension("srt");
    let text = fs::read_to_string(&srt_path);
    let _ = fs::remove_file(&srt_path);
    let text = text.map_err(|_| format!("Whisper no produjo subtítulos: {}", String::from_utf8_lossy(&out.stderr).lines().last().unwrap_or("")))?;
    let items = crate::panels::parse_srt(&text);
    Ok(if words { crate::karaoke::group_words(&items, 4) } else { items })
}
