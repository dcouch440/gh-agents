//! Shared utilities for tool modules.
//!
//! Functions here are used across multiple archetype tool modules
//! (documenter, belief_capture, room_config, task_force).

mod tests;

/// Classify a step's content status for the port manifest.
///
/// - `context` with non-empty `prompt_template` -> "populated" (include preview + word count)
/// - `context` with empty `prompt_template` -> "empty"
/// - All other execution modes -> "pending"
pub fn classify_content_status(
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
