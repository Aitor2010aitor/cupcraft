//! Autoactualización desde GitHub Releases, sin API de GitHub ni token.
//!
//! Cada release publica un `version.json` y un binario por plataforma. Se leen
//! por la URL estable `releases/latest/download/<archivo>`, que es pública y no
//! tiene límite de peticiones (el repositorio debe ser público).
//!
//! `version.json`:
//! ```json
//! {
//!   "version": "0.4.6",
//!   "notes": "Qué hay de nuevo",
//!   "assets": {
//!     "windows-x86_64": { "file": "cupcraft-windows-x86_64.exe", "sha256": "…" },
//!     "linux-x86_64":   { "file": "cupcraft-linux-x86_64",       "sha256": "…" },
//!     "macos-aarch64":  { "file": "cupcraft-macos-aarch64",      "sha256": "…" }
//!   }
//! }
//! ```
//! La clave de plataforma es `<std::env::consts::OS>-<std::env::consts::ARCH>`.

use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::OnceLock;
use std::time::Duration;

use eframe::egui;
use sha2::{Digest, Sha256};

use crate::theme;

const REPO: &str = "Aitor2010aitor/cupcraft";
const CURRENT: &str = env!("CARGO_PKG_VERSION");

#[derive(Clone)]
pub struct Update {
    pub version: String,
    pub notes: String,
    file: String,
    sha256: String,
}

enum Msg {
    Found(Update),
    Progress(f32),
    Ready,
    Failed(String),
}

enum State {
    Idle,
    Available(Update),
    Downloading(f32),
    Ready,
    Failed(String),
    Dismissed,
}

pub struct Updater {
    tx: Sender<Msg>,
    rx: Receiver<Msg>,
    state: State,
}

fn base_url() -> String {
    format!("https://github.com/{REPO}/releases/latest/download")
}

fn platform_key() -> String {
    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}

/// Compara versiones tipo `0.4.6` / `v0.4.6` / `0.4.8.1`. Un sufijo numérico
/// (`0.4.8-1`, la forma de Cargo de `0.4.8.1`) cuenta como un cuarto número;
/// otros sufijos como `-beta` se ignoran.
fn parse_version(v: &str) -> Vec<u64> {
    let v = v.trim().trim_start_matches('v');
    let mut parts = v.split(['-', '+']);
    let mut out: Vec<u64> = parts.next().unwrap_or("").split('.').map(|p| p.parse::<u64>().unwrap_or(0)).collect();
    if let Some(n) = v.split_once('-').and_then(|(_, rest)| rest.parse::<u64>().ok()) {
        out.push(n);
    }
    out
}

fn is_newer(remote: &str, local: &str) -> bool {
    let (mut a, mut b) = (parse_version(remote), parse_version(local));
    let n = a.len().max(b.len());
    a.resize(n, 0);
    b.resize(n, 0);
    a > b
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(8))
        .timeout_read(Duration::from_secs(30))
        .build()
}

fn fetch_latest() -> Result<Option<Update>, String> {
    let body = agent()
        .get(&format!("{}/version.json", base_url()))
        .call()
        .map_err(|e| format!("no se pudo comprobar actualizaciones: {e}"))?
        .into_string()
        .map_err(|e| e.to_string())?;
    let v: serde_json::Value = serde_json::from_str(&body).map_err(|e| format!("version.json inválido: {e}"))?;
    let version = v["version"].as_str().ok_or("version.json sin campo version")?.to_string();
    if !is_newer(&version, CURRENT) {
        return Ok(None);
    }
    let asset = &v["assets"][platform_key()];
    let (Some(file), Some(sha)) = (asset["file"].as_str(), asset["sha256"].as_str()) else {
        // Hay versión nueva pero no para esta plataforma: no molestar.
        return Ok(None);
    };
    Ok(Some(Update {
        version,
        notes: v["notes"].as_str().unwrap_or("").to_string(),
        file: file.to_string(),
        sha256: sha.to_lowercase(),
    }))
}

/// Quita el sufijo " (deleted)" que Linux añade a /proc/self/exe cuando el
/// archivo en ejecución ya fue reemplazado.
fn clean_exe_path(p: PathBuf) -> PathBuf {
    let trimmed = p.to_string_lossy().strip_suffix(" (deleted)").map(PathBuf::from);
    trimmed.unwrap_or(p)
}

