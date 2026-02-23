//! Workforce archetype formatter for the Agent Designer.
//!
//! Converts a mission brief + agent roster into the generic `DesignerInput`
//! that the Agent Designer consumes.

use std::collections::HashMap;

use uuid::Uuid;

use crate::config::capability_registry::CapabilityRegistry;
use crate::db::{TaskAgentRosterRow, TaskMissionBriefRow, WorkflowStepEdgeRow, WorkflowStepRow};
use crate::types::StepExecutionEnvelope;

use super::{format_envelopes_as_upstream, AgentDefinition, DependencyEdge, DesignerInput};

/// Build a `DesignerInput` from a workforce configuration.
pub fn build_workforce_designer_input(
    brief: &TaskMissionBriefRow,
    roster: &[TaskAgentRosterRow],
    completed_envelopes: &HashMap<Uuid, StepExecutionEnvelope>,
    steps: &[WorkflowStepRow],
    plan: Option<&str>,
    capability_registry: &CapabilityRegistry,
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

    // Append execution order to guidance so the Designer writes position-aware prompts
    if dependencies.is_empty() {
        guidance.push_str(
            "\n\nExecution order: all agents run in parallel (no dependencies). Each agent receives all prior outputs.",
        );
    } else {
        guidance.push_str("\n\nExecution order (enforced by runtime — you do not control this):");
        for dep in &dependencies {
            guidance.push_str(&format!(
                "\n  {} runs before {}",
                dep.from_agent_name, dep.to_agent_name
            ));
        }
        guidance.push_str("\nUse this ordering to write position-aware prompts (tell agents who runs before/after them).");
    }

    let mut upstream = format_envelopes_as_upstream(completed_envelopes, steps);
    if let Some(plan) = plan {
        if !plan.is_empty() {
            upstream.push(super::UpstreamContext {
                source_name: "Plan".to_string(),
                source_type: "plan".to_string(),
                content: plan.to_string(),
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
        available_tools: capability_registry.tool_descriptions(&brief.available_capabilities),
        archetype_guidance: guidance,
        dependencies,
    }
}
