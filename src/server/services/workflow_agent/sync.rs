//! Repo → DB sync: reads board repo files, diffs against DB state,
//! and applies minimal mutations.
//!
//! Follows the same phased pattern as `system_node::sync::sync_to_db`.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use tracing::{info, warn};
use uuid::Uuid;

use super::file_reader;
use super::slug_to_display_name;
use crate::db::traits::WorkflowRepo;
use crate::db::{CanvasElementMapRow, WorkflowStepEdgeRow, WorkflowStepRow};
use crate::server::services::ServiceError;
use crate::server::state::AppState;
use crate::server::ws::events::{WorkflowEvent, WorkflowEventKind};

#[cfg(test)]
#[path = "sync_tests.rs"]
mod tests;

// ── Public types ───────────────────────────────────────────────────────────

/// Result of syncing board repo files to the DB.
#[derive(Debug, Default)]
pub(crate) struct SyncResult {
    pub nodes_created: Vec<String>,
    pub nodes_updated: Vec<String>,
    pub nodes_removed: Vec<String>,
    pub edges_created: usize,
    pub edges_removed: usize,
}

/// A node definition read from the filesystem for diffing against DB state.
#[derive(Debug, Clone)]
pub(crate) struct DesiredNode {
    pub slug: String,
    pub description: String,
    pub depends_on: Vec<String>,
}

// ── Public API ─────────────────────────────────────────────────────────────

