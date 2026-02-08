//! Tests for Anthropic provider

use super::*;

#[test]
fn config_from_explicit_key() {
    let config = AnthropicConfig::new("test-key").with_model("claude-3-haiku");

    assert_eq!(config.api_key, "test-key");
    assert_eq!(config.model, "claude-3-haiku");
}

#[test]
fn client_rejects_empty_key() {
    let config = AnthropicConfig::new("");
    let result = AnthropicClient::new(config);
    assert!(result.is_err());
}

#[test]
fn client_creates_with_valid_key() {
    let config = AnthropicConfig::new("sk-ant-test-key");
    let result = AnthropicClient::new(config);
    assert!(result.is_ok());
}

#[test]
fn messages_url_correct() {
    let config = AnthropicConfig::new("test-key");
    let client = AnthropicClient::new(config).unwrap();
    assert_eq!(
        client.messages_url(),
        "https://api.anthropic.com/v1/messages"
    );
}

#[test]
fn messages_url_with_custom_base() {
    let config = AnthropicConfig::new("test-key").with_base_url("https://custom.api.com");
    let client = AnthropicClient::new(config).unwrap();
    assert_eq!(client.messages_url(), "https://custom.api.com/v1/messages");
}

#[test]
fn parse_stop_reasons() {
    assert_eq!(
        AnthropicClient::parse_stop_reason("end_turn"),
        StopReason::EndTurn
    );
    assert_eq!(
        AnthropicClient::parse_stop_reason("max_tokens"),
        StopReason::MaxTokens
    );
    assert_eq!(
        AnthropicClient::parse_stop_reason("stop_sequence"),
        StopReason::StopSequence
    );
    assert_eq!(
        AnthropicClient::parse_stop_reason("tool_use"),
        StopReason::ToolUse
    );
    assert_eq!(
        AnthropicClient::parse_stop_reason("unknown"),
        StopReason::EndTurn
    );
}

#[test]
fn build_request_uses_config_model_when_empty() {
    let config = AnthropicConfig::new("test-key").with_model("claude-3-opus");
    let client = AnthropicClient::new(config).unwrap();

    let llm_request = LLMRequest::new("", vec![Message::user("Hello")]);
    let api_request = client.build_request(&llm_request);

    assert_eq!(api_request.model, "claude-3-opus");
}

#[test]
fn build_request_uses_provided_model() {
    let config = AnthropicConfig::new("test-key").with_model("default-model");
    let client = AnthropicClient::new(config).unwrap();

    let llm_request = LLMRequest::new("claude-3-haiku", vec![Message::user("Hello")]);
    let api_request = client.build_request(&llm_request);

    assert_eq!(api_request.model, "claude-3-haiku");
}

#[test]
fn parse_sse_message_start() {
    let line = r#"data: {"type":"message_start","message":{"model":"claude-3","usage":{"input_tokens":10,"output_tokens":0}}}"#;
    let result = AnthropicClient::parse_sse_line(line);

    assert!(result.is_some());
    let chunk = result.unwrap().unwrap();
    match chunk {
        StreamChunk::MessageStart {
            model,
            input_tokens,
        } => {
            assert_eq!(model, "claude-3");
            assert_eq!(input_tokens, 10);
        }
        _ => panic!("Expected MessageStart chunk"),
    }
}

#[test]
fn parse_sse_content_delta() {
    let line = r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}"#;
    let result = AnthropicClient::parse_sse_line(line);

    assert!(result.is_some());
    let chunk = result.unwrap().unwrap();
    match chunk {
        StreamChunk::ContentDelta { text, index } => {
            assert_eq!(text, "Hello");
            assert_eq!(index, 0);
        }
        _ => panic!("Expected ContentDelta chunk"),
    }
}

#[test]
fn parse_sse_message_stop() {
    let line = r#"data: {"type":"message_stop"}"#;
    let result = AnthropicClient::parse_sse_line(line);

    assert!(result.is_some());
    let chunk = result.unwrap().unwrap();
    assert_eq!(chunk, StreamChunk::MessageStop);
}

#[test]
fn parse_sse_done_marker() {
    let line = "data: [DONE]";
    let result = AnthropicClient::parse_sse_line(line);

    assert!(result.is_some());
    let chunk = result.unwrap().unwrap();
    assert_eq!(chunk, StreamChunk::MessageStop);
}

