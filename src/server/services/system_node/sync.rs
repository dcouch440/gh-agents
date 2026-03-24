//! Sync system node agent files to DB state.
//!
//! After `complete_system` succeeds, this module diffs the agent's files
//! (`config.json`, `topology.json`, `agents/*.json`) against the current DB
//! state and applies minimal mutations. The DB is a projection of the files
//! for the frontend — the files remain the source of truth for execution.
//!
//! Follows the same diff pattern as `configure_team` in workforce tools,
//! but driven by file contents instead of tool input.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::Path;

use tracing::{info, warn};
use uuid::Uuid;

use super::normalize_agent_name;
use crate::db::traits::WorkflowRepo;
use crate::db::TaskAgentRosterRow;
use crate::server::services::pipeline::{self, AddStepInput, PipelineContext};
use crate::server::services::ServiceError;

use super::file_reader;

#[cfg(test)]
#[path = "sync_tests.rs"]
mod tests;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Result of syncing system node agent files to the DB.
#[derive(Debug, Default)]
pub(crate) struct SyncResult {
    pub agents_created: Vec<String>,
    pub agents_updated: Vec<String>,
    pub agents_removed: Vec<String>,
    pub edges_created: usize,
    pub edges_removed: usize,
    pub description_changed: bool,
}

/// Agent definition read from the filesystem for diffing against DB state.
#[derive(Debug, Clone)]
pub(crate) struct DesiredAgent {
    pub slug: String,
    pub name: String,
    pub role_description: String,
    pub capabilities: Vec<String>,
    pub depends_on: Vec<String>,
}

