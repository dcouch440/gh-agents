//! Prompt template management endpoints

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::constants::{MAX_PROMPT_LENGTH, MAX_TITLE_LENGTH};
use crate::server::auth as auth_utils;
use crate::server::state::AppState;

use super::AppError;

/// Response for a single prompt template.
#[derive(Serialize, utoipa::ToSchema)]
pub struct PromptTemplateResponse {
    pub id: Uuid,
    pub name: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

/// Request body for creating a prompt template.
#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreatePromptTemplateRequest {
    pub name: String,
    pub content: String,
}

/// Request body for updating a prompt template.
#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpdatePromptTemplateRequest {
    pub name: Option<String>,
    pub content: Option<String>,
}

/// GET /api/prompt-templates
#[utoipa::path(
    get,
    path = "/api/prompt-templates",
    tag = "Prompt Templates",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "List of prompt templates", body = Vec<PromptTemplateResponse>)
    )
)]
pub async fn list_prompt_templates(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
) -> Result<Json<Vec<PromptTemplateResponse>>, AppError> {
    let rows = state
        .repos()
        .prompt_templates
        .list_prompt_templates(auth.user_id.0)
        .await?;
    let items = rows
        .into_iter()
        .map(|r| PromptTemplateResponse {
            id: r.id,
            name: r.name,
            content: r.content,
            created_at: r.created_at,
        })
        .collect();
    Ok(Json(items))
}

/// POST /api/prompt-templates
#[utoipa::path(
    post,
    path = "/api/prompt-templates",
    tag = "Prompt Templates",
    security(("bearer_auth" = [])),
    request_body = CreatePromptTemplateRequest,
    responses(
        (status = 201, description = "Prompt template created", body = PromptTemplateResponse),
        (status = 400, description = "Invalid request")
    )
)]
pub async fn create_prompt_template(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Json(request): Json<CreatePromptTemplateRequest>,
) -> Result<(StatusCode, Json<PromptTemplateResponse>), AppError> {
    if request.name.trim().is_empty() || request.name.len() > MAX_TITLE_LENGTH {
        return Err(AppError::bad_request("Template name is empty or exceeds maximum length"));
    }
    if request.content.len() > MAX_PROMPT_LENGTH {
        return Err(AppError::bad_request("Template content exceeds maximum length"));
    }
    let repo = &state.repos().prompt_templates;
    let row = repo
        .create_prompt_template(auth.user_id.0, request.name, request.content)
        .await?;
    Ok((
        StatusCode::CREATED,
        Json(PromptTemplateResponse {
            id: row.id,
            name: row.name,
            content: row.content,
            created_at: row.created_at,
        }),
    ))
}

/// GET /api/prompt-templates/:id
#[utoipa::path(
    get,
    path = "/api/prompt-templates/{id}",
    tag = "Prompt Templates",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Prompt template ID")),
    responses(
        (status = 200, description = "Prompt template found", body = PromptTemplateResponse),
        (status = 404, description = "Not found")
    )
)]
pub async fn get_prompt_template(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<PromptTemplateResponse>, AppError> {
    let repo = &state.repos().prompt_templates;
    let row = repo
        .get_prompt_template(id)
        .await?
        .ok_or(AppError::not_found("Prompt template"))?;
    if row.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Prompt template"));
    }
    Ok(Json(PromptTemplateResponse {
        id: row.id,
        name: row.name,
        content: row.content,
        created_at: row.created_at,
    }))
}

/// PUT /api/prompt-templates/:id
#[utoipa::path(
    put,
    path = "/api/prompt-templates/{id}",
    tag = "Prompt Templates",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Prompt template ID")),
    request_body = UpdatePromptTemplateRequest,
    responses(
        (status = 200, description = "Updated prompt template", body = PromptTemplateResponse),
        (status = 404, description = "Not found")
    )
)]
pub async fn update_prompt_template(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdatePromptTemplateRequest>,
) -> Result<Json<PromptTemplateResponse>, AppError> {
    let repo = &state.repos().prompt_templates;
    let existing = repo
        .get_prompt_template(id)
        .await?
        .ok_or(AppError::not_found("Prompt template"))?;
    if existing.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Prompt template"));
    }
    if let Some(ref name) = request.name {
        if name.trim().is_empty() || name.len() > MAX_TITLE_LENGTH {
            return Err(AppError::bad_request("Template name is empty or exceeds maximum length"));
        }
    }
    if let Some(ref content) = request.content {
        if content.len() > MAX_PROMPT_LENGTH {
            return Err(AppError::bad_request("Template content exceeds maximum length"));
        }
    }
    let row = repo
        .update_prompt_template(id, request.name, request.content)
        .await?;
    Ok(Json(PromptTemplateResponse {
        id: row.id,
        name: row.name,
        content: row.content,
        created_at: row.created_at,
    }))
}

/// DELETE /api/prompt-templates/:id
#[utoipa::path(
    delete,
    path = "/api/prompt-templates/{id}",
    tag = "Prompt Templates",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Prompt template ID")),
    responses(
        (status = 204, description = "Deleted successfully"),
        (status = 404, description = "Not found")
    )
)]
pub async fn delete_prompt_template(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, AppError> {
    let repo = &state.repos().prompt_templates;
    let existing = repo
        .get_prompt_template(id)
        .await?
        .ok_or(AppError::not_found("Prompt template"))?;
    if existing.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Prompt template"));
    }
    repo.delete_prompt_template(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests;
