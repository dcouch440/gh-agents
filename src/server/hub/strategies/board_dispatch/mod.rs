//! BoardDispatchStrategy — ExecutionStrategy for the board dispatcher.
//!
//! Runs asynchronously after Phase 0 to dispatch configuration instructions
//! to per-node builders. Has only two tools: `dispatch_to_builders` and `think`.
//! No topology tools — Phase 0 already built the structure.

use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

use crate::config::protocols::{roles, AgentConfig, BOARD_DISPATCHER};
use crate::llm::{Message, Tool};
use crate::server::hub::board_state::{self, BoardStateVariant};
use crate::server::hub::error::HubError;
use crate::server::hub::protocols::template_resolve::resolve_template;
use crate::server::hub::strategy::ExecutionStrategy;
use crate::server::state::AppState;
use crate::types::UserId;

mod tests;

/// Tool names available to the board dispatcher.
const BOARD_DISPATCH_TOOLS: &[&str] = &["dispatch_to_builders", "think"];

/// Strategy for the board dispatcher background task.
///
/// Reads board_state (current node list), receives a changeset instruction,
/// and dispatches tailored configuration instructions to per-node builders.
///
/// Persistent: The `session_id` links to a board dispatcher session that
/// accumulates history across submits.
pub struct BoardDispatchStrategy {
    system_prompt: String,
    instruction: String,
    state: AppState,
    workflow_id: Uuid,
    user_id: UserId,
    session_id: Option<Uuid>,
}

impl BoardDispatchStrategy {
    fn config(&self) -> &AgentConfig {
        BOARD_DISPATCHER.agent("dispatcher")
    }

    /// Build a new board dispatch strategy.
    ///
    /// `session_id` links to the persistent board dispatcher session. When
    /// present, `build_messages()` loads prior dispatch history for continuity.
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
            // Board dispatcher sees all nodes — pass nil as "own" step
            Uuid::nil(),
        )
        .await
        .map_err(|e| e.to_string())?;

        let mut vars = HashMap::new();
        vars.insert("System.board_state".to_string(), board_state_xml);

        let system_prompt = resolve_template(roles::BOARD_DISPATCHER_SYSTEM, &vars);

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
impl ExecutionStrategy for BoardDispatchStrategy {
    fn system_prompt(&self) -> &str {
        &self.system_prompt
    }

    fn tools(&self) -> Vec<Tool> {
        BOARD_DISPATCH_TOOLS
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

        Ok(Some(resolve_template(
            roles::BOARD_DISPATCHER_SYSTEM,
            &vars,
        )))
    }

    async fn build_messages(&self, _input: &str) -> Result<Vec<Message>, HubError> {
        if let Some(session_id) = self.session_id {
            let history = self
                .state
                .repos()
                .sessions
                .get_session_history(session_id, 20)
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
            _ => serde_json::json!({
                "error": format!("Unknown tool: {name}")
            }),
        }
    }
}