#[test]
fn parse_sse_ignores_non_data_lines() {
    let line = "event: message_start";
    let result = AnthropicClient::parse_sse_line(line);
    assert!(result.is_none());

    let empty_line = "";
    let result = AnthropicClient::parse_sse_line(empty_line);
    assert!(result.is_none());
}

#[test]
fn handle_error_401_auth_error() {
    let body =
        r#"{"type":"error","error":{"type":"authentication_error","message":"Invalid API key"}}"#;
    let error = AnthropicClient::handle_error_response(401, body, None);
    match error {
        LLMError::AuthError(msg) => assert_eq!(msg, "Invalid API key"),
        _ => panic!("Expected AuthError, got {:?}", error),
    }
}

#[test]
fn handle_error_429_rate_limited() {
    let body =
        r#"{"type":"error","error":{"type":"rate_limit_error","message":"Too many requests"}}"#;
    let error = AnthropicClient::handle_error_response(429, body, None);
    match error {
        LLMError::RateLimited { retry_after_ms } => assert_eq!(retry_after_ms, 60000),
        _ => panic!("Expected RateLimited, got {:?}", error),
    }
}

#[test]
fn handle_error_429_with_retry_after() {
    let body =
        r#"{"type":"error","error":{"type":"rate_limit_error","message":"Too many requests"}}"#;
    let error = AnthropicClient::handle_error_response(429, body, Some(30000));
    match error {
        LLMError::RateLimited { retry_after_ms } => assert_eq!(retry_after_ms, 30000),
        _ => panic!("Expected RateLimited, got {:?}", error),
    }
}

#[test]
fn handle_error_500_api_error() {
    let body = r#"{"type":"error","error":{"type":"server_error","message":"Internal error"}}"#;
    let error = AnthropicClient::handle_error_response(500, body, None);
    match error {
        LLMError::ApiError { status, message } => {
            assert_eq!(status, 500);
            assert_eq!(message, "Internal error");
        }
        _ => panic!("Expected ApiError, got {:?}", error),
    }
}

#[test]
fn handle_error_unparseable_body() {
    let error = AnthropicClient::handle_error_response(502, "not json at all", None);
    match error {
        LLMError::ApiError { status, message } => {
            assert_eq!(status, 502);
            assert_eq!(message, "not json at all");
        }
        _ => panic!("Expected ApiError, got {:?}", error),
    }
}

#[test]
fn handle_error_empty_body() {
    let error = AnthropicClient::handle_error_response(500, "", None);
    match error {
        LLMError::ApiError { status, message } => {
            assert_eq!(status, 500);
            assert_eq!(message, "");
        }
        _ => panic!("Expected ApiError, got {:?}", error),
    }
}

#[test]
fn parse_sse_content_block_start() {
    let line = r#"data: {"type":"content_block_start","index":0}"#;
    let result = AnthropicClient::parse_sse_line(line);
    assert!(result.is_some());
    let chunk = result.unwrap().unwrap();
    match chunk {
        StreamChunk::ContentBlockStart { index } => assert_eq!(index, 0),
        _ => panic!("Expected ContentBlockStart"),
    }
}

#[test]
fn parse_sse_content_block_stop() {
    let line = r#"data: {"type":"content_block_stop","index":1}"#;
    let result = AnthropicClient::parse_sse_line(line);
    assert!(result.is_some());
    let chunk = result.unwrap().unwrap();
    match chunk {
        StreamChunk::ContentBlockStop { index } => assert_eq!(index, 1),
        _ => panic!("Expected ContentBlockStop"),
    }
}

#[test]
fn parse_sse_message_delta_with_stop_reason() {
    let line = r#"data: {"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":42}}"#;
    let result = AnthropicClient::parse_sse_line(line);
    assert!(result.is_some());
    let chunk = result.unwrap().unwrap();
    match chunk {
        StreamChunk::MessageDelta {
            stop_reason,
            output_tokens,
        } => {
            assert_eq!(stop_reason, Some(StopReason::EndTurn));
            assert_eq!(output_tokens, Some(42));
        }
        _ => panic!("Expected MessageDelta"),
    }
}

