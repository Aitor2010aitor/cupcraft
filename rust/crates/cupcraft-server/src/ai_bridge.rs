//! Puente de control para IA.
//!
//! Equivalente Rust de `server/ai-bridge.mjs`. Escucha solo en 127.0.0.1 y
//! exige token. Mantiene el estado que publica el navegador, la cola de
//! comandos y los resultados.

use rand::Rng;
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::io::Read;
use std::path::Path;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};
use url::Url;

const ACTION_NAMES: &[&str] = &[
    "getState",
    "listProjects",
    "newProject",
    "openProject",
    "goHome",
    "save",
    "listCapCutProjects",
    "openCapCutProject",
    "setPanel",
    "seek",
    "play",
    "pause",
    "togglePlay",
    "selectClip",
    "addText",
    "addSticker",
    "applyFilter",
    "applyEffect",
    "applyTransition",
    "updateClip",
    "split",
    "deleteClip",
    "duplicateClip",
    "undo",
    "redo",
    "setZoom",
    "listFilters",
    "listStickers",
    "listEffects",
    "listTransitions",
    "screenshot",
];

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn base36(mut value: u64) -> String {
    if value == 0 {
        return "0".to_string();
    }
    const ALPHABET: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut out = Vec::new();
    while value > 0 {
        out.push(ALPHABET[(value % 36) as usize]);
        value /= 36;
    }
    out.reverse();
    String::from_utf8(out).unwrap_or_else(|_| "0".to_string())
}

fn header(name: &str, value: &str) -> Header {
    Header::from_bytes(name.as_bytes(), value.as_bytes()).expect("cabecera válida")
}

fn cors_headers() -> Vec<Header> {
    vec![
        header("Access-Control-Allow-Origin", "*"),
        header(
            "Access-Control-Allow-Headers",
            "Content-Type, Authorization, X-openCUP-Token",
        ),
        header("Access-Control-Allow-Methods", "GET, POST, OPTIONS"),
    ]
}

fn respond_text(request: Request, status: u16, body: String) {
    let mut response = Response::from_string(body).with_status_code(StatusCode(status));
    for h in cors_headers() {
        response = response.with_header(h);
    }
    response = response.with_header(header("Content-Type", "text/plain; charset=utf-8"));
    response = response.with_header(header("Cache-Control", "no-store"));
    let _ = request.respond(response);
}

fn respond_json(request: Request, status: u16, body: &Value) {
    let text = serde_json::to_string_pretty(body).unwrap_or_else(|_| "{}".to_string());
    let mut response = Response::from_string(text).with_status_code(StatusCode(status));
    for h in cors_headers() {
        response = response.with_header(h);
    }
    response = response.with_header(header("Content-Type", "application/json; charset=utf-8"));
    response = response.with_header(header("Cache-Control", "no-store"));
    let _ = request.respond(response);
}

