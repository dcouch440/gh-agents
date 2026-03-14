//! Dispatch-level stream sink that broadcasts execution events via WebSocket.
//!
//! Routes LLM token/tool events from background dispatch agents (L2 manager
//! builder, L4 node builder) through the session event bus. Also appends
//! trace events to the task registry for REST retrieval.

use async_trait::async_trait;
use chrono::Utc;
use serde_json::Value;
use uuid::Uuid;

use super::StreamSink;
use crate::server::state::task_registry::TraceEvent;
use crate::server::state::AppState;
use crate::server::ws::events::{SessionEvent, SessionEventKind};

/// Routes dispatch agent output through the WebSocket event bus.
///
/// Each sink instance is scoped to a single dispatch execution identified
/// by `execution_id`. Events are broadcast on the `session` topic and
/// simultaneously stored in the task registry trace.
pub struct DispatchStreamSink {
    state: AppState,
    execution_id: Uuid,
    step_id: Uuid,
}

impl DispatchStreamSink {
    pub fn new(state: AppState, execution_id: Uuid, step_id: Uuid) -> Self {
        Self {
            state,
            execution_id,
            step_id,
        }
    }
}

#[async_trait]
impl StreamSink for DispatchStreamSink {
    async fn token(&self, text: &str) {
        let content = text.to_string();
        broadcast_dispatch_stream(
            &self.state,
            SessionEventKind::DispatchStreamToken {
                execution_id: self.execution_id,
                step_id: self.step_id,
                content: content.clone(),
            },
        );
        self.state.task_registry().append_trace(
            self.execution_id,
            TraceEvent::Token {
                content,
                ts: Utc::now(),
            },
        );
    }

    async fn tool_start(&self, name: &str, tool_id: &str, input: &Value) {
        let tool_name = name.to_string();
        let tool_id_str = tool_id.to_string();
        broadcast_dispatch_stream(
            &self.state,
            SessionEventKind::DispatchStreamToolStart {
                execution_id: self.execution_id,
                step_id: self.step_id,
                tool_name: tool_name.clone(),
                tool_id: tool_id_str.clone(),
                input: input.clone(),
            },
        );
        self.state.task_registry().append_trace(
            self.execution_id,
            TraceEvent::ToolStart {
                tool_name,
                tool_id: tool_id_str,
                input: input.clone(),
                ts: Utc::now(),
            },
        );
    }

    async fn tool_end(&self, name: &str, tool_id: &str, result: &Value) {
        let tool_name = name.to_string();
        let tool_id_str = tool_id.to_string();
        broadcast_dispatch_stream(
            &self.state,
            SessionEventKind::DispatchStreamToolEnd {
                execution_id: self.execution_id,
                step_id: self.step_id,
                tool_name: tool_name.clone(),
                tool_id: tool_id_str.clone(),
                result: result.clone(),
            },
        );
        self.state.task_registry().append_trace(
            self.execution_id,
            TraceEvent::ToolEnd {
                tool_name,
                tool_id: tool_id_str,
                result: result.clone(),
                ts: Utc::now(),
            },
        );
    }

    async fn panel_render(&self, _content: &str, _submit_label: &str) {
        // No-op for dispatch executions
    }

    async fn error(&self, msg: &str) {
        let error = msg.to_string();
        broadcast_dispatch_stream(
            &self.state,
            SessionEventKind::DispatchStreamError {
                execution_id: self.execution_id,
                step_id: self.step_id,
                error: error.clone(),
            },
        );
        self.state.task_registry().append_trace(
            self.execution_id,
            TraceEvent::Error {
                error,
                ts: Utc::now(),
            },
        );
    }

    async fn debug_system_prompt(&self, _ae_id: Uuid, content: &str) {
        let content = content.to_string();
        broadcast_dispatch_stream(
            &self.state,
            SessionEventKind::DispatchStreamSystemPrompt {
                execution_id: self.execution_id,
                step_id: self.step_id,
                content: content.clone(),
                agent_name: None,
            },
        );
        self.state.task_registry().append_trace(
            self.execution_id,
            TraceEvent::SystemPrompt {
                content,
                agent_name: None,
                ts: Utc::now(),
            },
        );
    }

    async fn debug_user_message(&self, _ae_id: Uuid, content: &str) {
        let content = content.to_string();
        broadcast_dispatch_stream(
            &self.state,
            SessionEventKind::DispatchStreamUserMessage {
                execution_id: self.execution_id,
                step_id: self.step_id,
                content: content.clone(),
                agent_name: None,
            },
        );
        self.state.task_registry().append_trace(
            self.execution_id,
            TraceEvent::UserMessage {
                content,
                agent_name: None,
                ts: Utc::now(),
            },
        );
    }

    async fn done(&self) {
        // No-op — lifecycle handled by existing DispatchCompleted event
    }
}

/// Broadcast a dispatch stream event on the session topic.
fn broadcast_dispatch_stream(state: &AppState, kind: SessionEventKind) {
    state.broadcast_session(SessionEvent {
        session_id: Uuid::nil(),
        user_id: None,
        kind,
    });
}
