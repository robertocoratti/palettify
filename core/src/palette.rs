use std::{collections::HashMap, fs, path::Path};

use serde::Deserialize;

use crate::color::{parse_hex, rgb_to_oklab};
use crate::error::CoreError;

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
    pub fn from_hex_list(name: &str, hex_colors: &[&str]) -> Result<Self, CoreError> {
        if hex_colors.is_empty() {
            return Err(CoreError::BadRequest(
                "palette must contain at least one color".into(),
            ));
        }

        let colors = hex_colors
            .iter()
            .enumerate()
            .map(|(i, hex)| {
                let (r, g, b) = parse_hex(hex).ok_or_else(|| {
                    CoreError::BadRequest(format!(
                        "color at index {i} is not valid hex: '{hex}'"
                    ))
                })?;
                Ok(PaletteColor { name: None, r, g, b, oklab: rgb_to_oklab(r, g, b) })
            })
            .collect::<Result<Vec<_>, CoreError>>()?;

        Ok(Self { name: name.to_string(), description: None, url: None, colors })
    }

    /// Parse a palette from the YAML content of a `.yaml` file.
    ///
    /// Each color entry is either a bare hex string or a single-key map
    /// `{color-name: "#RRGGBB"}`. Mixed palettes (some named, some not) are valid.
    pub fn from_yaml(content: &str) -> Result<Self, CoreError> {
        let file: PaletteFile = serde_yaml::from_str(content)
            .map_err(|e| CoreError::Internal(format!("invalid palette YAML: {e}")))?;

        if file.colors.is_empty() {
            return Err(CoreError::BadRequest(
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
                            CoreError::Internal(format!(
                                "color entry at index {i} is an empty map"
                            ))
                        })?;
                        (Some(name), hex)
                    }
                };

                let (r, g, b) = parse_hex(&hex).ok_or_else(|| {
                    CoreError::BadRequest(format!(
                        "color at index {i} is not valid hex: '{hex}'"
                    ))
                })?;

                Ok(PaletteColor { name, r, g, b, oklab: rgb_to_oklab(r, g, b) })
            })
            .collect::<Result<Vec<_>, CoreError>>()?;

        Ok(Self {
            name:        file.name,
            description: file.description,
            url:         file.url,
            colors,
        })
    }

    /// Load a palette from a `.yaml` file on disk.
    pub fn from_file(path: &Path) -> Result<Self, CoreError> {
        let display = path.display().to_string();
        let content = fs::read_to_string(path).map_err(|e| {
            CoreError::Internal(format!("could not read palette file '{display}': {e}"))
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
