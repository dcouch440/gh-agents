//! Run templates — frozen workflow snapshots for reproducible execution.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::{
    AgentRow, ProtocolDocumentDefRow, RoomStepConfigRow, RoomStepMemberRow, StepInputRow,
    StepOutputRow, StepRoutingRuleRow, TaskAgentRosterRow, TaskMissionBriefRow, ToolRow,
    WorkflowStepEdgeRow, WorkflowStepProtocolRow, WorkflowStepRow,
};
use crate::server::hub::dag::dag_state::PortMetadata;
use crate::server::state::AppState;

/// Complete frozen snapshot of a workflow's configuration.
/// All HashMap keys are step_id unless noted otherwise.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowSnapshot {
    pub steps: Vec<WorkflowStepRow>,
    pub edges: Vec<WorkflowStepEdgeRow>,
    pub step_inputs: HashMap<Uuid, Vec<StepInputRow>>,
    pub step_outputs: HashMap<Uuid, Vec<StepOutputRow>>,
    pub routing_rules: HashMap<Uuid, Vec<StepRoutingRuleRow>>,
    pub document_defs: HashMap<Uuid, Vec<ProtocolDocumentDefRow>>,
    pub protocols: HashMap<Uuid, WorkflowStepProtocolRow>,
    pub room_configs: HashMap<Uuid, RoomStepConfigRow>,
    pub room_members: HashMap<Uuid, Vec<RoomStepMemberRow>>,
    pub mission_briefs: HashMap<Uuid, TaskMissionBriefRow>,
    pub agent_rosters: HashMap<Uuid, Vec<TaskAgentRosterRow>>,
    /// Full agent definitions keyed by agent_id (not step_id).
    pub agents: HashMap<Uuid, AgentRow>,
    /// Tool assignments keyed by agent_id.
    pub agent_tools: HashMap<Uuid, Vec<ToolRow>>,
}

/// Capture the complete workflow configuration as a frozen snapshot.
pub(crate) async fn capture_workflow_snapshot(
    state: &AppState,
    workflow_id: Uuid,
) -> anyhow::Result<WorkflowSnapshot> {
    let wf_repo = &state.repos().workflows;

    let steps = wf_repo.list_steps(workflow_id).await?;
    let edges = wf_repo.list_edges(workflow_id).await?;

    let mut step_inputs: HashMap<Uuid, Vec<StepInputRow>> = HashMap::new();
    let mut step_outputs: HashMap<Uuid, Vec<StepOutputRow>> = HashMap::new();
    let mut routing_rules: HashMap<Uuid, Vec<StepRoutingRuleRow>> = HashMap::new();
    let mut document_defs: HashMap<Uuid, Vec<ProtocolDocumentDefRow>> = HashMap::new();
    let mut protocols: HashMap<Uuid, WorkflowStepProtocolRow> = HashMap::new();
    let mut room_configs: HashMap<Uuid, RoomStepConfigRow> = HashMap::new();
    let mut room_members: HashMap<Uuid, Vec<RoomStepMemberRow>> = HashMap::new();
    let mut mission_briefs: HashMap<Uuid, TaskMissionBriefRow> = HashMap::new();
    let mut agent_rosters: HashMap<Uuid, Vec<TaskAgentRosterRow>> = HashMap::new();

    // Collect all unique agent_ids we encounter
    let mut agent_ids: std::collections::HashSet<Uuid> = std::collections::HashSet::new();

    for step in &steps {
        // Ports
        let inputs = wf_repo.get_step_inputs(step.id).await?;
        let outputs = wf_repo.get_step_outputs(step.id).await?;
        if !inputs.is_empty() {
            step_inputs.insert(step.id, inputs);
        }
        if !outputs.is_empty() {
            step_outputs.insert(step.id, outputs);
        }

        // Routing rules (for label-based routing)
        if step.routing_mode.as_deref() == Some("label") {
            let rules = wf_repo.get_step_routing_rules(step.id).await?;
            for rule in &rules {
                agent_ids.insert(rule.agent_id);
            }
            if !rules.is_empty() {
                routing_rules.insert(step.id, rules);
            }
        }

        // Document definitions (for documenter steps)
        if step.execution_mode == "documenter" {
            let defs = wf_repo.list_document_defs(step.id).await?;
            if !defs.is_empty() {
                document_defs.insert(step.id, defs);
            }
        }

        // Protocol linkage
        if let Ok(Some(proto)) = state.repos().protocols.get_step_protocol(step.id).await {
            protocols.insert(step.id, proto);
        }

        // Room configs
        if step.execution_mode == "room" {
            if let Ok(Some(config)) = wf_repo.get_room_step_config(step.id).await {
                room_configs.insert(step.id, config);
            }
            if let Ok(members) = wf_repo.list_room_step_members(step.id).await {
                if !members.is_empty() {
                    room_members.insert(step.id, members);
                }
            }
        }

        // Task force configs
        if step.execution_mode == "task_force" || step.execution_mode == "workforce" {
            if let Ok(Some(brief)) = wf_repo.get_mission_brief(step.id).await {
                if let Ok(roster) = wf_repo.list_agent_roster(brief.id).await {
                    if !roster.is_empty() {
                        agent_rosters.insert(step.id, roster);
                    }
                }
                mission_briefs.insert(step.id, brief);
            }
        }

        // Workforce deliverables (document definitions)
        if step.execution_mode == "workforce" {
            let defs = wf_repo.list_document_defs(step.id).await?;
            if !defs.is_empty() {
                document_defs.insert(step.id, defs);
            }
        }

        // Collect agent_id from step
        if let Some(aid) = step.agent_id {
            agent_ids.insert(aid);
        }
        if let Some(aid) = step.interactive_agent_id {
            agent_ids.insert(aid);
        }
    }

    // Load full agent definitions + their tools
    let server_repo = state.repo();
    let mut agents: HashMap<Uuid, AgentRow> = HashMap::new();
    let mut agent_tools: HashMap<Uuid, Vec<ToolRow>> = HashMap::new();

    for aid in &agent_ids {
        if let Ok(Some(agent)) = server_repo.get_persisted_agent(*aid).await {
            agents.insert(*aid, agent);
        }
        if let Ok(tools) = server_repo.get_agent_tools(*aid).await {
            if !tools.is_empty() {
                agent_tools.insert(*aid, tools);
            }
        }
    }

    Ok(WorkflowSnapshot {
        steps,
        edges,
        step_inputs,
        step_outputs,
        routing_rules,
        document_defs,
        protocols,
        room_configs,
        room_members,
        mission_briefs,
        agent_rosters,
        agents,
        agent_tools,
    })
}

/// Build PortMetadata from a deserialized snapshot (no DB queries needed).
pub(crate) fn port_metadata_from_snapshot(snapshot: &WorkflowSnapshot) -> PortMetadata {
    PortMetadata::new(
        snapshot.step_inputs.clone(),
        snapshot.step_outputs.clone(),
        snapshot.routing_rules.clone(),
    )
}

pub(crate) mod restore;

#[cfg(test)]
mod tests;
