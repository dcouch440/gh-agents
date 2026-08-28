//! LLM request/response types

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur during LLM operations
#[derive(Error, Debug)]
pub enum LLMError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("API error: {status} - {message}")]
    ApiError { status: u16, message: String },

    #[error("Rate limited, retry after {retry_after_ms}ms")]
    RateLimited { retry_after_ms: u64 },

    #[error("Authentication failed: {0}")]
    AuthError(String),

    #[error("Invalid response: {0}")]
    ParseError(String),

    #[error("Stream error: {0}")]
    StreamError(String),

    /// Transport failure while reading a streaming response body.
    ///
    /// Distinct from `StreamError` (which covers stream-protocol faults like a
    /// buffer-cap overflow): this preserves the underlying `reqwest::Error` so
    /// retry classification can still call `is_timeout()`/`is_connect()`.
    /// Uses `#[source]` rather than `#[from]` — `HttpError` owns the
    /// `From<reqwest::Error>` impl.
    #[error("Stream transport error: {0}")]
    StreamTransport(#[source] reqwest::Error),

    #[error("Timeout after {0}ms")]
    Timeout(u64),

    #[error("Max retries ({0}) exceeded")]
    MaxRetriesExceeded(u32),
}

/// Message role in a conversation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
}

/// Message content: either plain text or structured content blocks.
///
/// Serializes as a plain string for text, or as an array for content blocks.
/// This matches the Anthropic API format where `content` can be either.
#[derive(Debug, Clone, PartialEq)]
pub enum MessageContent {
    Text(String),
    Blocks(Vec<ContentBlock>),
}

impl Serialize for MessageContent {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            MessageContent::Text(s) => serializer.serialize_str(s),
            MessageContent::Blocks(blocks) => blocks.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for MessageContent {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        match value {
            serde_json::Value::String(s) => Ok(MessageContent::Text(s)),
            serde_json::Value::Array(_) => {
                let blocks: Vec<ContentBlock> =
                    serde_json::from_value(value).map_err(serde::de::Error::custom)?;
                Ok(MessageContent::Blocks(blocks))
            }
            _ => Err(serde::de::Error::custom("expected string or array")),
        }
    }
}

impl MessageContent {
    /// Get the text content, concatenating text blocks if structured.
    pub fn as_text(&self) -> String {
        match self {
            MessageContent::Text(s) => s.clone(),
            MessageContent::Blocks(blocks) => blocks
                .iter()
                .filter_map(|b| match b {
                    ContentBlock::Text { text } => Some(text.as_str()),
                    _ => None,
                })
                .collect::<Vec<_>>()
                .join(""),
        }
    }
}

/// A single message in a conversation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: MessageContent,
}

impl Message {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: MessageContent::Text(content.into()),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: MessageContent::Text(content.into()),
        }
    }

    /// Create an assistant message with structured content blocks (text + tool use).
    pub fn assistant_with_blocks(blocks: Vec<ContentBlock>) -> Self {
        Self {
            role: Role::Assistant,
            content: MessageContent::Blocks(blocks),
        }
    }

    /// Create a user message with structured content blocks (text + images).
    pub fn user_with_blocks(blocks: Vec<ContentBlock>) -> Self {
        Self {
            role: Role::User,
            content: MessageContent::Blocks(blocks),
        }
    }

    /// Create a user message containing tool results.
    pub fn tool_results(results: Vec<ContentBlock>) -> Self {
        Self {
            role: Role::User,
            content: MessageContent::Blocks(results),
        }
    }

    /// Get the text content of this message.
    pub fn text(&self) -> String {
        self.content.as_text()
    }

    /// Estimate the character count of this message's content.
    pub fn estimated_chars(&self) -> usize {
        match &self.content {
            MessageContent::Text(s) => s.len(),
            MessageContent::Blocks(blocks) => blocks.iter().map(|b| b.estimated_chars()).sum(),
        }
    }
}

/// How much deliberation a reasoning model spends before answering.
///
/// Serializes to the exact strings DeepInfra's `reasoning_effort` parameter
/// accepts. Providers that have no equivalent ignore the field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReasoningEffort {
    /// Reasoning disabled entirely, where the model supports it.
    None,
    Minimal,
    Low,
    Medium,
    High,
    #[serde(rename = "xhigh")]
    XHigh,
    Max,
}

