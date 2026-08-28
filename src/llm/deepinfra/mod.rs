//! DeepInfra provider, speaking the OpenAI chat-completions wire format.
//!
//! Targets `{base}/chat/completions`. Supports function calling, streaming
//! (SSE), prompt-cache accounting, and the `reasoning_effort` parameter that
//! separates our model tiers — on DeepInfra all three tiers are the same
//! model and differ only by how much deliberation they are given.
//!
//! HTTP and SSE plumbing come from `SseHttpProvider`.

use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};

use super::provider::LLMResult;
use super::sse_provider::{SseHttpProvider, SseProviderAdapter};
use super::types::{
    ContentBlock, LLMError, LLMRequest, LLMResponse, Message, MessageContent, ReasoningEffort,
    Role, StopReason, StreamChunk, TokenUsage, Tool,
};

#[cfg(test)]
mod tests;

// ── Config ──────────────────────────────────────────────────────────────────

/// Configuration for the DeepInfra client.
#[derive(Debug, Clone)]
pub struct DeepInfraConfig {
    /// DeepInfra API key (Bearer token).
    pub api_key: String,
    /// Base URL, without the `/chat/completions` suffix.
    pub base_url: String,
    /// Model id, e.g. `deepseek-ai/DeepSeek-V4-Flash-0731`.
    pub model: String,
    /// Whole-request timeout in seconds.
    pub timeout_secs: u64,
    /// Per-read timeout in seconds; guards against a stalled stream while the
    /// whole-request timeout stays generous enough to survive queueing.
    pub read_timeout_secs: u64,
    /// Effort applied when a request does not specify its own.
    pub default_effort: Option<ReasoningEffort>,
}

impl DeepInfraConfig {
    /// Build from `DEEPINFRA_API_KEY` (required) and `DEEPINFRA_MODEL` (optional).
    ///
    /// `model` is only a fallback for a request that names no model. Every
    /// production path sets `LLMRequest::model` from the tier constants, so
    /// this is not a global override.
    pub fn from_env() -> Result<Self, LLMError> {
        let api_key = std::env::var(crate::constants::ENV_DEEPINFRA_API_KEY).map_err(|_| {
            LLMError::AuthError(format!(
                "{} not set",
                crate::constants::ENV_DEEPINFRA_API_KEY
            ))
        })?;
        let model = std::env::var(crate::constants::ENV_DEEPINFRA_MODEL)
            .unwrap_or_else(|_| crate::constants::DEEPINFRA_DEFAULT_MODEL.to_string());

        Ok(Self {
            api_key,
            base_url: crate::constants::DEEPINFRA_DEFAULT_BASE_URL.to_string(),
            model,
            timeout_secs: crate::constants::DEEPINFRA_CHAT_TIMEOUT_SECS,
            read_timeout_secs: crate::constants::DEEPINFRA_READ_TIMEOUT_SECS,
            default_effort: None,
        })
    }

    /// Set the model.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Set the base URL (used by tests to point at a mock server).
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    /// Set the whole-request timeout.
    pub fn with_timeout_secs(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }

    /// Set the effort applied when a request does not carry its own.
    pub fn with_default_effort(mut self, effort: ReasoningEffort) -> Self {
        self.default_effort = Some(effort);
        self
    }
}

// ── Adapter ─────────────────────────────────────────────────────────────────

/// DeepInfra-specific adapter for `SseHttpProvider`.
#[derive(Clone)]
pub struct DeepInfraAdapter {
    pub(crate) config: DeepInfraConfig,
}

/// The DeepInfra client.
pub type DeepInfraClient = SseHttpProvider<DeepInfraAdapter>;

impl DeepInfraClient {
    /// Create a client from config.
    pub fn with_config(config: DeepInfraConfig) -> Result<Self, LLMError> {
        if config.api_key.is_empty() {
            return Err(LLMError::AuthError(
                "DeepInfra API key cannot be empty".to_string(),
            ));
        }
        SseHttpProvider::new(DeepInfraAdapter { config })
    }

    /// Create a client from environment variables.
    pub fn from_env() -> Result<Self, LLMError> {
        Self::with_config(DeepInfraConfig::from_env()?)
    }
}

