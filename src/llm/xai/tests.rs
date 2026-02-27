#[cfg(test)]
mod tests {
    use crate::llm::sse_provider::SseProviderAdapter;
    use crate::llm::xai::{
        convert_message, parse_xai_response, parse_xai_sse_line, XaiAdapter, XaiResponse,
    };
    use crate::llm::*;

    // ── Helper ──────────────────────────────────────────────────────────

    fn test_config() -> XaiConfig {
        XaiConfig {
            api_key: "xai-test-key".to_string(),
            base_url: "https://api.x.ai".to_string(),
            model: "grok-3-latest".to_string(),
            timeout_secs: 60,
            web_search: false,
            x_search: false,
        }
    }

    fn test_adapter() -> XaiAdapter {
        XaiAdapter {
            config: test_config(),
        }
    }

    fn make_client() -> XaiClient {
        XaiClient::with_config(test_config()).unwrap()
    }

    // ── Config ──────────────────────────────────────────────────────────

    #[test]
    fn config_defaults() {
        let config = test_config();
        assert_eq!(config.base_url, "https://api.x.ai");
        assert_eq!(config.model, "grok-3-latest");
        assert!(!config.web_search);
        assert!(!config.x_search);
    }

    #[test]
    fn config_with_model_overrides() {
        let config = test_config().with_model("grok-4-1");
        assert_eq!(config.model, "grok-4-1");
    }

    #[test]
    fn config_with_base_url_overrides() {
        let config = test_config().with_base_url("http://localhost:8080");
        assert_eq!(config.base_url, "http://localhost:8080");
    }

    #[test]
    fn config_with_search_tools() {
        let config = test_config().with_web_search().with_x_search();
        assert!(config.web_search);
        assert!(config.x_search);
    }

    // ── Client creation ─────────────────────────────────────────────────

    #[test]
    fn client_rejects_empty_key() {
        let mut config = test_config();
        config.api_key = String::new();
        assert!(XaiClient::with_config(config).is_err());
    }

    #[test]
    fn client_creates_with_valid_key() {
        assert!(XaiClient::with_config(test_config()).is_ok());
    }

    #[test]
    fn endpoint_url_correct() {
        let adapter = test_adapter();
        assert_eq!(adapter.endpoint_url(), "https://api.x.ai/v1/responses");
    }

    #[test]
    fn endpoint_url_with_custom_base() {
        let mut adapter = test_adapter();
        adapter.config.base_url = "http://localhost:9000".to_string();
        assert_eq!(adapter.endpoint_url(), "http://localhost:9000/v1/responses");
    }

    #[test]
    fn provider_name_is_xai() {
        assert_eq!(make_client().provider_name(), "xai");
    }

    #[test]
    fn model_id_returns_configured_model() {
        assert_eq!(make_client().model_id(), "grok-3-latest");
    }

    // ── Request building ────────────────────────────────────────────────

    #[test]
    fn build_request_uses_config_model_when_empty() {
        let adapter = test_adapter();
        let req = LLMRequest::new("", vec![Message::user("Hello")]);
        let body = adapter.build_request_body(&req, false);
        assert_eq!(body["model"], "grok-3-latest");
    }

    #[test]
    fn build_request_uses_provided_model() {
        let adapter = test_adapter();
        let req = LLMRequest::new("grok-4-1-fast", vec![Message::user("Hello")]);
        let body = adapter.build_request_body(&req, false);
        assert_eq!(body["model"], "grok-4-1-fast");
    }

    #[test]
    fn build_request_with_system_prompt() {
        let adapter = test_adapter();
        let req = LLMRequest::new("", vec![Message::user("Hi")]).with_system("Be helpful");
        let body = adapter.build_request_body(&req, false);
        assert_eq!(body["instructions"], "Be helpful");
    }

    #[test]
    fn build_request_stream_flag() {
        let adapter = test_adapter();
        let req = LLMRequest::new("", vec![Message::user("Hi")]);

        let non_stream = adapter.build_request_body(&req, false);
        // stream=false is skipped during serialization
        assert!(non_stream.get("stream").is_none() || non_stream["stream"] == false);

        let stream = adapter.build_request_body(&req, true);
        assert_eq!(stream["stream"], true);
    }

