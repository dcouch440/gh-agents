//! Anthropic Messages API provider.
//!
//! Implements `SseProviderAdapter` for the Anthropic Messages API.
//! The actual HTTP/SSE boilerplate is handled by `SseHttpProvider`.

use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};

use super::provider::LLMResult;
use super::sse_provider::{SseHttpProvider, SseProviderAdapter};
use super::types::{
    ContentBlock, LLMError, LLMRequest, LLMResponse, Message, Role, StopReason, StreamChunk,
    TokenUsage,
};

/// Anthropic API version header value
const ANTHROPIC_VERSION: &str = "2023-06-01";

/// Default API base URL
const DEFAULT_BASE_URL: &str = "https://api.anthropic.com";

/// Default timeout for requests
const DEFAULT_TIMEOUT_SECS: u64 = 120;

// ── Config ───────────────────────────────────────────────────────────────

/// Configuration for the Anthropic client.
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
            .unwrap_or_else(|_| crate::constants::ANTHROPIC_DEFAULT_MODEL.to_string());

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
            model: crate::constants::ANTHROPIC_DEFAULT_MODEL.to_string(),
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

// ── Adapter ──────────────────────────────────────────────────────────────

/// Anthropic-specific adapter for `SseHttpProvider`.
#[derive(Clone)]
pub struct AnthropicAdapter {
    pub(crate) config: AnthropicConfig,
}

/// The Anthropic Messages API client.
///
/// This is a type alias for `SseHttpProvider<AnthropicAdapter>`, which handles
/// all shared HTTP and SSE boilerplate. Provider-specific logic (request
/// building, response parsing, SSE event mapping) lives in `AnthropicAdapter`.
pub type AnthropicClient = SseHttpProvider<AnthropicAdapter>;

impl AnthropicClient {
    /// Create a new Anthropic client from config.
    pub fn with_config(config: AnthropicConfig) -> Result<Self, LLMError> {
        if config.api_key.is_empty() {
            return Err(LLMError::AuthError("API key cannot be empty".to_string()));
        }
        let adapter = AnthropicAdapter { config };
        SseHttpProvider::new(adapter)
    }

    /// Create client from environment.
    pub fn from_env() -> Result<Self, LLMError> {
        let config = AnthropicConfig::from_env()?;
        Self::with_config(config)
    }

    /// Get the configured model.
    pub fn model(&self) -> &str {
        &self.adapter().config.model
    }
}

// ── SseProviderAdapter impl ──────────────────────────────────────────────

impl SseProviderAdapter for AnthropicAdapter {
    fn provider_name(&self) -> &'static str {
        "anthropic"
    }

    fn model_id(&self) -> &str {
        &self.config.model
    }

    fn endpoint_url(&self) -> String {
        format!("{}/v1/messages", self.config.base_url)
    }

    fn default_headers(&self) -> Result<HeaderMap, LLMError> {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-api-key",
            HeaderValue::from_str(&self.config.api_key)
                .map_err(|_| LLMError::AuthError("Invalid API key format".to_string()))?,
        );
        headers.insert(
            "anthropic-version",
            HeaderValue::from_static(ANTHROPIC_VERSION),
        );
        headers.insert("content-type", HeaderValue::from_static("application/json"));
        Ok(headers)
    }

    fn timeout_secs(&self) -> u64 {
        self.config.timeout_secs
    }

    fn build_request_body(&self, request: &LLMRequest, stream: bool) -> serde_json::Value {
        let api_request = AnthropicRequest {
            model: if request.model.is_empty() {
                self.config.model.clone()
            } else {
                request.model.clone()
            },
            messages: request.messages.iter().map(|m| m.into()).collect(),
            max_tokens: request.max_tokens,
            system: request.system.clone(),
            stream,
            tools: request.tools.clone(),
        };
        serde_json::to_value(api_request).unwrap_or_default()
    }

    fn parse_response(&self, body: &[u8]) -> Result<LLMResponse, LLMError> {
        let api_response: AnthropicResponse =
            serde_json::from_slice(body).map_err(|e| LLMError::ParseError(e.to_string()))?;

        let mut text_parts = Vec::new();
        let mut content_blocks = Vec::new();

        for block in &api_response.content {
            match block.block_type.as_str() {
                "text" => {
                    if let Some(text) = &block.text {
                        text_parts.push(text.clone());
                        content_blocks.push(ContentBlock::Text { text: text.clone() });
                    }
                }
                "tool_use" => {
                    if let (Some(id), Some(name), Some(input)) =
                        (&block.id, &block.name, &block.input)
                    {
                        content_blocks.push(ContentBlock::ToolUse {
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
            stop_reason: parse_stop_reason(&api_response.stop_reason),
            usage: TokenUsage {
                input_tokens: api_response.usage.input_tokens,
                output_tokens: api_response.usage.output_tokens,
                cached_input_tokens: 0,
            },
        })
    }

    fn parse_sse_line(&self, line: &str) -> Option<LLMResult<StreamChunk>> {
        if !line.starts_with("data: ") {
            return None;
        }

        let json_str = &line[6..];

        if json_str == "[DONE]" {
            return Some(Ok(StreamChunk::MessageStop));
        }

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
                        } else {
                            let partial_json = delta.partial_json?;
                            StreamChunk::InputJsonDelta {
                                index,
                                partial_json,
                            }
                        }
                    }
                    SSEData::ContentBlockStop { index } => StreamChunk::ContentBlockStop { index },
                    SSEData::MessageDelta { delta, usage } => StreamChunk::MessageDelta {
                        stop_reason: delta.stop_reason.map(|r| parse_stop_reason(&r)),
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
                tracing::warn!("Failed to parse SSE event: {} - line: {}", e, json_str);
                None
            }
        }
    }

    fn handle_error(&self, status: u16, body: &str, retry_after_ms: Option<u64>) -> LLMError {
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
}

// ── Helpers ──────────────────────────────────────────────────────────────

/// Parse stop reason string to enum.
pub(crate) fn parse_stop_reason(reason: &str) -> StopReason {
    match reason {
        "end_turn" => StopReason::EndTurn,
        "max_tokens" => StopReason::MaxTokens,
        "stop_sequence" => StopReason::StopSequence,
        "tool_use" => StopReason::ToolUse,
        _ => StopReason::EndTurn,
    }
}

// ── Anthropic API wire types ─────────────────────────────────────────────

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
    id: Option<String>,
    name: Option<String>,
    input: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
}

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

// ── SSE event types ──────────────────────────────────────────────────────

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
    id: Option<String>,
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DeltaData {
    #[serde(rename = "type")]
    #[allow(dead_code)]
    delta_type: String,
    text: Option<String>,
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

#[cfg(test)]
mod tests;
