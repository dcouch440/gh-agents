//! HTTP server module for nexor
//!
//! This module provides the Axum-based HTTP server that serves:
//! - REST API endpoints
//! - WebSocket connections for real-time updates
//! - Static files for the React frontend

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

/// Application state shared across all handlers
pub struct AppState {
    pub db: SqlitePool,
    pub scheduler: Arc<RwLock<Scheduler>>,
}

/// Start the HTTP server
pub async fn run_server(
    db: SqlitePool,
    scheduler: Arc<RwLock<Scheduler>>,
    addr: SocketAddr,
) -> Result<()> {
    let state = Arc::new(AppState { db, scheduler });

    // Build router
    let app = Router::new()
        .route("/health", get(health_check))
        .layer(CorsLayer::new().allow_origin(Any))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    info!("Starting server on {}", addr);

    // Start server
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Health check endpoint
async fn health_check() -> &'static str {
    "OK"
}
