/// Converts an sRGB u8 channel value [0, 255] to linear light [0.0, 1.0].
#[inline]
pub fn srgb_to_linear(c: u8) -> f32 {
    let c = c as f32 / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// Converts a linear light value [0.0, 1.0] back to an sRGB u8 channel [0, 255].
#[inline]
pub fn linear_to_srgb(c: f32) -> u8 {
    let c = c.clamp(0.0, 1.0);
    let gamma = if c <= 0.0031308 {
        12.92 * c
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    };
    (gamma * 255.0).round() as u8
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
mod tests {
    use super::*;

    #[test]
    fn parse_hex_with_hash() {
        assert_eq!(parse_hex("#ff0000"), Some((255, 0, 0)));
    }

    #[test]
    fn parse_hex_without_hash() {
        assert_eq!(parse_hex("00ff00"), Some((0, 255, 0)));
    }

    #[test]
    fn parse_hex_uppercase() {
        assert_eq!(parse_hex("#AABBCC"), Some((0xAA, 0xBB, 0xCC)));
    }

    #[test]
    fn parse_hex_rejects_short() {
        assert_eq!(parse_hex("#12345"), None);
    }

    #[test]
    fn parse_hex_rejects_invalid_chars() {
        assert_eq!(parse_hex("#xxyyzz"), None);
    }

    #[test]
    fn parse_hex_rejects_invalid_second_byte() {
        assert_eq!(parse_hex("#00zz00"), None);
    }

    #[test]
    fn parse_hex_rejects_invalid_third_byte() {
        assert_eq!(parse_hex("#0000zz"), None);
    }

    #[test]
    fn black_has_zero_lightness() {
        let [l, _, _] = rgb_to_oklab(0, 0, 0);
        assert!(l < 0.001, "expected L near 0, got {l}");
    }

    #[test]
    fn white_has_max_lightness() {
        let [l, _, _] = rgb_to_oklab(255, 255, 255);
        assert!(l > 0.999, "expected L near 1, got {l}");
    }

    #[test]
    fn neutral_grays_have_near_zero_ab() {
        let [_, a, b] = rgb_to_oklab(128, 128, 128);
        assert!(a.abs() < 0.01, "expected a near 0, got {a}");
        assert!(b.abs() < 0.01, "expected b near 0, got {b}");
    }

    #[test]
    fn srgb_linear_roundtrip() {
        for v in [0u8, 1, 10, 50, 127, 200, 254, 255] {
            let linear = srgb_to_linear(v);
            let back = linear_to_srgb(linear);
            assert_eq!(back, v, "roundtrip failed for {v}");
        }
    }

    #[test]
    fn linear_to_srgb_clamps_out_of_range() {
        assert_eq!(linear_to_srgb(-1.0), 0);
        assert_eq!(linear_to_srgb(2.0), 255);
    }
}