/// Sync system node agent files to DB state.
///
/// Reads `config.json`, `topology.json`, and `agents/*.json` from `base_dir`,
/// diffs against current DB state, and applies minimal mutations:
/// - Roster entries: create/update/remove agents
/// - Edges: add/remove dependency edges in the child workflow
/// - Step metadata: update name and designer_handoff (description)
/// - Mission brief: update task_description and capabilities
///
/// Returns a `SyncResult` with a summary of changes and whether the
/// config description changed (drives downstream cascade in slice 5).
pub(crate) async fn sync_to_db(
    base_dir: &Path,
    step_id: Uuid,
    workflow_id: Uuid,
    repo: &dyn WorkflowRepo,
    user_id: Uuid,
) -> Result<SyncResult, ServiceError> {
    let mut result = SyncResult::default();

    // Phase 1: Read files
    let (config_name, config_description) = file_reader::read_config(base_dir)
        .map_err(|e| ServiceError::Internal(anyhow::anyhow!("{e}")))?;

    let topology = file_reader::read_topology(base_dir)
        .map_err(|e| ServiceError::Internal(anyhow::anyhow!("{e}")))?;

    let desired_agents = read_desired_agents(base_dir, &topology)?;

    info!(
        step_id = %step_id,
        agents = desired_agents.len(),
        "Syncing system node agent files to DB"
    );

    // Phase 2: Ensure pipeline + mission brief
    let pip_ctx = PipelineContext {
        parent_step_id: step_id,
        parent_workflow_id: workflow_id,
    };
    let _pipeline = pipeline::create_pipeline(repo, &pip_ctx, user_id).await?;

    let all_caps: Vec<String> = {
        let mut set = BTreeSet::new();
        for agent in &desired_agents {
            set.extend(agent.capabilities.iter().cloned());
        }
        set.into_iter().collect()
    };

    let brief = repo
        .upsert_mission_brief(step_id, &config_description, &all_caps, "fail_fast", None)
        .await
        .map_err(ServiceError::Internal)?;

    // Phase 3: Diff agents
    let current_roster = repo
        .list_agent_roster(brief.id)
        .await
        .map_err(ServiceError::Internal)?;

    let agent_diff = diff_agents(&desired_agents, &current_roster);

    // Apply agent mutations
    let mut created_count: i32 = 0;
    let max_order = current_roster
        .iter()
        .map(|a| a.execution_order)
        .max()
        .unwrap_or(-1);

    for name in &agent_diff.to_create {
        let agent = desired_agents
            .iter()
            .find(|a| normalize_agent_name(&a.name) == normalize_agent_name(name))
            .unwrap();

        let next_order = max_order + 1 + created_count;

        let (step_added, _) = pipeline::add_step(
            repo,
            &pip_ctx,
            user_id,
            AddStepInput {
                name: agent.name.clone(),
                description: agent.role_description.clone(),
                execution_mode: "single".to_string(),
                agent_id: None,
                prompt_template: None,
                output_variable_name: None,
                display_order: Some(next_order + 1),
            },
        )
        .await?;

        let roster_agent = repo
            .add_roster_agent(
                brief.id,
                &agent.name,
                &agent.role_description,
                &agent.capabilities,
                next_order,
            )
            .await
            .map_err(ServiceError::Internal)?;

        repo.link_roster_agent_to_child_step(roster_agent.id, Some(step_added.step_id))
            .await
            .map_err(ServiceError::Internal)?;

        created_count += 1;
        result.agents_created.push(agent.name.clone());
    }

    for (agent_id, name) in &agent_diff.to_update {
        let agent = desired_agents
            .iter()
            .find(|a| normalize_agent_name(&a.name) == normalize_agent_name(name))
            .unwrap();

        let current = current_roster.iter().find(|r| r.id == *agent_id).unwrap();
        let new_role = if current.role_description != agent.role_description {
            Some(agent.role_description.clone())
        } else {
            None
        };
        let new_caps = if current.capabilities != agent.capabilities {
            Some(agent.capabilities.clone())
        } else {
            None
        };

        repo.update_roster_agent(*agent_id, None, new_role.clone(), new_caps)
            .await
            .map_err(ServiceError::Internal)?;

        // Sync child step description if role changed
        if new_role.is_some() {
            if let Some(child_step_id) = current.child_step_id {
                let _ = pipeline::update_step(
                    repo,
                    &pip_ctx,
                    child_step_id,
                    pipeline::UpdateStepInput {
                        description: Some(agent.role_description.clone()),
                        ..Default::default()
                    },
                )
                .await;
            }
        }

        result.agents_updated.push(name.clone());
    }

    for (agent_id, name, child_step_id) in &agent_diff.to_remove {
        if let Some(child_step_id) = child_step_id {
            if let Err(e) = pipeline::remove_step(repo, &pip_ctx, *child_step_id).await {
                warn!(agent = %name, error = %e, "Failed to remove child step");
            }
        }
        if let Err(e) = repo.remove_roster_agent(*agent_id).await {
            warn!(agent = %name, error = %e, "Failed to remove roster agent");
        }
        result.agents_removed.push(name.clone());
    }

    // Phase 4: Diff edges
    let step = repo
        .get_step(step_id)
        .await
        .map_err(ServiceError::Internal)?
        .ok_or_else(|| ServiceError::not_found("Step"))?;

    if let Some(child_wf_id) = step.child_workflow_id {
        // Reload roster to get fresh child_step_ids after creates
        let updated_roster = repo
            .list_agent_roster(brief.id)
            .await
            .map_err(ServiceError::Internal)?;

        let name_to_step: HashMap<String, Uuid> = updated_roster
            .iter()
            .filter_map(|a| {
                a.child_step_id
                    .map(|sid| (normalize_agent_name(&a.name), sid))
            })
            .collect();

        let current_edges = repo.list_edges(child_wf_id).await.unwrap_or_default();
        let agent_step_ids: HashSet<Uuid> = name_to_step.values().copied().collect();

        let edge_diff = diff_edges(
            &desired_agents,
            &name_to_step,
            &current_edges,
            &agent_step_ids,
        );

        for (from_sid, to_sid) in &edge_diff.to_add {
            match pipeline::add_edge(repo, &pip_ctx, *from_sid, *to_sid).await {
                Ok(_) => result.edges_created += 1,
                Err(ServiceError::Conflict(_)) => {} // already exists
                Err(ServiceError::Validation(msg)) => {
                    warn!(error = %msg, "Edge would create cycle, skipping");
                }
                Err(e) => return Err(e),
            }
        }

        for (from_sid, to_sid) in &edge_diff.to_remove {
            if let Err(e) = pipeline::remove_edge(repo, &pip_ctx, *from_sid, *to_sid).await {
                warn!(error = %e, "Failed to remove edge");
            }
            result.edges_removed += 1;
        }

        // Recompute execution order for child steps
        let _ = pipeline::recompute_execution_order(repo, child_wf_id).await;

        // Also recompute roster execution_order to match
        recompute_roster_order(repo, brief.id, child_wf_id).await;
    }

    // Phase 5: Sync step metadata + detect description change
    let previous_description = step.designer_handoff.clone();

    let mut updated_step = step;
    updated_step.name = Some(config_name.clone());
    repo.update_step(updated_step)
        .await
        .map_err(ServiceError::Internal)?;

    repo.update_designer_handoff(step_id, &config_description)
        .await
        .map_err(ServiceError::Internal)?;

    result.description_changed = previous_description != config_description;

    info!(
        step_id = %step_id,
        created = result.agents_created.len(),
        updated = result.agents_updated.len(),
        removed = result.agents_removed.len(),
        edges_created = result.edges_created,
        edges_removed = result.edges_removed,
        description_changed = result.description_changed,
        "Sync complete"
    );

    Ok(result)
}

