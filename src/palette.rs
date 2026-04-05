use std::{collections::HashMap, fs, path::Path};

use crate::color::{parse_hex, rgb_to_oklab};
use crate::error::AppError;

/// A single color stored in both sRGB and OKLab representations.
/// OKLab is precomputed once so nearest-neighbor searches never re-convert.
#[derive(Clone, Debug)]
pub struct PaletteColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
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
    pub name: String,
    pub colors: Vec<PaletteColor>,
}

impl Palette {
    /// Build a palette from a slice of hex strings.
    pub fn from_hex_list(name: &str, hex_colors: &[&str]) -> Result<Self, AppError> {
        if hex_colors.is_empty() {
            return Err(AppError::BadRequest("palette must contain at least one color".into()));
        }

        let colors = hex_colors
            .iter()
            .enumerate()
            .map(|(i, hex)| {
                let (r, g, b) = parse_hex(hex).ok_or_else(|| {
                    AppError::BadRequest(format!("color at index {i} is not valid hex: '{hex}'"))
                })?;
                Ok(PaletteColor {
                    r,
                    g,
                    b,
                    oklab: rgb_to_oklab(r, g, b),
                })
            })
            .collect::<Result<Vec<_>, AppError>>()?;

        Ok(Self { name: name.to_string(), colors })
    }

    /// Parse a palette from the text content of a .txt file.
    ///
    /// Lines that are exactly "#RRGGBB" are treated as colors.
    /// All other lines (comments, blank lines, metadata) are silently ignored.
    pub fn from_content(name: &str, content: &str) -> Result<Self, AppError> {
        let hex_colors: Vec<&str> = content
            .lines()
            .map(str::trim)
            .filter(|line| {
                line.len() == 7
                    && line.starts_with('#')
                    && line[1..].chars().all(|c| c.is_ascii_hexdigit())
            })
            .collect();

        Self::from_hex_list(name, &hex_colors)
    }

    /// Load a palette from a .txt file on disk.
    /// The palette name is derived from the file stem (e.g. "nord.txt" -> "nord").
    pub fn from_file(path: &Path) -> Result<Self, AppError> {
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let content = fs::read_to_string(path)
            .map_err(|e| AppError::Internal(format!("could not read palette file '{name}': {e}")))?;

        Self::from_content(&name, &content)
    }

    /// Load all .txt files from a directory, returning a map of slug to Palette.
    /// Files that fail to parse are logged and skipped.
    pub fn load_directory(dir: &Path) -> HashMap<String, Palette> {
        let mut map = HashMap::new();

        let entries = match fs::read_dir(dir) {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!("could not read palettes directory '{}': {}", dir.display(), e);
                return map;
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("txt") {
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

#[cfg(test)]
mod tests {
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
    fn from_content_rejects_all_non_color_lines() {
        assert!(Palette::from_content("empty", "# only comments\n\n").is_err());
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
    fn preview_colors_caps_at_available() {
        let p = Palette::from_hex_list("t", &["#ff0000"]).unwrap();
        assert_eq!(p.preview_colors(10).len(), 1);
    }

    #[test]
    fn hex_formatting_is_lowercase() {
        let p = Palette::from_hex_list("t", &["#AABBCC"]).unwrap();
        assert_eq!(p.colors[0].hex(), "#aabbcc");
    }

    #[test]
    fn color_count_matches_input() {
        let p = Palette::from_hex_list("t", &["#000000", "#ffffff", "#ff0000"]).unwrap();
        assert_eq!(p.color_count(), 3);
    }
}
