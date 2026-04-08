use std::{collections::HashMap, fs, path::Path};

use serde::Deserialize;

use crate::color::{parse_hex, rgb_to_oklab};
use crate::error::AppError;

// ── YAML schema ──────────────────────────────────────────────────────────────

/// Raw structure of a `.yaml` palette file as deserialized by serde.
#[derive(Deserialize)]
struct PaletteFile {
    name:        String,
    description: Option<String>,
    url:         Option<String>,
    colors:      Vec<ColorEntry>,
}

/// A single entry in the `colors` list.
///
/// Two forms are supported:
///   - Bare hex string:       `- "#2e3440"`
///   - Single-key named map:  `- polar-night-1: "#2e3440"`
#[derive(Deserialize)]
#[serde(untagged)]
enum ColorEntry {
    Plain(String),
    Named(HashMap<String, String>),
}

// ── Public types ─────────────────────────────────────────────────────────────

/// A single color stored in both sRGB and OKLab representations.
/// OKLab is precomputed once so nearest-neighbor searches never re-convert.
#[derive(Clone, Debug)]
pub struct PaletteColor {
    /// Optional semantic name as defined in the palette YAML (e.g. "polar-night-1").
    /// None for colors provided by the caller via the `palette` API field.
    pub name:  Option<String>,
    pub r:     u8,
    pub g:     u8,
    pub b:     u8,
    pub oklab: [f32; 3],
}

impl PaletteColor {
    pub fn hex(&self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }
}

/// A named collection of colors used to remap an image.
#[derive(Clone, Debug)]
pub struct Palette {
    pub name:        String,
    pub description: Option<String>,
    pub url:         Option<String>,
    pub colors:      Vec<PaletteColor>,
}

// ── Constructors ─────────────────────────────────────────────────────────────

impl Palette {
    /// Build a palette from a slice of hex strings (no names, no metadata).
    ///
    /// Used for caller-supplied custom palettes submitted via the `palette` API
    /// field. Description and URL are left empty.
    pub fn from_hex_list(name: &str, hex_colors: &[&str]) -> Result<Self, AppError> {
        if hex_colors.is_empty() {
            return Err(AppError::BadRequest(
                "palette must contain at least one color".into(),
            ));
        }

        let colors = hex_colors
            .iter()
            .enumerate()
            .map(|(i, hex)| {
                let (r, g, b) = parse_hex(hex).ok_or_else(|| {
                    AppError::BadRequest(format!(
                        "color at index {i} is not valid hex: '{hex}'"
                    ))
                })?;
                Ok(PaletteColor { name: None, r, g, b, oklab: rgb_to_oklab(r, g, b) })
            })
            .collect::<Result<Vec<_>, AppError>>()?;

        Ok(Self { name: name.to_string(), description: None, url: None, colors })
    }

    /// Parse a palette from the YAML content of a `.yaml` file.
    ///
    /// Each color entry is either a bare hex string or a single-key map
    /// `{color-name: "#RRGGBB"}`. Mixed palettes (some named, some not) are valid.
    pub fn from_yaml(content: &str) -> Result<Self, AppError> {
        let file: PaletteFile = serde_yaml::from_str(content)
            .map_err(|e| AppError::Internal(format!("invalid palette YAML: {e}")))?;

        if file.colors.is_empty() {
            return Err(AppError::BadRequest(
                "palette must contain at least one color".into(),
            ));
        }

        let colors = file
            .colors
            .into_iter()
            .enumerate()
            .map(|(i, entry)| {
                let (name, hex) = match entry {
                    ColorEntry::Plain(hex) => (None, hex),
                    ColorEntry::Named(map) => {
                        let (name, hex) = map.into_iter().next().ok_or_else(|| {
                            AppError::Internal(format!(
                                "color entry at index {i} is an empty map"
                            ))
                        })?;
                        (Some(name), hex)
                    }
                };

                let (r, g, b) = parse_hex(&hex).ok_or_else(|| {
                    AppError::BadRequest(format!(
                        "color at index {i} is not valid hex: '{hex}'"
                    ))
                })?;

                Ok(PaletteColor { name, r, g, b, oklab: rgb_to_oklab(r, g, b) })
            })
            .collect::<Result<Vec<_>, AppError>>()?;

        Ok(Self {
            name:        file.name,
            description: file.description,
            url:         file.url,
            colors,
        })
    }

