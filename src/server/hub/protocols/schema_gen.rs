//! Output schema auto-generation utilities for protocols.

use serde_json::json;

use super::types::PortConfig;

/// Generate a decomp output schema: array of `{port, content}` objects
/// where `port` is constrained to the configured port names.
///
/// When any port has a `content_schema`, generates `oneOf` per-port variants
/// with typed content fields. Otherwise falls back to a bare object content.
pub fn decomp_schema(ports: &[PortConfig]) -> serde_json::Value {
    let has_typed_content = ports.iter().any(|p| p.content_schema.is_some());

    if has_typed_content {
        let variants: Vec<serde_json::Value> = ports
            .iter()
            .map(|port| {
                let content_schema = port
                    .content_schema
                    .clone()
                    .unwrap_or_else(|| json!({"type": "object"}));
                json!({
                    "type": "object",
                    "required": ["port", "content"],
                    "properties": {
                        "port": { "const": port.port_name },
                        "content": content_schema
                    },
                    "additionalProperties": false
                })
            })
            .collect();

        json!({
            "type": "array",
            "items": { "oneOf": variants }
        })
    } else {
        let port_names: Vec<serde_json::Value> = ports
            .iter()
            .map(|p| serde_json::Value::String(p.port_name.clone()))
            .collect();

        json!({
            "type": "array",
            "items": {
                "type": "object",
                "required": ["port", "content"],
                "properties": {
                    "port": {
                        "type": "string",
                        "enum": port_names,
                        "description": "The target agent port for this task"
                    },
                    "content": {
                        "type": "object",
                        "description": "The task content to pass to the assigned agent"
                    }
                },
                "additionalProperties": false
            }
        })
    }
}

/// Generate a route output schema: single `{port, content}` object
/// where `port` is constrained to the configured port names.
///
/// When any port has a `content_schema`, generates `oneOf` per-port variants
/// with typed content fields. Otherwise falls back to a bare object content.
pub fn route_schema(ports: &[PortConfig]) -> serde_json::Value {
    let has_typed_content = ports.iter().any(|p| p.content_schema.is_some());

    if has_typed_content {
        let variants: Vec<serde_json::Value> = ports
            .iter()
            .map(|port| {
                let content_schema = port
                    .content_schema
                    .clone()
                    .unwrap_or_else(|| json!({"type": "object"}));
                json!({
                    "type": "object",
                    "required": ["port", "content"],
                    "properties": {
                        "port": { "const": port.port_name },
                        "content": content_schema
                    },
                    "additionalProperties": false
                })
            })
            .collect();

        json!({ "oneOf": variants })
    } else {
        let port_names: Vec<serde_json::Value> = ports
            .iter()
            .map(|p| serde_json::Value::String(p.port_name.clone()))
            .collect();

        json!({
            "type": "object",
            "required": ["port", "content"],
            "properties": {
                "port": {
                    "type": "string",
                    "enum": port_names,
                    "description": "The single target agent port to route to"
                },
                "content": {
                    "type": "object",
                    "description": "The content to pass to the selected agent"
                }
            },
            "additionalProperties": false
        })
    }
}

/// Generate a review output schema: `{decision, feedback}` object
/// with configurable decision options.
pub fn review_schema(decisions: &[String]) -> serde_json::Value {
    let decision_values: Vec<serde_json::Value> = decisions
        .iter()
        .map(|d| serde_json::Value::String(d.clone()))
        .collect();

    json!({
        "type": "object",
        "required": ["decision", "feedback"],
        "properties": {
            "decision": {
                "type": "string",
                "enum": decision_values,
                "description": "Your review decision"
            },
            "feedback": {
                "type": "string",
                "description": "Detailed feedback explaining your decision"
            }
        },
        "additionalProperties": false
    })
}

