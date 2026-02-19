//! Workforce archetype formatter for the Agent Designer.
//!
//! Converts a mission brief + agent roster into the generic `DesignerInput`
//! that the Agent Designer consumes.

use std::collections::HashMap;

use uuid::Uuid;

use crate::db::traits::ToolCapabilityRepo;
use crate::db::{TaskAgentRosterRow, TaskMissionBriefRow, WorkflowStepEdgeRow, WorkflowStepRow};
use crate::types::StepExecutionEnvelope;

use super::{
    build_tool_descriptions_from_db, format_envelopes_as_upstream, AgentDefinition, DependencyEdge,
    DesignerInput,
};

/// Build a `DesignerInput` from a workforce configuration.
pub async fn build_workforce_designer_input(
    brief: &TaskMissionBriefRow,
    roster: &[TaskAgentRosterRow],
    completed_envelopes: &HashMap<Uuid, StepExecutionEnvelope>,
    steps: &[WorkflowStepRow],
    assistant_notes: Option<&str>,
    tool_cap_repo: &dyn ToolCapabilityRepo,
    child_edges: &[WorkflowStepEdgeRow],
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

    // Build child_step_id → agent_name lookup for dependency resolution
    let child_step_to_name: HashMap<Uuid, &str> = roster
        .iter()
        .filter_map(|r| r.child_step_id.map(|csid| (csid, r.name.as_str())))
        .collect();

    // Convert child workflow edges to agent-name dependency edges
    // (filters out Designer→agent edges since Designer is not in roster)
    let dependencies: Vec<DependencyEdge> = child_edges
        .iter()
        .filter_map(|e| {
            let from = child_step_to_name.get(&e.from_step_id)?;
            let to = child_step_to_name.get(&e.to_step_id)?;
            Some(DependencyEdge {
                from_agent_name: from.to_string(),
                to_agent_name: to.to_string(),
            })
        })
        .collect();

    let mut guidance = format!("Failure mode: {}", brief.failure_mode);
    if let Some(ref downstream) = brief.downstream_context {
        if !downstream.is_empty() {
            guidance.push_str(&format!("\nDownstream context: {}", downstream));
        }
    }

    // Append dependency graph to guidance so the Designer sees it
    if dependencies.is_empty() {
        guidance.push_str(
            "\n\nNo inter-agent dependencies. All agents receive all prior agents' outputs.",
        );
    } else {
        guidance.push_str(
            "\n\nDependency graph (from \u{2192} to, meaning to receives from's output):",
        );
        for dep in &dependencies {
            guidance.push_str(&format!(
                "\n  {} \u{2192} {}",
                dep.from_agent_name, dep.to_agent_name
            ));
        }
        guidance.push_str("\nUse these dependencies to set receives_from routing for each agent.");
    }

    let mut upstream = format_envelopes_as_upstream(completed_envelopes, steps);
    if let Some(notes) = assistant_notes {
        if !notes.is_empty() {
            upstream.push(super::UpstreamContext {
                source_name: "Assistant's Notes".to_string(),
                source_type: "agent_notes".to_string(),
                content: notes.to_string(),
            });
        }
    }

    DesignerInput {
        archetype: "workforce".to_string(),
        context_description: format!(
            "A workforce executing a mission: {}",
            super::truncate_for_context(&brief.task_description, 200),
        ),
        agents,
        upstream,
        available_tools: build_tool_descriptions_from_db(
            &brief.available_capabilities,
            tool_cap_repo,
        )
        .await,
        archetype_guidance: guidance,
        dependencies,
    }
}
