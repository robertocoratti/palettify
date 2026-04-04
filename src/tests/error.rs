use super::*;
use axum::http::StatusCode;
use axum::response::IntoResponse;

fn status_of(e: AppError) -> StatusCode {
    let resp = e.into_response();
    resp.status()
}

#[test]
fn bad_request_maps_to_400() {
    assert_eq!(status_of(AppError::BadRequest("oops".into())), StatusCode::BAD_REQUEST);
}

#[test]
fn not_found_maps_to_404() {
    assert_eq!(status_of(AppError::NotFound("gone".into())), StatusCode::NOT_FOUND);
}

#[test]
fn unauthorized_maps_to_401() {
    assert_eq!(status_of(AppError::Unauthorized), StatusCode::UNAUTHORIZED);
}

#[test]
fn internal_maps_to_500() {
    assert_eq!(
        status_of(AppError::Internal("boom".into())),
        StatusCode::INTERNAL_SERVER_ERROR,
    );
}
