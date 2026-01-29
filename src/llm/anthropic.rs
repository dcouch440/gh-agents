//! Anthropic API client implementation

use async_trait::async_trait;
use futures::{Stream, StreamExt};
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client,
};
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use std::time::Duration;

use super::provider::{LLMProvider, LLMResult};
use super::types::{
    LLMError, LLMRequest, LLMResponse, Message, Role, StopReason, StreamChunk, TokenUsage,
};

/// Anthropic API version header value
const ANTHROPIC_VERSION: &str = "2023-06-01";

/// Default API base URL
const DEFAULT_BASE_URL: &str = "https://api.anthropic.com";

/// Default timeout for requests
const DEFAULT_TIMEOUT_SECS: u64 = 120;

/// Configuration for the Anthropic client
#[derive(Debug, Clone)]
pub struct AnthropicConfig {
    /// API key for authentication
    pub api_key: String,

    /// Base URL for the API (defaults to https://api.anthropic.com)
    pub base_url: String,

    /// Default model to use
    pub model: String,

    /// Request timeout in seconds
    pub timeout_secs: u64,
}

impl AnthropicConfig {
    /// Create config from environment
    pub fn from_env() -> Result<Self, LLMError> {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .map_err(|_| LLMError::AuthError("ANTHROPIC_API_KEY not set".to_string()))?;

        Ok(Self {
            api_key,
            base_url: DEFAULT_BASE_URL.to_string(),
            model: "claude-sonnet-4-20250514".to_string(),
            timeout_secs: DEFAULT_TIMEOUT_SECS,
        })
    }

    /// Create config with explicit API key
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: DEFAULT_BASE_URL.to_string(),
            model: "claude-sonnet-4-20250514".to_string(),
            timeout_secs: DEFAULT_TIMEOUT_SECS,
        }
    }

    /// Set the model
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Set the base URL (for testing)
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }
}

/// Anthropic Messages API client
pub struct AnthropicClient {
    /// HTTP client
    client: Client,

    /// Client configuration
    config: AnthropicConfig,
}

impl AnthropicClient {
    /// Create a new Anthropic client
    pub fn new(config: AnthropicConfig) -> Result<Self, LLMError> {
        // Validate API key format (basic check)
        if config.api_key.is_empty() {
            return Err(LLMError::AuthError("API key cannot be empty".to_string()));
        }

        // Build default headers
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-api-key",
            HeaderValue::from_str(&config.api_key)
                .map_err(|_| LLMError::AuthError("Invalid API key format".to_string()))?,
        );
        headers.insert(
            "anthropic-version",
            HeaderValue::from_static(ANTHROPIC_VERSION),
        );
        headers.insert("content-type", HeaderValue::from_static("application/json"));

        // Build HTTP client
        let client = Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()
            .map_err(LLMError::HttpError)?;

