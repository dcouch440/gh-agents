//! Port element renderer.
//!
//! Three port types, all builder-driven:
//! - [`render_incoming_port`]: L3 context port with status and preview
//! - [`render_input_port`]: L4 typed input with schema and json_path
//! - [`render_output_port`]: L4 typed output with schema

use super::render::XmlBuilder;
use super::types::*;

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

/// Render an [`InputPortSnapshot`] as a `<port>` element (L4 style).
pub fn render_input_port(port: &InputPortSnapshot, indent: usize) -> String {
    let mut el = XmlBuilder::new("port", indent);
    el.attr("name", &port.port_name);
    el.attr("from", &port.from_node);

    if let Some(ref schema) = port.schema {
        el.raw(
            &XmlBuilder::new("schema", indent + 1)
                .text(schema)
                .build(),
        );
    }
    if let Some(ref jp) = port.json_path {
        el.raw(
            &XmlBuilder::new("json_path", indent + 1)
                .text(jp)
                .build(),
        );
    }

    el.build()
}

/// Render an [`OutputPortSnapshot`] as a `<port>` element (L4 style).
pub fn render_output_port(port: &OutputPortSnapshot, indent: usize) -> String {
    let mut el = XmlBuilder::new("port", indent);
    el.attr("name", &port.port_name);
    el.attr("to", &port.to_node);

    if let Some(ref schema) = port.schema {
        el.raw(
            &XmlBuilder::new("schema", indent + 1)
                .text(schema)
                .build(),
        );
    }

    el.build()
}