/// Sync board repo files to DB state.
///
/// Phase 1: Read files (topology + nodes)
/// Phase 2: Diff nodes (create/update/remove steps)
/// Phase 3: Diff edges (add/remove)
/// Phase 4: Auto-layout (assign positions from topological levels)
/// Phase 5: Broadcast websocket updates
pub(crate) async fn sync_to_db(
    base_dir: &Path,
    workflow_id: Uuid,
    user_id: Uuid,
    repo: &dyn WorkflowRepo,
    state: &AppState,
) -> Result<SyncResult, ServiceError> {
    let mut result = SyncResult::default();

    // Phase 1: Read files
    let (topology, nodes_content) = file_reader::read_board(base_dir)
        .map_err(|e| ServiceError::Internal(anyhow::anyhow!("{e}")))?;

    let desired: Vec<DesiredNode> = topology
        .iter()
        .map(|(slug, deps)| DesiredNode {
            slug: slug.clone(),
            description: nodes_content.get(slug).cloned().unwrap_or_default(),
            depends_on: deps.clone(),
        })
        .collect();

    info!(
        workflow_id = %workflow_id,
        nodes = desired.len(),
        "Syncing board repo to DB"
    );

    // Phase 2: Diff + apply node mutations
    let current_steps = repo
        .list_steps(workflow_id)
        .await
        .map_err(ServiceError::Internal)?;

    let node_diff = diff_nodes(&desired, &current_steps);

    // Build slug → desired node lookup
    let desired_by_slug: HashMap<&str, &DesiredNode> =
        desired.iter().map(|n| (n.slug.as_str(), n)).collect();

    // Create new steps
    let max_order = current_steps
        .iter()
        .map(|s| s.display_order)
        .max()
        .unwrap_or(0);

    for (i, slug) in node_diff.to_create.iter().enumerate() {
        let node = desired_by_slug[slug.as_str()];
        let display_name = slug_to_display_name(slug);

        let step = WorkflowStepRow {
            id: Uuid::new_v4(),
            workflow_id,
            agent_id: None,
            execution_mode: "workforce".to_string(),
            agent_execution_mode: None,
            for_each_ref: None,
            prompt_template_id: None,
            prompt_template: String::new(),
            output_schema_id: None,
            output_variable_name: None,
            interactive_agent_id: None,
            for_each_label_field: None,
            room_id: None,
            routing_mode: None,
            routing_field: None,
            display_order: max_order + 1 + i as i32,
            version: 1,
            reasoning_trace: false,
            verification_agent_ids: None,
            position_x: None,
            position_y: None,
            width: None,
            height: None,
            name: Some(display_name),
            system_prompt_suffix: None,
            visible: true,
            description: node.description.clone(),
            board_context_cache: String::new(),
            board_context_updated_at: None,
            goal_summary: String::new(),
            goal_summary_updated_at: None,
            child_workflow_id: None,
            ref_id: Some(slug.clone()),
            pinned: false,
            run_results_summary: String::new(),
            designer_handoff: String::new(),
        };

        repo.create_step(step)
            .await
            .map_err(ServiceError::Internal)?;
        result.nodes_created.push(slug.clone());
    }

    // Update changed steps
    for (step_id, slug) in &node_diff.to_update {
        let node = desired_by_slug[slug.as_str()];
        let mut step = repo
            .get_step(*step_id)
            .await
            .map_err(ServiceError::Internal)?
            .ok_or_else(|| ServiceError::not_found("Step"))?;

        step.description = node.description.clone();
        repo.update_step(step)
            .await
            .map_err(ServiceError::Internal)?;
        result.nodes_updated.push(slug.clone());
    }

    // Remove deleted steps
    for (step_id, slug) in &node_diff.to_remove {
        repo.delete_step(*step_id)
            .await
            .map_err(ServiceError::Internal)?;
        result.nodes_removed.push(slug.clone());
    }

    // Phase 3: Diff + apply edge mutations
    // Reload steps to get IDs for newly created steps
    let updated_steps = repo
        .list_steps(workflow_id)
        .await
        .map_err(ServiceError::Internal)?;

    let slug_to_id: HashMap<String, Uuid> = updated_steps
        .iter()
        .filter_map(|s| s.ref_id.as_ref().map(|r| (r.clone(), s.id)))
        .collect();

    let current_edges = repo
        .list_edges(workflow_id)
        .await
        .map_err(ServiceError::Internal)?;

    let workforce_ids: HashSet<Uuid> = updated_steps
        .iter()
        .filter(|s| s.execution_mode == "workforce")
        .map(|s| s.id)
        .collect();

    let edge_diff = diff_edges(&desired, &slug_to_id, &current_edges, &workforce_ids);

    let mut created_edges: Vec<crate::db::WorkflowStepEdgeRow> = Vec::new();
    for (from_id, to_id) in &edge_diff.to_add {
        match repo.add_edge(workflow_id, *from_id, *to_id).await {
            Ok(edge) => {
                created_edges.push(edge);
                result.edges_created += 1;
            }
            Err(e) => warn!(error = %e, "Failed to add edge"),
        }
    }

    let mut removed_edges: Vec<(Uuid, Uuid, Uuid)> = Vec::new(); // (edge_id, from, to)
    for (from_id, to_id) in &edge_diff.to_remove {
        match repo.remove_edge(*from_id, *to_id).await {
            Ok(edge) => {
                removed_edges.push((edge.id, edge.from_step_id, edge.to_step_id));
                result.edges_removed += 1;
            }
            Err(e) => warn!(error = %e, "Failed to remove edge"),
        }
    }

    // Phase 4: Auto-layout (only newly created nodes — preserve user-set positions)
    if !result.nodes_created.is_empty() || result.edges_created > 0 || result.edges_removed > 0 {
        let fresh_steps = repo
            .list_steps(workflow_id)
            .await
            .map_err(ServiceError::Internal)?;
        let fresh_edges = repo
            .list_edges(workflow_id)
            .await
            .map_err(ServiceError::Internal)?;

        let workforce_steps: Vec<&WorkflowStepRow> = fresh_steps
            .iter()
            .filter(|s| s.execution_mode == "workforce")
            .collect();

        let positions = auto_layout(&workforce_steps, &fresh_edges);

        let new_step_ids: HashSet<Uuid> = result
            .nodes_created
            .iter()
            .filter_map(|slug| slug_to_id.get(slug).copied())
            .collect();

        for (step_id, x, y) in positions {
            if new_step_ids.contains(&step_id) {
                if let Ok(Some(mut step)) = repo.get_step(step_id).await {
                    step.position_x = Some(x);
                    step.position_y = Some(y);
                    let _ = repo.update_step(step).await;
                }
            }
        }
    }

    // Phase 5: Sync canvas elements BEFORE broadcasting (so frontend fetches fresh data)
    if !result.nodes_created.is_empty()
        || !result.nodes_removed.is_empty()
        || !result.nodes_updated.is_empty()
        || result.edges_created > 0
        || result.edges_removed > 0
    {
        if let Err(e) = sync_canvas_elements(workflow_id, user_id, repo, state).await {
            warn!(error = %e, "Failed to sync canvas elements");
        }
    }

    // Phase 6: Broadcast fine-grained events for real-time canvas updates
    for slug in &result.nodes_created {
        if let Some(&step_id) = slug_to_id.get(slug) {
            state.broadcast_workflow(WorkflowEvent {
                run_id: None,
                workflow_id,
                user_id: Some(user_id),
                kind: WorkflowEventKind::StepCreated {
                    step_id,
                    name: slug_to_display_name(slug),
                },
            });
        }
    }
    for (step_id, _slug) in &node_diff.to_remove {
        state.broadcast_workflow(WorkflowEvent {
            run_id: None,
            workflow_id,
            user_id: Some(user_id),
            kind: WorkflowEventKind::StepDeleted { step_id: *step_id },
        });
    }
    for slug in &result.nodes_updated {
        if let Some(&step_id) = slug_to_id.get(slug) {
            state.broadcast_workflow(WorkflowEvent {
                run_id: None,
                workflow_id,
                user_id: Some(user_id),
                kind: WorkflowEventKind::StepConfigUpdated { step_id },
            });
        }
    }
    for edge in &created_edges {
        state.broadcast_workflow(WorkflowEvent {
            run_id: None,
            workflow_id,
            user_id: Some(user_id),
            kind: WorkflowEventKind::EdgeCreated {
                edge_id: edge.id,
                from_step_id: edge.from_step_id,
                to_step_id: edge.to_step_id,
            },
        });
    }
    for (edge_id, from_id, to_id) in &removed_edges {
        state.broadcast_workflow(WorkflowEvent {
            run_id: None,
            workflow_id,
            user_id: Some(user_id),
            kind: WorkflowEventKind::EdgeDeleted {
                edge_id: *edge_id,
                from_step_id: *from_id,
                to_step_id: *to_id,
            },
        });
    }

    info!(
        workflow_id = %workflow_id,
        created = result.nodes_created.len(),
        updated = result.nodes_updated.len(),
        removed = result.nodes_removed.len(),
        edges_created = result.edges_created,
        edges_removed = result.edges_removed,
        "Board sync complete"
    );

    Ok(result)
}

