//! Agent element renderer.
//!
//! One function builds the `<agent>` element for L2 and L3.
//! Uses flat attributes with text content for description.
//! L1 doesn't render individual agents (they're a summary attribute on `<node>`).

use super::types::{AgentSnapshot, BoardStateVariant};
use crate::markup::XmlBuilder;

/// Render an [`AgentSnapshot`] as an `<agent>` XML element.
///
/// Uses flat attributes for capabilities/receives_from, text for description.
pub fn render_agent(agent: &AgentSnapshot, variant: BoardStateVariant, indent: usize) -> String {
    let mut el = XmlBuilder::new("agent", indent);

    el.attr("name", &agent.name);
    el.attr_if(variant.include_agent_ids(), "id", &agent.id.to_string());

    if !agent.capabilities.is_empty() {
        el.attr("capabilities", &agent.capabilities.join(", "));
    }
    if !agent.receives_from.is_empty() {
        el.attr("receives_from", &agent.receives_from.join(", "));
    }
    el.text(&agent.role_description);

    el.build()
}