        Ok(Self { client, config })
    }

    /// Create client from environment
    pub fn from_env() -> Result<Self, LLMError> {
        let config = AnthropicConfig::from_env()?;
        Self::new(config)
    }

    /// Get the messages API URL
    fn messages_url(&self) -> String {
        format!("{}/v1/messages", self.config.base_url)
    }

    /// Get the configured model
    pub fn model(&self) -> &str {
        &self.config.model
    }

    /// Convert LLMRequest to Anthropic API format
    fn build_request(&self, request: &LLMRequest) -> AnthropicRequest {
        AnthropicRequest {
            model: if request.model.is_empty() {
                self.config.model.clone()
            } else {
                request.model.clone()
            },
            messages: request.messages.iter().map(|m| m.into()).collect(),
            max_tokens: request.max_tokens,
            system: request.system.clone(),
            stream: request.stream,
        }
    }

    /// Parse stop reason string to enum
    fn parse_stop_reason(reason: &str) -> StopReason {
        match reason {
            "end_turn" => StopReason::EndTurn,
            "max_tokens" => StopReason::MaxTokens,
            "stop_sequence" => StopReason::StopSequence,
            "tool_use" => StopReason::ToolUse,
            _ => StopReason::EndTurn,
        }
    }

    /// Handle API error response
    fn handle_error_response(status: u16, body: &str) -> LLMError {
        // Try to parse as Anthropic error
        if let Ok(error) = serde_json::from_str::<AnthropicError>(body) {
            match status {
                401 => LLMError::AuthError(error.error.message),
                429 => {
                    // Try to extract retry-after from headers (simplified)
                    LLMError::RateLimited {
                        retry_after_ms: 60000,
                    }
                }
                _ => LLMError::ApiError {
                    status,
                    message: error.error.message,
                },
            }
        } else {
            LLMError::ApiError {
                status,
                message: body.to_string(),
            }
        }
    }

    /// Parse an SSE line into a StreamChunk
    fn parse_sse_line(line: &str) -> Option<LLMResult<StreamChunk>> {
        // SSE format: "data: {json}"
        if !line.starts_with("data: ") {
            return None;
        }

        let json_str = &line[6..]; // Skip "data: "

        // Handle special "[DONE]" marker (some APIs use this)
        if json_str == "[DONE]" {
            return Some(Ok(StreamChunk::MessageStop));
        }

        // Parse the JSON event
        match serde_json::from_str::<SSEData>(json_str) {
            Ok(event) => {
                let chunk = match event {
                    SSEData::MessageStart { message } => StreamChunk::MessageStart {
                        model: message.model,
                        input_tokens: message.usage.input_tokens,
                    },
                    SSEData::ContentBlockStart { index } => {
                        StreamChunk::ContentBlockStart { index }
                    }
                    SSEData::ContentBlockDelta { index, delta } => {
                        if let Some(text) = delta.text {
                            StreamChunk::ContentDelta { text, index }
                        } else {
                            return None;
                        }
                    }
                    SSEData::ContentBlockStop { index } => StreamChunk::ContentBlockStop { index },
                    SSEData::MessageDelta { delta, usage } => StreamChunk::MessageDelta {
                        stop_reason: delta.stop_reason.map(|r| Self::parse_stop_reason(&r)),
                        output_tokens: usage.map(|u| u.output_tokens),
                    },
                    SSEData::MessageStop => StreamChunk::MessageStop,
                    SSEData::Ping => StreamChunk::Ping,
                    SSEData::Error { error } => {
                        return Some(Err(LLMError::ApiError {
                            status: 500,
                            message: error.message,
                        }));
                    }
                };
                Some(Ok(chunk))
            }
            Err(e) => {
                // Log parse error but don't fail the stream
                tracing::warn!("Failed to parse SSE event: {} - line: {}", e, json_str);
                None
            }
        }
    }
}

// -- Anthropic API request/response structures --

/// Anthropic API request format
#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    messages: Vec<AnthropicMessage>,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    stream: bool,
}

#[derive(Debug, Serialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

impl From<&Message> for AnthropicMessage {
    fn from(msg: &Message) -> Self {
        Self {
            role: match msg.role {
                Role::User => "user".to_string(),
                Role::Assistant => "assistant".to_string(),
            },
            content: msg.content.clone(),
        }
    }
}

/// Anthropic API response format
#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    content: Vec<ContentBlock>,
    model: String,
    stop_reason: String,
    usage: AnthropicUsage,
}

#[derive(Debug, Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    block_type: String,
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
}

/// Anthropic API error response
#[derive(Debug, Deserialize)]
struct AnthropicError {
    #[serde(rename = "type")]
    #[allow(dead_code)]
    error_type: String,
    error: ErrorDetail,
}

#[derive(Debug, Deserialize)]
struct ErrorDetail {
    #[serde(rename = "type")]
    #[allow(dead_code)]
    error_type: String,
    message: String,
}

