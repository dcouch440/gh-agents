//! Router mode CRUD and tool assignment endpoints
//!
//! Router modes are configuration overlays that allow a single router to
//! operate in different "modes" with different prompts, temperature, and tools.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::traits::{CreateRouterModeInput, UpdateRouterModeInput};

use super::AppError;
use crate::constants::MAX_DESCRIPTION_LENGTH;
use crate::server::api::tools::ToolResponse;
use crate::server::auth as auth_utils;
use crate::server::state::AppState;

#[cfg(test)]
mod tests;

// ── Request Types ──────────────────────────────────────────────────────────

/// Request body for creating a router mode.
#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateRouterModeRequest {
    pub mode_key: String,
    pub display_name: String,
    pub description: String,
    pub system_prompt: String,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: i32,
    #[serde(default)]
    pub append_to_agent_system_prompt: bool,
    #[serde(default)]
    pub append_to_agent_tools: bool,
    #[serde(default)]
    pub display_order: i32,
}

fn default_temperature() -> f32 {
    0.7
}
fn default_max_tokens() -> i32 {
    4096
}

/// Request body for updating a router mode.
#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpdateRouterModeRequest {
    pub mode_key: Option<String>,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub system_prompt: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<i32>,
    pub append_to_agent_system_prompt: Option<bool>,
    pub append_to_agent_tools: Option<bool>,
    pub display_order: Option<i32>,
}

/// Request body for setting mode tools.
#[derive(Deserialize, utoipa::ToSchema)]
pub struct SetModeToolsRequest {
    pub tool_ids: Vec<Uuid>,
}

// ── Response Types ─────────────────────────────────────────────────────────

/// Response for a router mode.
#[derive(Serialize, utoipa::ToSchema)]
pub struct RouterModeResponse {
    pub id: Uuid,
    pub router_id: Uuid,
    pub mode_key: String,
    pub display_name: String,
    pub description: String,
    pub system_prompt: String,
    pub temperature: f32,
    pub max_tokens: i32,
    pub append_to_agent_system_prompt: bool,
    pub append_to_agent_tools: bool,
    pub display_order: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl RouterModeResponse {
    fn from_row(row: crate::db::ToolRouterModeRow) -> Self {
        Self {
            id: row.id,
            router_id: row.router_id,
            mode_key: row.mode_key,
            display_name: row.display_name,
            description: row.description,
            system_prompt: row.system_prompt,
            temperature: row.temperature,
            max_tokens: row.max_tokens,
            append_to_agent_system_prompt: row.append_to_agent_system_prompt,
            append_to_agent_tools: row.append_to_agent_tools,
            display_order: row.display_order,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

// ── Validation Helpers ─────────────────────────────────────────────────────

static MODE_KEY_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[a-z][a-z0-9_]*$").unwrap());

/// Validate mode_key: snake_case, 1-50 chars
fn validate_mode_key(key: &str) -> bool {
    !key.is_empty() && key.len() <= 50 && MODE_KEY_REGEX.is_match(key)
}

/// Validate temperature: 0.0 - 2.0
fn validate_temperature(temp: f32) -> bool {
    (0.0..=2.0).contains(&temp)
}

/// Validate max_tokens: 1 - 200,000
fn validate_max_tokens(tokens: i32) -> bool {
    (1..=200_000).contains(&tokens)
}

// ── Handlers ───────────────────────────────────────────────────────────────

/// GET /api/tool-routers/:router_id/modes - List all modes for a router.
#[utoipa::path(
    get,
    path = "/api/tool-routers/{router_id}/modes",
    tag = "Router Modes",
    security(("bearer_auth" = [])),
    params(("router_id" = Uuid, Path, description = "Router ID")),
    responses(
        (status = 200, description = "List of modes", body = Vec<RouterModeResponse>),
        (status = 404, description = "Router not found")
    )
)]
pub async fn list_router_modes(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(router_id): Path<Uuid>,
) -> Result<Json<Vec<RouterModeResponse>>, AppError> {
    let repo = &state.repos().tool_routers;

    // Verify user owns the router
    let router = repo
        .get_tool_router(router_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or(AppError::not_found("Router"))?;

    if router.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Router"));
    }

    // Fetch modes
    let modes = repo
        .list_router_modes(router_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(
        modes
            .into_iter()
            .map(RouterModeResponse::from_row)
            .collect(),
    ))
}

/// POST /api/tool-routers/:router_id/modes - Create a new mode.
#[utoipa::path(
    post,
    path = "/api/tool-routers/{router_id}/modes",
    tag = "Router Modes",
    security(("bearer_auth" = [])),
    params(("router_id" = Uuid, Path, description = "Router ID")),
    request_body = CreateRouterModeRequest,
    responses(
        (status = 201, description = "Mode created", body = RouterModeResponse),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "Router not found"),
        (status = 409, description = "Mode key already exists")
    )
)]
pub async fn create_router_mode(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(router_id): Path<Uuid>,
    Json(request): Json<CreateRouterModeRequest>,
) -> Result<(StatusCode, Json<RouterModeResponse>), AppError> {
    // Validate inputs
    if !validate_mode_key(&request.mode_key) {
        return Err(AppError::bad_request(
            "Invalid mode_key: must be snake_case, 1-50 chars",
        ));
    }
    if request.display_name.trim().is_empty() || request.display_name.len() > 200 {
        return Err(AppError::bad_request(
            "Display name is empty or exceeds maximum length",
        ));
    }
    if request.description.len() > MAX_DESCRIPTION_LENGTH {
        return Err(AppError::bad_request("Description exceeds maximum length"));
    }
    if !validate_temperature(request.temperature) {
        return Err(AppError::bad_request(
            "Temperature must be between 0.0 and 2.0",
        ));
    }
    if !validate_max_tokens(request.max_tokens) {
        return Err(AppError::bad_request(
            "max_tokens must be between 1 and 200000",
        ));
    }

    let repo = &state.repos().tool_routers;

    // Verify user owns the router
    let router = repo
        .get_tool_router(router_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or(AppError::not_found("Router"))?;

    if router.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Router"));
    }

    // Check for duplicate mode_key
    if repo
        .get_router_mode_by_key(router_id, &request.mode_key)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .is_some()
    {
        return Err(AppError::Conflict("Mode key already exists".into()));
    }

    // Create mode
    let mode = repo
        .create_router_mode(CreateRouterModeInput {
            router_id,
            mode_key: request.mode_key.clone(),
            display_name: request.display_name.clone(),
            description: request.description.clone(),
            system_prompt: request.system_prompt.clone(),
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            append_to_agent_system_prompt: request.append_to_agent_system_prompt,
            append_to_agent_tools: request.append_to_agent_tools,
            display_order: request.display_order,
        })
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(RouterModeResponse::from_row(mode)),
    ))
}

