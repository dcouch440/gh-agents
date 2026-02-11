//! Prompt injection generation for the documenter protocol.
//!
//! The prompt is defined as a readable `const` template with
//! `{{.Protocol.field}}` placeholders, resolved at expansion time via
//! [`crate::server::hub::protocols::template_resolve::resolve_template`].

use std::collections::HashMap;

use crate::db::ProtocolDocumentDefRow;
use crate::server::hub::protocols::template_resolve::resolve_template;
use crate::server::hub::protocols::text_utils::collapse_blank_lines;

const DOCUMENTER_TEMPLATE: &str = "\
You are a Document Strategist. Your job is to plan how each \
requested document should be researched and written.

Requested Documents:
{{.Protocol.requested_documents}}

{{.Protocol.available_capabilities}}
{{.Protocol.context_documents_instruction}}
For each document, provide:
- document_name: must match one of the document names listed above exactly
- research_strategy: a step-by-step plan for gathering the information \
needed to write this document
- required_capabilities: which capabilities the researcher needs \
from the list above (empty array if no research tools are needed)
- writer_prompt: detailed instructions for the writer, including \
tone, structure, target audience, and focus areas
- context_document_ids: short IDs of context documents the researcher \
and writer need (omit or leave empty if none are needed)

Respond with a JSON object containing a \"document_plans\" array \
with one entry per document.";

/// Generate the documenter prompt injection: instructs the strategist to plan
/// research and writing for each requested document.
///
/// When `has_context_documents` is true, the prompt includes instructions for
/// assigning context documents to specific document plans via `context_document_ids`.
pub fn documenter_prompt(
    doc_defs: &[serde_json::Value],
    capabilities: &[String],
    has_context_documents: bool,
) -> String {
    let mut vars = HashMap::new();
    vars.insert(
        "Protocol.requested_documents".to_string(),
        format_documents_block(doc_defs),
    );
    vars.insert(
        "Protocol.available_capabilities".to_string(),
        format_capabilities_block(capabilities),
    );
    vars.insert(
        "Protocol.context_documents_instruction".to_string(),
        format_context_documents_instruction(has_context_documents),
    );
    collapse_blank_lines(&resolve_template(DOCUMENTER_TEMPLATE, &vars))
}

/// Format the numbered document listing for the documenter template.
fn format_documents_block(doc_defs: &[serde_json::Value]) -> String {
    let mut parts = Vec::new();
    for (i, def) in doc_defs.iter().enumerate() {
        let name = def["name"].as_str().unwrap_or("Unnamed");
        let description = def["description"].as_str().unwrap_or("");
        let target_length = def["target_length"].as_i64().unwrap_or(2000);

        parts.push(String::new());
        if description.is_empty() {
            parts.push(format!(
                "{}. \"{}\" (target: ~{} characters)",
                i + 1,
                name,
                target_length
            ));
        } else {
            parts.push(format!(
                "{}. \"{}\" \u{2014} {} (target: ~{} characters)",
                i + 1,
                name,
                description,
                target_length
            ));
        }
    }
    parts.join("\n")
}

/// Format the capabilities block for the documenter template.
/// Returns empty string when no capabilities are available.
pub fn format_capabilities_block(capabilities: &[String]) -> String {
    if capabilities.is_empty() {
        return String::new();
    }
    let mut parts = vec!["Available Research Capabilities:".to_string()];
    for cap in capabilities {
        parts.push(format!("- {}", cap));
    }
    parts.join("\n")
}

/// Format a document definitions section from database rows.
///
/// Produces a header ("## Document Definitions") followed by a numbered listing
/// of each document. Used by the `DocumenterPromptFilter` to augment system prompts
/// at execution time, sharing the same formatting logic as the compiler.
pub fn format_document_defs_section(defs: &[ProtocolDocumentDefRow]) -> String {
    let mut out = format!(
        "## Document Definitions\nThe user has requested {} document(s) to be generated:\n",
        defs.len()
    );
    for (i, def) in defs.iter().enumerate() {
        let description = if def.description.is_empty() {
            "(no description provided)"
        } else {
            &def.description
        };
        if def.description.is_empty() {
            out.push_str(&format!(
                "\n{}. \"{}\" (target: ~{} characters) \u{2014} {}",
                i + 1,
                def.name,
                def.target_length,
                description,
            ));
        } else {
            out.push_str(&format!(
                "\n{}. \"{}\" \u{2014} {} (target: ~{} characters)",
                i + 1,
                def.name,
                description,
                def.target_length,
            ));
        }
    }
    out.push('\n');
    out
}

/// Format the context documents instruction for the documenter template.
/// Returns empty string when no context documents are available.
pub fn format_context_documents_instruction(has_context_documents: bool) -> String {
    if !has_context_documents {
        return String::new();
    }
    "Context Documents:\n\
     The user prompt contains context documents wrapped in <document_XXXXXXXX> tags.\n\
     For each document plan, assign relevant context documents by listing their \
     8-character IDs in the \"context_document_ids\" array. Only assign documents \
     that are directly relevant to that specific document's research and writing. \
     Leave the array empty if no context documents are needed for that plan."
        .to_string()
}
