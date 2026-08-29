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
    pub reasoning: Option<String>,
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

    let entries = rows.into_iter().flat_map(map_row_to_entries).collect();

    Ok(TimelineResponse {
        entries,
        has_more,
        next_cursor,
    })
}

/// Marker the engine writes ahead of each tool call in an assistant row.
const TOOL_USE_PREFIX: &str = "tool_use: ";

/// One timeline entry's worth of an `execution_messages` row, before it is
/// given an id and the row's shared metadata.
struct Segment {
    kind: TimelineEntryKind,
    content: String,
    tool_name: Option<String>,
}

/// Split one row into the entries it actually represents.
///
/// An assistant row can hold a text block *and* several `tool_use:` lines: the
/// engine joins every content block of a turn into a single row. The frontend
/// pairs calls against results, so a row holding N calls has to yield N call
/// entries — when it yielded one, the call counter fell behind the result
/// counter and every later card rendered a different call's result.
fn map_row_to_entries(row: TimelineRow) -> Vec<TimelineEntry> {
    let segments = split_row(&row);

    // A row with several calls has no id to give any one of them, so the
    // frontend falls back to positional pairing — which is correct once the
    // counts agree. Only a row carrying exactly one call can hand its id over.
    let call_count = segments
        .iter()
        .filter(|s| matches!(s.kind, TimelineEntryKind::ToolCall))
        .count();

    segments
        .into_iter()
        .enumerate()
        .map(|(i, seg)| {
            let tool_call_id = match seg.kind {
                TimelineEntryKind::ToolResult => row.tool_call_id.clone(),
                TimelineEntryKind::ToolCall if call_count == 1 => row.tool_call_id.clone(),
                _ => None,
            };

            TimelineEntry {
                // Derived rather than random so the id is stable across
                // refetches — the frontend uses it as a list key, and a fresh
                // uuid per poll would remount every card.
                id: if i == 0 {
                    row.id
                } else {
                    Uuid::new_v5(&row.id, i.to_string().as_bytes())
                },
                ts: row.ts,
                kind: seg.kind,
                step_id: row.step_id,
                step_name: row.step_name.clone(),
                agent_name: row.agent_name.clone(),
                agent_execution_id: row.agent_execution_id,
                content: seg.content,
                reasoning: row.reasoning.clone(),
                tool_name: seg.tool_name,
                tool_call_id,
                input_tokens: row.input_tokens,
                output_tokens: row.output_tokens,
            }
        })
        .collect()
}

/// Break a row into segments by role, splitting assistant rows on `tool_use:`.
fn split_row(row: &TimelineRow) -> Vec<Segment> {
    let whole = |kind| {
        vec![Segment {
            kind,
            content: row.content.clone(),
            tool_name: None,
        }]
    };

    match row.role.as_str() {
        "system" => whole(TimelineEntryKind::SystemPrompt),
        "user" => whole(TimelineEntryKind::UserMessage),
        "tool" => whole(TimelineEntryKind::ToolResult),
        "assistant" => {
            let segments = split_assistant(&row.content);
            // An empty assistant row still deserves to be visible.
            if segments.is_empty() {
                whole(TimelineEntryKind::AssistantMessage)
            } else {
                segments
            }
        }
        _ => whole(TimelineEntryKind::AssistantMessage),
    }
}

/// Split an assistant row into prose and one segment per tool call.
///
/// Each call is one line, because the engine formats the input with `Display`
/// on a `serde_json::Value` and JSON escapes any newline inside a string. The
/// marker is matched per line rather than at the start of the row: a turn that
/// opens with prose and then calls tools was previously classified as plain
/// text, contributing zero call entries against several results.
fn split_assistant(content: &str) -> Vec<Segment> {
    let mut segments = Vec::new();
    let mut prose: Vec<&str> = Vec::new();

    let flush = |prose: &mut Vec<&str>, out: &mut Vec<Segment>| {
        let text = prose.join("\n");
        prose.clear();
        if !text.trim().is_empty() {
            out.push(Segment {
                kind: TimelineEntryKind::AssistantMessage,
                content: text,
                tool_name: None,
            });
        }
    };

    for line in content.lines() {
        match line.strip_prefix(TOOL_USE_PREFIX) {
            Some(rest) => {
                flush(&mut prose, &mut segments);
                let (name, payload) = rest.split_once(' ').unwrap_or((rest, ""));
                segments.push(Segment {
                    kind: TimelineEntryKind::ToolCall,
                    // The payload alone, so the frontend can parse it as JSON
                    // instead of falling back to showing the raw line.
                    content: payload.to_string(),
                    tool_name: Some(name.to_string()),
                });
            }
            None => prose.push(line),
        }
    }
    flush(&mut prose, &mut segments);

    segments
}
