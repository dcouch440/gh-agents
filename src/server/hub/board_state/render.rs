//! Top-level board state renderer and XML builder.
//!
//! [`XmlBuilder`] is the shared primitive — all render functions use it
//! to construct elements with conditional attributes and children.
//! [`render`] is the entry point that wraps everything in `<board_state>`.

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

// ============================================================================
// XmlBuilder
// ============================================================================

/// Lightweight XML element builder.
///
/// Handles attribute escaping, conditional inclusion, and indented output.
/// All render functions use this as their shared primitive.
///
/// ```ignore
/// let xml = XmlBuilder::new("node", 1)
///     .attr("name", "Collector")
///     .attr_if(show_id, "id", &id.to_string())
///     .text("3 agents, task set")
///     .raw(&child_xml)
///     .build();
/// ```
pub(super) struct XmlBuilder {
    tag: String,
    attrs: Vec<(String, String)>,
    content: Vec<ContentItem>,
    indent: usize,
}

enum ContentItem {
    /// Text content — escaped for XML text (no quote escaping).
    Text(String),
    /// Pre-rendered XML — inserted as-is (already formatted and indented).
    Raw(String),
}

impl XmlBuilder {
    /// Create a new builder for the given tag at the specified indent level.
    pub fn new(tag: &str, indent: usize) -> Self {
        Self {
            tag: tag.to_string(),
            attrs: Vec::new(),
            content: Vec::new(),
            indent,
        }
    }

    /// Add an attribute (value is XML-escaped including quotes).
    pub fn attr(&mut self, key: &str, value: &str) -> &mut Self {
        self.attrs.push((key.to_string(), xml_escape(value)));
        self
    }

    /// Add an attribute only if the condition is true.
    pub fn attr_if(&mut self, cond: bool, key: &str, value: &str) -> &mut Self {
        if cond {
            self.attr(key, value);
        }
        self
    }

    /// Add an attribute only if the value is `Some` and non-empty.
    pub fn attr_opt(&mut self, key: &str, value: Option<&str>) -> &mut Self {
        if let Some(v) = value {
            if !v.is_empty() {
                self.attr(key, v);
            }
        }
        self
    }

    /// Add text content (XML text-escaped, no quote escaping).
    pub fn text(&mut self, content: &str) -> &mut Self {
        if !content.is_empty() {
            self.content.push(ContentItem::Text(content.to_string()));
        }
        self
    }

    /// Add pre-rendered XML content (inserted as-is).
    pub fn raw(&mut self, xml: &str) -> &mut Self {
        if !xml.is_empty() {
            self.content.push(ContentItem::Raw(xml.to_string()));
        }
        self
    }

    /// Build the final XML string.
    ///
    /// - No content → self-closing: `<tag attr="v" />\n`
    /// - Single text item → inline: `<tag attr="v">text</tag>\n`
    /// - Multiple items → multiline with indented children
    pub fn build(&self) -> String {
        let pad = "  ".repeat(self.indent);
        let mut out = String::new();

        // Opening tag
        out.push_str(&pad);
        out.push('<');
        out.push_str(&self.tag);
        for (k, v) in &self.attrs {
            out.push_str(&format!(" {}=\"{}\"", k, v));
        }

        if self.content.is_empty() {
            // Self-closing
            out.push_str(" />\n");
            return out;
        }

        // Single text item → inline
        if self.content.len() == 1 {
            if let ContentItem::Text(ref t) = self.content[0] {
                out.push('>');
                out.push_str(&xml_escape_text(t));
                out.push_str("</");
                out.push_str(&self.tag);
                out.push_str(">\n");
                return out;
            }
        }

        // Multiple items → multiline
        out.push_str(">\n");
        let child_pad = "  ".repeat(self.indent + 1);

        for item in &self.content {
            match item {
                ContentItem::Text(t) => {
                    out.push_str(&child_pad);
                    out.push_str(&xml_escape_text(t));
                    out.push('\n');
                }
                ContentItem::Raw(xml) => {
                    out.push_str(xml);
                }
            }
        }

        out.push_str(&pad);
        out.push_str("</");
        out.push_str(&self.tag);
        out.push_str(">\n");
        out
    }
}

// ============================================================================
// Escape helpers
// ============================================================================

/// Escape XML special characters in attribute values (includes `"`).
pub(super) fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Escape XML special characters in text content (no `"` escaping needed).
pub(super) fn xml_escape_text(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
