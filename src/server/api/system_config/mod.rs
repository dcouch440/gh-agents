//! System-wide configuration management endpoints

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::server::auth as auth_utils;
use crate::server::state::AppState;

// ============================================================================
// Types
// ============================================================================

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateSystemConfigRequest {
    pub config_type: String,
    pub config_key: String,
    pub config_value: serde_json::Value,
    pub description: Option<String>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct SystemConfigResponse {
    pub id: Uuid,
    pub config_type: String,
    pub config_key: String,
    pub config_value: serde_json::Value,
    pub description: Option<String>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Deserialize, utoipa::IntoParams)]
pub struct SystemConfigQuery {
    pub config_type: Option<String>,
}

// ============================================================================
// Handlers
// ============================================================================

/// GET /api/system-config
#[utoipa::path(
    get,
    path = "/api/system-config",
    tag = "System Config",
    security(("bearer_auth" = [])),
    params(SystemConfigQuery),
    responses(
        (status = 200, description = "List of system configs", body = Vec<SystemConfigResponse>)
    )
)]
pub async fn list_system_configs(
    State(state): State<AppState>,
    _auth: auth_utils::AuthUser,
    Query(query): Query<SystemConfigQuery>,
) -> Result<Json<Vec<SystemConfigResponse>>, StatusCode> {
    let repo = &state.repos().system_config;
    let rows = repo
        .list_system_configs(query.config_type)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(
        rows.into_iter()
            .map(|r| SystemConfigResponse {
                id: r.id,
                config_type: r.config_type,
                config_key: r.config_key,
                config_value: r.config_value,
                description: r.description,
                updated_at: r.updated_at,
            })
            .collect(),
    ))
}

/// POST /api/system-config
#[utoipa::path(
    post,
    path = "/api/system-config",
    tag = "System Config",
    security(("bearer_auth" = [])),
    request_body = CreateSystemConfigRequest,
    responses(
        (status = 200, description = "Config upserted", body = SystemConfigResponse),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn upsert_system_config(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Json(req): Json<CreateSystemConfigRequest>,
) -> Result<Json<SystemConfigResponse>, StatusCode> {
    if req.config_key.trim().is_empty() || req.config_type.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let repo = &state.repos().system_config;
    let row = repo
        .upsert_system_config(
            &req.config_type,
            &req.config_key,
            &req.config_value,
            req.description,
            Some(auth.user_id.0),
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(SystemConfigResponse {
        id: row.id,
        config_type: row.config_type,
        config_key: row.config_key,
        config_value: row.config_value,
        description: row.description,
        updated_at: row.updated_at,
    }))
}

/// DELETE /api/system-config/:key
#[utoipa::path(
    delete,
    path = "/api/system-config/{key}",
    tag = "System Config",
    security(("bearer_auth" = [])),
    params(("key" = String, Path, description = "Config key")),
    responses(
        (status = 204, description = "Deleted successfully")
    )
)]
pub async fn delete_system_config(
    State(state): State<AppState>,
    _auth: auth_utils::AuthUser,
    Path(key): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let repo = &state.repos().system_config;
    repo.delete_system_config(&key)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests;
