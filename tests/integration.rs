// Tests have been split into separate files:
//
//   tests/health.rs   - GET /health endpoint
//   tests/palettes.rs - GET /api/v1/palettes endpoint + palette library tests
//   tests/auth.rs     - API key authentication middleware
//   tests/process.rs  - POST /api/v1/process endpoint
//
// Shared helpers (test_state, test_app, make_multipart_body, minimal_png)
// live in tests/common/mod.rs.
