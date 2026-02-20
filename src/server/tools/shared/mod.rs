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

/// Extract a required array parameter from tool input.
///
/// Returns the array reference on success, or a JSON error value on failure.
pub(crate) fn require_array<'a>(input: &'a Value, field: &str) -> Result<&'a Vec<Value>, Value> {
    input[field]
        .as_array()
        .ok_or_else(|| json!({ "error": format!("Missing required parameter: {} (array)", field) }))
}

/// Extract a required i64 parameter from tool input.
///
/// Returns the integer on success, or a JSON error value on failure.
pub(crate) fn require_i64(input: &Value, field: &str) -> Result<i64, Value> {
    input[field]
        .as_i64()
        .ok_or_else(|| json!({ "error": format!("Missing required parameter: {}", field) }))
}

/// Extract a required string parameter and parse it as a UUID.
///
/// Returns the UUID on success, or a JSON error value on failure.
pub(crate) fn require_uuid(input: &Value, field: &str) -> Result<Uuid, Value> {
    let s = require_str(input, field)?;
    Uuid::parse_str(s).map_err(|_| json!({ "error": format!("Invalid UUID: {}", s) }))
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
