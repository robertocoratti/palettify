use palettify_core::color::rgb_to_oklab;
use palettify_core::palette::Palette;

// ── from_hex_list ────────────────────────────────────────────────────────────

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
    assert!(p.description.is_none());
    assert!(p.url.is_none());
    assert!(p.colors.iter().all(|c| c.name.is_none()));
}

// ── from_yaml ────────────────────────────────────────────────────────────────

#[test]
fn from_yaml_parses_plain_colors() {
    let yaml = r##"
name: test
colors:
  - "#ff0000"
  - "#00ff00"
  - "#0000ff"
"##;
    let p = Palette::from_yaml(yaml).unwrap();
    assert_eq!(p.name, "test");
    assert_eq!(p.colors.len(), 3);
    assert!(p.colors.iter().all(|c| c.name.is_none()));
    assert!(p.description.is_none());
    assert!(p.url.is_none());
}

#[test]
fn from_yaml_parses_named_colors() {
    let yaml = r##"
name: test
description: A test palette
url: https://example.com
colors:
  - background: "#282a36"
  - foreground: "#f8f8f2"
"##;
    let p = Palette::from_yaml(yaml).unwrap();
    assert_eq!(p.name, "test");
    assert_eq!(p.description.as_deref(), Some("A test palette"));
    assert_eq!(p.url.as_deref(), Some("https://example.com"));
    assert_eq!(p.colors[0].name.as_deref(), Some("background"));
    assert_eq!(p.colors[1].name.as_deref(), Some("foreground"));
}

#[test]
fn from_yaml_parses_mixed_named_and_plain_colors() {
    let yaml = r##"
name: mixed
colors:
  - named-color: "#ff0000"
  - "#00ff00"
"##;
    let p = Palette::from_yaml(yaml).unwrap();
    assert_eq!(p.colors[0].name.as_deref(), Some("named-color"));
    assert!(p.colors[1].name.is_none());
}

#[test]
fn from_yaml_rejects_empty_colors_list() {
    let yaml = "name: empty\ncolors: []\n";
    assert!(Palette::from_yaml(yaml).is_err());
}

#[test]
fn from_yaml_rejects_empty_named_entry() {
    let yaml = "name: bad\ncolors:\n  - {}\n";
    assert!(Palette::from_yaml(yaml).is_err());
}

#[test]
fn from_yaml_rejects_invalid_hex() {
    let yaml = "name: bad\ncolors:\n  - '#zzzzzz'\n";
    assert!(Palette::from_yaml(yaml).is_err());
}

#[test]
fn from_yaml_rejects_malformed_yaml() {
    assert!(Palette::from_yaml("not: valid: yaml: [[[").is_err());
}

#[test]
fn from_yaml_missing_name_returns_error() {
    let yaml = "colors:\n  - '#ff0000'\n";
    assert!(Palette::from_yaml(yaml).is_err());
}

// ── from_file ────────────────────────────────────────────────────────────────

#[test]
fn from_file_loads_valid_yaml_palette() {
    use std::io::Write;
    let dir  = std::env::temp_dir();
    let path = dir.join("palettify_test_from_file.yaml");
    let mut f = std::fs::File::create(&path).unwrap();
    writeln!(
        f,
        "name: test-palette\ndescription: Test\ncolors:\n  - red: \"#ff0000\"\n  - \"#00ff00\""
    )
    .unwrap();
    drop(f);

    let p = Palette::from_file(&path).unwrap();
    assert_eq!(p.name, "test-palette");
    assert_eq!(p.colors.len(), 2);

    let _ = std::fs::remove_file(&path);
}

#[test]
fn from_file_returns_error_for_missing_file() {
    let path = std::path::Path::new("/nonexistent/palettify/palette.yaml");
    assert!(Palette::from_file(path).is_err());
}

// ── load_directory ───────────────────────────────────────────────────────────

#[test]
fn load_directory_loads_yaml_files() {
    use std::io::Write;
    let dir = std::env::temp_dir().join("palettify_test_load_dir_yaml");
    std::fs::create_dir_all(&dir).unwrap();

    let path = dir.join("mypalette.yaml");
    let mut f = std::fs::File::create(&path).unwrap();
    writeln!(f, "name: mypalette\ncolors:\n  - \"#ff0000\"\n  - \"#00ff00\"").unwrap();
    drop(f);

    let map = Palette::load_directory(&dir);
    assert!(map.contains_key("mypalette"));

    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_dir(&dir);
}

#[test]
fn load_directory_skips_non_yaml_files() {
    let dir = std::env::temp_dir().join("palettify_test_non_yaml");
    std::fs::create_dir_all(&dir).unwrap();

    let path = dir.join("palette.txt");
    std::fs::File::create(&path).unwrap();

    let map = Palette::load_directory(&dir);
    assert!(map.is_empty());

    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_dir(&dir);
}

#[test]
fn load_directory_handles_nonexistent_dir() {
    let dir = std::path::Path::new("/nonexistent/palettify/dir");
    let map = Palette::load_directory(dir);
    assert!(map.is_empty());
}

#[test]
fn load_directory_skips_invalid_yaml_files() {
    use std::io::Write;
    let dir = std::env::temp_dir().join("palettify_test_invalid_yaml");
    std::fs::create_dir_all(&dir).unwrap();

    let path = dir.join("bad.yaml");
    let mut f = std::fs::File::create(&path).unwrap();
    writeln!(f, "name: bad\ncolors: []").unwrap();
    drop(f);

    let map = Palette::load_directory(&dir);
    assert!(map.is_empty());

    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_dir(&dir);
}

// ── nearest ──────────────────────────────────────────────────────────────────

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

// ── preview_colors ───────────────────────────────────────────────────────────

#[test]
fn preview_colors_returns_first_n() {
    let p = Palette::from_hex_list("t", &["#ff0000", "#00ff00", "#0000ff"]).unwrap();
    let preview = p.preview_colors(2);
    assert_eq!(preview.len(), 2);
    assert_eq!(preview[0], "#ff0000");
}

#[test]
fn preview_colors_caps_at_available() {
    let p = Palette::from_hex_list("t", &["#ff0000"]).unwrap();
    assert_eq!(p.preview_colors(10).len(), 1);
}

// ── PaletteColor::hex ────────────────────────────────────────────────────────

#[test]
fn hex_formatting_is_lowercase() {
    let p = Palette::from_hex_list("t", &["#AABBCC"]).unwrap();
    assert_eq!(p.colors[0].hex(), "#aabbcc");
}

// ── color_count ──────────────────────────────────────────────────────────────

#[test]
fn color_count_matches_input() {
    let p = Palette::from_hex_list("t", &["#000000", "#ffffff", "#ff0000"]).unwrap();
    assert_eq!(p.color_count(), 3);
}
