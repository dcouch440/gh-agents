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

    #[error("for_each ref '{reference}' did not resolve to array")]
    ForEachNotArray { reference: String },

    #[error("step {step_id} agent {agent_id} not found")]
    AgentNotFound { step_id: Uuid, agent_id: Uuid },

    #[error("interactive step {step_id} (execution {execution_id}) awaiting user input")]
    AwaitingUser { step_id: Uuid, execution_id: Uuid },

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
mod tests {
    use super::*;

    #[test]
    fn display_provider_not_configured() {
        let err = HubError::ProviderNotConfigured;
        assert_eq!(err.to_string(), "LLM provider not configured");
    }

    #[test]
    fn display_llm_call_failed() {
        let err = HubError::LlmCallFailed {
            round: 3,
            source: LLMError::Timeout(5000),
        };
        assert!(err.to_string().contains("round 3"));
    }

    #[test]
    fn display_context_budget() {
        let err = HubError::ContextBudgetExceeded { chars: 500_000, round: 7 };
        assert!(err.to_string().contains("500000"));
        assert!(err.to_string().contains("round 7"));
    }

    #[test]
    fn display_tool_failed() {
        let err = HubError::ToolFailed {
            tool_name: "search".into(),
            reason: "timeout".into(),
        };
        assert!(err.to_string().contains("search"));
        assert!(err.to_string().contains("timeout"));
    }

    #[test]
    fn display_max_rounds() {
        let err = HubError::MaxRoundsExhausted { max: 10 };
        assert!(err.to_string().contains("10"));
    }

    #[test]
    fn display_unknown_mode() {
        let err = HubError::UnknownMode { mode_id: "ghost".into() };
        assert!(err.to_string().contains("ghost"));
    }

    #[test]
    fn display_dag_cycle() {
        let err = HubError::DagCycle;
        assert_eq!(err.to_string(), "workflow cycle detected");
    }

    #[test]
    fn display_unresolved_variable() {
        let err = HubError::UnresolvedVariable { path: "steps.0.output".into() };
        assert!(err.to_string().contains("steps.0.output"));
    }

    #[test]
    fn display_for_each_not_array() {
        let err = HubError::ForEachNotArray { reference: "results".into() };
        assert!(err.to_string().contains("results"));
    }

    #[test]
    fn display_agent_not_found() {
        let step = Uuid::nil();
        let agent = Uuid::nil();
        let err = HubError::AgentNotFound { step_id: step, agent_id: agent };
        assert!(err.to_string().contains("00000000"));
    }

    #[test]
    fn display_awaiting_user() {
        let err = HubError::AwaitingUser {
            step_id: Uuid::nil(),
            execution_id: Uuid::nil(),
        };
        assert!(err.to_string().contains("awaiting user input"));
    }

    #[test]
    fn display_cancelled() {
        let err = HubError::Cancelled;
        assert_eq!(err.to_string(), "execution cancelled");
    }

    #[test]
    fn display_stream_interrupted() {
        let err = HubError::StreamInterrupted { execution_id: Uuid::nil() };
        assert!(err.to_string().contains("interrupted"));
    }

    #[test]
    fn display_prompt_not_found() {
        let err = HubError::PromptNotFound { key: "modes/home".into() };
        assert!(err.to_string().contains("modes/home"));
    }

    #[test]
    fn display_prompt_render_failed() {
        let err = HubError::PromptRenderFailed {
            key: "agents/worker".into(),
            var: "agent_name".into(),
        };
        assert!(err.to_string().contains("agents/worker"));
        assert!(err.to_string().contains("agent_name"));
    }

    #[test]
    fn from_anyhow() {
        let inner = anyhow::anyhow!("something broke");
        let err: HubError = inner.into();
        assert!(err.to_string().contains("something broke"));
    }
}
