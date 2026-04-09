use image::{DynamicImage, RgbImage};

use crate::algorithm::Algorithm;
use crate::palette::Palette;

/// Remap every pixel in `img` to a color in `palette` using the chosen algorithm.
pub fn process_image(img: DynamicImage, palette: &Palette, algorithm: Algorithm) -> RgbImage {
    algorithm.run(img, palette)
}
