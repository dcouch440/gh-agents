//! ExecutionStrategy implementations.

pub mod chat;
pub mod dag_step;
pub mod interactive_chat;
pub mod room_speaker;
pub mod router;

pub use chat::{ChatConfig, ChatStrategy};
pub use dag_step::DagStepStrategy;
pub use interactive_chat::{InteractiveChatConfig, InteractiveChatStrategy};
pub use room_speaker::{RoomSpeakerConfig, RoomSpeakerStrategy};
pub use router::RouterStrategy;

/// Approximate cost computation per model ($/1M tokens).
pub fn compute_cost(model_id: &str, input_tokens: i64, output_tokens: i64) -> f32 {
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