#[test]
fn parse_sse_message_delta_no_usage() {
    let line = r#"data: {"type":"message_delta","delta":{"stop_reason":"max_tokens"}}"#;
    let result = AnthropicClient::parse_sse_line(line);
    assert!(result.is_some());
    let chunk = result.unwrap().unwrap();
    match chunk {
        StreamChunk::MessageDelta {
            stop_reason,
            output_tokens,
        } => {
            assert_eq!(stop_reason, Some(StopReason::MaxTokens));
            assert_eq!(output_tokens, None);
        }
        _ => panic!("Expected MessageDelta"),
    }
}

#[test]
fn parse_sse_message_delta_no_stop_reason() {
    let line = r#"data: {"type":"message_delta","delta":{},"usage":{"output_tokens":10}}"#;
    let result = AnthropicClient::parse_sse_line(line);
    assert!(result.is_some());
    let chunk = result.unwrap().unwrap();
    match chunk {
        StreamChunk::MessageDelta {
            stop_reason,
            output_tokens,
        } => {
            assert_eq!(stop_reason, None);
            assert_eq!(output_tokens, Some(10));
        }
        _ => panic!("Expected MessageDelta"),
    }
}

#[test]
fn parse_sse_ping() {
    let line = r#"data: {"type":"ping"}"#;
    let result = AnthropicClient::parse_sse_line(line);
    assert!(result.is_some());
    let chunk = result.unwrap().unwrap();
    assert_eq!(chunk, StreamChunk::Ping);
}

#[test]
fn parse_sse_error_event() {
    let line =
        r#"data: {"type":"error","error":{"type":"overloaded_error","message":"Overloaded"}}"#;
    let result = AnthropicClient::parse_sse_line(line);
    assert!(result.is_some());
    let err = result.unwrap().unwrap_err();
    match err {
        LLMError::ApiError { status, message } => {
            assert_eq!(status, 500);
            assert_eq!(message, "Overloaded");
        }
        _ => panic!("Expected ApiError from SSE error event"),
    }
}

#[test]
fn parse_sse_invalid_json_returns_none() {
    let line = "data: {invalid json}";
    let result = AnthropicClient::parse_sse_line(line);
    assert!(result.is_none());
}

#[test]
fn parse_sse_content_delta_no_text_returns_none() {
    let line =
        r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta"}}"#;
    let result = AnthropicClient::parse_sse_line(line);
    assert!(result.is_none());
}

#[test]
fn config_with_base_url() {
    let config = AnthropicConfig::new("key").with_base_url("http://localhost:8080");
    assert_eq!(config.base_url, "http://localhost:8080");
}

#[test]
fn config_defaults() {
    let config = AnthropicConfig::new("key");
    assert_eq!(config.base_url, "https://api.anthropic.com");
    assert_eq!(config.model, "claude-sonnet-4-20250514");
    assert_eq!(config.timeout_secs, 120);
}

#[test]
fn client_model_accessor() {
    let config = AnthropicConfig::new("test-key").with_model("claude-3-opus");
    let client = AnthropicClient::new(config).unwrap();
    assert_eq!(client.model(), "claude-3-opus");
}

#[test]
fn client_provider_name() {
    let config = AnthropicConfig::new("test-key");
    let client = AnthropicClient::new(config).unwrap();
    assert_eq!(client.provider_name(), "anthropic");
}

#[test]
fn client_model_id() {
    let config = AnthropicConfig::new("test-key").with_model("claude-3-haiku");
    let client = AnthropicClient::new(config).unwrap();
    assert_eq!(client.model_id(), "claude-3-haiku");
}

#[test]
fn build_request_with_system_prompt() {
    let config = AnthropicConfig::new("test-key");
    let client = AnthropicClient::new(config).unwrap();

    let request = LLMRequest::new("model", vec![Message::user("Hi")]).with_system("Be helpful");
    let api_req = client.build_request(&request);

    assert_eq!(api_req.system, Some("Be helpful".to_string()));
    assert!(!api_req.stream);
}

#[test]
fn build_request_with_streaming() {
    let config = AnthropicConfig::new("test-key");
    let client = AnthropicClient::new(config).unwrap();

    let request = LLMRequest::new("model", vec![Message::user("Hi")]).with_streaming();
    let api_req = client.build_request(&request);

    assert!(api_req.stream);
}

