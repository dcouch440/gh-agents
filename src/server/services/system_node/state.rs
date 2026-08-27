//! Current state builder — reads the system node agent's repository and
//! produces `<current_state>` XML prepended to the agent's instruction.

use std::path::Path;

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;

/// Build the `<current_state>` XML from the repository filesystem.
///
/// Reads topology.json, agents/*.json, and config.json to produce a summary
/// of what exists, what's valid, and what's missing. Prepended to the
/// instruction once per generate — it is a snapshot, not refreshed between
/// rounds, so the agent should re-check via `run_command` (e.g. `cat`/`ls`)
/// if it needs current state mid-task.
pub(crate) fn build_current_state(base_dir: &Path) -> String {
    let topology_path = base_dir.join("topology.json");
    let config_path = base_dir.join("config.json");
    let agents_dir = base_dir.join("agents");

    // Empty state — nothing exists yet
    if !topology_path.exists() && !config_path.exists() {
        return "<current_state refresh=\"snapshot taken when this generate started — re-check via run_command if you need current state mid-task\">\n  \
                <topology status=\"empty\" />\n  \
                <config status=\"missing\" />\n\
                </current_state>"
            .to_string();
    }

    let mut lines = Vec::new();
    lines.push(
        "<current_state refresh=\"snapshot taken when this generate started — re-check via run_command if you need current state mid-task\">".into(),
    );

    // Parse topology and render agent statuses
    if let Ok(content) = std::fs::read_to_string(&topology_path) {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(agents) = val.get("agents").and_then(|v| v.as_object()) {
                lines.push("  <topology>".into());

                for (slug, entry) in agents {
                    let deps: Vec<&str> = entry
                        .get("depends_on")
                        .and_then(|v| v.as_array())
                        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
                        .unwrap_or_default();
                    let depends_on = deps.join(", ");

                    let agent_path = agents_dir.join(format!("{slug}.json"));
                    let status = if agent_path.exists() {
                        "configured"
                    } else {
                        "missing"
                    };

                    lines.push(format!(
                        "    <agent slug=\"{slug}\" depends_on=\"{depends_on}\" status=\"{status}\" />"
                    ));
                }

                lines.push("  </topology>".into());
            } else {
                lines.push("  <topology status=\"invalid\" />".into());
            }
        } else {
            lines.push("  <topology status=\"invalid\" />".into());
        }
    } else {
        lines.push("  <topology status=\"empty\" />".into());
    }

    // Config status
    if let Ok(content) = std::fs::read_to_string(&config_path) {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
            let name = val.get("name").and_then(|v| v.as_str()).unwrap_or("");
            if name.is_empty() {
                lines.push("  <config status=\"configured\" />".into());
            } else {
                lines.push(format!(
                    "  <config name=\"{name}\" status=\"configured\" />"
                ));
            }
        } else {
            lines.push("  <config status=\"invalid\" />".into());
        }
    } else {
        lines.push("  <config status=\"missing\" />".into());
    }

    lines.push("</current_state>".into());
    lines.join("\n")
}
