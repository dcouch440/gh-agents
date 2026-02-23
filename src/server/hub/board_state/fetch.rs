//! Board state fetch layer — loads snapshots from the database.
//!
//! Two entry points:
//! - [`fetch_node`]: single node for L3/L4 (own-node scope)
//! - [`fetch_board`]: all visible nodes for L1/L2 (all-nodes scope)
//!
//! Both funnel through [`assemble_node`] which builds a [`NodeSnapshot`]
//! from either pre-loaded bulk data or individual queries.

use std::collections::{HashMap, HashSet};

use anyhow::{Context, Result};
use uuid::Uuid;

use crate::db::traits::WorkflowRepo;
use crate::db::{StepQuestionStateRow, WorkflowStepEdgeRow, WorkflowStepRow};
use crate::server::tools::shared::classify_content_status;

use super::types::*;

// ============================================================================
// Public API
// ============================================================================

/// Fetch a single node's full snapshot (L3/L4 — own-node scope).
///
/// Loads edges once for the workflow, then assembles the node. Upstream
/// steps are fetched individually since we don't have the full step map.
pub async fn fetch_node(
    repo: &dyn WorkflowRepo,
    workflow_id: Uuid,
    step_id: Uuid,
) -> Result<NodeSnapshot> {
    let step = repo.get_step(step_id).await?.context("Step not found")?;

    let edges = repo.list_edges(workflow_id).await?;

    let mut node = assemble_node(repo, &step, &edges, None).await?;

    // Inject question state (L3/L4 don't render it, but keep data consistent)
    if let Ok(Some(qs)) = repo.get_step_question_state(step_id).await {
        node.compressed_status = Some(qs.status_text);
        node.asking = qs.question_text;
    }

    Ok(node)
}

/// Fetch all visible nodes in a workflow (L1/L2 — all-nodes scope).
///
/// Bulk-loads steps and edges once, then assembles each node using
/// the pre-loaded data for efficient upstream lookups.
pub async fn fetch_board(repo: &dyn WorkflowRepo, workflow_id: Uuid) -> Result<BoardSnapshot> {
    let workflow = repo
        .get_workflow(workflow_id)
        .await?
        .context("Workflow not found")?;

    let all_steps = repo.list_steps(workflow_id).await?;
    let all_edges = repo.list_edges(workflow_id).await?;

    // Build lookup map for efficient upstream resolution
    let steps_map: HashMap<Uuid, WorkflowStepRow> =
        all_steps.iter().map(|s| (s.id, s.clone())).collect();

    // Batch-load question states for all steps
    let step_ids: Vec<Uuid> = all_steps.iter().map(|s| s.id).collect();
    let question_states = repo.get_step_question_states(&step_ids).await?;
    let question_map: HashMap<Uuid, StepQuestionStateRow> = question_states
        .into_iter()
        .map(|qs| (qs.step_id, qs))
        .collect();

    let mut nodes = Vec::new();
    let mut all_capabilities: HashSet<String> = HashSet::new();

    for step in &all_steps {
        if !step.visible {
            continue;
        }
        if step.execution_mode == "manager" {
            continue;
        }

        let mut node = assemble_node(repo, step, &all_edges, Some(&steps_map)).await?;

        // Inject question state
        if let Some(qs) = question_map.get(&step.id) {
            node.compressed_status = Some(qs.status_text.clone());
            node.asking = qs.question_text.clone();
        }

        for cap in &node.capabilities {
            all_capabilities.insert(cap.clone());
        }

        nodes.push(node);
    }

    let mut available_capabilities: Vec<String> = all_capabilities.into_iter().collect();
    available_capabilities.sort();

    Ok(BoardSnapshot {
        workflow_name: workflow.name,
        workflow_id,
        nodes,
        available_capabilities,
    })
}

// ============================================================================
// Assembly
// ============================================================================

