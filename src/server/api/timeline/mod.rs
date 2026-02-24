//! Execution timeline API — unified debug stream for workflow runs.

use axum::extract::{Path, Query, State};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::AppError;
use crate::server::auth as auth_utils;
use crate::server::services::timeline as svc;
use crate::server::state::AppState;

#[cfg(test)]
mod tests;

/// Query parameters for the timeline endpoint.
#[derive(Debug, Deserialize, utoipa::IntoParams)]
pub struct TimelineQuery {
    /// Maximum number of entries to return (default 50, max 200).
    pub limit: Option<i64>,
    /// Cursor: only return entries before this timestamp (RFC 3339).
    pub before: Option<DateTime<Utc>>,
}

/// API response for the execution timeline.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct TimelineApiResponse {
    pub entries: Vec<TimelineEntryResponse>,
    pub has_more: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

/// A single entry in the execution timeline.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct TimelineEntryResponse {
    pub id: Uuid,
    pub ts: DateTime<Utc>,
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub step_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_name: Option<String>,
    pub agent_execution_id: Uuid,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    pub input_tokens: i64,
    pub output_tokens: i64,
}

impl From<svc::TimelineEntry> for TimelineEntryResponse {
    fn from(e: svc::TimelineEntry) -> Self {
        Self {
            id: e.id,
            ts: e.ts,
            kind: match e.kind {
                svc::TimelineEntryKind::SystemPrompt => "system_prompt".to_string(),
                svc::TimelineEntryKind::UserMessage => "user_message".to_string(),
                svc::TimelineEntryKind::AssistantMessage => "assistant_message".to_string(),
                svc::TimelineEntryKind::ToolCall => "tool_call".to_string(),
                svc::TimelineEntryKind::ToolResult => "tool_result".to_string(),
            },
            step_name: e.step_name,
            agent_name: e.agent_name,
            agent_execution_id: e.agent_execution_id,
            content: e.content,
            tool_name: e.tool_name,
            tool_call_id: e.tool_call_id,
            input_tokens: e.input_tokens,
            output_tokens: e.output_tokens,
        }
    }
}

/// Fetch the execution timeline for a workflow run.
///
/// Returns a flat, chronologically ordered debug stream of all messages,
/// tool calls, and system prompts across all agents in the execution.
#[utoipa::path(
    get,
    path = "/api/workflow-executions/{id}/timeline",
    tag = "Timeline",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "Workflow execution ID"),
        TimelineQuery,
    ),
    responses(
        (status = 200, description = "Execution timeline", body = TimelineApiResponse),
    )
)]
pub async fn get_execution_timeline(
    State(state): State<AppState>,
    _auth: auth_utils::AuthUser,
    Path(id): Path<Uuid>,
    Query(query): Query<TimelineQuery>,
) -> Result<Json<TimelineApiResponse>, AppError> {
    let limit = query.limit.unwrap_or(50).min(200).max(1);

    let response = svc::get_execution_timeline(
        state.repos().agent_executions.as_ref(),
        id,
        limit,
        query.before,
    )
    .await?;

    Ok(Json(TimelineApiResponse {
        entries: response.entries.into_iter().map(Into::into).collect(),
        has_more: response.has_more,
        next_cursor: response.next_cursor,
    }))
}
