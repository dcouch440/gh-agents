//! Output schema for the documenter protocol.
//!
//! Returns the static `response.json` from the strategist role definition.

use crate::config::protocols::roles;

/// Return the documenter strategy output schema.
///
/// The schema is defined in `config/protocols/documenter/strategist/response.json`
/// and embedded at compile time.
pub fn documenter_schema() -> serde_json::Value {
    let raw = roles::DOCUMENTER_STRATEGIST
        .response
        .expect("strategist must have response schema");
    serde_json::from_str(raw).expect("strategist response.json must be valid JSON")
}
