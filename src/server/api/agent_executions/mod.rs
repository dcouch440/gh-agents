//! Agent execution and interactive chat endpoints

use std::convert::Infallible;

use axum::{
    extract::{Path, Query, State},
    response::sse::{Event, Sse},
    Json,
};
use chrono::{DateTime, Utc};
use futures::stream::Stream;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tracing::error;
use uuid::Uuid;

use super::AppError;
use crate::server::auth as auth_utils;
use crate::server::hub;
use crate::server::state::{AppState, StreamChunk};

#[derive(Serialize, utoipa::ToSchema)]
pub struct AgentExecutionResponse {
    pub id: Uuid,
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
    pub is_exemplary: bool,
}

impl From<crate::db::AgentExecutionRow> for AgentExecutionResponse {
    fn from(r: crate::db::AgentExecutionRow) -> Self {
        Self {
            id: r.id,
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
            is_exemplary: r.is_exemplary,
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
pub struct ApproveExecutionRequest {
    pub structured_output: Option<serde_json::Value>,
}

#[derive(Deserialize)]
pub struct ListExecutionsQuery {
    pub status: Option<String>,
}

#[utoipa::path(
    get,
    path = "/api/agent-executions",
    tag = "Agent Executions",
    security(("bearer_auth" = [])),
    params(("status" = Option<String>, Query, description = "Filter by status")),
    responses(
        (status = 200, description = "List of agent executions", body = Vec<AgentExecutionResponse>),
    )
)]
pub async fn list_agent_executions(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Query(query): Query<ListExecutionsQuery>,
) -> Result<Json<Vec<AgentExecutionResponse>>, AppError> {
    let repo = &state.repos().agent_executions;
    let rows = repo
        .list_agent_executions(auth.user_id.0, query.status)
        .await?;
    let items: Vec<AgentExecutionResponse> =
        rows.into_iter().map(AgentExecutionResponse::from).collect();
    Ok(Json(items))
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
pub async fn get_agent_execution(
    State(state): State<AppState>,
    _auth: auth_utils::AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<AgentExecutionResponse>, AppError> {
    let repo = &state.repos().agent_executions;
    let row = repo
        .get_agent_execution(id)
        .await?
        .ok_or(AppError::not_found("Agent execution"))?;
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
pub async fn list_execution_messages(
    State(state): State<AppState>,
    _auth: auth_utils::AuthUser,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<ExecutionMessageResponse>>, AppError> {
    let repo = &state.repos().agent_executions;
    // Verify execution exists
    repo.get_agent_execution(id)
        .await?
        .ok_or(AppError::not_found("Agent execution"))?;
    let rows = repo.list_execution_messages(id).await?;
    Ok(Json(
        rows.into_iter()
            .map(ExecutionMessageResponse::from)
            .collect(),
    ))
}

/// GET /api/agent-executions/:id/messages/:stream_id/stream — SSE stream for agent response.
///
/// Streams the agent's response tokens as Server-Sent Events. The `stream_id` is
/// returned by `POST /api/agent-executions/:id/messages`.
#[utoipa::path(
    get,
    path = "/api/agent-executions/{id}/messages/{stream_id}/stream",
    tag = "Agent Executions",
    params(
        ("id" = Uuid, Path, description = "Agent execution ID"),
        ("stream_id" = Uuid, Path, description = "Stream ID from send message response")
    ),
    responses(
        (status = 200, description = "SSE event stream")
    )
)]
pub async fn execution_message_stream(
    State(state): State<AppState>,
    Path((_execution_id, stream_id)): Path<(Uuid, Uuid)>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    // Reuse the same streaming infrastructure as chat — keyed by stream_id
    let stream = async_stream::stream! {
        let (buffered, mut rx, already_done) = state.get_response_stream(stream_id);

        // Replay any buffered chunks
        for chunk in buffered {
            match chunk {
                StreamChunk::Token(text) => {
                    yield Ok(Event::default().event("token").data(serde_json::to_string(&text).unwrap_or(text)));
                }
                StreamChunk::ToolStart { name, tool_id } => {
                    let data = format!(r#"{{"name":"{}","id":"{}"}}"#, name, tool_id);
                    yield Ok(Event::default().event("tool_start").data(data));
                }
                StreamChunk::ToolEnd { name, tool_id } => {
                    let data = format!(r#"{{"name":"{}","id":"{}"}}"#, name, tool_id);
                    yield Ok(Event::default().event("tool_end").data(data));
                }
                StreamChunk::DocUpdate { doc_id, title } => {
                    let data = format!(r#"{{"doc_id":"{}","title":"{}"}}"#, doc_id, title);
                    yield Ok(Event::default().event("doc_update").data(data));
                }
                StreamChunk::Done => {
                    yield Ok(Event::default().event("done").data(""));
                    return;
                }
                StreamChunk::Error(e) => {
                    yield Ok(Event::default().event("error").data(e));
                    return;
                }
            }
        }

        if already_done {
            yield Ok(Event::default().event("done").data(""));
            return;
        }

        // Listen for live chunks
        loop {
            match rx.recv().await {
                Ok(chunk) => {
                    match chunk {
                        StreamChunk::Token(text) => {
                            yield Ok(Event::default().event("token").data(serde_json::to_string(&text).unwrap_or(text)));
                        }
                        StreamChunk::ToolStart { name, tool_id } => {
                            let data = format!(r#"{{"name":"{}","id":"{}"}}"#, name, tool_id);
                            yield Ok(Event::default().event("tool_start").data(data));
                        }
                        StreamChunk::ToolEnd { name, tool_id } => {
                            let data = format!(r#"{{"name":"{}","id":"{}"}}"#, name, tool_id);
                            yield Ok(Event::default().event("tool_end").data(data));
                        }
                        StreamChunk::DocUpdate { doc_id, title } => {
                            let data = format!(r#"{{"doc_id":"{}","title":"{}"}}"#, doc_id, title);
                            yield Ok(Event::default().event("doc_update").data(data));
                        }
                        StreamChunk::Done => {
                            yield Ok(Event::default().event("done").data(""));
                            break;
                        }
                        StreamChunk::Error(e) => {
                            yield Ok(Event::default().event("error").data(e));
                            break;
                        }
                    }
                }
                Err(broadcast::error::RecvError::Closed) => {
                    break;
                }
                Err(broadcast::error::RecvError::Lagged(_)) => {
                    continue;
                }
            }
        }
    };

    Sse::new(stream)
}

/// POST /api/agent-executions/:id/approve — approve an interactive agent execution.
///
/// With no `structured_output` body → approve as-is (main output used).
/// With `structured_output` → approve with changes (revised output used downstream).
///
/// After approval, if all interactive reviews for the workflow step are complete,
/// the paused DAG is resumed in the background.
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
) -> Result<Json<AgentExecutionResponse>, AppError> {
    let repo = &state.repos().agent_executions;
    let ae = repo
        .get_agent_execution(id)
        .await?
        .ok_or(AppError::not_found("Agent execution"))?;

    if !ae.is_interactive || ae.status != "awaiting_user" {
        return Err(AppError::bad_request(
            "Execution is not interactive or not awaiting user approval",
        ));
    }

    // Update status to completed, optionally with revised structured_output
    let updated = repo
        .update_agent_execution_status(id, "completed", ae.output.clone(), req.structured_output)
        .await?;

    // Check if we should resume the paused DAG
    if let Some(step_id) = ae.workflow_step_id {
        let all_approved = match repo.list_interactive_executions_for_step(step_id).await {
            Ok(interactive_execs) => interactive_execs
                .iter()
                .all(|iae| iae.status == "completed" || iae.id == id),
            Err(_) => false,
        };

        if all_approved {
            // Build the step output from the approved execution
            let step_output = crate::server::hub::dag::StepOutput {
                variable_name: String::new(), // Filled by resume logic from step metadata
                raw_output: updated.output.clone().unwrap_or_default(),
                structured_output: updated.structured_output.clone(),
            };

            let state_clone = state.clone();
            tokio::spawn(async move {
                if let Err(e) =
                    hub::dag::resume_dag_from_approval(&state_clone, step_id, step_output).await
                {
                    error!("DAG resume failed after approval: {}", e);
                }
            });
        }
    }

    Ok(Json(AgentExecutionResponse::from(updated)))
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct SetExemplaryRequest {
    pub is_exemplary: bool,
}

/// PUT /api/agent-executions/:id/exemplary — mark an execution as exemplary (or remove the mark).
///
/// Exemplary executions are used as few-shot examples in future runs of the
/// same agent and workflow step.
#[utoipa::path(
    put,
    path = "/api/agent-executions/{id}/exemplary",
    tag = "Agent Executions",
    security(("bearer_auth" = [])),
    params(("id" = Uuid, Path, description = "Agent execution ID")),
    request_body = SetExemplaryRequest,
    responses(
        (status = 200, description = "Exemplary flag updated", body = AgentExecutionResponse),
        (status = 404, description = "Not found")
    )
)]
pub async fn set_exemplary(
    State(state): State<AppState>,
    _auth: auth_utils::AuthUser,
    Path(id): Path<Uuid>,
    Json(req): Json<SetExemplaryRequest>,
) -> Result<Json<AgentExecutionResponse>, AppError> {
    let repo = &state.repos().agent_executions;
    // Verify execution exists
    repo.get_agent_execution(id)
        .await?
        .ok_or(AppError::not_found("Agent execution"))?;

    let updated = repo.set_execution_exemplary(id, req.is_exemplary).await?;

    Ok(Json(AgentExecutionResponse::from(updated)))
}

#[cfg(test)]
mod tests;
