//! Protocol Layer — the workflow primitive compiler.
//!
//! Protocols are reusable, user-configurable execution recipes that compile
//! into standard workflow primitives (steps, edges, ports, schemas). The
//! Protocol Engine manages registered compilers and orchestrates compilation.
//!
//! ```text
//! UI → Protocol Layer (compile) → Workflow Primitives → DAG Executor (unchanged)
//! ```

pub mod compiler;
pub mod compilers;
pub mod context;
pub mod error;
pub mod execution_recorder;
pub mod json_utils;
pub mod template_resolve;
pub mod text_utils;
pub mod types;

#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::sync::Arc;

use crate::db::ProtocolPortRow;

use compiler::ProtocolCompiler;
use error::ProtocolError;
use types::{PortConfig, ProtocolConfig, ProtocolExpansion};

/// The Protocol Engine — holds registered compilers and orchestrates compilation.
///
/// Analogous to `ExecutionEngine`, but for compile-time workflow construction
/// rather than runtime execution.
pub struct ProtocolEngine {
    compilers: HashMap<String, Arc<dyn ProtocolCompiler>>,
}

impl ProtocolEngine {
    /// Create a new engine.
    pub fn new() -> Self {
        Self {
            compilers: HashMap::new(),
        }
    }

    /// Register a custom compiler.
    pub fn register(&mut self, compiler: Arc<dyn ProtocolCompiler>) {
        self.compilers
            .insert(compiler.protocol_type().to_string(), compiler);
    }

    /// List all registered protocol types with their descriptions.
    pub fn list_types(&self) -> Vec<(&str, &str)> {
        self.compilers
            .values()
            .map(|c| (c.protocol_type(), c.description()))
            .collect()
    }

    /// Build a `ProtocolConfig` from DB rows and agent names.
    /// This bridges the DB layer to the pure compilation layer.
    pub fn build_config(
        &self,
        protocol_type: &str,
        config: serde_json::Value,
        ports: &[ProtocolPortRow],
        agent_names: &HashMap<uuid::Uuid, String>,
        agent_tools: &HashMap<uuid::Uuid, Vec<String>>,
        agent_schemas: &HashMap<uuid::Uuid, serde_json::Value>,
    ) -> ProtocolConfig {
        let port_configs: Vec<PortConfig> = ports
            .iter()
            .map(|p| PortConfig {
                port_name: p.port_name.clone(),
                description: p.description.clone(),
                agent_id: p.agent_id,
                agent_name: agent_names
                    .get(&p.agent_id)
                    .cloned()
                    .unwrap_or_else(|| "Unknown Agent".to_string()),
                agent_tools: agent_tools.get(&p.agent_id).cloned().unwrap_or_default(),
                display_order: p.display_order,
                content_schema: agent_schemas.get(&p.agent_id).cloned(),
            })
            .collect();

        ProtocolConfig {
            protocol_type: protocol_type.to_string(),
            config,
            ports: port_configs,
        }
    }

    /// Compile a protocol configuration into workflow primitives.
    /// This is a pure function — no DB, no side effects.
    pub fn expand(&self, config: &ProtocolConfig) -> Result<ProtocolExpansion, ProtocolError> {
        let compiler = self
            .compilers
            .get(&config.protocol_type)
            .ok_or_else(|| ProtocolError::UnknownType(config.protocol_type.clone()))?;

        compiler.validate(config)?;
        compiler.compile(config)
    }

    /// Preview what a compilation would produce (same as expand, but named
    /// for clarity in the API layer).
    pub fn preview(&self, config: &ProtocolConfig) -> Result<ProtocolExpansion, ProtocolError> {
        self.expand(config)
    }
}

impl Default for ProtocolEngine {
    fn default() -> Self {
        Self::new()
    }
}
