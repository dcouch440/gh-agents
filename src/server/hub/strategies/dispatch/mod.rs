//! DispatchStrategy — ExecutionStrategy for background dispatch agents.
//!
//! Runs asynchronously to configure a workforce step based on a plain English
//! instruction. Reuses the same workforce tools that the step-scoped chat
//! assistant uses, but runs in the background with no streaming.

use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

use crate::config::protocols::{roles, AgentConfig, WORKFORCE_BUILDER};
use crate::llm::{Message, Tool};
use crate::server::hub::error::HubError;
use crate::server::hub::protocols::template_resolve::resolve_template;
use crate::server::hub::strategy::ExecutionStrategy;
use crate::server::state::AppState;
use crate::server::tools::workforce::{self, WorkforceToolContext};

use super::chat::broadcast;
use super::chat::config::StepChatContext;

mod tests;

/// Strategy for background dispatch agents.
///
/// Given a plain English instruction and the current step state,
/// the agent decides what workforce tools to call to configure the step.
pub struct DispatchStrategy {
    system_prompt: String,
    instruction: String,
    state: AppState,
    step_id: Uuid,
    workflow_id: Uuid,
}

impl DispatchStrategy {
    /// Protocol config for the dispatcher agent role.
    fn config(&self) -> &AgentConfig {
        WORKFORCE_BUILDER.agent("dispatcher")
    }

    /// Broadcast a workflow event after a tool mutation.
    ///
    /// Reuses the same broadcast logic as ChatStrategy so the frontend
    /// sees live updates from background dispatch agents.
    fn broadcast_tool_event(&self, name: &str, input: &Value, result: &Value) {
        let step_ctx = StepChatContext {
            workflow_id: self.workflow_id,
            step_id: self.step_id,
            execution_mode: "workforce".to_string(),
            step_name: String::new(),
        };
        broadcast::broadcast_step_event(
            &self.state,
            Some(&step_ctx),
            None, // no user — background agent
            name,
            input,
            result,
        );
    }

    /// Build a new dispatch strategy.
    ///
    /// Loads the current step state snapshot and builds a system prompt
    /// that instructs the agent to configure the step.
    pub async fn new(
        state: AppState,
        step_id: Uuid,
        workflow_id: Uuid,
        instruction: String,
    ) -> Result<Self, String> {
        let board_state_xml = crate::server::hub::board_state::build(
            state.repos().workflows.as_ref(),
            crate::server::hub::board_state::BoardStateVariant::Dispatch,
            workflow_id,
            step_id,
        )
        .await
        .map_err(|e| e.to_string())?;

        let mut vars = HashMap::new();
        vars.insert("System.board_state".to_string(), board_state_xml);

        let system_prompt = resolve_template(roles::WORKFORCE_BUILDER_SYSTEM, &vars);

        Ok(Self {
            system_prompt,
            instruction,
            state,
            step_id,
            workflow_id,
        })
    }
}

#[async_trait]
impl ExecutionStrategy for DispatchStrategy {
    fn system_prompt(&self) -> &str {
        &self.system_prompt
    }

    fn tools(&self) -> Vec<Tool> {
        super::chat::tools::resolve_step_tools("workforce")
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
        let board_state_xml = crate::server::hub::board_state::build(
            self.state.repos().workflows.as_ref(),
            crate::server::hub::board_state::BoardStateVariant::Dispatch,
            self.workflow_id,
            self.step_id,
        )
        .await
        .map_err(|e| HubError::Internal(anyhow::anyhow!("{}", e)))?;

        let mut vars = HashMap::new();
        vars.insert("System.board_state".to_string(), board_state_xml);

        Ok(Some(resolve_template(
            roles::WORKFORCE_BUILDER_SYSTEM,
            &vars,
        )))
    }

    async fn build_messages(&self, _input: &str) -> Result<Vec<Message>, HubError> {
        Ok(vec![Message::user(&self.instruction)])
    }

    async fn execute_tool(&self, name: &str, input: &Value) -> Value {
        let ctx = WorkforceToolContext {
            workflow_id: self.workflow_id,
            step_id: self.step_id,
        };

        // Route universal step tools
        let node_tools = &["set_node_name", "set_node_description", "render_panel"];
        let result = if node_tools.contains(&name) {
            let tool_ctx = crate::server::tools::node_assistant::StepToolContext {
                workflow_id: self.workflow_id,
                step_id: self.step_id,
            };
            crate::server::tools::node_assistant::execute_node_assistant_tool(
                name,
                input,
                self.state.repos().workflows.as_ref(),
                &tool_ctx,
            )
            .await
        } else if name == "update_notes" {
            let content = input["content"].as_str().unwrap_or("");
            match self
                .state
                .repos()
                .workflows
                .upsert_assistant_notes(self.step_id, content)
                .await
            {
                Ok(()) => serde_json::json!({ "status": "ok" }),
                Err(e) => serde_json::json!({ "error": e.to_string() }),
            }
        } else if name == "think" {
            serde_json::json!({ "status": "ok" })
        } else {
            // Route workforce-specific tools
            workforce::execute_workforce_tool(
                name,
                input,
                self.state.repos().workflows.as_ref(),
                &ctx,
            )
            .await
        };

        // Broadcast workflow event so the frontend updates live
        self.broadcast_tool_event(name, input, &result);

        result
    }
}
