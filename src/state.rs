use std::{collections::HashMap, path::Path, sync::Arc};

use crate::{config::Config, palette::Palette};

pub type SharedState = Arc<AppState>;

pub struct AppState {
    pub config:   Config,
    pub palettes: HashMap<String, Palette>,
    /// Upstash REST client. None when credentials are not set (dev/test).
    /// Clone is cheap — reqwest::Client is Arc-backed internally.
    pub redis:    Option<crate::upstash::UpstashClient>,
}

impl AppState {
    /// Build the shared application state.
    ///
    /// Palettes are loaded in two steps:
    /// 1. Built-in palettes are read from `./palettes/` at runtime (relative to
    ///    the working directory). In production the Docker image copies the
    ///    `palettes/` directory into `/app/palettes/`.
    /// 2. If `PALETTES_DIR` is set, palettes from that directory are merged on
    ///    top, allowing new palettes to be added or existing ones overridden
    ///    without rebuilding the image.
    ///
    /// `redis` is pre-built in main so that connection errors surface at startup
    /// rather than on the first request.
    pub fn new(config: Config, redis: Option<crate::upstash::UpstashClient>) -> SharedState {
        // Step 1 — built-in palettes from the default directory.
        let mut palettes = Palette::load_directory(Path::new("palettes"));
        tracing::info!("loaded {} built-in palettes from ./palettes/", palettes.len());

        // Step 2 — optional overlay from PALETTES_DIR.
        if let Some(dir) = &config.palettes_dir {
            let extra = Palette::load_directory(dir);
            let n    = extra.len();
            let path = dir.display().to_string();
            tracing::info!("loaded {n} extra palettes from {path}");
            palettes.extend(extra);
        }

        tracing::info!("palettes available: {}", palettes.len());

        Arc::new(AppState { config, palettes, redis })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, Environment};

    fn base_config() -> Config {
        Config {
            port:                   3000,
            environment:            Environment::Development,
            palettes_dir:           None,
            api_keys:               None,
            max_upload_bytes:       10 * 1024 * 1024,
            max_image_pixels:       25_000_000,
            upstash_rest_url:       None,
            upstash_rest_token:     None,
            rate_limit_requests:    60,
            rate_limit_window_secs: 60,
        }
    }

    #[test]
    fn new_loads_palettes_from_default_directory() {
        // cargo test runs from the project root, so ./palettes/ resolves to
        // the actual palettes directory containing all built-in YAML files.
        let state = AppState::new(base_config(), None);
        assert!(!state.palettes.is_empty(), "built-in palettes must be loaded");
        assert!(state.redis.is_none());
    }

    #[test]
    fn new_with_palettes_dir_merges_extra_palettes() {
        use std::io::Write;
        let _ = tracing_subscriber::fmt().try_init();

        let dir = std::env::temp_dir().join("palettify_state_test_overlay");
        std::fs::create_dir_all(&dir).unwrap();

        let path = dir.join("custom.yaml");
        let mut f = std::fs::File::create(&path).unwrap();
        writeln!(
            f,
            "name: custom\ndescription: Test\ncolors:\n  - \"#aabbcc\"\n  - \"#112233\""
        )
        .unwrap();
        drop(f);

        let mut cfg = base_config();
        cfg.palettes_dir = Some(dir.clone());

        let state = AppState::new(cfg, None);
        assert!(state.palettes.contains_key("custom"), "overlay palette must be merged");

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
    }
}
