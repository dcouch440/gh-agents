//! ExecutionStrategy implementations.

pub mod agent_designer;
pub mod belief_capture;
pub mod chat;
pub mod dag_step;
pub mod dispatch;
pub mod room_speaker;
pub mod workforce_agent;

pub use agent_designer::{AgentDesignerConfig, AgentDesignerStrategy};
pub use belief_capture::{BeliefCaptureExtractorConfig, BeliefCaptureExtractorStrategy};
pub use chat::{ChatConfig, ChatStrategy, StepChatContext};
pub use dag_step::DagStepStrategy;
pub use dispatch::DispatchStrategy;
pub use room_speaker::{RoomSpeakerConfig, RoomSpeakerStrategy};
pub use workforce_agent::{WorkforceAgentConfig, WorkforceAgentStrategy};

use crate::llm::TokenUsage;
use crate::server::state::AppState;
use uuid::Uuid;

/// Log token usage to the ledger. Shared by all strategies that track costs.
pub async fn log_token_usage(
    state: &AppState,
    user_id: Uuid,
    agent_execution_id: Option<Uuid>,
    model_id: &str,
    usage: &TokenUsage,
) {
    let cost = compute_cost(
        model_id,
        usage.input_tokens as i64,
        usage.output_tokens as i64,
    );
    let _ = state
        .repos()
        .token_ledger
        .insert_ledger_entry(
            user_id,
            agent_execution_id,
            model_id,
            usage.input_tokens as i64,
            usage.output_tokens as i64,
            cost,
        )
        .await;
}

/// Complete an agent execution: log token usage + update status.
///
/// Handles optional state/user_id/ae_id gracefully (skips if None).
/// When `parse_structured` is true, attempts to parse JSON from the response.
pub async fn complete_agent_execution(
    state: Option<&AppState>,
    user_id: Option<Uuid>,
    agent_execution_id: Option<Uuid>,
    model_id: &str,
    response: &str,
    usage: &TokenUsage,
    parse_structured: bool,
) {
    if let (Some(state), Some(uid)) = (state, user_id) {
        log_token_usage(state, uid, agent_execution_id, model_id, usage).await;
    }

    if let (Some(state), Some(ae_id)) = (state, agent_execution_id) {
        let structured = if parse_structured {
            crate::server::hub::protocols::json_utils::parse_structured_output(response)
        } else {
            None
        };
        let _ = state
            .repos()
            .agent_executions
            .update_agent_execution_status(
                ae_id,
                "completed",
                Some(response.to_string()),
                structured,
            )
            .await;
    }
}

/// Approximate cost computation per model ($/1M tokens).
/// Local models (Ollama) are free — returns $0.00.
pub fn compute_cost(model_id: &str, input_tokens: i64, output_tokens: i64) -> f32 {
    // Known local model patterns — no API cost
    let is_local = model_id.contains("llama")
        || model_id.contains("mistral")
        || model_id.contains("codellama")
        || model_id.contains("gemma")
        || model_id.contains("phi")
        || model_id.contains("qwen")
        || model_id.contains("deepseek")
        || model_id.contains("vicuna");

    if is_local {
        return 0.0;
    }

    let (input_rate, output_rate) = if model_id.contains("opus") {
        (15.0_f32, 75.0_f32)
    } else if model_id.contains("sonnet") {
        (3.0, 15.0)
    } else if model_id.contains("haiku") {
        (0.25, 1.25)
    } else if model_id.contains("grok-4-0709") {
        // xAI Grok T1 (orchestrator)
        (3.0, 12.0)
    } else if model_id.contains("grok") && model_id.contains("reasoning") {
        // xAI Grok T2 (fast reasoning)
        (2.0, 8.0)
    } else if model_id.contains("grok") {
        // xAI Grok T3 / generic Grok fallback
        (0.6, 2.4)
    } else if model_id.contains("gpt-4o") {
        (2.5, 10.0)
    } else if model_id.contains("gpt-4") {
        (30.0, 60.0)
    } else {
        (1.0, 3.0)
    };

    let input_cost = (input_tokens as f32 / 1_000_000.0) * input_rate;
    let output_cost = (output_tokens as f32 / 1_000_000.0) * output_rate;
    input_cost + output_cost
}
