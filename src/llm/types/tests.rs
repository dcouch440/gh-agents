#[cfg(test)]
mod tests {
    //! Tests for LLM types

    use super::super::*;

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
            ..Default::default()
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
    fn accumulator_tool_blocks_override_end_turn_stop_reason() {
        // xAI Responses API may report EndTurn in response.completed even
        // when function_call blocks were streamed. The accumulator should
        // override stop_reason to ToolUse based on accumulated blocks.
        let mut acc = StreamAccumulator::new();
        acc.apply(&StreamChunk::MessageStart {
            model: "grok-4".to_string(),
            input_tokens: 10,
        });
        acc.apply(&StreamChunk::ToolUseStart {
            index: 0,
            id: "call_1".to_string(),
            name: "dispatch".to_string(),
        });
        acc.apply(&StreamChunk::InputJsonDelta {
            index: 0,
            partial_json: r#"{"instruction":"add agents"}"#.to_string(),
        });
        acc.apply(&StreamChunk::ContentBlockStop { index: 0 });
        // Provider reports EndTurn instead of ToolUse
        acc.apply(&StreamChunk::MessageDelta {
            stop_reason: Some(StopReason::EndTurn),
            output_tokens: Some(15),
        });

        let response = acc.build().unwrap();
        // Must be ToolUse — the blocks are authoritative
        assert_eq!(response.stop_reason, StopReason::ToolUse);
        assert_eq!(response.content_blocks.len(), 1);
        assert!(
            matches!(&response.content_blocks[0], ContentBlock::ToolUse { name, .. } if name == "dispatch")
        );
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

    // ── ContentBlock::Image ─────────────────────────────────────────

    #[test]
    fn image_content_block_serializes_for_anthropic_api() {
        let block = ContentBlock::image_png_base64("iVBORw0KGgo".to_string());
        let json = serde_json::to_value(&block).unwrap();
        assert_eq!(json["type"], "image");
        assert_eq!(json["source"]["type"], "base64");
        assert_eq!(json["source"]["media_type"], "image/png");
        assert_eq!(json["source"]["data"], "iVBORw0KGgo");
    }

    #[test]
    fn image_content_block_roundtrips() {
        let block = ContentBlock::image_png_base64("AAAA".to_string());
        let json = serde_json::to_string(&block).unwrap();
        let parsed: ContentBlock = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, block);
    }

    #[test]
    fn user_message_with_text_and_image_blocks() {
        let msg = Message::user_with_blocks(vec![
            ContentBlock::Text {
                text: "Describe this sketch:".to_string(),
            },
            ContentBlock::image_png_base64("iVBOR".to_string()),
        ]);
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["role"], "user");
        let content = json["content"].as_array().unwrap();
        assert_eq!(content.len(), 2);
        assert_eq!(content[0]["type"], "text");
        assert_eq!(content[1]["type"], "image");
    }

    #[test]
    fn image_estimated_chars_returns_data_length() {
        let block = ContentBlock::image_png_base64("ABCDEF".to_string());
        assert_eq!(block.estimated_chars(), 6);
    }
}

#[cfg(test)]
mod accumulator_tests {
    use super::super::*;

    fn tool_names(r: &LLMResponse) -> Vec<(&str, String)> {
        r.content_blocks
            .iter()
            .filter_map(|b| match b {
                ContentBlock::ToolUse { name, input, .. } => {
                    Some((name.as_str(), input.to_string()))
                }
                _ => None,
            })
            .collect()
    }

