//! Execution filter pipeline for the LLM execution engine.
//!
//! Filters hook into three points of the execution loop:
//! - `on_start`: Before the loop — augment system prompt and initial messages
//! - `on_response`: After each LLM response — accept or trigger retry
//! - `on_output`: On final content before return — transform content

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::Value as JsonValue;
use uuid::Uuid;

use crate::llm::{LLMResponse, Message};

use crate::server::hub::error::HubError;

pub mod agent_guidance;
pub mod debate_verification;
pub mod few_shot;
pub mod partial_json_recovery;
pub mod reasoning_trace;
pub mod schema_enhancement;
pub mod schema_validation_retry;

pub use agent_guidance::AgentGuidanceFilter;
pub use debate_verification::DebateVerificationFilter;
pub use few_shot::FewShotFilter;
pub use partial_json_recovery::PartialJsonRecoveryFilter;
pub use reasoning_trace::ReasoningTraceFilter;
pub use schema_enhancement::SchemaEnhancementFilter;
pub use schema_validation_retry::SchemaValidationRetryFilter;

/// Action a filter returns after inspecting an LLM response.
#[derive(Debug, Clone)]
pub enum ResponseAction {
    /// Accept the response and continue normal flow.
    Accept,
    /// Retry the LLM call with feedback injected as a user message.
    Retry { feedback: String },
}

/// Context passed to filters at each hook point.
///
/// Carries execution metadata and a key-value store for inter-filter communication.
#[derive(Debug, Clone)]
pub struct FilterContext {
    /// The model being used for this execution.
    pub model_id: String,
    /// The agent executing this step.
    pub agent_id: Uuid,
    /// The workflow step being executed (None for non-DAG executions).
    pub step_id: Option<Uuid>,
    /// Current round number within the execution loop (set by engine).
    pub round: u32,
    /// Whether the step has an output schema configured.
    pub has_output_schema: bool,
    /// The raw JSON schema value, if one exists.
    pub output_schema: Option<JsonValue>,
    /// Key-value metadata for inter-filter communication.
    pub metadata: HashMap<String, JsonValue>,
}

impl FilterContext {
    /// Create a new FilterContext with the given model and agent IDs.
    pub fn new(model_id: &str, agent_id: Uuid) -> Self {
        Self {
            model_id: model_id.to_string(),
            agent_id,
            step_id: None,
            round: 0,
            has_output_schema: false,
            output_schema: None,
            metadata: HashMap::new(),
        }
    }

    /// Set the workflow step ID.
    pub fn with_step_id(mut self, step_id: Uuid) -> Self {
        self.step_id = Some(step_id);
        self
    }

    /// Set schema information on the context.
    pub fn with_schema(mut self, schema: JsonValue) -> Self {
        self.has_output_schema = true;
        self.output_schema = Some(schema);
        self
    }
}

/// A filter that hooks into the execution engine's lifecycle.
///
/// All methods have default no-op implementations, so filters only
/// need to override the hooks they care about.
#[async_trait]
pub trait ExecutionFilter: Send + Sync {
    /// Human-readable name for logging and debugging.
    fn name(&self) -> &str;

    /// Called before the execution loop begins.
    ///
    /// Can modify the system prompt and initial message list.
    async fn on_start(
        &self,
        _ctx: &FilterContext,
        system_prompt: String,
        messages: Vec<Message>,
    ) -> Result<(String, Vec<Message>), HubError> {
        Ok((system_prompt, messages))
    }

    /// Called after each LLM response in the execution loop.
    ///
    /// Can inspect the response and decide whether to accept it or
    /// request a retry with feedback. The engine enforces max 1 retry
    /// per filter per execution.
    async fn on_response(
        &self,
        _ctx: &FilterContext,
        _response: &LLMResponse,
    ) -> Result<ResponseAction, HubError> {
        Ok(ResponseAction::Accept)
    }

    /// Called on the final content before returning the ExecutionResult.
    ///
    /// Can transform the content string (e.g., fix truncated JSON).
    async fn on_output(&self, _ctx: &FilterContext, content: String) -> Result<String, HubError> {
        Ok(content)
    }
}

/// Type alias for a shared filter reference.
pub type SharedFilter = Arc<dyn ExecutionFilter>;

#[cfg(test)]
mod tests;
