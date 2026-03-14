//! Board state enrichment — annotates snapshots with data from external sources.
//!
//! The core board state module fetches from the workflow DB only. This module
//! adds optional enrichments (e.g. design status from the system store) that
//! require additional data sources.

use std::collections::HashMap;

use uuid::Uuid;

use crate::db::traits::SystemFileRepo;
use crate::server::hub::dag::agent_designer::{agent_name_to_slug, normalize_agent_name};
use crate::server::services::system_store::store as system_store;

use super::types::{AgentDesignStatus, BoardSnapshot};

/// Enrich agent snapshots with design status from the system store.
///
/// Queries `design/{step_id}/agents/*.json` and annotates each agent:
/// - If the agent's name is in `changed_agents` → `Pending` (needs redesign)
/// - Else if the agent has an existing config in the store → `Designed { version, path }`
/// - Else → `Pending` (no config ever written)
///
/// When `changed_agents` is empty (e.g. pipeline path with no builder), all
/// agents without existing configs are marked pending.
pub async fn enrich_design_status(
    snapshot: &mut BoardSnapshot,
    system_files: &dyn SystemFileRepo,
    step_id: Uuid,
    workflow_id: Uuid,
    changed_agents: &[String],
) {
    let prefix = format!("design/{}/agents/", step_id);

    let files = system_store::list_files(system_files, workflow_id, &prefix)
        .await
        .unwrap_or_default();

    // Build slug → (version, path) map from store files
    let file_map: HashMap<String, (i32, String)> = files
        .into_iter()
        .filter_map(|f| {
            let filename = f.path.rsplit('/').next()?;
            let raw_slug = filename.strip_suffix(".json")?;
            let slug = normalize_agent_name(raw_slug);
            Some((slug, (f.version, f.path)))
        })
        .collect();

    // Normalize changed_agents for matching
    let changed_set: Vec<String> = changed_agents
        .iter()
        .map(|n| normalize_agent_name(n))
        .collect();

    for node in &mut snapshot.nodes {
        for agent in &mut node.agents {
            let slug = agent_name_to_slug(&agent.name);

            if changed_set.contains(&slug) {
                // Builder changed this agent — needs (re)design regardless of store state
                agent.design_status = AgentDesignStatus::Pending;
            } else if let Some((version, path)) = file_map.get(&slug) {
                // Existing config in store — mark as designed
                agent.design_status = AgentDesignStatus::Designed {
                    version: *version,
                    config_path: path.clone(),
                };
            } else {
                // No config ever written — needs design
                agent.design_status = AgentDesignStatus::Pending;
            }
        }
    }
}
