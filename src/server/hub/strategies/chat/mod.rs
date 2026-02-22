//! ChatStrategy — replaces the orchestrator's `handle_message` execution loop.
//!
//! Handles interactive chat sessions: loads history, streams tokens, saves
//! messages, auto-names sessions, and triggers compaction.
//!
//! Heavy logic is delegated to focused sub-modules:
//! - `config` — configuration types
//! - `tools` — step tool resolution and dispatch
//! - `broadcast` — workflow event broadcasting
//! - `messages` — session history and message building
//! - `completion` — post-processing (save, auto-name, compaction, beliefs)

use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

use crate::llm::{Message, TokenUsage, Tool};
use crate::server::state::AppState;
use crate::types::UserId;

use super::super::error::HubError;
use super::super::strategy::ExecutionStrategy;

pub(crate) mod broadcast;
mod completion;
pub(crate) mod config;
pub(crate) mod dispatch;
mod messages;
pub(crate) mod tools;

pub use config::{ChatConfig, StepChatContext};
// Re-export for test compatibility
#[cfg(test)]
pub(crate) use tools::{resolve_chat_step_tools, resolve_step_tools};

/// Strategy for interactive chat sessions.
///
/// Loads session history, executes server tools (agent management, docs, etc.),
/// and handles post-processing (save message, auto-name, compaction).
pub struct ChatStrategy {
    config: ChatConfig,
    state: AppState,
    user_id: UserId,
    session_id: Option<Uuid>,
    message_id: Uuid,
    step_context: Option<StepChatContext>,
}

impl ChatStrategy {
    pub fn new(
        config: ChatConfig,
        state: AppState,
        user_id: UserId,
        session_id: Option<Uuid>,
        message_id: Uuid,
    ) -> Self {
        Self {
            config,
            state,
            user_id,
            session_id,
            message_id,
            step_context: None,
        }
    }

    /// Create a ChatStrategy with step context for step-scoped chat sessions.
    pub fn with_step_context(
        config: ChatConfig,
        state: AppState,
        user_id: UserId,
        session_id: Option<Uuid>,
        message_id: Uuid,
        step_context: StepChatContext,
    ) -> Self {
        Self {
            config,
            state,
            user_id,
            session_id,
            message_id,
            step_context: Some(step_context),
        }
    }

    /// Broadcast a workflow event when a step tool mutates data.
    fn broadcast_step_event(&self, name: &str, input: &Value, result: &Value) {
        broadcast::broadcast_step_event(
            &self.state,
            self.step_context.as_ref(),
            Some(self.user_id),
            name,
            input,
            result,
        );
    }
}

#[async_trait]
impl ExecutionStrategy for ChatStrategy {
    fn system_prompt(&self) -> &str {
        &self.config.system_prompt
    }

    fn tools(&self) -> Vec<Tool> {
        if let Some(ref ctx) = self.step_context {
            return tools::resolve_chat_step_tools(&ctx.execution_mode);
        }
        vec![]
    }

    fn model_id(&self) -> &str {
        &self.config.model_id
    }

    fn max_rounds(&self) -> u32 {
        self.config.max_rounds
    }

    fn context_budget(&self) -> usize {
        self.config.context_budget
    }

    fn streaming(&self) -> bool {
        true
    }

    fn temperature(&self) -> f32 {
        self.config.temperature
    }

    async fn build_messages(&self, input: &str) -> Result<Vec<Message>, HubError> {
        messages::build_chat_messages(&self.state, self.session_id, self.config.max_history, input)
            .await
    }

    async fn execute_tool(&self, name: &str, input: &Value) -> Value {
        if let Some(ref ctx) = self.step_context {
            if let Some(value) = tools::dispatch_step_tool(name, input, &self.state, ctx).await {
                self.broadcast_step_event(name, input, &value);
                if name == "render_panel" {
                    if let Some(content) = value["content"].as_str() {
                        let submit_label = value["submit_label"].as_str().unwrap_or("Submit");
                        self.state.send_stream_chunk(
                            self.message_id,
                            crate::server::state::StreamChunk::PanelRender {
                                content: content.to_string(),
                                submit_label: submit_label.to_string(),
                            },
                        );
                    }
                }
                return value;
            }
        }
        serde_json::json!({ "error": format!("Unknown tool: {}", name) })
    }

    fn state(&self) -> Option<&AppState> {
        Some(&self.state)
    }

    fn user_id(&self) -> Option<Uuid> {
        Some(self.user_id.0)
    }

    async fn on_complete(&self, response: &str, usage: &TokenUsage) -> Result<(), HubError> {
        completion::on_chat_complete(
            &self.state,
            self.user_id,
            self.session_id,
            self.step_context.as_ref(),
            &self.config.model_id,
            response,
            usage,
        )
        .await
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests;
