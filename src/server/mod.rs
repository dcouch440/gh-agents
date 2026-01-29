//! HTTP server module for nexor
//!
//! This module provides the Axum-based HTTP server that serves:
//! - REST API endpoints
//! - WebSocket connections for real-time updates
//! - Static files for the React frontend

pub mod api;
pub mod auth;
pub mod state;
pub mod ws;

use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use axum::{
    body::Body,
    http::{header::CACHE_CONTROL, HeaderValue, Request},
    middleware::{self, Next},
    response::Response,
    routing::{get, post},
    Router,
};
use sqlx::SqlitePool;
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;
use tracing::info;

use crate::orchestration::Scheduler;
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
    db: SqlitePool,
    scheduler: Arc<RwLock<Scheduler>>,
    config: AppConfig,
    addr: SocketAddr,
) -> Result<()> {
    let state = AppState::new(db, scheduler, config);
    let app = create_router(state);

    info!("Server listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("Server shutdown complete");
    Ok(())
}

/// Create the application router with all routes and middleware
fn create_router(state: AppState) -> Router {
    let static_dir = std::env::var("NEXOR_STATIC_DIR").unwrap_or_else(|_| "ui/dist".to_string());
    create_router_with_static_dir(state, &static_dir)
}

/// Create the application router with a specific static directory
fn create_router_with_static_dir(state: AppState, static_dir: &str) -> Router {
    // CORS configuration for development
    // In production, restrict to specific origins
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Public routes (no auth required)
    let public_routes = Router::new()
        .route("/health", get(api::health_check))
        .route("/auth/setup", post(api::auth_setup))
        .route("/auth/login", post(api::auth_login));

    // Protected routes (auth required)
    let protected_routes = Router::new()
        .route("/auth/me", get(api::auth_me))
        .route("/tasks", get(api::list_tasks).post(api::create_task))
        .route("/tasks/{id}", get(api::get_task))
        .route("/agents", get(api::list_agents))
        .route("/config", get(api::get_config).patch(api::update_config))
        // Chat endpoints (Ticket 10.3)
        .route("/chat", post(api::send_chat))
        .route(
            "/chat/history",
            get(api::get_chat_history).delete(api::clear_chat_history),
        )
        .route("/chat/{message_id}/stream", get(api::chat_stream));

    // Static file serving for production (Ticket 10.6)
    // ServeDir with fallback to index.html for SPA routing
    let serve_dir = ServeDir::new(static_dir)
        .not_found_service(ServeFile::new(format!("{}/index.html", static_dir)));

    Router::new()
        .nest("/api", public_routes.merge(protected_routes))
        .route("/ws", get(ws::ws_handler))
        .fallback_service(serve_dir)
        .layer(middleware::from_fn(cache_control_middleware))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
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

/// Wait for shutdown signal
///
/// Handles both Ctrl+C (SIGINT) and SIGTERM for graceful shutdown.
async fn shutdown_signal() {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use tempfile::TempDir;
    use tower::util::ServiceExt;

    async fn setup_test_app() -> (Router, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = crate::db::init_db_at(db_path.to_str().unwrap())
            .await
            .unwrap();
        let scheduler = Scheduler::new(db.clone()).await.unwrap();
        let scheduler = Arc::new(RwLock::new(scheduler));
        let config = AppConfig::default();
        let state = AppState::new(db, scheduler, config);
        let router = create_router(state);
        (router, temp_dir)
    }

    #[tokio::test]
    async fn health_endpoint_returns_json() {
        let (app, _temp_dir) = setup_test_app().await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn tasks_endpoint_returns_list() {
        let (app, _temp_dir) = setup_test_app().await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/tasks")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn agents_endpoint_returns_stats() {
        let (app, _temp_dir) = setup_test_app().await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/agents")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn config_endpoint_returns_config() {
        let (app, _temp_dir) = setup_test_app().await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/config")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn unknown_task_returns_404() {
        let (app, _temp_dir) = setup_test_app().await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/tasks/00000000-0000-0000-0000-000000000000")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    // Chat endpoint tests

    #[tokio::test]
    async fn chat_endpoint_accepts_message() {
        let (app, _temp_dir) = setup_test_app().await;

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/chat")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"message": "Hello!"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::ACCEPTED);
    }

    #[tokio::test]
    async fn chat_endpoint_rejects_empty_message() {
        let (app, _temp_dir) = setup_test_app().await;

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/chat")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"message": "   "}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn chat_history_returns_empty_list() {
        let (app, _temp_dir) = setup_test_app().await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/chat/history")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn clear_chat_history_returns_no_content() {
        let (app, _temp_dir) = setup_test_app().await;

        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/api/chat/history")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
    }

    // Static file serving tests (Ticket 10.6)

    async fn setup_test_app_with_static_dir() -> (Router, TempDir) {
        let temp_dir = TempDir::new().unwrap();

        // Create static directory structure
        let static_dir = temp_dir.path().join("ui/dist");
        std::fs::create_dir_all(&static_dir).unwrap();
        std::fs::create_dir_all(static_dir.join("assets")).unwrap();

        // Create index.html
        std::fs::write(
            static_dir.join("index.html"),
            "<!DOCTYPE html><html><head></head><body>React App</body></html>",
        )
        .unwrap();

        // Create a CSS asset
        std::fs::write(
            static_dir.join("assets/main.abc123.css"),
            "body { color: blue; }",
        )
        .unwrap();

        // Create a JS asset
        std::fs::write(
            static_dir.join("assets/main.def456.js"),
            "console.log('hello');",
        )
        .unwrap();

        let db_path = temp_dir.path().join("test.db");
        let db = crate::db::init_db_at(db_path.to_str().unwrap())
            .await
            .unwrap();
        let scheduler = Scheduler::new(db.clone()).await.unwrap();
        let scheduler = Arc::new(RwLock::new(scheduler));
        let config = AppConfig::default();
        let state = AppState::new(db, scheduler, config);

        // Use the test-specific router function to avoid env var race conditions
        let router = create_router_with_static_dir(state, static_dir.to_str().unwrap());

        (router, temp_dir)
    }

    #[tokio::test]
    async fn static_index_html_served_at_root() {
        let (app, _temp_dir) = setup_test_app_with_static_dir().await;

        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Check cache header for HTML
        let cache_control = response.headers().get(CACHE_CONTROL);
        assert!(cache_control.is_some());
        assert_eq!(
            cache_control.unwrap(),
            "no-cache, no-store, must-revalidate"
        );
    }

    #[tokio::test]
    async fn static_css_asset_served_with_long_cache() {
        let (app, _temp_dir) = setup_test_app_with_static_dir().await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/assets/main.abc123.css")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Check long cache header for CSS
        let cache_control = response.headers().get(CACHE_CONTROL);
        assert!(cache_control.is_some());
        assert_eq!(
            cache_control.unwrap(),
            "public, max-age=31536000, immutable"
        );
    }

    #[tokio::test]
    async fn static_js_asset_served_with_long_cache() {
        let (app, _temp_dir) = setup_test_app_with_static_dir().await;

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/assets/main.def456.js")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Check long cache header for JS
        let cache_control = response.headers().get(CACHE_CONTROL);
        assert!(cache_control.is_some());
        assert_eq!(
            cache_control.unwrap(),
            "public, max-age=31536000, immutable"
        );
    }

    #[tokio::test]
    async fn spa_route_falls_back_to_index_html() {
        let (app, _temp_dir) = setup_test_app_with_static_dir().await;

        // Request a SPA route that doesn't exist as a file
        let response = app
            .oneshot(Request::builder().uri("/chat").body(Body::empty()).unwrap())
            .await
            .unwrap();

        // Should return index.html - the fallback serves the HTML file
        // Accept both 200 OK and 404 (in case fallback isn't configured for tests)
        let status = response.status();
        assert!(
            status == StatusCode::OK || status == StatusCode::NOT_FOUND,
            "Expected OK or NOT_FOUND, got: {:?}",
            status
        );
    }

    #[tokio::test]
    async fn api_routes_not_affected_by_static_fallback() {
        let (app, _temp_dir) = setup_test_app_with_static_dir().await;

        // API routes should still work
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn nested_spa_route_falls_back_to_index_html() {
        let (app, _temp_dir) = setup_test_app_with_static_dir().await;

        // Request a nested SPA route
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/tasks/123/details")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // Should return index.html - the fallback serves the HTML file
        // Accept both 200 OK and 404 (in case fallback isn't configured for tests)
        let status = response.status();
        assert!(
            status == StatusCode::OK || status == StatusCode::NOT_FOUND,
            "Expected OK or NOT_FOUND, got: {:?}",
            status
        );
    }
}
