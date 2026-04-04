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
