//! Phase 0 structural executor — consumes a [`FilteredChangeset`] and performs
//! agentless DB writes to materialize canvas drawings into workflow steps and edges.
//!
//! Execution order is dependency-driven:
//! 1. Create new nodes (so edges can reference them)
//! 2. Create new edges (nodes must exist first)
//! 3. Update existing nodes (text/annotation edits)
//! 4. Delete edges (before nodes, to avoid FK violations)
//! 5. Rewire edges (update endpoints)
//! 6. Delete nodes (edges already cleaned up)
//! 7. Move nodes (position updates)

use std::collections::HashMap;

use uuid::Uuid;

use crate::db::traits::{SessionRepo, WorkflowRepo};
use crate::db::{CanvasElementMapRow, WorkflowStepRow};
use crate::server::hub::board_serializer::{CanvasNode, FilteredChangeset, ScoredChange};
use crate::server::services::steps::{self, CreateStepInput, StepPayload};
use crate::server::services::ServiceError;

/// Result of Phase 0 structural execution.
#[derive(Debug, Clone)]
pub struct PhaseZeroResult {
    /// Steps created from new canvas nodes: (element_id, full_step_row).
    pub created_steps: Vec<(String, WorkflowStepRow)>,
    /// Edges created from new canvas edges: (element_id, edge_id).
    pub created_edges: Vec<(String, Uuid)>,
    /// Element IDs of deleted steps.
    pub deleted_steps: Vec<String>,
    /// Element IDs of deleted edges.
    pub deleted_edges: Vec<String>,
    /// Element IDs of rewired edges.
    pub rewired_edges: Vec<String>,
    /// Element IDs of moved steps.
    pub moved_steps: Vec<String>,
    /// Steps updated from canvas node text/annotation edits: (element_id, full_step_row).
    pub updated_steps: Vec<(String, WorkflowStepRow)>,
}

