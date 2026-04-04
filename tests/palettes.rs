// Integration tests for GET /api/v1/palettes
// Also includes unit-level tests that exercise palette parsing via the library
// interface, without starting a server.

mod common;

use axum::{body::Body, http::Request};
use tower::ServiceExt;

use common::{body_json, test_app, test_state};
use palettify::palette::Palette;

// Endpoint tests

#[tokio::test]
async fn list_palettes_returns_200() {
    let app = test_app(test_state(None));
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/palettes")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), axum::http::StatusCode::OK);
}

#[tokio::test]
async fn list_palettes_includes_all_embedded_palettes() {
    let app = test_app(test_state(None));
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/palettes")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = body_json(resp).await;
    let total = body["total"].as_u64().unwrap();
    assert!(total >= 10, "expected at least 10 embedded palettes, got {total}");
}

#[tokio::test]
async fn list_palettes_response_is_sorted_alphabetically() {
    let app = test_app(test_state(None));
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/palettes")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = body_json(resp).await;
    let names: Vec<String> = body["palettes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["name"].as_str().unwrap().to_string())
        .collect();
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(names, sorted, "palette list should be sorted alphabetically");
}

#[tokio::test]
async fn each_palette_entry_has_required_fields() {
    let app = test_app(test_state(None));
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/palettes")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = body_json(resp).await;
    for palette in body["palettes"].as_array().unwrap() {
        assert!(palette["name"].is_string(), "missing 'name' field");
        assert!(palette["colors"].is_number(), "missing 'colors' field");
        assert!(palette["preview"].is_array(), "missing 'preview' field");
    }
}

// Library-level palette tests (no server)

#[test]
fn palette_from_content_ignores_comments_and_blank_lines() {
    let content = "# Nord palette\n\n#2e3440\n#3b4252\n";
    let p = Palette::from_content("nord", content).unwrap();
    assert_eq!(p.color_count(), 2);
}

#[test]
fn palette_nearest_is_deterministic() {
    let p = Palette::from_hex_list("bw", &["#000000", "#ffffff"]).unwrap();
    let lab = palettify::color::rgb_to_oklab(100, 100, 100);
    let a = p.nearest(&lab);
    let b = p.nearest(&lab);
    assert_eq!((a.r, a.g, a.b), (b.r, b.g, b.b));
}

#[test]
fn palette_preview_is_capped_at_requested_count() {
    let p = Palette::from_hex_list("t", &["#ff0000", "#00ff00", "#0000ff", "#ffffff"]).unwrap();
    assert_eq!(p.preview_colors(2).len(), 2);
}
