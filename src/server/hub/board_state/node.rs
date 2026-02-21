//! Node element renderer.
//!
//! One function builds the `<node>` element for all 4 variants.
//! The variant's flags control which attributes and children are included.

use super::agent;
use super::port;
use crate::markup::XmlBuilder;
use super::types::*;

/// Render a [`NodeSnapshot`] as a `<node>` XML element for the given variant.
pub fn render_node(node: &NodeSnapshot, variant: BoardStateVariant) -> String {
    let indent = match variant.scope() {
        Scope::AllNodes => 2, // inside <workflow>
        Scope::OwnNode => 1, // inside <board_state>
    };

    let mut el = XmlBuilder::new("node", indent);

    // ── Attributes ──────────────────────────────────────────────────────

    el.attr("name", &node.name);
    el.attr_if(variant.include_node_ids(), "id", &node.id.to_string());
    el.attr("protocol", &node.protocol);
    el.attr("status", &node.status);
    el.attr_if(
        variant.include_task_attr() && !node.task.is_empty(),
        "task",
        &node.task,
    );

    // L3: capabilities as attribute on <node>
    if matches!(variant, BoardStateVariant::NodeAssistant) && !node.capabilities.is_empty() {
        el.attr("capabilities", &node.capabilities.join(", "));
    }

    el.attr_opt("receives", node.receives.as_deref());

    // L1/L2: agent names as attribute (not rendered as children)
    if !variant.include_agent_children() && !node.agents.is_empty() {
        let names: Vec<&str> = node.agents.iter().map(|a| a.name.as_str()).collect();
        el.attr("agents", &names.join(", "));
    }

    // ── Text content ────────────────────────────────────────────────────

    el.text(&node.summary);

    // ── Asking (L1 only) ────────────────────────────────────────────────

    if variant.include_asking() {
        if let Some(ref question) = node.asking {
            el.raw(&XmlBuilder::new("asking", indent + 1).text(question).build());
        }
    }

    // ── L4: input/output ports with schemas ─────────────────────────────

    if variant.include_port_schemas() {
        if !node.input_ports.is_empty() {
            let mut section = XmlBuilder::new("input_ports", indent + 1);
            for p in &node.input_ports {
                section.raw(&port::render_input_port(p, indent + 2));
            }
            el.raw(&section.build());
        }
        if !node.output_ports.is_empty() {
            let mut section = XmlBuilder::new("output_ports", indent + 1);
            for p in &node.output_ports {
                section.raw(&port::render_output_port(p, indent + 2));
            }
            el.raw(&section.build());
        }
    }

    // ── L4: capabilities as child element ───────────────────────────────

    if matches!(variant, BoardStateVariant::Dispatch) && !node.capabilities.is_empty() {
        el.raw(
            &XmlBuilder::new("capabilities", indent + 1)
                .text(&node.capabilities.join(", "))
                .build(),
        );
    }

    // ── Agents ──────────────────────────────────────────────────────────

    if variant.include_agent_children() && !node.agents.is_empty() {
        if variant.include_agent_ids() {
            // L4: wrap in <agent_roster>
            let mut roster = XmlBuilder::new("agent_roster", indent + 1);
            for a in &node.agents {
                roster.raw(&agent::render_agent(a, variant, indent + 2));
            }
            el.raw(&roster.build());
        } else {
            // L3: flat agent children
            for a in &node.agents {
                el.raw(&agent::render_agent(a, variant, indent + 1));
            }
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

    // ── L4: notes ───────────────────────────────────────────────────────

    if variant.include_notes() && !node.notes.is_empty() {
        el.raw(
            &XmlBuilder::new("notes", indent + 1)
                .text(&node.notes)
                .build(),
        );
    }

    el.build()
}
