//! ProtocolCompiler trait — parameterizes the Protocol Engine.
//!
//! Each protocol type implements this trait to define how it validates
//! configuration, generates schemas, creates prompt injections, and
//! compiles into workflow primitives.
//!
//! See `compilers/documenter/` for the reference implementation.

use super::error::ProtocolError;
use super::types::{ProtocolConfig, ProtocolExpansion};

/// A protocol compiler that knows how to turn a protocol configuration
/// into concrete workflow primitives (steps, edges, ports, schemas).
///
/// Mirrors the `ExecutionStrategy` pattern — each protocol type implements this.
/// All methods are synchronous and pure (no DB, no side effects).
pub trait ProtocolCompiler: Send + Sync {
    /// Protocol type identifier (e.g., "documenter").
    fn protocol_type(&self) -> &str;

    /// Human-readable description of what this protocol type does.
    fn description(&self) -> &str;

    /// Validate that the configuration is valid for this protocol type.
    fn validate(&self, config: &ProtocolConfig) -> Result<(), ProtocolError>;

    /// Generate the output schema based on configured ports/agents.
    fn generate_schema(&self, config: &ProtocolConfig) -> Result<serde_json::Value, ProtocolError>;

    /// Generate the prompt injection text for the orchestrator agent.
    fn generate_prompt_injection(&self, config: &ProtocolConfig) -> Result<String, ProtocolError>;

    /// Compile the protocol into concrete workflow primitives.
    fn compile(&self, config: &ProtocolConfig) -> Result<ProtocolExpansion, ProtocolError>;
}