/// Generate a transform output schema from user-provided schema config.
/// Falls back to a flexible object schema if none provided.
pub fn transform_schema(user_schema: Option<&serde_json::Value>) -> serde_json::Value {
    match user_schema {
        Some(schema) => schema.clone(),
        None => json!({
            "type": "object",
            "description": "Structured output from the transform step"
        }),
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::hub::protocols::types::PortConfig;
    use uuid::Uuid;

    fn make_ports() -> Vec<PortConfig> {
        vec![
            PortConfig {
                port_name: "frontend".to_string(),
                description: "Frontend agent".to_string(),
                agent_id: Uuid::new_v4(),
                agent_name: "Frontend Dev".to_string(),
                agent_tools: vec![],
                display_order: 0,
                content_schema: None,
            },
            PortConfig {
                port_name: "backend".to_string(),
                description: "Backend agent".to_string(),
                agent_id: Uuid::new_v4(),
                agent_name: "Backend Dev".to_string(),
                agent_tools: vec![],
                display_order: 1,
                content_schema: None,
            },
        ]
    }

    #[test]
    fn decomp_schema_has_port_enum() {
        let ports = make_ports();
        let schema = decomp_schema(&ports);
        let items = &schema["items"];
        let port_enum = &items["properties"]["port"]["enum"];
        assert_eq!(port_enum[0], "frontend");
        assert_eq!(port_enum[1], "backend");
        assert_eq!(schema["type"], "array");
    }

    #[test]
    fn route_schema_is_single_object() {
        let ports = make_ports();
        let schema = route_schema(&ports);
        assert_eq!(schema["type"], "object");
        let port_enum = &schema["properties"]["port"]["enum"];
        assert_eq!(port_enum[0], "frontend");
        assert_eq!(port_enum[1], "backend");
    }

    #[test]
    fn review_schema_has_decision_enum() {
        let decisions = vec![
            "approve".to_string(),
            "reject".to_string(),
            "revise".to_string(),
        ];
        let schema = review_schema(&decisions);
        let decision_enum = &schema["properties"]["decision"]["enum"];
        assert_eq!(decision_enum[0], "approve");
        assert_eq!(decision_enum[1], "reject");
        assert_eq!(decision_enum[2], "revise");
    }

    #[test]
    fn transform_schema_uses_provided_schema() {
        let custom = json!({"type": "object", "properties": {"title": {"type": "string"}}});
        let schema = transform_schema(Some(&custom));
        assert_eq!(schema, custom);
    }

    #[test]
    fn transform_schema_falls_back_to_default() {
        let schema = transform_schema(None);
        assert_eq!(schema["type"], "object");
    }

    // =========================================================================
    // Content Schema Typing Tests
    // =========================================================================

    fn make_typed_ports() -> Vec<PortConfig> {
        vec![
            PortConfig {
                port_name: "frontend".to_string(),
                description: "Frontend agent".to_string(),
                agent_id: Uuid::new_v4(),
                agent_name: "Frontend Dev".to_string(),
                agent_tools: vec![],
                display_order: 0,
                content_schema: Some(json!({
                    "type": "object",
                    "properties": {
                        "component": {"type": "string"},
                        "styles": {"type": "boolean"}
                    }
                })),
            },
            PortConfig {
                port_name: "backend".to_string(),
                description: "Backend agent".to_string(),
                agent_id: Uuid::new_v4(),
                agent_name: "Backend Dev".to_string(),
                agent_tools: vec![],
                display_order: 1,
                content_schema: Some(json!({
                    "type": "object",
                    "properties": {
                        "endpoint": {"type": "string"},
                        "method": {"type": "string"}
                    }
                })),
            },
        ]
    }

    #[test]
    fn decomp_schema_with_content_schemas_uses_one_of() {
        let ports = make_typed_ports();
        let schema = decomp_schema(&ports);

        assert_eq!(schema["type"], "array");
        let one_of = schema["items"]["oneOf"].as_array().unwrap();
        assert_eq!(one_of.len(), 2);

        // First variant: frontend with typed content
        assert_eq!(one_of[0]["properties"]["port"]["const"], "frontend");
        assert!(one_of[0]["properties"]["content"]["properties"]["component"].is_object());

        // Second variant: backend with typed content
        assert_eq!(one_of[1]["properties"]["port"]["const"], "backend");
        assert!(one_of[1]["properties"]["content"]["properties"]["endpoint"].is_object());
    }

    #[test]
    fn decomp_schema_mixed_typed_untyped() {
        let mut ports = make_typed_ports();
        ports[1].content_schema = None; // Backend has no schema

        let schema = decomp_schema(&ports);

        let one_of = schema["items"]["oneOf"].as_array().unwrap();
        assert_eq!(one_of.len(), 2);

        // Frontend: typed
        assert!(one_of[0]["properties"]["content"]["properties"]["component"].is_object());

        // Backend: falls back to bare object
        assert_eq!(one_of[1]["properties"]["content"]["type"], "object");
        assert!(one_of[1]["properties"]["content"]["properties"].is_null());
    }

    #[test]
    fn decomp_schema_no_content_schemas_uses_enum() {
        // Original format when no schemas present
        let ports = make_ports();
        let schema = decomp_schema(&ports);

        // Should have the enum format, not oneOf
        assert!(schema["items"]["oneOf"].is_null());
        let port_enum = &schema["items"]["properties"]["port"]["enum"];
        assert_eq!(port_enum[0], "frontend");
    }

    #[test]
    fn route_schema_with_content_schemas_uses_one_of() {
        let ports = make_typed_ports();
        let schema = route_schema(&ports);

        let one_of = schema["oneOf"].as_array().unwrap();
        assert_eq!(one_of.len(), 2);

        assert_eq!(one_of[0]["properties"]["port"]["const"], "frontend");
        assert!(one_of[0]["properties"]["content"]["properties"]["component"].is_object());
    }

    #[test]
    fn route_schema_no_content_schemas_uses_enum() {
        let ports = make_ports();
        let schema = route_schema(&ports);

        assert!(schema["oneOf"].is_null());
        assert_eq!(schema["type"], "object");
        let port_enum = &schema["properties"]["port"]["enum"];
        assert_eq!(port_enum[0], "frontend");
    }

    // =========================================================================
    // Documenter Schema Tests
    // =========================================================================

    fn make_doc_defs() -> Vec<serde_json::Value> {
        vec![
            json!({"name": "API Reference", "description": "REST API docs", "target_length": 5000}),
            json!({"name": "Architecture Guide", "description": "System overview", "target_length": 3000}),
        ]
    }

    #[test]
    fn documenter_schema_has_document_plans_array() {
        let defs = make_doc_defs();
        let schema = documenter_schema(&defs);

        assert_eq!(schema["type"], "object");
        assert!(schema["required"]
            .as_array()
            .unwrap()
            .contains(&json!("document_plans")));
        assert_eq!(schema["properties"]["document_plans"]["type"], "array");
    }

    #[test]
    fn documenter_schema_has_document_name_enum() {
        let defs = make_doc_defs();
        let schema = documenter_schema(&defs);

        let name_enum =
            &schema["properties"]["document_plans"]["items"]["properties"]["document_name"]["enum"];
        assert_eq!(name_enum[0], "API Reference");
        assert_eq!(name_enum[1], "Architecture Guide");
    }

    #[test]
    fn documenter_schema_requires_all_plan_fields() {
        let defs = make_doc_defs();
        let schema = documenter_schema(&defs);

        let required = schema["properties"]["document_plans"]["items"]["required"]
            .as_array()
            .unwrap();
        assert!(required.contains(&json!("document_name")));
        assert!(required.contains(&json!("research_strategy")));
        assert!(required.contains(&json!("required_capabilities")));
        assert!(required.contains(&json!("writer_prompt")));
    }

    #[test]
    fn documenter_schema_empty_defs_omits_enum() {
        let schema = documenter_schema(&[]);

        let name_schema =
            &schema["properties"]["document_plans"]["items"]["properties"]["document_name"];
        assert_eq!(name_schema["type"], "string");
        assert!(name_schema["enum"].is_null());
    }

    #[test]
    fn documenter_schema_disallows_additional_properties() {
        let defs = make_doc_defs();
        let schema = documenter_schema(&defs);

        assert_eq!(schema["additionalProperties"], false);
        assert_eq!(
            schema["properties"]["document_plans"]["items"]["additionalProperties"],
            false
        );
    }

    #[test]
    fn documenter_schema_includes_optional_context_document_ids() {
        let defs = make_doc_defs();
        let schema = documenter_schema(&defs);

        let ctx_field =
            &schema["properties"]["document_plans"]["items"]["properties"]["context_document_ids"];
        assert_eq!(ctx_field["type"], "array");
        assert_eq!(ctx_field["items"]["type"], "string");

        // context_document_ids is NOT required
        let required = schema["properties"]["document_plans"]["items"]["required"]
            .as_array()
            .unwrap();
        assert!(!required.contains(&json!("context_document_ids")));
    }
}
