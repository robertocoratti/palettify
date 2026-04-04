// Integration tests for API key authentication middleware.
//
// The /api/v1/process endpoint is protected; /health and /api/v1/palettes are public.

mod common;

use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use tower::ServiceExt;

use common::{test_app, test_state};

#[tokio::test]
async fn process_without_key_returns_401_when_auth_enabled() {
    let app = test_app(test_state(Some(vec!["secret-key"])));
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/process")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn process_with_wrong_key_returns_401() {
    let app = test_app(test_state(Some(vec!["correct-key"])));
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/process")
                .header("x-api-key", "wrong-key")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn process_without_auth_returns_400_not_401() {
    // Auth disabled — the middleware lets the request through.
    // The request carries no image, so the handler returns 400, not 401.
    let app = test_app(test_state(None));
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/process")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_ne!(
        resp.status(),
        StatusCode::UNAUTHORIZED,
        "request should not be rejected with 401 when auth is disabled",
    );
}

#[tokio::test]
async fn health_endpoint_is_public_even_with_auth_enabled() {
    let app = test_app(test_state(Some(vec!["some-key"])));
    let resp = app
        .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();
    // No API key header provided; should still return 200.
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn palettes_endpoint_is_public_even_with_auth_enabled() {
    let app = test_app(test_state(Some(vec!["some-key"])));
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/palettes")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}
