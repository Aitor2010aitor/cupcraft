//! cupcraft — escritorio.
//!
//! Un único ejecutable Rust que arranca el servidor local y abre la interfaz
//! web existente en una ventana WebView2. No usa Electron, Node, C++ ni .NET.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::{Path, PathBuf};
use tao::dpi::LogicalSize;
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoop};
use tao::window::{Theme, WindowBuilder};
use wry::{WebContext, WebViewBuilder};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let root = match app_root() {
        Some(root) => root,
        None => {
            fail(
                "No se encontró la carpeta de la aplicación.\n\n\
                 Descomprime cupcraft completo (cupcraft-desktop.exe, index.html, src y assets) \
                 y ejecuta cupcraft-desktop.exe desde esa misma carpeta.\n\n\
                 Si acabas de copiar solo el .exe, vuelve a copiar la carpeta entera.",
            );
            return;
        }
    };

    let preferred_port = std::env::var("CUPCRAFT_PORT")
        .or_else(|_| std::env::var("PORT"))
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(8787);

    if args
        .iter()
        .any(|arg| arg == "--server-only" || arg == "--server")
    {
        if let Err(error) = cupcraft_server::serve_forever(root, preferred_port) {
            fail(&format!("No se pudo arrancar el servidor de cupcraft: {error}"));
        }
        return;
    }

    // Una sola ventana por usuario: si ya hay una abierta, se trae al frente en
    // lugar de arrancar otro servidor o mostrar un aviso sin contexto.
    if !acquire_single_instance() {
        log_line("cupcraft ya está abierto: se intenta enfocar la ventana existente.");
        if !focus_existing_window() {
            fail(
                "cupcraft ya está abierto, pero su ventana no está visible.\n\n\
                 Ciérralo desde el Administrador de tareas (cupcraft-desktop.exe) y vuelve a abrirlo.",
            );
        }
        return;
    }

    let data_dir = webview_data_dir();
    log_line(&format!("Datos de WebView2: {}", data_dir.to_string_lossy()));

    let server = match cupcraft_server::start(root.clone(), preferred_port, 20) {
        Ok(server) => server,
        Err(error) => {
            fail(&format!(
                "No se pudo arrancar cupcraft.\n\n{error}\n\n\
                 Comprueba que la carpeta de la aplicación no está bloqueada y vuelve a intentarlo."
            ));
            return;
        }
    };
    let url = format!("http://127.0.0.1:{}", server.port);
    log_line(&format!(
        "cupcraft {} — app {} — puente IA http://127.0.0.1:{}",
        env!("CARGO_PKG_VERSION"),
        url,
        server.bridge.port()
    ));

    let event_loop = EventLoop::new();
    let window = match WindowBuilder::new()
        .with_title(&format!("cupcraft v{}", env!("CARGO_PKG_VERSION")))
        .with_inner_size(LogicalSize::new(1440.0, 900.0))
        .with_min_inner_size(LogicalSize::new(1120.0, 720.0))
        .with_theme(Some(Theme::Dark))
        .build(&event_loop)
    {
        Ok(window) => window,
        Err(error) => {
            fail(&format!("No se pudo crear la ventana de cupcraft.\n\n{error}"));
            return;
        }
    };

    let mut context = WebContext::new(Some(data_dir));
    let webview = match WebViewBuilder::new_with_web_context(&mut context)
        .with_url(&url)
        .with_background_color((13, 13, 15, 255))
        .with_autoplay(true)
        .with_devtools(cfg!(debug_assertions))
        .build(&window)
    {
        Ok(webview) => webview,
        Err(error) => {
            fail(&format!(
                "No se pudo abrir WebView2.\n\n{error}\n\n\
                 Instala Microsoft Edge WebView2 Runtime desde https://go.microsoft.com/fwlink/p/?LinkId=2124703 \
                 y vuelve a abrir cupcraft."
            ));
            return;
        }
    };

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        if let Event::WindowEvent { event, .. } = event {
            if let WindowEvent::CloseRequested = event {
                *control_flow = ControlFlow::Exit;
            }
        }
        // Mantener vivos la ventana, el WebView, el contexto y el servidor
        // mientras dure el bucle de eventos.
        let _ = (&webview, &server, &context);
    });
}

/// Busca la carpeta que contiene `index.html` y `src/`.
fn app_root() -> Option<PathBuf> {
    if let Ok(custom) = std::env::var("CUPCRAFT_ROOT") {
        let path = PathBuf::from(custom);
        if path.join("index.html").is_file() {
            return Some(path);
        }
    }
    if let Ok(custom) = std::env::var("openCUP_ROOT") {
        let path = PathBuf::from(custom);
        if path.join("index.html").is_file() {
            return Some(path);
        }
    }

    if let Ok(exe) = std::env::current_exe() {
        let mut dir = exe.parent().map(Path::to_path_buf);
        while let Some(candidate) = dir {
            if candidate.join("index.html").is_file() {
                return Some(candidate);
            }
            dir = candidate.parent().map(Path::to_path_buf);
        }
    }

    // En desarrollo: rust/crates/cupcraft-desktop -> raíz del repositorio.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace = manifest.parent()?.parent()?.parent()?;
    if workspace.join("index.html").is_file() {
        return Some(workspace.to_path_buf());
    }

    let current = std::env::current_dir().ok()?;
    if current.join("index.html").is_file() {
        return Some(current);
    }
    None
}

