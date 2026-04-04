/// Converts an sRGB u8 channel value [0, 255] to linear light [0.0, 1.0].
#[inline]
fn srgb_to_linear(c: u8) -> f32 {
    let c = c as f32 / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// Converts linear sRGB to OKLab.
///
/// OKLab is a perceptual color space where Euclidean distance correlates
/// with human perception of color difference. Using it for nearest-neighbor
/// search produces visually better palette mappings than working in sRGB.
///
/// Reference: <https://bottosson.github.io/posts/oklab/>
#[inline]
pub fn rgb_to_oklab(r: u8, g: u8, b: u8) -> [f32; 3] {
    let r = srgb_to_linear(r);
    let g = srgb_to_linear(g);
    let b = srgb_to_linear(b);

    // Linear RGB to LMS (approximate cone responses).
    let l = 0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b;
    let m = 0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b;
    let s = 0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b;

    // Cube root for perceptual compression.
    let l_ = l.cbrt();
    let m_ = m.cbrt();
    let s_ = s.cbrt();

    // LMS to OKLab.
    [
        0.2104542553 * l_ + 0.7936177850 * m_ - 0.0040720468 * s_,
        1.9779984951 * l_ - 2.4285922050 * m_ + 0.4505937099 * s_,
        0.0259040371 * l_ + 0.7827717662 * m_ - 0.8086757660 * s_,
    ]
}

/// Parses a hex color string into (r, g, b).
/// Accepts both "#RRGGBB" and "RRGGBB" formats.
pub fn parse_hex(hex: &str) -> Option<(u8, u8, u8)> {
    let hex = hex.trim().trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some((r, g, b))
}

#[cfg(test)]
#[path = "tests/color.rs"]
mod tests;
