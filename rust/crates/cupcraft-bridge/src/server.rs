//! Servidor HTTP axum para integración de agentes de IA con cupcraft.
//!
//! Escucha exclusivamente en 127.0.0.1 y exige token de autenticación.
//! Actúa directamente sobre el editor en memoria (sin navegador ni sondeo).

use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use cupcraft_core::{Editor, Project};
use cupcraft_persist::Store;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::{Any, CorsLayer};

pub const DEFAULT_PORT: u16 = 3010;

#[derive(Clone)]
pub struct BridgeState {
    pub token: String,
    pub editor: Arc<Mutex<Editor>>,
    pub store: Arc<Mutex<Option<Store>>>,
    pub playhead_nanos: Arc<AtomicU64>,
    pub is_playing: Arc<std::sync::atomic::AtomicBool>,
}

impl BridgeState {
    pub fn new(token: impl Into<String>, editor: Arc<Mutex<Editor>>, store: Option<Store>) -> Self {
        Self {
            token: token.into(),
            editor,
            store: Arc::new(Mutex::new(store)),
            playhead_nanos: Arc::new(AtomicU64::new(0)),
            is_playing: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    pub fn playhead(&self) -> f64 {
        self.playhead_nanos.load(Ordering::Relaxed) as f64 / 1_000_000_000.0
    }

    pub fn set_playhead(&self, sec: f64) {
        let nanos = (sec.max(0.0) * 1_000_000_000.0) as u64;
        self.playhead_nanos.store(nanos, Ordering::Relaxed);
    }
}

#[derive(Debug, Deserialize)]
pub struct ActionRequest {
    pub id: Option<String>,
    pub action: String,
    #[serde(default)]
    pub params: Value,
}

#[derive(Debug, Serialize)]
pub struct ActionResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Genera un token aleatorio seguro de 32 caracteres.
pub fn generate_token() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..32)
        .map(|_| {
            let b = rng.gen_range(0..36);
            if b < 10 {
                (b'0' + b) as char
            } else {
                (b'a' + (b - 10)) as char
            }
        })
        .collect()
}

/// Construye la aplicación axum con todas las rutas y middleware de CORS.
pub fn build_router(state: BridgeState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/health", get(health_handler))
        .route("/api/ai/state", get(state_handler))
        .route("/api/ai/actions", get(actions_handler))
        .route("/api/ai/action", post(action_handler))
        .layer(cors)
        .with_state(state)
}

fn check_auth(headers: &HeaderMap, query: &HashMap<String, String>, expected: &str) -> bool {
    if let Some(token) = headers.get("x-cupcraft-token").and_then(|v| v.to_str().ok()) {
        if token == expected {
            return true;
        }
    }
    if let Some(auth) = headers.get("authorization").and_then(|v| v.to_str().ok()) {
        if let Some(token) = auth.strip_prefix("Bearer ") {
            if token == expected {
                return true;
            }
        }
    }
    if let Some(q_token) = query.get("token") {
        if q_token == expected {
            return true;
        }
    }
    false
}

async fn health_handler() -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "service": "cupcraft-bridge",
        "version": "0.4.5"
    }))
}

async fn actions_handler() -> impl IntoResponse {
    Json(json!({
        "ok": true,
        "actions": [
            "getState",
            "listProjects",
            "newProject",
            "openProject",
            "save",
            "seek",
            "play",
            "pause",
            "togglePlay",
            "selectClip",
            "addText",
            "addSticker",
            "addEffect",
            "applyFilter",
            "updateClip",
            "split",
            "deleteClip",
            "duplicateClip",
            "undo",
            "redo"
        ]
    }))
}

