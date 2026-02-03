//! Agent execution and interactive chat endpoints

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

#[derive(Serialize, utoipa::ToSchema)]
pub struct AgentExecutionResponse {
    pub id: Uuid,
    pub stage_execution_id: Uuid,
    pub agent_id: Uuid,
    pub workflow_step_id: Option<Uuid>,
    pub is_interactive: bool,
    pub parent_agent_execution_id: Option<Uuid>,
    pub system_prompt_rendered: String,
    pub input: String,
    pub output: Option<String>,
    pub structured_output: Option<serde_json::Value>,
    pub selected_mode_id: Option<Uuid>,
    pub status: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl From<crate::db::AgentExecutionRow> for AgentExecutionResponse {
    fn from(r: crate::db::AgentExecutionRow) -> Self {
        Self {
            id: r.id,
            stage_execution_id: r.stage_execution_id,
            agent_id: r.agent_id,
            workflow_step_id: r.workflow_step_id,
            is_interactive: r.is_interactive,
            parent_agent_execution_id: r.parent_agent_execution_id,
            system_prompt_rendered: r.system_prompt_rendered,
            input: r.input,
            output: r.output,
            structured_output: r.structured_output,
            selected_mode_id: r.selected_mode_id,
            status: r.status,
            started_at: r.started_at,
            completed_at: r.completed_at,
        }
    }
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct ExecutionMessageResponse {
    pub id: Uuid,
    pub agent_execution_id: Uuid,
    pub role: String,
    pub content: String,
    pub tool_call_id: Option<String>,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub created_at: DateTime<Utc>,
}

impl From<crate::db::ExecutionMessageRow> for ExecutionMessageResponse {
    fn from(r: crate::db::ExecutionMessageRow) -> Self {
        Self {
            id: r.id,
            agent_execution_id: r.agent_execution_id,
            role: r.role,
            content: r.content,
            tool_call_id: r.tool_call_id,
            input_tokens: r.input_tokens,
            output_tokens: r.output_tokens,
            created_at: r.created_at,
        }
    }
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct SendMessageRequest {
    pub content: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct ApproveExecutionRequest {
    pub structured_output: Option<serde_json::Value>,
}

#[utoipa::path(
    get,
    path = "/api/agent-executions/{id}",
    tag = "Agent Executions",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Agent execution ID")),
    responses(
        (status = 200, description = "Agent execution found", body = AgentExecutionResponse),
        (status = 404, description = "Not found")
    )
)]
pub async fn get_agent_execution(State(state): State<AppState>, _auth: auth_utils::AuthUser, Path(id): Path<Uuid>) -> Result<Json<AgentExecutionResponse>, StatusCode> {
    let repo = state.agent_execution_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let row = repo.get_agent_execution(id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(AgentExecutionResponse::from(row)))
}

#[utoipa::path(
    get,
    path = "/api/agent-executions/{id}/messages",
    tag = "Agent Executions",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Agent execution ID")),
    responses(
        (status = 200, description = "List of execution messages", body = Vec<ExecutionMessageResponse>),
        (status = 404, description = "Not found")
    )
)]
pub async fn list_execution_messages(State(state): State<AppState>, _auth: auth_utils::AuthUser, Path(id): Path<Uuid>) -> Result<Json<Vec<ExecutionMessageResponse>>, StatusCode> {
    let repo = state.agent_execution_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    // Verify execution exists
    repo.get_agent_execution(id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;
    let rows = repo.list_execution_messages(id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(rows.into_iter().map(ExecutionMessageResponse::from).collect()))
}

/// POST /api/agent-executions/:id/messages — send a user message to an interactive agent execution.
#[utoipa::path(
    post,
    path = "/api/agent-executions/{id}/messages",
    tag = "Agent Executions",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Agent execution ID")),
    request_body = SendMessageRequest,
    responses(
        (status = 200, description = "Message sent", body = ExecutionMessageResponse),
        (status = 400, description = "Not interactive"),
        (status = 404, description = "Not found")
    )
)]
pub async fn send_execution_message(
    State(state): State<AppState>,
    _auth: auth_utils::AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<SendMessageRequest>,
) -> Result<Json<ExecutionMessageResponse>, StatusCode> {
    let repo = state.agent_execution_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let ae = repo.get_agent_execution(id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;

    if !ae.is_interactive {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Record the user message
    let msg = repo
        .create_execution_message(id, "user", &req.content, None, 0, 0)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ExecutionMessageResponse::from(msg)))
}

/// POST /api/agent-executions/:id/approve — approve an interactive agent execution.
///
/// With no `structured_output` body → approve as-is (main output used).
/// With `structured_output` → approve with changes (revised output used downstream).
#[utoipa::path(
    post,
    path = "/api/agent-executions/{id}/approve",
    tag = "Agent Executions",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Agent execution ID")),
    request_body = ApproveExecutionRequest,
    responses(
        (status = 200, description = "Execution approved", body = AgentExecutionResponse),
        (status = 400, description = "Not interactive or not awaiting user"),
        (status = 404, description = "Not found")
    )
)]
pub async fn approve_execution(
    State(state): State<AppState>,
    _auth: auth_utils::AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<ApproveExecutionRequest>,
) -> Result<Json<AgentExecutionResponse>, StatusCode> {
    let repo = state.agent_execution_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let ae = repo.get_agent_execution(id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?.ok_or(StatusCode::NOT_FOUND)?;

    if !ae.is_interactive || ae.status != "awaiting_user" {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Update status to completed, optionally with revised structured_output
    let updated = repo
        .update_agent_execution_status(id, "completed", ae.output.clone(), req.structured_output)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(AgentExecutionResponse::from(updated)))
}

mod tests;