impl ReasoningEffort {
    /// The wire value for this effort level.
    ///
    /// # Examples
    ///
    /// ```
    /// use nexor::llm::ReasoningEffort;
    ///
    /// assert_eq!(ReasoningEffort::XHigh.as_str(), "xhigh");
    /// assert_eq!(ReasoningEffort::None.as_str(), "none");
    /// ```
    pub fn as_str(&self) -> &'static str {
        match self {
            ReasoningEffort::None => "none",
            ReasoningEffort::Minimal => "minimal",
            ReasoningEffort::Low => "low",
            ReasoningEffort::Medium => "medium",
            ReasoningEffort::High => "high",
            ReasoningEffort::XHigh => "xhigh",
            ReasoningEffort::Max => "max",
        }
    }
}

/// Request to send to an LLM
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LLMRequest {
    /// The model to use
    pub model: String,

    /// Messages in the conversation
    pub messages: Vec<Message>,

    /// Optional system prompt
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,

    /// Maximum tokens to generate
    pub max_tokens: u32,

    /// Temperature for sampling (0.0 - 1.0)
    #[serde(default = "default_temperature")]
    pub temperature: f32,

    /// Whether to stream the response
    #[serde(default)]
    pub stream: bool,

    /// Tool definitions available for the model to call
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<Tool>,

    /// Reasoning effort, for providers that support it.
    ///
    /// `None` leaves the parameter off the wire entirely, so the provider
    /// applies its own default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effort: Option<ReasoningEffort>,
}

impl Default for LLMRequest {
    fn default() -> Self {
        Self {
            model: String::new(),
            messages: vec![],
            system: None,
            max_tokens: crate::constants::DEFAULT_MAX_TOKENS_UTILITY,
            temperature: default_temperature(),
            stream: false,
            tools: vec![],
            effort: None,
        }
    }
}

fn default_temperature() -> f32 {
    crate::constants::DEFAULT_TEMPERATURE
}

impl LLMRequest {
    /// Create a new request with sensible defaults
    pub fn new(model: impl Into<String>, messages: Vec<Message>) -> Self {
        Self {
            model: model.into(),
            messages,
            system: None,
            max_tokens: crate::constants::DEFAULT_MAX_TOKENS_UTILITY,
            temperature: default_temperature(),
            stream: false,
            tools: vec![],
            effort: None,
        }
    }

    /// Set the system prompt
    pub fn with_system(mut self, system: impl Into<String>) -> Self {
        self.system = Some(system.into());
        self
    }

    /// Set max tokens
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    /// Enable streaming
    pub fn with_streaming(mut self) -> Self {
        self.stream = true;
        self
    }

    /// Set tool definitions
    pub fn with_tools(mut self, tools: Vec<Tool>) -> Self {
        self.tools = tools;
        self
    }

    /// Set the reasoning effort.
    pub fn with_effort(mut self, effort: ReasoningEffort) -> Self {
        self.effort = Some(effort);
        self
    }
}

/// A tool definition for the Anthropic API
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tool {
    /// Tool name (e.g. "create_agent")
    pub name: String,
    /// Human-readable description of what the tool does
    pub description: String,
    /// JSON Schema describing the tool's input parameters
    pub input_schema: serde_json::Value,
}

/// A content block in an LLM message (text, tool use, tool result, or image).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    /// Text content
    Text { text: String },
    /// Tool use request from the model
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    /// Tool result sent back to the model
    ToolResult {
        tool_use_id: String,
        content: String,
    },
    /// Image content for vision-capable models.
    ///
    /// Serializes to Anthropic format by default via serde tags. The xAI adapter
    /// handles its own serialization in `convert_message`.
    Image { source: ImageSource },
}

/// Image source data for vision content blocks.
///
/// Serializes to Anthropic's format:
/// ```json
/// { "type": "base64", "media_type": "image/png", "data": "..." }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageSource {
    /// Source type — always `"base64"` for inline images.
    #[serde(rename = "type")]
    pub source_type: String,
    /// MIME type (e.g. `"image/png"`).
    pub media_type: String,
    /// Base64-encoded image bytes.
    pub data: String,
}

impl ContentBlock {
    /// Create an image content block from base64-encoded PNG data.
    pub fn image_png_base64(data: String) -> Self {
        ContentBlock::Image {
            source: ImageSource {
                source_type: "base64".to_string(),
                media_type: "image/png".to_string(),
                data,
            },
        }
    }

    /// Estimate the character count of this content block.
    pub fn estimated_chars(&self) -> usize {
        match self {
            ContentBlock::Text { text } => text.len(),
            ContentBlock::ToolUse { id, name, input } => {
                id.len() + name.len() + input.to_string().len()
            }
            ContentBlock::ToolResult {
                tool_use_id,
                content,
            } => tool_use_id.len() + content.len(),
            ContentBlock::Image { source } => source.data.len(),
        }
    }
}

