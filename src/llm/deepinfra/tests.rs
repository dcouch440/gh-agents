#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::llm::provider::LLMProvider;
    use crate::llm::types::{ContentBlock, ImageSource, Message, StopReason};
    use serde_json::json;

    fn adapter() -> DeepInfraAdapter {
        DeepInfraAdapter {
            config: DeepInfraConfig {
                api_key: "k".into(),
                base_url: "https://example.invalid/v1/openai".into(),
                model: crate::constants::MODEL_DEEPSEEK_V4_FLASH.into(),
                timeout_secs: 30,
                read_timeout_secs: 10,
                default_effort: None,
            },
        }
    }

    // ── config / wiring ────────────────────────────────────────────────────

    #[test]
    fn endpoint_is_chat_completions() {
        assert_eq!(
            adapter().endpoint_url(),
            "https://example.invalid/v1/openai/chat/completions"
        );
    }

    #[test]
    fn auth_header_is_a_bearer_token() {
        let h = adapter().default_headers().unwrap();
        assert_eq!(h.get("authorization").unwrap(), "Bearer k");
    }

    #[test]
    fn empty_api_key_is_rejected() {
        let cfg = DeepInfraConfig {
            api_key: String::new(),
            ..adapter().config
        };
        assert!(matches!(
            DeepInfraClient::with_config(cfg),
            Err(LLMError::AuthError(_))
        ));
    }

    #[test]
    fn a_read_timeout_is_always_set() {
        // DeepInfra queues requests, so the whole-request timeout is long;
        // without a read timeout a stalled stream would hang for all of it.
        assert_eq!(adapter().read_timeout_secs(), Some(10));
    }

    // ── request body ───────────────────────────────────────────────────────

    #[test]
    fn system_prompt_becomes_the_first_message() {
        let req = LLMRequest::new("m", vec![Message::user("hi")]).with_system("be terse");
        let body = adapter().build_request_body(&req, false);
        assert_eq!(body["messages"][0]["role"], "system");
        assert_eq!(body["messages"][0]["content"], "be terse");
        assert_eq!(body["messages"][1]["role"], "user");
    }

    #[test]
    fn effort_is_sent_when_set_and_omitted_when_not() {
        let a = adapter();
        let req = LLMRequest::new("m", vec![Message::user("hi")]);
        assert!(a
            .build_request_body(&req, false)
            .get("reasoning_effort")
            .is_none());

        let req = req.with_effort(ReasoningEffort::XHigh);
        assert_eq!(
            a.build_request_body(&req, false)["reasoning_effort"],
            "xhigh"
        );
    }

    #[test]
    fn request_effort_overrides_the_config_default() {
        let mut a = adapter();
        a.config.default_effort = Some(ReasoningEffort::None);
        let req = LLMRequest::new("m", vec![]).with_effort(ReasoningEffort::Max);
        assert_eq!(a.build_request_body(&req, false)["reasoning_effort"], "max");
    }

    #[test]
    fn config_default_effort_applies_when_the_request_has_none() {
        let mut a = adapter();
        a.config.default_effort = Some(ReasoningEffort::High);
        let req = LLMRequest::new("m", vec![]);
        assert_eq!(
            a.build_request_body(&req, false)["reasoning_effort"],
            "high"
        );
    }

    // Without include_usage an OpenAI-compatible stream reports no usage at
    // all, and every streamed call would bill as zero tokens.
    #[test]
    fn streaming_requests_ask_for_usage() {
        let req = LLMRequest::new("m", vec![]);
        let body = adapter().build_request_body(&req, true);
        assert_eq!(body["stream"], true);
        assert_eq!(body["stream_options"]["include_usage"], true);

        let body = adapter().build_request_body(&req, false);
        assert_eq!(body["stream"], false);
        assert!(body.get("stream_options").is_none());
    }

    #[test]
    fn an_empty_request_model_falls_back_to_the_configured_one() {
        let mut req = LLMRequest::new("", vec![]);
        req.model = String::new();
        let body = adapter().build_request_body(&req, false);
        assert_eq!(body["model"], crate::constants::MODEL_DEEPSEEK_V4_FLASH);
    }

    #[test]
    fn tools_use_the_nested_function_shape() {
        let req = LLMRequest::new("m", vec![]).with_tools(vec![Tool {
            name: "brave_search".into(),
            description: "search".into(),
            input_schema: json!({"type": "object"}),
        }]);
        let body = adapter().build_request_body(&req, false);
        assert_eq!(body["tools"][0]["type"], "function");
        assert_eq!(body["tools"][0]["function"]["name"], "brave_search");
        assert_eq!(body["tools"][0]["function"]["parameters"]["type"], "object");
    }

    #[test]
    fn tools_are_omitted_entirely_when_empty() {
        let req = LLMRequest::new("m", vec![]);
        assert!(adapter()
            .build_request_body(&req, false)
            .get("tools")
            .is_none());
    }

    #[test]
    fn assistant_tool_calls_serialize_arguments_as_a_string() {
        let msg = Message::assistant_with_blocks(vec![ContentBlock::ToolUse {
            id: "call_1".into(),
            name: "read_file".into(),
            input: json!({"path": "a.rs"}),
        }]);
        let req = LLMRequest::new("m", vec![msg]);
        let body = adapter().build_request_body(&req, false);
        let call = &body["messages"][0]["tool_calls"][0];
        assert_eq!(call["id"], "call_1");
        assert_eq!(call["type"], "function");
        assert_eq!(call["function"]["name"], "read_file");
        // A string, not an object — this is the OpenAI contract.
        assert_eq!(call["function"]["arguments"], r#"{"path":"a.rs"}"#);
    }

    // Tool results arrive inside a *user* message here, but OpenAI wants each
    // as its own `role: "tool"` message.
    #[test]
    fn tool_results_become_separate_tool_role_messages() {
        let msg = Message::tool_results(vec![
            ContentBlock::ToolResult {
                tool_use_id: "call_1".into(),
                content: "ok".into(),
            },
            ContentBlock::ToolResult {
                tool_use_id: "call_2".into(),
                content: "also ok".into(),
            },
        ]);
        let req = LLMRequest::new("m", vec![msg]);
        let body = adapter().build_request_body(&req, false);
        let msgs = body["messages"].as_array().unwrap();
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0]["role"], "tool");
        assert_eq!(msgs[0]["tool_call_id"], "call_1");
        assert_eq!(msgs[0]["content"], "ok");
        assert_eq!(msgs[1]["tool_call_id"], "call_2");
    }

    #[test]
    fn images_become_data_uri_image_url_parts() {
        let msg = Message::user_with_blocks(vec![
            ContentBlock::Text {
                text: "what is this".into(),
            },
            ContentBlock::Image {
                source: ImageSource {
                    source_type: "base64".into(),
                    media_type: "image/png".into(),
                    data: "AAAA".into(),
                },
            },
        ]);
        let req = LLMRequest::new("m", vec![msg]);
        let body = adapter().build_request_body(&req, false);
        let parts = body["messages"][0]["content"].as_array().unwrap();
        assert_eq!(parts[0]["type"], "text");
        assert_eq!(parts[1]["type"], "image_url");
        assert_eq!(parts[1]["image_url"]["url"], "data:image/png;base64,AAAA");
    }

    #[test]
    fn a_single_text_block_collapses_to_a_bare_string() {
        let msg = Message::user_with_blocks(vec![ContentBlock::Text {
            text: "hello".into(),
        }]);
        let req = LLMRequest::new("m", vec![msg]);
        let body = adapter().build_request_body(&req, false);
        assert_eq!(body["messages"][0]["content"], "hello");
    }

    // ── non-streaming response ─────────────────────────────────────────────

    #[test]
    fn parse_response_reads_text_and_usage_including_cached() {
        let body = json!({
            "model": "deepseek-ai/DeepSeek-V4-Flash-0731",
            "choices": [{"message": {"content": "hello"}, "finish_reason": "stop"}],
            "usage": {
                "prompt_tokens": 1000,
                "completion_tokens": 50,
                "prompt_tokens_details": {"cached_tokens": 800}
            }
        })
        .to_string();
        let r = adapter().parse_response(body.as_bytes()).unwrap();
        assert_eq!(r.content, "hello");
        assert_eq!(r.stop_reason, StopReason::EndTurn);
        assert_eq!(r.usage.input_tokens, 1000);
        assert_eq!(r.usage.cached_input_tokens, 800);
        // Cached is a subset of prompt_tokens, so billable input is 200.
        assert_eq!(r.usage.uncached_input_tokens(), 200);
    }

    #[test]
    fn missing_cached_details_mean_zero_not_an_error() {
        let body = json!({
            "choices": [{"message": {"content": "x"}, "finish_reason": "stop"}],
            "usage": {"prompt_tokens": 10, "completion_tokens": 2}
        })
        .to_string();
        let r = adapter().parse_response(body.as_bytes()).unwrap();
        assert_eq!(r.usage.cached_input_tokens, 0);
        assert_eq!(r.usage.uncached_input_tokens(), 10);
    }

    #[test]
    fn parse_response_reads_tool_calls_and_forces_tool_use() {
        let body = json!({
            "choices": [{
                "message": {
                    "content": null,
                    "tool_calls": [{
                        "id": "call_1",
                        "type": "function",
                        "function": {"name": "read_file", "arguments": "{\"path\":\"a.rs\"}"}
                    }]
                },
                "finish_reason": "tool_calls"
            }]
        })
        .to_string();
        let r = adapter().parse_response(body.as_bytes()).unwrap();
        assert_eq!(r.stop_reason, StopReason::ToolUse);
        match &r.content_blocks[0] {
            ContentBlock::ToolUse { id, name, input } => {
                assert_eq!(id, "call_1");
                assert_eq!(name, "read_file");
                assert_eq!(input["path"], "a.rs");
            }
            other => panic!("expected tool use, got {other:?}"),
        }
    }

    #[test]
    fn malformed_tool_arguments_do_not_fail_the_whole_response() {
        let body = json!({
            "choices": [{
                "message": {
                    "tool_calls": [{
                        "id": "c", "type": "function",
                        "function": {"name": "f", "arguments": "{oops"}
                    }]
                },
                "finish_reason": "tool_calls"
            }]
        })
        .to_string();
        let r = adapter().parse_response(body.as_bytes()).unwrap();
        match &r.content_blocks[0] {
            ContentBlock::ToolUse { input, .. } => assert_eq!(input, &json!({})),
            other => panic!("expected tool use, got {other:?}"),
        }
    }

    #[test]
    fn a_response_with_no_choices_is_a_parse_error() {
        let body = json!({"choices": []}).to_string();
        assert!(matches!(
            adapter().parse_response(body.as_bytes()),
            Err(LLMError::ParseError(_))
        ));
    }

    #[test]
    fn finish_reason_length_maps_to_max_tokens() {
        let body = json!({
            "choices": [{"message": {"content": "x"}, "finish_reason": "length"}]
        })
        .to_string();
        let r = adapter().parse_response(body.as_bytes()).unwrap();
        assert_eq!(r.stop_reason, StopReason::MaxTokens);
    }

    // ── streaming ──────────────────────────────────────────────────────────

    fn events(line: &str) -> Vec<StreamChunk> {
        parse_openai_sse_line(line)
            .into_iter()
            .map(|r| r.unwrap())
            .collect()
    }

    #[test]
    fn done_sentinel_ends_the_stream() {
        assert_eq!(events("data: [DONE]"), vec![StreamChunk::MessageStop]);
    }

    #[test]
    fn non_data_and_malformed_lines_are_skipped_not_fatal() {
        assert!(events(": keepalive").is_empty());
        assert!(events("event: whatever").is_empty());
        assert!(events("data: {not json").is_empty());
        assert!(events("data:").is_empty());
    }

    #[test]
    fn content_deltas_become_content_chunks() {
        let e = events(r#"data: {"choices":[{"delta":{"content":"hel"}}]}"#);
        assert_eq!(
            e,
            vec![StreamChunk::ContentDelta {
                text: "hel".into(),
                index: 0
            }]
        );
    }

    // Chain of thought must not be concatenated into the answer.
    #[test]
    fn reasoning_content_is_not_surfaced_as_output() {
        let e = events(r#"data: {"choices":[{"delta":{"reasoning_content":"thinking..."}}]}"#);
        assert!(
            e.is_empty(),
            "reasoning must not reach the response, got {e:?}"
        );
    }

    #[test]
    fn a_tool_call_opening_emits_start_then_arguments() {
        let e = events(
            r#"data: {"choices":[{"delta":{"tool_calls":[{"index":0,"id":"c1","function":{"name":"f","arguments":"{\"a\""}}]}}]}"#,
        );
        assert_eq!(
            e,
            vec![
                StreamChunk::ToolUseStart {
                    index: 0,
                    id: "c1".into(),
                    name: "f".into()
                },
                StreamChunk::InputJsonDelta {
                    index: 0,
                    partial_json: r#"{"a""#.into()
                },
            ]
        );
    }

    #[test]
    fn argument_only_frames_carry_the_index_forward() {
        let e = events(
            r#"data: {"choices":[{"delta":{"tool_calls":[{"index":1,"function":{"arguments":":2}"}}]}}]}"#,
        );
        assert_eq!(
            e,
            vec![StreamChunk::InputJsonDelta {
                index: 1,
                partial_json: ":2}".into()
            }]
        );
    }

    #[test]
    fn usage_frames_carry_cached_tokens() {
        let e = events(
            r#"data: {"choices":[],"usage":{"prompt_tokens":900,"completion_tokens":10,"prompt_tokens_details":{"cached_tokens":700}}}"#,
        );
        assert_eq!(
            e,
            vec![StreamChunk::UsageUpdate {
                input_tokens: Some(900),
                output_tokens: Some(10),
                cached_input_tokens: Some(700)
            }]
        );
    }

    // The end-to-end shape: two interleaved tool calls must survive the whole
    // adapter → accumulator path with their own arguments.
    #[test]
    fn a_full_two_tool_stream_reassembles_correctly() {
        use crate::llm::types::StreamAccumulator;
        let a = adapter();
        let mut acc = StreamAccumulator::new();
        for chunk in a.pre_stream_events() {
            acc.apply(&chunk);
        }
        let lines = [
            r#"data: {"choices":[{"delta":{"tool_calls":[{"index":0,"id":"c0","function":{"name":"alpha","arguments":""}}]}}]}"#,
            r#"data: {"choices":[{"delta":{"tool_calls":[{"index":1,"id":"c1","function":{"name":"beta","arguments":""}}]}}]}"#,
            r#"data: {"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"{\"x\":1}"}}]}}]}"#,
            r#"data: {"choices":[{"delta":{"tool_calls":[{"index":1,"function":{"arguments":"{\"y\":2}"}}]}}]}"#,
            r#"data: {"choices":[{"delta":{},"finish_reason":"tool_calls"}]}"#,
            r#"data: {"choices":[],"usage":{"prompt_tokens":50,"completion_tokens":8,"prompt_tokens_details":{"cached_tokens":40}}}"#,
            "data: [DONE]",
        ];
        for l in lines {
            for chunk in a.parse_sse_events(l) {
                acc.apply(&chunk.unwrap());
            }
        }
        let r = acc.build().expect("response");
        let calls: Vec<_> = r
            .content_blocks
            .iter()
            .filter_map(|b| match b {
                ContentBlock::ToolUse { name, input, .. } => Some((name.clone(), input.clone())),
                _ => None,
            })
            .collect();
        assert_eq!(calls.len(), 2, "both calls must survive: {calls:?}");
        assert_eq!(calls[0], ("alpha".to_string(), json!({"x": 1})));
        assert_eq!(calls[1], ("beta".to_string(), json!({"y": 2})));
        assert_eq!(r.stop_reason, StopReason::ToolUse);
        assert_eq!(r.usage.cached_input_tokens, 40);
        assert_eq!(r.usage.uncached_input_tokens(), 10);
    }

    // ── errors ─────────────────────────────────────────────────────────────

    #[test]
    fn a_429_without_retry_after_still_backs_off() {
        let e = adapter().handle_error(429, "{}", None);
        assert!(matches!(
            e,
            LLMError::RateLimited {
                retry_after_ms: 60_000
            }
        ));
    }

    #[test]
    fn rate_limits_surface_retry_after() {
        let e = adapter().handle_error(429, "{}", Some(2500));
        assert!(matches!(
            e,
            LLMError::RateLimited {
                retry_after_ms: 2500
            }
        ));
    }

    #[test]
    fn auth_failures_map_to_auth_error() {
        let e = adapter().handle_error(401, r#"{"error":{"message":"bad key"}}"#, None);
        match e {
            LLMError::AuthError(m) => assert!(m.contains("bad key"), "{m}"),
            other => panic!("expected AuthError, got {other:?}"),
        }
    }

    #[test]
    fn other_statuses_keep_the_code_and_message() {
        let e = adapter().handle_error(503, r#"{"detail":"overloaded"}"#, None);
        match e {
            LLMError::ApiError { status, message } => {
                assert_eq!(status, 503);
                assert!(message.contains("overloaded"), "{message}");
            }
            other => panic!("expected ApiError, got {other:?}"),
        }
    }

    #[test]
    fn a_non_json_error_body_is_still_reported() {
        let e = adapter().handle_error(500, "<html>gateway</html>", None);
        match e {
            LLMError::ApiError { message, .. } => assert!(message.contains("gateway"), "{message}"),
            other => panic!("expected ApiError, got {other:?}"),
        }
    }

    // ── integration ────────────────────────────────────────────────────────

    #[tokio::test]
    async fn send_message_round_trips_against_a_mock_server() {
        use wiremock::matchers::{header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .and(header("authorization", "Bearer k"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "model": "deepseek-ai/DeepSeek-V4-Flash-0731",
                "choices": [{"message": {"content": "pong"}, "finish_reason": "stop"}],
                "usage": {"prompt_tokens": 5, "completion_tokens": 1,
                          "prompt_tokens_details": {"cached_tokens": 4}}
            })))
            .mount(&server)
            .await;

        let client = DeepInfraClient::with_config(DeepInfraConfig {
            api_key: "k".into(),
            base_url: server.uri(),
            model: crate::constants::MODEL_DEEPSEEK_V4_FLASH.into(),
            timeout_secs: 10,
            read_timeout_secs: 5,
            default_effort: None,
        })
        .unwrap();

        let r = client
            .send_message(LLMRequest::new("", vec![Message::user("ping")]))
            .await
            .unwrap();
        assert_eq!(r.content, "pong");
        assert_eq!(r.usage.cached_input_tokens, 4);
    }

    #[tokio::test]
    async fn a_429_from_the_server_becomes_rate_limited() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/chat/completions"))
            .respond_with(
                ResponseTemplate::new(429)
                    .insert_header("retry-after", "3")
                    .set_body_string("{}"),
            )
            .mount(&server)
            .await;

        let client = DeepInfraClient::with_config(DeepInfraConfig {
            api_key: "k".into(),
            base_url: server.uri(),
            model: "m".into(),
            timeout_secs: 10,
            read_timeout_secs: 5,
            default_effort: None,
        })
        .unwrap();

        let err = client
            .send_message(LLMRequest::new("", vec![Message::user("x")]))
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            LLMError::RateLimited {
                retry_after_ms: 3000
            }
        ));
    }
}