    #[test]
    fn build_request_with_function_tools() {
        let adapter = test_adapter();
        let tool = Tool {
            name: "get_weather".to_string(),
            description: "Get the weather".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": { "city": { "type": "string" } }
            }),
        };
        let req = LLMRequest::new("", vec![Message::user("Hi")]).with_tools(vec![tool]);
        let body = adapter.build_request_body(&req, false);

        let tools = body["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["type"], "function");
        assert_eq!(tools[0]["name"], "get_weather");
        assert!(tools[0].get("parameters").is_some());
    }

    #[test]
    fn build_request_with_builtin_tools() {
        let mut config = test_config();
        config.web_search = true;
        config.x_search = true;
        let adapter = XaiAdapter { config };

        let req = LLMRequest::new("", vec![Message::user("Hi")]);
        let body = adapter.build_request_body(&req, false);

        let tools = body["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 2);
        assert_eq!(tools[0]["type"], "web_search");
        assert_eq!(tools[1]["type"], "x_search");
    }

    #[test]
    fn build_request_mixes_builtin_and_function_tools() {
        let mut config = test_config();
        config.web_search = true;
        let adapter = XaiAdapter { config };

        let tool = Tool {
            name: "read_file".to_string(),
            description: "Read a file".to_string(),
            input_schema: serde_json::json!({"type": "object"}),
        };
        let req = LLMRequest::new("", vec![Message::user("Hi")]).with_tools(vec![tool]);
        let body = adapter.build_request_body(&req, false);

        let tools = body["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 2);
        assert_eq!(tools[0]["type"], "web_search");
        assert_eq!(tools[1]["type"], "function");
        assert_eq!(tools[1]["name"], "read_file");
    }

    #[test]
    fn build_request_no_tools_omitted() {
        let adapter = test_adapter();
        let req = LLMRequest::new("", vec![Message::user("Hi")]);
        let body = adapter.build_request_body(&req, false);
        assert!(body.get("tools").is_none());
    }

    // ── Message conversion ──────────────────────────────────────────────

    #[test]
    fn convert_message_user_text() {
        let msg = Message::user("Hello");
        let mut out = Vec::new();
        convert_message(&msg, &mut out);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0]["role"], "user");
        assert_eq!(out[0]["content"], "Hello");
    }

    #[test]
    fn convert_message_assistant_text() {
        let msg = Message::assistant("Hi there");
        let mut out = Vec::new();
        convert_message(&msg, &mut out);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0]["type"], "message");
        assert_eq!(out[0]["role"], "assistant");
        assert_eq!(out[0]["content"][0]["type"], "output_text");
        assert_eq!(out[0]["content"][0]["text"], "Hi there");
    }

    #[test]
    fn convert_message_assistant_with_tool_calls() {
        let blocks = vec![
            ContentBlock::Text {
                text: "Let me check.".to_string(),
            },
            ContentBlock::ToolUse {
                id: "call_123".to_string(),
                name: "get_weather".to_string(),
                input: serde_json::json!({"city": "NYC"}),
            },
        ];
        let msg = Message::assistant_with_blocks(blocks);
        let mut out = Vec::new();
        convert_message(&msg, &mut out);

        assert_eq!(out.len(), 2);
        assert_eq!(out[0]["type"], "message");
        assert_eq!(out[0]["content"][0]["text"], "Let me check.");
        assert_eq!(out[1]["type"], "function_call");
        assert_eq!(out[1]["call_id"], "call_123");
        assert_eq!(out[1]["name"], "get_weather");
    }

    #[test]
    fn convert_message_tool_results() {
        let blocks = vec![ContentBlock::ToolResult {
            tool_use_id: "call_123".to_string(),
            content: "Sunny, 72F".to_string(),
        }];
        let msg = Message::tool_results(blocks);
        let mut out = Vec::new();
        convert_message(&msg, &mut out);

        assert_eq!(out.len(), 1);
        assert_eq!(out[0]["type"], "function_call_output");
        assert_eq!(out[0]["call_id"], "call_123");
        assert_eq!(out[0]["output"], "Sunny, 72F");
    }

    #[test]
    fn convert_message_mixed_user_blocks() {
        let blocks = vec![
            ContentBlock::Text {
                text: "Here are the results:".to_string(),
            },
            ContentBlock::ToolResult {
                tool_use_id: "call_1".to_string(),
                content: "Result 1".to_string(),
            },
            ContentBlock::ToolResult {
                tool_use_id: "call_2".to_string(),
                content: "Result 2".to_string(),
            },
        ];
        let msg = Message::tool_results(blocks);
        let mut out = Vec::new();
        convert_message(&msg, &mut out);

        assert_eq!(out.len(), 3);
        assert_eq!(out[0]["role"], "user");
        assert_eq!(out[0]["content"], "Here are the results:");
        assert_eq!(out[1]["type"], "function_call_output");
        assert_eq!(out[1]["call_id"], "call_1");
        assert_eq!(out[2]["type"], "function_call_output");
        assert_eq!(out[2]["call_id"], "call_2");
    }

    #[test]
    fn convert_message_user_blocks_with_image() {
        let blocks = vec![
            ContentBlock::Text {
                text: "Describe this sketch:".to_string(),
            },
            ContentBlock::image_png_base64("iVBORw0KGgo=".to_string()),
        ];
        let msg = Message::user_with_blocks(blocks);
        let mut out = Vec::new();
        convert_message(&msg, &mut out);

        assert_eq!(out.len(), 1);
        assert_eq!(out[0]["role"], "user");
        // With images present, content must be a structured array
        let content = out[0]["content"].as_array().unwrap();
        assert_eq!(content.len(), 2);
        assert_eq!(content[0]["type"], "input_text");
        assert_eq!(content[0]["text"], "Describe this sketch:");
        assert_eq!(content[1]["type"], "input_image");
        assert_eq!(
            content[1]["image_url"],
            "data:image/png;base64,iVBORw0KGgo="
        );
        assert_eq!(content[1]["detail"], "high");
    }

    // ── Response parsing ────────────────────────────────────────────────

    #[test]
    fn parse_response_text_only() {
        let api_resp: XaiResponse = serde_json::from_value(serde_json::json!({
            "model": "grok-3-latest",
            "output": [{
                "type": "message",
                "role": "assistant",
                "content": [{"type": "output_text", "text": "Hello!"}]
            }],
            "usage": {"input_tokens": 10, "output_tokens": 5},
            "status": "completed"
        }))
        .unwrap();

        let resp = parse_xai_response(api_resp);
        assert_eq!(resp.content, "Hello!");
        assert_eq!(resp.model, "grok-3-latest");
        assert_eq!(resp.stop_reason, StopReason::EndTurn);
        assert_eq!(resp.usage.input_tokens, 10);
        assert_eq!(resp.usage.output_tokens, 5);
    }

    #[test]
    fn parse_response_with_function_calls() {
        let api_resp: XaiResponse = serde_json::from_value(serde_json::json!({
            "model": "grok-3-latest",
            "output": [
                {
                    "type": "message",
                    "role": "assistant",
                    "content": [{"type": "output_text", "text": "Let me search."}]
                },
                {
                    "type": "function_call",
                    "call_id": "call_abc",
                    "name": "get_weather",
                    "arguments": "{\"city\":\"NYC\"}"
                }
            ],
            "usage": {"input_tokens": 15, "output_tokens": 20},
            "status": "completed"
        }))
        .unwrap();

        let resp = parse_xai_response(api_resp);
        assert_eq!(resp.content, "Let me search.");
        assert_eq!(resp.stop_reason, StopReason::ToolUse);
        assert_eq!(resp.content_blocks.len(), 2);

        match &resp.content_blocks[1] {
            ContentBlock::ToolUse { id, name, input } => {
                assert_eq!(id, "call_abc");
                assert_eq!(name, "get_weather");
                assert_eq!(input["city"], "NYC");
            }
            other => panic!("Expected ToolUse, got {:?}", other),
        }
    }

    #[test]
    fn parse_response_function_call_only() {
        let api_resp: XaiResponse = serde_json::from_value(serde_json::json!({
            "model": "grok-3-latest",
            "output": [{
                "type": "function_call",
                "call_id": "call_xyz",
                "name": "read_file",
                "arguments": "{\"path\":\"src/main.rs\"}"
            }],
            "usage": {"input_tokens": 10, "output_tokens": 8},
            "status": "completed"
        }))
        .unwrap();

        let resp = parse_xai_response(api_resp);
        assert_eq!(resp.content, "");
        assert_eq!(resp.stop_reason, StopReason::ToolUse);
        assert_eq!(resp.content_blocks.len(), 1);
    }

    #[test]
    fn parse_response_incomplete_status() {
        let api_resp: XaiResponse = serde_json::from_value(serde_json::json!({
            "model": "grok-3-latest",
            "output": [{
                "type": "message",
                "role": "assistant",
                "content": [{"type": "output_text", "text": "Truncated..."}]
            }],
            "usage": {"input_tokens": 10, "output_tokens": 100},
            "status": "incomplete"
        }))
        .unwrap();

        let resp = parse_xai_response(api_resp);
        assert_eq!(resp.stop_reason, StopReason::MaxTokens);
    }

    #[test]
    fn parse_response_ignores_unknown_output_types() {
        let api_resp: XaiResponse = serde_json::from_value(serde_json::json!({
            "model": "grok-3-latest",
            "output": [
                {"type": "web_search_call", "id": "ws_1", "status": "completed"},
                {
                    "type": "message",
                    "role": "assistant",
                    "content": [{"type": "output_text", "text": "Found it!"}]
                }
            ],
            "usage": {"input_tokens": 50, "output_tokens": 10},
            "status": "completed"
        }))
        .unwrap();

        let resp = parse_xai_response(api_resp);
        assert_eq!(resp.content, "Found it!");
        assert_eq!(resp.content_blocks.len(), 1);
        assert_eq!(resp.stop_reason, StopReason::EndTurn);
    }

    // ── SSE parsing ─────────────────────────────────────────────────────

    #[test]
    fn parse_sse_text_delta() {
        let line = r#"data: {"type":"response.output_text.delta","output_index":0,"content_index":0,"delta":"Hello"}"#;
        let result = parse_xai_sse_line(line);
        assert!(result.is_some());
        match result.unwrap().unwrap() {
            StreamChunk::ContentDelta { text, index } => {
                assert_eq!(text, "Hello");
                assert_eq!(index, 0);
            }
            other => panic!("Expected ContentDelta, got {:?}", other),
        }
    }

    #[test]
    fn parse_sse_function_call_added() {
        let line = r#"data: {"type":"response.output_item.added","output_index":1,"item":{"type":"function_call","call_id":"call_1","name":"search"}}"#;
        let result = parse_xai_sse_line(line);
        assert!(result.is_some());
        match result.unwrap().unwrap() {
            StreamChunk::ToolUseStart { index, id, name } => {
                assert_eq!(index, 1);
                assert_eq!(id, "call_1");
                assert_eq!(name, "search");
            }
            other => panic!("Expected ToolUseStart, got {:?}", other),
        }
    }

    #[test]
    fn parse_sse_function_call_args_delta() {
        let line = r#"data: {"type":"response.function_call_arguments.delta","output_index":1,"delta":"{\"q\":"}"#;
        let result = parse_xai_sse_line(line);
        assert!(result.is_some());
        match result.unwrap().unwrap() {
            StreamChunk::InputJsonDelta {
                index,
                partial_json,
            } => {
                assert_eq!(index, 1);
                assert_eq!(partial_json, r#"{"q":"#);
            }
            other => panic!("Expected InputJsonDelta, got {:?}", other),
        }
    }

    #[test]
    fn parse_sse_function_call_args_done() {
        let line = r#"data: {"type":"response.function_call_arguments.done","output_index":1,"arguments":"{\"q\":\"test\"}"}"#;
        let result = parse_xai_sse_line(line);
        assert!(result.is_some());
        match result.unwrap().unwrap() {
            StreamChunk::ContentBlockStop { index } => assert_eq!(index, 1),
            other => panic!("Expected ContentBlockStop, got {:?}", other),
        }
    }

    #[test]
    fn parse_sse_response_created() {
        let line = r#"data: {"type":"response.created","response":{"id":"resp_1","model":"grok-3-latest","status":"in_progress"}}"#;
        let result = parse_xai_sse_line(line);
        assert!(result.is_some());
        match result.unwrap().unwrap() {
            StreamChunk::MessageStart {
                model,
                input_tokens,
            } => {
                assert_eq!(model, "grok-3-latest");
                assert_eq!(input_tokens, 0);
            }
            other => panic!("Expected MessageStart, got {:?}", other),
        }
    }

    #[test]
    fn parse_sse_response_completed_text_only() {
        let line = r#"data: {"type":"response.completed","response":{"model":"grok-3","output":[{"type":"message"}],"usage":{"input_tokens":25,"output_tokens":50},"status":"completed"}}"#;
        let result = parse_xai_sse_line(line);
        assert!(result.is_some());
        match result.unwrap().unwrap() {
            StreamChunk::MessageDelta {
                stop_reason,
                output_tokens,
            } => {
                assert_eq!(stop_reason, Some(StopReason::EndTurn));
                assert_eq!(output_tokens, Some(50));
            }
            other => panic!("Expected MessageDelta, got {:?}", other),
        }
    }

    #[test]
    fn parse_sse_response_completed_with_tool_calls() {
        let line = r#"data: {"type":"response.completed","response":{"model":"grok-3","output":[{"type":"function_call","call_id":"c1","name":"f"}],"usage":{"input_tokens":10,"output_tokens":20},"status":"completed"}}"#;
        let result = parse_xai_sse_line(line);
        assert!(result.is_some());
        match result.unwrap().unwrap() {
            StreamChunk::MessageDelta {
                stop_reason,
                output_tokens,
            } => {
                assert_eq!(stop_reason, Some(StopReason::ToolUse));
                assert_eq!(output_tokens, Some(20));
            }
            other => panic!("Expected MessageDelta with ToolUse, got {:?}", other),
        }
    }

    #[test]
    fn parse_sse_response_failed() {
        let line = r#"data: {"type":"response.failed","response":{"error":{"message":"Model overloaded"}}}"#;
        let result = parse_xai_sse_line(line);
        assert!(result.is_some());
        match result.unwrap() {
            Err(LLMError::ApiError { status, message }) => {
                assert_eq!(status, 500);
                assert_eq!(message, "Model overloaded");
            }
            other => panic!("Expected ApiError, got {:?}", other),
        }
    }

    #[test]
    fn parse_sse_response_incomplete() {
        let line = r#"data: {"type":"response.incomplete","response":{"status":"incomplete"}}"#;
        let result = parse_xai_sse_line(line);
        assert!(result.is_some());
        match result.unwrap().unwrap() {
            StreamChunk::MessageDelta { stop_reason, .. } => {
                assert_eq!(stop_reason, Some(StopReason::MaxTokens));
            }
            other => panic!("Expected MessageDelta with MaxTokens, got {:?}", other),
        }
    }

    #[test]
    fn parse_sse_ignores_non_data_lines() {
        assert!(parse_xai_sse_line("event: response.created").is_none());
        assert!(parse_xai_sse_line("").is_none());
        assert!(parse_xai_sse_line(": keepalive").is_none());
    }

    #[test]
    fn parse_sse_ignores_unknown_event_types() {
        let line = r#"data: {"type":"response.in_progress","response":{"status":"in_progress"}}"#;
        assert!(parse_xai_sse_line(line).is_none());
    }

    #[test]
    fn parse_sse_invalid_json_returns_none() {
        assert!(parse_xai_sse_line("data: {invalid json}").is_none());
    }

    #[test]
    fn parse_sse_output_item_added_non_function_call_ignored() {
        let line = r#"data: {"type":"response.output_item.added","output_index":0,"item":{"type":"message","role":"assistant"}}"#;
        assert!(parse_xai_sse_line(line).is_none());
    }

    // ── Error handling ──────────────────────────────────────────────────

    #[test]
    fn handle_error_401_auth() {
        let adapter = test_adapter();
        let body = r#"{"error":{"message":"Invalid API key"}}"#;
        match adapter.handle_error(401, body, None) {
            LLMError::AuthError(msg) => assert_eq!(msg, "Invalid API key"),
            other => panic!("Expected AuthError, got {:?}", other),
        }
    }

    #[test]
    fn handle_error_429_rate_limited() {
        let adapter = test_adapter();
        let body = r#"{"error":{"message":"Rate limit exceeded"}}"#;
        match adapter.handle_error(429, body, None) {
            LLMError::RateLimited { retry_after_ms } => assert_eq!(retry_after_ms, 60000),
            other => panic!("Expected RateLimited, got {:?}", other),
        }
    }

    #[test]
    fn handle_error_429_with_retry_after() {
        let adapter = test_adapter();
        let body = r#"{"error":{"message":"Rate limit exceeded"}}"#;
        match adapter.handle_error(429, body, Some(5000)) {
            LLMError::RateLimited { retry_after_ms } => assert_eq!(retry_after_ms, 5000),
            other => panic!("Expected RateLimited, got {:?}", other),
        }
    }

    #[test]
    fn handle_error_500_api_error() {
        let adapter = test_adapter();
        let body = r#"{"error":{"message":"Internal server error"}}"#;
        match adapter.handle_error(500, body, None) {
            LLMError::ApiError { status, message } => {
                assert_eq!(status, 500);
                assert_eq!(message, "Internal server error");
            }
            other => panic!("Expected ApiError, got {:?}", other),
        }
    }

    #[test]
    fn handle_error_unparseable_body() {
        let adapter = test_adapter();
        match adapter.handle_error(502, "not json", None) {
            LLMError::ApiError { status, message } => {
                assert_eq!(status, 502);
                assert_eq!(message, "not json");
            }
            other => panic!("Expected ApiError, got {:?}", other),
        }
    }

    // ── Pre/post stream events ──────────────────────────────────────────

    #[test]
    fn pre_stream_events_has_content_block_start() {
        let adapter = test_adapter();
        let events = adapter.pre_stream_events();
        assert_eq!(events.len(), 1);
        assert!(matches!(
            events[0],
            StreamChunk::ContentBlockStart { index: 0 }
        ));
    }

    #[test]
    fn post_stream_events_has_stop_and_message_stop() {
        let adapter = test_adapter();
        let events = adapter.post_stream_events();
        assert_eq!(events.len(), 2);
        assert!(matches!(
            events[0],
            StreamChunk::ContentBlockStop { index: 0 }
        ));
        assert!(matches!(events[1], StreamChunk::MessageStop));
    }

    // ── Integration tests (wiremock) ────────────────────────────────────

    #[tokio::test]
    async fn send_message_success() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let response_body = serde_json::json!({
            "model": "grok-3-latest",
            "output": [{
                "type": "message",
                "role": "assistant",
                "content": [{"type": "output_text", "text": "Hello from Grok!"}]
            }],
            "usage": {"input_tokens": 12, "output_tokens": 8},
            "status": "completed"
        });

        Mock::given(method("POST"))
            .and(path("/v1/responses"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
            .mount(&mock_server)
            .await;

        let config = test_config().with_base_url(mock_server.uri());
        let client = XaiClient::with_config(config).unwrap();

        let request = LLMRequest::new("grok-3-latest", vec![Message::user("Hi")]);
        let response = client.send_message(request).await.unwrap();

        assert_eq!(response.content, "Hello from Grok!");
        assert_eq!(response.model, "grok-3-latest");
        assert_eq!(response.stop_reason, StopReason::EndTurn);
        assert_eq!(response.usage.input_tokens, 12);
        assert_eq!(response.usage.output_tokens, 8);
    }

    #[tokio::test]
    async fn send_message_with_function_calls() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let response_body = serde_json::json!({
            "model": "grok-3-latest",
            "output": [{
                "type": "function_call",
                "call_id": "call_xyz",
                "name": "read_file",
                "arguments": "{\"path\":\"src/main.rs\"}"
            }],
            "usage": {"input_tokens": 20, "output_tokens": 15},
            "status": "completed"
        });

        Mock::given(method("POST"))
            .and(path("/v1/responses"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
            .mount(&mock_server)
            .await;

        let config = test_config().with_base_url(mock_server.uri());
        let client = XaiClient::with_config(config).unwrap();

        let request = LLMRequest::new("grok-3-latest", vec![Message::user("Read main.rs")]);
        let response = client.send_message(request).await.unwrap();

        assert_eq!(response.stop_reason, StopReason::ToolUse);
        assert_eq!(response.content_blocks.len(), 1);
        match &response.content_blocks[0] {
            ContentBlock::ToolUse { id, name, input } => {
                assert_eq!(id, "call_xyz");
                assert_eq!(name, "read_file");
                assert_eq!(input["path"], "src/main.rs");
            }
            other => panic!("Expected ToolUse, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn send_message_server_error() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/responses"))
            .respond_with(
                ResponseTemplate::new(500)
                    .set_body_string(r#"{"error":{"message":"Server overloaded"}}"#),
            )
            .mount(&mock_server)
            .await;

        let config = test_config().with_base_url(mock_server.uri());
        let client = XaiClient::with_config(config).unwrap();

        let request = LLMRequest::new("grok-3-latest", vec![Message::user("Hi")]);
        match client.send_message(request).await.unwrap_err() {
            LLMError::ApiError { status, message } => {
                assert_eq!(status, 500);
                assert_eq!(message, "Server overloaded");
            }
            e => panic!("Expected ApiError, got {:?}", e),
        }
    }

    #[tokio::test]
    async fn send_message_auth_error() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/responses"))
            .respond_with(
                ResponseTemplate::new(401)
                    .set_body_string(r#"{"error":{"message":"Invalid API key"}}"#),
            )
            .mount(&mock_server)
            .await;

        let config = test_config().with_base_url(mock_server.uri());
        let client = XaiClient::with_config(config).unwrap();

        let request = LLMRequest::new("grok-3-latest", vec![Message::user("Hi")]);
        match client.send_message(request).await.unwrap_err() {
            LLMError::AuthError(msg) => assert_eq!(msg, "Invalid API key"),
            e => panic!("Expected AuthError, got {:?}", e),
        }
    }

    #[tokio::test]
    async fn send_message_rate_limited_with_retry_after() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/responses"))
            .respond_with(
                ResponseTemplate::new(429)
                    .set_body_string(r#"{"error":{"message":"Rate limited"}}"#)
                    .insert_header("retry-after", "3.0"),
            )
            .mount(&mock_server)
            .await;

        let config = test_config().with_base_url(mock_server.uri());
        let client = XaiClient::with_config(config).unwrap();

        let request = LLMRequest::new("grok-3-latest", vec![Message::user("Hi")]);
        match client.send_message(request).await.unwrap_err() {
            LLMError::RateLimited { retry_after_ms } => assert_eq!(retry_after_ms, 3000),
            e => panic!("Expected RateLimited, got {:?}", e),
        }
    }

    #[tokio::test]
    async fn send_message_invalid_json_response() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/responses"))
            .respond_with(ResponseTemplate::new(200).set_body_string("not json"))
            .mount(&mock_server)
            .await;

        let config = test_config().with_base_url(mock_server.uri());
        let client = XaiClient::with_config(config).unwrap();

        let request = LLMRequest::new("grok-3-latest", vec![Message::user("Hi")]);
        assert!(matches!(
            client.send_message(request).await.unwrap_err(),
            LLMError::ParseError(_)
        ));
    }

    #[tokio::test]
    async fn send_message_request_includes_tools() {
        use wiremock::matchers::{body_partial_json, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let response_body = serde_json::json!({
            "model": "grok-3-latest",
            "output": [{"type": "message", "role": "assistant", "content": [{"type": "output_text", "text": "ok"}]}],
            "usage": {"input_tokens": 5, "output_tokens": 1},
            "status": "completed"
        });

        Mock::given(method("POST"))
            .and(path("/v1/responses"))
            .and(body_partial_json(serde_json::json!({
                "tools": [{"type": "web_search"}]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
            .mount(&mock_server)
            .await;

        let config = test_config()
            .with_base_url(mock_server.uri())
            .with_web_search();
        let client = XaiClient::with_config(config).unwrap();

        let request = LLMRequest::new("grok-3-latest", vec![Message::user("Search for Rust")]);
        let response = client.send_message(request).await.unwrap();
        assert_eq!(response.content, "ok");
    }
}
