//! Tests for DAG step strategy

use crate::server::hub::strategies::dag_step::{compute_cost, parse_structured_output};

#[test]
fn compute_cost_sonnet() {
    let cost = compute_cost("claude-sonnet-4-20250514", 1_000_000, 500_000);
    // 3.0 + 7.5 = 10.5
    assert!((cost - 10.5).abs() < 0.01);
}

#[test]
fn compute_cost_haiku() {
    let cost = compute_cost("claude-3-haiku", 1_000_000, 1_000_000);
    // 0.25 + 1.25 = 1.50
    assert!((cost - 1.50).abs() < 0.01);
}

#[test]
fn compute_cost_opus() {
    let cost = compute_cost("claude-3-opus", 100_000, 50_000);
    // 1.5 + 3.75 = 5.25
    assert!((cost - 5.25).abs() < 0.01);
}

#[test]
fn compute_cost_unknown_model_uses_fallback() {
    // Unknown model → fallback rates (1.0 input, 3.0 output per 1M tokens)
    let cost = compute_cost("future-model-v9", 1_000_000, 1_000_000);
    // 1.0 + 3.0 = 4.0
    assert!((cost - 4.0).abs() < 0.01);
}

#[test]
fn compute_cost_zero_tokens() {
    assert_eq!(compute_cost("claude-sonnet-4-20250514", 0, 0), 0.0);
    assert_eq!(compute_cost("llama3", 0, 0), 0.0);
    assert_eq!(compute_cost("unknown", 0, 0), 0.0);
}

#[test]
fn compute_cost_local_models_free() {
    let models = [
        "llama3",
        "mistral-7b",
        "deepseek-coder",
        "codellama",
        "gemma:2b",
        "phi-3",
        "qwen2",
        "vicuna-13b",
    ];
    for model in models {
        assert_eq!(
            compute_cost(model, 1_000_000, 1_000_000),
            0.0,
            "expected 0 cost for local model: {}",
            model
        );
    }
}

#[test]
fn compute_cost_gpt4o() {
    let cost = compute_cost("gpt-4o", 1_000_000, 1_000_000);
    // 2.5 + 10.0 = 12.5
    assert!((cost - 12.5).abs() < 0.01);
}

#[test]
fn compute_cost_gpt4() {
    let cost = compute_cost("gpt-4", 1_000_000, 1_000_000);
    // 30.0 + 60.0 = 90.0
    assert!((cost - 90.0).abs() < 0.01);
}

#[test]
fn compute_cost_large_token_counts() {
    // 100M tokens — verify no overflow or NaN
    let cost = compute_cost("claude-sonnet-4-20250514", 100_000_000, 100_000_000);
    // 100 * 3.0 + 100 * 15.0 = 300 + 1500 = 1800
    assert!((cost - 1800.0).abs() < 1.0);
    assert!(cost.is_finite());
}

#[test]
fn parse_structured_output_direct_json() {
    let result = parse_structured_output(r#"{"key": "value"}"#);
    assert!(result.is_some());
    assert_eq!(result.unwrap()["key"], "value");
}

#[test]
fn parse_structured_output_code_fence() {
    let input = "Here is the result:\n```json\n{\"key\": \"value\"}\n```";
    let result = parse_structured_output(input);
    assert!(result.is_some());
    assert_eq!(result.unwrap()["key"], "value");
}

#[test]
fn parse_structured_output_embedded_json() {
    let input = "The answer is {\"key\": \"value\"} as shown.";
    let result = parse_structured_output(input);
    assert!(result.is_some());
}

#[test]
fn parse_structured_output_plain_text() {
    let result = parse_structured_output("Just plain text, no JSON here.");
    assert!(result.is_none());
}

#[test]
fn parse_structured_output_array() {
    let result = parse_structured_output(r#"[{"a": 1}, {"a": 2}]"#);
    assert!(result.is_some());
    assert!(result.unwrap().is_array());
}
