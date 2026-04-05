use std::time::{SystemTime, UNIX_EPOCH};

use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};

use crate::{error::AppError, state::SharedState};

/// Combined authentication + rate-limiting + usage-tracking middleware.
///
/// Execution order:
///   1. Validate API key when auth is enabled → 401 if missing/invalid.
///   2. Determine rate-limit identity: the API key if present, otherwise
///      the client IP (resolved through Fly.io / proxy headers).
///   3. Run fixed-window rate-limit check via Upstash REST → 429 if exceeded.
///      If Upstash is unreachable the check is skipped (fail-open) so a
///      transient outage never takes down the API.
///   4. Fire-and-forget INCR on the monthly usage counter for the API key.
pub async fn authenticate_and_rate_limit(
    State(state): State<SharedState>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    // ── 1. Authentication ────────────────────────────────────────────────────
    let api_key: Option<String> = if let Some(api_keys) = &state.config.api_keys {
        let provided = request
            .headers()
            .get("x-api-key")
            .and_then(|v| v.to_str().ok())
            .ok_or(AppError::Unauthorized)?;

        if !api_keys.contains(provided) {
            return Err(AppError::Unauthorized);
        }
        Some(provided.to_string())
    } else {
        // Auth disabled (dev mode) — still capture key for tracking if supplied.
        request
            .headers()
            .get("x-api-key")
            .and_then(|v| v.to_str().ok())
            .map(String::from)
    };

    // ── 2 & 3. Rate limiting ─────────────────────────────────────────────────
    if let Some(upstash) = &state.redis {
        let identifier = match &api_key {
            Some(key) => format!("key:{key}"),
            None      => format!("ip:{}", extract_ip(&request)),
        };

        match upstash
            .check_rate_limit(&identifier, state.config.rate_limit_requests, state.config.rate_limit_window_secs)
            .await
        {
            Ok(false) => {
                tracing::warn!(identity = %identifier, "rate limit exceeded");
                return Err(AppError::TooManyRequests);
            }
            Err(e) => {
                // Fail open — a transient Upstash error must not block requests.
                tracing::warn!("Upstash rate-limit check failed (fail-open): {e}");
            }
            Ok(true) => {}
        }

        // ── 4. Usage tracking (fire-and-forget) ──────────────────────────────
        if let Some(key) = api_key.clone() {
            let client    = upstash.clone();
            let usage_key = format!("usage:{key}:{}", current_month());
            tokio::spawn(async move {
                if let Err(e) = client.track_usage(&usage_key, 60 * 60 * 24 * 93).await {
                    tracing::warn!(%key, "usage tracking failed: {e}");
                }
            });
        }
    }

    Ok(next.run(request).await)
}

/// Extract the real client IP from proxy headers.
/// Fly.io injects `Fly-Client-IP`; generic proxies use `X-Forwarded-For`.
fn extract_ip(request: &Request) -> String {
    request
        .headers()
        .get("fly-client-ip")
        .or_else(|| request.headers().get("x-forwarded-for"))
        .or_else(|| request.headers().get("x-real-ip"))
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

/// Returns the current month as "YYYY-MM" using only std (no chrono needed).
/// Uses Howard Hinnant's civil-calendar algorithm.
fn current_month() -> String {
    let days = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        / 86_400;

    let z    = days as i64 + 719_468;
    let era  = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe  = z - era * 146_097;
    let yoe  = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y    = yoe + era * 400;
    let doy  = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp   = (5 * doy + 2) / 153;
    let m    = if mp < 10 { mp + 3 } else { mp - 9 };
    let y    = if m <= 2 { y + 1 } else { y };

    format!("{y:04}-{m:02}")
}