/// Ruta del ejecutable en ejecución, fijada la primera vez que se pide (al
/// arrancar, en `cleanup_old`). En Linux `current_exe()` lee /proc/self/exe, que
/// sigue al archivo aunque se renombre: tras mover el ejecutable a `.old`
/// devolvería esa ruta y `relaunch()` volvería a abrir la versión anterior en
/// bucle, con el aviso de actualización saliendo siempre.
fn exe_path() -> Result<PathBuf, String> {
    static EXE: OnceLock<Result<PathBuf, String>> = OnceLock::new();
    EXE.get_or_init(|| std::env::current_exe().map(clean_exe_path).map_err(|e| e.to_string()))
        .clone()
}

fn old_path(exe: &std::path::Path) -> PathBuf {
    let mut s = exe.as_os_str().to_owned();
    s.push(".old");
    PathBuf::from(s)
}

/// Borra el ejecutable anterior que dejó una actualización previa.
pub fn cleanup_old() {
    if let Ok(exe) = exe_path() {
        let _ = std::fs::remove_file(old_path(&exe));
    }
}

/// Crea un archivo temporal que solo puede leer y escribir el usuario actual.
/// `create_new` (O_EXCL) no sigue enlaces simbólicos preexistentes, y se prefiere
/// `XDG_RUNTIME_DIR` (privado) a /tmp: el archivo acabará instalándose como root,
/// así que nadie más debe poder sustituirlo tras verificar el SHA-256.
#[cfg(unix)]
fn create_private_temp() -> Result<(PathBuf, std::fs::File), String> {
    use std::os::unix::fs::OpenOptionsExt;
    let dir = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .filter(|p| p.is_dir())
        .unwrap_or_else(std::env::temp_dir);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let path = dir.join(format!("cupcraft-update-{}-{nanos}", std::process::id()));
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&path)
        .map_err(|e| format!("no se puede escribir en la carpeta temporal: {e}"))?;
    Ok((path, file))
}

#[cfg(not(unix))]
fn create_private_temp() -> Result<(PathBuf, std::fs::File), String> {
    Err("no se puede escribir junto al ejecutable".into())
}

/// Sustituye el ejecutable con permisos de administrador (`pkexec` muestra el
/// diálogo de contraseña del sistema). Se copia junto al destino y se renombra:
/// un `mv` sí puede reemplazar un ejecutable en uso.
#[cfg(unix)]
fn install_privileged(src: &std::path::Path, exe: &std::path::Path) -> Result<(), String> {
    let status = std::process::Command::new("pkexec")
        .args(["/bin/sh", "-c", r#"install -m 755 "$1" "$2.new" && mv -f "$2.new" "$2""#, "sh"])
        .arg(src)
        .arg(exe)
        .status()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                "falta pkexec (polkit): instala el .deb de la nueva versión a mano".to_string()
            } else {
                format!("no se pudieron pedir permisos de administrador: {e}")
            }
        })?;
    match status.code() {
        Some(0) => Ok(()),
        Some(126) => Err("cancelaste la petición de permisos de administrador".into()),
        Some(127) => Err("no se obtuvo autorización de administrador".into()),
        _ => Err("no se pudo instalar la nueva versión con permisos de administrador".into()),
    }
}

#[cfg(not(unix))]
fn install_privileged(_src: &std::path::Path, _exe: &std::path::Path) -> Result<(), String> {
    Err("no se pudo reemplazar el ejecutable".into())
}