/// GET /api/router-modes/:id - Get a single mode by ID.
#[utoipa::path(
    get,
    path = "/api/router-modes/{id}",
    tag = "Router Modes",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Mode ID")),
    responses(
        (status = 200, description = "Mode found", body = RouterModeResponse),
        (status = 404, description = "Not found")
    )
)]
pub async fn get_router_mode(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<RouterModeResponse>, AppError> {
    let repo = &state.repos().tool_routers;

    let mode = repo
        .get_router_mode(id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or(AppError::not_found("Router mode"))?;

    // Verify user owns the parent router
    let router = repo
        .get_tool_router(mode.router_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or(AppError::not_found("Router"))?;

    if router.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Router mode"));
    }

    Ok(Json(RouterModeResponse::from_row(mode)))
}

/// PUT /api/router-modes/:id - Update a mode.
#[utoipa::path(
    put,
    path = "/api/router-modes/{id}",
    tag = "Router Modes",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Mode ID")),
    request_body = UpdateRouterModeRequest,
    responses(
        (status = 200, description = "Updated mode", body = RouterModeResponse),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "Not found"),
        (status = 409, description = "Mode key already exists")
    )
)]
pub async fn update_router_mode(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateRouterModeRequest>,
) -> Result<Json<RouterModeResponse>, AppError> {
    let repo = &state.repos().tool_routers;

    // Get existing mode
    let existing = repo
        .get_router_mode(id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or(AppError::not_found("Router mode"))?;

    // Verify user owns the parent router
    let router = repo
        .get_tool_router(existing.router_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or(AppError::not_found("Router"))?;

    if router.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Router mode"));
    }

    // Validate mode_key if provided
    if let Some(ref key) = request.mode_key {
        if !validate_mode_key(key) {
            return Err(AppError::bad_request(
                "Invalid mode_key: must be snake_case, 1-50 chars",
            ));
        }
        // Check for duplicate (if different from current)
        if key != &existing.mode_key
            && repo
                .get_router_mode_by_key(existing.router_id, key)
                .await
                .map_err(|e| AppError::Internal(e.to_string()))?
                .is_some()
        {
            return Err(AppError::Conflict("Mode key already exists".into()));
        }
    }

    // Validate other fields
    if let Some(ref name) = request.display_name {
        if name.trim().is_empty() || name.len() > 200 {
            return Err(AppError::bad_request(
                "Display name is empty or exceeds maximum length",
            ));
        }
    }
    if let Some(ref desc) = request.description {
        if desc.len() > MAX_DESCRIPTION_LENGTH {
            return Err(AppError::bad_request("Description exceeds maximum length"));
        }
    }
    if let Some(temp) = request.temperature {
        if !validate_temperature(temp) {
            return Err(AppError::bad_request(
                "Temperature must be between 0.0 and 2.0",
            ));
        }
    }
    if let Some(tokens) = request.max_tokens {
        if !validate_max_tokens(tokens) {
            return Err(AppError::bad_request(
                "max_tokens must be between 1 and 200000",
            ));
        }
    }

    // Update
    let updated = repo
        .update_router_mode(UpdateRouterModeInput {
            id,
            mode_key: request.mode_key,
            display_name: request.display_name,
            description: request.description,
            system_prompt: request.system_prompt,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            append_to_agent_system_prompt: request.append_to_agent_system_prompt,
            append_to_agent_tools: request.append_to_agent_tools,
            display_order: request.display_order,
        })
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(RouterModeResponse::from_row(updated)))
}

