//! DispatchStrategy — ExecutionStrategy for background dispatch agents.
//!
//! Runs asynchronously to configure a workforce step based on a plain English
//! instruction. Reuses the same workforce tools that the step-scoped chat
//! assistant uses, but runs in the background with no streaming.

use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

use crate::config::protocols::{roles, AgentConfig, WORKFORCE_BUILDER};
use crate::llm::{Message, Tool};
use crate::server::hub::error::HubError;
use crate::server::hub::protocols::template_resolve::resolve_template;
use crate::server::hub::strategy::ExecutionStrategy;
use crate::server::services::dispatch::Passdown;
use crate::server::state::AppState;
use crate::server::tools::workforce::{self, WorkforceToolContext};

use super::chat::broadcast;
use super::chat::config::StepChatContext;

mod tests;

/// Strategy for background dispatch agents.
///
/// Given a plain English instruction and the current step state,
/// the agent decides what workforce tools to call to configure the step.
///
/// Persistent: The `session_id` links to a builder session that accumulates
/// history across dispatches, so the L4 agent can see prior work.
pub struct DispatchStrategy {
    system_prompt: String,
    instruction: String,
    state: AppState,
    step_id: Uuid,
    workflow_id: Uuid,
    session_id: Option<Uuid>,
    /// Captured passdown from `complete_task` tool call.
    passdown: Mutex<Option<Passdown>>,
    /// Agent execution ID for debug stream events.
    agent_execution_id: Option<Uuid>,
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
    ///
    /// `session_id` links to the persistent L4 builder session. When present,
    /// `build_messages()` loads prior dispatch history for continuity.
    pub async fn new(
        state: AppState,
        step_id: Uuid,
        workflow_id: Uuid,
        instruction: String,
        session_id: Option<Uuid>,
    ) -> Result<Self, String> {
        let board_state_xml = crate::server::hub::board_state::build(
            state.repos().workflows.as_ref(),
            None, // Dispatch doesn't need initial_instructions
            crate::server::hub::board_state::BoardStateVariant::Dispatch,
            workflow_id,
            step_id,
        )
        .await
        .map_err(|e| e.to_string())?;

        let dispatch_status_xml =
            crate::server::hub::dispatch_status::build(state.task_registry(), step_id);

        let mut vars = HashMap::new();
        vars.insert("System.board_state".to_string(), board_state_xml);
        vars.insert("System.dispatch_status".to_string(), dispatch_status_xml);

        let system_prompt = resolve_template(roles::WORKFORCE_BUILDER_SYSTEM, &vars);

        Ok(Self {
            system_prompt,
            instruction,
            state,
            step_id,
            workflow_id,
            session_id,
            passdown: Mutex::new(None),
            agent_execution_id: None,
        })
    }

    /// Set the agent execution ID (created after strategy construction).
    pub fn set_agent_execution_id(&mut self, id: Option<Uuid>) {
        self.agent_execution_id = id;
    }

    /// Take the captured passdown, if any.
    ///
    /// Called by the executor after the engine loop ends.
    pub fn take_passdown(&self) -> Option<Passdown> {
        self.passdown.lock().ok().and_then(|mut p| p.take())
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

    fn should_stop(&self) -> bool {
        self.passdown.lock().map(|p| p.is_some()).unwrap_or(false)
    }

    fn temperature(&self) -> f32 {
        self.config().temperature
    }

    fn state(&self) -> Option<&AppState> {
        Some(&self.state)
    }

    fn agent_execution_id(&self) -> Option<Uuid> {
        self.agent_execution_id
    }

    async fn rebuild_system_prompt(&self) -> Result<Option<String>, HubError> {
        let board_state_xml = crate::server::hub::board_state::build(
            self.state.repos().workflows.as_ref(),
            None,
            crate::server::hub::board_state::BoardStateVariant::Dispatch,
            self.workflow_id,
            self.step_id,
        )
        .await
        .map_err(|e| HubError::Internal(anyhow::anyhow!("{}", e)))?;

        let dispatch_status_xml =
            crate::server::hub::dispatch_status::build(self.state.task_registry(), self.step_id);

        let mut vars = HashMap::new();
        vars.insert("System.board_state".to_string(), board_state_xml);
        vars.insert("System.dispatch_status".to_string(), dispatch_status_xml);

        Ok(Some(resolve_template(
            roles::WORKFORCE_BUILDER_SYSTEM,
            &vars,
        )))
    }

    async fn build_messages(&self, _input: &str) -> Result<Vec<Message>, HubError> {
        let text_instruction = if let Some(session_id) = self.session_id {
            let history = self
                .state
                .repos()
                .sessions
                .get_session_history(session_id, 20)
                .await
                .unwrap_or_default();

            if !history.is_empty() {
                super::build_pruned_instruction(&history, &self.instruction, 3)
            } else {
                self.instruction.clone()
            }
        } else {
            self.instruction.clone()
        };

        Ok(vec![Message::user(&text_instruction)])
    }

    async fn execute_tool(&self, name: &str, input: &Value) -> Value {
        // Handle complete_task — capture passdown and signal completion
        if name == "complete_task" {
            let passdown = Passdown {
                summary: input["summary"].as_str().unwrap_or("").to_string(),
                question: input["question"].as_str().map(String::from),
            };
            if let Ok(mut guard) = self.passdown.lock() {
                *guard = Some(passdown);
            }
            return serde_json::json!({ "status": "completed" });
        }

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