// ---------------------------------------------------------------------------
// Pure diff helpers (testable without DB)
// ---------------------------------------------------------------------------

/// Result of diffing desired agents against current roster.
#[derive(Debug, Default)]
pub(crate) struct AgentDiff {
    /// Agent names to create (not in current roster).
    pub to_create: Vec<String>,
    /// (roster_id, agent_name) pairs to update (changed role or capabilities).
    pub to_update: Vec<(Uuid, String)>,
    /// (roster_id, agent_name, child_step_id) triples to remove (not in desired).
    pub to_remove: Vec<(Uuid, String, Option<Uuid>)>,
}

/// Diff desired agents against current roster entries.
///
/// Matches by normalized name (case-insensitive, strips separators).
/// Returns which agents need to be created, updated, or removed.
pub(crate) fn diff_agents(desired: &[DesiredAgent], current: &[TaskAgentRosterRow]) -> AgentDiff {
    let current_by_name: HashMap<String, &TaskAgentRosterRow> = current
        .iter()
        .map(|a| (normalize_agent_name(&a.name), a))
        .collect();

    let mut diff = AgentDiff::default();
    let mut matched_ids: HashSet<Uuid> = HashSet::new();

    for agent in desired {
        let norm = normalize_agent_name(&agent.name);
        if let Some(current_agent) = current_by_name.get(&norm) {
            matched_ids.insert(current_agent.id);

            let role_changed = current_agent.role_description != agent.role_description;
            let caps_changed = current_agent.capabilities != agent.capabilities;

            if role_changed || caps_changed {
                diff.to_update.push((current_agent.id, agent.name.clone()));
            }
        } else {
            diff.to_create.push(agent.name.clone());
        }
    }

    for current_agent in current {
        if !matched_ids.contains(&current_agent.id) {
            diff.to_remove.push((
                current_agent.id,
                current_agent.name.clone(),
                current_agent.child_step_id,
            ));
        }
    }

    diff
}

/// Result of diffing desired edges against current edges.
#[derive(Debug, Default)]
pub(crate) struct EdgeDiff {
    /// (from_step_id, to_step_id) pairs to add.
    pub to_add: Vec<(Uuid, Uuid)>,
    /// (from_step_id, to_step_id) pairs to remove.
    pub to_remove: Vec<(Uuid, Uuid)>,
}

