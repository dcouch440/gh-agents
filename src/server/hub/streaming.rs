//! Stream sink abstraction for routing LLM output to different consumers.
//!
//! - `SseSink` wraps AppState's BufferedStream for chat SSE clients.
//! - `WsSink` broadcasts pipeline execution events via WebSocket.
//! - `NullSink` discards all output for background/non-interactive runs.

use async_trait::async_trait;
use uuid::Uuid;

use crate::server::state::{AppState, StreamChunk};
use crate::server::ws::PipelineUpdate;

/// Trait for receiving streaming LLM output.
///
/// Implementations decide where tokens go: SSE, WebSocket, or nowhere.
#[async_trait]
pub trait StreamSink: Send + Sync {
    /// A text token was generated.
    async fn token(&self, text: &str);

    /// A tool call started.
    async fn tool_start(&self, name: &str, tool_id: &str);

    /// A tool call finished.
    async fn tool_end(&self, name: &str, tool_id: &str);

    /// An error occurred during execution.
    async fn error(&self, msg: &str);

    /// Execution is complete.
    async fn done(&self);
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
            .send_stream_chunk(self.message_id, StreamChunk::Token(text.to_string()))
            .await;
    }

    async fn tool_start(&self, name: &str, tool_id: &str) {
        self.state
            .send_stream_chunk(
                self.message_id,
                StreamChunk::ToolStart {
                    name: name.to_string(),
                    tool_id: tool_id.to_string(),
                },
            )
            .await;
    }

    async fn tool_end(&self, name: &str, tool_id: &str) {
        self.state
            .send_stream_chunk(
                self.message_id,
                StreamChunk::ToolEnd {
                    name: name.to_string(),
                    tool_id: tool_id.to_string(),
                },
            )
            .await;
    }

    async fn error(&self, msg: &str) {
        self.state
            .send_stream_chunk(self.message_id, StreamChunk::Error(msg.to_string()))
            .await;
    }

    async fn done(&self) {
        self.state
            .send_stream_chunk(self.message_id, StreamChunk::Done)
            .await;
    }
}

// ── WsSink ──────────────────────────────────────────────────────────────────

/// Routes execution events to WebSocket pipeline subscribers.
pub struct WsSink {
    state: AppState,
    run_id: Uuid,
    pipeline_id: Uuid,
    stage_number: i32,
    stage_name: Option<String>,
    agent_id: Option<String>,
}

impl WsSink {
    pub fn new(
        state: AppState,
        run_id: Uuid,
        pipeline_id: Uuid,
        stage_number: i32,
        stage_name: Option<String>,
        agent_id: Option<String>,
    ) -> Self {
        Self {
            state,
            run_id,
            pipeline_id,
            stage_number,
            stage_name,
            agent_id,
        }
    }
}

#[async_trait]
impl StreamSink for WsSink {
    async fn token(&self, _text: &str) {
        // WsSink doesn't forward individual tokens — pipeline observers
        // get stage-level events, not token-by-token streaming.
    }

    async fn tool_start(&self, name: &str, _tool_id: &str) {
        self.state.broadcast_pipeline(PipelineUpdate {
            run_id: self.run_id,
            pipeline_id: self.pipeline_id,
            event: "tool_start".into(),
            stage_number: Some(self.stage_number),
            stage_name: self.stage_name.clone(),
            agent_id: self.agent_id.clone(),
            output: Some(name.to_string()),
            input_tokens: None,
            output_tokens: None,
            duration_ms: None,
            user_input: None,
            timestamp: chrono::Utc::now(),
            user_id: None,
        });
    }

    async fn tool_end(&self, name: &str, _tool_id: &str) {
        self.state.broadcast_pipeline(PipelineUpdate {
            run_id: self.run_id,
            pipeline_id: self.pipeline_id,
            event: "tool_end".into(),
            stage_number: Some(self.stage_number),
            stage_name: self.stage_name.clone(),
            agent_id: self.agent_id.clone(),
            output: Some(name.to_string()),
            input_tokens: None,
            output_tokens: None,
            duration_ms: None,
            user_input: None,
            timestamp: chrono::Utc::now(),
            user_id: None,
        });
    }

    async fn error(&self, msg: &str) {
        self.state.broadcast_pipeline(PipelineUpdate {
            run_id: self.run_id,
            pipeline_id: self.pipeline_id,
            event: "step_error".into(),
            stage_number: Some(self.stage_number),
            stage_name: self.stage_name.clone(),
            agent_id: self.agent_id.clone(),
            output: Some(msg.to_string()),
            input_tokens: None,
            output_tokens: None,
            duration_ms: None,
            user_input: None,
            timestamp: chrono::Utc::now(),
            user_id: None,
        });
    }

    async fn done(&self) {
        // Stage completion is signaled by the DAG orchestrator, not by the sink.
    }
}

// ── NullSink ────────────────────────────────────────────────────────────────

/// Discards all output. Used for background/non-interactive executions.
pub struct NullSink;

#[async_trait]
impl StreamSink for NullSink {
    async fn token(&self, _text: &str) {}
    async fn tool_start(&self, _name: &str, _tool_id: &str) {}
    async fn tool_end(&self, _name: &str, _tool_id: &str) {}
    async fn error(&self, _msg: &str) {}
    async fn done(&self) {}
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn null_sink_all_methods_are_noop() {
        let sink = NullSink;
        sink.token("hello").await;
        sink.tool_start("search", "t1").await;
        sink.tool_end("search", "t1").await;
        sink.error("boom").await;
        sink.done().await;
    }

    #[tokio::test]
    async fn sse_sink_sends_chunks() {
        use crate::db::traits::MockServerRepo;
        use crate::types::AppConfig;
        use std::sync::Arc;

        let mut mock = MockServerRepo::new();
        mock.expect_health_check().returning(|| true);
        let repo: Arc<dyn crate::db::traits::ServerRepo> = Arc::new(mock);
        let (state, _rx) = AppState::with_repo(None, repo, AppConfig::default());

        let msg_id = Uuid::new_v4();
        state.ensure_response_stream(msg_id).await;

        let sink = SseSink::new(state.clone(), msg_id);
        sink.token("hi").await;
        sink.tool_start("search", "t1").await;
        sink.tool_end("search", "t1").await;
        sink.error("oops").await;
        sink.done().await;

        let (buf, _rx, done) = state.get_response_stream(msg_id).await;
        assert_eq!(buf.len(), 5);
        assert!(done);
    }
}
