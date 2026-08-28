//! Configuration types for chat strategy.

use uuid::Uuid;

/// Configuration for a chat execution.
pub struct ChatConfig {
    pub system_prompt: String,
    pub tool_names: Vec<String>,
    pub model_id: String,
    pub max_rounds: u32,
    pub context_budget: usize,
    pub temperature: f32,
    pub max_history: u32,
    pub max_tokens: u32,
    pub effort: Option<crate::llm::ReasoningEffort>,
}

impl Default for ChatConfig {
    fn default() -> Self {
        Self {
            system_prompt: String::new(),
            tool_names: vec![],
            model_id: String::new(),
            max_rounds: 10,
            context_budget: 480_000,
            temperature: crate::constants::DEFAULT_TEMPERATURE,
            max_history: 50,
            // The worker budget, not the utility one: this is what the engine
            // hardcoded before max_tokens was configurable, and dropping to
            // the utility default would halve every chat response.
            max_tokens: crate::constants::DEFAULT_MAX_TOKENS_WORKER,
            effort: None,
        }
    }
}

/// Optional context for step-scoped chat sessions.
///
/// When present, `execute_tool` routes step-specific tools to the
/// appropriate dispatcher (e.g., workforce tools) instead of
/// generic server tools.
pub struct StepChatContext {
    pub workflow_id: Uuid,
    pub step_id: Uuid,
    pub execution_mode: String,
    pub step_name: String,
}
