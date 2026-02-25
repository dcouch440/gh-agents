//! ManagerDispatchStrategy — ExecutionStrategy for the manager builder (L2).
//!
//! Runs asynchronously to create/modify workflow topology and dispatch
//! instructions directly to node builders. Uses topology tools
//! (create_pipeline, etc.) and dispatch_to_builders for configuration.

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
    "dispatch_to_builders",
    "think",
];

/// Strategy for the manager builder (L2) background dispatch.
///
/// Reads board_state, uses topology tools to create/modify workflow structure,
/// dispatches instructions to node assistants, and reports what it did.
///
/// Persistent: The `session_id` links to a builder session that accumulates
/// history across dispatches, so the L2 agent can see prior work.
pub struct ManagerDispatchStrategy {
    system_prompt: String,
    instruction: String,
    state: AppState,
    workflow_id: Uuid,
    user_id: UserId,
    session_id: Option<Uuid>,
}

impl ManagerDispatchStrategy {
    fn config(&self) -> &AgentConfig {
        MANAGER_BUILDER.agent("dispatcher")
    }

    /// Build a new manager dispatch strategy.
    ///
    /// `session_id` links to the persistent L2 builder session. When present,
    /// `build_messages()` loads prior dispatch history for continuity.
    pub async fn new(
        state: AppState,
        workflow_id: Uuid,
        user_id: UserId,
        instruction: String,
        session_id: Option<Uuid>,
    ) -> Result<Self, String> {
        let board_state_xml = board_state::build(
            state.repos().workflows.as_ref(),
            Some(state.repos().sessions.as_ref()),
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
            session_id,
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
            Some(self.state.repos().sessions.as_ref()),
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
        if let Some(session_id) = self.session_id {
            // Load history from the persistent builder session.
            // The executor already inserted the current instruction as the last
            // user message, so history includes everything we need.
            let history = self
                .state
                .repos()
                .sessions
                .get_session_history(session_id, 20) // 10 pairs max
                .await
                .unwrap_or_default();

            if !history.is_empty() {
                let mut messages = Vec::with_capacity(history.len());
                for row in &history {
                    match row.role.as_str() {
                        "user" => messages.push(Message::user(&row.content)),
                        "assistant" => messages.push(Message::assistant(&row.content)),
                        _ => {}
                    }
                }
                return Ok(messages);
            }
        }

        // Fallback: no session or empty history
        Ok(vec![Message::user(&self.instruction)])
    }

    async fn execute_tool(&self, name: &str, input: &Value) -> Value {
        match name {
            "think" => serde_json::json!({ "status": "ok" }),
            "dispatch_to_builders" => {
                crate::server::services::dispatch::execute_dispatch_to_builders_tool(
                    &self.state,
                    input,
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
