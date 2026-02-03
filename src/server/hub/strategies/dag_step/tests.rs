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
