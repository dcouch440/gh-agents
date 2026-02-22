//! Generic DAG-level stream sink that broadcasts step execution events via WebSocket.
//!
//! Any DAG execution mode (workforce agents, sub-workflow steps, etc.) can use
//! `DagStreamSink` to route LLM token/tool events through the event bus.

use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

use super::StreamSink;
use crate::server::hub::dag::{broadcast_workflow_event, WorkflowExecutionContext};
use crate::server::state::AppState;
use crate::server::ws::events::WorkflowEventKind;

/// Routes LLM output events through the WebSocket event bus for live streaming.
///
/// Each sink instance is scoped to a specific execution source (e.g., a workforce
/// agent, a child step, etc.) identified by `source_id` and `source_name`.
pub struct DagStreamSink {
    state: AppState,
    ctx: WorkflowExecutionContext,
    workflow_id: Uuid,
    step_id: Uuid,
    source_id: Uuid,
    source_name: String,
}

impl DagStreamSink {
    pub fn new(
        state: AppState,
        ctx: WorkflowExecutionContext,
        workflow_id: Uuid,
        step_id: Uuid,
        source_id: Uuid,
        source_name: String,
    ) -> Self {
        Self {
            state,
            ctx,
            workflow_id,
            step_id,
            source_id,
            source_name,
        }
    }
}

#[async_trait]
impl StreamSink for DagStreamSink {
    async fn token(&self, text: &str) {
        broadcast_workflow_event(
            &self.state,
            &self.ctx,
            self.workflow_id,
            WorkflowEventKind::StepStreamToken {
                step_id: self.step_id,
                source_id: self.source_id,
                source_name: self.source_name.clone(),
                content: text.to_string(),
            },
        );
    }

    async fn tool_start(&self, name: &str, tool_id: &str, _input: &Value) {
        broadcast_workflow_event(
            &self.state,
            &self.ctx,
            self.workflow_id,
            WorkflowEventKind::StepStreamToolStart {
                step_id: self.step_id,
                source_id: self.source_id,
                source_name: self.source_name.clone(),
                tool_name: name.to_string(),
                tool_id: tool_id.to_string(),
            },
        );
    }

    async fn tool_end(&self, name: &str, tool_id: &str, _result: &Value) {
        broadcast_workflow_event(
            &self.state,
            &self.ctx,
            self.workflow_id,
            WorkflowEventKind::StepStreamToolEnd {
                step_id: self.step_id,
                source_id: self.source_id,
                source_name: self.source_name.clone(),
                tool_name: name.to_string(),
                tool_id: tool_id.to_string(),
            },
        );
    }

    async fn panel_render(&self, _content: &str, _submit_label: &str) {
        // No-op for DAG executions
    }

    async fn error(&self, msg: &str) {
        broadcast_workflow_event(
            &self.state,
            &self.ctx,
            self.workflow_id,
            WorkflowEventKind::StepStreamError {
                step_id: self.step_id,
                source_id: self.source_id,
                source_name: self.source_name.clone(),
                error: msg.to_string(),
            },
        );
    }

    async fn done(&self) {
        // No-op — lifecycle handled by existing progress events
    }
}
