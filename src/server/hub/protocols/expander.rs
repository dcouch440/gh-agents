//! ProtocolExpander trait — parameterizes the Protocol Engine.
//!
//! Each protocol type (decomp, transform, review, route) implements this trait
//! to define how it validates configuration, generates schemas, creates prompt
//! injections, and expands into workflow primitives.

use super::error::ProtocolError;
use super::types::{ProtocolConfig, ProtocolExpansion};

/// A protocol expander that knows how to turn a protocol configuration
/// into concrete workflow primitives (steps, edges, ports, schemas).
///
/// Mirrors the `ExecutionStrategy` pattern — each protocol type implements this.
/// All methods are synchronous and pure (no DB, no side effects).
pub trait ProtocolExpander: Send + Sync {
    /// Protocol type identifier (e.g., "decomp", "transform", "review", "route").
    fn protocol_type(&self) -> &str;

    /// Human-readable description of what this protocol type does.
    fn description(&self) -> &str;

    /// Validate that the configuration is valid for this protocol type.
    fn validate(&self, config: &ProtocolConfig) -> Result<(), ProtocolError>;

    /// Generate the output schema based on configured ports/agents.
    fn generate_schema(&self, config: &ProtocolConfig) -> Result<serde_json::Value, ProtocolError>;

    /// Generate the prompt injection text for the orchestrator agent.
    fn generate_prompt_injection(&self, config: &ProtocolConfig) -> Result<String, ProtocolError>;

    /// Expand the protocol into concrete workflow primitives.
    fn expand(&self, config: &ProtocolConfig) -> Result<ProtocolExpansion, ProtocolError>;
}
