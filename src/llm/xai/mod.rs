//! xAI Responses API client implementing `LLMProvider`.
//!
//! Targets the `/v1/responses` endpoint — xAI's primary API surface.
//! Supports function calling, streaming (SSE), and built-in server-side
//! tools (`web_search`, `x_search`) alongside user-defined function tools.
//!
//! The existing `GrokResearchClient` is a simpler research-only client.
//! This provider is the full-featured replacement for general-purpose use.

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
    ContentBlock, LLMError, LLMRequest, LLMResponse, Message, MessageContent, Role, StopReason,
    StreamChunk, TokenUsage, Tool,
};

// ── Config ──────────────────────────────────────────────────────────────────

/// Configuration for the xAI Responses API client.
#[derive(Debug, Clone)]
pub struct XaiConfig {
    /// xAI API key (Bearer token).
    pub api_key: String,
    /// Base URL for the xAI API.
    pub base_url: String,
    /// Default model for responses.
    pub model: String,
    /// Request timeout in seconds.
    pub timeout_secs: u64,
    /// Enable the built-in `web_search` server-side tool.
    pub web_search: bool,
    /// Enable the built-in `x_search` server-side tool.
    pub x_search: bool,
}

impl XaiConfig {
    /// Create config from environment variables.
    ///
    /// Reads `XAI_API_KEY` (required) and `XAI_MODEL` (optional).
    /// Built-in search tools are disabled by default.
    pub fn from_env() -> Result<Self, LLMError> {
        let api_key = std::env::var(crate::constants::ENV_XAI_API_KEY).map_err(|_| {
            LLMError::AuthError(format!("{} not set", crate::constants::ENV_XAI_API_KEY))
        })?;

        let model = std::env::var(crate::constants::ENV_XAI_MODEL)
            .unwrap_or_else(|_| crate::constants::XAI_DEFAULT_CHAT_MODEL.to_string());

        Ok(Self {
            api_key,
            base_url: crate::constants::XAI_DEFAULT_BASE_URL.to_string(),
            model,
            timeout_secs: crate::constants::XAI_CHAT_TIMEOUT_SECS,
            web_search: false,
            x_search: false,
        })
    }

    /// Set the model.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Set the base URL (useful for testing).
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    /// Enable the built-in `web_search` tool.
    pub fn with_web_search(mut self) -> Self {
        self.web_search = true;
        self
    }

    /// Enable the built-in `x_search` tool.
    pub fn with_x_search(mut self) -> Self {
        self.x_search = true;
        self
    }
}

// ── Client ──────────────────────────────────────────────────────────────────

/// xAI Responses API client.
#[derive(Clone)]
pub struct XaiClient {
    client: Client,
    config: XaiConfig,
}

impl XaiClient {
    /// Create a new client from config.
    pub fn new(config: XaiConfig) -> Result<Self, LLMError> {
        if config.api_key.is_empty() {
            return Err(LLMError::AuthError(
                "xAI API key cannot be empty".to_string(),
            ));
        }

        let mut headers = HeaderMap::new();
        headers.insert(
            "authorization",
            HeaderValue::from_str(&format!("Bearer {}", config.api_key))
                .map_err(|_| LLMError::AuthError("Invalid xAI API key format".to_string()))?,
        );
        headers.insert("content-type", HeaderValue::from_static("application/json"));

        let client = Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()
            .map_err(LLMError::HttpError)?;

        Ok(Self { client, config })
    }

    /// Create client from environment variables.
    pub fn from_env() -> Result<Self, LLMError> {
        let config = XaiConfig::from_env()?;
        Self::new(config)
    }

    /// Responses API endpoint URL.
    fn responses_url(&self) -> String {
        format!("{}/v1/responses", self.config.base_url)
    }

    // ── Request building ────────────────────────────────────────────────

    /// Convert an `LLMRequest` to the Responses API request body.
    fn build_request(&self, request: &LLMRequest, stream: bool) -> XaiRequest {
        let model = if request.model.is_empty() {
            self.config.model.clone()
        } else {
            request.model.clone()
        };

        // Build the flat input array from conversation messages
        let mut input = Vec::new();
        for msg in &request.messages {
            Self::convert_message(msg, &mut input);
        }

        // Build tools array: built-in tools + function tools
        let mut tools = Vec::new();

        if self.config.web_search {
            tools.push(serde_json::json!({"type": "web_search"}));
        }
        if self.config.x_search {
            tools.push(serde_json::json!({"type": "x_search"}));
        }

        for tool in &request.tools {
            tools.push(Self::function_tool_json(tool));
        }

        XaiRequest {
            model,
            instructions: request.system.clone(),
            input,
            tools: if tools.is_empty() { None } else { Some(tools) },
            max_output_tokens: Some(request.max_tokens),
            temperature: Some(request.temperature),
            stream,
        }
    }

