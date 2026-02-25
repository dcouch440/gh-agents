//! Graph context builder for the node assistant system prompt.
//!
//! Produces a text snapshot of the workflow graph (all nodes, edges,
//! and the selected node's current state) for injection into the
//! `{{.System.graph_context}}` template variable.

use uuid::Uuid;

use crate::db::traits::WorkflowRepo;
use crate::server::hub::error::HubError;

mod tests;

/// Build a text representation of the workflow graph for the assistant.
///
/// Shows all nodes with their execution modes, marks the selected node,
/// and lists edge connections. The output is compact enough for system
/// prompt injection (~200-400 tokens for a typical 10-node workflow).
pub async fn build_graph_context(
    repo: &dyn WorkflowRepo,
    workflow_id: Uuid,
    step_id: Uuid,
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
    out.push_str("Workflow nodes:\n");

    for step in &steps {
        let name = step.name.as_deref().unwrap_or("(unnamed)");
        let selected = if step.id == step_id {
            " [SELECTED]"
        } else {
            ""
        };
        let desc = if step.description.is_empty() {
            String::new()
        } else {
            let truncated: String = step.description.chars().take(100).collect();
            if truncated.len() < step.description.len() {
                format!(" — {}...", truncated)
            } else {
                format!(" — {}", truncated)
            }
        };
        out.push_str(&format!(
            "  - {} ({}){}{}\n",
            name, step.execution_mode, selected, desc
        ));
    }

    if !edges.is_empty() {
        out.push_str("\nConnections:\n");
        for edge in &edges {
            let from_name = steps
                .iter()
                .find(|s| s.id == edge.from_step_id)
                .and_then(|s| s.name.as_deref())
                .unwrap_or("(unnamed)");
            let to_name = steps
                .iter()
                .find(|s| s.id == edge.to_step_id)
                .and_then(|s| s.name.as_deref())
                .unwrap_or("(unnamed)");
            out.push_str(&format!("  {} -> {}\n", from_name, to_name));
        }
    }

    Ok(out)
}
