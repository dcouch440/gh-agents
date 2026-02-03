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
                let blocks: Vec<ContentBlock> = serde_json::from_value(value).map_err(serde::de::Error::custom)?;
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

/// A content block in an LLM response (text or tool use)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    /// Text content
    Text { text: String },
    /// Tool use request from the model
    ToolUse { id: String, name: String, input: serde_json::Value },
    /// Tool result sent back to the model
    ToolResult { tool_use_id: String, content: String },
}

impl ContentBlock {
    /// Estimate the character count of this content block.
    pub fn estimated_chars(&self) -> usize {
        match self {
            ContentBlock::Text { text } => text.len(),
            ContentBlock::ToolUse { id, name, input } => id.len() + name.len() + input.to_string().len(),
            ContentBlock::ToolResult { tool_use_id, content } => tool_use_id.len() + content.len(),
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
}

impl TokenUsage {
    pub fn total(&self) -> u32 {
        self.input_tokens + self.output_tokens
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

/// Accumulated stream state for building final response
#[derive(Debug, Default)]
pub struct StreamAccumulator {
    pub content: String,
    pub content_blocks: Vec<ContentBlock>,
    pub model: Option<String>,
    pub stop_reason: Option<StopReason>,
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
    /// In-progress tool use block being assembled from streaming chunks
    current_tool_use: Option<ToolUseAccumulator>,
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
            StreamChunk::MessageStart { model, input_tokens } => {
                self.model = Some(model.clone());
                self.input_tokens = Some(*input_tokens);
            }
            StreamChunk::MessageDelta { stop_reason, output_tokens } => {
                if let Some(reason) = stop_reason {
                    self.stop_reason = Some(*reason);
                }
                if let Some(tokens) = output_tokens {
                    self.output_tokens = Some(*tokens);
                }
            }
            StreamChunk::ToolUseStart { id, name, .. } => {
                self.current_tool_use = Some(ToolUseAccumulator {
                    id: id.clone(),
                    name: name.clone(),
                    input_json: String::new(),
                });
            }
            StreamChunk::InputJsonDelta { partial_json, .. } => {
                if let Some(ref mut tool) = self.current_tool_use {
                    tool.input_json.push_str(partial_json);
                }
            }
            StreamChunk::ContentBlockStop { .. } => {
                // Finalize any in-progress tool use block
                if let Some(tool) = self.current_tool_use.take() {
                    let input = serde_json::from_str(&tool.input_json).unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
                    self.content_blocks.push(ContentBlock::ToolUse { id: tool.id, name: tool.name, input });
                }
            }
            _ => {}
        }
    }

    /// Build the final response (returns None if incomplete)
    pub fn build(self) -> Option<LLMResponse> {
        // Add any accumulated text as a content block
        let mut blocks = self.content_blocks;
        if !self.content.is_empty() {
            blocks.insert(0, ContentBlock::Text { text: self.content.clone() });
        }

        Some(LLMResponse {
            content: self.content,
            content_blocks: blocks,
            model: self.model?,
            stop_reason: self.stop_reason?,
            usage: TokenUsage {
                input_tokens: self.input_tokens.unwrap_or(0),
                output_tokens: self.output_tokens.unwrap_or(0),
            },
        })
    }
}


#[cfg(test)]
mod tests;