/// Assemble a [`NodeSnapshot`] from a step row and shared edge data.
///
/// When `steps_map` is `Some` (board path), upstream lookups use the map.
/// When `None` (single-node path), upstream steps are fetched individually.
async fn assemble_node(
    repo: &dyn WorkflowRepo,
    step: &WorkflowStepRow,
    workflow_edges: &[WorkflowStepEdgeRow],
    steps_map: Option<&HashMap<Uuid, WorkflowStepRow>>,
) -> Result<NodeSnapshot> {
    // Mission brief (task, capabilities, failure_mode)
    let brief = repo.get_mission_brief(step.id).await?;

    let task = brief
        .as_ref()
        .map(|b| b.task_description.clone())
        .unwrap_or_default();
    let capabilities = brief
        .as_ref()
        .map(|b| b.available_capabilities.clone())
        .unwrap_or_default();
    let failure_mode = brief
        .as_ref()
        .map(|b| b.failure_mode.clone())
        .unwrap_or_default();

    // Agent roster + dependency resolution
    let (agents, has_deps) = if let Some(ref brief) = brief {
        load_agents(repo, brief.id, step.child_workflow_id).await?
    } else {
        (vec![], false)
    };

    // Upstream detection
    let upstream_ids: Vec<Uuid> = workflow_edges
        .iter()
        .filter(|e| e.to_step_id == step.id)
        .map(|e| e.from_step_id)
        .collect();

    let receives = if upstream_ids.is_empty() {
        None
    } else {
        let names: Vec<String> = resolve_upstream_names(&upstream_ids, steps_map, repo).await;
        Some(names.join(", "))
    };

    // Incoming context (upstream nodes with content status)
    let incoming_context = build_incoming_context(&upstream_ids, steps_map, repo).await?;

    // Input/output ports (L4)
    let input_ports = load_input_ports(repo, step.id, workflow_edges, steps_map).await?;
    let output_ports = load_output_ports(repo, step.id, workflow_edges, steps_map).await?;

    // Step plan (L4)
    let plan = repo.get_plan(step.id).await?.unwrap_or_default();

    // Derived fields
    let status = derive_node_status(step, !task.is_empty(), agents.len());
    let summary = derive_node_summary(!task.is_empty(), agents.len(), has_deps);

    Ok(NodeSnapshot {
        id: step.id,
        ref_id: step.ref_id.clone(),
        name: step.name.clone().unwrap_or_else(|| "(unnamed)".to_string()),
        protocol: step.execution_mode.clone(),
        status,
        task,
        capabilities,
        failure_mode,
        summary,
        compressed_status: None, // populated by caller from step_question_state
        agents,
        input_ports,
        output_ports,
        incoming_context,
        plan,
        asking: None, // populated by caller from step_question_state
        receives,
        initial_instructions_sent: false, // populated by caller for L1/L2
    })
}

// ============================================================================
// Agent Loading
// ============================================================================

/// Load agents from the roster and resolve inter-agent dependencies.
///
/// Returns `(agents, has_dependencies)`.
async fn load_agents(
    repo: &dyn WorkflowRepo,
    brief_id: Uuid,
    child_workflow_id: Option<Uuid>,
) -> Result<(Vec<AgentSnapshot>, bool)> {
    let roster = repo.list_agent_roster(brief_id).await?;

    if roster.is_empty() {
        return Ok((vec![], false));
    }

    // Load child workflow edges for dependency resolution
    let child_edges = if let Some(cwf_id) = child_workflow_id {
        repo.list_edges(cwf_id).await.unwrap_or_default()
    } else {
        vec![]
    };

    // Map child_step_id → agent name for edge resolution
    let step_to_name: HashMap<Uuid, &str> = roster
        .iter()
        .filter_map(|a| a.child_step_id.map(|csid| (csid, a.name.as_str())))
        .collect();

    let agent_step_ids: HashSet<Uuid> = step_to_name.keys().copied().collect();

    // Build receives_from map
    let mut receives_from_map: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut has_deps = false;

    for edge in &child_edges {
        if agent_step_ids.contains(&edge.from_step_id) && agent_step_ids.contains(&edge.to_step_id)
        {
            if let (Some(&from_name), Some(&to_name)) = (
                step_to_name.get(&edge.from_step_id),
                step_to_name.get(&edge.to_step_id),
            ) {
                receives_from_map
                    .entry(to_name)
                    .or_default()
                    .push(from_name);
                has_deps = true;
            }
        }
    }

    let agents = roster
        .iter()
        .map(|a| {
            let receives_from = receives_from_map
                .get(a.name.as_str())
                .map(|v| v.iter().map(|s| s.to_string()).collect())
                .unwrap_or_default();

            AgentSnapshot {
                id: a.id,
                name: a.name.clone(),
                role_description: a.role_description.clone(),
                capabilities: a.capabilities.clone(),
                receives_from,
            }
        })
        .collect();

    Ok((agents, has_deps))
}

// ============================================================================
// Upstream & Incoming Context
// ============================================================================

/// Resolve upstream step IDs to names, using the pre-loaded map if available.
async fn resolve_upstream_names(
    upstream_ids: &[Uuid],
    steps_map: Option<&HashMap<Uuid, WorkflowStepRow>>,
    repo: &dyn WorkflowRepo,
) -> Vec<String> {
    let mut names = Vec::new();
    for &uid in upstream_ids {
        let name = if let Some(map) = steps_map {
            map.get(&uid).and_then(|s| s.name.clone())
        } else {
            repo.get_step(uid).await.ok().flatten().and_then(|s| s.name)
        };
        names.push(name.unwrap_or_else(|| format!("Step {}", uid)));
    }
    names
}

