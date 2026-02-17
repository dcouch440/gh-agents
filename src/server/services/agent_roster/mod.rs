//! Agent roster service: CRUD for task force agent roster entries on workflow steps.

use std::collections::HashMap;
use uuid::Uuid;

use crate::db::traits::WorkflowRepo;
use crate::db::TaskAgentRosterRow;

use super::error::ServiceError;
use super::steps::verify_step_access;

pub struct CreateRosterAgentInput {
    pub user_id: Uuid,
    pub workflow_id: Uuid,
    pub step_id: Uuid,
    pub name: String,
    pub role_description: String,
    pub capabilities: Vec<String>,
    pub execution_order: i32,
}

/// Info about a deleted roster agent, for the consistency scanner.
pub struct DeletedRosterAgentInfo {
    pub agent_name: String,
    pub step_name: String,
}

/// A roster agent with its computed depends_on list.
pub struct RosterAgentWithDeps {
    pub agent: TaskAgentRosterRow,
    pub depends_on: Vec<Uuid>,
}

/// List roster agents for a step, including computed depends_on from child workflow edges.
pub async fn list_roster_agents(
    repo: &dyn WorkflowRepo,
    user_id: Uuid,
    workflow_id: Uuid,
    step_id: Uuid,
) -> Result<Vec<RosterAgentWithDeps>, ServiceError> {
    verify_step_access(repo, user_id, workflow_id, step_id).await?;

    let brief = repo.get_mission_brief(step_id).await?;
    let agents = match brief {
        Some(b) => repo.list_agent_roster(b.id).await?,
        None => vec![],
    };

    // Build depends_on from child workflow edges
    let step = repo.get_step(step_id).await?;
    let child_edges = match step.and_then(|s| s.child_workflow_id) {
        Some(child_wf_id) => repo.list_edges(child_wf_id).await.unwrap_or_default(),
        None => vec![],
    };

    // Map child_step_id → roster agent ID
    let child_step_to_agent: HashMap<Uuid, Uuid> = agents
        .iter()
        .filter_map(|a| a.child_step_id.map(|csid| (csid, a.id)))
        .collect();

    let results = agents
        .into_iter()
        .map(|a| {
            let deps = match a.child_step_id {
                Some(child_step_id) => child_edges
                    .iter()
                    .filter(|e| e.to_step_id == child_step_id)
                    .filter_map(|e| child_step_to_agent.get(&e.from_step_id))
                    .copied()
                    .collect(),
                None => vec![],
            };
            RosterAgentWithDeps {
                agent: a,
                depends_on: deps,
            }
        })
        .collect();

    Ok(results)
}

/// Create a roster agent on a step, verifying ownership.
/// Auto-creates a mission brief if one doesn't exist.
pub async fn create_roster_agent(
    repo: &dyn WorkflowRepo,
    input: CreateRosterAgentInput,
) -> Result<TaskAgentRosterRow, ServiceError> {
    verify_step_access(repo, input.user_id, input.workflow_id, input.step_id).await?;

    // Ensure a mission brief exists
    let brief = repo
        .upsert_mission_brief(input.step_id, "", &[], "fail_fast", None)
        .await?;

    let row = repo
        .add_roster_agent(
            brief.id,
            &input.name,
            &input.role_description,
            &input.capabilities,
            input.execution_order,
        )
        .await?;

    Ok(row)
}

/// Delete a roster agent, verifying ownership.
/// Returns info about the deleted agent for the consistency scanner.
pub async fn delete_roster_agent(
    repo: &dyn WorkflowRepo,
    user_id: Uuid,
    workflow_id: Uuid,
    step_id: Uuid,
    roster_agent_id: Uuid,
) -> Result<DeletedRosterAgentInfo, ServiceError> {
    verify_step_access(repo, user_id, workflow_id, step_id).await?;

    // Load agent name before deleting (for consistency scanner)
    let agent_name = if let Ok(Some(brief)) = repo.get_mission_brief(step_id).await {
        repo.list_agent_roster(brief.id)
            .await
            .ok()
            .and_then(|roster| roster.into_iter().find(|a| a.id == roster_agent_id))
            .map(|a| a.name)
            .unwrap_or_default()
    } else {
        String::new()
    };

    let step_name = repo
        .get_step(step_id)
        .await
        .ok()
        .flatten()
        .and_then(|s| s.name)
        .unwrap_or_default();

    repo.remove_roster_agent(roster_agent_id).await?;

    Ok(DeletedRosterAgentInfo {
        agent_name,
        step_name,
    })
}

mod tests;