impl SseProviderAdapter for DeepInfraAdapter {
    fn provider_name(&self) -> &'static str {
        "deepinfra"
    }

    fn model_id(&self) -> &str {
        &self.config.model
    }

    fn endpoint_url(&self) -> String {
        format!("{}/chat/completions", self.config.base_url)
    }

    fn default_headers(&self) -> Result<HeaderMap, LLMError> {
        let mut headers = HeaderMap::new();
        headers.insert(
            "authorization",
            HeaderValue::from_str(&format!("Bearer {}", self.config.api_key))
                .map_err(|_| LLMError::AuthError("Invalid DeepInfra API key format".to_string()))?,
        );
        headers.insert("content-type", HeaderValue::from_static("application/json"));
        Ok(headers)
    }

    fn timeout_secs(&self) -> u64 {
        self.config.timeout_secs
    }

    fn read_timeout_secs(&self) -> Option<u64> {
        Some(self.config.read_timeout_secs)
    }

    fn build_request_body(&self, request: &LLMRequest, stream: bool) -> serde_json::Value {
        let model = if request.model.is_empty() {
            self.config.model.clone()
        } else {
            request.model.clone()
        };

        let mut messages = Vec::new();
        if let Some(ref system) = request.system {
            messages.push(serde_json::json!({ "role": "system", "content": system }));
        }
        for m in &request.messages {
            convert_message(m, &mut messages);
        }

        let effort = request.effort.or(self.config.default_effort);

        let body = DeepInfraRequest {
            model,
            messages,
            max_tokens: Some(request.max_tokens),
            temperature: Some(request.temperature),
            stream,
            // Without this an OpenAI-compatible stream omits usage entirely,
            // and every streamed call would bill as zero tokens.
            stream_options: stream.then(|| serde_json::json!({ "include_usage": true })),
            tools: (!request.tools.is_empty())
                .then(|| request.tools.iter().map(function_tool_json).collect()),
            reasoning_effort: effort.map(|e| e.as_str()),
        };

        serde_json::to_value(body).unwrap_or_else(|_| serde_json::json!({}))
    }

    fn parse_response(&self, body: &[u8]) -> Result<LLMResponse, LLMError> {
        let parsed: OaiResponse = serde_json::from_slice(body)
            .map_err(|e| LLMError::ParseError(format!("DeepInfra response: {}", e)))?;

        let choice = parsed
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| LLMError::ParseError("DeepInfra returned no choices".to_string()))?;

        let text = choice.message.content.unwrap_or_default();
        let mut blocks = Vec::new();
        if !text.is_empty() {
            blocks.push(ContentBlock::Text { text: text.clone() });
        }
        for call in choice.message.tool_calls.unwrap_or_default() {
            blocks.push(ContentBlock::ToolUse {
                id: call.id,
                name: call.function.name,
                input: parse_tool_arguments(&call.function.arguments),
            });
        }

        let has_tools = blocks
            .iter()
            .any(|b| matches!(b, ContentBlock::ToolUse { .. }));
        let stop_reason = if has_tools {
            StopReason::ToolUse
        } else {
            map_finish_reason(choice.finish_reason.as_deref())
        };

        Ok(LLMResponse {
            content: text,
            content_blocks: blocks,
            model: parsed.model.unwrap_or_else(|| self.config.model.clone()),
            stop_reason,
            usage: parsed.usage.map(to_token_usage).unwrap_or_default(),
        })
    }

    fn parse_sse_line(&self, _line: &str) -> Option<LLMResult<StreamChunk>> {
        // One OpenAI event can yield several internal chunks; see
        // `parse_sse_events`.
        None
    }

    fn parse_sse_events(&self, line: &str) -> Vec<LLMResult<StreamChunk>> {
        parse_openai_sse_line(line)
    }

    fn handle_error(&self, status: u16, body: &str, retry_after_ms: Option<u64>) -> LLMError {
        let message = extract_error_message(body);
        match status {
            401 | 403 => LLMError::AuthError(format!("DeepInfra auth failed: {}", message)),
            429 => LLMError::RateLimited {
                // DeepInfra queues rather than rejecting by default, so a 429
                // means a real capacity limit; back off properly when it does
                // not tell us how long.
                retry_after_ms: retry_after_ms.unwrap_or(60_000),
            },
            _ => LLMError::ApiError { status, message },
        }
    }

    fn pre_stream_events(&self) -> Vec<StreamChunk> {
        // OpenAI streams have no message-start event, but the accumulator
        // needs a model before it will build a response.
        vec![StreamChunk::MessageStart {
            model: self.config.model.clone(),
            input_tokens: 0,
        }]
    }

    fn post_stream_events(&self) -> Vec<StreamChunk> {
        // Belt and braces: a stream cut short before `[DONE]` still terminates.
        vec![StreamChunk::MessageStop]
    }
}

// ── Message conversion ──────────────────────────────────────────────────────

