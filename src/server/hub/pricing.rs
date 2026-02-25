//! Model pricing for token cost computation.
//!
//! Centralized pricing rates for all supported LLM providers.
//! Local models (Ollama) are free — returns $0.00.

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
