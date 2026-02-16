//! Upstream content normalization for belief extraction.
//!
//! Collects and normalizes content from all upstream steps, adapting to
//! each step's execution mode (context, workforce, room, etc.).

use std::collections::HashMap;

use tracing::warn;
use uuid::Uuid;

use crate::db::WorkflowStepRow;
use crate::server::hub::dag::get_parent_steps;
use crate::server::state::AppState;
use crate::types::StepExecutionEnvelope;

/// Maximum characters for a single source content block.
const MAX_SOURCE_CONTENT_CHARS: usize = 100_000;

/// A single source of content extracted from an upstream step.
pub struct UpstreamSource {
    /// Display name for this source.
    pub title: String,
    /// The execution mode of the source step.
    pub source_type: String,
    /// The text content to feed to the extractor.
    pub content: String,
    /// Source step ID for belief attribution.
    pub source_step_id: Uuid,
    /// Source step name for belief attribution.
    pub source_step_name: String,
    /// Document def ID if this came from a documenter doc.
    pub source_document_def_id: Option<Uuid>,
    /// Document title if from a documenter doc.
    pub source_document_title: Option<String>,
}

/// Collect and normalize content from all upstream steps.
///
/// Uses `get_parent_steps()` to find direct upstream step IDs, then
/// normalizes each by `execution_mode`.
pub async fn collect_upstream_sources(
    step: &WorkflowStepRow,
    edges: &[crate::db::WorkflowStepEdgeRow],
    steps: &[WorkflowStepRow],
    completed_envelopes: &HashMap<Uuid, StepExecutionEnvelope>,
    _state: &AppState,
) -> Vec<UpstreamSource> {
    let step_map: HashMap<Uuid, &WorkflowStepRow> = steps.iter().map(|s| (s.id, s)).collect();
    let parent_ids = get_parent_steps(step.id, edges);
    let mut sources = Vec::new();

    for parent_id in parent_ids {
        let parent_step = match step_map.get(&parent_id) {
            Some(s) => *s,
            None => continue,
        };
        let envelope = match completed_envelopes.get(&parent_id) {
            Some(e) => e,
            None => continue,
        };
        let data = match &envelope.data {
            Some(d) => d,
            None => continue,
        };

        let step_name = parent_step
            .name
            .clone()
            .unwrap_or_else(|| parent_id.to_string());

        match parent_step.execution_mode.as_str() {
            "context" => {
                if let serde_json::Value::String(content) = data {
                    if !content.is_empty() {
                        sources.push(UpstreamSource {
                            title: step_name.clone(),
                            source_type: "context".to_string(),
                            content: truncate(content),
                            source_step_id: parent_id,
                            source_step_name: step_name,
                            source_document_def_id: None,
                            source_document_title: None,
                        });
                    }
                }
            }
            "workforce" => {
                // Iterate JSON object keys — one source per agent
                if let serde_json::Value::Object(map) = data {
                    for (agent_key, agent_output) in map {
                        let content = match agent_output {
                            serde_json::Value::String(s) => s.clone(),
                            other => serde_json::to_string_pretty(other).unwrap_or_default(),
                        };
                        if !content.is_empty() {
                            sources.push(UpstreamSource {
                                title: format!("{} > {}", step_name, agent_key),
                                source_type: "workforce".to_string(),
                                content: truncate(&content),
                                source_step_id: parent_id,
                                source_step_name: step_name.clone(),
                                source_document_def_id: None,
                                source_document_title: None,
                            });
                        }
                    }
                }
            }
            _ => {
                // room, single, for_each, and any other mode
                let content = match data {
                    serde_json::Value::String(s) => s.clone(),
                    other => serde_json::to_string_pretty(other).unwrap_or_default(),
                };
                if !content.is_empty() {
                    sources.push(UpstreamSource {
                        title: step_name.clone(),
                        source_type: parent_step.execution_mode.clone(),
                        content: truncate(&content),
                        source_step_id: parent_id,
                        source_step_name: step_name,
                        source_document_def_id: None,
                        source_document_title: None,
                    });
                }
            }
        }
    }

    if sources.is_empty() {
        warn!(
            step_id = %step.id,
            "No upstream sources found for belief capture"
        );
    }

    sources
}

/// Truncate content to `MAX_SOURCE_CONTENT_CHARS`.
fn truncate(content: &str) -> String {
    if content.len() <= MAX_SOURCE_CONTENT_CHARS {
        content.to_string()
    } else {
        let mut s = content[..MAX_SOURCE_CONTENT_CHARS].to_string();
        s.push_str("\n\n[Content truncated]");
        s
    }
}
