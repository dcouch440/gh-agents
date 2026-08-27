//! File reader — reads the workflow agent's board repo from disk.
//!
//! Pure functions: reads `topology.json` + `nodes/*.md` from a base directory.
//! No DB access, no side effects.

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

#[cfg(test)]
#[path = "file_reader_tests.rs"]
mod tests;

// ── Types ──────────────────────────────────────────────────────────────────

/// Deserialized `topology.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct BoardTopology {
    pub nodes: HashMap<String, NodeEntry>,
}

/// Topology (slug → depends_on) paired with node contents (slug → markdown).
pub(crate) type Board = (HashMap<String, Vec<String>>, HashMap<String, String>);

/// A single node entry in `topology.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct NodeEntry {
    pub depends_on: Vec<String>,
}

// ── Public API ─────────────────────────────────────────────────────────────

/// Read `topology.json` and return slug → depends_on map.
pub(crate) fn read_topology(base_dir: &Path) -> Result<HashMap<String, Vec<String>>, String> {
    let topology_path = base_dir.join("topology.json");
    let content = std::fs::read_to_string(&topology_path)
        .map_err(|e| format!("cannot read topology.json: {e}"))?;

    let topology: BoardTopology = serde_json::from_str(&content)
        .map_err(|e| format!("invalid JSON in topology.json: {e}"))?;

    Ok(topology
        .nodes
        .into_iter()
        .map(|(slug, entry)| (slug, entry.depends_on))
        .collect())
}

/// Read all node files in the `nodes/` directory.
///
/// Returns slug → content map. Only reads `.md` files.
pub(crate) fn read_all_nodes(base_dir: &Path) -> Result<HashMap<String, String>, String> {
    let nodes_dir = base_dir.join("nodes");

    if !nodes_dir.exists() {
        return Ok(HashMap::new());
    }

    let entries =
        std::fs::read_dir(&nodes_dir).map_err(|e| format!("cannot read nodes/ directory: {e}"))?;

    let mut nodes = HashMap::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("md") {
            if let Some(slug) = path.file_stem().and_then(|s| s.to_str()) {
                let content = std::fs::read_to_string(&path)
                    .map_err(|e| format!("cannot read nodes/{slug}.md: {e}"))?;
                nodes.insert(slug.to_string(), content.trim().to_string());
            }
        }
    }

    Ok(nodes)
}

/// Read topology + all node contents in one call.
///
/// Returns `(topology_map, nodes_map)`.
pub(crate) fn read_board(base_dir: &Path) -> Result<Board, String> {
    let topology = read_topology(base_dir)?;
    let nodes = read_all_nodes(base_dir)?;
    Ok((topology, nodes))
}

/// Snapshot the board repo as relative_path → content.
///
/// Captures `topology.json` + all `nodes/*.md`. Used for pre/post diffing
/// around `run_command` to detect file changes for immediate sync.
pub(crate) fn snapshot_board_files(base_dir: &Path) -> HashMap<std::path::PathBuf, String> {
    let mut snapshot = HashMap::new();

    // topology.json
    let topology_path = base_dir.join("topology.json");
    if let Ok(content) = std::fs::read_to_string(&topology_path) {
        snapshot.insert(std::path::PathBuf::from("topology.json"), content);
    }

    // nodes/*.md
    let nodes_dir = base_dir.join("nodes");
    if let Ok(entries) = std::fs::read_dir(&nodes_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("md") {
                if let (Some(name), Ok(content)) = (
                    path.file_name().map(|n| n.to_string_lossy().to_string()),
                    std::fs::read_to_string(&path),
                ) {
                    snapshot.insert(std::path::PathBuf::from("nodes").join(&name), content);
                }
            }
        }
    }

    snapshot
}
