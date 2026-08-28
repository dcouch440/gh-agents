//! Shared utilities for tool modules.
//!
//! Functions here are used across multiple archetype tool modules
//! (node_assistant, workforce).

use serde_json::{json, Value};
use uuid::Uuid;

use crate::db::traits::WorkflowRepo;
use crate::db::WorkflowStepRow;

mod tests;

// =========================================================================
// Parameter extraction helpers
// =========================================================================

/// Extract a required string parameter from tool input.
///
/// Returns the string slice on success, or a JSON error value on failure.
pub(crate) fn require_str<'a>(input: &'a Value, field: &str) -> Result<&'a str, Value> {
    input[field]
        .as_str()
        .ok_or_else(|| json!({ "error": format!("Missing required parameter: {}", field) }))
}

/// Extract a required string parameter and parse it as a UUID.
///
/// Returns the UUID on success, or a JSON error value on failure.
pub(crate) fn require_uuid(input: &Value, field: &str) -> Result<Uuid, Value> {
    let s = require_str(input, field)?;
    Uuid::parse_str(s).map_err(|_| json!({ "error": format!("Invalid UUID: {}", s) }))
}

/// Build a standard error JSON object for tool responses.
pub(crate) fn error_json(message: impl Into<String>) -> Value {
    json!({ "error": message.into() })
}

// =========================================================================
// Allow-list enforcement
// =========================================================================

/// Whether `name` is permitted for this agent.
///
/// `None` means no allow-list was supplied and every tool is permitted; the
/// caller has already narrowed the tool set some other way.
///
/// This is the single definition of a security check that was previously
/// copy-pasted into each dispatch branch, where the copies could drift.
pub(crate) fn is_tool_allowed(name: &str, allowed: Option<&[String]>) -> bool {
    match allowed {
        Some(list) => list.iter().any(|t| t == name),
        None => true,
    }
}

/// The standard refusal returned when a tool is outside the agent's allow-list.
pub(crate) fn tool_not_allowed_error(name: &str) -> Value {
    error_json(format!("Tool '{}' is not allowed for this agent", name))
}

/// Load a workflow step by ID, returning a JSON error if not found.
pub(crate) async fn load_step_or_error(
    repo: &dyn WorkflowRepo,
    step_id: Uuid,
) -> Result<WorkflowStepRow, Value> {
    match repo.get_step(step_id).await {
        Ok(Some(s)) => Ok(s),
        Ok(None) => Err(json!({ "error": "Step not found" })),
        Err(e) => Err(json!({ "error": format!("Failed to load step: {}", e) })),
    }
}

/// Classify a step's content status for the port manifest.
///
/// - `context` with non-empty `prompt_template` -> "populated" (include preview + word count)
/// - `context` with empty `prompt_template` -> "empty"
/// - All other execution modes -> "pending"
pub(crate) fn classify_content_status(
    step: &crate::db::WorkflowStepRow,
) -> (&'static str, Option<String>, Option<usize>) {
    if step.execution_mode == "context" {
        let content = &step.prompt_template;
        if content.trim().is_empty() {
            ("empty", None, None)
        } else {
            let preview: String = content.chars().take(500).collect();
            let word_count = content.split_whitespace().count();
            ("populated", Some(preview), Some(word_count))
        }
    } else {
        ("pending", None, None)
    }
}
