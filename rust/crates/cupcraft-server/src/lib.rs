//! Servidor local de cupcraft en Rust.
//!
//! Sustituye a `server.mjs` sin cambiar la interfaz web: sirve `index.html`,
//! `src/` y `assets/`, la API de importación de CapCut y el puente de IA.
//! Escucha únicamente en 127.0.0.1.

pub mod ai_bridge;
pub mod capcut;
pub mod tts;

use ai_bridge::BridgeHandle;
use capcut::AllowedMedia;
use percent_encoding::percent_decode_str;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};
use url::Url;

pub struct ServerHandle {
    pub port: u16,
    pub root: PathBuf,
    pub bridge: Arc<BridgeHandle>,
    server: Arc<Server>,
}

impl Drop for ServerHandle {
    fn drop(&mut self) {
        self.server.unblock();
    }
}

struct State {
    root: PathBuf,
    allowed: AllowedMedia,
    bridge: Arc<BridgeHandle>,
    app_port: u16,
}

fn header(name: &str, value: &str) -> Header {
    Header::from_bytes(name.as_bytes(), value.as_bytes()).expect("cabecera válida")
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn respond_json(request: Request, status: u16, body: &Value) {
    let text = serde_json::to_string(body).unwrap_or_else(|_| "{}".to_string());
    let response = Response::from_string(text)
        .with_status_code(StatusCode(status))
        .with_header(header("Content-Type", "application/json; charset=utf-8"))
        .with_header(header("Cache-Control", "no-store"));
    let _ = request.respond(response);
}

fn respond_bytes(request: Request, status: u16, content_type: &str, cache: &str, bytes: Vec<u8>) {
    let length = bytes.len();
    let response = Response::new(
        StatusCode(status),
        vec![
            header("Content-Type", content_type),
            header("Cache-Control", cache),
            header("Content-Length", &length.to_string()),
        ],
        std::io::Cursor::new(bytes),
        Some(length),
        None,
    );
    let _ = request.respond(response);
}

fn respond_text(request: Request, status: u16, text: &str) {
    let response = Response::from_string(text.to_string())
        .with_status_code(StatusCode(status))
        .with_header(header("Content-Type", "text/plain; charset=utf-8"))
        .with_header(header("Cache-Control", "no-store"));
    let _ = request.respond(response);
}

fn static_mime(extension: &str) -> &'static str {
    match extension {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" => "application/json; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        "mp4" => "video/mp4",
        "mov" => "video/quicktime",
        "webm" => "video/webm",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "m4a" => "audio/mp4",
        "ogg" => "audio/ogg",
        _ => "application/octet-stream",
    }
}

fn parse_query(url: &Url) -> HashMap<String, String> {
    url.query_pairs().into_owned().collect()
}

fn read_json_body(request: &mut Request, limit: usize) -> Result<Value, String> {
    let mut text = String::new();
    request
        .as_reader()
        .take(limit as u64)
        .read_to_string(&mut text)
        .map_err(|_| "No se pudo leer el cuerpo".to_string())?;
    if text.trim().is_empty() {
        return Ok(json!({}));
    }
    serde_json::from_str(&text).map_err(|_| "JSON no válido".to_string())
}

fn host_allowed(host: &str) -> bool {
    let host = host.to_ascii_lowercase();
    host == "ibyteimg.com" || host.ends_with(".ibyteimg.com")
}

fn handle_image_proxy(request: Request, query: &HashMap<String, String>) {
    let Some(target) = query.get("u") else {
        respond_text(request, 400, "url no válida");
        return;
    };
    let Ok(remote) = Url::parse(target) else {
        respond_text(request, 400, "url no válida");
        return;
    };
    let allowed_scheme = matches!(remote.scheme(), "http" | "https");
    let allowed_host = remote.host_str().map(host_allowed).unwrap_or(false);
    if !allowed_scheme || !allowed_host {
        respond_text(request, 403, "host no permitido");
        return;
    }
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(20))
        .build();
    match agent.get(remote.as_str()).call() {
        Ok(response) => {
            let content_type = response
                .header("content-type")
                .unwrap_or("image/webp")
                .to_string();
            let mut bytes = Vec::new();
            if response
                .into_reader()
                .take(20 * 1024 * 1024)
                .read_to_end(&mut bytes)
                .is_err()
            {
                respond_text(request, 502, "no se pudo descargar");
                return;
            }
            respond_bytes(request, 200, &content_type, "public, max-age=86400", bytes);
        }
        Err(_) => respond_text(request, 502, "no se pudo descargar"),
    }
}