/// Expand one internal message into zero or more OpenAI messages.
///
/// The shapes do not map one-to-one. Tool results arrive here inside a *user*
/// message (the Anthropic convention this codebase follows), but OpenAI wants
/// each one as its own `role: "tool"` message, so a single internal message
/// can produce several.
fn convert_message(message: &Message, out: &mut Vec<serde_json::Value>) {
    let role = match message.role {
        Role::User => "user",
        Role::Assistant => "assistant",
    };

    match &message.content {
        MessageContent::Text(text) => {
            out.push(serde_json::json!({ "role": role, "content": text }));
        }
        MessageContent::Blocks(blocks) => {
            let mut parts: Vec<serde_json::Value> = Vec::new();
            let mut tool_calls: Vec<serde_json::Value> = Vec::new();

            for block in blocks {
                match block {
                    ContentBlock::Text { text } => {
                        parts.push(serde_json::json!({ "type": "text", "text": text }));
                    }
                    ContentBlock::Image { source } => {
                        parts.push(serde_json::json!({
                            "type": "image_url",
                            "image_url": {
                                "url": format!("data:{};base64,{}", source.media_type, source.data)
                            }
                        }));
                    }
                    ContentBlock::ToolUse { id, name, input } => {
                        tool_calls.push(serde_json::json!({
                            "id": id,
                            "type": "function",
                            "function": {
                                "name": name,
                                // OpenAI carries arguments as a JSON *string*.
                                "arguments": input.to_string(),
                            }
                        }));
                    }
                    ContentBlock::ToolResult {
                        tool_use_id,
                        content,
                    } => {
                        // Flush anything accumulated so far, so ordering with
                        // respect to the tool results is preserved.
                        flush_parts(role, &mut parts, &mut tool_calls, out);
                        out.push(serde_json::json!({
                            "role": "tool",
                            "tool_call_id": tool_use_id,
                            "content": content,
                        }));
                    }
                }
            }
            flush_parts(role, &mut parts, &mut tool_calls, out);
        }
    }
}

/// Emit the pending content parts and tool calls as one message, if any.
fn flush_parts(
    role: &str,
    parts: &mut Vec<serde_json::Value>,
    tool_calls: &mut Vec<serde_json::Value>,
    out: &mut Vec<serde_json::Value>,
) {
    if parts.is_empty() && tool_calls.is_empty() {
        return;
    }
    let mut msg = serde_json::Map::new();
    msg.insert("role".into(), serde_json::json!(role));

    // A single text part collapses to a bare string, which is what every
    // OpenAI-compatible server accepts; the array form is only needed for
    // multimodal content.
    let content = match parts.len() {
        0 => serde_json::Value::Null,
        1 => match parts[0].get("type").and_then(|t| t.as_str()) {
            Some("text") => parts[0]
                .get("text")
                .cloned()
                .unwrap_or(serde_json::Value::Null),
            _ => serde_json::Value::Array(std::mem::take(parts)),
        },
        _ => serde_json::Value::Array(std::mem::take(parts)),
    };
    msg.insert("content".into(), content);

    if !tool_calls.is_empty() {
        msg.insert(
            "tool_calls".into(),
            serde_json::Value::Array(std::mem::take(tool_calls)),
        );
    }
    parts.clear();
    tool_calls.clear();
    out.push(serde_json::Value::Object(msg));
}

/// Serialize a tool into OpenAI's nested function shape.
fn function_tool_json(tool: &Tool) -> serde_json::Value {
    serde_json::json!({
        "type": "function",
        "function": {
            "name": tool.name,
            "description": tool.description,
            "parameters": tool.input_schema,
        }
    })
}

/// Parse tool-call arguments, tolerating malformed or empty JSON.
///
/// A model that emits broken arguments should get a tool result saying so,
/// not have the whole response fail to parse.
fn parse_tool_arguments(raw: &str) -> serde_json::Value {
    if raw.trim().is_empty() {
        return serde_json::json!({});
    }
    serde_json::from_str(raw).unwrap_or_else(|_| serde_json::json!({}))
}

/// Map an OpenAI `finish_reason` onto the internal stop reason.
fn map_finish_reason(reason: Option<&str>) -> StopReason {
    match reason {
        Some("tool_calls") | Some("function_call") => StopReason::ToolUse,
        Some("length") => StopReason::MaxTokens,
        Some("stop") => StopReason::EndTurn,
        Some("content_filter") => StopReason::ContentFiltered,
        _ => StopReason::EndTurn,
    }
}

/// Pull a human-readable message out of an error body, falling back to the
/// raw body when it is not the shape we expect.
fn extract_error_message(body: &str) -> String {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|v| {
            v.get("error")
                .and_then(|e| e.get("message").or(Some(e)))
                .and_then(|m| m.as_str().map(str::to_string))
                .or_else(|| v.get("detail").and_then(|d| d.as_str().map(str::to_string)))
        })
        .unwrap_or_else(|| body.chars().take(500).collect())
}

/// Convert OpenAI usage into internal usage.
///
/// `prompt_tokens` is inclusive of the cached portion, so `cached_input_tokens`
/// is stored as the subset it is; billing subtracts it rather than adding.
fn to_token_usage(u: OaiUsage) -> TokenUsage {
    TokenUsage {
        input_tokens: u.prompt_tokens.unwrap_or(0),
        output_tokens: u.completion_tokens.unwrap_or(0),
        cached_input_tokens: u
            .prompt_tokens_details
            .and_then(|d| d.cached_tokens)
            .unwrap_or(0),
    }
}

