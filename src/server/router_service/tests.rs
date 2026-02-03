//! Tests for router service

use super::*;
use serde_json::Value;

#[test]
fn extract_json_from_code_fence() {
    let input = r#"Here's the decision:
```json
{"tool": "read_file", "tool_args": {"path": "/tmp/foo"}, "is_async": false, "reason": "test"}
```"#;
    let result = extract_json(input);
    let parsed: Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["tool"], "read_file");
}

#[test]
fn extract_json_raw() {
    let input = r#"{"tool": null, "reason": "no match"}"#;
    let result = extract_json(input);
    let parsed: Value = serde_json::from_str(&result).unwrap();
    assert!(parsed["tool"].is_null());
}

#[test]
fn parse_decision_no_tool() {
    let json = r#"{"tool": null, "tool_args": null, "is_async": false, "reason": "nothing relevant"}"#;
    let decision = parse_router_decision(json).unwrap();
    assert!(decision.tool.is_none());
    assert_eq!(decision.reason.as_deref(), Some("nothing relevant"));
}

#[test]
fn parse_decision_sync() {
    let json = r#"{"tool": "search", "tool_args": {"query": "test"}, "is_async": false, "reason": "search needed"}"#;
    let decision = parse_router_decision(json).unwrap();
    assert_eq!(decision.tool.as_deref(), Some("search"));
    assert!(!decision.is_async);
}

#[test]
fn parse_decision_async_with_passdown() {
    let json = r#"{"tool": "analyze_repo", "tool_args": {}, "is_async": true, "passdown": "Analyzing the repo now...", "reason": "heavy"}"#;
    let decision = parse_router_decision(json).unwrap();
    assert!(decision.is_async);
    assert_eq!(decision.passdown.as_deref(), Some("Analyzing the repo now..."));
}

#[test]
fn build_tool_specs_empty() {
    assert_eq!(build_tool_specs(&[]), "No tools available.");
}

#[test]
fn build_tool_specs_formats_correctly() {
    let tools = vec![Tool {
        name: "read_file".to_string(),
        description: "Read a file from disk".to_string(),
        input_schema: json!({"type": "object", "properties": {}}),
    }];
    let specs = build_tool_specs(&tools);
    assert!(specs.contains("**read_file**"));
    assert!(specs.contains("Read a file from disk"));
}

#[test]
fn truncate_short_string() {
    assert_eq!(truncate("hello", 10), "hello");
}

#[test]
fn truncate_long_string() {
    let result = truncate("hello world this is long", 10);
    assert!(result.ends_with('…'));
    assert!(result.starts_with("hello wor"));
}
