//! ManagerDispatchStrategy — ExecutionStrategy for the manager builder (L2).
//!
//! Runs asynchronously to create/modify workflow topology and dispatch
//! instructions to node assistants. Uses topology tools (create_pipeline, etc.)
//! and dispatch_to_nodes for messaging.

use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

use crate::config::protocols::{roles, AgentConfig, MANAGER_BUILDER};
use crate::llm::{Message, Tool};
use crate::server::hub::board_state::{self, BoardStateVariant};
use crate::server::hub::error::HubError;
use crate::server::hub::protocols::template_resolve::resolve_template;
use crate::server::hub::strategy::ExecutionStrategy;
use crate::server::services::messaging;
use crate::server::state::AppState;
use crate::server::tools::manager::{self, ManagerToolContext};
use crate::types::UserId;

mod tests;

/// Tool names available to the manager builder.
const MANAGER_BUILDER_TOOLS: &[&str] = &[
    "create_pipeline",
    "create_parallel",
    "insert_node",
    "remove_node",
    "wire_edge",
    "remove_edge",
    "dispatch_to_nodes",
    "think",
];

/// Strategy for the manager builder (L2) background dispatch.
///
/// Reads board_state, uses topology tools to create/modify workflow structure,
/// dispatches instructions to node assistants, and reports what it did.
pub struct ManagerDispatchStrategy {
    system_prompt: String,
    instruction: String,
    state: AppState,
    workflow_id: Uuid,
    user_id: UserId,
}

impl ManagerDispatchStrategy {
    fn config(&self) -> &AgentConfig {
        MANAGER_BUILDER.agent("dispatcher")
    }

    /// Build a new manager dispatch strategy.
    pub async fn new(
        state: AppState,
        workflow_id: Uuid,
        user_id: UserId,
        instruction: String,
    ) -> Result<Self, String> {
        let board_state_xml = board_state::build(
            state.repos().workflows.as_ref(),
            BoardStateVariant::ManagerBuilder,
            workflow_id,
            // L2 sees all nodes — pass nil as "own" step since scope is AllNodes
            Uuid::nil(),
        )
        .await
        .map_err(|e| e.to_string())?;

        let mut vars = HashMap::new();
        vars.insert("System.board_state".to_string(), board_state_xml);

        let system_prompt = resolve_template(roles::MANAGER_BUILDER_SYSTEM, &vars);

        Ok(Self {
            system_prompt,
            instruction,
            state,
            workflow_id,
            user_id,
        })
    }
}

#[async_trait]
impl ExecutionStrategy for ManagerDispatchStrategy {
    fn system_prompt(&self) -> &str {
        &self.system_prompt
    }

    fn tools(&self) -> Vec<Tool> {
        MANAGER_BUILDER_TOOLS
            .iter()
            .filter_map(|name| crate::tools::registry::get_tool_definition(name))
            .collect()
    }

    fn model_id(&self) -> &str {
        &self.config().model_id
    }

    fn max_rounds(&self) -> u32 {
        self.config().max_rounds
    }

    fn context_budget(&self) -> usize {
        self.config().context_budget
    }

    fn streaming(&self) -> bool {
        false
    }

    fn temperature(&self) -> f32 {
        self.config().temperature
    }

    fn state(&self) -> Option<&AppState> {
        Some(&self.state)
    }

    async fn rebuild_system_prompt(&self) -> Result<Option<String>, HubError> {
        let board_state_xml = board_state::build(
            self.state.repos().workflows.as_ref(),
            BoardStateVariant::ManagerBuilder,
            self.workflow_id,
            Uuid::nil(),
        )
        .await
        .map_err(|e| HubError::Internal(anyhow::anyhow!("{}", e)))?;

        let mut vars = HashMap::new();
        vars.insert("System.board_state".to_string(), board_state_xml);

        Ok(Some(resolve_template(roles::MANAGER_BUILDER_SYSTEM, &vars)))
    }

    async fn build_messages(&self, _input: &str) -> Result<Vec<Message>, HubError> {
        Ok(vec![Message::user(&self.instruction)])
    }

    async fn execute_tool(&self, name: &str, input: &Value) -> Value {
        match name {
            "think" => serde_json::json!({ "status": "ok" }),
            "dispatch_to_nodes" => {
                messaging::execute_dispatch_to_nodes_tool(
                    &self.state,
                    input,
                    "Manager",
                    self.user_id,
                    self.workflow_id,
                )
                .await
            }
            _ => {
                // Route to topology tool handlers
                let ctx = ManagerToolContext {
                    workflow_id: self.workflow_id,
                    user_id: self.user_id,
                };
                manager::execute_manager_tool(name, input, &self.state, &ctx).await
            }
        }
    }
}