fn welcome_page(port: u16) -> String {
    format!(
        r#"<!doctype html><html lang="es"><meta charset="utf-8"><title>cupcraft AI Bridge</title>
<style>body{{background:#0c1112;color:#e8f4f4;font:14px/1.6 system-ui,sans-serif;padding:40px;max-width:860px;margin:auto}}code{{background:#132b2e;padding:2px 6px;border-radius:4px;color:#7deff5}}pre{{background:#101a1c;border:1px solid #1f3a3d;border-radius:8px;padding:16px;overflow:auto}}a{{color:#57e5ef}}h1{{font-size:24px}}</style>
<h1>cupcraft AI Bridge</h1>
<p>Puerto <code>{port}</code>. Requiere el token que aparece en cupcraft → Menú → Puente IA.</p>
<pre>curl -H "Authorization: Bearer $openCUP_AI_TOKEN" http://127.0.0.1:{port}/api/ai/status
curl -H "Authorization: Bearer $openCUP_AI_TOKEN" http://127.0.0.1:{port}/api/ai/state
curl -X POST -H "Authorization: Bearer $openCUP_AI_TOKEN" -H "Content-Type: application/json" \
  -d '{{"action":"addText","params":{{"content":"Hola desde una IA","start":0,"duration":3}},"wait":true}}' \
  http://127.0.0.1:{port}/api/ai/command</pre>
<p><a href="/api/ai/actions">Ver todas las acciones</a></p>
<p>Este servidor escucha únicamente en 127.0.0.1. No se expone a Internet.</p>"#
    )
}

pub fn actions() -> Value {
    json!([
        { "action": "getState", "description": "Devuelve el estado completo: proyecto, clips, medios, selección, cursor y paneles." },
        { "action": "listProjects", "description": "Lista los proyectos guardados en cupcraft." },
        { "action": "newProject", "params": { "name": "string" }, "description": "Crea un proyecto nuevo y abre el editor." },
        { "action": "openProject", "params": { "id": "string", "name": "string (alternativa)" }, "description": "Abre un proyecto guardado por id o nombre." },
        { "action": "goHome", "description": "Vuelve a la pantalla de inicio." },
        { "action": "save", "description": "Guarda el proyecto actual." },
        { "action": "listCapCutProjects", "description": "Lista los borradores nativos de CapCut detectados en este equipo." },
        { "action": "openCapCutProject", "params": { "id": "string del listado" }, "description": "Importa y abre un proyecto nativo de CapCut." },
        { "action": "setPanel", "params": { "panel": "media|audio|text|stickers|effects|transitions|captions|filters|adjust|templates|avatar" }, "description": "Cambia el panel de herramientas." },
        { "action": "seek", "params": { "time": "segundos" }, "description": "Mueve el cursor de reproducción." },
        { "action": "play", "description": "Reproduce la línea de tiempo." },
        { "action": "pause", "description": "Pausa la reproducción." },
        { "action": "togglePlay", "description": "Alterna reproducción y pausa." },
        { "action": "selectClip", "params": { "id": "string|null" }, "description": "Selecciona un clip por id." },
        { "action": "addText", "params": { "content": "string", "start": "segundos", "duration": "segundos", "size": "número", "color": "#rrggbb", "weight": "100-900" }, "description": "Añade un texto o subtítulo." },
        { "action": "addSticker", "params": { "value": "emoji", "start": "segundos", "duration": "segundos", "size": "número", "scale": "número", "background": "#rrggbb", "shape": "none|circle|square" }, "description": "Añade un sticker como capa independiente." },
        { "action": "applyFilter", "params": { "clip": "id (opcional)", "filter": "id de listFilters" }, "description": "Aplica un filtro de color del catálogo local (731 filtros en 19 categorías)." },
        { "action": "applyEffect", "params": { "clip": "id (opcional)", "effect": "id de listEffects" }, "description": "Aplica un efecto visual del catálogo local." },
        { "action": "applyTransition", "params": { "clip": "id (opcional)", "transition": "id de listTransitions", "duration": "segundos" }, "description": "Aplica una transición de entrada al clip (16 tipos)." },
        { "action": "updateClip", "params": { "id": "string", "patch": "objeto" }, "description": "Modifica propiedades de un clip (volumen, escala, posición, rotación, velocidad, texto o sticker)." },
        { "action": "split", "description": "Divide en el cursor." },
        { "action": "deleteClip", "description": "Elimina el clip seleccionado." },
        { "action": "duplicateClip", "description": "Duplica el clip seleccionado." },
        { "action": "undo", "description": "Deshace el último cambio." },
        { "action": "redo", "description": "Rehace el último cambio." },
        { "action": "setZoom", "params": { "value": "número 30-260" }, "description": "Cambia el zoom de la línea de tiempo." },
        { "action": "listFilters", "description": "Lista los 731 filtros disponibles con su categoría." },
        { "action": "listStickers", "description": "Lista los 219 stickers disponibles por categoría." },
        { "action": "listEffects", "description": "Lista los 168 efectos disponibles con su categoría." },
        { "action": "listTransitions", "description": "Lista las transiciones disponibles por categoría." },
        { "action": "screenshot", "description": "Captura un fotograma del reproductor y devuelve un data URL." }
    ])
}

#[derive(Clone)]
struct Client {
    id: String,
    first_seen: i64,
    last_seen: i64,
    state: Value,
    state_at: i64,
}

#[derive(Clone)]
struct Command {
    id: String,
    action: String,
    params: Value,
    status: String,
    created_at: i64,
    client: Option<String>,
    taken_at: Option<i64>,
    finished_at: Option<i64>,
    result: Option<Value>,
    error: String,
}

struct Bridge {
    clients: HashMap<String, Client>,
    commands: HashMap<String, Command>,
    pending: Vec<String>,
    results: HashMap<String, Value>,
    sequence: u64,
}

impl Bridge {
    fn purge(&mut self, now: i64) {
        let cutoff = now - 20_000;
        self.clients.retain(|_, client| client.last_seen >= cutoff);
    }

    fn live_count(&self, now: i64) -> usize {
        let cutoff = now - 5_000;
        self.clients
            .values()
            .filter(|client| client.last_seen >= cutoff)
            .count()
    }

    fn newest(&self, now: i64) -> Option<String> {
        let cutoff = now - 5_000;
        let mut live: Vec<&Client> = self
            .clients
            .values()
            .filter(|client| client.last_seen >= cutoff)
            .collect();
        live.sort_by(|a, b| {
            b.first_seen
                .cmp(&a.first_seen)
                .then_with(|| b.last_seen.cmp(&a.last_seen))
        });
        live.first().map(|client| client.id.clone())
    }

    fn touch_client(&mut self, id: &str, now: i64) {
        let client = self
            .clients
            .entry(id.to_string())
            .or_insert_with(|| Client {
                id: id.to_string(),
                first_seen: now,
                last_seen: now,
                state: Value::Null,
                state_at: 0,
            });
        client.last_seen = now;
    }

    fn set_state(&mut self, id: &str, value: Value, started_at: i64) -> bool {
        let now = now_ms();
        let first_seen = if started_at > 0 { started_at } else { now };
        let client = self
            .clients
            .entry(id.to_string())
            .or_insert_with(|| Client {
                id: id.to_string(),
                first_seen,
                last_seen: now,
                state: Value::Null,
                state_at: 0,
            });
        client.last_seen = now;
        client.state = value;
        client.state_at = now;
        true
    }

    fn trim_commands(&mut self) {
        if self.commands.len() < 300 {
            return;
        }
        let mut entries: Vec<(String, i64)> = self
            .commands
            .values()
            .map(|command| (command.id.clone(), command.created_at))
            .collect();
        entries.sort_by_key(|(_, created)| *created);
        for (id, _) in entries.into_iter().take(100) {
            self.commands.remove(&id);
            self.results.remove(&id);
        }
    }

    fn enqueue(&mut self, action: &str, params: Value) -> (String, i64) {
        let now = now_ms();
        self.sequence += 1;
        let id = format!("cmd_{}_{}", base36(now as u64), base36(self.sequence));
        let command = Command {
            id: id.clone(),
            action: action.to_string(),
            params,
            status: "pending".to_string(),
            created_at: now,
            client: None,
            taken_at: None,
            finished_at: None,
            result: None,
            error: String::new(),
        };
        self.commands.insert(id.clone(), command);
        self.pending.push(id.clone());
        self.trim_commands();
        (id, now)
    }

    fn finish(&mut self, id: &str, ok: bool, result: Option<Value>, error: String) -> bool {
        if !self.commands.contains_key(id) {
            return false;
        }
        let finished_at = now_ms();
        if let Some(command) = self.commands.get_mut(id) {
            command.status = if ok { "done" } else { "error" }.to_string();
            command.finished_at = Some(finished_at);
            command.result = result.clone();
            command.error = error.clone();
        }
        self.results.insert(
            id.to_string(),
            json!({
                "id": id,
                "ok": ok,
                "result": result.unwrap_or(Value::Null),
                "error": error,
                "finishedAt": finished_at,
            }),
        );
        self.pending.retain(|item| item != id);
        true
    }

    fn log(&self) -> Value {
        let mut entries: Vec<&Command> = self.commands.values().collect();
        entries.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Value::Array(
            entries
                .into_iter()
                .take(50)
                .map(|command| {
                    json!({
                        "id": command.id,
                        "action": command.action,
                        "status": command.status,
                        "client": command.client,
                        "createdAt": command.created_at,
                        "finishedAt": command.finished_at,
                        "error": command.error,
                    })
                })
                .collect(),
        )
    }
}

struct Inner {
    bridge: Mutex<Bridge>,
    changed: Condvar,
}

/// Acceso compartido al estado del puente (sin servidor).
struct BridgeApi {
    inner: Arc<Inner>,
    token: String,
    port: u16,
}

impl BridgeApi {
    fn status(&self, include_token: bool) -> Value {
        let now = now_ms();
        let mut bridge = self.inner.bridge.lock().unwrap();
        bridge.purge(now);
        let target = bridge.newest(now);
        let clients = bridge.live_count(now);
        let pending = bridge.pending.len();
        let last_state_at = target
            .as_ref()
            .and_then(|id| bridge.clients.get(id))
            .map(|client| client.state_at)
            .unwrap_or(0);
        let mut object = Map::new();
        object.insert("ok".to_string(), json!(true));
        object.insert("name".to_string(), json!("cupcraft AI Bridge"));
        object.insert("version".to_string(), json!("1.0.0"));
        object.insert("port".to_string(), json!(self.port));
        object.insert("enabled".to_string(), json!(true));
        object.insert("connected".to_string(), json!(clients > 0));
        object.insert("clients".to_string(), json!(clients));
        object.insert(
            "target".to_string(),
            target.map(Value::String).unwrap_or(Value::Null),
        );
        object.insert("pending".to_string(), json!(pending));
        object.insert("lastStateAt".to_string(), json!(last_state_at));
        object.insert(
            "endpoint".to_string(),
            json!(format!("http://127.0.0.1:{}/api/ai", self.port)),
        );
        object.insert(
            "docs".to_string(),
            json!(format!("http://127.0.0.1:{}/api/ai/actions", self.port)),
        );
        if include_token {
            object.insert("token".to_string(), json!(self.token));
        }
        Value::Object(object)
    }

    fn snapshot(&self) -> Value {
        let now = now_ms();
        let mut bridge = self.inner.bridge.lock().unwrap();
        bridge.purge(now);
        let target = bridge.newest(now);
        let clients = bridge.live_count(now);
        let (state, at) = target
            .as_ref()
            .and_then(|id| bridge.clients.get(id))
            .map(|client| (client.state.clone(), client.state_at))
            .unwrap_or((Value::Null, 0));
        json!({
            "ok": true,
            "client": target,
            "at": at,
            "clients": clients,
            "state": state,
        })
    }

    fn log(&self) -> Value {
        let bridge = self.inner.bridge.lock().unwrap();
        json!({ "ok": true, "commands": bridge.log() })
    }

    fn take(&self, client_id: &str) -> Option<Value> {
        let now = now_ms();
        let mut bridge = self.inner.bridge.lock().unwrap();
        bridge.purge(now);
        bridge.touch_client(client_id, now);
        let target = bridge.newest(now);
        if target.as_deref() != Some(client_id) {
            return None;
        }
        let index = bridge.pending.iter().position(|id| {
            bridge
                .commands
                .get(id)
                .map(|command| command.status == "pending")
                .unwrap_or(false)
        })?;
        let id = bridge.pending[index].clone();
        if let Some(command) = bridge.commands.get_mut(&id) {
            command.status = "taken".to_string();
            command.client = Some(client_id.to_string());
            command.taken_at = Some(now);
            return Some(json!({
                "id": command.id,
                "action": command.action,
                "params": command.params,
                "createdAt": command.created_at,
            }));
        }
        None
    }

    fn result(&self, client_id: &str, id: &str, ok: bool, result: Value, error: String) -> bool {
        let mut bridge = self.inner.bridge.lock().unwrap();
        let now = now_ms();
        bridge.touch_client(client_id, now);
        let done = bridge.finish(id, ok, Some(result), error);
        drop(bridge);
        self.inner.changed.notify_all();
        done
    }

    fn state(&self, client_id: &str, value: Value, started_at: i64) -> bool {
        let mut bridge = self.inner.bridge.lock().unwrap();
        bridge.set_state(client_id, value, started_at)
    }

    fn enqueue(&self, action: &str, params: Value) -> (String, i64) {
        let mut bridge = self.inner.bridge.lock().unwrap();
        bridge.enqueue(action, params)
    }

    fn cancel(&self, id: &str) -> Option<bool> {
        let mut bridge = self.inner.bridge.lock().unwrap();
        if !bridge.commands.contains_key(id) {
            return None;
        }
        let done = bridge.finish(id, false, None, "Cancelado".to_string());
        drop(bridge);
        self.inner.changed.notify_all();
        Some(done)
    }

    fn wait_result(&self, id: &str, timeout_ms: u64) -> Value {
        let deadline = Instant::now() + Duration::from_millis(timeout_ms.max(1000));
        let mut guard = self.inner.bridge.lock().unwrap();
        loop {
            if let Some(result) = guard.results.get(id) {
                return result.clone();
            }
            let now = Instant::now();
            if now >= deadline {
                break;
            }
            let (next, _) = self
                .inner
                .changed
                .wait_timeout(guard, deadline - now)
                .unwrap_or_else(|error| error.into_inner());
            guard = next;
        }
        json!({
            "id": id,
            "ok": false,
            "pending": true,
            "error": "Sin respuesta del navegador. Abre cupcraft y activa el puente IA.",
        })
    }
}

pub struct BridgeHandle {
    api: BridgeApi,
    server: Arc<Server>,
}

impl BridgeHandle {
    pub fn port(&self) -> u16 {
        self.api.port
    }

    pub fn token(&self) -> &str {
        &self.api.token
    }

    pub fn status(&self, include_token: bool) -> Value {
        self.api.status(include_token)
    }

    pub fn snapshot(&self) -> Value {
        self.api.snapshot()
    }

    pub fn log(&self) -> Value {
        self.api.log()
    }

    pub fn take(&self, client_id: &str) -> Option<Value> {
        self.api.take(client_id)
    }

    pub fn result(
        &self,
        client_id: &str,
        id: &str,
        ok: bool,
        result: Value,
        error: String,
    ) -> bool {
        self.api.result(client_id, id, ok, result, error)
    }

    pub fn state(&self, client_id: &str, value: Value, started_at: i64) -> bool {
        self.api.state(client_id, value, started_at)
    }

    pub fn enqueue(&self, action: &str, params: Value) -> (String, i64) {
        self.api.enqueue(action, params)
    }

    pub fn cancel(&self, id: &str) -> Option<bool> {
        self.api.cancel(id)
    }

    pub fn wait_result(&self, id: &str, timeout_ms: u64) -> Value {
        self.api.wait_result(id, timeout_ms)
    }
}

impl Drop for BridgeHandle {
    fn drop(&mut self) {
        self.server.unblock();
    }
}

fn authorized(request: &Request, parsed: &Url, token: &str) -> bool {
    for header in request.headers() {
        let value = header.value.as_str();
        if header.field.equiv("Authorization") {
            let lower = value.to_ascii_lowercase();
            if lower.starts_with("bearer ") && value[7..].trim() == token {
                return true;
            }
        }
        if header.field.equiv("X-openCUP-Token") && value == token {
            return true;
        }
    }
    parsed
        .query_pairs()
        .any(|(key, value)| key == "token" && value == token)
}

fn read_body(request: &mut Request, limit: usize) -> Result<Value, String> {
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

fn handle(mut request: Request, inner: Arc<Inner>, token: String, port: u16) {
    let parsed = match Url::parse(&format!("http://127.0.0.1{}", request.url())) {
        Ok(url) => url,
        Err(_) => {
            respond_text(request, 400, "Solicitud no válida".to_string());
            return;
        }
    };
    let path = parsed.path().to_string();
    if request.method() == &Method::Options {
        let mut response = Response::empty(StatusCode(204));
        for h in cors_headers() {
            response = response.with_header(h);
        }
        let _ = request.respond(response);
        return;
    }

    if !authorized(&request, &parsed, &token) {
        respond_text(
            request,
            401,
            "Token requerido. Consulta cupcraft → Menú → Puente IA.".to_string(),
        );
        return;
    }

    let api = BridgeApi {
        inner,
        token: token.clone(),
        port,
    };

    // Se clona el método para que el `match` no retenga un préstamo de
    // `request` y cada rama pueda responder consumiéndolo.
    let method = request.method().clone();
    match (method, path.as_str()) {
        (Method::Get, "/") | (Method::Get, "/api/ai") => {
            let mut response =
                Response::from_string(welcome_page(port)).with_status_code(StatusCode(200));
            for h in cors_headers() {
                response = response.with_header(h);
            }
            response = response.with_header(header("Content-Type", "text/html; charset=utf-8"));
            response = response.with_header(header("Cache-Control", "no-store"));
            let _ = request.respond(response);
        }
        (Method::Get, "/api/ai/actions") => {
            respond_json(request, 200, &json!({ "ok": true, "actions": actions() }));
        }
        (Method::Get, "/api/ai/status") => {
            respond_json(request, 200, &api.status(false));
        }
        (Method::Get, "/api/ai/state") => {
            respond_json(request, 200, &api.snapshot());
        }
        (Method::Get, "/api/ai/result") => {
            let id = parsed
                .query_pairs()
                .find(|(key, _)| key == "id")
                .map(|(_, value)| value.to_string())
                .unwrap_or_default();
            let bridge = api.inner.bridge.lock().unwrap();
            match bridge.results.get(&id) {
                Some(result) => respond_json(request, 200, result),
                None => respond_json(
                    request,
                    200,
                    &json!({ "ok": false, "id": id, "pending": true }),
                ),
            }
        }
        (Method::Get, "/api/ai/commands") => {
            respond_json(request, 200, &api.log());
        }
        (Method::Post, "/api/ai/command") => {
            let body = match read_body(&mut request, 1_000_000) {
                Ok(body) => body,
                Err(error) => {
                    respond_json(request, 400, &json!({ "ok": false, "error": error }));
                    return;
                }
            };
            let action = body
                .get("action")
                .and_then(|value| value.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            if action.is_empty() {
                respond_json(
                    request,
                    400,
                    &json!({ "ok": false, "error": "Falta action" }),
                );
                return;
            }
            if !ACTION_NAMES.contains(&action.as_str()) {
                respond_json(
                    request,
                    400,
                    &json!({ "ok": false, "error": format!("Acción desconocida: {}", action) }),
                );
                return;
            }
            let params = body.get("params").cloned().unwrap_or_else(|| json!({}));
            let (id, created_at) = api.enqueue(&action, params);
            let wait = body
                .get("wait")
                .and_then(|value| value.as_bool())
                .unwrap_or(true);
            if !wait {
                respond_json(
                    request,
                    202,
                    &json!({ "ok": true, "id": id, "status": "pending", "createdAt": created_at }),
                );
                return;
            }
            let timeout = body
                .get("timeout")
                .and_then(|value| value.as_f64())
                .unwrap_or(25_000.0)
                .clamp(1000.0, 30_000.0) as u64;
            let result = api.wait_result(&id, timeout);
            let pending = result
                .get("pending")
                .and_then(|value| value.as_bool())
                .unwrap_or(false);
            let ok = result
                .get("ok")
                .and_then(|value| value.as_bool())
                .unwrap_or(false);
            let status = if ok {
                200
            } else if pending {
                202
            } else {
                500
            };
            respond_json(request, status, &result);
        }
        (Method::Post, "/api/ai/command/cancel") => {
            let body = match read_body(&mut request, 1_000_000) {
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
            match api.cancel(id) {
                Some(_) => respond_json(
                    request,
                    200,
                    &json!({ "ok": true, "id": id, "status": "error" }),
                ),
                None => respond_json(
                    request,
                    404,
                    &json!({ "ok": false, "error": "Comando no encontrado" }),
                ),
            }
        }
        _ => respond_json(
            request,
            404,
            &json!({ "ok": false, "error": "No encontrado" }),
        ),
    }
}

fn load_token(root: &Path) -> String {
    if let Ok(value) = std::env::var("openCUP_AI_TOKEN") {
        let value = value.trim();
        if !value.is_empty() {
            return value.to_string();
        }
    }
    let path = root.join(".openCUP-ai-token");
    if let Ok(saved) = std::fs::read_to_string(&path) {
        let saved = saved.trim();
        if !saved.is_empty() {
            return saved.to_string();
        }
    }
    let mut rng = rand::thread_rng();
    let bytes: [u8; 18] = rng.gen();
    let token: String = bytes.iter().map(|byte| format!("{:02x}", byte)).collect();
    let _ = std::fs::write(&path, &token);
    token
}

pub fn start(root: &Path, preferred: u16, attempts: u16) -> Result<BridgeHandle, String> {
    let token = load_token(root);
    let mut last_error = String::new();
    let mut selected: Option<(Server, u16)> = None;
    for attempt in 0..=attempts {
        let port = preferred.saturating_add(attempt);
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
            "No se pudo abrir el puente IA en 127.0.0.1:{} ({})",
            preferred, last_error
        ));
    };

    let server = Arc::new(server);
    let inner = Arc::new(Inner {
        bridge: Mutex::new(Bridge {
            clients: HashMap::new(),
            commands: HashMap::new(),
            pending: Vec::new(),
            results: HashMap::new(),
            sequence: 0,
        }),
        changed: Condvar::new(),
    });

    let worker = Arc::clone(&server);
    let worker_inner = Arc::clone(&inner);
    let worker_token = token.clone();
    std::thread::spawn(move || {
        for request in worker.incoming_requests() {
            let inner = Arc::clone(&worker_inner);
            let token = worker_token.clone();
            std::thread::spawn(move || handle(request, inner, token, port));
        }
    });

    Ok(BridgeHandle {
        api: BridgeApi { inner, token, port },
        server,
    })
}
