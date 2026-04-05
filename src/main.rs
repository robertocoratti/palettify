use axum::{extract::DefaultBodyLimit, routing::{get, post}, Router};
use std::net::SocketAddr;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

mod algorithm;
mod api;
mod color;
mod config;
mod error;
mod middleware;
mod palette;
mod palettes;
mod processor;
mod state;
mod upstash;

#[tokio::main]
async fn main() {
    // Load .env file when present. In production the platform injects env vars
    // directly, so a missing file is not an error.
    let _ = dotenvy::dotenv();

    let config = config::Config::from_env();
    let max_upload = config.max_upload_bytes;

    // Initialise structured logging. RUST_LOG controls the filter; defaults
    // to info for this crate and tower_http so request traces are visible.
    let log_format = tracing_subscriber::fmt::layer();
    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "palettify=info,tower_http=info".into()),
        )
        .with(log_format)
        .init();

    // Build the Upstash REST client if credentials are present.
    // No TCP connection is made at startup — the first request will verify
    // reachability. A missing URL is not an error; rate limiting and usage
    // tracking are simply disabled (useful for local development).
    let redis = match (&config.upstash_rest_url, &config.upstash_rest_token) {
        (Some(url), Some(token)) => {
            tracing::info!(
                requests    = config.rate_limit_requests,
                window_secs = config.rate_limit_window_secs,
                "Upstash configured — rate limiting and usage tracking enabled",
            );
            Some(upstash::UpstashClient::new(url.clone(), token.clone()))
        }
        _ => {
            tracing::warn!(
                "UPSTASH_REDIS_REST_URL / UPSTASH_REDIS_REST_TOKEN not set \
                 — rate limiting and usage tracking disabled"
            );
            None
        }
    };

    let state = state::AppState::new(config, redis);
    let port  = state.config.port;

    // Protected routes: authentication + rate limiting + usage tracking run
    // as a single middleware layer before the handler.
    let protected = Router::new()
        .route("/api/v1/process", post(api::process))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            middleware::authenticate_and_rate_limit,
        ));

    let app = Router::new()
        .route("/health", get(api::health))
        .route("/api/v1/palettes", get(api::list_palettes))
        .merge(protected)
        .with_state(state)
        .layer(DefaultBodyLimit::max(max_upload))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive());

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("palettify v{} listening on {}", env!("CARGO_PKG_VERSION"), addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind address");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("server error");
}

/// Waits for Ctrl-C (all platforms) or SIGTERM (Unix).
/// Allows in-flight requests to complete before the process exits.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let sigterm = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let sigterm = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c  => tracing::info!("received Ctrl+C, shutting down"),
        _ = sigterm => tracing::info!("received SIGTERM, shutting down"),
    }
}
