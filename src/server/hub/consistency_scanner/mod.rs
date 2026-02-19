//! Consistency scanner — detects stale references across workflow nodes.
//!
//! When an entity (document def, roster agent, etc.) is deleted from one node,
//! other nodes' assistant notes may still reference it. This module uses Haiku
//! to scan for inconsistencies and broadcasts issues via WebSocket.
//!
//! The scanner is debounced: rapid deletions accumulate in a `DashMap` on
//! `AppState` and are processed as a single batch after a 2-second delay.
//!
//! **DISABLED**: The scanner is currently detached from all call sites. The
//! dispatcher updates assistant notes asynchronously, so when the scanner runs
//! after a deletion the notes may not yet reflect the changes — leading to
//! false-positive stale-reference reports. Re-enable once note updates are
//! guaranteed to land before the scan fires.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::llm::{
    LLMRequest, Message as LlmMessage,
};
use crate::server::state::AppState;
use crate::server::ws::events::{WorkflowEvent, WorkflowEventKind};

#[cfg(test)]
mod tests;

// ── Types ──────────────────────────────────────────────────────────────────

/// A deleted item that should be checked for stale references in notes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeletedItem {
    pub item_type: DeletedItemType,
    pub name: String,
    pub id: Uuid,
    pub source_step_id: Uuid,
    pub source_step_name: String,
}

/// The kind of entity that was deleted.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeletedItemType {
    RosterAgent,
}

/// A consistency issue found by the scanner.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsistencyIssue {
    pub step_id: Uuid,
    pub step_name: String,
    pub description: String,
    pub severity: String,
    pub deleted_item_name: String,
    pub deleted_item_type: String,
}

// ── Constants ──────────────────────────────────────────────────────────────

const SCAN_DEBOUNCE_MS: u64 = 2000;
const MAX_TOKENS_SCAN: u32 = 1024;

const SYSTEM_PROMPT: &str = r#"You detect stale references in workflow configuration notes.

You receive two sections:
1. <deletions> — items that were just deleted from the workflow
2. <notes> — assistant notes from every step in the workflow

Your job: identify any notes that still reference a deleted item. A reference can be:
- An exact name match (e.g. "Security Scanner" matching a deleted agent named "Security Scanner")
- A UUID match (e.g. agent_id: abc-123)
- A clear semantic reference (e.g. "the security scanner" matching deleted agent "Security Scanner")

Return a JSON object:
```json
{
  "issues": [
    {
      "step_id": "uuid-of-step-with-stale-reference",
      "step_name": "Name of the step",
      "description": "Notes reference deleted agent 'Security Scanner'",
      "severity": "warning",
      "deleted_item_name": "Security Scanner",
      "deleted_item_type": "roster_agent"
    }
  ]
}
```

If no stale references are found, return: {"issues": []}

Be precise. Only flag actual references, not coincidental word overlap."#;

// ── Public API ─────────────────────────────────────────────────────────────

/// Schedule a debounced consistency scan for a workflow.
///
/// Accumulates the deleted item in the AppState `DashMap`, then spawns a
/// task that sleeps for `SCAN_DEBOUNCE_MS` before draining and scanning.
/// Multiple rapid calls coalesce into a single scan.
pub fn schedule_consistency_scan(state: AppState, workflow_id: Uuid, item: DeletedItem) {
    state
        .pending_scan_items()
        .entry(workflow_id)
        .or_default()
        .push(item);

    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(SCAN_DEBOUNCE_MS)).await;

        // Drain accumulated items — only one task gets them
        let items = state
            .pending_scan_items()
            .remove(&workflow_id)
            .map(|(_, v)| v)
            .unwrap_or_default();

        if items.is_empty() {
            return;
        }

        if let Err(e) = run_consistency_scan(&state, workflow_id, &items).await {
            tracing::warn!("Consistency scan failed for workflow {workflow_id}: {e}");
        }
    });
}

// ── Scanner ────────────────────────────────────────────────────────────────

async fn run_consistency_scan(
    state: &AppState,
    workflow_id: Uuid,
    deleted_items: &[DeletedItem],
) -> Result<(), anyhow::Error> {
    // 1. Load all assistant notes for the workflow
    let all_notes = state
        .repos()
        .workflows
        .get_all_assistant_notes_for_workflow(workflow_id)
        .await?;

    if all_notes.is_empty() {
        return Ok(());
    }

    // 2. Build prompt input
    let prompt_input = format_scan_input(deleted_items, &all_notes);

    // 3. Call utility model
    let client = crate::llm::create_utility_client()?;

    let request = LLMRequest::new(
        crate::constants::MODEL_TIER3,
        vec![LlmMessage::user(prompt_input)],
    )
    .with_system(SYSTEM_PROMPT)
    .with_max_tokens(MAX_TOKENS_SCAN);

    let response = client.send_message(request).await?;

    // 4. Parse response
    let issues = parse_scan_output(&response.content);

    // 5. Broadcast issues (even if empty — clears stale frontend state)
    state.broadcast_workflow(WorkflowEvent {
        run_id: None,
        workflow_id,
        user_id: None,
        kind: WorkflowEventKind::ConsistencyIssues { issues },
    });

    Ok(())
}

// ── Formatting ─────────────────────────────────────────────────────────────

pub(crate) fn format_scan_input(
    deleted_items: &[DeletedItem],
    all_notes: &[(Uuid, Option<String>, String, String)],
) -> String {
    let mut out = String::from("<deletions>\n");
    for item in deleted_items {
        let type_label = match item.item_type {
            DeletedItemType::RosterAgent => "Agent",
        };
        out.push_str(&format!(
            "- {} \"{}\" (id: {}) from step \"{}\" (step_id: {})\n",
            type_label, item.name, item.id, item.source_step_name, item.source_step_id
        ));
    }
    out.push_str("</deletions>\n\n<notes>\n");

    for (step_id, step_name, _mode, content) in all_notes {
        let name = step_name.as_deref().unwrap_or("(unnamed)");
        out.push_str(&format!(
            "[{} (step_id: {})]\n{}\n\n",
            name, step_id, content
        ));
    }
    out.push_str("</notes>");
    out
}

// ── Parsing ────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct ScanOutput {
    issues: Vec<ConsistencyIssue>,
}

pub(crate) fn parse_scan_output(content: &str) -> Vec<ConsistencyIssue> {
    use crate::server::hub::protocols::json_utils::parse_structured_output;

    match parse_structured_output(content) {
        Some(json) => match serde_json::from_value::<ScanOutput>(json) {
            Ok(output) => output.issues,
            Err(e) => {
                tracing::warn!("Failed to parse consistency scan output: {e}");
                vec![]
            }
        },
        None => {
            tracing::warn!("No structured JSON in consistency scan response");
            vec![]
        }
    }
}
