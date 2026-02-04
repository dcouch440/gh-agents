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
        let api_key = std::env::var(crate::constants::ENV_ANTHROPIC_API_KEY).map_err(|_| {
            LLMError::AuthError(format!(
                "{} not set",
                crate::constants::ENV_ANTHROPIC_API_KEY
            ))
        })?;

        let model = std::env::var(crate::constants::ENV_ANTHROPIC_MODEL)
            .unwrap_or_else(|_| crate::constants::DEFAULT_MODEL.to_string());

        Ok(Self {
            api_key,
            base_url: DEFAULT_BASE_URL.to_string(),
            model,
            timeout_secs: DEFAULT_TIMEOUT_SECS,
        })
    }

    /// Create config with explicit API key
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: DEFAULT_BASE_URL.to_string(),
            model: crate::constants::DEFAULT_MODEL.to_string(),
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
#[derive(Clone)]
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
            tools: request.tools.clone(),
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
    fn handle_error_response(status: u16, body: &str, retry_after_ms: Option<u64>) -> LLMError {
        // Try to parse as Anthropic error
        if let Ok(error) = serde_json::from_str::<AnthropicError>(body) {
            match status {
                401 => LLMError::AuthError(error.error.message),
                429 => LLMError::RateLimited {
                    retry_after_ms: retry_after_ms.unwrap_or(60000),
                },
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

    /// Extract retry-after value from response headers (in milliseconds)
    fn parse_retry_after(response: &reqwest::Response) -> Option<u64> {
        response
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<f64>().ok())
            .map(|secs| (secs * 1000.0) as u64)
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
                    SSEData::ContentBlockStart {
                        index,
                        content_block,
                    } => {
                        // Check if this is a tool_use content block
                        if let Some(ref cb) = content_block {
                            if cb.block_type == "tool_use" {
                                if let (Some(id), Some(name)) = (&cb.id, &cb.name) {
                                    return Some(Ok(StreamChunk::ToolUseStart {
                                        index,
                                        id: id.clone(),
                                        name: name.clone(),
                                    }));
                                }
                            }
                        }
                        StreamChunk::ContentBlockStart { index }
                    }
                    SSEData::ContentBlockDelta { index, delta } => {
                        if let Some(text) = delta.text {
                            StreamChunk::ContentDelta { text, index }
                        } else if let Some(partial_json) = delta.partial_json {
                            StreamChunk::InputJsonDelta {
                                index,
                                partial_json,
                            }
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
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<super::types::Tool>,
}

#[derive(Debug, Serialize)]
struct AnthropicMessage {
    role: String,
    content: serde_json::Value,
}

impl From<&Message> for AnthropicMessage {
    fn from(msg: &Message) -> Self {
        let content = match &msg.content {
            super::types::MessageContent::Text(s) => serde_json::Value::String(s.clone()),
            super::types::MessageContent::Blocks(blocks) => {
                serde_json::to_value(blocks).unwrap_or(serde_json::Value::Array(vec![]))
            }
        };
        Self {
            role: match msg.role {
                Role::User => "user".to_string(),
                Role::Assistant => "assistant".to_string(),
            },
            content,
        }
    }
}

/// Anthropic API response format
#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    content: Vec<ApiContentBlock>,
    model: String,
    stop_reason: String,
    usage: AnthropicUsage,
}

#[derive(Debug, Deserialize)]
struct ApiContentBlock {
    #[serde(rename = "type")]
    block_type: String,
    text: Option<String>,
    // Tool use fields
    id: Option<String>,
    name: Option<String>,
    input: Option<serde_json::Value>,
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
    ContentBlockStart {
        index: usize,
        content_block: Option<ContentBlockStartData>,
    },

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
struct ContentBlockStartData {
    #[serde(rename = "type")]
    block_type: String,
    /// Tool use ID (only for tool_use blocks)
    id: Option<String>,
    /// Tool name (only for tool_use blocks)
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DeltaData {
    #[serde(rename = "type")]
    #[allow(dead_code)]
    delta_type: String,
    text: Option<String>,
    /// Partial JSON for tool use input (only for input_json_delta)
    partial_json: Option<String>,
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
            let retry_after = Self::parse_retry_after(&response);
            let body = response.text().await.unwrap_or_default();
            return Err(Self::handle_error_response(status, &body, retry_after));
        }

        let api_response: AnthropicResponse = response
            .json()
            .await
            .map_err(|e| LLMError::ParseError(e.to_string()))?;

        // Build content blocks and extract text
        let mut text_parts = Vec::new();
        let mut content_blocks = Vec::new();

        for block in &api_response.content {
            match block.block_type.as_str() {
                "text" => {
                    if let Some(text) = &block.text {
                        text_parts.push(text.clone());
                        content_blocks
                            .push(super::types::ContentBlock::Text { text: text.clone() });
                    }
                }
                "tool_use" => {
                    if let (Some(id), Some(name), Some(input)) =
                        (&block.id, &block.name, &block.input)
                    {
                        content_blocks.push(super::types::ContentBlock::ToolUse {
                            id: id.clone(),
                            name: name.clone(),
                            input: input.clone(),
                        });
                    }
                }
                _ => {}
            }
        }

        Ok(LLMResponse {
            content: text_parts.join(""),
            content_blocks,
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
            let retry_after = Self::parse_retry_after(&response);
            let body = response.text().await.unwrap_or_default();
            return Err(Self::handle_error_response(status, &body, retry_after));
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
mod tests;
