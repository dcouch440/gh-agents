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
    let request = LLMRequest::new("claude-3", vec![Message::user("Hi")]).with_system("Be helpful");

    let json = serde_json::to_string(&request).unwrap();
    let parsed: LLMRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, request);
}

// ── StreamAccumulator: tool use ──────────────────────────────────────

#[test]
fn accumulator_tool_use_single_block() {
    let mut acc = StreamAccumulator::new();
    acc.apply(&StreamChunk::MessageStart {
        model: "claude-3".to_string(),
        input_tokens: 20,
    });
    acc.apply(&StreamChunk::ToolUseStart {
        index: 0,
        id: "toolu_123".to_string(),
        name: "get_weather".to_string(),
    });
    acc.apply(&StreamChunk::InputJsonDelta {
        index: 0,
        partial_json: r#"{"city":"#.to_string(),
    });
    acc.apply(&StreamChunk::InputJsonDelta {
        index: 0,
        partial_json: r#""London"}"#.to_string(),
    });
    acc.apply(&StreamChunk::ContentBlockStop { index: 0 });
    acc.apply(&StreamChunk::MessageDelta {
        stop_reason: Some(StopReason::ToolUse),
        output_tokens: Some(15),
    });

    let response = acc.build().unwrap();
    assert_eq!(response.stop_reason, StopReason::ToolUse);
    assert!(response.content.is_empty());
    assert_eq!(response.content_blocks.len(), 1);
    match &response.content_blocks[0] {
        ContentBlock::ToolUse { id, name, input } => {
            assert_eq!(id, "toolu_123");
            assert_eq!(name, "get_weather");
            assert_eq!(input, &serde_json::json!({"city": "London"}));
        }
        _ => panic!("expected ToolUse content block"),
    }
}

#[test]
fn accumulator_text_plus_tool_use() {
    let mut acc = StreamAccumulator::new();
    acc.apply(&StreamChunk::MessageStart {
        model: "claude-3".to_string(),
        input_tokens: 10,
    });
    // Text block first
    acc.apply(&StreamChunk::ContentDelta {
        text: "Let me check".to_string(),
        index: 0,
    });
    acc.apply(&StreamChunk::ContentBlockStop { index: 0 });
    // Tool use block second
    acc.apply(&StreamChunk::ToolUseStart {
        index: 1,
        id: "t1".to_string(),
        name: "search".to_string(),
    });
    acc.apply(&StreamChunk::InputJsonDelta {
        index: 1,
        partial_json: r#"{"q":"test"}"#.to_string(),
    });
    acc.apply(&StreamChunk::ContentBlockStop { index: 1 });
    acc.apply(&StreamChunk::MessageDelta {
        stop_reason: Some(StopReason::ToolUse),
        output_tokens: Some(20),
    });

    let response = acc.build().unwrap();
    assert_eq!(response.content, "Let me check");
    assert_eq!(response.content_blocks.len(), 2);
    assert!(
        matches!(&response.content_blocks[0], ContentBlock::Text { text } if text == "Let me check")
    );
    assert!(
        matches!(&response.content_blocks[1], ContentBlock::ToolUse { name, .. } if name == "search")
    );
}

#[test]
fn accumulator_tool_use_invalid_json_input() {
    let mut acc = StreamAccumulator::new();
    acc.apply(&StreamChunk::MessageStart {
        model: "claude-3".to_string(),
        input_tokens: 5,
    });
    acc.apply(&StreamChunk::ToolUseStart {
        index: 0,
        id: "t1".to_string(),
        name: "broken".to_string(),
    });
    acc.apply(&StreamChunk::InputJsonDelta {
        index: 0,
        partial_json: "not valid json".to_string(),
    });
    acc.apply(&StreamChunk::ContentBlockStop { index: 0 });
    acc.apply(&StreamChunk::MessageDelta {
        stop_reason: Some(StopReason::ToolUse),
        output_tokens: Some(5),
    });

    let response = acc.build().unwrap();
    match &response.content_blocks[0] {
        ContentBlock::ToolUse { input, .. } => {
            assert_eq!(input, &serde_json::json!({}));
        }
        _ => panic!("expected ToolUse content block"),
    }
}

#[test]
fn accumulator_tool_use_multiple_json_chunks() {
    let mut acc = StreamAccumulator::new();
    acc.apply(&StreamChunk::MessageStart {
        model: "claude-3".to_string(),
        input_tokens: 10,
    });
    acc.apply(&StreamChunk::ToolUseStart {
        index: 0,
        id: "t1".to_string(),
        name: "create_file".to_string(),
    });
    // Split JSON across 4 chunks
    acc.apply(&StreamChunk::InputJsonDelta {
        index: 0,
        partial_json: r#"{"#.to_string(),
    });
    acc.apply(&StreamChunk::InputJsonDelta {
        index: 0,
        partial_json: r#""path":"#.to_string(),
    });
    acc.apply(&StreamChunk::InputJsonDelta {
        index: 0,
        partial_json: r#""src/main.rs","#.to_string(),
    });
    acc.apply(&StreamChunk::InputJsonDelta {
        index: 0,
        partial_json: r#""content":"hello"}"#.to_string(),
    });
    acc.apply(&StreamChunk::ContentBlockStop { index: 0 });
    acc.apply(&StreamChunk::MessageDelta {
        stop_reason: Some(StopReason::ToolUse),
        output_tokens: Some(10),
    });

    let response = acc.build().unwrap();
    match &response.content_blocks[0] {
        ContentBlock::ToolUse { input, .. } => {
            assert_eq!(
                input,
                &serde_json::json!({"path": "src/main.rs", "content": "hello"})
            );
        }
        _ => panic!("expected ToolUse content block"),
    }
}
