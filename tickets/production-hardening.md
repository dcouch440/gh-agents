# Production Hardening — Server Middleware & Infrastructure

## Objective

Close the gaps between "works in development" and "safe to run in production" by adding missing middleware layers, fixing health check semantics, and hardening the database connection lifecycle.

---

## Scope

### 1. Request Body Size Limit

Add `DefaultBodyLimit` middleware from `tower-http`. No request should accept unbounded payloads.

- Global default: 1 MB
- Specific overrides where needed (file uploads, board submit with large canvas state)
- Prevents OOM via oversized JSON payloads

### 2. Security Headers

Add a middleware layer that sets on every response:

- `Strict-Transport-Security: max-age=63072000; includeSubDomains` (HSTS)
- `X-Content-Type-Options: nosniff`
- `X-Frame-Options: DENY`
- `Content-Security-Policy: default-src 'self'` (tune for actual asset sources)
- `Referrer-Policy: strict-origin-when-cross-origin`
- `Permissions-Policy: camera=(), microphone=(), geolocation=()`

### 3. Request Timeout

Add `tower-http::timeout::TimeoutLayer` on the router.

- Default: 30 seconds for API routes
- SSE/WebSocket routes excluded (long-lived connections)
- Prevents slow-client connection exhaustion

### 4. Health Check Status Codes

`/api/health` currently returns 200 even when `db_connected: false`. Fix:

- Return 200 only when all checks pass
- Return 503 when any dependency is unhealthy
- Load balancers and K8s probes rely on status codes, not response bodies

### 5. Response Compression

`tower-http` compression feature is already in `Cargo.toml` but never layered on the router. Add `CompressionLayer` for gzip/brotli on API responses.

### 6. CORS Fail-Safe

CORS currently defaults to `Allow-Origin: *` with a warning when `CORS_ORIGINS` is unset. In production mode (`NEXOR_ENV=production`):

- Require `CORS_ORIGINS` to be explicitly set
- Fail startup if missing, don't fall back to permissive

### 7. Database Pool Configuration

Add production pool settings to `init_db_with_config()`:

- `acquire_timeout`: 5 seconds
- `idle_timeout`: 120 seconds
- `max_lifetime`: 30 minutes
- `min_connections`: 5
- Call `pool.close().await` in the shutdown handler

### 8. Global Panic Hook

Install `std::panic::set_hook()` at startup that logs panics as structured tracing errors. A panic in a sync path currently crashes the server silently.

### 9. Startup DB Retry

`init_db()` fails immediately if Postgres is unavailable. Add retry with exponential backoff (3 attempts, 1s/2s/4s) before giving up. Handles slow container orchestration startups.

### 10. Pagination on List Endpoints

Most list endpoints (`list_workflows`, `list_documents`, `list_agents`, etc.) return unbounded results. Add `limit` (default 50, max 1000) and `offset` query params to all list handlers.

### 11. BufferedStream Size Cap

`BufferedStream::buffer` (Vec<StreamChunk>) has no size limit. Long-running SSE streams accumulate unbounded chunks. Add a max buffer size — once reached, drop oldest chunks (late-joining clients get recent context, not full replay).

---

## Out of Scope

- Metrics/Prometheus endpoint (separate ticket)
- Circuit breaker for LLM providers (separate ticket)
- API versioning (premature — API is not public yet)
- Token refresh/rotation (auth improvements are separate)
- TLS termination (handled by reverse proxy, document in deployment guide)

---

## Implementation Notes

Items 1-6 are pure middleware additions — no business logic changes, low risk. Items 7-9 are infrastructure hardening. Items 10-11 touch more surface area but are mechanical.

Most of these are independent and can be done in any order. The security headers and body size limit are the quickest wins.

---

## Guiding Principle

> Every production gap is a door left open. Close the doors before building the next room.
