//! Timeline service: unified execution debug stream for workflow runs.
//!
//! Joins agent_executions + execution_messages + workflow_steps into a flat,
//! chronologically ordered timeline. Supports cursor-based pagination for
//! scrolling backwards through history.

use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::db::traits::AgentExecutionRepo;
use crate::db::TimelineRow;

use super::ServiceError;

#[cfg(test)]
mod tests;

/// A single entry in the execution timeline.
#[derive(Debug, Clone, Serialize)]
pub struct TimelineEntry {
    pub id: Uuid,
    pub ts: DateTime<Utc>,
    pub kind: TimelineEntryKind,
    /// Needed by the frontend to group traces by node; `step_name` alone is
    /// ambiguous when two steps share a name.
    pub step_id: Option<Uuid>,
    pub step_name: Option<String>,
    pub agent_name: Option<String>,
    pub agent_execution_id: Uuid,
    pub content: String,
    pub tool_name: Option<String>,
    pub tool_call_id: Option<String>,
    pub input_tokens: i64,
    pub output_tokens: i64,
}

/// Categorized timeline entry kind, derived from execution_message role.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TimelineEntryKind {
    SystemPrompt,
    UserMessage,
    AssistantMessage,
    ToolCall,
    ToolResult,
}

/// Paginated timeline response.
#[derive(Debug, Clone, Serialize)]
pub struct TimelineResponse {
    pub entries: Vec<TimelineEntry>,
    pub has_more: bool,
    pub next_cursor: Option<String>,
}

/// Fetch the execution timeline for a workflow run with cursor-based pagination.
///
/// Returns entries in chronological order (oldest first) for natural reading.
/// The DB query fetches newest-first for pagination, then we reverse.
pub async fn get_execution_timeline(
    repo: &dyn AgentExecutionRepo,
    workflow_execution_id: Uuid,
    limit: i64,
    before: Option<DateTime<Utc>>,
) -> Result<TimelineResponse, ServiceError> {
    let fetch_limit = limit + 1; // Fetch one extra to detect has_more

    let mut rows = repo
        .list_execution_timeline(workflow_execution_id, fetch_limit, before)
        .await?;

    let has_more = rows.len() as i64 > limit;
    if has_more {
        rows.truncate(limit as usize);
    }

    // Rows come DESC from DB — get the cursor before reversing
    let next_cursor = if has_more {
        rows.last().map(|r| r.ts.to_rfc3339())
    } else {
        None
    };

    // Reverse to chronological order (oldest first)
    rows.reverse();

    let entries = rows.into_iter().map(map_row_to_entry).collect();

    Ok(TimelineResponse {
        entries,
        has_more,
        next_cursor,
    })
}

fn map_row_to_entry(row: TimelineRow) -> TimelineEntry {
    let (kind, tool_name, tool_call_id) = classify_message(&row);

    TimelineEntry {
        id: row.id,
        ts: row.ts,
        kind,
        step_id: row.step_id,
        step_name: row.step_name,
        agent_name: row.agent_name,
        agent_execution_id: row.agent_execution_id,
        content: row.content,
        tool_name,
        tool_call_id: row.tool_call_id.clone().or(tool_call_id),
        input_tokens: row.input_tokens,
        output_tokens: row.output_tokens,
    }
}

/// Classify message role into timeline entry kind.
/// Tool calls are detected from assistant messages containing "tool_use:" prefix.
fn classify_message(row: &TimelineRow) -> (TimelineEntryKind, Option<String>, Option<String>) {
    match row.role.as_str() {
        "system" => (TimelineEntryKind::SystemPrompt, None, None),
        "user" => (TimelineEntryKind::UserMessage, None, None),
        "assistant" => {
            // Check if this is a tool call message (format: "tool_use: name {json}")
            if row.content.starts_with("tool_use: ") {
                let rest = &row.content["tool_use: ".len()..];
                let tool_name = rest.split_whitespace().next().map(|s| s.to_string());
                (TimelineEntryKind::ToolCall, tool_name, None)
            } else {
                (TimelineEntryKind::AssistantMessage, None, None)
            }
        }
        "tool" => {
            // tool_call_id is already on the row
            (TimelineEntryKind::ToolResult, None, None)
        }
        _ => (TimelineEntryKind::AssistantMessage, None, None),
    }
}
