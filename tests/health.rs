// Integration tests for GET /health

mod common;

use axum::{body::Body, http::Request};
use tower::ServiceExt;

use common::{body_json, test_app, test_state};

#[tokio::test]
async fn health_returns_200() {
    let app = test_app(test_state(None));
    let resp = app
        .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), axum::http::StatusCode::OK);
}

#[tokio::test]
async fn health_body_contains_version_and_palette_count() {
    let app = test_app(test_state(None));
    let resp = app
        .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let body = body_json(resp).await;
    assert_eq!(body["status"], "ok");
    assert!(body["version"].is_string());
    assert!(
        body["palettes_loaded"].as_u64().unwrap() > 0,
        "expected at least one embedded palette to be loaded",
    );
}

#[tokio::test]
async fn health_shows_auth_disabled_when_no_keys_configured() {
    let app = test_app(test_state(None));
    let resp = app
        .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let body = body_json(resp).await;
    assert_eq!(body["auth_enabled"], false);
}

#[tokio::test]
async fn health_shows_auth_enabled_when_keys_configured() {
    let app = test_app(test_state(Some(vec!["some-key"])));
    let resp = app
        .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    let body = body_json(resp).await;
    assert_eq!(body["auth_enabled"], true);
}