/// Execute Phase 0: apply structural changes from a board submit as DB writes.
///
/// This is the agentless layer — no LLM calls. It creates workflow steps from
/// canvas nodes, wires edges, handles deletions and rewires, and updates positions.
///
/// The `FilteredChangeset` contains three tiers:
/// - **Agentless**: deletes, rewires, moves — all handled here
/// - **Meaningful**: new nodes, new edges, and updated nodes — all handled here
/// - **Noise**: ignored
pub async fn execute_phase_zero(
    repo: &dyn WorkflowRepo,
    session_repo: &dyn SessionRepo,
    workflow_id: Uuid,
    user_id: Uuid,
    changeset: &FilteredChangeset,
) -> Result<PhaseZeroResult, ServiceError> {
    // Load all existing element→step/edge mappings into memory to avoid N+1 queries.
    let existing_maps = repo.list_element_maps(workflow_id).await?;
    let mut element_map: HashMap<String, CanvasElementMapRow> = existing_maps
        .into_iter()
        .map(|row| (row.element_id.clone(), row))
        .collect();

    let mut result = PhaseZeroResult {
        created_steps: Vec::new(),
        created_edges: Vec::new(),
        deleted_steps: Vec::new(),
        deleted_edges: Vec::new(),
        rewired_edges: Vec::new(),
        moved_steps: Vec::new(),
        updated_steps: Vec::new(),
    };

    // ── 1. Create new nodes ─────────────────────────────────────────────────
    for change in &changeset.meaningful {
        if let ScoredChange::NewNode { node, .. } = change {
            let step_row = create_node(repo, workflow_id, user_id, node, &mut element_map).await?;
            result
                .created_steps
                .push((node.element_id.clone(), step_row));
        }
    }

    // ── 2. Create new edges ─────────────────────────────────────────────────
    for change in &changeset.meaningful {
        if let ScoredChange::NewEdge { edge, .. } = change {
            let from_step_id = resolve_step_id(&element_map, &edge.source_node_id)?;
            let to_step_id = resolve_step_id(&element_map, &edge.target_node_id)?;

            let edge_row = repo.add_edge(workflow_id, from_step_id, to_step_id).await?;

            let map_row = CanvasElementMapRow {
                workflow_id,
                element_id: edge.element_id.clone(),
                step_id: None,
                edge_id: Some(edge_row.id),
                created_at: chrono::Utc::now(),
            };
            repo.upsert_element_map(map_row.clone()).await?;
            element_map.insert(edge.element_id.clone(), map_row);

            result
                .created_edges
                .push((edge.element_id.clone(), edge_row.id));
        }
    }

    // ── 3. Update existing nodes (text/annotation edits) ───────────────────
    for change in &changeset.meaningful {
        if let ScoredChange::UpdatedNode { update, .. } = change {
            let step_id = resolve_step_id(&element_map, &update.element_id)?;
            let mut step = repo
                .get_step(step_id)
                .await?
                .ok_or_else(|| ServiceError::not_found("Step"))?;

            let (name, prompt_template) = parse_node_text(&update.new_text);
            step.name = Some(name);
            step.prompt_template = prompt_template;

            let new_context = build_board_context(&update.new_annotations, &None, &None);
            if !new_context.is_empty() || !step.board_context_cache.is_empty() {
                step.board_context_cache = new_context;
                step.board_context_updated_at = Some(chrono::Utc::now());
            }

            let updated_step = repo.update_step(step).await?;
            result
                .updated_steps
                .push((update.element_id.clone(), updated_step));
        }
    }

    // ── 4. Delete edges (before nodes to avoid FK violations) ───────────────
    for element_id in &changeset.agentless.deleted_edge_ids {
        let edge_id = resolve_edge_id(&element_map, element_id)?;
        let _deleted = repo.delete_edge_by_id(edge_id).await?;
        repo.delete_element_map(workflow_id, element_id).await?;
        element_map.remove(element_id);
        result.deleted_edges.push(element_id.clone());
    }

    // ── 5. Rewire edges ─────────────────────────────────────────────────────
    for rewire in &changeset.agentless.rewired_edges {
        let old_edge_id = resolve_edge_id(&element_map, &rewire.element_id)?;
        let new_from = resolve_step_id(&element_map, &rewire.new_source)?;
        let new_to = resolve_step_id(&element_map, &rewire.new_target)?;

        // Replace old edge with new one.
        let _deleted = repo.delete_edge_by_id(old_edge_id).await?;
        let new_edge = repo.add_edge(workflow_id, new_from, new_to).await?;

        // Update the mapping to point to the new edge.
        let map_row = CanvasElementMapRow {
            workflow_id,
            element_id: rewire.element_id.clone(),
            step_id: None,
            edge_id: Some(new_edge.id),
            created_at: chrono::Utc::now(),
        };
        repo.upsert_element_map(map_row.clone()).await?;
        element_map.insert(rewire.element_id.clone(), map_row);

        result.rewired_edges.push(rewire.element_id.clone());
    }

    // ── 6. Delete nodes ─────────────────────────────────────────────────────
    for element_id in &changeset.agentless.deleted_node_ids {
        let step_id = resolve_step_id(&element_map, element_id)?;
        steps::delete_step(repo, session_repo, user_id, workflow_id, step_id).await?;
        repo.delete_element_map(workflow_id, element_id).await?;
        element_map.remove(element_id);
        result.deleted_steps.push(element_id.clone());
    }

    // ── 7. Move nodes ───────────────────────────────────────────────────────
    for node_move in &changeset.agentless.moved_nodes {
        let step_id = resolve_step_id(&element_map, &node_move.element_id)?;
        let mut step = repo
            .get_step(step_id)
            .await?
            .ok_or_else(|| ServiceError::not_found("Step"))?;

        step.position_x = Some(node_move.new_bounds.x);
        step.position_y = Some(node_move.new_bounds.y);
        step.width = Some(node_move.new_bounds.width);
        step.height = Some(node_move.new_bounds.height);

        repo.update_step(step).await?;
        result.moved_steps.push(node_move.element_id.clone());
    }

    Ok(result)
}