// ── Canvas element generation ──────────────────────────────────────────────

/// Build Excalidraw JSON elements from workforce steps + edges.
///
/// Pure function — no DB writes. Returns a JSON array of rectangles, text, and arrows
/// in the same format as the frontend's serialize.ts.
pub(crate) fn build_canvas_elements(
    workforce_steps: &[&WorkflowStepRow],
    edges: &[WorkflowStepEdgeRow],
) -> Vec<serde_json::Value> {
    let mut elements = Vec::new();

    const MAX_BOX_WIDTH: f64 = 400.0;
    const MIN_BOX_WIDTH: f64 = 200.0;
    const MIN_BOX_HEIGHT: f64 = 48.0;
    const PAD_X: f64 = 20.0;
    const PAD_Y: f64 = 16.0;
    const CHAR_WIDTH: f64 = 9.6; // 16px font * 0.6
    const LINE_HEIGHT: f64 = 22.4; // 16px font * 1.4

    for step in workforce_steps {
        let x = step.position_x.unwrap_or(100.0);
        let y = step.position_y.unwrap_or(100.0);

        // Use the full description (markdown brief) as box text
        let text = if step.description.is_empty() {
            step.name
                .as_deref()
                .or(step.ref_id.as_deref())
                .unwrap_or("Node")
                .to_string()
        } else {
            step.description.clone()
        };

        // Estimate box size from text content
        let max_content_width = MAX_BOX_WIDTH - PAD_X * 2.0;
        let lines: Vec<&str> = text.lines().collect();
        let mut total_lines = 0.0_f64;
        for line in &lines {
            let line_width = line.len() as f64 * CHAR_WIDTH;
            let wrapped = (line_width / max_content_width).ceil().max(1.0);
            total_lines += wrapped;
        }
        let content_height = total_lines * LINE_HEIGHT;
        let longest_line = lines.iter().map(|l| l.len()).max().unwrap_or(10) as f64 * CHAR_WIDTH;
        let w = step
            .width
            .unwrap_or_else(|| (longest_line + PAD_X * 2.0).clamp(MIN_BOX_WIDTH, MAX_BOX_WIDTH));
        // Add 20% safety margin for word-wrap differences between estimation and renderer
        let h = step
            .height
            .unwrap_or_else(|| (content_height * 1.2 + PAD_Y * 2.0).max(MIN_BOX_HEIGHT));
        let text_id = format!("{}-text", step.id);

        // Connected arrow IDs for boundElements
        let arrow_ids: Vec<&Uuid> = edges
            .iter()
            .filter(|e| e.from_step_id == step.id || e.to_step_id == step.id)
            .map(|e| &e.id)
            .collect();

        let mut bound_elements = vec![serde_json::json!({ "id": text_id, "type": "text" })];
        for aid in &arrow_ids {
            bound_elements.push(serde_json::json!({ "id": aid.to_string(), "type": "arrow" }));
        }

        // Rectangle
        elements.push(serde_json::json!({
            "type": "rectangle",
            "id": step.id.to_string(),
            "x": x,
            "y": y,
            "width": w,
            "height": h,
            "isDeleted": false,
            "boundElements": bound_elements,
        }));

        // Text
        elements.push(serde_json::json!({
            "type": "text",
            "id": text_id,
            "x": x + PAD_X,
            "y": y + PAD_Y,
            "width": (w - PAD_X * 2.0).max(0.0),
            "height": (h - PAD_Y * 2.0).max(0.0),
            "isDeleted": false,
            "text": text,
            "containerId": step.id.to_string(),
        }));
    }

    // Arrows
    for edge in edges {
        elements.push(serde_json::json!({
            "type": "arrow",
            "id": edge.id.to_string(),
            "x": 0,
            "y": 0,
            "width": 0,
            "height": 0,
            "isDeleted": false,
            "startBinding": { "elementId": edge.from_step_id.to_string() },
            "endBinding": { "elementId": edge.to_step_id.to_string() },
        }));
    }

    elements
}

