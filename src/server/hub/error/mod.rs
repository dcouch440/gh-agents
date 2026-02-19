//! Typed errors for the hub execution engine.
//!
//! Every failure path is explicit. Callers decide recovery strategy.

use thiserror::Error;
use uuid::Uuid;

use crate::llm::LLMError;

/// Errors that can occur during hub execution.
#[derive(Error, Debug)]
pub enum HubError {
    #[error("LLM provider not configured")]
    ProviderNotConfigured,

    #[error("LLM call failed (round {round}): {source}")]
    LlmCallFailed {
        round: u32,
        #[source]
        source: LLMError,
    },

    #[error("context budget exceeded: {chars} chars at round {round}")]
    ContextBudgetExceeded { chars: usize, round: u32 },

    #[error("tool '{tool_name}' failed: {reason}")]
    ToolFailed { tool_name: String, reason: String },

    #[error("max tool rounds ({max}) exhausted")]
    MaxRoundsExhausted { max: u32 },

    #[error("mode '{mode_id}' not found")]
    UnknownMode { mode_id: String },

    #[error("workflow cycle detected")]
    DagCycle,

    #[error("variable '{path}' unresolved")]
    UnresolvedVariable { path: String },

    #[error("step {step_id} agent {agent_id} not found")]
    AgentNotFound { step_id: Uuid, agent_id: Uuid },

    #[error("interactive step {step_id} (execution {execution_id}) awaiting user input")]
    AwaitingUser { step_id: Uuid, execution_id: Uuid },

    #[error("port resolution failed for step {step_id}: {reason}")]
    PortResolutionFailed { step_id: Uuid, reason: String },

    #[error("provider '{provider}' not available for step {step_id} (agent: {agent_name})")]
    ProviderUnavailable {
        provider: String,
        step_id: Uuid,
        agent_name: String,
    },

    #[error("execution cancelled")]
    Cancelled,

    #[error("stream interrupted for execution {execution_id}")]
    StreamInterrupted { execution_id: Uuid },

    #[error("prompt '{key}' not found in registry")]
    PromptNotFound { key: String },

    #[error("prompt render failed for '{key}': missing variable '{var}'")]
    PromptRenderFailed { key: String, var: String },

    #[error(transparent)]
    Db(#[from] sqlx::Error),

    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests;