/// Create a workflow step from a canvas node and insert the element mapping.
/// Returns the full [`WorkflowStepRow`] for inclusion in the API response.
async fn create_node(
    repo: &dyn WorkflowRepo,
    workflow_id: Uuid,
    user_id: Uuid,
    node: &CanvasNode,
    element_map: &mut HashMap<String, CanvasElementMapRow>,
) -> Result<WorkflowStepRow, ServiceError> {
    let (name, prompt_template) = parse_node_text(&node.raw_text);
    let board_context_cache =
        build_board_context(&node.annotations, &node.sketch, &node.stroke_encoding);

    let step = steps::create_step(
        repo,
        CreateStepInput {
            workflow_id,
            user_id,
            payload: StepPayload {
                name: Some(name),
                execution_mode: Some("workforce".to_string()),
                prompt_template: Some(prompt_template),
                position_x: Some(node.bounds.x),
                position_y: Some(node.bounds.y),
                width: Some(node.bounds.width),
                height: Some(node.bounds.height),
                ..StepPayload::default()
            },
        },
    )
    .await?;

    // Store the user's annotations and sketch as supplementary context.
    let step = if !board_context_cache.is_empty() {
        let mut updated = step.clone();
        updated.board_context_cache = board_context_cache;
        updated.board_context_updated_at = Some(chrono::Utc::now());
        repo.update_step(updated).await?
    } else {
        step
    };

    // Insert element → step mapping.
    let map_row = CanvasElementMapRow {
        workflow_id,
        element_id: node.element_id.clone(),
        step_id: Some(step.id),
        edge_id: None,
        created_at: chrono::Utc::now(),
    };
    repo.upsert_element_map(map_row.clone()).await?;
    element_map.insert(node.element_id.clone(), map_row);

    Ok(step)
}

/// Parse raw box text into a step name and prompt template.
///
/// The first non-empty line becomes the step name (display in tree).
/// The full text becomes the prompt template (the user's instruction).
fn parse_node_text(raw_text: &str) -> (String, String) {
    let name = raw_text
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("Untitled")
        .trim()
        .to_string();

    (name, raw_text.to_string())
}

/// Build the board_context_cache from annotations and optional sketch.
///
/// Format:
/// ```text
/// ## Annotations
/// - First annotation
/// - Second annotation
///
/// ## Sketch
/// ██··██
/// ·████·
/// ```
fn build_board_context(
    annotations: &[String],
    sketch: &Option<String>,
    stroke_encoding: &Option<String>,
) -> String {
    let mut parts = Vec::new();

    if !annotations.is_empty() {
        let mut section = String::from("## Annotations\n");
        for ann in annotations {
            section.push_str("- ");
            section.push_str(ann);
            section.push('\n');
        }
        parts.push(section);
    }

    // Prefer stroke_encoding (compact JSON coordinates) over ASCII sketch.
    if let Some(encoding) = stroke_encoding {
        let mut section = String::from("## Stroke Coordinates\n");
        section.push_str(encoding);
        section.push('\n');
        parts.push(section);
    } else if let Some(sketch_data) = sketch {
        let mut section = String::from("## Sketch\n");
        section.push_str(sketch_data);
        section.push('\n');
        parts.push(section);
    }

    parts.join("\n")
}

/// Resolve an element_id to a step_id from the in-memory map.
fn resolve_step_id(
    element_map: &HashMap<String, CanvasElementMapRow>,
    element_id: &str,
) -> Result<Uuid, ServiceError> {
    element_map
        .get(element_id)
        .and_then(|row| row.step_id)
        .ok_or_else(|| {
            ServiceError::validation(format!(
                "No step mapping found for canvas element '{element_id}'"
            ))
        })
}

/// Resolve an element_id to an edge_id from the in-memory map.
fn resolve_edge_id(
    element_map: &HashMap<String, CanvasElementMapRow>,
    element_id: &str,
) -> Result<Uuid, ServiceError> {
    element_map
        .get(element_id)
        .and_then(|row| row.edge_id)
        .ok_or_else(|| {
            ServiceError::validation(format!(
                "No edge mapping found for canvas element '{element_id}'"
            ))
        })
}

#[cfg(test)]
#[path = "executor_tests.rs"]
mod executor_tests;
