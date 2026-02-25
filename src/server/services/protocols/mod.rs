//! Protocol service: create, read, update, delete protocols, manage ports,
//! preview expansions, and apply protocols to workflow steps.

use once_cell::sync::Lazy;
use regex::Regex;
use uuid::Uuid;

use super::error::ServiceError;

mod apply;
mod crud;
mod resolve;
mod tests;

// Re-export all public items from submodules.
pub use apply::{apply_protocol, preview_expansion};
pub use crud::{
    create_port, create_protocol, delete_port, delete_protocol, get_protocol, list_protocol_ports,
    list_protocols, update_port, update_protocol,
};
pub(crate) use resolve::resolve_protocol_associations;

/// Valid port name pattern: lowercase alphanumeric + underscores, starting with a letter.
static PORT_NAME_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[a-z][a-z0-9_]*$").unwrap());

/// Maximum allowed port name length.
const MAX_PORT_NAME_LEN: usize = 50;

// ============================================================================
// Service input types
// ============================================================================

/// Input for creating a new protocol via the service layer.
pub struct CreateProtocolServiceInput {
    pub name: String,
    pub description: Option<String>,
    pub protocol_type: String,
    pub config: Option<serde_json::Value>,
    pub agent_id: Option<Uuid>,
    pub output_schema_id: Option<Uuid>,
    pub prompt_template_id: Option<Uuid>,
}

/// Result of applying a protocol to a workflow step.
pub struct ApplyResult {
    pub output_schema_id: Uuid,
    pub created_steps: Vec<CreatedStep>,
}

/// A step created during protocol application.
pub struct CreatedStep {
    pub port_name: String,
    pub step_id: Uuid,
    pub agent_id: Option<Uuid>,
}

// ============================================================================
// Shared helpers
// ============================================================================

/// Validate a port name against the allowed pattern and length.
pub(crate) fn validate_port_name(name: &str) -> Result<(), ServiceError> {
    if name.is_empty() || name.len() > MAX_PORT_NAME_LEN || !PORT_NAME_REGEX.is_match(name) {
        return Err(ServiceError::validation(format!(
            "Invalid port name \"{}\": must match [a-z][a-z0-9_]* and be at most {} characters",
            name, MAX_PORT_NAME_LEN
        )));
    }
    Ok(())
}