#[test]
fn build_request_max_tokens() {
    let config = AnthropicConfig::new("test-key");
    let client = AnthropicClient::new(config).unwrap();

    let request = LLMRequest::new("model", vec![Message::user("Hi")]).with_max_tokens(1000);
    let api_req = client.build_request(&request);

    assert_eq!(api_req.max_tokens, 1000);
}

#[test]
fn anthropic_message_from_user() {
    let msg = Message::user("Hello");
    let api_msg: AnthropicMessage = (&msg).into();
    assert_eq!(api_msg.role, "user");
    assert_eq!(api_msg.content, "Hello");
}

#[test]
fn anthropic_message_from_assistant() {
    let msg = Message::assistant("Hi there");
    let api_msg: AnthropicMessage = (&msg).into();
    assert_eq!(api_msg.role, "assistant");
    assert_eq!(api_msg.content, "Hi there");
}

#[test]
fn build_request_multiple_messages() {
    let config = AnthropicConfig::new("test-key");
    let client = AnthropicClient::new(config).unwrap();

    let request = LLMRequest::new(
        "model",
        vec![
            Message::user("Hello"),
            Message::assistant("Hi"),
            Message::user("How are you?"),
        ],
    );
    let api_req = client.build_request(&request);

    assert_eq!(api_req.messages.len(), 3);
    assert_eq!(api_req.messages[0].role, "user");
    assert_eq!(api_req.messages[1].role, "assistant");
    assert_eq!(api_req.messages[2].role, "user");
}

#[tokio::test]
async fn send_message_success() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let mock_server = MockServer::start().await;

    let response_body = serde_json::json!({
        "content": [{"type": "text", "text": "Hello!"}],
        "model": "claude-3",
        "stop_reason": "end_turn",
        "usage": {"input_tokens": 10, "output_tokens": 5}
    });

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
        .mount(&mock_server)
        .await;

    let config = AnthropicConfig::new("test-key").with_base_url(mock_server.uri());
    let client = AnthropicClient::new(config).unwrap();

    let request = LLMRequest::new("claude-3", vec![Message::user("Hi")]);
    let response = client.send_message(request).await.unwrap();

    assert_eq!(response.content, "Hello!");
    assert_eq!(response.model, "claude-3");
    assert_eq!(response.stop_reason, StopReason::EndTurn);
    assert_eq!(response.usage.input_tokens, 10);
    assert_eq!(response.usage.output_tokens, 5);
}

#[tokio::test]
async fn send_message_multiple_content_blocks() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let mock_server = MockServer::start().await;

    let response_body = serde_json::json!({
        "content": [
            {"type": "text", "text": "Hello "},
            {"type": "text", "text": "world!"},
            {"type": "tool_use", "id": "t1"}
        ],
        "model": "claude-3",
        "stop_reason": "end_turn",
        "usage": {"input_tokens": 10, "output_tokens": 5}
    });

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
        .mount(&mock_server)
        .await;

    let config = AnthropicConfig::new("test-key").with_base_url(mock_server.uri());
    let client = AnthropicClient::new(config).unwrap();

    let request = LLMRequest::new("claude-3", vec![Message::user("Hi")]);
    let response = client.send_message(request).await.unwrap();

    // Only text blocks joined
    assert_eq!(response.content, "Hello world!");
}

#[tokio::test]
async fn send_message_server_error() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let mock_server = MockServer::start().await;

    let error_body =
        r#"{"type":"error","error":{"type":"server_error","message":"Internal error"}}"#;

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(500).set_body_string(error_body))
        .mount(&mock_server)
        .await;

    let config = AnthropicConfig::new("test-key").with_base_url(mock_server.uri());
    let client = AnthropicClient::new(config).unwrap();

    let request = LLMRequest::new("claude-3", vec![Message::user("Hi")]);
    let result = client.send_message(request).await;

    assert!(result.is_err());
    match result.unwrap_err() {
        LLMError::ApiError { status, message } => {
            assert_eq!(status, 500);
            assert_eq!(message, "Internal error");
        }
        e => panic!("Expected ApiError, got {:?}", e),
    }
}

