//! Protocol Layer — the workflow primitive compiler.
//!
//! Protocols are reusable, user-configurable execution recipes that expand
//! into standard workflow primitives (steps, edges, ports, schemas). The
//! Protocol Engine manages registered expanders and orchestrates expansion.
//!
//! ```text
//! UI → Protocol Layer (expand) → Workflow Primitives → DAG Executor (unchanged)
//! ```

pub mod error;
pub mod expander;
pub mod expanders;
pub mod prompt_gen;
pub mod schema_gen;
pub mod types;

mod tests;

use std::collections::HashMap;
use std::sync::Arc;

use crate::db::ProtocolPortRow;

use error::ProtocolError;
use expander::ProtocolExpander;
use types::{PortConfig, ProtocolConfig, ProtocolExpansion};

/// The Protocol Engine — holds registered expanders and orchestrates expansion.
///
/// Analogous to `ExecutionEngine`, but for compile-time workflow construction
/// rather than runtime execution.
pub struct ProtocolEngine {
    expanders: HashMap<String, Arc<dyn ProtocolExpander>>,
}

impl ProtocolEngine {
    /// Create a new engine with all built-in expanders registered.
    pub fn new() -> Self {
        let mut engine = Self {
            expanders: HashMap::new(),
        };
        engine.register_builtins();
        engine
    }

    /// Register a custom expander.
    pub fn register(&mut self, expander: Arc<dyn ProtocolExpander>) {
        self.expanders
            .insert(expander.protocol_type().to_string(), expander);
    }

    /// List all registered protocol types with their descriptions.
    pub fn list_types(&self) -> Vec<(&str, &str)> {
        self.expanders
            .values()
            .map(|e| (e.protocol_type(), e.description()))
            .collect()
    }

    /// Build a `ProtocolConfig` from DB rows and agent names.
    /// This bridges the DB layer to the pure expansion layer.
    pub fn build_config(
        &self,
        protocol_type: &str,
        config: serde_json::Value,
        ports: &[ProtocolPortRow],
        agent_names: &HashMap<uuid::Uuid, String>,
        agent_tools: &HashMap<uuid::Uuid, Vec<String>>,
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
            })
            .collect();

        ProtocolConfig {
            protocol_type: protocol_type.to_string(),
            config,
            ports: port_configs,
        }
    }

    /// Expand a protocol configuration into workflow primitives.
    /// This is a pure function — no DB, no side effects.
    pub fn expand(&self, config: &ProtocolConfig) -> Result<ProtocolExpansion, ProtocolError> {
        let expander = self
            .expanders
            .get(&config.protocol_type)
            .ok_or_else(|| ProtocolError::UnknownType(config.protocol_type.clone()))?;

        expander.validate(config)?;
        expander.expand(config)
    }

    /// Preview what an expansion would produce (same as expand, but named
    /// for clarity in the API layer).
    pub fn preview(&self, config: &ProtocolConfig) -> Result<ProtocolExpansion, ProtocolError> {
        self.expand(config)
    }

    /// Register all built-in protocol expanders.
    fn register_builtins(&mut self) {
        use expanders::{DecompExpander, ReviewExpander, RouteExpander, TransformExpander};

        self.register(Arc::new(DecompExpander));
        self.register(Arc::new(TransformExpander));
        self.register(Arc::new(ReviewExpander));
        self.register(Arc::new(RouteExpander));
    }
}

impl Default for ProtocolEngine {
    fn default() -> Self {
        Self::new()
    }
}
