//! ExecutionStrategy implementations.

pub mod cavernous;
pub mod chat;
pub mod dag_step;
pub mod documenter_research;
pub mod documenter_strategy;
pub mod documenter_writer;
pub mod interactive_chat;
pub mod room_speaker;
pub mod router;

pub use cavernous::CavernousStepStrategy;
pub use chat::{ChatConfig, ChatStrategy};
pub use dag_step::DagStepStrategy;
pub use documenter_research::DocumenterResearchStrategy;
pub use documenter_strategy::DocumenterStrategyStrategy;
pub use documenter_writer::DocumenterWriterStrategy;
pub use interactive_chat::{InteractiveChatConfig, InteractiveChatStrategy};
pub use room_speaker::{RoomSpeakerConfig, RoomSpeakerStrategy};
pub use router::RouterStrategy;

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
    let cost = compute_cost(model_id, usage.input_tokens as i64, usage.output_tokens as i64);
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
