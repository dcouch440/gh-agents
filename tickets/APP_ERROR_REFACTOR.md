u

## Summary

Replace all `Result<T, StatusCode>` handler return types with a unified `AppError` enum that provides structured error responses to clients and preserves error context for server-side logging.

## Motivation

Currently every handler returns `Result<T, StatusCode>` and uses `.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)` to discard the original error. This means:

- Clients get bare status codes with no error message or context
- Server logs lose the original error — debugging production issues is guesswork
- Each handler repeats the same `map_err` boilerplate with no consistency

## Scope

- **115 handler signatures** across 21 files in `src/server/api/`
- **197 `map_err` call sites** to update
- **1 new file** for the `AppError` type

## Implementation

### 1. Create `src/server/api/error.rs`

```rust
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

pub enum AppError {
    NotFound(String),
    BadRequest(String),
    Unauthorized(String),
    Conflict(String),
    Internal(anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg.clone()),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, msg.clone()),
            AppError::Internal(err) => {
                tracing::error!("Internal error: {err:#}");
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string())
            }
        };
        (status, Json(ErrorBody { error: message })).into_response()
    }
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        AppError::Internal(err)
    }
}
```

### 2. Re-export from `src/server/api/mod.rs`

```rust
pub mod error;
pub use error::AppError;
```

### 3. Update handlers (file by file)

For each handler in `src/server/api/*/mod.rs`:

- Change return type: `Result<Json<T>, StatusCode>` -> `Result<Json<T>, AppError>`
- Replace `map_err` calls with appropriate variants:
  - `.map_err(|_| StatusCode::NOT_FOUND)` -> `.map_err(|e| AppError::NotFound(e.to_string()))`
  - `.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)` -> `.map_err(AppError::Internal)` (via `From<anyhow>`) or `.map_err(|e| AppError::Internal(e.into()))`
  - `.map_err(|_| StatusCode::BAD_REQUEST)` -> `.map_err(|e| AppError::BadRequest(e.to_string()))`

### 4. Update CLAUDE.md

Change the handler convention line from `StatusCode` to `AppError`.

## Files touched

| Directory | Handlers | map_err sites |
|-----------|----------|---------------|
| workflows/ | 16 | 33 |
| sessions/ | 14 | 29 |
| rooms/ | 13 | 19 |
| router_modes/ | 7 | 20 |
| collections/ | 8 | 12 |
| tool_routers/ | 7 | 11 |
| tools/ | 7 | 10 |
| documents/ | 6 | 8 |
| auth/ | 2 | 8 |
| agent_executions/ | 5 | 8 |
| prompt_templates/ | 5 | 7 |
| output_schemas/ | 5 | 7 |
| agents/ | 5 | 6 |
| agent_context/ | 2 | 4 |
| results/ | 3 | 4 |
| tasks/ | 3 | 3 |
| chat/ | 2 | 3 |
| session_context/ | 2 | 2 |
| costs/ | 1 | 2 |
| cancellation/ | 1 | 1 |
| config/ | 1 | 0 |

## Verification

```bash
cargo check          # All 115 signatures must compile
cargo test           # No behavioral regressions
cargo clippy         # Clean
```

## Risk

Low. Each handler is independent. If it compiles, it works. No behavioral changes — same status codes returned, just with added error bodies.

## Notes

- Do this on a dedicated branch to avoid conflicts with feature work
- Existing tests that assert on `StatusCode` responses may need minor updates to account for JSON error bodies
- Add variants to `AppError` as needed — start minimal, expand later
