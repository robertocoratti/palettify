mod floyd_steinberg;
mod nearest;
mod ordered;

use image::{DynamicImage, RgbImage};

use crate::palette::Palette;

/// Palette-mapping algorithm used when remapping image pixels.
#[derive(Copy, Clone, Default)]
pub enum Algorithm {
    /// Per-pixel nearest-color match in OKLab space.
    /// Fully parallel (Rayon). Best for pixel art and flat-color images.
    #[default]
    Nearest,
    /// Floyd-Steinberg error diffusion dithering.
    /// Sequential (row-by-row). Best for photographs and smooth gradients.
    FloydSteinberg,
    /// Ordered (Bayer 4×4) dithering.
    /// Fully parallel (Rayon). Gives a stylised crosshatch / retro look.
    Ordered,
}

impl Algorithm {
    /// Parse an algorithm from a string value supplied by the caller.
    /// Returns `None` for unrecognised values so the caller can emit the
    /// appropriate error with context.
    pub fn from_str(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "nearest"         => Some(Self::Nearest),
            ""                => Some(Self::Nearest),
            "floyd-steinberg" => Some(Self::FloydSteinberg),
            "dither"          => Some(Self::FloydSteinberg),
            "diffusion"       => Some(Self::FloydSteinberg),
            "ordered"         => Some(Self::Ordered),
            "bayer"           => Some(Self::Ordered),
            _                 => None,
        }
    }

    /// Remap every pixel in `img` to a color in `palette` using this algorithm.
    pub fn run(self, img: DynamicImage, palette: &Palette) -> RgbImage {
        match self {
            Algorithm::Nearest        => nearest::process(img, palette),
            Algorithm::FloydSteinberg => floyd_steinberg::process(img, palette),
            Algorithm::Ordered        => ordered::process(img, palette),
        }
    }
}