/// Generate Excalidraw JSON elements from steps + edges, upsert element maps
/// and canvas snapshot, then broadcast.
///
/// Produces the same format as the frontend's serialize.ts: rectangles with bound text
/// elements, and arrows with start/end bindings. The Board component loads these on
/// page refresh via getBoardElements.
pub(crate) async fn sync_canvas_elements(
    workflow_id: Uuid,
    user_id: Uuid,
    repo: &dyn WorkflowRepo,
    state: &AppState,
) -> Result<(), ServiceError> {
    let steps = repo
        .list_steps(workflow_id)
        .await
        .map_err(ServiceError::Internal)?;
    let edges = repo
        .list_edges(workflow_id)
        .await
        .map_err(ServiceError::Internal)?;

    let workforce_steps: Vec<&WorkflowStepRow> = steps
        .iter()
        .filter(|s| s.execution_mode == "workforce")
        .collect();

    let elements = build_canvas_elements(&workforce_steps, &edges);

    // Ensure element maps exist so canvas sync can resolve element IDs
    for step in &workforce_steps {
        repo.upsert_element_map(CanvasElementMapRow {
            workflow_id,
            element_id: step.id.to_string(),
            step_id: Some(step.id),
            edge_id: None,
            created_at: chrono::Utc::now(),
        })
        .await
        .map_err(ServiceError::Internal)?;
    }
    for edge in &edges {
        repo.upsert_element_map(CanvasElementMapRow {
            workflow_id,
            element_id: edge.id.to_string(),
            step_id: None,
            edge_id: Some(edge.id),
            created_at: chrono::Utc::now(),
        })
        .await
        .map_err(ServiceError::Internal)?;
    }

    let elements_json = serde_json::to_string(&elements).map_err(|e| {
        ServiceError::Internal(anyhow::anyhow!("Failed to serialize elements: {e}"))
    })?;

    let row = crate::db::CanvasSnapshotRow {
        workflow_id,
        snapshot_json: String::new(), // Not used by getBoardElements
        elements_json,
        last_response_json: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };

    repo.upsert_canvas_snapshot(row)
        .await
        .map_err(ServiceError::Internal)?;

    // Broadcast so frontend reloads board elements
    state.broadcast_workflow(WorkflowEvent {
        run_id: None,
        workflow_id,
        user_id: Some(user_id),
        kind: WorkflowEventKind::BoardElementsUpdated {},
    });

    Ok(())
}

