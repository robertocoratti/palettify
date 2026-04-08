# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
# Run in development
cargo run

# Build optimized release binary (~5 MB, no runtime deps)
cargo build --release

# Run all tests
cargo test

# Run a single test file (e.g., auth integration tests)
cargo test --test auth

# Run a specific test by name
cargo test test_name

# Run unit tests inside a module (e.g., src/color.rs tests)
cargo test --lib color

# Check for compile errors without building
cargo check

# Lint
cargo clippy

# Log verbosity at runtime
RUST_LOG=palettify=debug,tower_http=debug cargo run
```

## Architecture

This is an Axum HTTP API server with two public routes and one protected route:

- `GET /health` — status check
- `GET /api/v1/palettes` — list available palettes
- `POST /api/v1/process` — process an image (protected by middleware)

**Request lifecycle for `POST /api/v1/process`:**
1. `middleware::authenticate_and_rate_limit` runs first — validates `X-Api-Key` header (when `API_KEYS` is set), then enforces a sliding-window rate limit via Upstash Redis. Both checks are optional in dev (fail-open on missing config).
2. `api::process` parses the multipart body (via `api::extract::parse_process_multipart`), resolves the palette (built-in by name or custom hex list), decodes the image, then dispatches to `tokio::task::spawn_blocking` to avoid blocking the async runtime.
3. `processor::process_image` delegates to `algorithm::Algorithm::run`, which applies the chosen remapping strategy (nearest-color, Floyd-Steinberg dithering, or ordered/Bayer dithering) and returns an `ImageBuffer<Rgb<u8>>`.
4. The result is encoded to PNG/JPEG/WebP and returned as a binary response.

**Key modules:**
- [src/config.rs](src/config.rs) — `Config::from_env()` reads all env vars at startup; zero config needed for dev
- [src/state.rs](src/state.rs) — `AppState` holds `Config`, the palette map (`HashMap<String, Palette>`), and the optional `UpstashClient`; wrapped in `Arc<AppState>` as `SharedState`
- [src/palettes.rs](src/palettes.rs) — embeds the `palettes/*.txt` files at compile time via `include_str!`; `EMBEDDED` is a static slice of `(name, content)` pairs
- [src/palette.rs](src/palette.rs) — `Palette` struct with `from_hex_list`, `from_content`, and `load_directory`; also exposes OKLab colors as a cached `Vec`
- [src/color.rs](src/color.rs) — sRGB ↔ OKLab conversion and nearest-color search
- [src/algorithm/](src/algorithm/mod.rs) — `Algorithm` enum (`Nearest` | `FloydSteinberg` | `Ordered`) with `from_str` and `run`; each variant is implemented in its own submodule (`nearest.rs`, `floyd_steinberg.rs`, `ordered.rs`)
- [src/processor.rs](src/processor.rs) — thin dispatch layer: calls `algorithm.run(img, palette)` inside `spawn_blocking`
- [src/api/](src/api/mod.rs) — handler modules: `health`, `palettes`, `process`; `extract.rs` parses multipart fields; `output.rs` encodes the result image
- [src/upstash.rs](src/upstash.rs) — thin HTTP client over the Upstash Redis REST API; uses a Lua `EVAL` script for atomic sliding-window rate limiting
- [src/error.rs](src/error.rs) — `AppError` enum that implements `IntoResponse` for consistent JSON error bodies

**Palette loading order (startup):** embedded palettes (always) → disk palettes from `PALETTES_DIR` (overlay, can override embedded ones by name).

**Unit tests** live inline inside each module (`#[cfg(test)] mod tests { ... }`). **Integration tests** live in `tests/` and share helpers from `tests/common/mod.rs` (`test_state`, `test_app`, `make_multipart_body`, `minimal_png`).

## Environment Variables

| Variable | Default | Description |
|---|---|---|
| `PORT` | `3000` | TCP port |
| `ENVIRONMENT` | `development` | `production` hides internal error detail |
| `API_KEYS` | (unset = auth disabled) | Comma-separated valid keys for `X-Api-Key` |
| `MAX_UPLOAD_MB` | `10` | Max request body size |
| `MAX_IMAGE_PIXELS` | `25000000` | Max decoded image area (width×height); ~5000×5000 |
| `UPSTASH_REDIS_REST_URL` | (unset = rate limiting off) | Upstash REST endpoint |
| `UPSTASH_REDIS_REST_TOKEN` | — | Upstash bearer token |
| `RATE_LIMIT_REQUESTS` | `60` | Requests per window |
| `RATE_LIMIT_WINDOW_SECS` | `60` | Window duration |
| `PALETTES_DIR` | (unset) | Path to directory of extra `.txt` palette files |

Copy `.env.example` to `.env` for local development.
