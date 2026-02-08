//! Built-in protocol definitions seeded at startup.
//!
//! These are framework-level protocol types that exist system-wide.
//! Uses deterministic UUIDs so re-seeding is idempotent.

use uuid::Uuid;

/// A lightweight struct for seeding — no timestamps needed.
pub struct BuiltinProtocol {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub protocol_type: String,
    pub config: serde_json::Value,
}

/// Namespace UUID for deterministic protocol ID generation.
const PROTOCOLS_NS: Uuid = Uuid::from_bytes([
    0x70, 0x72, 0x6f, 0x74, 0x6f, 0x63, 0x6f, 0x6c, 0x73, 0x2d, 0x6e, 0x65, 0x78, 0x6f, 0x72, 0x21,
]);

/// Returns the 5 built-in protocol definitions.
pub fn builtin_protocol_definitions() -> Vec<BuiltinProtocol> {
    vec![
        BuiltinProtocol {
            id: Uuid::new_v5(&PROTOCOLS_NS, b"Decomposition"),
            name: "Decomposition".to_string(),
            description: "Decompose a task into subtasks and fan out to specialist agents"
                .to_string(),
            protocol_type: "decomp".to_string(),
            config: serde_json::json!({}),
        },
        BuiltinProtocol {
            id: Uuid::new_v5(&PROTOCOLS_NS, b"Route"),
            name: "Route".to_string(),
            description: "Analyze input and route to the best-fit specialist agent".to_string(),
            protocol_type: "route".to_string(),
            config: serde_json::json!({}),
        },
        BuiltinProtocol {
            id: Uuid::new_v5(&PROTOCOLS_NS, b"Review"),
            name: "Review".to_string(),
            description: "Evaluate input and provide a structured decision with feedback"
                .to_string(),
            protocol_type: "review".to_string(),
            config: serde_json::json!({"decisions": ["approve", "reject", "revise"]}),
        },
        BuiltinProtocol {
            id: Uuid::new_v5(&PROTOCOLS_NS, b"Transform"),
            name: "Transform".to_string(),
            description: "Process input and produce structured output matching a schema"
                .to_string(),
            protocol_type: "transform".to_string(),
            config: serde_json::json!({}),
        },
        BuiltinProtocol {
            id: Uuid::new_v5(&PROTOCOLS_NS, b"Default"),
            name: "Default".to_string(),
            description: "Standard structured output with a response field".to_string(),
            protocol_type: "default".to_string(),
            config: serde_json::json!({
                "output_schema": {
                    "type": "object",
                    "required": ["response"],
                    "properties": {
                        "response": { "type": "string" }
                    },
                    "additionalProperties": false
                }
            }),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn returns_five_definitions() {
        let defs = builtin_protocol_definitions();
        assert_eq!(defs.len(), 5);
    }

    #[test]
    fn all_names_unique() {
        let defs = builtin_protocol_definitions();
        let names: HashSet<&str> = defs.iter().map(|d| d.name.as_str()).collect();
        assert_eq!(names.len(), 5);
    }

    #[test]
    fn all_ids_deterministic_and_unique() {
        let defs1 = builtin_protocol_definitions();
        let defs2 = builtin_protocol_definitions();
        // Deterministic
        for (a, b) in defs1.iter().zip(defs2.iter()) {
            assert_eq!(a.id, b.id);
        }
        // Unique
        let ids: HashSet<Uuid> = defs1.iter().map(|d| d.id).collect();
        assert_eq!(ids.len(), 5);
    }

    #[test]
    fn default_protocol_has_output_schema() {
        let defs = builtin_protocol_definitions();
        let default = defs.iter().find(|d| d.protocol_type == "default").unwrap();
        let schema = default.config.get("output_schema").unwrap();
        assert_eq!(schema["type"], "object");
        assert!(schema["required"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("response")));
    }
}
