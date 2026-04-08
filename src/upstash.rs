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

/// Result of a rate-limit check, used by the middleware to populate
/// `X-RateLimit-*` response headers.
#[derive(Debug, Clone)]
pub struct RateLimitInfo {
    /// Whether the request is within the allowed limit.
    pub allowed: bool,
    /// Current request count in this window (after incrementing).
    pub count: i64,
    /// Unix timestamp at which the current window expires and the counter resets.
    pub window_reset: u64,
}

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
    /// Returns `Ok(RateLimitInfo)` with the check result and counters that
    /// callers use to populate `X-RateLimit-*` response headers.
    /// On network/parse error returns `Err` — callers must fail open so a
    /// Redis outage never takes down the API.
    pub async fn check_rate_limit(
        &self,
        identifier:  &str,
        limit:       u64,
        window_secs: u64,
    ) -> Result<RateLimitInfo, reqwest::Error> {
        let window_id    = unix_secs() / window_secs;
        let window_reset = (window_id + 1) * window_secs;
        let key          = format!("ratelimit:{identifier}:{window_id}");

        let count: i64 = self.command(vec![json!("INCR"), json!(&key)]).await?;

        // On the first request in a new window, set the TTL so the key
        // expires automatically. Fire-and-forget: a failure here is harmless
        // because the next window rotation will create a fresh key anyway.
        if count == 1 {
            let client = self.clone();
            let k   = key.clone();
            let ttl = (window_secs + 1).to_string();
            tokio::spawn(async move {
                let _ = client.command::<i64>(vec![json!("EXPIRE"), json!(k), json!(ttl)]).await;
            });
        }

        Ok(RateLimitInfo {
            allowed:      count <= limit as i64,
            count,
            window_reset,
        })
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{extract::State, routing::post, Json, Router};
    use serde_json::{json, Value};
    use std::sync::{Arc, Mutex};

    // ── Mock Upstash REST server ─────────────────────────────────────────────

    /// Starts a tiny axum server that mimics the Upstash REST API.
    /// `initial_count` is the value returned before the first INCR increment.
    /// Returns the base URL of the server.
    async fn start_mock(initial_count: i64) -> String {
        let counter = Arc::new(Mutex::new(initial_count));

        async fn incr_handler(
            State(counter): State<Arc<Mutex<i64>>>,
            Json(_cmd): Json<Value>,
        ) -> Json<Value> {
            let mut c = counter.lock().unwrap();
            *c += 1;
            Json(json!({ "result": *c }))
        }

        async fn pipeline_handler(Json(_cmds): Json<Value>) -> Json<Value> {
            Json(json!([{ "result": 1 }, { "result": 1 }]))
        }

        let app = Router::new()
            .route("/",         post(incr_handler))
            .route("/pipeline", post(pipeline_handler))
            .with_state(counter);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

        format!("http://127.0.0.1:{}", addr.port())
    }

    // ── check_rate_limit ────────────────────────────────────────────────────

    #[tokio::test]
    async fn check_rate_limit_allows_first_request() {
        // Counter starts at 0 → INCR returns 1 → 1 ≤ 10 → allowed.
        // count == 1 also triggers the EXPIRE fire-and-forget spawn.
        let url    = start_mock(0).await;
        let client = UpstashClient::new(url, "tok".into());
        let info   = client.check_rate_limit("id", 10, 60).await.unwrap();
        // Give the EXPIRE spawn a chance to run (exercises the async block).
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        assert!(info.allowed);
        assert_eq!(info.count, 1);
        assert!(info.window_reset > 0);
    }

    #[tokio::test]
    async fn check_rate_limit_allows_non_first_request() {
        // Counter starts at 1 → INCR returns 2 → no EXPIRE spawn, still allowed.
        let url    = start_mock(1).await;
        let client = UpstashClient::new(url, "tok".into());
        let info   = client.check_rate_limit("id", 10, 60).await.unwrap();
        assert!(info.allowed);
        assert_eq!(info.count, 2);
    }

    #[tokio::test]
    async fn check_rate_limit_blocks_when_exceeded() {
        // Counter starts at 10 → INCR returns 11 → 11 > 10 → blocked.
        let url    = start_mock(10).await;
        let client = UpstashClient::new(url, "tok".into());
        let info   = client.check_rate_limit("id", 10, 60).await.unwrap();
        assert!(!info.allowed);
        assert_eq!(info.count, 11);
    }

    #[tokio::test]
    async fn check_rate_limit_returns_err_on_connection_failure() {
        let client = UpstashClient::new("http://127.0.0.1:19996".into(), "tok".into());
        assert!(client.check_rate_limit("id", 10, 60).await.is_err());
    }

    // ── track_usage ─────────────────────────────────────────────────────────

    #[tokio::test]
    async fn track_usage_succeeds() {
        let url = start_mock(0).await;
        let client = UpstashClient::new(url, "tok".into());
        client.track_usage("usage:key:2024-01", 60 * 60 * 24 * 90).await.unwrap();
    }
}
