//! Filesystem sync helpers for canvas live sync.
//!
//! Writes individual node files and regenerates topology.json from DB state.

use std::collections::HashMap;
use std::path::Path;

use crate::db::{WorkflowStepEdgeRow, WorkflowStepRow};
use crate::server::services::workflow_agent::file_reader::{BoardTopology, NodeEntry};

/// Write a single node markdown file.
pub(crate) fn write_node_file(base_dir: &Path, slug: &str, content: &str) -> Result<(), String> {
    let nodes_dir = base_dir.join("nodes");
    std::fs::create_dir_all(&nodes_dir)
        .map_err(|e| format!("cannot create nodes/ directory: {e}"))?;

    let path = nodes_dir.join(format!("{slug}.md"));
    std::fs::write(&path, content).map_err(|e| format!("cannot write nodes/{slug}.md: {e}"))
}

/// Remove a single node markdown file.
pub(crate) fn remove_node_file(base_dir: &Path, slug: &str) -> Result<(), String> {
    let path = base_dir.join("nodes").join(format!("{slug}.md"));
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| format!("cannot remove nodes/{slug}.md: {e}"))?;
    }
    Ok(())
}

/// Rewrite topology.json from the current DB steps and edges.
///
/// Full regeneration (not incremental). Only includes workforce steps.
pub(crate) fn rewrite_topology(
    base_dir: &Path,
    steps: &[WorkflowStepRow],
    edges: &[WorkflowStepEdgeRow],
) -> Result<(), String> {
    let workforce_steps: Vec<&WorkflowStepRow> = steps
        .iter()
        .filter(|s| s.execution_mode == "workforce")
        .collect();

    // Build step_id -> slug lookup
    let id_to_slug: HashMap<uuid::Uuid, &str> = workforce_steps
        .iter()
        .filter_map(|s| s.ref_id.as_deref().map(|slug| (s.id, slug)))
        .collect();

    // Build topology nodes
    let mut topology_nodes = HashMap::with_capacity(workforce_steps.len());
    for step in &workforce_steps {
        if let Some(slug) = step.ref_id.as_deref() {
            let depends_on: Vec<String> = edges
                .iter()
                .filter(|e| e.to_step_id == step.id)
                .filter_map(|e| id_to_slug.get(&e.from_step_id))
                .map(|s| s.to_string())
                .collect();

            topology_nodes.insert(slug.to_string(), NodeEntry { depends_on });
        }
    }

    let topology = BoardTopology {
        nodes: topology_nodes,
    };

    std::fs::create_dir_all(base_dir).map_err(|e| format!("cannot create base directory: {e}"))?;

    let json = serde_json::to_string_pretty(&topology)
        .map_err(|e| format!("cannot serialize topology: {e}"))?;
    std::fs::write(base_dir.join("topology.json"), &json)
        .map_err(|e| format!("cannot write topology.json: {e}"))
}
