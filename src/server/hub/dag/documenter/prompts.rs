//! Static output builder for the documenter pipeline.

#[cfg(test)]
use serde_json::Value as JsonValue;

/// Build a structured output JSON summarising document results.
///
/// Used by tests to verify the final `StepOutput` shape.
#[cfg(test)]
pub fn build_documents_output(statuses: Vec<JsonValue>) -> JsonValue {
    serde_json::json!({ "documents": statuses })
}