/// Diff desired edges (from topology depends_on) against current DB edges.
///
/// Only considers agent-to-agent edges (both endpoints must be in the
/// agent step ID set). Non-agent edges (e.g., Designer steps) are ignored.
pub(crate) fn diff_edges(
    desired_agents: &[DesiredAgent],
    name_to_step: &HashMap<String, Uuid>,
    current_edges: &[crate::db::WorkflowStepEdgeRow],
    agent_step_ids: &HashSet<Uuid>,
) -> EdgeDiff {
    // Build desired edge set from topology depends_on
    let mut desired: HashSet<(Uuid, Uuid)> = HashSet::new();
    for agent in desired_agents {
        if let Some(&to_sid) = name_to_step.get(&normalize_agent_name(&agent.name)) {
            for dep_slug in &agent.depends_on {
                if let Some(&from_sid) = name_to_step.get(&normalize_agent_name(dep_slug)) {
                    desired.insert((from_sid, to_sid));
                }
            }
        }
    }

    // Build current agent-only edge set
    let current: HashSet<(Uuid, Uuid)> = current_edges
        .iter()
        .filter(|e| {
            agent_step_ids.contains(&e.from_step_id) && agent_step_ids.contains(&e.to_step_id)
        })
        .map(|e| (e.from_step_id, e.to_step_id))
        .collect();

    let to_add: Vec<(Uuid, Uuid)> = desired.difference(&current).copied().collect();
    let to_remove: Vec<(Uuid, Uuid)> = current.difference(&desired).copied().collect();

    EdgeDiff { to_add, to_remove }
}

/// Detect whether config.json description changed vs the stored value.
pub(crate) fn description_changed(previous: &str, current: &str) -> bool {
    previous != current
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Read desired agents from filesystem: topology slugs + agent JSON files.
fn read_desired_agents(
    base_dir: &Path,
    topology: &HashMap<String, Vec<String>>,
) -> Result<Vec<DesiredAgent>, ServiceError> {
    let agents_dir = base_dir.join("agents");
    let mut agents = Vec::with_capacity(topology.len());

    for (slug, depends_on) in topology {
        let agent_path = agents_dir.join(format!("{slug}.json"));
        let content = std::fs::read_to_string(&agent_path).map_err(|e| {
            ServiceError::Internal(anyhow::anyhow!("cannot read agents/{slug}.json: {e}"))
        })?;

        let config: AgentFileConfig = serde_json::from_str(&content).map_err(|e| {
            ServiceError::Internal(anyhow::anyhow!("invalid JSON in agents/{slug}.json: {e}"))
        })?;

        agents.push(DesiredAgent {
            slug: slug.clone(),
            name: config.name,
            role_description: config.system_prompt,
            capabilities: config.capabilities,
            depends_on: depends_on.clone(),
        });
    }

    Ok(agents)
}

/// Agent config JSON shape (subset for roster sync -- we don't need assignment/expected_output).
#[derive(serde::Deserialize)]
struct AgentFileConfig {
    name: String,
    system_prompt: String,
    #[serde(default)]
    capabilities: Vec<String>,
}

/// Recompute roster `execution_order` to match child step ordering.
///
/// After `pipeline::recompute_execution_order` updates child steps,
/// this syncs the roster entries to match.
async fn recompute_roster_order(repo: &dyn WorkflowRepo, brief_id: Uuid, child_wf_id: Uuid) {
    let roster = match repo.list_agent_roster(brief_id).await {
        Ok(r) => r,
        Err(_) => return,
    };

    let child_steps = match repo.list_steps(child_wf_id).await {
        Ok(s) => s,
        Err(_) => return,
    };

    // Build child_step_id -> display_order lookup
    let step_order: HashMap<Uuid, i32> = child_steps
        .iter()
        .map(|s| (s.id, s.display_order))
        .collect();

    for agent in &roster {
        if let Some(child_step_id) = agent.child_step_id {
            if let Some(&order) = step_order.get(&child_step_id) {
                if agent.execution_order != order {
                    let _ = repo.update_roster_agent_order(agent.id, order).await;
                }
            }
        }
    }
}
