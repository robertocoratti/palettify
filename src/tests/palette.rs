use super::*;
use crate::color::rgb_to_oklab;

#[test]
fn from_hex_list_rejects_empty() {
    assert!(Palette::from_hex_list("empty", &[]).is_err());
}

#[test]
fn from_hex_list_rejects_invalid_hex() {
    assert!(Palette::from_hex_list("bad", &["#zzzzzz"]).is_err());
}

#[test]
fn from_hex_list_accepts_valid_colors() {
    let p = Palette::from_hex_list("rgb", &["#ff0000", "#00ff00", "#0000ff"]).unwrap();
    assert_eq!(p.colors.len(), 3);
    assert_eq!(p.name, "rgb");
}

#[test]
fn from_content_ignores_comments_and_blank_lines() {
    let content = "# Nord\n\n#2e3440\n#3b4252\n// not a color\n#eceff4\n";
    let p = Palette::from_content("nord", content).unwrap();
    assert_eq!(p.colors.len(), 3);
}

#[test]
fn nearest_returns_exact_match() {
    let p = Palette::from_hex_list("test", &["#ff0000", "#00ff00", "#0000ff"]).unwrap();
    let red_lab = rgb_to_oklab(255, 0, 0);
    let nearest = p.nearest(&red_lab);
    assert_eq!((nearest.r, nearest.g, nearest.b), (255, 0, 0));
}

#[test]
fn nearest_returns_closest_color() {
    let p = Palette::from_hex_list("bw", &["#000000", "#ffffff"]).unwrap();
    let near_white = rgb_to_oklab(200, 200, 200);
    let nearest = p.nearest(&near_white);
    assert_eq!((nearest.r, nearest.g, nearest.b), (255, 255, 255));
}

#[test]
fn preview_colors_returns_first_n() {
    let p = Palette::from_hex_list("t", &["#ff0000", "#00ff00", "#0000ff"]).unwrap();
    let preview = p.preview_colors(2);
    assert_eq!(preview.len(), 2);
    assert_eq!(preview[0], "#ff0000");
}

#[test]
fn hex_formatting_is_lowercase() {
    let p = Palette::from_hex_list("t", &["#AABBCC"]).unwrap();
    assert_eq!(p.colors[0].hex(), "#aabbcc");
}