// ── Pure diff helpers (testable without DB) ────────────────────────────────

/// Result of diffing desired nodes against current DB steps.
#[derive(Debug, Default)]
pub(crate) struct NodeDiff {
    /// Slugs to create (not in current steps).
    pub to_create: Vec<String>,
    /// (step_id, slug) pairs to update (description changed).
    pub to_update: Vec<(Uuid, String)>,
    /// (step_id, slug) pairs to remove (not in desired).
    pub to_remove: Vec<(Uuid, String)>,
}

/// Diff desired nodes against current steps.
///
/// Matches by `ref_id` (slug). Detects description changes.
pub(crate) fn diff_nodes(desired: &[DesiredNode], current: &[WorkflowStepRow]) -> NodeDiff {
    let current_by_slug: HashMap<&str, &WorkflowStepRow> = current
        .iter()
        .filter(|s| s.execution_mode == "workforce")
        .filter_map(|s| s.ref_id.as_deref().map(|r| (r, s)))
        .collect();

    let mut diff = NodeDiff::default();
    let mut matched_ids: HashSet<Uuid> = HashSet::new();

    for node in desired {
        if let Some(step) = current_by_slug.get(node.slug.as_str()) {
            matched_ids.insert(step.id);

            if step.description != node.description {
                diff.to_update.push((step.id, node.slug.clone()));
            }
        } else {
            diff.to_create.push(node.slug.clone());
        }
    }

    let desired_slugs: HashSet<&str> = desired.iter().map(|n| n.slug.as_str()).collect();
    for step in current {
        if step.execution_mode != "workforce" {
            continue;
        }
        if !matched_ids.contains(&step.id) {
            if let Some(ref_id) = &step.ref_id {
                if !desired_slugs.contains(ref_id.as_str()) {
                    diff.to_remove.push((step.id, ref_id.clone()));
                }
            }
        }
    }

    diff
}

/// Result of diffing desired edges against current DB edges.
#[derive(Debug, Default)]
pub(crate) struct EdgeDiff {
    /// (from_step_id, to_step_id) pairs to add.
    pub to_add: Vec<(Uuid, Uuid)>,
    /// (from_step_id, to_step_id) pairs to remove.
    pub to_remove: Vec<(Uuid, Uuid)>,
}

