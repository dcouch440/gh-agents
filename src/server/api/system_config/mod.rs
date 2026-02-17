//! System-wide configuration management endpoints

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::AppError;
use crate::server::auth as auth_utils;
use crate::server::services::system_config as svc;
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

impl From<crate::db::SystemConfigRow> for SystemConfigResponse {
    fn from(r: crate::db::SystemConfigRow) -> Self {
        Self {
            id: r.id,
            config_type: r.config_type,
            config_key: r.config_key,
            config_value: r.config_value,
            description: r.description,
            updated_at: r.updated_at,
        }
    }
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
) -> Result<Json<Vec<SystemConfigResponse>>, AppError> {
    let rows =
        svc::list_system_configs(state.repos().system_config.as_ref(), query.config_type).await?;
    Ok(Json(
        rows.into_iter().map(SystemConfigResponse::from).collect(),
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
) -> Result<Json<SystemConfigResponse>, AppError> {
    let row = svc::upsert_system_config(
        state.repos().system_config.as_ref(),
        svc::UpsertSystemConfigInput {
            config_type: req.config_type,
            config_key: req.config_key,
            config_value: req.config_value,
            description: req.description,
            created_by: Some(auth.user_id.0),
        },
    )
    .await?;
    Ok(Json(SystemConfigResponse::from(row)))
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
) -> Result<StatusCode, AppError> {
    svc::delete_system_config(state.repos().system_config.as_ref(), &key).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests;
