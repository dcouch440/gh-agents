//! Task force archetype formatter for the Agent Designer.
//!
//! Converts a mission brief + agent roster into the generic `DesignerInput`
//! that the Agent Designer consumes.

use std::collections::HashMap;

use uuid::Uuid;

use crate::db::{TaskAgentRosterRow, TaskMissionBriefRow, WorkflowStepRow};
use crate::types::StepExecutionEnvelope;

use super::{
    build_tool_descriptions, format_envelopes_as_upstream, AgentDefinition, DesignerInput,
};

/// Build a `DesignerInput` from a task force configuration.
pub fn build_task_force_designer_input(
    brief: &TaskMissionBriefRow,
    roster: &[TaskAgentRosterRow],
    completed_envelopes: &HashMap<Uuid, StepExecutionEnvelope>,
    steps: &[WorkflowStepRow],
) -> DesignerInput {
    let agents = roster
        .iter()
        .map(|r| AgentDefinition {
            id: r.id.to_string(),
            name: r.name.clone(),
            role: r.role_description.clone(),
            capabilities: r.capabilities.clone(),
            execution_order: r.execution_order,
            additional_context: String::new(),
        })
        .collect();

    let mut guidance = format!("Failure mode: {}", brief.failure_mode);
    if let Some(ref downstream) = brief.downstream_context {
        if !downstream.is_empty() {
            guidance.push_str(&format!("\nDownstream context: {}", downstream));
        }
    }

    DesignerInput {
        archetype: "task_force".to_string(),
        context_description: format!(
            "A task force executing a mission: {}",
            super::truncate_for_context(&brief.task_description, 200),
        ),
        agents,
        upstream: format_envelopes_as_upstream(completed_envelopes, steps),
        available_tools: build_tool_descriptions(&brief.available_capabilities),
        archetype_guidance: guidance,
    }
}
