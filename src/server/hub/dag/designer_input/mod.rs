//! Archetype-agnostic input types and shared utilities for the Agent Designer.
//!
//! Each archetype (task_force, documenter, room) provides a formatter function
//! that converts its domain-specific configuration into a generic `DesignerInput`.
//! The Agent Designer consumes `DesignerInput` to generate optimized prompt pairs.

pub mod documenter;
pub mod room;
pub mod task_force;
mod tests;

use std::collections::{HashMap, HashSet};

use uuid::Uuid;

use crate::db::WorkflowStepRow;
use crate::types::StepExecutionEnvelope;

// ── Generic input types ─────────────────────────────────────────────────────

/// Archetype-agnostic input for the Agent Designer.
/// Each archetype builds this from its own configuration.
#[derive(Debug, Clone)]
pub struct DesignerInput {
    /// Which archetype is requesting design ("task_force", "documenter", "room").
    pub archetype: String,

    /// High-level description of what this execution does.
    pub context_description: String,

    /// The agents that need prompt pairs generated.
    pub agents: Vec<AgentDefinition>,

    /// Upstream context available to all agents.
    pub upstream: Vec<UpstreamContext>,

    /// Tool descriptions for capabilities these agents may use.
    pub available_tools: Vec<ToolDescription>,

    /// Archetype-specific guidance for the designer.
    /// Extra instructions that vary by archetype (e.g., documenter phase info,
    /// room interaction mode, task force failure mode).
    pub archetype_guidance: String,
}

/// One agent that needs a prompt pair designed.
#[derive(Debug, Clone)]
pub struct AgentDefinition {
    /// Stable identifier — roster entry ID, document def ID, room member ID, etc.
    pub id: String,
    /// Human-readable name for the agent.
    pub name: String,
    /// What this agent does.
    pub role: String,
    /// What tools/capabilities this agent has access to.
    pub capabilities: Vec<String>,
    /// Execution order relative to other agents (0-indexed).
    pub execution_order: i32,
    /// Extra context specific to this agent (e.g., strategist's research_strategy,
    /// room member's perspective, belief subset).
    pub additional_context: String,
}

/// Upstream content available to the agents.
#[derive(Debug, Clone)]
pub struct UpstreamContext {
    /// Name of the upstream source.
    pub source_name: String,
    /// Type of upstream (context, documenter, task_force, belief_capture, room).
    pub source_type: String,
    /// The actual content (may be truncated for context budget).
    pub content: String,
}

/// Description of an available tool/capability.
#[derive(Debug, Clone)]
pub struct ToolDescription {
    pub name: String,
    pub description: String,
}

/// A room member for the designer input, decoupled from specific DB row types.
/// At runtime, `id` is `agent_id.to_string()`. At design-time, it's the member row ID.
#[derive(Debug, Clone)]
pub struct RoomDesignerMember {
    pub id: String,
    pub name: String,
    pub role: String,
    pub perspective: String,
}

// ── Shared utilities ────────────────────────────────────────────────────────

/// Convert step execution envelopes into generic upstream context.
/// Used by all archetype formatters.
pub fn format_envelopes_as_upstream(
    envelopes: &HashMap<Uuid, StepExecutionEnvelope>,
    steps: &[WorkflowStepRow],
) -> Vec<UpstreamContext> {
    if envelopes.is_empty() {
        return vec![UpstreamContext {
            source_name: "none".to_string(),
            source_type: "none".to_string(),
            content: "No upstream outputs available. This is the first step in the workflow."
                .to_string(),
        }];
    }

    let context_step_ids: HashSet<Uuid> = steps
        .iter()
        .filter(|s| s.execution_mode == "context")
        .map(|s| s.id)
        .collect();

    envelopes
        .iter()
        .map(|(step_id, env)| {
            let data_str = env.data.as_ref().map(|d| d.to_string()).unwrap_or_default();
            let source_type = if context_step_ids.contains(step_id) {
                "context"
            } else {
                "step"
            };
            UpstreamContext {
                source_name: step_id.to_string(),
                source_type: source_type.to_string(),
                content: truncate_for_context(&data_str, 4000).to_string(),
            }
        })
        .collect()
}

/// Convert capability names into tool descriptions.
pub fn build_tool_descriptions(capabilities: &[String]) -> Vec<ToolDescription> {
    capabilities
        .iter()
        .map(|cap| {
            let desc = match cap.as_str() {
                "file_read" => "Read file contents from the repository",
                "file_write" => "Create or modify files in the repository",
                "grep" => "Search file contents with regex patterns",
                "shell" => "Execute shell commands in a sandboxed environment",
                "git" => "Run git operations (status, diff, log, commit, branch)",
                "github_api" => "Interact with GitHub API (issues, PRs, reviews)",
                "web_search" => "Search the web for information",
                "database_query" => "Execute read-only SQL queries",
                "document_read" => "Read a document from the knowledge base by ID",
                other => other,
            };
            ToolDescription {
                name: cap.clone(),
                description: desc.to_string(),
            }
        })
        .collect()
}

/// Truncate long content for context injection, respecting char boundaries.
pub fn truncate_for_context(content: &str, max_chars: usize) -> &str {
    if content.len() <= max_chars {
        content
    } else {
        let mut end = max_chars;
        while end > 0 && !content.is_char_boundary(end) {
            end -= 1;
        }
        &content[..end]
    }
}
