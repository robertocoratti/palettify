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
        let mut map: HashMap<String, Palette> = HashMap::new();

        for (name, content) in palettes::EMBEDDED {
            match Palette::from_content(name, content) {
                Ok(p) => {
                    map.insert(p.name.clone(), p);
                }
                Err(e) => {
                    tracing::warn!("embedded palette '{}' failed to load: {}", name, e);
                }
            }
        }

        if let Some(dir) = &config.palettes_dir {
            let from_disk = Palette::load_directory(dir);
            tracing::info!(
                "loaded {} palettes from disk ({})",
                from_disk.len(),
                dir.display()
            );
            map.extend(from_disk);
        }

        tracing::info!("palettes available: {}", map.len());

        Arc::new(AppState { config, palettes: map, redis })
    }
}
