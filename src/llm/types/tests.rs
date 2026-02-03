//! Tests for LLM types

use super::*;

#[test]
fn message_user_creates_user_role() {
    let msg = Message::user("Hello");
    assert_eq!(msg.role, Role::User);
    assert_eq!(msg.text(), "Hello");
}

#[test]
fn message_assistant_creates_assistant_role() {
    let msg = Message::assistant("Hi there!");
    assert_eq!(msg.role, Role::Assistant);
    assert_eq!(msg.text(), "Hi there!");
}

#[test]
fn request_builder_works() {
    let request = LLMRequest::new("claude-3", vec![Message::user("Hi")])
        .with_system("You are helpful")
        .with_max_tokens(1000)
        .with_streaming();

    assert_eq!(request.model, "claude-3");
    assert_eq!(request.system, Some("You are helpful".to_string()));
    assert_eq!(request.max_tokens, 1000);
    assert!(request.stream);
}

#[test]
fn request_defaults_are_sensible() {
    let request = LLMRequest::new("claude-3", vec![]);
    assert_eq!(request.max_tokens, 4096);
    assert!((request.temperature - 0.7).abs() < f32::EPSILON);
    assert!(!request.stream);
    assert!(request.system.is_none());
}

#[test]
fn token_usage_total() {
    let usage = TokenUsage {
        input_tokens: 100,
        output_tokens: 50,
    };
    assert_eq!(usage.total(), 150);
}

#[test]
fn accumulator_builds_response() {
    let mut acc = StreamAccumulator::new();
    acc.apply(&StreamChunk::MessageStart {
        model: "claude-3".to_string(),
        input_tokens: 10,
    });
    acc.apply(&StreamChunk::ContentDelta {
        text: "Hello ".to_string(),
        index: 0,
    });
    acc.apply(&StreamChunk::ContentDelta {
        text: "world!".to_string(),
        index: 0,
    });
    acc.apply(&StreamChunk::MessageDelta {
        stop_reason: Some(StopReason::EndTurn),
        output_tokens: Some(5),
    });

    let response = acc.build().unwrap();
    assert_eq!(response.content, "Hello world!");
    assert_eq!(response.model, "claude-3");
    assert_eq!(response.stop_reason, StopReason::EndTurn);
}

#[test]
fn accumulator_returns_none_if_incomplete() {
    let acc = StreamAccumulator::new();
    assert!(acc.build().is_none());
}

#[test]
fn message_serialization_works() {
    let msg = Message::user("Hello");
    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains("\"role\":\"user\""));
    assert!(json.contains("\"content\":\"Hello\""));

    let parsed: Message = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, msg);
}

#[test]
fn request_serialization_works() {
    let request =
        LLMRequest::new("claude-3", vec![Message::user("Hi")]).with_system("Be helpful");

    let json = serde_json::to_string(&request).unwrap();
    let parsed: LLMRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, request);
}