// ── SSE parsing ─────────────────────────────────────────────────────────────

/// Parse one `data: ...` line into zero or more internal chunks.
pub(crate) fn parse_openai_sse_line(line: &str) -> Vec<LLMResult<StreamChunk>> {
    let data = match line.strip_prefix("data:") {
        Some(rest) => rest.trim(),
        None => return vec![],
    };
    if data.is_empty() {
        return vec![];
    }
    if data == "[DONE]" {
        return vec![Ok(StreamChunk::MessageStop)];
    }

    let chunk: OaiStreamChunk = match serde_json::from_str(data) {
        Ok(c) => c,
        // A malformed keepalive or comment frame must not kill the stream.
        Err(_) => return vec![],
    };

    let mut out = Vec::new();

    if let Some(choice) = chunk.choices.into_iter().next() {
        if let Some(delta) = choice.delta {
            if let Some(text) = delta.content {
                if !text.is_empty() {
                    out.push(Ok(StreamChunk::ContentDelta { text, index: 0 }));
                }
            }
            // `reasoning_content` is the model's private deliberation. It is
            // deliberately NOT surfaced as a ContentDelta: it would be
            // concatenated into `content` and returned to the user as though
            // it were the answer.
            for call in delta.tool_calls.unwrap_or_default() {
                let index = call.index.unwrap_or(0);
                let id = call.id.clone();
                let name = call.function.as_ref().and_then(|f| f.name.clone());
                // Opened when *either* field appears, not only when both do in
                // the same frame. Some OpenAI-compatible backends send the id
                // first and the name in a later frame; requiring both together
                // dropped the call and every argument delta with it, while
                // `finish_reason` still reported `tool_calls`.
                if id.is_some() || name.is_some() {
                    out.push(Ok(StreamChunk::ToolUseStart {
                        index,
                        id: id.unwrap_or_default(),
                        name: name.unwrap_or_default(),
                    }));
                }
                if let Some(args) = call.function.and_then(|f| f.arguments) {
                    if !args.is_empty() {
                        out.push(Ok(StreamChunk::InputJsonDelta {
                            index,
                            partial_json: args,
                        }));
                    }
                }
            }
        }
        if let Some(reason) = choice.finish_reason {
            out.push(Ok(StreamChunk::MessageDelta {
                stop_reason: Some(map_finish_reason(Some(&reason))),
                output_tokens: None,
            }));
        }
    }

    if let Some(u) = chunk.usage {
        out.push(Ok(StreamChunk::UsageUpdate {
            input_tokens: u.prompt_tokens,
            output_tokens: u.completion_tokens,
            cached_input_tokens: u.prompt_tokens_details.and_then(|d| d.cached_tokens),
        }));
    }

    out
}

// ── Wire types ──────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct DeepInfraRequest {
    model: String,
    messages: Vec<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream_options: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning_effort: Option<&'static str>,
}

#[derive(Debug, Deserialize)]
struct OaiResponse {
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    choices: Vec<OaiChoice>,
    #[serde(default)]
    usage: Option<OaiUsage>,
}

#[derive(Debug, Deserialize)]
struct OaiChoice {
    message: OaiMessage,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OaiMessage {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<OaiToolCall>>,
}

#[derive(Debug, Deserialize)]
struct OaiToolCall {
    id: String,
    function: OaiFunction,
}

#[derive(Debug, Deserialize)]
struct OaiFunction {
    name: String,
    #[serde(default)]
    arguments: String,
}

#[derive(Debug, Deserialize)]
struct OaiUsage {
    #[serde(default)]
    prompt_tokens: Option<u32>,
    #[serde(default)]
    completion_tokens: Option<u32>,
    #[serde(default)]
    prompt_tokens_details: Option<OaiPromptDetails>,
}

#[derive(Debug, Deserialize)]
struct OaiPromptDetails {
    #[serde(default)]
    cached_tokens: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct OaiStreamChunk {
    #[serde(default)]
    choices: Vec<OaiStreamChoice>,
    #[serde(default)]
    usage: Option<OaiUsage>,
}

#[derive(Debug, Deserialize)]
struct OaiStreamChoice {
    #[serde(default)]
    delta: Option<OaiDelta>,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OaiDelta {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<OaiDeltaToolCall>>,
}

#[derive(Debug, Deserialize)]
struct OaiDeltaToolCall {
    #[serde(default)]
    index: Option<usize>,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    function: Option<OaiDeltaFunction>,
}

#[derive(Debug, Clone, Deserialize)]
struct OaiDeltaFunction {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    arguments: Option<String>,
}
