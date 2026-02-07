//! Output schema auto-generation utilities for protocols.

use serde_json::json;

use super::types::PortConfig;

/// Generate a decomp output schema: array of `{port, content}` objects
/// where `port` is constrained to the configured port names.
pub fn decomp_schema(ports: &[PortConfig]) -> serde_json::Value {
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

/// Generate a route output schema: single `{port, content}` object
/// where `port` is constrained to the configured port names.
pub fn route_schema(ports: &[PortConfig]) -> serde_json::Value {
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
            },
            PortConfig {
                port_name: "backend".to_string(),
                description: "Backend agent".to_string(),
                agent_id: Uuid::new_v4(),
                agent_name: "Backend Dev".to_string(),
                agent_tools: vec![],
                display_order: 1,
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
}
