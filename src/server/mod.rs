//! HTTP server module for nexor
//!
//! This module provides the Axum-based HTTP server that serves:
//! - REST API endpoints
//! - WebSocket connections for real-time updates
//! - Static files for the React frontend

pub mod api;
pub mod auth;
pub mod executors;
pub mod hub;
pub mod openapi;
pub mod rate_limit;
mod routes;
pub mod services;
pub mod state;
pub mod tools;
pub mod ws;

use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use axum::{
    body::Body,
    extract::State,
    http::{header::CACHE_CONTROL, HeaderName, HeaderValue, Request, StatusCode},
    middleware::{self, Next},
    response::Response,
    routing::get,
    Router,
};
use sqlx::PgPool;
use tower_governor::governor::GovernorConfigBuilder;
use tower_governor::key_extractor::SmartIpKeyExtractor;
use tower_governor::GovernorLayer;
use tower_http::cors::{AllowOrigin, Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::{DefaultMakeSpan, TraceLayer};
use tracing::{info, warn};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::constants::routes as route_paths;
use crate::env::Env;
use crate::types::AppConfig;

pub use state::AppState;

/// Start the HTTP server
///
/// This is the main entry point for the server. It sets up:
/// - Application state with database and scheduler
/// - Router with API endpoints
/// - CORS configuration for development
/// - Request tracing middleware
/// - Graceful shutdown handling
pub async fn start_server(
    db: PgPool,
    config: AppConfig,
    addr: SocketAddr,
    env: Arc<Env>,
) -> Result<()> {
    let (state, chat_rx) = AppState::new(db, config, env).await;

    // Spawn the chat consumer to process chat messages via LLM
    let _chat_consumer_handle = executors::chat::spawn_chat_consumer(state.clone(), chat_rx);

    // Spawn periodic container reaper
    let _reaper_handle = crate::execution::ContainerManager::real().spawn_reaper(
        std::time::Duration::from_secs(crate::constants::CONTAINER_REAPER_MAX_AGE_SECS),
        std::time::Duration::from_secs(crate::constants::CONTAINER_REAPER_INTERVAL_SECS),
        state.shutdown_token().clone(),
    );

    // Spawn periodic task registry cleanup (prune terminal dispatch entries older than 1 hour)
    let _task_cleanup_handle = spawn_task_registry_cleanup(
        state.clone(),
        std::time::Duration::from_secs(300),
        chrono::Duration::hours(1),
    );

    let shutdown_state = state.clone();
    let app = create_router(state.clone());

    info!("Server listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal(shutdown_state))
    .await?;

    // Drain: wait for active executions to complete (with timeout)
    let drain_timeout =
        std::time::Duration::from_secs(crate::constants::SHUTDOWN_DRAIN_TIMEOUT_SECS);
    drain_active_executions(&state, drain_timeout).await;

    // Final reap: force-cleanup any remaining orphaned containers
    let reaped = crate::execution::ContainerManager::real()
        .reap_orphaned_containers(std::time::Duration::ZERO)
        .await;
    if reaped > 0 {
        warn!("Force-reaped {} container(s) during shutdown", reaped);
    }

    info!("Server shutdown complete");
    Ok(())
}

/// Create the application router with all routes, middleware, and rate limiting
fn create_router(state: AppState) -> Router {
    let env = state.env();
    let static_dir = env.static_dir.clone();
    let cors = build_cors_layer(env.cors_origins.as_deref());
    let skip_rate_limit = env.skip_rate_limit;

    let public_routes = if skip_rate_limit {
        info!("Rate limiting disabled (NEXOR_SKIP_RATE_LIMIT=1)");
        routes::build_public_routes()
    } else {
        // Auth routes: 25 requests per second per IP (burst 100). Keyed by IP
        // because there is no session yet — this is the bucket a login attempt
        // is counted against.
        //
        // Unlike the protected API below, nothing on the frontend calls these
        // routes on a loop — login/register/setup fire once per session and
        // `/health` is polled by infra, not the app. The bucket only has to
        // absorb several users behind the same IP (office NAT, shared proxy)
        // hitting login around the same time, not a sustained per-user rate.
        // 25/s with a 100 burst comfortably covers that without leaving the
        // door open to credential-stuffing at hundreds of attempts/sec.
        //
        // tower_governor's `per_second(n)` is NOT "n requests per second" — it
        // sets the replenishment *period* to n seconds, i.e. one slot back
        // every n seconds (see `GovernorConfigBuilder::finish`, which feeds it
        // straight into `Quota::with_period`). `per_second(500)` was refilling
        // one slot every 500 seconds instead of 500/sec, so once the burst was
        // spent the bucket needed the better part of a day to recover. Get N
        // req/sec sustained by setting the period to 1000/N ms instead.
        let auth_rate_limit = GovernorConfigBuilder::default()
            .per_millisecond(40) // 1000ms / 25 req/s
            .burst_size(100)
            .key_extractor(SmartIpKeyExtractor)
            .finish()
            .expect("valid governor config");
        routes::build_public_routes().layer(GovernorLayer {
            config: std::sync::Arc::new(auth_rate_limit),
        })
    };

    let protected_routes = if skip_rate_limit {
        routes::build_protected_routes(state.clone())
    } else {
        // 20 requests per second per signed-in session (burst 1000). Keyed by
        // session rather than IP: see `rate_limit` for why IP-keying silently
        // throttles every user behind a proxy or Docker bridge as if they
        // were one caller.
        //
        // The 20/s floor is derived, not guessed. The only real per-tab
        // background load while a workflow is open is `workflowLiveStore`'s
        // poller (`frontend/src/stores/workflowLiveStore/sync.ts`): every
        // `ACTIVE_POLL_MS` (2s) while a run is active it fires `getLiveState`
        // plus one trace fetch per active dispatch plus a timeline fetch —
        // ~4 requests per tick, i.e. ~2 req/s sustained per open tab (idle
        // tabs fall back to `IDLE_POLL_MS` (15s) and barely register). Two
        // tabs open on the same session doubles that to a 4 req/s floor. On
        // top of that floor we need headroom for actual interactive work
        // that isn't part of the poller — autosave, step edits, dispatching
        // runs — so the sustained rate is set to 5x the floor. That lands at
        // 20 req/s: comfortably above real usage, nowhere near the 200 req/s
        // (12000/min) this bucket used to allow.
        //
        // The burst stays high (1000) because it has to absorb a page load,
        // which fans out into one roster fetch per workflow step plus the
        // editor's own hydration calls — unrelated to the sustained rate and
        // sized independently.
        //
        // See the auth bucket above for why this is `per_millisecond`, not
        // `per_second`: tower_governor's `per_second(n)` sets a refill period
        // of n *seconds per slot*, not n slots per second. `per_second(200)`
        // was replenishing one slot every 200 seconds — once the 1000-request
        // burst was spent (easily, given the live-poll fan-out), the session
        // stayed 429'd for the better part of an hour no matter how idle it
        // was, which is exactly the symptom this bucket exists to prevent.
        let api_rate_limit = GovernorConfigBuilder::default()
            .per_millisecond(50) // 1000ms / 20 req/s
            .burst_size(1000)
            .key_extractor(rate_limit::SessionOrIpKeyExtractor)
            .finish()
            .expect("valid governor config");
        routes::build_protected_routes(state.clone()).layer(GovernorLayer {
            config: std::sync::Arc::new(api_rate_limit),
        })
    };

    let serve_dir = ServeDir::new(&static_dir)
        .not_found_service(ServeFile::new(format!("{}/index.html", static_dir)));

    Router::new()
        .nest("/api", public_routes.merge(protected_routes))
        .merge(
            SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", openapi::ApiDoc::openapi()),
        )
        .route(route_paths::WS, get(ws::ws_handler))
        .fallback_service(serve_dir)
        .layer(middleware::from_fn(request_id_middleware))
        .layer(middleware::from_fn(cache_control_middleware))
        .layer(cors)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(tracing::Level::DEBUG))
                .on_request(tower_http::trace::DefaultOnRequest::new().level(tracing::Level::DEBUG))
                .on_response(
                    tower_http::trace::DefaultOnResponse::new().level(tracing::Level::DEBUG),
                ),
        )
        .with_state(state)
}