    /// Load a palette from a `.yaml` file on disk.
    pub fn from_file(path: &Path) -> Result<Self, AppError> {
        let display = path.display().to_string();
        let content = fs::read_to_string(path).map_err(|e| {
            AppError::Internal(format!("could not read palette file '{display}': {e}"))
        })?;
        Self::from_yaml(&content)
    }

    /// Load all `.yaml` files from a directory, returning a map of slug to Palette.
    /// Files that fail to parse are logged and skipped.
    pub fn load_directory(dir: &Path) -> HashMap<String, Palette> {
        let mut map = HashMap::new();

        let entries = match fs::read_dir(dir) {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!(
                    "could not read palettes directory '{}': {}",
                    dir.display(),
                    e
                );
                return map;
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("yaml") {
                continue;
            }
            match Palette::from_file(&path) {
                Ok(p) => {
                    tracing::debug!("loaded palette '{}' ({} colors)", p.name, p.colors.len());
                    map.insert(p.name.clone(), p);
                }
                Err(e) => {
                    tracing::warn!("skipping palette '{}': {}", path.display(), e);
                }
            }
        }

        map
    }

    // ── Queries ───────────────────────────────────────────────────────────────

    /// Return the palette color nearest to `lab` in OKLab space.
    ///
    /// Uses squared Euclidean distance — no sqrt needed for finding the minimum.
    /// For palette sizes of 16-30 colors, brute-force over a contiguous Vec
    /// is faster than a KD-tree because the entire dataset fits in L1 cache.
    #[inline]
    pub fn nearest(&self, lab: &[f32; 3]) -> &PaletteColor {
        self.colors
            .iter()
            .min_by(|a, b| {
                dist_sq(lab, &a.oklab)
                    .partial_cmp(&dist_sq(lab, &b.oklab))
                    .unwrap()
            })
            .unwrap()
    }

    pub fn color_count(&self) -> usize {
        self.colors.len()
    }

    /// Returns the first N hex strings for preview purposes.
    pub fn preview_colors(&self, n: usize) -> Vec<String> {
        self.colors.iter().take(n).map(|c| c.hex()).collect()
    }
}

#[inline(always)]
fn dist_sq(a: &[f32; 3], b: &[f32; 3]) -> f32 {
    let dl = a[0] - b[0];
    let da = a[1] - b[1];
    let db = a[2] - b[2];
    dl * dl + da * da + db * db
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::rgb_to_oklab;

    // ── from_hex_list ────────────────────────────────────────────────────────

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

    // ── from_yaml ────────────────────────────────────────────────────────────

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
        // A map entry with no key-value pair `{}` should trigger the empty-map error.
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

    // ── from_file ────────────────────────────────────────────────────────────

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

    // ── load_directory ───────────────────────────────────────────────────────

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

    // ── nearest ──────────────────────────────────────────────────────────────

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

    // ── preview_colors ───────────────────────────────────────────────────────

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

    // ── PaletteColor::hex ────────────────────────────────────────────────────

    #[test]
    fn hex_formatting_is_lowercase() {
        let p = Palette::from_hex_list("t", &["#AABBCC"]).unwrap();
        assert_eq!(p.colors[0].hex(), "#aabbcc");
    }

    // ── color_count ──────────────────────────────────────────────────────────

    #[test]
    fn color_count_matches_input() {
        let p = Palette::from_hex_list("t", &["#000000", "#ffffff", "#ff0000"]).unwrap();
        assert_eq!(p.color_count(), 3);
    }
}
