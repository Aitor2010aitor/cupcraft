use crate::server::*;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use cupcraft_core::{Editor, Project};
use http_body_util::BodyExt;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower::ServiceExt;

#[tokio::test]
async fn test_generate_token() {
    let t1 = generate_token();
    let t2 = generate_token();
    assert_eq!(t1.len(), 32);
    assert_eq!(t2.len(), 32);
    assert_ne!(t1, t2);
}

#[tokio::test]
async fn test_health_check() {
    let editor = Arc::new(Mutex::new(Editor::new()));
    let state = BridgeState::new("secret-token", editor, None);
    let app = build_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "ok");
}

#[tokio::test]
async fn test_auth_rejection() {
    let editor = Arc::new(Mutex::new(Editor::new()));
    let state = BridgeState::new("secret-token", editor, None);
    let app = build_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/ai/state")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_state_and_action() {
    let mut ed = Editor::new();
    ed.open(Project::new("Test Project"));
    let editor = Arc::new(Mutex::new(ed));
    let token = "my-test-token";
    let state = BridgeState::new(token, editor, None);
    let app = build_router(state);

    // 1. Get State with X-Cupcraft-Token
    let req = Request::builder()
        .uri("/api/ai/state")
        .header("x-cupcraft-token", token)
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    let state_json: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(state_json["ok"], true);
    assert_eq!(state_json["project"]["name"], "Test Project");

    // 2. Post Action: addText
    let action_body = serde_json::json!({
        "action": "addText",
        "params": {
            "text": "Hello AI Bridge",
            "start": 0.0,
            "duration": 5.0
        }
    });

    let req2 = Request::builder()
        .method("POST")
        .uri("/api/ai/action")
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&action_body).unwrap()))
        .unwrap();

    let resp2 = app.clone().oneshot(req2).await.unwrap();
    assert_eq!(resp2.status(), StatusCode::OK);
    let body2 = resp2.into_body().collect().await.unwrap().to_bytes();
    let act_json: Value = serde_json::from_slice(&body2).unwrap();
    assert_eq!(act_json["ok"], true);
    assert!(act_json["data"]["clipId"].is_string());

    // 3. Post Action: seek
    let seek_body = serde_json::json!({
        "action": "seek",
        "params": { "time": 2.5 }
    });

    let req3 = Request::builder()
        .method("POST")
        .uri("/api/ai/action")
        .header("x-cupcraft-token", token)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&seek_body).unwrap()))
        .unwrap();

    let resp3 = app.clone().oneshot(req3).await.unwrap();
    assert_eq!(resp3.status(), StatusCode::OK);

    // 4. Post Action: undo
    let undo_body = serde_json::json!({ "action": "undo" });
    let req4 = Request::builder()
        .method("POST")
        .uri("/api/ai/action")
        .header("x-cupcraft-token", token)
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_vec(&undo_body).unwrap()))
        .unwrap();

    let resp4 = app.oneshot(req4).await.unwrap();
    assert_eq!(resp4.status(), StatusCode::OK);
    let body4 = resp4.into_body().collect().await.unwrap().to_bytes();
    let undo_json: Value = serde_json::from_slice(&body4).unwrap();
    assert_eq!(undo_json["ok"], true);
    assert_eq!(undo_json["data"]["undone"], true);
}
