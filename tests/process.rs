// Integration tests for POST /api/v1/process

mod common;

use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use tower::ServiceExt;

use common::{make_multipart_body, minimal_png, test_app, test_state};

#[tokio::test]
async fn process_with_valid_palette_name_returns_200_png() {
    let app = test_app(test_state(None));
    let (content_type, body) = make_multipart_body(&minimal_png(), Some("nord"), None);

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/process")
                .header("Content-Type", content_type)
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let ct = resp.headers().get("content-type").unwrap().to_str().unwrap();
    assert!(ct.contains("image/png"), "expected image/png, got {ct}");
}

#[tokio::test]
async fn process_with_unknown_palette_returns_404() {
    let app = test_app(test_state(None));
    let (content_type, body) =
        make_multipart_body(&minimal_png(), Some("does-not-exist"), None);

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/process")
                .header("Content-Type", content_type)
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn process_without_palette_returns_400() {
    let app = test_app(test_state(None));
    let (content_type, body) = make_multipart_body(&minimal_png(), None, None);

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/process")
                .header("Content-Type", content_type)
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn process_with_jpg_format_returns_jpeg_content_type() {
    let app = test_app(test_state(None));
    let (content_type, body) =
        make_multipart_body(&minimal_png(), Some("nord"), Some("jpg"));

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/process")
                .header("Content-Type", content_type)
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let ct = resp.headers().get("content-type").unwrap().to_str().unwrap();
    assert!(ct.contains("image/jpeg"), "expected image/jpeg, got {ct}");
}

#[tokio::test]
async fn process_with_correct_api_key_returns_200() {
    let app = test_app(test_state(Some(vec!["test-key-123"])));
    let (content_type, body) = make_multipart_body(&minimal_png(), Some("nord"), None);

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/process")
                .header("Content-Type", content_type)
                .header("x-api-key", "test-key-123")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn process_response_includes_content_disposition() {
    let app = test_app(test_state(None));
    let (content_type, body) = make_multipart_body(&minimal_png(), Some("nord"), None);

    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::POST)
                .uri("/api/v1/process")
                .header("Content-Type", content_type)
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    assert!(
        resp.headers().contains_key("content-disposition"),
        "response should include Content-Disposition header",
    );
}