/// Build incoming context snapshots from upstream step IDs.
async fn build_incoming_context(
    upstream_ids: &[Uuid],
    steps_map: Option<&HashMap<Uuid, WorkflowStepRow>>,
    repo: &dyn WorkflowRepo,
) -> Result<Vec<IncomingContextSnapshot>> {
    let mut result = Vec::new();

    for &uid in upstream_ids {
        let upstream = if let Some(map) = steps_map {
            map.get(&uid).cloned()
        } else {
            repo.get_step(uid).await.ok().flatten()
        };

        let Some(upstream) = upstream else {
            continue;
        };

        let (status, preview, word_count) = classify_content_status(&upstream);
        let name = upstream
            .name
            .unwrap_or_else(|| format!("Step {}", upstream.id));

        result.push(IncomingContextSnapshot {
            name,
            source_mode: upstream.execution_mode.clone(),
            status: status.to_string(),
            preview,
            word_count,
        });
    }

    Ok(result)
}

// ============================================================================
// Port Loading
// ============================================================================

/// Load typed input ports for L4.
async fn load_input_ports(
    repo: &dyn WorkflowRepo,
    step_id: Uuid,
    workflow_edges: &[WorkflowStepEdgeRow],
    steps_map: Option<&HashMap<Uuid, WorkflowStepRow>>,
) -> Result<Vec<InputPortSnapshot>> {
    let rows = repo.get_step_inputs(step_id).await?;
    let mut result = Vec::new();

    for row in rows {
        // Find the edge that connects to this input port to resolve source node + json_path
        let edge = workflow_edges.iter().find(|e| {
            e.to_step_id == step_id && e.to_input_port.as_deref() == Some(&row.port_name)
        });

        let from_node = edge
            .and_then(|e| {
                if let Some(map) = steps_map {
                    map.get(&e.from_step_id).and_then(|s| s.name.clone())
                } else {
                    None
                }
            })
            .unwrap_or_else(|| "(unconnected)".to_string());

        let json_path = edge.and_then(|e| e.transform_jsonpath.clone());

        result.push(InputPortSnapshot {
            port_name: row.port_name,
            from_node,
            schema: row.json_schema.map(|s| s.to_string()),
            json_path,
        });
    }

    Ok(result)
}

/// Load typed output ports for L4.
async fn load_output_ports(
    repo: &dyn WorkflowRepo,
    step_id: Uuid,
    workflow_edges: &[WorkflowStepEdgeRow],
    steps_map: Option<&HashMap<Uuid, WorkflowStepRow>>,
) -> Result<Vec<OutputPortSnapshot>> {
    let rows = repo.get_step_outputs(step_id).await?;
    let mut result = Vec::new();

    for row in rows {
        let to_node = workflow_edges
            .iter()
            .find(|e| {
                e.from_step_id == step_id && e.from_output_port.as_deref() == Some(&row.port_name)
            })
            .and_then(|e| {
                if let Some(map) = steps_map {
                    map.get(&e.to_step_id).and_then(|s| s.name.clone())
                } else {
                    None
                }
            })
            .unwrap_or_else(|| "(unconnected)".to_string());

        result.push(OutputPortSnapshot {
            port_name: row.port_name,
            to_node,
            schema: row.json_schema.map(|s| s.to_string()),
        });
    }

    Ok(result)
}

// ============================================================================
// Status & Summary Derivation
// ============================================================================

/// Derive a node's status from its current state.
fn derive_node_status(step: &WorkflowStepRow, has_task: bool, agent_count: usize) -> String {
    if step.pinned {
        "completed".to_string()
    } else if !step.run_results_summary.is_empty() {
        "has_output".to_string()
    } else if agent_count > 0 && has_task {
        "configured".to_string()
    } else if has_task {
        "configuring".to_string()
    } else {
        "idle".to_string()
    }
}

/// Derive a compressed summary line for L1/L2 display.
fn derive_node_summary(has_task: bool, agent_count: usize, has_deps: bool) -> String {
    if agent_count == 0 && !has_task {
        return "Not configured".to_string();
    }

    let mut parts = Vec::new();

    if agent_count > 0 {
        parts.push(format!(
            "{} agent{}",
            agent_count,
            if agent_count == 1 { "" } else { "s" }
        ));
    }

    if has_task {
        parts.push("task set".to_string());
    } else {
        parts.push("no task yet".to_string());
    }

    if has_deps && agent_count > 1 {
        parts.push("dependencies set".to_string());
    }

    parts.join(", ")
}
