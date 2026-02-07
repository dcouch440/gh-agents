//! Ollama API client implementation for local LLM inference.
//!
//! Talks to Ollama's `/api/chat` endpoint on localhost. Supports both
//! streaming (newline-delimited JSON) and non-streaming modes, tool calling,
//! and maps Ollama's token counts to the common `TokenUsage` type.

use async_trait::async_trait;
use futures::{Stream, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use std::time::Duration;

use super::provider::{LLMProvider, LLMResult};
use super::types::{
    ContentBlock, LLMError, LLMRequest, LLMResponse, Message, MessageContent, Role, StopReason,
    StreamChunk, TokenUsage, Tool,
};

/// Configuration for the Ollama client.
#[derive(Debug, Clone)]
pub struct OllamaConfig {
    /// Base URL for the Ollama API (default: http://localhost:11434)
    pub base_url: String,

    /// Default model to use (e.g. "llama3.1:34b", "mistral:latest")
    pub model: String,

    /// Request timeout in seconds (default: 300 — local models are slower)
    pub timeout_secs: u64,
}

impl OllamaConfig {
    /// Create config from environment variables.
    ///
    /// Reads `OLLAMA_BASE_URL` (optional, defaults to localhost:11434) and
    /// `OLLAMA_MODEL` (required when Ollama is enabled).
    pub fn from_env() -> Result<Self, LLMError> {
        let model = std::env::var(crate::constants::ENV_OLLAMA_MODEL).map_err(|_| {
            LLMError::AuthError(format!(
                "{} not set — required when Ollama is enabled",
                crate::constants::ENV_OLLAMA_MODEL
            ))
        })?;

        let base_url = std::env::var(crate::constants::ENV_OLLAMA_BASE_URL)
            .unwrap_or_else(|_| crate::constants::OLLAMA_DEFAULT_BASE_URL.to_string());

        let timeout_secs = crate::constants::OLLAMA_DEFAULT_TIMEOUT_SECS;

        Ok(Self {
            base_url,
            model,
            timeout_secs,
        })
    }

    /// Set the model.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Set the base URL (for testing).
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }
}

/// Ollama API client for local LLM inference.
#[derive(Clone)]
pub struct OllamaClient {
    client: Client,
    config: OllamaConfig,
}

impl OllamaClient {
    /// Create a new Ollama client.
    pub fn new(config: OllamaConfig) -> Result<Self, LLMError> {
        if config.model.is_empty() {
            return Err(LLMError::AuthError(
                "Ollama model cannot be empty".to_string(),
            ));
        }

        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()
            .map_err(LLMError::HttpError)?;

        Ok(Self { client, config })
    }

    /// Create client from environment.
    pub fn from_env() -> Result<Self, LLMError> {
        let config = OllamaConfig::from_env()?;
        Self::new(config)
    }

    /// Get the chat API URL.
    fn chat_url(&self) -> String {
        format!("{}/api/chat", self.config.base_url)
    }

    /// Convert an LLMRequest to Ollama's chat request format.
    fn build_request(&self, request: &LLMRequest, stream: bool) -> OllamaChatRequest {
        let model = if request.model.is_empty() {
            self.config.model.clone()
        } else {
            request.model.clone()
        };

        // Build messages: system prompt as first message, then conversation
        let mut messages = Vec::new();

        if let Some(ref system) = request.system {
            messages.push(OllamaMessage {
                role: "system".to_string(),
                content: Some(system.clone()),
                tool_calls: None,
            });
        }

        for msg in &request.messages {
            messages.push(OllamaMessage::from_llm_message(msg));
        }

        // Map tools to Ollama format
        let tools: Vec<OllamaTool> = request.tools.iter().map(OllamaTool::from_tool).collect();

        OllamaChatRequest {
            model,
            messages,
            stream,
            options: Some(OllamaOptions {
                temperature: Some(request.temperature),
                num_predict: Some(request.max_tokens as i64),
            }),
            tools: if tools.is_empty() { None } else { Some(tools) },
        }
    }

