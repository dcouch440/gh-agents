//! DispatchStrategy — ExecutionStrategy for background dispatch agents.
//!
//! Runs asynchronously to configure a workforce step based on a plain English
//! instruction. Reuses the same workforce tools that the step-scoped chat
//! assistant uses, but runs in the background with no streaming.

use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

use crate::config::protocols::roles;
use crate::llm::{Message, Tool};
use crate::server::hub::error::HubError;
use crate::server::hub::protocols::template_resolve::resolve_template;
use crate::server::hub::strategy::ExecutionStrategy;
use crate::server::state::AppState;
use crate::server::tools::workforce::{self, WorkforceToolContext};

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
        let ctx = WorkforceToolContext {
            workflow_id,
            step_id,
        };

        let snapshot =
            workforce::build_config_snapshot(state.repos().workflows.as_ref(), &ctx).await?;

        let mut vars = HashMap::new();
        vars.insert("System.current_config".to_string(), snapshot);

        let system_prompt = resolve_template(roles::DISPATCH_SYSTEM, &vars);

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
        crate::constants::DEFAULT_MODEL
    }

    fn max_rounds(&self) -> u32 {
        15
    }

    fn context_budget(&self) -> usize {
        200_000
    }

    fn streaming(&self) -> bool {
        false
    }

    fn temperature(&self) -> f32 {
        0.3
    }

    fn state(&self) -> Option<&AppState> {
        Some(&self.state)
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
        if node_tools.contains(&name) {
            let tool_ctx = crate::server::tools::node_assistant::StepToolContext {
                workflow_id: self.workflow_id,
                step_id: self.step_id,
            };
            return crate::server::tools::node_assistant::execute_node_assistant_tool(
                name,
                input,
                self.state.repos().workflows.as_ref(),
                &tool_ctx,
            )
            .await;
        }

        // Route update_notes
        if name == "update_notes" {
            let content = input["content"].as_str().unwrap_or("");
            match self
                .state
                .repos()
                .workflows
                .upsert_assistant_notes(self.step_id, content)
                .await
            {
                Ok(()) => return serde_json::json!({ "status": "ok" }),
                Err(e) => return serde_json::json!({ "error": e.to_string() }),
            }
        }

        // Route think tool
        if name == "think" {
            return serde_json::json!({ "status": "ok" });
        }

        // Route workforce-specific tools
        workforce::execute_workforce_tool(name, input, self.state.repos().workflows.as_ref(), &ctx)
            .await
    }
}