// -- SSE event types from Anthropic --

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum SSEData {
    #[serde(rename = "message_start")]
    MessageStart { message: MessageStartData },

    #[serde(rename = "content_block_start")]
    ContentBlockStart { index: usize },

    #[serde(rename = "content_block_delta")]
    ContentBlockDelta { index: usize, delta: DeltaData },

    #[serde(rename = "content_block_stop")]
    ContentBlockStop { index: usize },

    #[serde(rename = "message_delta")]
    MessageDelta {
        delta: MessageDeltaData,
        usage: Option<DeltaUsage>,
    },

    #[serde(rename = "message_stop")]
    MessageStop,

    #[serde(rename = "ping")]
    Ping,

    #[serde(rename = "error")]
    Error { error: ErrorDetail },
}

#[derive(Debug, Deserialize)]
struct MessageStartData {
    model: String,
    usage: AnthropicUsage,
}

#[derive(Debug, Deserialize)]
struct DeltaData {
    #[serde(rename = "type")]
    #[allow(dead_code)]
    delta_type: String,
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MessageDeltaData {
    stop_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DeltaUsage {
    output_tokens: u32,
}

// -- LLMProvider trait implementation --

#[async_trait]
impl LLMProvider for AnthropicClient {
    async fn send_message(&self, request: LLMRequest) -> LLMResult<LLMResponse> {
        let api_request = self.build_request(&request);

        let response = self
            .client
            .post(self.messages_url())
            .json(&api_request)
            .send()
            .await?;

        let status = response.status().as_u16();

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(Self::handle_error_response(status, &body));
        }

        let api_response: AnthropicResponse = response
            .json()
            .await
            .map_err(|e| LLMError::ParseError(e.to_string()))?;

        // Extract text content from response
        let content = api_response
            .content
            .iter()
            .filter_map(|block| {
                if block.block_type == "text" {
                    block.text.clone()
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("");

        Ok(LLMResponse {
            content,
            model: api_response.model,
            stop_reason: Self::parse_stop_reason(&api_response.stop_reason),
            usage: TokenUsage {
                input_tokens: api_response.usage.input_tokens,
                output_tokens: api_response.usage.output_tokens,
            },
        })
    }

    async fn send_message_stream(
        &self,
        request: LLMRequest,
    ) -> LLMResult<Pin<Box<dyn Stream<Item = LLMResult<StreamChunk>> + Send>>> {
        let mut api_request = self.build_request(&request);
        api_request.stream = true;

        let response = self
            .client
            .post(self.messages_url())
            .json(&api_request)
            .send()
            .await?;

        let status = response.status().as_u16();

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(Self::handle_error_response(status, &body));
        }

        // Create a stream from the response bytes
        let byte_stream = response.bytes_stream();

        // Convert to line stream and parse SSE events
        let stream = async_stream::stream! {
            let mut buffer = String::new();

            let mut stream = byte_stream;
            while let Some(chunk_result) = stream.next().await {
                match chunk_result {
                    Ok(bytes) => {
                        // Append to buffer
                        buffer.push_str(&String::from_utf8_lossy(&bytes));

                        // Process complete lines
                        while let Some(newline_pos) = buffer.find('\n') {
                            let line = buffer[..newline_pos].trim().to_string();
                            buffer = buffer[newline_pos + 1..].to_string();

                            // Skip empty lines and event type lines
                            if line.is_empty() || line.starts_with("event:") {
                                continue;
                            }

                            if let Some(result) = Self::parse_sse_line(&line) {
                                yield result;
                            }
                        }
                    }
                    Err(e) => {
                        yield Err(LLMError::StreamError(e.to_string()));
                        break;
                    }
                }
            }
        };

        Ok(Box::pin(stream))
    }

    fn provider_name(&self) -> &'static str {
        "anthropic"
    }

    fn model_id(&self) -> &str {
        &self.config.model
    }
}

#[cfg(test)]
mod tests {
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
        let body = r#"{"type":"error","error":{"type":"authentication_error","message":"Invalid API key"}}"#;
        let error = AnthropicClient::handle_error_response(401, body);
        match error {
            LLMError::AuthError(msg) => assert_eq!(msg, "Invalid API key"),
            _ => panic!("Expected AuthError, got {:?}", error),
        }
    }

    #[test]
    fn handle_error_429_rate_limited() {
        let body = r#"{"type":"error","error":{"type":"rate_limit_error","message":"Too many requests"}}"#;
        let error = AnthropicClient::handle_error_response(429, body);
        match error {
            LLMError::RateLimited { retry_after_ms } => assert_eq!(retry_after_ms, 60000),
            _ => panic!("Expected RateLimited, got {:?}", error),
        }
    }

    #[test]
    fn handle_error_500_api_error() {
        let body = r#"{"type":"error","error":{"type":"server_error","message":"Internal error"}}"#;
        let error = AnthropicClient::handle_error_response(500, body);
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
        let error = AnthropicClient::handle_error_response(502, "not json at all");
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
        let error = AnthropicClient::handle_error_response(500, "");
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
            StreamChunk::MessageDelta { stop_reason, output_tokens } => {
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
            StreamChunk::MessageDelta { stop_reason, output_tokens } => {
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
            StreamChunk::MessageDelta { stop_reason, output_tokens } => {
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
        let line = r#"data: {"type":"error","error":{"type":"overloaded_error","message":"Overloaded"}}"#;
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
        let line = r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta"}}"#;
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

        let request = LLMRequest::new("model", vec![Message::user("Hi")])
            .with_system("Be helpful");
        let api_req = client.build_request(&request);

        assert_eq!(api_req.system, Some("Be helpful".to_string()));
        assert!(!api_req.stream);
    }

    #[test]
    fn build_request_with_streaming() {
        let config = AnthropicConfig::new("test-key");
        let client = AnthropicClient::new(config).unwrap();

        let request = LLMRequest::new("model", vec![Message::user("Hi")])
            .with_streaming();
        let api_req = client.build_request(&request);

        assert!(api_req.stream);
    }

    #[test]
    fn build_request_max_tokens() {
        let config = AnthropicConfig::new("test-key");
        let client = AnthropicClient::new(config).unwrap();

        let request = LLMRequest::new("model", vec![Message::user("Hi")])
            .with_max_tokens(1000);
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

        let request = LLMRequest::new("model", vec![
            Message::user("Hello"),
            Message::assistant("Hi"),
            Message::user("How are you?"),
        ]);
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

        let config = AnthropicConfig::new("test-key")
            .with_base_url(mock_server.uri());
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

        let config = AnthropicConfig::new("test-key")
            .with_base_url(mock_server.uri());
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

        let error_body = r#"{"type":"error","error":{"type":"server_error","message":"Internal error"}}"#;

        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(500).set_body_string(error_body))
            .mount(&mock_server)
            .await;

        let config = AnthropicConfig::new("test-key")
            .with_base_url(mock_server.uri());
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

        let error_body = r#"{"type":"error","error":{"type":"authentication_error","message":"Invalid API key"}}"#;

        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(401).set_body_string(error_body))
            .mount(&mock_server)
            .await;

        let config = AnthropicConfig::new("test-key")
            .with_base_url(mock_server.uri());
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

        let error_body = r#"{"type":"error","error":{"type":"rate_limit_error","message":"Too many requests"}}"#;

        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(429).set_body_string(error_body))
            .mount(&mock_server)
            .await;

        let config = AnthropicConfig::new("test-key")
            .with_base_url(mock_server.uri());
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

        let config = AnthropicConfig::new("test-key")
            .with_base_url(mock_server.uri());
        let client = AnthropicClient::new(config).unwrap();

        let request = LLMRequest::new("claude-3", vec![Message::user("Hi")]);
        let result = client.send_message(request).await;

        assert!(matches!(result.unwrap_err(), LLMError::ParseError(_)));
    }
}