#[tokio::test]
async fn send_message_auth_error() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let mock_server = MockServer::start().await;

    let error_body =
        r#"{"type":"error","error":{"type":"authentication_error","message":"Invalid API key"}}"#;

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(401).set_body_string(error_body))
        .mount(&mock_server)
        .await;

    let config = AnthropicConfig::new("test-key").with_base_url(mock_server.uri());
    let client = AnthropicClient::new(config).unwrap();

    let request = LLMRequest::new("claude-3", vec![Message::user("Hi")]);
    let result = client.send_message(request).await;

    match result.unwrap_err() {
        LLMError::AuthError(msg) => assert_eq!(msg, "Invalid API key"),
        e => panic!("Expected AuthError, got {:?}", e),
    }
}

#[tokio::test]
async fn send_message_rate_limited() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let mock_server = MockServer::start().await;

    let error_body =
        r#"{"type":"error","error":{"type":"rate_limit_error","message":"Too many requests"}}"#;

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(429).set_body_string(error_body))
        .mount(&mock_server)
        .await;

    let config = AnthropicConfig::new("test-key").with_base_url(mock_server.uri());
    let client = AnthropicClient::new(config).unwrap();

    let request = LLMRequest::new("claude-3", vec![Message::user("Hi")]);
    let result = client.send_message(request).await;

    match result.unwrap_err() {
        LLMError::RateLimited { retry_after_ms } => assert_eq!(retry_after_ms, 60000),
        e => panic!("Expected RateLimited, got {:?}", e),
    }
}

#[tokio::test]
async fn send_message_invalid_json_response() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_string("not json"))
        .mount(&mock_server)
        .await;

    let config = AnthropicConfig::new("test-key").with_base_url(mock_server.uri());
    let client = AnthropicClient::new(config).unwrap();

    let request = LLMRequest::new("claude-3", vec![Message::user("Hi")]);
    let result = client.send_message(request).await;

    assert!(matches!(result.unwrap_err(), LLMError::ParseError(_)));
}

// ── SSE tool use parsing ─────────────────────────────────────────────

#[test]
fn parse_sse_tool_use_start() {
    let line = r#"data: {"type":"content_block_start","index":1,"content_block":{"type":"tool_use","id":"toolu_123","name":"get_weather"}}"#;
    let result = AnthropicClient::parse_sse_line(line);
    assert!(result.is_some());
    let chunk = result.unwrap().unwrap();
    match chunk {
        StreamChunk::ToolUseStart { index, id, name } => {
            assert_eq!(index, 1);
            assert_eq!(id, "toolu_123");
            assert_eq!(name, "get_weather");
        }
        _ => panic!("Expected ToolUseStart, got {:?}", chunk),
    }
}

#[test]
fn parse_sse_input_json_delta() {
    let line = r#"data: {"type":"content_block_delta","index":1,"delta":{"type":"input_json_delta","partial_json":"{\"city\":"}}"#;
    let result = AnthropicClient::parse_sse_line(line);
    assert!(result.is_some());
    let chunk = result.unwrap().unwrap();
    match chunk {
        StreamChunk::InputJsonDelta {
            index,
            partial_json,
        } => {
            assert_eq!(index, 1);
            assert_eq!(partial_json, r#"{"city":"#);
        }
        _ => panic!("Expected InputJsonDelta, got {:?}", chunk),
    }
}

#[tokio::test]
async fn send_message_rate_limited_with_retry_after() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let mock_server = MockServer::start().await;

    let error_body =
        r#"{"type":"error","error":{"type":"rate_limit_error","message":"Too many requests"}}"#;

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(
            ResponseTemplate::new(429)
                .set_body_string(error_body)
                .insert_header("retry-after", "2.5"),
        )
        .mount(&mock_server)
        .await;

    let config = AnthropicConfig::new("test-key").with_base_url(mock_server.uri());
    let client = AnthropicClient::new(config).unwrap();

    let request = LLMRequest::new("claude-3", vec![Message::user("Hi")]);
    let result = client.send_message(request).await;

    match result.unwrap_err() {
        LLMError::RateLimited { retry_after_ms } => assert_eq!(retry_after_ms, 2500),
        e => panic!("Expected RateLimited, got {:?}", e),
    }
}