/// Build CORS layer from the parsed CORS origins.
///
/// - If origins are provided, parse comma-separated values.
/// - If `None`, default to permissive (dev mode) with a warning.
fn build_cors_layer(cors_origins: Option<&str>) -> CorsLayer {
    // The limiter puts the wait time on the 429 as `x-ratelimit-after`. A
    // cross-origin caller cannot read it unless it is explicitly exposed, and
    // without it a throttled client has nothing to back off against.
    let expose: [HeaderName; 3] = [
        HeaderName::from_static("x-ratelimit-after"),
        HeaderName::from_static("x-ratelimit-limit"),
        HeaderName::from_static("x-ratelimit-remaining"),
    ];

    match cors_origins {
        Some(origins) => {
            let parsed: Vec<HeaderValue> = origins
                .split(',')
                .filter_map(|o| {
                    let trimmed = o.trim();
                    HeaderValue::from_str(trimmed)
                        .map_err(|e| warn!("Invalid CORS origin '{}': {}", trimmed, e))
                        .ok()
                })
                .collect();
            if parsed.is_empty() {
                warn!("CORS_ORIGINS set but no valid origins parsed, falling back to permissive");
                CorsLayer::new()
                    .allow_origin(Any)
                    .allow_methods(Any)
                    .allow_headers(Any)
                    .expose_headers(expose)
            } else {
                info!("CORS restricted to {} origin(s)", parsed.len());
                CorsLayer::new()
                    .allow_origin(AllowOrigin::list(parsed))
                    .allow_methods(Any)
                    .allow_headers(Any)
                    .allow_credentials(true)
                    .expose_headers(expose)
            }
        }
        None => {
            warn!("CORS_ORIGINS not set — allowing all origins (dev mode). Set CORS_ORIGINS for production.");
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any)
                .expose_headers(expose)
        }
    }
}

