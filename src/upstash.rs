/// Thin async client for the Upstash Redis REST API.
///
/// Instead of a raw Redis TCP+TLS connection (which has brittle cross-platform
/// crate support), we use Upstash's HTTP REST endpoint. Every Redis command
/// maps 1-to-1 to a JSON array; responses are `{"result": <value>}`.
///
/// Clone is cheap — `reqwest::Client` is Arc-backed internally.
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Clone)]
pub struct UpstashClient {
    http:  reqwest::Client,
    url:   String,
    token: String,
}

#[derive(Deserialize)]
struct RestResult<T> {
    result: T,
}

impl UpstashClient {
    pub fn new(url: String, token: String) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .expect("failed to build HTTP client");
        Self { http, url, token }
    }

    /// Execute a single Redis command and deserialize its result.
    async fn command<T>(&self, cmd: Vec<Value>) -> Result<T, reqwest::Error>
    where
        T: for<'de> Deserialize<'de>,
    {
        self.http
            .post(&self.url)
            .bearer_auth(&self.token)
            .json(&cmd)
            .send()
            .await?
            .json::<RestResult<T>>()
            .await
            .map(|r| r.result)
    }

    /// Execute multiple commands in a single HTTP round-trip.
    async fn pipeline(&self, cmds: Vec<Vec<Value>>) -> Result<(), reqwest::Error> {
        let url = format!("{}/pipeline", self.url);
        self.http
            .post(&url)
            .bearer_auth(&self.token)
            .json(&cmds)
            .send()
            .await?;
        Ok(())
    }

    /// Fixed-window rate-limit check.
    ///
    /// Uses a single atomic `INCR` per request — no Lua script required.
    /// The window key includes the current window ID so it naturally rotates
    /// every `window_secs` seconds. TTL is set fire-and-forget on the first
    /// increment so the key is cleaned up automatically.
    ///
    /// Returns `Ok(true)` if the request is allowed, `Ok(false)` if the limit
    /// is exceeded. On network/parse error returns `Err` — callers must
    /// fail open so a Redis outage never takes down the API.
    pub async fn check_rate_limit(
        &self,
        identifier:  &str,
        limit:       u64,
        window_secs: u64,
    ) -> Result<bool, reqwest::Error> {
        let window_id = unix_secs() / window_secs;
        let key = format!("ratelimit:{identifier}:{window_id}");

        let count: i64 = self.command(vec![json!("INCR"), json!(&key)]).await?;

        // On the first request in a new window, set the TTL so the key
        // expires automatically. Fire-and-forget: a failure here is harmless
        // because the next window rotation will create a fresh key anyway.
        if count == 1 {
            let client = self.clone();
            let k = key.clone();
            let ttl = (window_secs + 1).to_string();
            tokio::spawn(async move {
                let _ = client.command::<i64>(vec![json!("EXPIRE"), json!(k), json!(ttl)]).await;
            });
        }

        Ok(count <= limit as i64)
    }

    /// Increment a monthly usage counter and set its TTL (~3 months).
    /// Designed to be called inside `tokio::spawn` (fire-and-forget).
    pub async fn track_usage(
        &self,
        usage_key:   &str,
        expire_secs: i64,
    ) -> Result<(), reqwest::Error> {
        self.pipeline(vec![
            vec![json!("INCR"),   json!(usage_key)],
            vec![json!("EXPIRE"), json!(usage_key), json!(expire_secs.to_string())],
        ])
        .await
    }
}

fn unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