fn download_and_install(u: &Update, progress: &dyn Fn(f32)) -> Result<(), String> {
    let exe = exe_path()?;
    let mut new_s = exe.as_os_str().to_owned();
    new_s.push(".new");
    let mut new = PathBuf::from(new_s);

    let resp = agent()
        .get(&format!("{}/{}", base_url(), u.file))
        .call()
        .map_err(|e| format!("descarga fallida: {e}"))?;
    let total = resp.header("Content-Length").and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
    let mut reader = resp.into_reader();
    // Junto al ejecutable si se puede escribir ahí. Si no (en Linux, /usr/bin
    // tras instalar el .deb) se descarga a una carpeta temporal privada y el
    // último paso se hace con permisos de administrador.
    let mut privileged = false;
    let mut out = match std::fs::File::create(&new) {
        Ok(f) => f,
        Err(e) if cfg!(unix) && e.kind() == std::io::ErrorKind::PermissionDenied => {
            let (path, f) = create_private_temp()?;
            privileged = true;
            new = path;
            f
        }
        Err(e) => return Err(format!("no se puede escribir junto al ejecutable: {e}")),
    };
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 256 * 1024];
    let mut done = 0u64;
    let result = (|| -> Result<(), String> {
        loop {
            let n = reader.read(&mut buf).map_err(|e| format!("descarga interrumpida: {e}"))?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
            out.write_all(&buf[..n]).map_err(|e| e.to_string())?;
            done += n as u64;
            if total > 0 {
                progress(done as f32 / total as f32);
            }
        }
        out.flush().map_err(|e| e.to_string())
    })();
    drop(out);
    if let Err(e) = result {
        let _ = std::fs::remove_file(&new);
        return Err(e);
    }
    let digest: String = hasher.finalize().iter().map(|b| format!("{b:02x}")).collect();
    if digest != u.sha256 {
        let _ = std::fs::remove_file(&new);
        return Err("la descarga está corrupta (SHA256 no coincide)".into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&new, std::fs::Permissions::from_mode(0o755));
    }
    if privileged {
        let r = install_privileged(&new, &exe);
        let _ = std::fs::remove_file(&new);
        return r;
    }
    // Un ejecutable en uso no se puede sobrescribir en Windows, pero sí renombrar.
    let old = old_path(&exe);
    let _ = std::fs::remove_file(&old);
    std::fs::rename(&exe, &old).map_err(|e| format!("no se pudo reemplazar el ejecutable: {e}"))?;
    if let Err(e) = std::fs::rename(&new, &exe) {
        let _ = std::fs::rename(&old, &exe); // deshacer
        let _ = std::fs::remove_file(&new);
        return Err(format!("no se pudo instalar la nueva versión: {e}"));
    }
    Ok(())
}

fn whats_new_path() -> PathBuf {
    crate::app::app_dir().join("whats_new.json")
}

/// Deja anotado qué versión se acaba de instalar para enseñar sus novedades al
/// arrancar de nuevo.
fn save_whats_new(u: &Update) {
    let dir = crate::app::app_dir();
    let _ = std::fs::create_dir_all(&dir);
    let json = serde_json::json!({ "version": u.version, "notes": u.notes });
    let _ = std::fs::write(whats_new_path(), json.to_string());
}

/// Si la app acaba de actualizarse devuelve `(versión, notas)` una sola vez.
/// Solo vale si la versión anotada es la que está corriendo ahora.
pub fn take_whats_new() -> Option<(String, String)> {
    let path = whats_new_path();
    let text = std::fs::read_to_string(&path).ok()?;
    let _ = std::fs::remove_file(&path);
    let v: serde_json::Value = serde_json::from_str(&text).ok()?;
    let version = v["version"].as_str()?.to_string();
    if parse_version(&version) != parse_version(CURRENT) {
        return None;
    }
    Some((version, v["notes"].as_str().unwrap_or("").to_string()))
}

/// Lanza la nueva versión y cierra el proceso actual.
pub fn relaunch() -> ! {
    if let Ok(exe) = exe_path() {
        let _ = std::process::Command::new(exe).args(std::env::args().skip(1)).spawn();
    }
    std::process::exit(0);
}

