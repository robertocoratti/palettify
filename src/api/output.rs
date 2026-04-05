use axum::{body::Body, http::{header, Response, StatusCode}};
use image::{DynamicImage, ImageFormat, RgbImage};
use std::io::Cursor;

use crate::error::AppError;

/// Output image format requested by the caller.
#[derive(Clone, Copy, Default)]
pub enum OutputFormat {
    #[default]
    Png,
    Jpeg,
    WebP,
}

impl OutputFormat {
    /// Parse an output format from a string, defaulting to PNG for unknown values.
    pub fn from_str(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "jpg" | "jpeg" => Self::Jpeg,
            "webp"         => Self::WebP,
            _              => Self::Png,
        }
    }

    fn image_format(self) -> ImageFormat {
        match self {
            Self::Png  => ImageFormat::Png,
            Self::Jpeg => ImageFormat::Jpeg,
            Self::WebP => ImageFormat::WebP,
        }
    }

    pub fn content_type(self) -> &'static str {
        match self {
            Self::Png  => "image/png",
            Self::Jpeg => "image/jpeg",
            Self::WebP => "image/webp",
        }
    }

    pub fn ext(self) -> &'static str {
        match self {
            Self::Png  => "png",
            Self::Jpeg => "jpg",
            Self::WebP => "webp",
        }
    }
}

/// Encode `img` to the requested format and return a ready-to-send HTTP response.
pub fn build_image_response(img: RgbImage, format: OutputFormat) -> Result<Response<Body>, AppError> {
    let mut buf = Cursor::new(Vec::new());
    DynamicImage::ImageRgb8(img)
        .write_to(&mut buf, format.image_format())
        .map_err(|e| AppError::Internal(format!("failed to encode output image: {e}")))?;

    let bytes = buf.into_inner();

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, format.content_type())
        .header(header::CONTENT_LENGTH, bytes.len())
        .header(
            header::CONTENT_DISPOSITION,
            format!("inline; filename=\"result.{}\"", format.ext()),
        )
        .body(Body::from(bytes))
        .unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgb, RgbImage};

    #[test]
    fn from_str_defaults_to_png() {
        assert!(matches!(OutputFormat::from_str("unknown"), OutputFormat::Png));
        assert!(matches!(OutputFormat::from_str(""), OutputFormat::Png));
        assert!(matches!(OutputFormat::default(), OutputFormat::Png));
    }

    #[test]
    fn from_str_jpeg_variants() {
        assert!(matches!(OutputFormat::from_str("jpg"), OutputFormat::Jpeg));
        assert!(matches!(OutputFormat::from_str("jpeg"), OutputFormat::Jpeg));
        assert!(matches!(OutputFormat::from_str("JPG"), OutputFormat::Jpeg));
    }

    #[test]
    fn from_str_webp() {
        assert!(matches!(OutputFormat::from_str("webp"), OutputFormat::WebP));
        assert!(matches!(OutputFormat::from_str("WEBP"), OutputFormat::WebP));
    }

    #[test]
    fn content_type_values() {
        assert_eq!(OutputFormat::Png.content_type(),  "image/png");
        assert_eq!(OutputFormat::Jpeg.content_type(), "image/jpeg");
        assert_eq!(OutputFormat::WebP.content_type(), "image/webp");
    }

    #[test]
    fn ext_values() {
        assert_eq!(OutputFormat::Png.ext(),  "png");
        assert_eq!(OutputFormat::Jpeg.ext(), "jpg");
        assert_eq!(OutputFormat::WebP.ext(), "webp");
    }

    #[test]
    fn build_response_sets_content_type_png() {
        let img = RgbImage::from_pixel(1, 1, Rgb([255u8, 0, 0]));
        let resp = build_image_response(img, OutputFormat::Png).unwrap();
        assert_eq!(resp.status(), axum::http::StatusCode::OK);
        assert_eq!(resp.headers().get("content-type").unwrap(), "image/png");
    }

    #[test]
    fn build_response_sets_content_type_jpeg() {
        let img = RgbImage::from_pixel(1, 1, Rgb([0u8, 255, 0]));
        let resp = build_image_response(img, OutputFormat::Jpeg).unwrap();
        assert_eq!(resp.headers().get("content-type").unwrap(), "image/jpeg");
    }
}