/// Carpeta estable para el perfil de WebView2. Así funciona aunque la
/// aplicación se instale en Program Files o se mueva de carpeta.
fn webview_data_dir() -> PathBuf {
    let base = app_data_dir().join("WebView2");
    if !base.is_dir() {
        // Migración única desde la versión anterior: perfil junto al .exe o el
        // perfil del antiguo lanzador C++ en %LOCALAPPDATA%\openCUP.
        if let Some(old) = adjacent_profile() {
            if old.is_dir() {
                log_line(&format!(
                    "Migrando proyectos desde {} a {}",
                    old.to_string_lossy(),
                    base.to_string_lossy()
                ));
                if copy_tree(&old, &base).is_ok() {
                    log_line("Migración completada.");
                } else {
                    log_line("No se pudo migrar el perfil anterior; se empieza uno nuevo.");
                }
            }
        }
        if !base.is_dir() {
            if let Some(local) = std::env::var_os("LOCALAPPDATA") {
                let old = PathBuf::from(local).join("openCUP").join("WebView2");
                if old.is_dir() {
                    log_line(&format!(
                        "Migrando proyectos antiguos desde {}",
                        old.to_string_lossy()
                    ));
                    let _ = copy_tree(&old, &base);
                }
            }
        }
    }
    let _ = std::fs::create_dir_all(&base);
    base
}

fn app_data_dir() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("cupcraft")
}

fn adjacent_profile() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?.to_path_buf();
    let name = exe.file_name()?.to_string_lossy().to_string();
    let profile = dir.join(format!("{name}.WebView2"));
    profile.is_dir().then_some(profile)
}

fn copy_tree(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let destination = to.join(entry.file_name());
        if file_type.is_dir() {
            copy_tree(&entry.path(), &destination)?;
        } else if file_type.is_file() {
            std::fs::copy(entry.path(), &destination)?;
        }
    }
    Ok(())
}

/// Crea el mutex de instancia única. Devuelve false si ya había otra copia.
fn acquire_single_instance() -> bool {
    #[cfg(windows)]
    {
        use std::ptr::null;
        use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS};
        use windows_sys::Win32::System::Threading::CreateMutexW;

        let name: Vec<u16> = "cupcraft.Desktop.SingleInstance\0".encode_utf16().collect();
        unsafe {
            let handle = CreateMutexW(null(), 1, name.as_ptr());
            if handle.is_null() {
                return true; // sin mutex no se bloquea el arranque
            }
            if GetLastError() == ERROR_ALREADY_EXISTS {
                CloseHandle(handle);
                return false;
            }
            // El handle se deja vivo hasta que termine el proceso.
        }
    }
    true
}

/// Trae al frente la ventana de la instancia que ya está ejecutándose.
fn focus_existing_window() -> bool {
    #[cfg(windows)]
    {
        use std::ptr::null;
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            FindWindowW, IsIconic, SetForegroundWindow, ShowWindow, SW_RESTORE,
        };

        let title: Vec<u16> = format!("cupcraft v{}\0", env!("CARGO_PKG_VERSION"))
            .encode_utf16()
            .collect();
        let start = std::time::Instant::now();
        unsafe {
            loop {
                let hwnd = FindWindowW(null(), title.as_ptr());
                if !hwnd.is_null() {
                    if IsIconic(hwnd) != 0 {
                        ShowWindow(hwnd, SW_RESTORE);
                    }
                    SetForegroundWindow(hwnd);
                    return true;
                }
                // La otra instancia puede estar aún creando la ventana.
                if start.elapsed() > std::time::Duration::from_secs(6) {
                    return false;
                }
                std::thread::sleep(std::time::Duration::from_millis(120));
            }
        }
    }
    #[allow(unreachable_code)]
    false
}

/// Muestra un error visible: en una app de ventana no hay consola donde verlo.
fn fail(message: &str) {
    log_line(message);
    #[cfg(windows)]
    unsafe {
        use std::ptr::null_mut;
        use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};

        let text: Vec<u16> = format!("{message}\0").encode_utf16().collect();
        let caption: Vec<u16> = "cupcraft\0".encode_utf16().collect();
        MessageBoxW(null_mut(), text.as_ptr(), caption.as_ptr(), MB_OK | MB_ICONERROR);
    }
}

fn log_line(message: &str) {
    if cfg!(debug_assertions) {
        println!("{message}");
    }
    let dir = app_data_dir();
    let _ = std::fs::create_dir_all(&dir);
    let line = format!("[{}] {}\r\n", timestamp(), message);
    use std::io::Write;
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("cupcraft.log"))
    {
        let _ = file.write_all(line.as_bytes());
    }
}

fn timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    format!("{seconds}")
}