impl Updater {
    /// Crea el actualizador y lanza la comprobación en segundo plano.
    pub fn start(ctx: &egui::Context) -> Self {
        cleanup_old();
        let (tx, rx) = channel();
        let t = tx.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            // Los fallos de red al comprobar se ignoran en silencio.
            if let Ok(Some(u)) = fetch_latest() {
                let _ = t.send(Msg::Found(u));
                ctx.request_repaint();
            }
        });
        Self { tx, rx, state: State::Idle }
    }

    fn install(&mut self, ctx: &egui::Context, u: Update) {
        self.state = State::Downloading(0.0);
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let (ptx, pctx) = (tx.clone(), ctx.clone());
            let r = download_and_install(&u, &move |p| {
                let _ = ptx.send(Msg::Progress(p));
                pctx.request_repaint();
            });
            if r.is_ok() {
                save_whats_new(&u);
            }
            let _ = tx.send(match r {
                Ok(()) => Msg::Ready,
                Err(e) => Msg::Failed(e),
            });
            ctx.request_repaint();
        });
    }

    /// Dibuja el aviso. Devuelve `true` cuando la nueva versión ya está
    /// instalada y la app debe guardar y reiniciarse.
    pub fn ui(&mut self, ctx: &egui::Context) -> bool {
        while let Ok(m) = self.rx.try_recv() {
            self.state = match m {
                Msg::Found(u) => State::Available(u),
                Msg::Progress(p) => State::Downloading(p),
                Msg::Ready => State::Ready,
                Msg::Failed(e) => State::Failed(e),
            };
        }
        let mut start: Option<Update> = None;
        let mut dismiss = false;
        let mut restart = false;
        if !matches!(self.state, State::Idle | State::Dismissed) {
            egui::Area::new(egui::Id::new("cupcraft-updater"))
                .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-20.0, -20.0))
                .order(egui::Order::Tooltip)
                .show(ctx, |ui| {
                    egui::Frame::NONE
                        .fill(theme::PANEL3)
                        .stroke(egui::Stroke::new(1.0, theme::ACCENT2))
                        .corner_radius(egui::CornerRadius::from(10))
                        .inner_margin(egui::Margin::symmetric(16, 12))
                        .show(ui, |ui| {
                            ui.set_max_width(300.0);
                            match &self.state {
                                State::Available(u) => {
                                    ui.label(egui::RichText::new(format!("Nueva versión {} disponible", u.version)).color(theme::TEXT).strong());
                                    if !u.notes.is_empty() {
                                        ui.label(egui::RichText::new(&u.notes).color(theme::MUTED).size(12.0));
                                    }
                                    ui.add_space(6.0);
                                    ui.horizontal(|ui| {
                                        if ui.button("Actualizar ahora").clicked() {
                                            start = Some(u.clone());
                                        }
                                        if ui.button("Más tarde").clicked() {
                                            dismiss = true;
                                        }
                                    });
                                }
                                State::Downloading(p) => {
                                    ui.label(egui::RichText::new("Descargando actualización…").color(theme::TEXT));
                                    ui.add(egui::ProgressBar::new(*p).show_percentage());
                                }
                                State::Ready => {
                                    ui.label(egui::RichText::new("Actualización instalada").color(theme::TEXT).strong());
                                    ui.label(egui::RichText::new("Se guardará tu proyecto y cupcraft se reiniciará.").color(theme::MUTED).size(12.0));
                                    ui.add_space(6.0);
                                    if ui.button("Reiniciar ahora").clicked() {
                                        restart = true;
                                    }
                                }
                                State::Failed(e) => {
                                    ui.label(egui::RichText::new("No se pudo actualizar").color(theme::DANGER).strong());
                                    ui.label(egui::RichText::new(e).color(theme::MUTED).size(12.0));
                                    ui.add_space(6.0);
                                    if ui.button("Cerrar").clicked() {
                                        dismiss = true;
                                    }
                                }
                                State::Idle | State::Dismissed => {}
                            }
                        });
                });
        }
        if let Some(u) = start {
            self.install(ctx, u);
        }
        if dismiss {
            self.state = State::Dismissed;
        }
        restart
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exe_path_drops_deleted_suffix() {
        assert_eq!(clean_exe_path(PathBuf::from("/usr/bin/cupcraft (deleted)")), PathBuf::from("/usr/bin/cupcraft"));
        assert_eq!(clean_exe_path(PathBuf::from("/usr/bin/cupcraft")), PathBuf::from("/usr/bin/cupcraft"));
    }

    #[test]
    fn version_compare() {
        assert!(is_newer("0.4.6", "0.4.5"));
        assert!(is_newer("v0.5.0", "0.4.9"));
        assert!(is_newer("1.0", "0.9.9"));
        assert!(!is_newer("0.4.5", "0.4.5"));
        assert!(!is_newer("0.4.4", "0.4.5"));
        assert!(!is_newer("0.4.5-beta", "0.4.5"));
        assert!(is_newer("0.4.8.1", "0.4.8"));
        assert!(is_newer("0.4.8-1", "0.4.8"));
        assert!(!is_newer("0.4.8.1", "0.4.8-1"));
        assert!(is_newer("0.4.9", "0.4.8-1"));
    }
}
