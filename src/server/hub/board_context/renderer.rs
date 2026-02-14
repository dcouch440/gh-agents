//! Board renderer — assembles a structured document of the entire workflow
//! board for Haiku consumption.
//!
//! Two queries: steps + edges. Produces a compact text document that includes
//! node names, archetypes, descriptions, goal summaries, and connections.

use uuid::Uuid;

use crate::db::traits::WorkflowRepo;
use crate::server::hub::error::HubError;

/// Maximum description length before truncation (chars).
const MAX_DESCRIPTION_CHARS: usize = 200;

/// Render the full board as a structured document for Haiku distillation.
///
/// Output is optimized for LLM consumption — compact, structured, and
/// focused on intent/purpose rather than implementation details.
pub async fn render_board(
    repo: &dyn WorkflowRepo,
    workflow_id: Uuid,
) -> Result<String, HubError> {
    let steps = repo
        .list_steps(workflow_id)
        .await
        .map_err(HubError::Internal)?;
    let edges = repo
        .list_edges(workflow_id)
        .await
        .map_err(HubError::Internal)?;

    let mut out = String::new();

    out.push_str(&format!(
        "Board: {} nodes, {} connections\n\n",
        steps.len(),
        edges.len()
    ));

    // Build a name lookup for edge resolution
    let step_name = |id: Uuid| -> String {
        steps
            .iter()
            .find(|s| s.id == id)
            .and_then(|s| s.name.as_deref())
            .unwrap_or("(unnamed)")
            .to_string()
    };

    for step in &steps {
        let name = step.name.as_deref().unwrap_or("(unnamed)");

        out.push_str(&format!("[Node: {}] ({})\n", name, step.execution_mode));

        // Description (truncated)
        if !step.description.is_empty() {
            let desc: String = step.description.chars().take(MAX_DESCRIPTION_CHARS).collect();
            if desc.len() < step.description.len() {
                out.push_str(&format!("  Description: {}...\n", desc));
            } else {
                out.push_str(&format!("  Description: {}\n", desc));
            }
        }

        // Goal summary (from conversation distillation)
        if !step.goal_summary.is_empty() {
            out.push_str(&format!("  Goal: {}\n", step.goal_summary));
        } else {
            out.push_str("  Goal: (not yet established)\n");
        }

        // Connections for this node
        let incoming: Vec<String> = edges
            .iter()
            .filter(|e| e.to_step_id == step.id)
            .map(|e| step_name(e.from_step_id))
            .collect();
        let outgoing: Vec<String> = edges
            .iter()
            .filter(|e| e.from_step_id == step.id)
            .map(|e| step_name(e.to_step_id))
            .collect();

        if !incoming.is_empty() || !outgoing.is_empty() {
            let mut conn_parts = Vec::new();
            for name in &incoming {
                conn_parts.push(format!("\u{2190} {}", name));
            }
            for name in &outgoing {
                conn_parts.push(format!("\u{2192} {}", name));
            }
            out.push_str(&format!("  Connections: {}\n", conn_parts.join(", ")));
        }

        out.push('\n');
    }

    Ok(out)
}
