use image::{DynamicImage, Rgb, RgbImage};
use rayon::prelude::*;

use crate::color::rgb_to_oklab;
use crate::palette::Palette;

/// Bayer 4×4 threshold matrix. Values in [0, 15]; divide by 16 to normalise.
const BAYER_4X4: [[f32; 4]; 4] = [
    [ 0.0,  8.0,  2.0, 10.0],
    [12.0,  4.0, 14.0,  6.0],
    [ 3.0, 11.0,  1.0,  9.0],
    [15.0,  7.0, 13.0,  5.0],
];

/// Dithering spread in sRGB byte units (±half this value).
/// 32 ≈ 1/8 of the 0-255 range — visible but not overwhelming on small palettes.
const BAYER_SPREAD: f32 = 32.0;

pub(super) fn process(img: DynamicImage, palette: &Palette) -> RgbImage {
    let img = img.to_rgb8();
    let width = img.width();
    let height = img.height();

    let pixels: Vec<Rgb<u8>> = img.pixels().cloned().collect();

    let raw: Vec<u8> = pixels
        .par_iter()
        .enumerate()
        .flat_map(|(i, pixel)| {
            let x = i % width as usize;
            let y = i / width as usize;
            let [r, g, b] = pixel.0;

            let t = (BAYER_4X4[y % 4][x % 4] / 16.0 - 0.5) * BAYER_SPREAD;

            let r2 = (r as f32 + t).clamp(0.0, 255.0) as u8;
            let g2 = (g as f32 + t).clamp(0.0, 255.0) as u8;
            let b2 = (b as f32 + t).clamp(0.0, 255.0) as u8;

            let lab = rgb_to_oklab(r2, g2, b2);
            let nearest = palette.nearest(&lab);
            [nearest.r, nearest.g, nearest.b]
        })
        .collect();

    RgbImage::from_raw(width, height, raw)
        .expect("pixel buffer size mismatch — this is a bug in ordered::process")
}
