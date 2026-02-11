//! Static output builder for the documenter pipeline.

use serde_json::Value as JsonValue;

/// Build a structured output JSON summarising document results.
///
/// Used by tests and the executor to create the final `StepOutput`.
pub fn build_documents_output(statuses: Vec<JsonValue>) -> JsonValue {
    serde_json::json!({ "documents": statuses })
}
