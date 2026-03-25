//! System node agent service — shared domain logic for configuring
//! runtime agent systems from a file-based repository.
//!
//! Used by:
//! - `SystemNodeStrategy` (execution strategy)
//! - Sync step (files → DB projection)
//! - Dispatch executor (container + base_dir resolution)
//! - DAG runtime (file executor)
//! - Future workflow-level system agents

pub mod file_reader;
pub mod state;
pub mod sync;
pub mod validate;

// ── Name normalization ─────────────────────────────────────────────────────
// Shared across sync, pipeline output, and any code that matches agent names.

/// Normalize an agent name for case-insensitive matching across case styles.
/// Strips spaces, underscores, hyphens, and lowercases.
/// "SecurityAuditor", "security_auditor", "Security Auditor" all → "securityauditor"
pub fn normalize_agent_name(name: &str) -> String {
    name.chars()
        .filter(|c| *c != ' ' && *c != '_' && *c != '-')
        .flat_map(|c| c.to_lowercase())
        .collect()
}

/// Convert an agent name to a filesystem-safe slug.
///
/// Reuses `normalize_agent_name()` — "Web Researcher" → "webresearcher".
pub fn agent_name_to_slug(name: &str) -> String {
    normalize_agent_name(name)
}

// ── Base directory resolution ──────────────────────────────────────────────
// Shared by dispatch executor and DAG runtime.

use std::path::PathBuf;
use uuid::Uuid;

use crate::server::state::AppState;

/// Resolve the base_dir for a system node agent's file repository.
///
/// Uses a dedicated path (`workflows/{wf_id}/system_node/{step_id}/`) that
/// persists across dispatches and survives run garbage collection.
///
/// With JuiceFS: `{mount}/workflows/{wf_id}/system_node/{step_id}/`
/// Without JuiceFS: `{tmp}/nexor_system_node/{step_id}/`
pub fn resolve_base_dir(state: &AppState, workflow_id: Uuid, step_id: Uuid) -> PathBuf {
    if let Some(workspace) = state.workspace() {
        let path = workspace.system_node_path(workflow_id, step_id);
        let _ = std::fs::create_dir_all(&path);
        return path;
    }

    std::env::temp_dir()
        .join("nexor_system_node")
        .join(step_id.to_string())
}
