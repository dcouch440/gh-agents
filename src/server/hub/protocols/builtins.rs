//! Built-in protocol definitions seeded at startup.
//!
//! These are framework-level protocol types that exist system-wide.
//! Uses deterministic UUIDs so re-seeding is idempotent.
//!
//! Protocol compilers handle all protocol logic dynamically at compilation
//! time. Schemas and prompts are generated based on port configuration.
//! The protocol row stores only the type metadata and default config.

use uuid::Uuid;

/// A built-in protocol definition for seeding.
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

/// Returns the built-in protocol definitions.
///
/// Each protocol type has a corresponding compiler in `hub::protocols::compilers`
/// that generates schemas, prompts, and workflow primitives dynamically based on
/// configuration at compilation time.
pub fn builtin_protocol_definitions() -> Vec<BuiltinProtocol> {
    vec![
        // =====================================================================
        // Documenter — structured document generation pipeline
        // =====================================================================
        BuiltinProtocol {
            id: Uuid::new_v5(&PROTOCOLS_NS, b"Documenter"),
            name: "Documenter".into(),
            description: "Generate structured documents from upstream context. \
                Define document templates with names, descriptions, and target lengths. \
                At runtime, an LLM strategist plans research for each document, \
                capability-resolved tools gather information, then specialist writers \
                produce the final content."
                .into(),
            protocol_type: "documenter".into(),
            config: serde_json::json!({}),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn returns_one_definition() {
        let defs = builtin_protocol_definitions();
        assert_eq!(defs.len(), 1);
    }

    #[test]
    fn all_names_unique() {
        let defs = builtin_protocol_definitions();
        let names: HashSet<&str> = defs.iter().map(|d| d.name.as_str()).collect();
        assert_eq!(names.len(), defs.len());
    }

    #[test]
    fn all_ids_deterministic_and_unique() {
        let defs1 = builtin_protocol_definitions();
        let defs2 = builtin_protocol_definitions();
        for (a, b) in defs1.iter().zip(defs2.iter()) {
            assert_eq!(a.id, b.id);
        }
        let ids: HashSet<Uuid> = defs1.iter().map(|d| d.id).collect();
        assert_eq!(ids.len(), defs1.len());
    }

    #[test]
    fn documenter_type_matches_compiler() {
        let defs = builtin_protocol_definitions();
        assert_eq!(defs[0].protocol_type, "documenter");
    }

    #[test]
    fn all_descriptions_non_empty() {
        for def in builtin_protocol_definitions() {
            assert!(
                !def.description.is_empty(),
                "{} has empty description",
                def.name
            );
        }
    }
}
