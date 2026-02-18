//! Health check endpoint

use axum::{extract::State, Json};
use serde::Serialize;

use crate::server::state::AppState;

/// Health check response
#[derive(Serialize, utoipa::ToSchema)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub db_connected: bool,
}

/// Enhanced health check endpoint
///
/// Returns JSON with status details including version and database connectivity.
#[utoipa::path(
    get,
    path = "/api/health",
    tag = "Health",
    responses(
        (status = 200, description = "Server health status", body = HealthResponse)
    )
)]
pub async fn health_check(State(state): State<AppState>) -> Json<HealthResponse> {
    let db_connected = state.repos().auth_config.health_check().await;

    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        db_connected,
    })
}

#[cfg(test)]
mod tests;