/// Middleware to require valid JWT on protected routes.
/// Defense-in-depth: handlers also extract AuthUser, but this catches any handler that forgets.
async fn require_auth(
    State(state): State<AppState>,
    request: axum::http::Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // Try Authorization header first, then fall back to ?token= query param
    // (needed for EventSource/SSE which cannot set custom headers)
    let token = request
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.to_string())
        .or_else(|| {
            request.uri().query().and_then(|q| {
                q.split('&')
                    .find_map(|pair| pair.strip_prefix("token=").map(|v| v.to_string()))
            })
        })
        .ok_or(StatusCode::UNAUTHORIZED)?;

    auth::verify_token(&token, state.jwt_secret()).map_err(|_| StatusCode::UNAUTHORIZED)?;

    Ok(next.run(request).await)
}

/// Middleware to set cache headers for static assets
///
/// - Long cache (1 year) for hashed assets (JS, CSS in /assets/)
/// - No cache for HTML files (including SPA fallback routes)
/// - Default cache for other static files
async fn cache_control_middleware(request: Request<Body>, next: Next) -> Response {
    let path = request.uri().path().to_string();
    let mut response = next.run(request).await;

    // Skip cache headers for API routes - they handle their own caching
    if path.starts_with("/api") || path.starts_with("/ws") {
        return response;
    }

    // Long cache for hashed assets (typically in /assets/ with hash in filename)
    if path.contains("/assets/") && (path.ends_with(".js") || path.ends_with(".css")) {
        response.headers_mut().insert(
            CACHE_CONTROL,
            HeaderValue::from_static("public, max-age=31536000, immutable"),
        );
    }
    // No cache for HTML files (ensures fresh version on deployment)
    // Check both path and content-type to handle SPA fallback routes
    else if path == "/" || path.ends_with(".html") || is_html_response(&response) {
        response.headers_mut().insert(
            CACHE_CONTROL,
            HeaderValue::from_static("no-cache, no-store, must-revalidate"),
        );
    }

    response
}

/// Middleware to assign a unique request ID to each request and log a one-line summary.
///
/// Produces: `GET /api/workflows 200 12ms req=94965f2b`
async fn request_id_middleware(request: Request<Body>, next: Next) -> Response {
    let request_id = uuid::Uuid::new_v4().to_string();
    let method = request.method().clone();
    let path = request.uri().path().to_owned();
    let start = std::time::Instant::now();

    let mut response = next.run(request).await;

    let elapsed = start.elapsed();
    let status = response.status().as_u16();
    let short_id = &request_id[..8];
    info!("{method} {path} {status} {elapsed:.0?}  req={short_id}");

    if let Ok(value) = HeaderValue::from_str(&request_id) {
        response
            .headers_mut()
            .insert(HeaderName::from_static("x-request-id"), value);
    }
    response
}

/// Check if a response is HTML based on Content-Type header
fn is_html_response(response: &Response) -> bool {
    use axum::http::header::CONTENT_TYPE;

    response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|ct| ct.starts_with("text/html"))
        .unwrap_or(false)
}

/// Wait for shutdown signal, then cancel all running executions.
///
/// Handles both Ctrl+C (SIGINT) and SIGTERM for graceful shutdown.
async fn shutdown_signal(state: AppState) {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => info!("Received Ctrl+C, starting graceful shutdown..."),
        _ = terminate => info!("Received SIGTERM, starting graceful shutdown..."),
    }

    // Signal all running executions to cancel
    let cancelled = state.cancel_all_executions();
    if cancelled > 0 {
        info!(
            "Cancelled {} running execution(s), waiting for drain...",
            cancelled
        );
    }

    // Cancel all running dispatch tasks
    let dispatch_cancelled = state.task_registry().cancel_all();
    if dispatch_cancelled > 0 {
        info!("Cancelled {} running dispatch task(s)", dispatch_cancelled);
    }

    // Cancel the master shutdown token (stops background tasks like the reaper)
    state.shutdown_token().cancel();
}

/// Wait for all active executions to drain, polling every second until
/// the cancellation_tokens map is empty or the timeout expires.
async fn drain_active_executions(state: &AppState, timeout: std::time::Duration) {
    let start = std::time::Instant::now();
    loop {
        let remaining = state.active_execution_count();
        if remaining == 0 {
            info!("All executions drained cleanly");
            break;
        }
        if start.elapsed() >= timeout {
            warn!(
                "Drain timeout ({:?}) expired with {} execution(s) still active",
                timeout, remaining
            );
            break;
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}

/// Spawn a background task that periodically prunes terminal dispatch entries
/// from the task registry. Runs every `interval` and removes completed/failed/cancelled
/// entries older than `max_age`. Stops when the shutdown token is cancelled.
fn spawn_task_registry_cleanup(
    state: AppState,
    interval: std::time::Duration,
    max_age: chrono::Duration,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let shutdown = state.shutdown_token().clone();
        loop {
            tokio::select! {
                _ = tokio::time::sleep(interval) => {}
                _ = shutdown.cancelled() => break,
            }
            let cutoff = chrono::Utc::now() - max_age;
            state.task_registry().cleanup_before(cutoff);
        }
    })
}

mod tests;