fn parse_range(value: &str, size: u64) -> Option<(u64, u64)> {
    let value = value.trim();
    let rest = value.strip_prefix("bytes=")?;
    let (start_text, end_text) = rest.split_once('-')?;
    let start = if start_text.is_empty() {
        0
    } else {
        start_text.parse::<u64>().ok()?
    };
    let end = if end_text.is_empty() {
        size.saturating_sub(1)
    } else {
        end_text.parse::<u64>().ok()?
    };
    if size == 0 {
        return None;
    }
    let start = start.min(size - 1);
    let end = end.max(start).min(size - 1);
    Some((start, end))
}

fn serve_local_media(request: Request, path: &Path) {
    let metadata = match std::fs::metadata(path) {
        Ok(metadata) if metadata.is_file() => metadata,
        _ => {
            respond_text(request, 404, "Not found");
            return;
        }
    };
    let size = metadata.len();
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let mime = static_mime(&extension);
    let range = request
        .headers()
        .iter()
        .find(|h| h.field.equiv("Range"))
        .map(|h| h.value.as_str().to_string());

    if let Some(range) = range {
        if let Some((start, end)) = parse_range(&range, size) {
            let length = end - start + 1;
            let mut file = match std::fs::File::open(path) {
                Ok(file) => file,
                Err(_) => {
                    respond_text(request, 404, "Not found");
                    return;
                }
            };
            if file.seek(SeekFrom::Start(start)).is_err() {
                respond_text(request, 500, "Internal error");
                return;
            }
            let response = Response::new(
                StatusCode(206),
                vec![
                    header("Content-Type", mime),
                    header(
                        "Content-Range",
                        &format!("bytes {}-{}/{}", start, end, size),
                    ),
                    header("Accept-Ranges", "bytes"),
                    header("Content-Length", &length.to_string()),
                    header("Cache-Control", "no-store"),
                ],
                file.take(length),
                Some(length as usize),
                None,
            );
            let _ = request.respond(response);
            return;
        }
    }

    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(_) => {
            respond_text(request, 404, "Not found");
            return;
        }
    };
    let response = Response::new(
        StatusCode(200),
        vec![
            header("Content-Type", mime),
            header("Content-Length", &size.to_string()),
            header("Accept-Ranges", "bytes"),
            header("Cache-Control", "no-store"),
        ],
        file,
        Some(size as usize),
        None,
    );
    let _ = request.respond(response);
}

