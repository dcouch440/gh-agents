//! RoomSpeakerStrategy — ExecutionStrategy for a single speaker turn in an agent room.
//!
//! Each speaker in a room gets their own strategy instance. The strategy assembles
//! the system prompt with room context, transcript, and gatekeeper follow-up,
//! then records tokens and updates the agent_execution on completion.

use async_trait::async_trait;
use serde_json::Value;
use tracing::info;
use uuid::Uuid;

use crate::agents::execution_tools;
use crate::db::AgentRow;
use crate::execution::ExecutionContext;
use crate::llm::{Message, TokenUsage, Tool};
use crate::server::state::AppState;

use super::super::error::HubError;
use super::super::strategy::ExecutionStrategy;

/// Configuration for a room speaker execution.
pub struct RoomSpeakerConfig {
    /// The agent executing this turn.
    pub agent: AgentRow,
    /// Composed system prompt (agent base + room context + mode overlay).
    pub system_prompt: String,
    /// The user message (original user message + gatekeeper followup_context).
    pub user_prompt: String,
    /// Tool definitions (empty if room.tools_enabled is false).
    pub tools: Vec<Tool>,
    /// Tool name allow-list for execution_tools.
    pub tool_names: Vec<String>,
    /// Temperature for sampling (from mode resolution or agent default).
    pub temperature: f32,
    /// Execution context for file/git/test tool calls.
    pub execution_context: Option<ExecutionContext>,
    /// User ID for token ledger.
    pub user_id: Uuid,
    /// Agent execution ID (created before calling the engine).
    pub agent_execution_id: Uuid,
}

/// Strategy for a single room speaker's LLM call.
///
/// Uses the agent's model/temperature, optional tools, and records
/// token usage + execution status on completion.
pub struct RoomSpeakerStrategy {
    config: RoomSpeakerConfig,
    state: AppState,
}

impl RoomSpeakerStrategy {
    pub fn new(config: RoomSpeakerConfig, state: AppState) -> Self {
        Self { config, state }
    }
}

#[async_trait]
impl ExecutionStrategy for RoomSpeakerStrategy {
    fn system_prompt(&self) -> &str {
        &self.config.system_prompt
    }

    fn tools(&self) -> Vec<Tool> {
        self.config.tools.clone()
    }

    fn model_id(&self) -> &str {
        &self.config.agent.model_id
    }

    fn max_rounds(&self) -> u32 {
        // Room speakers get fewer tool rounds — keep discussions moving
        5
    }

    fn context_budget(&self) -> usize {
        480_000
    }

    fn streaming(&self) -> bool {
        true
    }

    fn temperature(&self) -> f32 {
        self.config.temperature  // Use mode-resolved temperature
    }

    async fn build_messages(&self, _input: &str) -> Result<Vec<Message>, HubError> {
        Ok(vec![Message::user(&self.config.user_prompt)])
    }

    async fn execute_tool(&self, name: &str, input: &Value) -> Value {
        match &self.config.execution_context {
            Some(exec_ctx) => {
                info!(
                    agent = %self.config.agent.name,
                    tool = %name,
                    "Room speaker tool call"
                );
                execution_tools::execute_execution_tool(
                    name,
                    input,
                    exec_ctx,
                    Some(&self.config.tool_names),
                )
                .await
            }
            None => {
                serde_json::json!({ "error": "No execution context available for tool calls" })
            }
        }
    }

    async fn on_complete(&self, response: &str, usage: &TokenUsage) -> Result<(), HubError> {
        // Record token usage
        if let Some(tl_repo) = &self.state.token_ledger_repo {
            let cost = super::compute_cost(
                &self.config.agent.model_id,
                usage.input_tokens as i64,
                usage.output_tokens as i64,
            );
            let _ = tl_repo
                .insert_ledger_entry(
                    self.config.user_id,
                    Some(self.config.agent_execution_id),
                    &self.config.agent.model_id,
                    usage.input_tokens as i64,
                    usage.output_tokens as i64,
                    cost,
                )
                .await;
        }

        // Update agent_execution with final status
        if let Some(ae_repo) = &self.state.agent_execution_repo {
            let _ = ae_repo
                .update_agent_execution_status(
                    self.config.agent_execution_id,
                    "completed",
                    Some(response.to_string()),
                    None,
                )
                .await;
        }

        Ok(())
    }
}
