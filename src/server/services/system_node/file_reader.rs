//! File reader — converts the system node agent's repository into DesignedAgentPrompts.
//!
//! Pure function: reads topology.json + agents/*.json from a base directory,
//! maps them to the `DesignedAgentPrompt` structs that `execute_agent_levels` consumes.

use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

use crate::server::hub::dag::pipeline::DesignedAgentPrompt;

#[cfg(test)]
#[path = "file_reader_tests.rs"]
mod tests;

// ---------------------------------------------------------------------------
// Agent config deserialization (matches the schema in the vision doc)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct AgentConfig {
    name: String,
    system_prompt: String,
    assignment: String,
    expected_output: String,
    #[serde(default)]
    capabilities: Vec<String>,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Read the system node agent's repository and produce `DesignedAgentPrompt`s.
///
/// Reads `topology.json` for the dependency graph, then reads each
/// `agents/{slug}.json` for agent configs. The `execution_order` is set
/// to the iteration index — `compute_execution_levels` derives the real
/// parallel ordering from `receives_from` at execution time.
///
/// `agent_roster_entry_id` is set to `Uuid::nil()` — the sync step
/// (slice 3) assigns real IDs after creating DB rows.
pub(crate) fn read_agent_configs(base_dir: &Path) -> Result<Vec<DesignedAgentPrompt>, String> {
    // 1. Read and parse topology.json
    let topology_path = base_dir.join("topology.json");
    let topology_content = std::fs::read_to_string(&topology_path)
        .map_err(|e| format!("cannot read topology.json: {e}"))?;

    let topology: Value = serde_json::from_str(&topology_content)
        .map_err(|e| format!("invalid JSON in topology.json: {e}"))?;

    let agents_map = topology
        .get("agents")
        .and_then(|v| v.as_object())
        .ok_or("topology.json missing \"agents\" object")?;

    // 2. Extract slug → depends_on mapping
    let mut slug_deps: Vec<(String, Vec<String>)> = Vec::with_capacity(agents_map.len());
    for (slug, entry) in agents_map {
        let depends_on: Vec<String> = entry
            .get("depends_on")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        slug_deps.push((slug.clone(), depends_on));
    }

    // 3. Read each agent file and build slug → display_name map
    let agents_dir = base_dir.join("agents");
    let mut slug_to_name: HashMap<String, String> = HashMap::with_capacity(slug_deps.len());
    let mut configs: Vec<(Vec<String>, AgentConfig)> = Vec::with_capacity(slug_deps.len());

    for (slug, depends_on) in &slug_deps {
        let agent_path = agents_dir.join(format!("{slug}.json"));
        let agent_content = std::fs::read_to_string(&agent_path)
            .map_err(|e| format!("cannot read agents/{slug}.json: {e}"))?;

        let config: AgentConfig = serde_json::from_str(&agent_content)
            .map_err(|e| format!("invalid JSON in agents/{slug}.json: {e}"))?;

        slug_to_name.insert(slug.clone(), config.name.clone());
        configs.push((depends_on.clone(), config));
    }

    // 4. Build prompts with receives_from resolved from slugs to display names.
    // Downstream code (compute_execution_levels, filter_outputs_for_agent) matches
    // receives_from against agent_name via normalize_agent_name — this only works
    // when both sides are display names, not topology slugs.
    let mut prompts = Vec::with_capacity(configs.len());
    for (i, (depends_on, config)) in configs.into_iter().enumerate() {
        let resolved_receives: Vec<String> = depends_on
            .iter()
            .filter_map(|dep_slug| slug_to_name.get(dep_slug).cloned())
            .collect();

        prompts.push(DesignedAgentPrompt {
            agent_roster_entry_id: Uuid::nil(),
            agent_name: config.name,
            tools: config.capabilities,
            system_prompt: config.system_prompt,
            assignment: config.assignment,
            expected_output: Some(config.expected_output),
            execution_order: i as i32,
            receives_from: resolved_receives,
        });
    }

    Ok(prompts)
}

/// Read `config.json` and return (name, description).
///
/// Used by the sync step to update the step row and detect description changes.
pub(crate) fn read_config(base_dir: &Path) -> Result<(String, String), String> {
    let config_path = base_dir.join("config.json");
    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| format!("cannot read config.json: {e}"))?;

    let val: Value =
        serde_json::from_str(&content).map_err(|e| format!("invalid JSON in config.json: {e}"))?;

    let name = val
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let description = val
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    Ok((name, description))
}

/// Read `topology.json` and return the slug → depends_on map.
///
/// Used by the sync step to diff against DB edges.
pub(crate) fn read_topology(base_dir: &Path) -> Result<HashMap<String, Vec<String>>, String> {
    let topology_path = base_dir.join("topology.json");
    let content = std::fs::read_to_string(&topology_path)
        .map_err(|e| format!("cannot read topology.json: {e}"))?;

    let val: Value = serde_json::from_str(&content)
        .map_err(|e| format!("invalid JSON in topology.json: {e}"))?;

    let agents_map = val
        .get("agents")
        .and_then(|v| v.as_object())
        .ok_or("topology.json missing \"agents\" object")?;

    let mut result = HashMap::with_capacity(agents_map.len());
    for (slug, entry) in agents_map {
        let depends_on: Vec<String> = entry
            .get("depends_on")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        result.insert(slug.clone(), depends_on);
    }

    Ok(result)
}
