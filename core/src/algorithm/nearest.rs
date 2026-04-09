use image::{DynamicImage, Rgb, RgbImage};
use rayon::prelude::*;

use crate::color::rgb_to_oklab;
use crate::palette::Palette;

pub(super) fn process(img: DynamicImage, palette: &Palette) -> RgbImage {
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
        .expect("pixel buffer size mismatch — this is a bug in nearest::process")
}
