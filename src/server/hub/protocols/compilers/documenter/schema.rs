//! Output schema generation for the documenter protocol.

use serde_json::json;

/// Generate a documenter output schema: `{ document_plans: [{ document_name, ... }] }`
/// where `document_name` is constrained to the configured document definition names.
///
/// The strategy LLM must return one plan per document, specifying research strategy,
/// required capabilities, and writer instructions.
pub fn documenter_schema(doc_defs: &[serde_json::Value]) -> serde_json::Value {
    let names: Vec<serde_json::Value> = doc_defs
        .iter()
        .filter_map(|d| d["name"].as_str().map(|s| json!(s)))
        .collect();

    let document_name_schema = if names.is_empty() {
        json!({
            "type": "string",
            "description": "The name of the document to produce"
        })
    } else {
        json!({
            "type": "string",
            "enum": names,
            "description": "The name of the document to produce"
        })
    };

    json!({
        "type": "object",
        "required": ["document_plans"],
        "properties": {
            "document_plans": {
                "type": "array",
                "items": {
                    "type": "object",
                    "required": ["document_name", "research_strategy", "required_capabilities", "writer_prompt"],
                    "properties": {
                        "document_name": document_name_schema,
                        "research_strategy": {
                            "type": "string",
                            "description": "Step-by-step plan for researching this document"
                        },
                        "required_capabilities": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Capability keys the researcher needs (e.g. web_search, code_analysis)"
                        },
                        "writer_prompt": {
                            "type": "string",
                            "description": "Detailed instructions for the writer including tone, structure, and focus areas"
                        },
                        "context_document_ids": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Short IDs of context documents from the <context> block relevant to this document. Use the 8-character ID from each <document_XXXXXXXX> tag. Omit or leave empty if no context documents are needed."
                        }
                    },
                    "additionalProperties": false
                }
            }
        },
        "additionalProperties": false
    })
}
