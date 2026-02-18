//! Workforce archetype formatter for the Agent Designer.
//!
//! Converts a mission brief + agent roster + deliverables into the generic
//! `DesignerInput` that the Agent Designer consumes. Includes deliverable
//! assignments per agent so the designer can generate document-aware prompts.

use std::collections::HashMap;

use uuid::Uuid;

use crate::db::traits::ToolCapabilityRepo;
use crate::db::{ProtocolDocumentDefRow, TaskAgentRosterRow, TaskMissionBriefRow, WorkflowStepRow};
use crate::types::StepExecutionEnvelope;

use super::{
    build_tool_descriptions_from_db, format_envelopes_as_upstream, AgentDefinition, DesignerInput,
};

/// Build a `DesignerInput` from a workforce configuration.
///
/// Includes deliverable assignments for each agent in the `additional_context`
/// field, so the designer can generate prompts that instruct agents to produce
/// their assigned documents.
pub async fn build_workforce_designer_input(
    brief: &TaskMissionBriefRow,
    roster: &[TaskAgentRosterRow],
    doc_defs: &[ProtocolDocumentDefRow],
    completed_envelopes: &HashMap<Uuid, StepExecutionEnvelope>,
    steps: &[WorkflowStepRow],
    assistant_notes: Option<&str>,
    tool_cap_repo: &dyn ToolCapabilityRepo,
) -> DesignerInput {
    // Group deliverables by agent roster entry ID
    let mut deliverables_by_agent: HashMap<Uuid, Vec<&ProtocolDocumentDefRow>> = HashMap::new();
    let mut unassigned: Vec<&ProtocolDocumentDefRow> = Vec::new();

    for def in doc_defs {
        if let Some(agent_id) = def.agent_roster_entry_id {
            deliverables_by_agent.entry(agent_id).or_default().push(def);
        } else {
            unassigned.push(def);
        }
    }

    let agents = roster
        .iter()
        .map(|r| {
            let agent_deliverables = deliverables_by_agent
                .get(&r.id)
                .map(|defs| {
                    defs.iter()
                        .map(|d| {
                            format!(
                                "- {} (~{} words): {}",
                                d.name, d.target_length, d.description
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n")
                })
                .unwrap_or_default();

            let additional = if agent_deliverables.is_empty() {
                "No deliverables assigned.".to_string()
            } else {
                format!("Assigned deliverables:\n{}", agent_deliverables)
            };

            AgentDefinition {
                id: r.id.to_string(),
                name: r.name.clone(),
                role: r.role_description.clone(),
                capabilities: r.capabilities.clone(),
                execution_order: r.execution_order,
                additional_context: additional,
            }
        })
        .collect();

    let mut guidance = format!("Failure mode: {}", brief.failure_mode);
    if let Some(ref downstream) = brief.downstream_context {
        if !downstream.is_empty() {
            guidance.push_str(&format!("\nDownstream context: {}", downstream));
        }
    }

    // Add unassigned deliverables to guidance
    if !unassigned.is_empty() {
        let unassigned_list: Vec<String> = unassigned
            .iter()
            .map(|d| {
                format!(
                    "- {} (~{} words): {}",
                    d.name, d.target_length, d.description
                )
            })
            .collect();
        guidance.push_str(&format!(
            "\n\nUnassigned deliverables (assign to appropriate agents):\n{}",
            unassigned_list.join("\n")
        ));
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
            "A workforce executing a mission with document deliverables: {}",
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
    }
}