/// Reason the model stopped generating
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    /// Model reached natural end
    EndTurn,
    /// Max tokens limit reached
    MaxTokens,
    /// Stop sequence encountered
    StopSequence,
    /// Tool use requested (for future)
    ToolUse,
}

/// Token usage information
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    /// Portion of `input_tokens` served from the provider's prompt cache.
    ///
    /// A SUBSET of `input_tokens`, not an addition to it — OpenAI-compatible
    /// APIs report `prompt_tokens` inclusive of cached tokens. Billing must
    /// therefore charge `input_tokens - cached_input_tokens` at the uncached
    /// rate. Providers without prompt caching leave this zero.
    #[serde(default)]
    pub cached_input_tokens: u32,
}

impl TokenUsage {
    pub fn total(&self) -> u32 {
        self.input_tokens + self.output_tokens
    }

    /// Input tokens billed at the full (uncached) rate.
    ///
    /// Saturating: a provider reporting more cached than total input must not
    /// wrap into an enormous charge.
    pub fn uncached_input_tokens(&self) -> u32 {
        self.input_tokens.saturating_sub(self.cached_input_tokens)
    }
}

/// Complete response from an LLM
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LLMResponse {
    /// The generated text content (concatenation of all text blocks)
    pub content: String,

    /// Structured content blocks (text and tool use)
    #[serde(default)]
    pub content_blocks: Vec<ContentBlock>,

    /// Model that generated the response
    pub model: String,

    /// Why generation stopped
    pub stop_reason: StopReason,

    /// Token usage for this request
    pub usage: TokenUsage,
}

/// A chunk from a streaming response
#[derive(Debug, Clone, PartialEq)]
pub enum StreamChunk {
    /// Text content delta
    ContentDelta {
        /// The text fragment
        text: String,
        /// Index of the content block (for multi-block responses)
        index: usize,
    },

    /// Message started event
    MessageStart {
        /// The model being used
        model: String,
        /// Input tokens used for this request
        input_tokens: u32,
    },

    /// Content block started
    ContentBlockStart {
        /// Index of the content block
        index: usize,
    },

    /// Content block finished
    ContentBlockStop {
        /// Index of the content block
        index: usize,
    },

    /// Message delta (includes usage at end)
    MessageDelta {
        /// Stop reason when complete
        stop_reason: Option<StopReason>,
        /// Output tokens used so far
        output_tokens: Option<u32>,
    },

    /// Tool use content block started (streaming)
    ToolUseStart {
        /// Index of the content block
        index: usize,
        /// Tool use ID
        id: String,
        /// Tool name
        name: String,
    },

    /// Partial JSON input for a tool use (streaming)
    InputJsonDelta {
        /// Index of the content block
        index: usize,
        /// Partial JSON string
        partial_json: String,
    },

    /// Usage totals reported in a dedicated frame.
    ///
    /// OpenAI-compatible providers deliver usage in a final chunk rather than
    /// on the message-delta event, and only they report cached prompt tokens.
    /// Additive so providers that have no such frame never emit it.
    UsageUpdate {
        /// Prompt tokens, inclusive of any cached portion.
        input_tokens: Option<u32>,
        /// Completion tokens, inclusive of reasoning tokens.
        output_tokens: Option<u32>,
        /// Portion of `input_tokens` served from the prompt cache.
        cached_input_tokens: Option<u32>,
    },

    /// Final message stop event
    MessageStop,

    /// Ping event (for keepalive)
    Ping,
}

/// Raw SSE event from the stream
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct SSEEvent {
    /// Event type (message_start, content_block_delta, etc.)
    #[serde(rename = "type")]
    pub event_type: String,

    /// Event data (varies by type)
    #[serde(flatten)]
    pub data: serde_json::Value,
}

/// State for accumulating a single tool use block during streaming
#[derive(Debug, Clone)]
struct ToolUseAccumulator {
    id: String,
    name: String,
    input_json: String,
}

/// Convert a completed tool-use accumulator into a content block.
///
/// Malformed or empty JSON becomes an empty object rather than an error: the
/// model gets a tool call it can see failed, instead of the call vanishing.
fn finish_tool_use(tool: ToolUseAccumulator) -> ContentBlock {
    let input = serde_json::from_str(&tool.input_json)
        .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
    ContentBlock::ToolUse {
        id: tool.id,
        name: tool.name,
        input,
    }
}

