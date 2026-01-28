//! HTTP server module for nexor
//!
//! This module provides the Axum-based HTTP server that serves:
//! - REST API endpoints
//! - WebSocket connections for real-time updates
//! - Static files for the React frontend

pub mod state;

use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use axum::{routing::get, Router};
use sqlx::SqlitePool;
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};
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
    // CORS configuration for development
    // In production, restrict to specific origins
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/api/health", get(health_check))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// Health check endpoint
///
/// Returns "ok" if the server is running.
/// Used by load balancers and monitoring systems.
async fn health_check() -> &'static str {
    "ok"
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

    #[tokio::test]
    async fn health_check_returns_ok() {
        let response = health_check().await;
        assert_eq!(response, "ok");
    }
}
