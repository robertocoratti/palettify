use std::{collections::HashMap, sync::Arc};

use crate::{config::Config, palette::Palette, palettes};

pub type SharedState = Arc<AppState>;

pub struct AppState {
    pub config: Config,
    pub palettes: HashMap<String, Palette>,
    /// Upstash REST client. None when credentials are not set (dev/test).
    /// Clone is cheap — reqwest::Client is Arc-backed internally.
    pub redis: Option<crate::upstash::UpstashClient>,
}

impl AppState {
    /// Build the shared application state.
    ///
    /// Palettes are loaded in two steps:
    /// 1. Embedded palettes (compiled into the binary) are always loaded first.
    /// 2. If PALETTES_DIR points to a valid directory, palettes are loaded from
    ///    disk and merged on top, allowing new palettes to be added or existing
    ///    ones overridden without recompiling.
    ///
    /// `redis` is pre-built in main so that connection errors surface at startup
    /// rather than on the first request.
    pub fn new(config: Config, redis: Option<crate::upstash::UpstashClient>) -> SharedState {
        let mut map = load_palettes(palettes::EMBEDDED);

        if let Some(dir) = &config.palettes_dir {
            let from_disk = Palette::load_directory(dir);
            let n    = from_disk.len();
            let path = dir.display().to_string();
            tracing::info!("loaded {n} palettes from disk ({path})");
            map.extend(from_disk);
        }

        tracing::info!("palettes available: {}", map.len());

        Arc::new(AppState { config, palettes: map, redis })
    }
}

/// Load palettes from a slice of `(name, content)` pairs, skipping any that
/// fail to parse (all embedded palettes are valid, but the Err arm is kept
/// so that a future bad entry is handled gracefully rather than panicking).
pub(crate) fn load_palettes(entries: &[(&str, &str)]) -> HashMap<String, Palette> {
    let mut map = HashMap::new();
    for (name, content) in entries {
        match Palette::from_content(name, content) {
            Ok(p) => {
                map.insert(p.name.clone(), p);
            }
            Err(e) => {
                tracing::warn!("embedded palette '{}' failed to load: {}", name, e);
            }
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, Environment};

    fn base_config() -> Config {
        Config {
            port:                  3000,
            environment:           Environment::Development,
            palettes_dir:          None,
            api_keys:              None,
            max_upload_bytes:      10 * 1024 * 1024,
            upstash_rest_url:      None,
            upstash_rest_token:    None,
            rate_limit_requests:   60,
            rate_limit_window_secs: 60,
        }
    }

    #[test]
    fn new_without_palettes_dir_loads_embedded_palettes() {
        let state = AppState::new(base_config(), None);
        assert!(!state.palettes.is_empty(), "embedded palettes must be present");
        assert!(state.redis.is_none());
    }

    #[test]
    fn new_with_palettes_dir_merges_disk_palettes() {
        use std::io::Write;
        // Init tracing so tracing::info! evaluates its format arguments.
        let _ = tracing_subscriber::fmt().try_init();

        let dir = std::env::temp_dir().join("palettify_state_test");
        std::fs::create_dir_all(&dir).unwrap();

        let path = dir.join("custom.txt");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(f, "#aabbcc\n#112233").unwrap();
        drop(f);

        let mut cfg = base_config();
        cfg.palettes_dir = Some(dir.clone());

        let state = AppState::new(cfg, None);
        assert!(state.palettes.contains_key("custom"), "disk palette must be merged");

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
    }

    // ── load_palettes helper ─────────────────────────────────────────────────

    #[test]
    fn load_palettes_returns_valid_entries() {
        let map = load_palettes(&[("red-blue", "#ff0000\n#0000ff")]);
        assert!(map.contains_key("red-blue"));
    }

    #[test]
    fn load_palettes_skips_entries_with_no_colors() {
        // Exercises the Err branch (empty content → from_content returns Err).
        let map = load_palettes(&[("bad", "# only comments, no hex colors")]);
        assert!(map.is_empty());
    }
}
