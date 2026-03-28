//! Node element renderer.
//!
//! One function builds the `<node>` element for all 3 variants.
//! The variant's flags control which attributes and children are included.

use super::agent;
use super::port;
use super::types::*;
use crate::markup::XmlBuilder;

/// Render a [`NodeSnapshot`] as a `<node>` XML element for the given variant.
pub fn render_node(node: &NodeSnapshot, variant: BoardStateVariant) -> String {
    let indent = match variant.scope() {
        Scope::AllNodes => 2, // inside <workflow>
        Scope::OwnNode => 1,  // inside <board_state>
    };

    let mut el = XmlBuilder::new("node", indent);

    // ── Attributes ──────────────────────────────────────────────────────

    el.attr_opt("ref", node.ref_id.as_deref());
    el.attr_if(variant.include_node_ids(), "id", &node.id.to_string());
    el.attr("status", &node.status);
    el.attr_if(
        variant.include_task_attr() && !node.task.is_empty(),
        "task",
        &node.task,
    );

    // L2/L3: capabilities as attribute on <node>
    if matches!(
        variant,
        BoardStateVariant::ManagerBuilder | BoardStateVariant::NodeAssistant
    ) && !node.capabilities.is_empty()
    {
        el.attr("capabilities", &node.capabilities.join(", "));
    }

    el.attr_opt("receives", node.receives.as_deref());
    el.attr_if(
        variant.include_initial_instructions() && node.initial_instructions_sent,
        "initial_instructions",
        "sent",
    );

    // L1: agent names as attribute (not rendered as children)
    if !variant.include_agent_children() && !node.agents.is_empty() {
        let names: Vec<&str> = node.agents.iter().map(|a| a.name.as_str()).collect();
        el.attr("agents", &names.join(", "));
    }

    // ── Text content ────────────────────────────────────────────────────

    el.text(&node.summary);

    // ── Compressed status (L1/L2) ───────────────────────────────────────

    if variant.include_compressed_status() {
        if let Some(ref status) = node.compressed_status {
            el.raw(&XmlBuilder::new("status", indent + 1).text(status).build());
        }
    }

    // ── Asking (L1 only) ────────────────────────────────────────────────

    if variant.include_asking() {
        if let Some(ref question) = node.asking {
            el.raw(&XmlBuilder::new("asking", indent + 1).text(question).build());
        }
    }

    // ── Agents ──────────────────────────────────────────────────────────

    if variant.include_agent_children() && !node.agents.is_empty() {
        for a in &node.agents {
            el.raw(&agent::render_agent(a, variant, indent + 1));
        }
    }

    // ── L3: incoming context ports ──────────────────────────────────────

    if matches!(variant, BoardStateVariant::NodeAssistant) && !node.incoming_context.is_empty() {
        let mut incoming = XmlBuilder::new("incoming", indent + 1);
        for p in &node.incoming_context {
            incoming.raw(&port::render_incoming_port(p, indent + 2));
        }
        el.raw(&incoming.build());
    }

    el.build()
}
