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
#[cfg(unix)]
const SOURCE_URL: &str = "https://github.com/ggml-org/whisper.cpp/archive/refs/tags/v1.7.6.tar.gz";
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
    #[cfg(not(windows))]
    {
        return crate::app::app_dir().join("whisper");
    }
    #[cfg(windows)]
    cwd
}

fn find_cli(root: &Path) -> Option<PathBuf> {
    let mut stack = vec![root.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in fs::read_dir(&d).ok()?.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.file_name().and_then(|n| n.to_str()).map(|n| n.eq_ignore_ascii_case("whisper-cli.exe") || n == "whisper-cli").unwrap_or(false) {
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

#[cfg(not(unix))]
fn install_engine(d: &Path, progress: &dyn Fn(String, f32)) -> Result<(), String> {
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
    if !st.success() || find_cli(d).is_none() {
        return Err("el paquete descargado no contiene whisper-cli.exe".into());
    }
    Ok(())
}

#[cfg(unix)]
fn have(cmd: &str) -> bool {
    std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).any(|d| d.join(cmd).is_file()))
        .unwrap_or(false)
        || ["/usr/bin", "/usr/local/bin", "/bin"].iter().any(|d| Path::new(d).join(cmd).is_file())
}

/// Instala con permiso de administrador (`pkexec`, ventana de contraseña del
/// sistema) los paquetes que falten: compilador, cmake y FFmpeg.
#[cfg(unix)]
pub fn install_system_deps(need_build: bool, progress: &dyn Fn(String, f32)) -> Result<(), String> {
    let mut pkgs: Vec<&str> = Vec::new();
    if need_build {
        if !have("cmake") {
            pkgs.push("cmake");
        }
        if !have("make") || !(have("g++") || have("c++")) {
            pkgs.push("build-essential");
        }
    }
    if cupcraft_media::FfmpegLocation::refresh().is_none() {
        pkgs.push("ffmpeg");
    }
    if pkgs.is_empty() {
        return Ok(());
    }
    if !have("apt-get") {
        return Err(format!("instala estos paquetes con tu gestor de paquetes y vuelve a intentarlo: {}", pkgs.join(", ")));
    }
    if !have("pkexec") {
        return Err(format!("falta pkexec; ejecuta en una terminal: sudo apt-get install {}", pkgs.join(" ")));
    }
    progress(format!("Pidiendo permiso de administrador para instalar {}…", pkgs.join(", ")), 0.02);
    let st = command("pkexec")
        .args(["env", "DEBIAN_FRONTEND=noninteractive", "apt-get", "install", "-y"])
        .args(&pkgs)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| format!("no se pudo pedir permiso: {e}"))?;
    match st.code() {
        Some(0) => {}
        Some(126) | Some(127) => return Err("permiso de administrador denegado o cancelado".into()),
        _ => return Err(format!("apt-get no pudo instalar {} (¿hay conexión a internet?)", pkgs.join(", "))),
    }
    cupcraft_media::FfmpegLocation::refresh();
    Ok(())
}

/// Linux: instala dependencias (pkexec) y compila whisper.cpp desde su código.
#[cfg(unix)]
fn install_engine(d: &Path, progress: &dyn Fn(String, f32)) -> Result<(), String> {
    use std::io::{BufRead, BufReader};
    use std::os::unix::fs::PermissionsExt;
    install_system_deps(true, progress)?;
    let tgz = d.join("whisper-src.tar.gz");
    download(SOURCE_URL, &tgz, "código del motor Whisper", 0.05, 0.05, progress)?;
    let src = d.join("src");
    let _ = fs::remove_dir_all(&src);
    fs::create_dir_all(&src).map_err(|e| e.to_string())?;
    let st = command("tar")
        .arg("-xzf")
        .arg(&tgz)
        .arg("-C")
        .arg(&src)
        .arg("--strip-components=1")
        .status()
        .map_err(|e| format!("no se pudo descomprimir: {e}"))?;
    let _ = fs::remove_file(&tgz);
    if !st.success() {
        return Err("no se pudo descomprimir el código del motor".into());
    }
    progress("Compilando el motor Whisper (puede tardar varios minutos)…".into(), 0.1);
    let build = src.join("build");
    let st = command("cmake")
        .arg("-S")
        .arg(&src)
        .arg("-B")
        .arg(&build)
        .args(["-DCMAKE_BUILD_TYPE=Release", "-DBUILD_SHARED_LIBS=OFF", "-DWHISPER_BUILD_TESTS=OFF", "-DGGML_NATIVE=OFF"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| format!("cmake: {e}"))?;
    if !st.success() {
        let _ = fs::remove_dir_all(&src);
        return Err("cmake no pudo preparar la compilación".into());
    }
    let jobs = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(2).to_string();
    let mut child = command("cmake")
        .arg("--build")
        .arg(&build)
        .args(["--config", "Release", "--target", "whisper-cli", "-j", &jobs])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("cmake: {e}"))?;
    if let Some(out) = child.stdout.take() {
        for line in BufReader::new(out).lines().map_while(Result::ok) {
            // Líneas de make: "[ 45%] Building CXX object …"
            if let Some(pct) = line.trim_start().strip_prefix('[').and_then(|r| r.split('%').next()).and_then(|n| n.trim().parse::<f32>().ok()) {
                progress("Compilando el motor Whisper (puede tardar varios minutos)…".into(), 0.1 + 0.1 * pct / 100.0);
            }
        }
    }
    let ok = child.wait().map(|s| s.success()).unwrap_or(false);
    let bin = build.join("bin").join("whisper-cli");
    if !ok || !bin.is_file() {
        let _ = fs::remove_dir_all(&src);
        return Err("la compilación del motor Whisper falló".into());
    }
    let dest = d.join("whisper-cli");
    fs::copy(&bin, &dest).map_err(|e| e.to_string())?;
    let _ = fs::set_permissions(&dest, fs::Permissions::from_mode(0o755));
    let _ = fs::remove_dir_all(&src);
    Ok(())
}

/// Descarga e instala motor + modelo en `tools\whisper`.
pub fn install(progress: &dyn Fn(String, f32)) -> Result<(), String> {
    let d = dir();
    fs::create_dir_all(&d).map_err(|e| e.to_string())?;
    if find_cli(&d).is_none() {
        install_engine(&d, progress)?;
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
