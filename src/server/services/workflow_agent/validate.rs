//! Validation for the workflow agent's board repo files.
//!
//! All functions are pure — no DB access. Reads files from the base directory
//! for cross-reference checks.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use super::file_reader;

#[cfg(test)]
#[path = "validate_tests.rs"]
mod tests;

// ── Per-file validators ────────────────────────────────────────────────────

/// Validate `topology.json` content.
///
/// Checks: valid JSON, has "nodes" object, each node has "depends_on" array of strings.
pub(crate) fn validate_topology(content: &str) -> Result<(), String> {
    let val: serde_json::Value =
        serde_json::from_str(content).map_err(|e| format!("invalid JSON: {e}"))?;

    let obj = val.as_object().ok_or("expected a JSON object")?;

    let nodes = obj
        .get("nodes")
        .and_then(|v| v.as_object())
        .ok_or("missing or invalid \"nodes\" object")?;

    for (slug, entry) in nodes {
        let deps = entry
            .get("depends_on")
            .and_then(|v| v.as_array())
            .ok_or_else(|| format!("node \"{slug}\" missing \"depends_on\" array"))?;

        for dep in deps {
            if !dep.is_string() {
                return Err(format!(
                    "node \"{slug}\" has non-string value in depends_on"
                ));
            }
        }
    }

    Ok(())
}

/// Validate a node markdown file content.
pub(crate) fn validate_node(content: &str, slug: &str) -> Result<(), String> {
    if content.trim().is_empty() {
        return Err(format!("nodes/{slug}.md is empty"));
    }
    Ok(())
}

// ── Cycle detection ────────────────────────────────────────────────────────

/// Check topology for cycles using DFS with in-stack tracking.
///
/// Returns `Ok(())` if acyclic, or `Err` with the cycle path.
pub(crate) fn detect_cycles(topology: &HashMap<String, Vec<String>>) -> Result<(), String> {
    let mut visited = HashSet::new();
    let mut in_stack = HashSet::new();

    for slug in topology.keys() {
        if !visited.contains(slug.as_str()) {
            let mut path = Vec::new();
            if has_cycle(slug, topology, &mut visited, &mut in_stack, &mut path) {
                return Err(format!("cycle detected: {}", path.join(" -> ")));
            }
        }
    }

    Ok(())
}

fn has_cycle<'a>(
    node: &'a str,
    topology: &'a HashMap<String, Vec<String>>,
    visited: &mut HashSet<&'a str>,
    in_stack: &mut HashSet<&'a str>,
    path: &mut Vec<&'a str>,
) -> bool {
    visited.insert(node);
    in_stack.insert(node);
    path.push(node);

    if let Some(deps) = topology.get(node) {
        for dep in deps {
            if !visited.contains(dep.as_str()) {
                if let Some(dep_entry) = topology.get(dep.as_str()) {
                    // dep exists in topology — recurse
                    let _ = dep_entry; // just checking existence
                    if has_cycle(dep, topology, visited, in_stack, path) {
                        return true;
                    }
                }
                // dep not in topology — dangling reference, not a cycle
            } else if in_stack.contains(dep.as_str()) {
                path.push(dep);
                return true;
            }
        }
    }

    in_stack.remove(node);
    path.pop();
    false
}

// ── Cross-reference validation ─────────────────────────────────────────────

/// A single cross-reference error.
#[derive(Debug)]
pub(crate) struct CrossRefError {
    pub file: String,
    pub error: String,
}

/// Cross-reference topology slugs against node files.
///
/// Checks:
/// - Every slug in topology.json has a matching `nodes/{slug}.md`
/// - No orphaned `.md` files in `nodes/` not listed in topology
/// - Every `depends_on` reference points to an existing slug
/// - No cycles in the dependency graph
pub(crate) fn cross_reference(base_dir: &Path) -> Vec<CrossRefError> {
    let mut errors = Vec::new();

    // Read topology
    let topology = match file_reader::read_topology(base_dir) {
        Ok(t) => t,
        Err(e) => {
            errors.push(CrossRefError {
                file: "topology.json".into(),
                error: e,
            });
            return errors;
        }
    };

    let topology_slugs: HashSet<&str> = topology.keys().map(|s| s.as_str()).collect();

    // Check each topology slug has a matching node file
    let nodes_dir = base_dir.join("nodes");
    for slug in &topology_slugs {
        let node_path = nodes_dir.join(format!("{slug}.md"));
        if !node_path.exists() {
            errors.push(CrossRefError {
                file: format!("nodes/{slug}.md"),
                error: "listed in topology.json but file does not exist".into(),
            });
        }
    }

    // Check for orphaned node files
    if let Ok(entries) = std::fs::read_dir(&nodes_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if let Some(slug) = name_str.strip_suffix(".md") {
                if !topology_slugs.contains(slug) {
                    errors.push(CrossRefError {
                        file: format!("nodes/{slug}.md"),
                        error: "file exists but not listed in topology.json".into(),
                    });
                }
            }
        }
    }

    // Check depends_on references
    for (slug, deps) in &topology {
        for dep in deps {
            if !topology_slugs.contains(dep.as_str()) {
                errors.push(CrossRefError {
                    file: "topology.json".into(),
                    error: format!("node \"{slug}\" depends on \"{dep}\" which does not exist"),
                });
            }
        }
    }

    // Check for cycles
    if let Err(cycle_msg) = detect_cycles(&topology) {
        errors.push(CrossRefError {
            file: "topology.json".into(),
            error: cycle_msg,
        });
    }

    errors
}
