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

/// A single message in a conversation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
}

impl Message {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
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
            max_tokens: 4096,
            temperature: default_temperature(),
            stream: false,
            tools: vec![],
        }
    }
}

fn default_temperature() -> f32 {
    0.7
}

impl LLMRequest {
    /// Create a new request with sensible defaults
    pub fn new(model: impl Into<String>, messages: Vec<Message>) -> Self {
        Self {
            model: model.into(),
            messages,
            system: None,
            max_tokens: 4096,
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
            StreamChunk::MessageStart {
                model,
                input_tokens,
            } => {
                self.model = Some(model.clone());
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
                    let input = serde_json::from_str(&tool.input_json)
                        .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
                    self.content_blocks.push(ContentBlock::ToolUse {
                        id: tool.id,
                        name: tool.name,
                        input,
                    });
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
            blocks.insert(
                0,
                ContentBlock::Text {
                    text: self.content.clone(),
                },
            );
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
mod tests {
    use super::*;

    #[test]
    fn message_user_creates_user_role() {
        let msg = Message::user("Hello");
        assert_eq!(msg.role, Role::User);
        assert_eq!(msg.content, "Hello");
    }

    #[test]
    fn message_assistant_creates_assistant_role() {
        let msg = Message::assistant("Hi there!");
        assert_eq!(msg.role, Role::Assistant);
        assert_eq!(msg.content, "Hi there!");
    }

    #[test]
    fn request_builder_works() {
        let request = LLMRequest::new("claude-3", vec![Message::user("Hi")])
            .with_system("You are helpful")
            .with_max_tokens(1000)
            .with_streaming();

        assert_eq!(request.model, "claude-3");
        assert_eq!(request.system, Some("You are helpful".to_string()));
        assert_eq!(request.max_tokens, 1000);
        assert!(request.stream);
    }

    #[test]
    fn request_defaults_are_sensible() {
        let request = LLMRequest::new("claude-3", vec![]);
        assert_eq!(request.max_tokens, 4096);
        assert!((request.temperature - 0.7).abs() < f32::EPSILON);
        assert!(!request.stream);
        assert!(request.system.is_none());
    }

    #[test]
    fn token_usage_total() {
        let usage = TokenUsage {
            input_tokens: 100,
            output_tokens: 50,
        };
        assert_eq!(usage.total(), 150);
    }

    #[test]
    fn accumulator_builds_response() {
        let mut acc = StreamAccumulator::new();
        acc.apply(&StreamChunk::MessageStart {
            model: "claude-3".to_string(),
            input_tokens: 10,
        });
        acc.apply(&StreamChunk::ContentDelta {
            text: "Hello ".to_string(),
            index: 0,
        });
        acc.apply(&StreamChunk::ContentDelta {
            text: "world!".to_string(),
            index: 0,
        });
        acc.apply(&StreamChunk::MessageDelta {
            stop_reason: Some(StopReason::EndTurn),
            output_tokens: Some(5),
        });

        let response = acc.build().unwrap();
        assert_eq!(response.content, "Hello world!");
        assert_eq!(response.model, "claude-3");
        assert_eq!(response.stop_reason, StopReason::EndTurn);
    }

    #[test]
    fn accumulator_returns_none_if_incomplete() {
        let acc = StreamAccumulator::new();
        assert!(acc.build().is_none());
    }

    #[test]
    fn message_serialization_works() {
        let msg = Message::user("Hello");
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"role\":\"user\""));
        assert!(json.contains("\"content\":\"Hello\""));

        let parsed: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, msg);
    }

    #[test]
    fn request_serialization_works() {
        let request =
            LLMRequest::new("claude-3", vec![Message::user("Hi")]).with_system("Be helpful");

        let json = serde_json::to_string(&request).unwrap();
        let parsed: LLMRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, request);
    }
}
