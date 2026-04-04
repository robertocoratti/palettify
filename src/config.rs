use std::{collections::HashSet, env, path::PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub enum Environment {
    Development,
    Production,
}

impl Default for Environment {
    fn default() -> Self {
        Self::Development
    }
}

/// Application configuration resolved from environment variables at startup.
///
/// All fields have sensible defaults so the server can run with zero
/// configuration in development. In production, set at least API_KEYS and
/// ENVIRONMENT=production.
#[derive(Debug, Clone)]
pub struct Config {
    /// TCP port the server binds to. Default: 3000.
    pub port: u16,

    /// Runtime environment. Controls log format and error detail verbosity.
    pub environment: Environment,

    /// Directory containing .txt palette files loaded at startup.
    /// If None or the path does not exist, only embedded palettes are used.
    pub palettes_dir: Option<PathBuf>,

    /// Accepted API keys for the X-Api-Key header.
    /// If None, authentication is disabled (useful in development).
    pub api_keys: Option<HashSet<String>>,

    /// Maximum allowed request body size in bytes. Default: 50 MB.
    pub max_upload_bytes: usize,

    /// Sustained request rate for POST /api/v1/process, per IP (req/sec). Default: 2.
    pub process_rps: u32,

    /// Burst allowance for POST /api/v1/process, per IP. Default: 5.
    pub process_burst: u32,
}

impl Config {
    pub fn from_env() -> Self {
        let port = env::var("PORT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(3000);

        let environment = match env::var("ENVIRONMENT").as_deref() {
            Ok("production") | Ok("prod") => Environment::Production,
            _ => Environment::Development,
        };

        let palettes_dir = env::var("PALETTES_DIR")
            .ok()
            .map(PathBuf::from)
            .filter(|p| p.is_dir());

        let api_keys = env::var("API_KEYS")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .map(|s| {
                s.split(',')
                    .map(|k| k.trim().to_string())
                    .filter(|k| !k.is_empty())
                    .collect::<HashSet<_>>()
            });

        let max_upload_bytes = env::var("MAX_UPLOAD_MB")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(50)
            * 1024
            * 1024;

        let process_rps = env::var("PROCESS_RPS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(2);

        let process_burst = env::var("PROCESS_BURST")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(5);

        Self {
            port,
            environment,
            palettes_dir,
            api_keys,
            max_upload_bytes,
            process_rps,
            process_burst,
        }
    }

    #[allow(dead_code)]
    pub fn is_production(&self) -> bool {
        self.environment == Environment::Production
    }

    pub fn auth_enabled(&self) -> bool {
        self.api_keys.is_some()
    }
}

#[cfg(test)]
#[path = "tests/config.rs"]
mod tests;
