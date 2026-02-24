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
    /// Agent name for debug events (roster agent name for workforce, agent name for single steps).
    agent_name: Option<String>,
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
            agent_name: None,
        }
    }

    /// Set the agent name for debug events.
    pub fn with_agent_name(mut self, name: Option<String>) -> Self {
        self.agent_name = name;
        self
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

    async fn debug_system_prompt(&self, ae_id: Uuid, content: &str) {
        broadcast_workflow_event(
            &self.state,
            &self.ctx,
            self.workflow_id,
            WorkflowEventKind::DebugSystemPrompt {
                step_id: self.step_id,
                agent_execution_id: ae_id,
                agent_name: self.agent_name.clone(),
                content: content.to_string(),
            },
        );
    }

    async fn debug_user_message(&self, ae_id: Uuid, content: &str) {
        broadcast_workflow_event(
            &self.state,
            &self.ctx,
            self.workflow_id,
            WorkflowEventKind::DebugUserMessage {
                step_id: self.step_id,
                agent_execution_id: ae_id,
                agent_name: self.agent_name.clone(),
                content: content.to_string(),
            },
        );
    }

    async fn debug_assistant_message(&self, ae_id: Uuid, content: &str) {
        broadcast_workflow_event(
            &self.state,
            &self.ctx,
            self.workflow_id,
            WorkflowEventKind::DebugAssistantMessage {
                step_id: self.step_id,
                agent_execution_id: ae_id,
                agent_name: self.agent_name.clone(),
                content: content.to_string(),
            },
        );
    }

    async fn debug_tool_call(&self, ae_id: Uuid, tool_name: &str, tool_id: &str, input: &Value) {
        broadcast_workflow_event(
            &self.state,
            &self.ctx,
            self.workflow_id,
            WorkflowEventKind::DebugToolCall {
                step_id: self.step_id,
                agent_execution_id: ae_id,
                agent_name: self.agent_name.clone(),
                tool_name: tool_name.to_string(),
                tool_id: tool_id.to_string(),
                input: input.clone(),
            },
        );
    }

    async fn debug_tool_result(&self, ae_id: Uuid, tool_name: &str, tool_id: &str, result: &str) {
        broadcast_workflow_event(
            &self.state,
            &self.ctx,
            self.workflow_id,
            WorkflowEventKind::DebugToolResult {
                step_id: self.step_id,
                agent_execution_id: ae_id,
                agent_name: self.agent_name.clone(),
                tool_name: tool_name.to_string(),
                tool_id: tool_id.to_string(),
                result: result.to_string(),
            },
        );
    }
}
