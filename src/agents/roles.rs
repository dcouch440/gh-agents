//! Agent role system (minimal types for 3.3, will be expanded in 3.4)

use serde::{Deserialize, Serialize};

// Re-export CommunicationStyle from types for convenience
pub use crate::types::CommunicationStyle;

/// Unique identifier for a role
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RoleId(pub String);

impl RoleId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

/// Output format for role responses
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputFormat {
    /// Structured plan with tickets/slices
    Plan,
    /// Code with report
    CodeAndReport,
    /// Simple result/answer
    Result,
    /// Summary document
    Summary,
    /// Custom format (described in string)
    Custom(String),
}

impl Default for OutputFormat {
    fn default() -> Self {
        OutputFormat::Result
    }
}
