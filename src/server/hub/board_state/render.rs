//! Top-level board state renderer.
//!
//! [`render`] is the entry point that wraps everything in `<board_state>`.

use crate::markup::XmlBuilder;

use super::node;
use super::types::*;

// ============================================================================
// Public API
// ============================================================================

/// Render a [`BoardSnapshot`] as `<board_state>` XML for the given variant.
pub fn render(snapshot: &BoardSnapshot, variant: BoardStateVariant) -> String {
    let mut bs = XmlBuilder::new("board_state", 0);

    match variant.scope() {
        Scope::AllNodes => {
            let mut wf = XmlBuilder::new("workflow", 1);
            wf.attr("name", &snapshot.workflow_name);
            wf.attr_if(variant.include_node_ids(), "id", &snapshot.workflow_id.to_string());

            if matches!(variant, BoardStateVariant::ManagerAssistant) {
                wf.attr("status", derive_workflow_status(&snapshot.nodes));
            }

            for n in &snapshot.nodes {
                wf.raw(&node::render_node(n, variant));
            }
            bs.raw(&wf.build());

            if matches!(variant, BoardStateVariant::ManagerBuilder)
                && !snapshot.available_capabilities.is_empty()
            {
                bs.raw(
                    &XmlBuilder::new("available_capabilities", 1)
                        .text(&snapshot.available_capabilities.join(", "))
                        .build(),
                );
            }
        }
        Scope::OwnNode => {
            if let Some(n) = snapshot.nodes.first() {
                bs.raw(&node::render_node(n, variant));
            }
        }
    }

    bs.build()
}

/// Derive a workflow-level status from its nodes.
fn derive_workflow_status(nodes: &[NodeSnapshot]) -> &'static str {
    if nodes.is_empty() {
        return "empty";
    }
    let all_completed = nodes.iter().all(|n| n.status == "completed");
    let any_configured = nodes
        .iter()
        .any(|n| n.status == "configured" || n.status == "completed");

    if all_completed {
        "completed"
    } else if any_configured {
        "configuring"
    } else {
        "idle"
    }
}
