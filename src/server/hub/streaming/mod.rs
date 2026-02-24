//! Stream sink abstraction for routing LLM output to different consumers.
//!
//! - `SseSink` wraps AppState's BufferedStream for chat SSE clients.
//! - `NullSink` discards all output for background/non-interactive runs.

pub mod dag;
pub mod dispatch;

use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

use crate::server::state::{AppState, StreamChunk};

pub use dag::DagStreamSink;
pub use dispatch::DispatchStreamSink;

/// Trait for receiving streaming LLM output.
///
/// Implementations decide where tokens go: SSE, WebSocket, or nowhere.
#[async_trait]
pub trait StreamSink: Send + Sync {
    /// A text token was generated.
    async fn token(&self, text: &str);

    /// A tool call started. `input` contains the tool arguments.
    async fn tool_start(&self, name: &str, tool_id: &str, input: &Value);

    /// A tool call finished. `result` contains the tool return value.
    async fn tool_end(&self, name: &str, tool_id: &str, result: &Value);

    /// An interactive panel was rendered on the node.
    async fn panel_render(&self, content: &str, submit_label: &str);

    /// An error occurred during execution.
    async fn error(&self, msg: &str);

    /// Execution is complete.
    async fn done(&self);

    // ── Debug stream events (default no-op) ──────────────────────────

    /// Emit a debug event for the system prompt sent to the LLM.
    async fn debug_system_prompt(&self, _ae_id: Uuid, _content: &str) {}

    /// Emit a debug event for the user message sent to the LLM.
    async fn debug_user_message(&self, _ae_id: Uuid, _content: &str) {}

    /// Emit a debug event for the full assistant response.
    async fn debug_assistant_message(&self, _ae_id: Uuid, _content: &str) {}

    /// Emit a debug event for a tool call with its input payload.
    async fn debug_tool_call(
        &self,
        _ae_id: Uuid,
        _tool_name: &str,
        _tool_id: &str,
        _input: &Value,
    ) {
    }

    /// Emit a debug event for a tool result with its output.
    async fn debug_tool_result(
        &self,
        _ae_id: Uuid,
        _tool_name: &str,
        _tool_id: &str,
        _result: &str,
    ) {
    }
}

// ── SseSink ─────────────────────────────────────────────────────────────────

/// Routes tokens to the AppState buffered response stream for SSE clients.
pub struct SseSink {
    state: AppState,
    message_id: Uuid,
}

impl SseSink {
    pub fn new(state: AppState, message_id: Uuid) -> Self {
        Self { state, message_id }
    }
}

#[async_trait]
impl StreamSink for SseSink {
    async fn token(&self, text: &str) {
        self.state
            .send_stream_chunk(self.message_id, StreamChunk::Token(text.to_string()));
    }

    async fn tool_start(&self, name: &str, tool_id: &str, _input: &Value) {
        self.state.send_stream_chunk(
            self.message_id,
            StreamChunk::ToolStart {
                name: name.to_string(),
                tool_id: tool_id.to_string(),
            },
        );
    }

    async fn tool_end(&self, name: &str, tool_id: &str, _result: &Value) {
        self.state.send_stream_chunk(
            self.message_id,
            StreamChunk::ToolEnd {
                name: name.to_string(),
                tool_id: tool_id.to_string(),
            },
        );
    }

    async fn panel_render(&self, content: &str, submit_label: &str) {
        self.state.send_stream_chunk(
            self.message_id,
            StreamChunk::PanelRender {
                content: content.to_string(),
                submit_label: submit_label.to_string(),
            },
        );
    }

    async fn error(&self, msg: &str) {
        self.state
            .send_stream_chunk(self.message_id, StreamChunk::Error(msg.to_string()));
    }

    async fn done(&self) {
        self.state
            .send_stream_chunk(self.message_id, StreamChunk::Done);
    }
}

// ── NullSink ────────────────────────────────────────────────────────────────

/// Discards all output. Used for background/non-interactive executions.
pub struct NullSink;

#[async_trait]
impl StreamSink for NullSink {
    async fn token(&self, _text: &str) {}
    async fn tool_start(&self, _name: &str, _tool_id: &str, _input: &Value) {}
    async fn tool_end(&self, _name: &str, _tool_id: &str, _result: &Value) {}
    async fn panel_render(&self, _content: &str, _submit_label: &str) {}
    async fn error(&self, _msg: &str) {}
    async fn done(&self) {}
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests;
