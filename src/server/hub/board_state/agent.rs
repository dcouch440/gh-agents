//! Agent element renderer.
//!
//! One function builds the `<agent>` element for L3 and L4.
//! L3 uses flat attributes with text content; L4 uses child elements.
//! L1/L2 don't render individual agents (they're a summary attribute on `<node>`).

use crate::markup::XmlBuilder;
use super::types::*;

/// Render an [`AgentSnapshot`] as an `<agent>` XML element.
///
/// The variant controls the structure:
/// - **L3**: attributes for capabilities/receives_from, text for description
/// - **L4**: child elements for role, capabilities, depends_on, plus id attribute
pub fn render_agent(agent: &AgentSnapshot, variant: BoardStateVariant, indent: usize) -> String {
    let mut el = XmlBuilder::new("agent", indent);

    el.attr("name", &agent.name);
    el.attr_if(variant.include_agent_ids(), "id", &agent.id.to_string());

    if variant.include_agent_ids() {
        // L4: structured child elements
        if !agent.role_description.is_empty() {
            el.raw(
                &XmlBuilder::new("role", indent + 1)
                    .text(&agent.role_description)
                    .build(),
            );
        }
        if !agent.capabilities.is_empty() {
            el.raw(
                &XmlBuilder::new("capabilities", indent + 1)
                    .text(&agent.capabilities.join(", "))
                    .build(),
            );
        }
        if !agent.receives_from.is_empty() {
            el.raw(
                &XmlBuilder::new("depends_on", indent + 1)
                    .text(&agent.receives_from.join(", "))
                    .build(),
            );
        }
    } else {
        // L3: flat attributes + text content
        if !agent.capabilities.is_empty() {
            el.attr("capabilities", &agent.capabilities.join(", "));
        }
        if !agent.receives_from.is_empty() {
            el.attr("receives_from", &agent.receives_from.join(", "));
        }
        el.text(&agent.role_description);
    }

    el.build()
}