    /// Parse a non-streaming response.
    fn parse_response(body: &str) -> LLMResult<LLMResponse> {
        let response: OllamaChatResponse =
            serde_json::from_str(body).map_err(|e| LLMError::ParseError(e.to_string()))?;

        let mut text_parts = Vec::new();
        let mut content_blocks = Vec::new();

        // Extract text content
        if let Some(ref content) = response.message.content {
            if !content.is_empty() {
                text_parts.push(content.clone());
                content_blocks.push(ContentBlock::Text {
                    text: content.clone(),
                });
            }
        }

        // Extract tool calls
        if let Some(ref tool_calls) = response.message.tool_calls {
            for tc in tool_calls {
                content_blocks.push(ContentBlock::ToolUse {
                    id: tc.function.name.clone(), // Ollama doesn't provide IDs; use name
                    name: tc.function.name.clone(),
                    input: tc.function.arguments.clone(),
                });
            }
        }

        let stop_reason = if response.message.tool_calls.is_some() {
            StopReason::ToolUse
        } else {
            StopReason::EndTurn
        };

        Ok(LLMResponse {
            content: text_parts.join(""),
            content_blocks,
            model: response.model,
            stop_reason,
            usage: TokenUsage {
                input_tokens: response.prompt_eval_count.unwrap_or(0),
                output_tokens: response.eval_count.unwrap_or(0),
            },
        })
    }

    /// Parse a single streaming chunk (newline-delimited JSON).
    fn parse_stream_chunk(line: &str) -> Option<LLMResult<StreamChunk>> {
        if line.trim().is_empty() {
            return None;
        }

        match serde_json::from_str::<OllamaStreamChunk>(line) {
            Ok(chunk) => {
                if chunk.done {
                    // Final chunk with token counts
                    Some(Ok(StreamChunk::MessageDelta {
                        stop_reason: Some(StopReason::EndTurn),
                        output_tokens: chunk.eval_count,
                    }))
                } else if let Some(ref msg) = chunk.message {
                    if let Some(ref content) = msg.content {
                        if !content.is_empty() {
                            return Some(Ok(StreamChunk::ContentDelta {
                                text: content.clone(),
                                index: 0,
                            }));
                        }
                    }
                    None
                } else {
                    None
                }
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to parse Ollama stream chunk: {} - line: {}",
                    e,
                    line
                );
                None
            }
        }
    }
}

// ── Ollama API request/response structures ────────────────────────────────

#[derive(Debug, Serialize)]
struct OllamaChatRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<OllamaOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<OllamaTool>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OllamaMessage {
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<OllamaToolCall>>,
}

