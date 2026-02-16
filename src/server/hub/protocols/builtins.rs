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

/// Returns the built-in protocol definitions.
///
/// Each protocol type has a corresponding compiler in `hub::protocols::compilers`
/// that generates schemas, prompts, and workflow primitives dynamically based on
/// configuration at compilation time.
pub fn builtin_protocol_definitions() -> Vec<BuiltinProtocol> {
    vec![]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_empty_definitions() {
        let defs = builtin_protocol_definitions();
        assert!(defs.is_empty());
    }
}