    /// Serialize a function tool to the Responses API format.
    ///
    /// Responses API uses a flat structure:
    /// `{"type": "function", "name": "...", "description": "...", "parameters": {...}}`
    fn function_tool_json(tool: &Tool) -> serde_json::Value {
        serde_json::json!({
            "type": "function",
            "name": tool.name,
            "description": tool.description,
            "parameters": tool.input_schema,
        })
    }

    /// Convert an internal `Message` into Responses API input items.
    ///
    /// The Responses API uses a flat array where:
    /// - User text: `{"role": "user", "content": "..."}`
    /// - Assistant text: `{"role": "assistant", "content": [{"type": "output_text", "text": "..."}]}`
    /// - Assistant tool calls: separate `{"type": "function_call", ...}` items
    /// - Tool results: `{"type": "function_call_output", "call_id": "...", "output": "..."}`
    fn convert_message(msg: &Message, out: &mut Vec<serde_json::Value>) {
        match (&msg.role, &msg.content) {
            (Role::User, MessageContent::Text(s)) => {
                out.push(serde_json::json!({"role": "user", "content": s}));
            }
            (Role::Assistant, MessageContent::Text(s)) => {
                out.push(serde_json::json!({
                    "type": "message",
                    "role": "assistant",
                    "content": [{"type": "output_text", "text": s}]
                }));
            }
            (Role::Assistant, MessageContent::Blocks(blocks)) => {
                // Collect text for the assistant message
                let text: String = blocks
                    .iter()
                    .filter_map(|b| match b {
                        ContentBlock::Text { text } => Some(text.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("");

                if !text.is_empty() {
                    out.push(serde_json::json!({
                        "type": "message",
                        "role": "assistant",
                        "content": [{"type": "output_text", "text": text}]
                    }));
                }

                // Each tool call is a separate input item
                for block in blocks {
                    if let ContentBlock::ToolUse { id, name, input } = block {
                        out.push(serde_json::json!({
                            "type": "function_call",
                            "call_id": id,
                            "name": name,
                            "arguments": serde_json::to_string(input).unwrap_or_default(),
                        }));
                    }
                }
            }
            (Role::User, MessageContent::Blocks(blocks)) => {
                let mut text_parts = Vec::new();

                for block in blocks {
                    match block {
                        ContentBlock::Text { text } => text_parts.push(text.clone()),
                        ContentBlock::ToolResult {
                            tool_use_id,
                            content,
                        } => {
                            // Flush accumulated text first
                            if !text_parts.is_empty() {
                                out.push(
                                    serde_json::json!({"role": "user", "content": text_parts.join("")}),
                                );
                                text_parts.clear();
                            }
                            out.push(serde_json::json!({
                                "type": "function_call_output",
                                "call_id": tool_use_id,
                                "output": content,
                            }));
                        }
                        ContentBlock::ToolUse { .. } => {} // Shouldn't appear in user messages
                    }
                }

                if !text_parts.is_empty() {
                    out.push(
                        serde_json::json!({"role": "user", "content": text_parts.join("")}),
                    );
                }
            }
        }
    }

    // ── Response parsing ────────────────────────────────────────────────

    /// Parse a non-streaming Responses API response into `LLMResponse`.
    fn parse_response(api_response: XaiResponse) -> LLMResponse {
        let mut text_parts = Vec::new();
        let mut content_blocks = Vec::new();
        let mut has_function_calls = false;

        for item in &api_response.output {
            match item.item_type.as_str() {
                "message" => {
                    if let Some(ref content) = item.content {
                        for part in content {
                            if part.part_type == "output_text" {
                                if let Some(ref text) = part.text {
                                    text_parts.push(text.clone());
                                    content_blocks
                                        .push(ContentBlock::Text { text: text.clone() });
                                }
                            }
                        }
                    }
                }
                "function_call" => {
                    has_function_calls = true;
                    let input = item
                        .arguments
                        .as_deref()
                        .and_then(|s| serde_json::from_str(s).ok())
                        .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

                    content_blocks.push(ContentBlock::ToolUse {
                        id: item.call_id.clone().unwrap_or_default(),
                        name: item.name.clone().unwrap_or_default(),
                        input,
                    });
                }
                _ => {} // web_search_call, x_search_call, reasoning, etc. — skip
            }
        }

        // Tool blocks are authoritative — if we parsed function_call items,
        // the stop reason is ToolUse regardless of the status field value.
        let stop_reason = if has_function_calls {
            StopReason::ToolUse
        } else {
            match api_response.status.as_str() {
                "incomplete" => StopReason::MaxTokens,
                _ => StopReason::EndTurn,
            }
        };

        LLMResponse {
            content: text_parts.join(""),
            content_blocks,
            model: api_response.model,
            stop_reason,
            usage: TokenUsage {
                input_tokens: api_response.usage.input_tokens,
                output_tokens: api_response.usage.output_tokens,
            },
        }
    }

    // ── Streaming ───────────────────────────────────────────────────────

    /// Parse a single SSE event line into a `StreamChunk`.
    ///
    /// Responses API SSE format:
    /// ```text
    /// event: response.output_text.delta
    /// data: {"type":"response.output_text.delta","delta":"Hello",...}
    /// ```
    fn parse_sse_line(line: &str) -> Option<LLMResult<StreamChunk>> {
        if !line.starts_with("data: ") {
            return None;
        }

        let json_str = &line[6..];

        // Parse the generic event envelope
        let event: serde_json::Value = match serde_json::from_str(json_str) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!("Failed to parse xAI SSE event: {} - line: {}", e, json_str);
                return None;
            }
        };

        let event_type = event.get("type").and_then(|v| v.as_str()).unwrap_or("");

        match event_type {
            // Text content streaming
            "response.output_text.delta" => {
                let delta = event.get("delta").and_then(|v| v.as_str()).unwrap_or("");
                if delta.is_empty() {
                    return None;
                }
                Some(Ok(StreamChunk::ContentDelta {
                    text: delta.to_string(),
                    index: 0,
                }))
            }

            // Function call: new item added
            "response.output_item.added" => {
                let item = event.get("item")?;
                let item_type = item.get("type").and_then(|v| v.as_str())?;
                if item_type == "function_call" {
                    let call_id = item
                        .get("call_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let name = item
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let index = event
                        .get("output_index")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as usize;
                    Some(Ok(StreamChunk::ToolUseStart {
                        index,
                        id: call_id,
                        name,
                    }))
                } else {
                    None
                }
            }

            // Function call: argument chunks
            "response.function_call_arguments.delta" => {
                let delta = event
                    .get("delta")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if delta.is_empty() {
                    return None;
                }
                let index = event
                    .get("output_index")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as usize;
                Some(Ok(StreamChunk::InputJsonDelta {
                    index,
                    partial_json: delta.to_string(),
                }))
            }

            // Function call arguments complete — finalize the tool use block
            "response.function_call_arguments.done" | "response.output_item.done" => {
                let item_type = if event_type == "response.output_item.done" {
                    event
                        .get("item")
                        .and_then(|i| i.get("type"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                } else {
                    "function_call"
                };

                if item_type == "function_call" {
                    let index = event
                        .get("output_index")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as usize;
                    Some(Ok(StreamChunk::ContentBlockStop { index }))
                } else {
                    None
                }
            }

            // Response lifecycle
            "response.created" => {
                let model = event
                    .get("response")
                    .and_then(|r| r.get("model"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                Some(Ok(StreamChunk::MessageStart {
                    model,
                    input_tokens: 0,
                }))
            }

            "response.completed" => {
                let usage = event.get("response").and_then(|r| r.get("usage"));
                let _input_tokens = usage
                    .and_then(|u| u.get("input_tokens"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32;
                let output_tokens = usage
                    .and_then(|u| u.get("output_tokens"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32;

                // Check if any output items are function calls
                let has_tool_calls = event
                    .get("response")
                    .and_then(|r| r.get("output"))
                    .and_then(|o| o.as_array())
                    .map(|items| {
                        items.iter().any(|i| {
                            i.get("type").and_then(|v| v.as_str()) == Some("function_call")
                        })
                    })
                    .unwrap_or(false);

                let stop_reason = if has_tool_calls {
                    StopReason::ToolUse
                } else {
                    StopReason::EndTurn
                };

                // Emit usage via MessageStart (to update accumulator), then stop
                Some(Ok(StreamChunk::MessageDelta {
                    stop_reason: Some(stop_reason),
                    output_tokens: Some(output_tokens),
                }))
            }

            "response.failed" => {
                let error_msg = event
                    .get("response")
                    .and_then(|r| r.get("error"))
                    .and_then(|e| e.get("message"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown error");
                Some(Err(LLMError::ApiError {
                    status: 500,
                    message: error_msg.to_string(),
                }))
            }

            "response.incomplete" => Some(Ok(StreamChunk::MessageDelta {
                stop_reason: Some(StopReason::MaxTokens),
                output_tokens: None,
            })),

            _ => None,
        }
    }

    // ── Error handling ──────────────────────────────────────────────────

    /// Map xAI error responses to `LLMError`.
    fn handle_error(status: u16, body: &str, retry_after_ms: Option<u64>) -> LLMError {
        // Try OpenAI-style error format first, then xAI-specific
        let message = serde_json::from_str::<serde_json::Value>(body)
            .ok()
            .and_then(|v| {
                v.get("error")
                    .and_then(|e| e.get("message"))
                    .and_then(|m| m.as_str())
                    .map(String::from)
            })
            .unwrap_or_else(|| body.to_string());

        match status {
            401 => LLMError::AuthError(message),
            429 => LLMError::RateLimited {
                retry_after_ms: retry_after_ms.unwrap_or(60000),
            },
            _ => LLMError::ApiError { status, message },
        }
    }

    /// Extract retry-after header value in milliseconds.
    fn parse_retry_after(response: &reqwest::Response) -> Option<u64> {
        response
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<f64>().ok())
            .map(|secs| (secs * 1000.0) as u64)
    }
}

// ── Responses API wire types ────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct XaiRequest {
    model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    instructions: Option<String>,
    input: Vec<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_output_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    stream: bool,
}

// ── Response types ──────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct XaiResponse {
    model: String,
    #[serde(default)]
    output: Vec<XaiOutputItem>,
    #[serde(default)]
    usage: XaiUsage,
    #[serde(default)]
    status: String,
}

#[derive(Debug, Deserialize)]
struct XaiOutputItem {
    #[serde(rename = "type", default)]
    item_type: String,
    /// For message items: content blocks
    #[serde(default)]
    content: Option<Vec<XaiContentPart>>,
    /// For function_call items: the call ID
    call_id: Option<String>,
    /// For function_call items: function name
    name: Option<String>,
    /// For function_call items: JSON arguments string
    arguments: Option<String>,
}

#[derive(Debug, Deserialize)]
struct XaiContentPart {
    #[serde(rename = "type", default)]
    part_type: String,
    text: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct XaiUsage {
    #[serde(default)]
    input_tokens: u32,
    #[serde(default)]
    output_tokens: u32,
}

// ── LLMProvider implementation ──────────────────────────────────────────────

#[async_trait]
impl LLMProvider for XaiClient {
    async fn send_message(&self, request: LLMRequest) -> LLMResult<LLMResponse> {
        let api_request = self.build_request(&request, false);

        let response = self
            .client
            .post(self.responses_url())
            .json(&api_request)
            .send()
            .await?;

        let status = response.status().as_u16();

        if !response.status().is_success() {
            let retry_after = Self::parse_retry_after(&response);
            let body = response.text().await.unwrap_or_default();
            return Err(Self::handle_error(status, &body, retry_after));
        }

        let api_response: XaiResponse = response
            .json()
            .await
            .map_err(|e| LLMError::ParseError(e.to_string()))?;

        Ok(Self::parse_response(api_response))
    }

    async fn send_message_stream(
        &self,
        request: LLMRequest,
    ) -> LLMResult<Pin<Box<dyn Stream<Item = LLMResult<StreamChunk>> + Send>>> {
        let api_request = self.build_request(&request, true);

        let response = self
            .client
            .post(self.responses_url())
            .json(&api_request)
            .send()
            .await?;

        let status = response.status().as_u16();

        if !response.status().is_success() {
            let retry_after = Self::parse_retry_after(&response);
            let body = response.text().await.unwrap_or_default();
            return Err(Self::handle_error(status, &body, retry_after));
        }

        let mut lines = super::stream::safe_line_stream_default(response.bytes_stream());

        let stream = async_stream::stream! {
            // Synthetic ContentBlockStart for accumulator compatibility
            yield Ok(StreamChunk::ContentBlockStart { index: 0 });

            while let Some(line_result) = lines.next().await {
                match line_result {
                    Ok(line) => {
                        let line = line.trim().to_string();
                        if line.is_empty() || line.starts_with("event:") {
                            continue;
                        }

                        if let Some(result) = Self::parse_sse_line(&line) {
                            yield result;
                        }
                    }
                    Err(e) => {
                        yield Err(e);
                        return;
                    }
                }
            }

            yield Ok(StreamChunk::ContentBlockStop { index: 0 });
            yield Ok(StreamChunk::MessageStop);
        };

        Ok(Box::pin(stream))
    }

    fn provider_name(&self) -> &'static str {
        "xai"
    }

    fn model_id(&self) -> &str {
        &self.config.model
    }
}

#[cfg(test)]
mod tests;
