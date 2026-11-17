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
use wry::WebViewBuilder;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let root = match app_root() {
        Some(root) => root,
        None => {
            log_line("No se encontró la carpeta de la aplicación (index.html).");
            return;
        }
    };

    let preferred_port = std::env::var("PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(8787);

    if args
        .iter()
        .any(|arg| arg == "--server-only" || arg == "--server")
    {
        if let Err(error) = cupcraft_server::serve_forever(root, preferred_port) {
            log_line(&format!("No se pudo arrancar el servidor: {error}"));
        }
        return;
    }

    let server = match cupcraft_server::start(root.clone(), preferred_port, 20) {
        Ok(server) => server,
        Err(error) => {
            log_line(&format!("No se pudo arrancar cupcraft: {error}"));
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
            log_line(&format!("No se pudo crear la ventana: {error}"));
            return;
        }
    };

    let webview = match WebViewBuilder::new()
        .with_url(&url)
        .with_background_color((13, 13, 15, 255))
        .with_autoplay(true)
        .with_devtools(cfg!(debug_assertions))
        .build(&window)
    {
        Ok(webview) => webview,
        Err(error) => {
            log_line(&format!(
                "No se pudo crear WebView2: {error}. Comprueba que el runtime de WebView2 está instalado."
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
        // Mantener vivos la ventana, el WebView y el servidor mientras dure el
        // bucle de eventos.
        let _ = (&webview, &server);
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

fn log_line(message: &str) {
    if cfg!(debug_assertions) {
        println!("{message}");
    }
    let Ok(local) = std::env::var("LOCALAPPDATA") else {
        return;
    };
    let dir = Path::new(&local).join("openCUP");
    let _ = std::fs::create_dir_all(&dir);
    let line = format!("[{}] {}\r\n", timestamp(), message);
    use std::io::Write;
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("openCUP.log"))
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
    format!("{}", seconds)
}