fn handle_api(mut request: Request, state: &State, url: &Url) {
    let path = url.path();
    let query = parse_query(url);
    // Se clona el método para que el `match` no retenga un préstamo de
    // `request` y cada rama pueda responder consumiéndolo.
    let method = request.method().clone();
    match (method, path) {
        (Method::Get, "/api/tts/voices") => {
            let voices = tts::list_voices(&state.root);
            respond_json(request, 200, &voices);
        }
        (Method::Post, "/api/tts/generate") => {
            let body = match read_json_body(&mut request, 1_000_000) {
                Ok(body) => body,
                Err(error) => {
                    respond_json(request, 400, &json!({ "ok": false, "error": error }));
                    return;
                }
            };
            let text = body
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim();
            if text.is_empty() {
                respond_json(
                    request,
                    400,
                    &json!({ "ok": false, "error": "El texto no puede estar vacío" }),
                );
                return;
            }
            let voice = body
                .get("voice")
                .and_then(|v| v.as_str())
                .unwrap_or("es_ES-davefx-medium");
            let speed = body
                .get("speed")
                .and_then(|v| v.as_f64())
                .map(|v| v as f32)
                .unwrap_or(1.0);

            match tts::generate(&state.root, text, voice, speed) {
                Ok(bytes) => {
                    respond_bytes(request, 200, "audio/wav", "no-store", bytes);
                }
                Err(err) => {
                    respond_json(request, 500, &json!({ "ok": false, "error": err }));
                }
            }
        }
        (Method::Get, "/api/img") => {
            handle_image_proxy(request, &query);
        }
        (Method::Get, "/api/capcut/roots") => {
            respond_json(
                request,
                200,
                &json!({ "ok": true, "roots": capcut::roots() }),
            );
        }
        (Method::Get, "/api/capcut/projects") => {
            let projects = capcut::scan_projects(&state.allowed);
            respond_json(
                request,
                200,
                &json!({ "ok": true, "projects": projects, "roots": capcut::roots() }),
            );
        }
        (Method::Post, "/api/capcut/import") => {
            let body = match read_json_body(&mut request, 4_000_000) {
                Ok(body) => body,
                Err(error) => {
                    respond_json(request, 400, &json!({ "ok": false, "error": error }));
                    return;
                }
            };
            let id = body
                .get("id")
                .and_then(|value| value.as_str())
                .unwrap_or("");
            if id.is_empty() {
                respond_json(
                    request,
                    400,
                    &json!({ "ok": false, "error": "Falta el id del proyecto de CapCut" }),
                );
                return;
            }
            match capcut::read_draft(id, &state.allowed) {
                Ok(project) => {
                    let warnings = project
                        .get("warnings")
                        .and_then(|value| value.as_array())
                        .cloned()
                        .unwrap_or_default();
                    let media_count = project
                        .get("media")
                        .and_then(|value| value.as_array())
                        .map(|items| items.len())
                        .unwrap_or(0);
                    respond_json(
                        request,
                        200,
                        &json!({
                            "ok": true,
                            "project": project,
                            "warnings": warnings,
                            "mediaCount": media_count,
                        }),
                    );
                }
                Err(error) => respond_json(request, 400, &json!({ "ok": false, "error": error })),
            }
        }
        (Method::Get, "/api/capcut/media") => {
            let decoded = query
                .get("path")
                .and_then(|value| capcut::decode_path(value))
                .unwrap_or_default();
            if decoded.is_empty() {
                respond_text(request, 400, "Bad request");
                return;
            }
            let target = PathBuf::from(decoded);
            if !capcut::is_allowed_media(&state.allowed, &target) {
                respond_text(request, 403, "Forbidden");
                return;
            }
            serve_local_media(request, &target);
        }
        (Method::Get, "/api/capcut/cover") => {
            let decoded = query
                .get("path")
                .and_then(|value| capcut::decode_path(value))
                .unwrap_or_default();
            if decoded.is_empty() {
                respond_text(request, 400, "Bad request");
                return;
            }
            let target = PathBuf::from(decoded);
            if !target.is_file() {
                respond_text(request, 404, "Not found");
                return;
            }
            capcut::allow_media(&state.allowed, &target);
            serve_local_media(request, &target);
        }
        (Method::Get, "/api/ai/status") => {
            let mut value = state.bridge.status(true);
            if let Some(object) = value.as_object_mut() {
                object.insert(
                    "app".to_string(),
                    json!(format!("http://127.0.0.1:{}", state.app_port)),
                );
                object.insert("roots".to_string(), json!(capcut::roots()));
            }
            respond_json(request, 200, &value);
        }
        (Method::Post, "/api/ai/poll") => {
            let body = match read_json_body(&mut request, 4_000_000) {
                Ok(body) => body,
                Err(error) => {
                    respond_json(request, 400, &json!({ "ok": false, "error": error }));
                    return;
                }
            };
            let client_id = body
                .get("clientId")
                .and_then(|value| value.as_str())
                .unwrap_or("")
                .chars()
                .take(120)
                .collect::<String>();
            if client_id.is_empty() {
                respond_json(
                    request,
                    400,
                    &json!({ "ok": false, "error": "Falta clientId" }),
                );
                return;
            }
            if let Some(value) = body.get("state") {
                let started_at = body
                    .get("startedAt")
                    .and_then(|value| value.as_i64())
                    .unwrap_or(0);
                state.bridge.state(&client_id, value.clone(), started_at);
            }
            let command = state.bridge.take(&client_id);
            respond_json(
                request,
                200,
                &json!({
                    "ok": true,
                    "connected": true,
                    "command": command,
                    "serverTime": now_ms(),
                }),
            );
        }
        (Method::Post, "/api/ai/result") => {
            let body = match read_json_body(&mut request, 4_000_000) {
                Ok(body) => body,
                Err(error) => {
                    respond_json(request, 400, &json!({ "ok": false, "error": error }));
                    return;
                }
            };
            let client_id = body
                .get("clientId")
                .and_then(|value| value.as_str())
                .unwrap_or("");
            let id = body
                .get("id")
                .and_then(|value| value.as_str())
                .unwrap_or("");
            if client_id.is_empty() || id.is_empty() {
                respond_json(
                    request,
                    400,
                    &json!({ "ok": false, "error": "Faltan clientId o id" }),
                );
                return;
            }
            let ok = body
                .get("ok")
                .and_then(|value| value.as_bool())
                .unwrap_or(true);
            let result = body.get("result").cloned().unwrap_or(Value::Null);
            let error = body
                .get("error")
                .and_then(|value| value.as_str())
                .unwrap_or("")
                .to_string();
            let done = state.bridge.result(client_id, id, ok, result, error);
            respond_json(request, 200, &json!({ "ok": done, "id": id }));
        }
        (Method::Post, "/api/ai/state") => {
            let body = match read_json_body(&mut request, 4_000_000) {
                Ok(body) => body,
                Err(error) => {
                    respond_json(request, 400, &json!({ "ok": false, "error": error }));
                    return;
                }
            };
            let client_id = body
                .get("clientId")
                .and_then(|value| value.as_str())
                .unwrap_or("");
            if client_id.is_empty() {
                respond_json(
                    request,
                    400,
                    &json!({ "ok": false, "error": "Falta clientId" }),
                );
                return;
            }
            let started_at = body
                .get("startedAt")
                .and_then(|value| value.as_i64())
                .unwrap_or(0);
            let value = body.get("state").cloned().unwrap_or(Value::Null);
            state.bridge.state(client_id, value, started_at);
            respond_json(request, 200, &json!({ "ok": true, "at": now_ms() }));
        }
        _ => respond_json(
            request,
            404,
            &json!({ "ok": false, "error": "API no encontrada" }),
        ),
    }
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

fn handle_static(request: Request, root: &Path, path: &str) {
    let decoded = percent_decode_str(path).decode_utf8_lossy().to_string();
    let relative = if decoded == "/" {
        "index.html"
    } else {
        decoded.trim_start_matches('/')
    };
    if relative
        .split(|c| c == '/' || c == '\\')
        .any(|part| part == "..")
    {
        respond_text(request, 403, "Forbidden");
        return;
    }
    let candidate = lexical_normalize(&root.join(relative.replace('/', "\\")));
    if !candidate.starts_with(root) {
        respond_text(request, 403, "Forbidden");
        return;
    }
    let metadata = match std::fs::metadata(&candidate) {
        Ok(metadata) if metadata.is_file() => metadata,
        _ => {
            respond_text(request, 404, "Not found");
            return;
        }
    };
    let extension = candidate
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let mime = static_mime(&extension);
    let file = match std::fs::File::open(&candidate) {
        Ok(file) => file,
        Err(_) => {
            respond_text(request, 500, "Internal error");
            return;
        }
    };
    let size = metadata.len();
    let response = Response::new(
        StatusCode(200),
        vec![
            header("Content-Type", mime),
            header("Content-Length", &size.to_string()),
            header("Cache-Control", "no-store"),
        ],
        file,
        Some(size as usize),
        None,
    );
    let _ = request.respond(response);
}

fn handle_request(request: Request, state: Arc<State>) {
    let parsed = match Url::parse(&format!("http://127.0.0.1{}", request.url())) {
        Ok(url) => url,
        Err(_) => {
            respond_text(request, 400, "Solicitud no válida");
            return;
        }
    };
    if parsed.path().starts_with("/api/") {
        handle_api(request, &state, &parsed);
        return;
    }
    handle_static(request, &state.root, parsed.path());
}

/// Arranca el servidor principal y el puente IA.
pub fn start(root: PathBuf, preferred_port: u16, attempts: u16) -> Result<ServerHandle, String> {
    let absolute = std::path::absolute(&root).unwrap_or(root);
    let root = lexical_normalize(&absolute);
    if !root.join("index.html").is_file() {
        return Err(format!(
            "No se encuentra index.html en {}",
            root.to_string_lossy()
        ));
    }
    let allowed: AllowedMedia = Arc::new(std::sync::Mutex::new(std::collections::HashSet::new()));
    let bridge_port = std::env::var("openCUP_AI_PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(8790);
    let bridge = Arc::new(ai_bridge::start(&root, bridge_port, 20)?);

    let mut last_error = String::new();
    let mut selected: Option<(Server, u16)> = None;
    for attempt in 0..=attempts {
        let port = preferred_port.saturating_add(attempt);
        match Server::http(("127.0.0.1", port)) {
            Ok(server) => {
                selected = Some((server, port));
                break;
            }
            Err(error) => last_error = format!("{}", error),
        }
    }
    let Some((server, port)) = selected else {
        return Err(format!(
            "No se pudo abrir cupcraft en 127.0.0.1:{} ({})",
            preferred_port, last_error
        ));
    };

    let server = Arc::new(server);
    let state = Arc::new(State {
        root: root.clone(),
        allowed,
        bridge: Arc::clone(&bridge),
        app_port: port,
    });
    let worker = Arc::clone(&server);
    std::thread::spawn(move || {
        for request in worker.incoming_requests() {
            let state = Arc::clone(&state);
            std::thread::spawn(move || handle_request(request, state));
        }
    });

    Ok(ServerHandle {
        port,
        root,
        bridge,
        server,
    })
}

/// Arranque mínimo para usar el servidor sin ventana (tests o `--server`).
pub fn serve_forever(root: PathBuf, port: u16) -> Result<(), String> {
    let handle = start(root, port, 20)?;
    println!("cupcraft listo en http://127.0.0.1:{}", handle.port);
    println!(
        "Puente IA en http://127.0.0.1:{} (token: {})",
        handle.bridge.port(),
        handle.bridge.token()
    );
    std::thread::park();
    drop(handle);
    Ok(())
}