impl OllamaMessage {
    fn from_llm_message(msg: &Message) -> Self {
        let role = match msg.role {
            Role::User => "user",
            Role::Assistant => "assistant",
        };

        let content = match &msg.content {
            MessageContent::Text(s) => Some(s.clone()),
            MessageContent::Blocks(blocks) => {
                // Concatenate text blocks; tool use/results handled separately
                let text: String = blocks
                    .iter()
                    .filter_map(|b| match b {
                        ContentBlock::Text { text } => Some(text.as_str()),
                        ContentBlock::ToolResult { content, .. } => Some(content.as_str()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
                    .join("");
                if text.is_empty() {
                    None
                } else {
                    Some(text)
                }
            }
        };

        Self {
            role: role.to_string(),
            content,
            tool_calls: None,
        }
    }
}

#[derive(Debug, Serialize)]
struct OllamaOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    num_predict: Option<i64>,
}

#[derive(Debug, Serialize)]
struct OllamaTool {
    #[serde(rename = "type")]
    tool_type: String,
    function: OllamaToolFunction,
}

#[derive(Debug, Serialize)]
struct OllamaToolFunction {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

impl OllamaTool {
    fn from_tool(tool: &Tool) -> Self {
        Self {
            tool_type: "function".to_string(),
            function: OllamaToolFunction {
                name: tool.name.clone(),
                description: tool.description.clone(),
                parameters: tool.input_schema.clone(),
            },
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct OllamaToolCall {
    function: OllamaToolCallFunction,
}

#[derive(Debug, Serialize, Deserialize)]
struct OllamaToolCallFunction {
    name: String,
    arguments: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct OllamaChatResponse {
    model: String,
    message: OllamaResponseMessage,
    #[serde(default)]
    prompt_eval_count: Option<u32>,
    #[serde(default)]
    eval_count: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct OllamaResponseMessage {
    #[allow(dead_code)]
    role: String,
    content: Option<String>,
    tool_calls: Option<Vec<OllamaToolCall>>,
}

/// Streaming chunk from Ollama (newline-delimited JSON).
#[derive(Debug, Deserialize)]
struct OllamaStreamChunk {
    #[allow(dead_code)]
    model: Option<String>,
    message: Option<OllamaStreamMessage>,
    done: bool,
    #[serde(default)]
    eval_count: Option<u32>,
    #[serde(default)]
    prompt_eval_count: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct OllamaStreamMessage {
    #[allow(dead_code)]
    role: Option<String>,
    content: Option<String>,
}

// ── LLMProvider trait implementation ──────────────────────────────────────

#[async_trait]
impl LLMProvider for OllamaClient {
    async fn send_message(&self, request: LLMRequest) -> LLMResult<LLMResponse> {
        let api_request = self.build_request(&request, false);

        let response = self
            .client
            .post(self.chat_url())
            .json(&api_request)
            .send()
            .await?;

        let status = response.status().as_u16();

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(LLMError::ApiError {
                status,
                message: format!("Ollama error: {}", body),
            });
        }

        let body = response.text().await.map_err(LLMError::HttpError)?;
        Self::parse_response(&body)
    }

    async fn send_message_stream(
        &self,
        request: LLMRequest,
    ) -> LLMResult<Pin<Box<dyn Stream<Item = LLMResult<StreamChunk>> + Send>>> {
        let api_request = self.build_request(&request, true);

        let response = self
            .client
            .post(self.chat_url())
            .json(&api_request)
            .send()
            .await?;

        let status = response.status().as_u16();

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(LLMError::ApiError {
                status,
                message: format!("Ollama error: {}", body),
            });
        }

        // Ollama streams newline-delimited JSON (not SSE)
        let byte_stream = response.bytes_stream();

        let stream = async_stream::stream! {
            let mut buffer = String::new();

            // Emit a synthetic MessageStart for accumulator compatibility
            yield Ok(StreamChunk::MessageStart {
                model: String::new(),
                input_tokens: 0,
            });
            yield Ok(StreamChunk::ContentBlockStart { index: 0 });

            let mut stream = byte_stream;
            let mut final_input_tokens: Option<u32> = None;

            while let Some(chunk_result) = stream.next().await {
                match chunk_result {
                    Ok(bytes) => {
                        buffer.push_str(&String::from_utf8_lossy(&bytes));

                        while let Some(newline_pos) = buffer.find('\n') {
                            let line = buffer[..newline_pos].to_string();
                            buffer = buffer[newline_pos + 1..].to_string();

                            if line.trim().is_empty() {
                                continue;
                            }

                            // Check for done=true to capture input tokens
                            if let Ok(chunk) = serde_json::from_str::<OllamaStreamChunk>(&line) {
                                if chunk.done {
                                    final_input_tokens = chunk.prompt_eval_count;
                                }
                            }

                            if let Some(result) = Self::parse_stream_chunk(&line) {
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

            yield Ok(StreamChunk::ContentBlockStop { index: 0 });

            // Emit input token count if we captured it
            if let Some(input_tokens) = final_input_tokens {
                yield Ok(StreamChunk::MessageStart {
                    model: String::new(),
                    input_tokens,
                });
            }

            yield Ok(StreamChunk::MessageStop);
        };

        Ok(Box::pin(stream))
    }

    fn provider_name(&self) -> &'static str {
        "ollama"
    }

    fn model_id(&self) -> &str {
        &self.config.model
    }
}

#[cfg(test)]
mod tests;