async fn state_handler(
    State(state): State<BridgeState>,
    headers: HeaderMap,
    Query(query): Query<HashMap<String, String>>,
) -> Result<Json<Value>, StatusCode> {
    if !check_auth(&headers, &query, &state.token) {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let editor = state.editor.lock().await;
    let playhead = state.playhead();
    let is_playing = state.is_playing.load(Ordering::Relaxed);
    let selection = editor.selection().map(|s| s.to_string());
    let can_undo = editor.can_undo();
    let can_redo = editor.can_redo();

    let project_json = editor.project().map(|p| {
        json!({
            "id": p.id,
            "name": p.name,
            "ratio": p.ratio,
            "duration": p.timeline_length(),
            "tracks": p.tracks,
            "mediaCount": p.media.len()
        })
    });

    Ok(Json(json!({
        "ok": true,
        "playhead": playhead,
        "playing": is_playing,
        "selection": selection,
        "canUndo": can_undo,
        "canRedo": can_redo,
        "project": project_json
    })))
}

async fn action_handler(
    State(state): State<BridgeState>,
    headers: HeaderMap,
    Query(query): Query<HashMap<String, String>>,
    Json(payload): Json<ActionRequest>,
) -> Result<Json<ActionResponse>, StatusCode> {
    if !check_auth(&headers, &query, &state.token) {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let action = payload.action.as_str();
    let req_id = payload.id;
    let params = payload.params;

    let mut editor = state.editor.lock().await;

    match action {
        "getState" => {
            let playhead = state.playhead();
            let is_playing = state.is_playing.load(Ordering::Relaxed);
            let selection = editor.selection().map(|s| s.to_string());
            let project_json = editor.project().map(|p| {
                json!({
                    "id": p.id,
                    "name": p.name,
                    "duration": p.timeline_length(),
                    "tracks": p.tracks
                })
            });
            Ok(Json(ActionResponse {
                ok: true,
                id: req_id,
                data: Some(json!({
                    "playhead": playhead,
                    "playing": is_playing,
                    "selection": selection,
                    "project": project_json
                })),
                error: None,
            }))
        }

        "seek" => {
            let t = params.get("time").and_then(|v| v.as_f64()).unwrap_or(0.0);
            state.set_playhead(t);
            Ok(Json(ActionResponse {
                ok: true,
                id: req_id,
                data: Some(json!({ "playhead": t })),
                error: None,
            }))
        }

        "play" => {
            state.is_playing.store(true, Ordering::Relaxed);
            Ok(Json(ActionResponse {
                ok: true,
                id: req_id,
                data: Some(json!({ "playing": true })),
                error: None,
            }))
        }

        "pause" => {
            state.is_playing.store(false, Ordering::Relaxed);
            Ok(Json(ActionResponse {
                ok: true,
                id: req_id,
                data: Some(json!({ "playing": false })),
                error: None,
            }))
        }

        "togglePlay" => {
            let current = state.is_playing.load(Ordering::Relaxed);
            state.is_playing.store(!current, Ordering::Relaxed);
            Ok(Json(ActionResponse {
                ok: true,
                id: req_id,
                data: Some(json!({ "playing": !current })),
                error: None,
            }))
        }

        "selectClip" => {
            let clip_id = params.get("clipId").or_else(|| params.get("id")).and_then(|v| v.as_str());
            editor.select(clip_id.map(str::to_string));
            Ok(Json(ActionResponse {
                ok: true,
                id: req_id,
                data: Some(json!({ "selection": clip_id })),
                error: None,
            }))
        }

        "addText" => {
            let text = params.get("text").and_then(|v| v.as_str()).unwrap_or("Texto");
            let duration = params.get("duration").and_then(|v| v.as_f64()).unwrap_or(4.0);
            let start = params.get("start").and_then(|v| v.as_f64()).unwrap_or_else(|| state.playhead());
            let style_val = params.get("style");

            let added_id = editor.mutate(|p| {
                p.add_text_clip(text, Some(start), duration, style_val)
            });

            Ok(Json(ActionResponse {
                ok: added_id.is_some(),
                id: req_id,
                data: added_id.map(|cid| json!({ "clipId": cid })),
                error: None,
            }))
        }

        "addSticker" => {
            let content = params.get("content").and_then(|v| v.as_str()).unwrap_or("✨");
            let duration = params.get("duration").and_then(|v| v.as_f64());
            let start = params.get("start").and_then(|v| v.as_f64()).or_else(|| Some(state.playhead()));
            let preset = params.get("preset").cloned();

            let added_id = editor.mutate(|p| {
                p.add_sticker_clip(content, duration, start, preset)
            });

            Ok(Json(ActionResponse {
                ok: added_id.is_some(),
                id: req_id,
                data: added_id.map(|cid| json!({ "clipId": cid })),
                error: None,
            }))
        }

        "addEffect" => {
            let effect_id = params.get("effectId").or_else(|| params.get("id")).and_then(|v| v.as_str()).unwrap_or("radial_blur");
            let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("Efecto");
            let duration = params.get("duration").and_then(|v| v.as_f64());
            let start = params.get("start").and_then(|v| v.as_f64()).or_else(|| Some(state.playhead()));
            let strength = params.get("strength").and_then(|v| v.as_f64());

            let added_id = editor.mutate(|p| {
                p.add_effect_clip(effect_id, name, duration, start, strength)
            });

            Ok(Json(ActionResponse {
                ok: added_id.is_some(),
                id: req_id,
                data: added_id.map(|cid| json!({ "clipId": cid })),
                error: None,
            }))
        }

        "applyFilter" => {
            let clip_id = params.get("clipId").and_then(|v| v.as_str()).map(str::to_string).or_else(|| editor.selection().map(str::to_string));
            let filter_key = params.get("filter").and_then(|v| v.as_str()).unwrap_or("none");

            if let Some(cid) = clip_id {
                let ok = editor.mutate(|p| p.apply_filter(&cid, filter_key)).unwrap_or(false);
                Ok(Json(ActionResponse {
                    ok,
                    id: req_id,
                    data: Some(json!({ "clipId": cid, "filter": filter_key })),
                    error: if ok { None } else { Some("Clip no encontrado".to_string()) },
                }))
            } else {
                Ok(Json(ActionResponse {
                    ok: false,
                    id: req_id,
                    data: None,
                    error: Some("Ningún clip seleccionado".to_string()),
                }))
            }
        }

        "split" => {
            let time = params.get("time").and_then(|v| v.as_f64()).unwrap_or_else(|| state.playhead());
            let clip_id = params.get("clipId").and_then(|v| v.as_str()).map(str::to_string).or_else(|| editor.selection().map(str::to_string));

            let ok = editor.mutate(|p| {
                p.split_at(time, clip_id.as_deref()) > 0
            }).unwrap_or(false);

            Ok(Json(ActionResponse {
                ok,
                id: req_id,
                data: Some(json!({ "splitAt": time })),
                error: if ok { None } else { Some("No se pudo dividir".to_string()) },
            }))
        }

        "deleteClip" => {
            let clip_id = params.get("clipId").and_then(|v| v.as_str()).map(str::to_string).or_else(|| editor.selection().map(str::to_string));

            if let Some(cid) = clip_id {
                let removed = editor.mutate(|p| p.remove_clip(&cid)).unwrap_or(false);
                if removed {
                    editor.select(None);
                }
                Ok(Json(ActionResponse {
                    ok: removed,
                    id: req_id,
                    data: Some(json!({ "deleted": cid })),
                    error: if removed { None } else { Some("Clip no encontrado".to_string()) },
                }))
            } else {
                Ok(Json(ActionResponse {
                    ok: false,
                    id: req_id,
                    data: None,
                    error: Some("Ningún clip seleccionado".to_string()),
                }))
            }
        }

        "undo" => {
            let ok = editor.undo();
            Ok(Json(ActionResponse {
                ok,
                id: req_id,
                data: Some(json!({ "undone": ok })),
                error: None,
            }))
        }

        "redo" => {
            let ok = editor.redo();
            Ok(Json(ActionResponse {
                ok,
                id: req_id,
                data: Some(json!({ "redone": ok })),
                error: None,
            }))
        }

        "newProject" => {
            let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("Sin título");
            let p = Project::new(name);
            let pid = p.id.clone();
            editor.open(p);
            state.set_playhead(0.0);
            Ok(Json(ActionResponse {
                ok: true,
                id: req_id,
                data: Some(json!({ "projectId": pid, "name": name })),
                error: None,
            }))
        }

        _ => Ok(Json(ActionResponse {
            ok: false,
            id: req_id,
            data: None,
            error: Some(format!("Acción desconocida: {action}")),
        })),
    }
}

/// Inicia el servidor axum en una tarea de background tokio.
pub async fn start_server(state: BridgeState, port: u16) -> Result<SocketAddr, std::io::Error> {
    let app = build_router(state);
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let bound_addr = listener.local_addr()?;

    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });

    Ok(bound_addr)
}
