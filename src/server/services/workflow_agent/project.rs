//! DB → Repo projection: reads steps + edges from the DB and writes
//! `board.md` + `topology.json` + `nodes/*.md` to the board repo on disk.
//!
//! Full overwrite approach — the DB is the source of truth for this direction.

use std::collections::HashMap;
use std::path::Path;

use uuid::Uuid;

use super::file_reader::{BoardTopology, NodeEntry, BOARD_SPEC_FILE};
use super::{name_to_slug, next_unnamed_slug};
use crate::db::traits::WorkflowRepo;
use crate::db::WorkflowStepRow;
use crate::server::services::ServiceError;

#[cfg(test)]
#[path = "project_tests.rs"]
mod tests;

// ── Public API ─────────────────────────────────────────────────────────────

/// Project the current DB state to the board repo on disk.
///
/// 1. Reads all workforce steps for the workflow
/// 2. Maps each step to a slug from `ref_id` (or auto-generates one)
/// 3. Writes `topology.json` with nodes + depends_on from edges
/// 4. Writes `nodes/{slug}.md` with step descriptions
/// 5. Cleans up orphaned `.md` files
///
/// Returns the number of nodes projected.
pub(crate) async fn project_to_repo(
    base_dir: &Path,
    workflow_id: Uuid,
    repo: &dyn WorkflowRepo,
) -> Result<usize, ServiceError> {
    // Phase 1: Load steps + edges
    let steps = repo
        .list_steps(workflow_id)
        .await
        .map_err(ServiceError::Internal)?;
    let edges = repo
        .list_edges(workflow_id)
        .await
        .map_err(ServiceError::Internal)?;

    // Phase 2: Build slug mappings for workforce steps
    let projected = build_projected_nodes(&steps);

    // Build step_id → slug lookup for edge mapping
    let id_to_slug: HashMap<Uuid, &str> = projected
        .iter()
        .map(|n| (n.step_id, n.slug.as_str()))
        .collect();

    // Phase 3: Build topology
    let mut topology_nodes = HashMap::with_capacity(projected.len());
    for node in &projected {
        let depends_on: Vec<String> = edges
            .iter()
            .filter(|e| e.to_step_id == node.step_id)
            .filter_map(|e| id_to_slug.get(&e.from_step_id))
            .map(|s| s.to_string())
            .collect();

        topology_nodes.insert(node.slug.clone(), NodeEntry { depends_on });
    }

    let topology = BoardTopology {
        nodes: topology_nodes,
    };

    // Phase 4: Write files
    let nodes_dir = base_dir.join("nodes");
    std::fs::create_dir_all(&nodes_dir).map_err(|e| {
        ServiceError::Internal(anyhow::anyhow!("cannot create nodes/ directory: {e}"))
    })?;

    let topology_json = serde_json::to_string_pretty(&topology)
        .map_err(|e| ServiceError::Internal(anyhow::anyhow!("cannot serialize topology: {e}")))?;
    std::fs::write(base_dir.join("topology.json"), &topology_json)
        .map_err(|e| ServiceError::Internal(anyhow::anyhow!("cannot write topology.json: {e}")))?;

    for node in &projected {
        let node_path = nodes_dir.join(format!("{}.md", node.slug));
        std::fs::write(&node_path, &node.description).map_err(|e| {
            ServiceError::Internal(anyhow::anyhow!("cannot write nodes/{}.md: {e}", node.slug))
        })?;
    }

    // Phase 5: Write board.md
    let spec = repo
        .get_board_spec(workflow_id)
        .await
        .map_err(ServiceError::Internal)?;
    write_board_spec(base_dir, &spec)?;

    // Phase 6: Clean up orphaned .md files
    let valid_slugs: std::collections::HashSet<&str> =
        projected.iter().map(|n| n.slug.as_str()).collect();

    if let Ok(entries) = std::fs::read_dir(&nodes_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if let Some(slug) = name_str.strip_suffix(".md") {
                if !valid_slugs.contains(slug) {
                    let _ = std::fs::remove_file(entry.path());
                }
            }
        }
    }

    let count = projected.len();

    // Phase 7: Update ref_ids in DB for steps that got new slugs
    for node in &projected {
        if node.ref_id_changed {
            let mut step = repo
                .get_step(node.step_id)
                .await
                .map_err(ServiceError::Internal)?
                .ok_or_else(|| ServiceError::not_found("Step"))?;
            step.ref_id = Some(node.slug.clone());
            repo.update_step(step)
                .await
                .map_err(ServiceError::Internal)?;
        }
    }

    Ok(count)
}

// ── Internals ──────────────────────────────────────────────────────────────

/// Write `board.md`, or remove it when the board has no spec.
///
/// Removed rather than left stale: this direction is a full overwrite, the
/// agent reads the directory with `cat`, and a board.md holding contracts the
/// board no longer has is worse than no board.md at all. A spec that is only
/// whitespace is no spec.
pub(crate) fn write_board_spec(base_dir: &Path, spec: &str) -> Result<(), ServiceError> {
    let path = base_dir.join(BOARD_SPEC_FILE);
    if spec.trim().is_empty() {
        let _ = std::fs::remove_file(&path);
        return Ok(());
    }
    std::fs::write(&path, spec)
        .map_err(|e| ServiceError::Internal(anyhow::anyhow!("cannot write {BOARD_SPEC_FILE}: {e}")))
}

struct ProjectedNode {
    step_id: Uuid,
    slug: String,
    description: String,
    ref_id_changed: bool,
}

/// Build projected nodes from steps, assigning slugs.
///
/// Filters to workforce steps. Extracts slug from `ref_id` when it looks like
/// a valid slug, otherwise generates one from `step.name` or unnamed counter.
fn build_projected_nodes(steps: &[WorkflowStepRow]) -> Vec<ProjectedNode> {
    let workforce_steps: Vec<&WorkflowStepRow> = steps
        .iter()
        .filter(|s| s.execution_mode == "workforce")
        .collect();

    let mut used_slugs: Vec<String> = Vec::new();
    let mut projected = Vec::with_capacity(workforce_steps.len());

    for step in &workforce_steps {
        let (slug, changed) = resolve_slug(step, &used_slugs);
        used_slugs.push(slug.clone());
        projected.push(ProjectedNode {
            step_id: step.id,
            slug,
            description: step.description.clone(),
            ref_id_changed: changed,
        });
    }

    projected
}

/// Resolve the slug for a step.
///
/// Returns `(slug, ref_id_changed)`.
pub(crate) fn resolve_slug(step: &WorkflowStepRow, used_slugs: &[String]) -> (String, bool) {
    // If ref_id is already a valid slug, use it
    if let Some(ref_id) = &step.ref_id {
        if is_valid_slug(ref_id) {
            return (ref_id.clone(), false);
        }
    }

    // Generate from name
    if let Some(name) = &step.name {
        if !name.is_empty() {
            let slug = name_to_slug(name);
            if !used_slugs.contains(&slug) {
                return (slug, true);
            }
        }
    }

    // Fallback to unnamed
    let existing_refs: Vec<&str> = used_slugs.iter().map(|s| s.as_str()).collect();
    let slug = next_unnamed_slug(&existing_refs);
    (slug, true)
}

/// Check if a string is a valid slug: `[a-z][a-z0-9_]*`
pub(crate) fn is_valid_slug(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}