/// Accumulated stream state for building final response
#[derive(Debug, Default)]
pub struct StreamAccumulator {
    pub content: String,
    pub content_blocks: Vec<ContentBlock>,
    pub model: Option<String>,
    pub stop_reason: Option<StopReason>,
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
    pub cached_input_tokens: Option<u32>,
    /// In-progress tool use blocks, keyed by content-block index.
    ///
    /// Keyed rather than a single slot because OpenAI-compatible streams may
    /// interleave argument deltas for several tool calls: `open 0, open 1,
    /// args 0, args 1`. A single slot would finalize call 0 with empty
    /// arguments and then feed its arguments into call 1. `BTreeMap` also
    /// keeps the finalization order deterministic.
    tool_uses: std::collections::BTreeMap<usize, ToolUseAccumulator>,
}

impl StreamAccumulator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply a chunk to the accumulator
    pub fn apply(&mut self, chunk: &StreamChunk) {
        match chunk {
            StreamChunk::ContentDelta { text, .. } => {
                self.content.push_str(text);
            }
            StreamChunk::MessageStart {
                model,
                input_tokens,
            } => {
                // Only overwrite with a real name. A later frame that omits
                // the model would otherwise blank it, and an empty model id
                // falls through every pricing branch to the generic fallback
                // and lands empty in the token ledger.
                if !model.is_empty() {
                    self.model = Some(model.clone());
                }
                self.input_tokens = Some(*input_tokens);
            }
            StreamChunk::MessageDelta {
                stop_reason,
                output_tokens,
            } => {
                if let Some(reason) = stop_reason {
                    self.stop_reason = Some(*reason);
                }
                if let Some(tokens) = output_tokens {
                    self.output_tokens = Some(*tokens);
                }
            }
            StreamChunk::UsageUpdate {
                input_tokens,
                output_tokens,
                cached_input_tokens,
            } => {
                if let Some(t) = input_tokens {
                    self.input_tokens = Some(*t);
                }
                if let Some(t) = output_tokens {
                    self.output_tokens = Some(*t);
                }
                if let Some(t) = cached_input_tokens {
                    self.cached_input_tokens = Some(*t);
                }
            }
            StreamChunk::ToolUseStart { index, id, name } => {
                self.tool_uses.insert(
                    *index,
                    ToolUseAccumulator {
                        id: id.clone(),
                        name: name.clone(),
                        input_json: String::new(),
                    },
                );
            }
            StreamChunk::InputJsonDelta {
                index,
                partial_json,
            } => {
                if let Some(tool) = self.tool_uses.get_mut(index) {
                    tool.input_json.push_str(partial_json);
                }
            }
            StreamChunk::ContentBlockStop { index } => {
                // Finalize the tool use at this index, if there is one. Text
                // blocks also emit a stop and are simply absent from the map.
                if let Some(tool) = self.tool_uses.remove(index) {
                    self.content_blocks.push(finish_tool_use(tool));
                }
            }
            _ => {}
        }
    }

    /// Build the final response (returns None if incomplete)
    pub fn build(mut self) -> Option<LLMResponse> {
        // Drain tool uses that never received a stop event. Not every
        // provider emits one per block, and dropping them here would lose
        // the tool call entirely.
        let leftovers: Vec<_> = std::mem::take(&mut self.tool_uses)
            .into_values()
            .map(finish_tool_use)
            .collect();

        // Add any accumulated text as a content block
        let mut blocks = self.content_blocks;
        blocks.extend(leftovers);
        if !self.content.is_empty() {
            blocks.insert(
                0,
                ContentBlock::Text {
                    text: self.content.clone(),
                },
            );
        }

        // If we accumulated tool use blocks, the stop reason MUST be ToolUse
        // regardless of what the provider's completion event reported. This
        // protects against providers (e.g. xAI Responses API) whose
        // response.completed event doesn't reliably signal tool calls.
        let has_tool_blocks = blocks
            .iter()
            .any(|b| matches!(b, ContentBlock::ToolUse { .. }));
        let stop_reason = if has_tool_blocks {
            StopReason::ToolUse
        } else {
            self.stop_reason?
        };

        Some(LLMResponse {
            content: self.content,
            content_blocks: blocks,
            model: self.model?,
            stop_reason,
            usage: TokenUsage {
                input_tokens: self.input_tokens.unwrap_or(0),
                output_tokens: self.output_tokens.unwrap_or(0),
                cached_input_tokens: self.cached_input_tokens.unwrap_or(0),
            },
        })
    }
}

#[cfg(test)]
mod tests;