/// Diff desired edges against current edges.
///
/// Only considers workforce-to-workforce edges.
pub(crate) fn diff_edges(
    desired: &[DesiredNode],
    slug_to_id: &HashMap<String, Uuid>,
    current_edges: &[WorkflowStepEdgeRow],
    workforce_ids: &HashSet<Uuid>,
) -> EdgeDiff {
    // Build desired edge set from topology depends_on
    let mut desired_set: HashSet<(Uuid, Uuid)> = HashSet::new();
    for node in desired {
        if let Some(&to_id) = slug_to_id.get(&node.slug) {
            for dep in &node.depends_on {
                if let Some(&from_id) = slug_to_id.get(dep) {
                    desired_set.insert((from_id, to_id));
                }
            }
        }
    }

    // Build current workforce-only edge set
    let current_set: HashSet<(Uuid, Uuid)> = current_edges
        .iter()
        .filter(|e| {
            workforce_ids.contains(&e.from_step_id) && workforce_ids.contains(&e.to_step_id)
        })
        .map(|e| (e.from_step_id, e.to_step_id))
        .collect();

    let to_add: Vec<(Uuid, Uuid)> = desired_set.difference(&current_set).copied().collect();
    let to_remove: Vec<(Uuid, Uuid)> = current_set.difference(&desired_set).copied().collect();

    EdgeDiff { to_add, to_remove }
}

// ── Auto-layout ────────────────────────────────────────────────────────────

const LEVEL_SPACING_X: f64 = 400.0;
const NODE_SPACING_Y: f64 = 200.0;
const INITIAL_OFFSET_X: f64 = 100.0;
const INITIAL_OFFSET_Y: f64 = 100.0;

/// Assign positions based on topological levels (Kahn's algorithm).
///
/// Level 0 nodes at x=INITIAL_OFFSET_X, level 1 at x + LEVEL_SPACING_X, etc.
/// Within a level, nodes are spaced vertically.
pub(crate) fn auto_layout(
    steps: &[&WorkflowStepRow],
    edges: &[WorkflowStepEdgeRow],
) -> Vec<(Uuid, f64, f64)> {
    let step_ids: HashSet<Uuid> = steps.iter().map(|s| s.id).collect();

    // Build adjacency and in-degree
    let mut in_degree: HashMap<Uuid, usize> = steps.iter().map(|s| (s.id, 0)).collect();
    let mut adjacency: HashMap<Uuid, Vec<Uuid>> =
        steps.iter().map(|s| (s.id, Vec::new())).collect();

    for edge in edges {
        if step_ids.contains(&edge.from_step_id) && step_ids.contains(&edge.to_step_id) {
            adjacency
                .entry(edge.from_step_id)
                .or_default()
                .push(edge.to_step_id);
            *in_degree.entry(edge.to_step_id).or_default() += 1;
        }
    }

    // BFS by levels
    let mut levels: Vec<Vec<Uuid>> = Vec::new();
    let mut queue: Vec<Uuid> = in_degree
        .iter()
        .filter(|(_, &deg)| deg == 0)
        .map(|(&id, _)| id)
        .collect();
    queue.sort(); // deterministic ordering

    while !queue.is_empty() {
        let current_level = queue.clone();
        levels.push(current_level.clone());
        queue.clear();

        for &node in &current_level {
            if let Some(neighbors) = adjacency.get(&node) {
                for &next in neighbors {
                    if let Some(deg) = in_degree.get_mut(&next) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push(next);
                        }
                    }
                }
            }
        }
        queue.sort();
    }

    // Assign positions
    let mut positions = Vec::new();
    for (level_idx, level) in levels.iter().enumerate() {
        let x = INITIAL_OFFSET_X + level_idx as f64 * LEVEL_SPACING_X;
        for (node_idx, &step_id) in level.iter().enumerate() {
            let y = INITIAL_OFFSET_Y + node_idx as f64 * NODE_SPACING_Y;
            positions.push((step_id, x, y));
        }
    }

    positions
}