    // The bug: a single in-progress slot finalized tool 0 with empty arguments
    // when tool 1 opened, then fed tool 0's arguments into tool 1.
    #[test]
    fn interleaved_tool_call_deltas_stay_with_their_own_index() {
        let mut acc = StreamAccumulator::new();
        acc.apply(&StreamChunk::MessageStart {
            model: "m".into(),
            input_tokens: 1,
        });
        acc.apply(&StreamChunk::ToolUseStart {
            index: 0,
            id: "a".into(),
            name: "alpha".into(),
        });
        acc.apply(&StreamChunk::ToolUseStart {
            index: 1,
            id: "b".into(),
            name: "beta".into(),
        });
        acc.apply(&StreamChunk::InputJsonDelta {
            index: 0,
            partial_json: r#"{"x":1}"#.into(),
        });
        acc.apply(&StreamChunk::InputJsonDelta {
            index: 1,
            partial_json: r#"{"y":2}"#.into(),
        });
        acc.apply(&StreamChunk::ContentBlockStop { index: 0 });
        acc.apply(&StreamChunk::ContentBlockStop { index: 1 });
        acc.apply(&StreamChunk::MessageStop);

        let r = acc.build().expect("response");
        let tools = tool_names(&r);
        assert_eq!(tools.len(), 2, "both tool calls must survive");
        assert_eq!(tools[0], ("alpha", r#"{"x":1}"#.to_string()));
        assert_eq!(tools[1], ("beta", r#"{"y":2}"#.to_string()));
    }

    #[test]
    fn split_argument_deltas_reassemble_per_index() {
        let mut acc = StreamAccumulator::new();
        acc.apply(&StreamChunk::MessageStart {
            model: "m".into(),
            input_tokens: 0,
        });
        acc.apply(&StreamChunk::ToolUseStart {
            index: 0,
            id: "a".into(),
            name: "alpha".into(),
        });
        for frag in [r#"{"path""#, r#":"#, r#""a.rs"}"#] {
            acc.apply(&StreamChunk::InputJsonDelta {
                index: 0,
                partial_json: frag.into(),
            });
        }
        acc.apply(&StreamChunk::ContentBlockStop { index: 0 });
        let r = acc.build().expect("response");
        assert_eq!(tool_names(&r)[0].1, r#"{"path":"a.rs"}"#);
    }

    // Not every provider emits a stop per block; dropping the call entirely
    // would lose the model's intent.
    #[test]
    fn tool_uses_without_a_stop_event_are_still_emitted() {
        let mut acc = StreamAccumulator::new();
        acc.apply(&StreamChunk::MessageStart {
            model: "m".into(),
            input_tokens: 0,
        });
        acc.apply(&StreamChunk::ToolUseStart {
            index: 0,
            id: "a".into(),
            name: "alpha".into(),
        });
        acc.apply(&StreamChunk::InputJsonDelta {
            index: 0,
            partial_json: r#"{"x":1}"#.into(),
        });
        let r = acc.build().expect("response");
        assert_eq!(tool_names(&r).len(), 1);
        assert_eq!(r.stop_reason, StopReason::ToolUse);
    }

    #[test]
    fn malformed_tool_arguments_become_an_empty_object() {
        let mut acc = StreamAccumulator::new();
        acc.apply(&StreamChunk::MessageStart {
            model: "m".into(),
            input_tokens: 0,
        });
        acc.apply(&StreamChunk::ToolUseStart {
            index: 0,
            id: "a".into(),
            name: "alpha".into(),
        });
        acc.apply(&StreamChunk::InputJsonDelta {
            index: 0,
            partial_json: "{not json".into(),
        });
        acc.apply(&StreamChunk::ContentBlockStop { index: 0 });
        let r = acc.build().expect("response");
        assert_eq!(tool_names(&r)[0].1, "{}");
    }

    // An empty model id falls through every pricing branch to the generic
    // fallback and lands empty in the token ledger.
    #[test]
    fn a_later_frame_without_a_model_does_not_blank_it() {
        let mut acc = StreamAccumulator::new();
        acc.apply(&StreamChunk::MessageStart {
            model: "deepseek-ai/DeepSeek-V4-Flash-0731".into(),
            input_tokens: 10,
        });
        acc.apply(&StreamChunk::MessageStart {
            model: String::new(),
            input_tokens: 10,
        });
        acc.apply(&StreamChunk::MessageDelta {
            stop_reason: Some(StopReason::EndTurn),
            output_tokens: Some(5),
        });
        let r = acc.build().expect("response");
        assert_eq!(r.model, "deepseek-ai/DeepSeek-V4-Flash-0731");
    }

    #[test]
    fn usage_update_carries_cached_tokens_into_the_response() {
        let mut acc = StreamAccumulator::new();
        acc.apply(&StreamChunk::MessageStart {
            model: "m".into(),
            input_tokens: 0,
        });
        acc.apply(&StreamChunk::UsageUpdate {
            input_tokens: Some(1000),
            output_tokens: Some(200),
            cached_input_tokens: Some(800),
        });
        acc.apply(&StreamChunk::MessageDelta {
            stop_reason: Some(StopReason::EndTurn),
            output_tokens: None,
        });
        let r = acc.build().expect("response");
        assert_eq!(r.usage.input_tokens, 1000);
        assert_eq!(r.usage.output_tokens, 200);
        assert_eq!(r.usage.cached_input_tokens, 800);
        // Cached is a subset, so the billable uncached portion is the remainder.
        assert_eq!(r.usage.uncached_input_tokens(), 200);
    }

    #[test]
    fn uncached_input_saturates_rather_than_wrapping() {
        let u = TokenUsage {
            input_tokens: 10,
            output_tokens: 0,
            cached_input_tokens: 99,
        };
        assert_eq!(u.uncached_input_tokens(), 0);
    }

    #[test]
    fn reasoning_effort_serializes_to_the_wire_values() {
        for (e, want) in [
            (ReasoningEffort::None, "none"),
            (ReasoningEffort::Minimal, "minimal"),
            (ReasoningEffort::Low, "low"),
            (ReasoningEffort::Medium, "medium"),
            (ReasoningEffort::High, "high"),
            (ReasoningEffort::XHigh, "xhigh"),
            (ReasoningEffort::Max, "max"),
        ] {
            assert_eq!(e.as_str(), want);
            assert_eq!(serde_json::to_value(e).unwrap(), serde_json::json!(want));
        }
    }

    #[test]
    fn effort_is_absent_from_the_wire_when_unset() {
        let req = LLMRequest::new("m", vec![]);
        let v = serde_json::to_value(&req).unwrap();
        assert!(v.get("effort").is_none(), "{v}");

        let req = LLMRequest::new("m", vec![]).with_effort(ReasoningEffort::XHigh);
        let v = serde_json::to_value(&req).unwrap();
        assert_eq!(v["effort"], serde_json::json!("xhigh"));
    }

    // ── tool-call accumulation across odd frame shapes ─────────────────────
    //
    // These are the shapes real OpenAI-compatible backends emit that the
    // strict reading of the protocol does not.

    #[test]
    fn repeating_the_open_frame_does_not_discard_accumulated_arguments() {
        let mut acc = StreamAccumulator::new();
        acc.apply(&StreamChunk::MessageStart {
            model: "m".into(),
            input_tokens: 1,
        });
        // Some backends resend id+name on every frame of the call.
        acc.apply(&StreamChunk::ToolUseStart {
            index: 0,
            id: "c1".into(),
            name: "f".into(),
        });
        acc.apply(&StreamChunk::InputJsonDelta {
            index: 0,
            partial_json: r#"{"a":"#.into(),
        });
        acc.apply(&StreamChunk::ToolUseStart {
            index: 0,
            id: "c1".into(),
            name: "f".into(),
        });
        acc.apply(&StreamChunk::InputJsonDelta {
            index: 0,
            partial_json: "1}".into(),
        });
        acc.apply(&StreamChunk::MessageDelta {
            stop_reason: Some(StopReason::ToolUse),
            output_tokens: Some(2),
        });

        let r = acc.build().expect("response");
        match r.content_blocks.first().expect("a tool block") {
            ContentBlock::ToolUse { name, input, .. } => {
                assert_eq!(name, "f");
                assert_eq!(input, &serde_json::json!({"a": 1}));
            }
            other => panic!("expected a tool use, got {other:?}"),
        }
    }

    #[test]
    fn arguments_arriving_before_the_name_are_not_dropped() {
        let mut acc = StreamAccumulator::new();
        acc.apply(&StreamChunk::MessageStart {
            model: "m".into(),
            input_tokens: 1,
        });
        // id first, arguments next, name only in a later frame.
        acc.apply(&StreamChunk::ToolUseStart {
            index: 0,
            id: "c1".into(),
            name: String::new(),
        });
        acc.apply(&StreamChunk::InputJsonDelta {
            index: 0,
            partial_json: r#"{"a":1}"#.into(),
        });
        acc.apply(&StreamChunk::ToolUseStart {
            index: 0,
            id: String::new(),
            name: "f".into(),
        });
        acc.apply(&StreamChunk::MessageDelta {
            stop_reason: Some(StopReason::ToolUse),
            output_tokens: Some(2),
        });

        let r = acc.build().expect("response");
        match r.content_blocks.first().expect("a tool block") {
            ContentBlock::ToolUse { id, name, input } => {
                assert_eq!(id, "c1");
                assert_eq!(name, "f");
                assert_eq!(input, &serde_json::json!({"a": 1}));
            }
            other => panic!("expected a tool use, got {other:?}"),
        }
    }

    // A delta for an index that was never opened must still accumulate, or the
    // call disappears while `finish_reason` still reports `tool_calls`.
    #[test]
    fn a_delta_for_an_unopened_index_opens_the_block() {
        let mut acc = StreamAccumulator::new();
        acc.apply(&StreamChunk::MessageStart {
            model: "m".into(),
            input_tokens: 1,
        });
        acc.apply(&StreamChunk::InputJsonDelta {
            index: 0,
            partial_json: r#"{"a":1}"#.into(),
        });
        acc.apply(&StreamChunk::MessageDelta {
            stop_reason: Some(StopReason::ToolUse),
            output_tokens: Some(1),
        });

        let r = acc.build().expect("response");
        assert!(
            r.content_blocks
                .iter()
                .any(|b| matches!(b, ContentBlock::ToolUse { .. })),
            "the tool call was dropped: {:?}",
            r.content_blocks
        );
    }
}