/// DELETE /api/router-modes/:id - Delete a mode.
#[utoipa::path(
    delete,
    path = "/api/router-modes/{id}",
    tag = "Router Modes",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Mode ID")),
    responses(
        (status = 204, description = "Deleted successfully"),
        (status = 404, description = "Not found")
    )
)]
pub async fn delete_router_mode(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let repo = &state.repos().tool_routers;

    let existing = repo
        .get_router_mode(id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or(AppError::not_found("Router mode"))?;

    // Verify user owns the parent router
    let router = repo
        .get_tool_router(existing.router_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or(AppError::not_found("Router"))?;

    if router.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Router mode"));
    }

    repo.delete_router_mode(id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/router-modes/:id/tools - Get tools assigned to a mode.
#[utoipa::path(
    get,
    path = "/api/router-modes/{id}/tools",
    tag = "Router Modes",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Mode ID")),
    responses(
        (status = 200, description = "List of tools", body = Vec<ToolResponse>),
        (status = 404, description = "Mode not found")
    )
)]
pub async fn get_mode_tools(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<ToolResponse>>, AppError> {
    let repo = &state.repos().tool_routers;

    let mode = repo
        .get_router_mode(id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or(AppError::not_found("Router mode"))?;

    // Verify user owns the parent router
    let router = repo
        .get_tool_router(mode.router_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or(AppError::not_found("Router"))?;

    if router.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Router mode"));
    }

    let tools = repo
        .get_mode_tools(id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(
        tools.into_iter().map(ToolResponse::from_row).collect(),
    ))
}

/// PUT /api/router-modes/:id/tools - Set tools for a mode.
#[utoipa::path(
    put,
    path = "/api/router-modes/{id}/tools",
    tag = "Router Modes",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Mode ID")),
    request_body = SetModeToolsRequest,
    responses(
        (status = 204, description = "Mode tools updated"),
        (status = 404, description = "Mode not found")
    )
)]
pub async fn set_mode_tools(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(id): Path<Uuid>,
    Json(request): Json<SetModeToolsRequest>,
) -> Result<StatusCode, AppError> {
    let repo = &state.repos().tool_routers;

    let mode = repo
        .get_router_mode(id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or(AppError::not_found("Router mode"))?;

    // Verify user owns the parent router
    let router = repo
        .get_tool_router(mode.router_id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or(AppError::not_found("Router"))?;

    if router.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Router mode"));
    }

    repo.set_mode_tools(id, &request.tool_ids)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(StatusCode::NO_CONTENT)
}
