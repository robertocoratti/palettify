use image::{DynamicImage, Rgb, RgbImage};
use rayon::prelude::*;

use crate::color::rgb_to_oklab;
use crate::palette::Palette;

/// Remap every pixel in `img` to its nearest color in `palette`.
///
/// Each pixel is converted to OKLab, the nearest palette color is found via
/// squared Euclidean distance, and the result pixel is set to that color's
/// sRGB values.
///
/// Rayon parallelizes the work across all logical CPUs. The pixel Vec is
/// collected in order so the resulting image is always correct.
pub fn process_image(img: DynamicImage, palette: &Palette) -> RgbImage {
    let img = img.to_rgb8();
    let width = img.width();
    let height = img.height();

    let pixels: Vec<Rgb<u8>> = img.pixels().cloned().collect();

    let raw: Vec<u8> = pixels
        .par_iter()
        .flat_map(|pixel| {
            let [r, g, b] = pixel.0;
            let lab = rgb_to_oklab(r, g, b);
            let nearest = palette.nearest(&lab);
            [nearest.r, nearest.g, nearest.b]
        })
        .collect();

    RgbImage::from_raw(width, height, raw)
        .expect("pixel buffer size mismatch — this is a bug in process_image")
}

#[cfg(test)]
#[path = "tests/processor.rs"]
mod tests;
