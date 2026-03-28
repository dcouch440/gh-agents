//! Port element renderer.
//!
//! Renders incoming context ports (L3 style) as XML elements.

use super::types::*;
use crate::markup::XmlBuilder;

/// Render an [`IncomingContextSnapshot`] as a `<port>` element (L3 style).
pub fn render_incoming_port(port: &IncomingContextSnapshot, indent: usize) -> String {
    let mut el = XmlBuilder::new("port", indent);
    el.attr("name", &port.name);
    el.attr("from", &port.source_mode);
    el.attr("status", &port.status);

    if let Some(ref preview) = port.preview {
        el.text(preview);
    }

    el.build()
}
