//! Unified event types for the WebSocket event system.
//!
//! All domain events flow through [`ServerEvent`] and are serialized into
//! a flat [`WireMessage`] JSON shape for transmission to clients.
//!
//! # Wire Format
//!
//! Every event is sent as:
//! ```json
//! {
//!   "topic": "workflow",
//!   "event": "step_started",
//!   "ts": "2024-01-01T00:00:00Z",
//!   "run_id": "abc-123",
//!   "user_id": "def-456",
//!   "data": { ... }
//! }
//! ```

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ============================================================================
// Topic
// ============================================================================

/// Subscription topic. Clients subscribe to topics, not individual events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Topic {
    Workflow,
    Room,
    Session,
}

// ============================================================================
// Wire message (the single JSON shape on the wire)
// ============================================================================

/// Flat JSON structure sent to clients. Every server event becomes one of these.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WireMessage {
    pub topic: Topic,
    pub event: String,
    pub ts: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<Uuid>,
    pub data: serde_json::Value,
}

// ============================================================================
// Control messages (sent directly on socket, not broadcast)
// ============================================================================

/// Messages sent directly to a single client in response to their requests.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum ControlMessage {
    /// Acknowledges subscription changes, returns current topic list.
    #[serde(rename = "subscribed")]
    Subscribed { topics: Vec<Topic> },
    /// Error message for invalid requests.
    #[serde(rename = "error")]
    Error { message: String },
    /// Application-level pong for latency measurement.
    #[serde(rename = "pong")]
    Pong {
        client_ts: String,
        server_ts: DateTime<Utc>,
    },
    /// Notifies client that events were missed due to slow consumption.
    /// Client should re-fetch relevant state via REST API.
    #[serde(rename = "events_missed")]
    EventsMissed { missed_count: u64, message: String },
}

// ============================================================================
// Client messages
// ============================================================================

/// Messages clients send to the server.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    /// Subscribe to one or more topics.
    #[serde(rename = "subscribe")]
    Subscribe { topics: Vec<Topic> },
    /// Unsubscribe from one or more topics.
    #[serde(rename = "unsubscribe")]
    Unsubscribe { topics: Vec<Topic> },
    /// Subscribe to events for a specific run (run-scoped filtering).
    #[serde(rename = "subscribe_run")]
    SubscribeRun { run_id: Uuid },
    /// Unsubscribe from a specific run.
    #[serde(rename = "unsubscribe_run")]
    UnsubscribeRun { run_id: Uuid },
    /// Application-level ping for latency measurement.
    #[serde(rename = "ping")]
    Ping { ts: String },
}

// ============================================================================
// ServerEvent (top-level enum for broadcast channel)
// ============================================================================

/// The single event type carried by the unified broadcast channel.
#[derive(Debug, Clone)]
pub enum ServerEvent {
    Workflow(WorkflowEvent),
    Room(RoomEvent),
    Session(SessionEvent),
}

impl ServerEvent {
    /// The topic this event belongs to.
    pub fn topic(&self) -> Topic {
        match self {
            Self::Workflow(_) => Topic::Workflow,
            Self::Room(_) => Topic::Room,
            Self::Session(_) => Topic::Session,
        }
    }

    /// The user this event is scoped to, if any.
    /// Events with `None` are broadcast to all subscribers of the topic.
    pub fn user_id(&self) -> Option<Uuid> {
        match self {
            Self::Workflow(e) => e.user_id,
            Self::Room(e) => e.user_id,
            Self::Session(e) => e.user_id,
        }
    }

    /// The run ID this event belongs to, for run-scoped filtering.
    pub fn run_id(&self) -> Option<Uuid> {
        match self {
            Self::Workflow(e) => e.run_id,
            Self::Room(e) => e.run_id,
            Self::Session(_) => None,
        }
    }

    /// Convert into the flat wire format for serialization.
    pub fn into_wire_message(self) -> WireMessage {
        match self {
            Self::Workflow(e) => e.into_wire_message(),
            Self::Room(e) => e.into_wire_message(),
            Self::Session(e) => e.into_wire_message(),
        }
    }
}

// ============================================================================
// WorkflowEvent
// ============================================================================

/// Workflow execution lifecycle event.
///
/// `run_id` is optional: workflow-run events carry `Some(run_id)` for
/// run-scoped filtering, while tool-mutation events from interactive
/// chat sessions use `None` to reach all topic subscribers.
#[derive(Debug, Clone)]
pub struct WorkflowEvent {
    pub run_id: Option<Uuid>,
    pub workflow_id: Uuid,
    pub user_id: Option<Uuid>,
    pub kind: WorkflowEventKind,
}

/// The specific workflow event variant with its data.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowEventKind {
    Started {
        total_steps: usize,
    },
    StepStarted {
        step_id: Uuid,
        step_name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        agent_id: Option<Uuid>,
        #[serde(skip_serializing_if = "Option::is_none")]
        execution_id: Option<Uuid>,
    },
    StepCompleted {
        step_id: Uuid,
        step_name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        agent_id: Option<Uuid>,
        #[serde(skip_serializing_if = "Option::is_none")]
        output: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        input_tokens: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        output_tokens: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        duration_ms: Option<u64>,
    },
    StepFailed {
        step_id: Uuid,
        step_name: String,
        error: String,
    },
    StepPaused {
        step_id: Uuid,
        step_name: String,
    },
    /// Workforce agent execution progress (started, completed, failed).
    WorkforceAgentProgress {
        step_id: Uuid,
        agent_name: String,
        roster_agent_id: Uuid,
        agent_index: usize,
        total_agents: usize,
        status: String,
    },
    /// Workforce designer pre-phase progress (started, completed, failed).
    WorkforceDesignerProgress {
        step_id: Uuid,
        status: String,
    },
    /// A single agent config was written by the designer.
    DesignerAgentDesigned {
        step_id: Uuid,
        agent_name: String,
        designed_count: usize,
        total_count: usize,
    },
    Completed {
        #[serde(skip_serializing_if = "Option::is_none")]
        duration_ms: Option<u64>,
    },
    Failed {
        error: String,
    },
    Resumed {
        step_id: Uuid,
    },
    /// Agent roster was changed (agent added, updated, or removed).
    RosterChanged {
        step_id: Uuid,
    },
    /// Room members were changed (member added, updated, or removed).
    RoomMembersChanged {
        step_id: Uuid,
    },
    /// Step configuration was updated (name, description, prompt).
    StepConfigUpdated {
        step_id: Uuid,
    },
    /// Step name was updated.
    StepNameUpdated {
        step_id: Uuid,
        name: String,
    },
    /// Plan was updated for a step.
    PlanUpdated {
        step_id: Uuid,
        content: String,
    },
    /// Live text token from a step-scoped execution source (agent, speaker, child step).
    StepStreamToken {
        step_id: Uuid,
        source_id: Uuid,
        source_name: String,
        content: String,
    },
    /// A step-scoped execution source started a tool call.
    StepStreamToolStart {
        step_id: Uuid,
        source_id: Uuid,
        source_name: String,
        tool_name: String,
        tool_id: String,
    },
    /// A step-scoped execution source's tool call completed.
    StepStreamToolEnd {
        step_id: Uuid,
        source_id: Uuid,
        source_name: String,
        tool_name: String,
        tool_id: String,
    },
    /// A step-scoped execution source encountered an error.
    StepStreamError {
        step_id: Uuid,
        source_id: Uuid,
        source_name: String,
        error: String,
    },
    // ── Debug events ──────────────────────────────────────────────
    /// System prompt sent to LLM (debug stream).
    DebugSystemPrompt {
        step_id: Uuid,
        agent_execution_id: Uuid,
        agent_name: Option<String>,
        content: String,
    },
    /// User message sent to LLM (debug stream).
    DebugUserMessage {
        step_id: Uuid,
        agent_execution_id: Uuid,
        agent_name: Option<String>,
        content: String,
    },
    /// Full assistant response text (debug stream — not per-token).
    DebugAssistantMessage {
        step_id: Uuid,
        agent_execution_id: Uuid,
        agent_name: Option<String>,
        content: String,
    },
    /// Tool call with input payload (debug stream).
    DebugToolCall {
        step_id: Uuid,
        agent_execution_id: Uuid,
        agent_name: Option<String>,
        tool_name: String,
        tool_id: String,
        input: serde_json::Value,
    },
    /// Tool result with output payload (debug stream).
    DebugToolResult {
        step_id: Uuid,
        agent_execution_id: Uuid,
        agent_name: Option<String>,
        tool_name: String,
        tool_id: String,
        result: String,
    },

    /// Step pin state was toggled.
    StepPinChanged {
        step_id: Uuid,
        pinned: bool,
    },
    /// A workflow step was created.
    StepCreated {
        step_id: Uuid,
        name: String,
    },
    /// A workflow step was deleted.
    StepDeleted {
        step_id: Uuid,
    },
    /// A workflow edge was created.
    EdgeCreated {
        edge_id: Uuid,
        from_step_id: Uuid,
        to_step_id: Uuid,
    },
    /// A workflow edge was deleted.
    EdgeDeleted {
        edge_id: Uuid,
        from_step_id: Uuid,
        to_step_id: Uuid,
    },
    /// A protocol was applied to a step.
    ProtocolApplied {
        step_id: Uuid,
        protocol_id: Uuid,
    },
    /// A protocol was unapplied from a step.
    ProtocolUnapplied {
        step_id: Uuid,
    },
}

impl WorkflowEvent {
    fn event_name(&self) -> &'static str {
        match &self.kind {
            WorkflowEventKind::Started { .. } => "started",
            WorkflowEventKind::StepStarted { .. } => "step_started",
            WorkflowEventKind::StepCompleted { .. } => "step_completed",
            WorkflowEventKind::StepFailed { .. } => "step_failed",
            WorkflowEventKind::StepPaused { .. } => "step_paused",
            WorkflowEventKind::WorkforceAgentProgress { .. } => "workforce_agent_progress",
            WorkflowEventKind::WorkforceDesignerProgress { .. } => "workforce_designer_progress",
            WorkflowEventKind::DesignerAgentDesigned { .. } => "designer_agent_designed",
            WorkflowEventKind::Completed { .. } => "completed",
            WorkflowEventKind::Failed { .. } => "failed",
            WorkflowEventKind::Resumed { .. } => "resumed",
            WorkflowEventKind::RosterChanged { .. } => "roster_changed",
            WorkflowEventKind::RoomMembersChanged { .. } => "room_members_changed",
            WorkflowEventKind::StepConfigUpdated { .. } => "step_config_updated",
            WorkflowEventKind::StepNameUpdated { .. } => "step_name_updated",
            WorkflowEventKind::PlanUpdated { .. } => "plan_updated",
            WorkflowEventKind::StepStreamToken { .. } => "step_stream_token",
            WorkflowEventKind::StepStreamToolStart { .. } => "step_stream_tool_start",
            WorkflowEventKind::StepStreamToolEnd { .. } => "step_stream_tool_end",
            WorkflowEventKind::StepStreamError { .. } => "step_stream_error",
            WorkflowEventKind::DebugSystemPrompt { .. } => "debug_system_prompt",
            WorkflowEventKind::DebugUserMessage { .. } => "debug_user_message",
            WorkflowEventKind::DebugAssistantMessage { .. } => "debug_assistant_message",
            WorkflowEventKind::DebugToolCall { .. } => "debug_tool_call",
            WorkflowEventKind::DebugToolResult { .. } => "debug_tool_result",
            WorkflowEventKind::StepPinChanged { .. } => "step_pin_changed",
            WorkflowEventKind::StepCreated { .. } => "step_created",
            WorkflowEventKind::StepDeleted { .. } => "step_deleted",
            WorkflowEventKind::EdgeCreated { .. } => "edge_created",
            WorkflowEventKind::EdgeDeleted { .. } => "edge_deleted",
            WorkflowEventKind::ProtocolApplied { .. } => "protocol_applied",
            WorkflowEventKind::ProtocolUnapplied { .. } => "protocol_unapplied",
        }
    }

    fn into_wire_message(self) -> WireMessage {
        let event_name = self.event_name().to_string();
        let mut data = extract_variant_data(&self.kind);
        // Inject common workflow fields into data
        if let serde_json::Value::Object(ref mut map) = data {
            map.insert("workflow_id".into(), serde_json::json!(self.workflow_id));
        }
        WireMessage {
            topic: Topic::Workflow,
            event: event_name,
            ts: Utc::now(),
            run_id: self.run_id,
            user_id: self.user_id,
            data,
        }
    }
}

// ============================================================================
// RoomEvent
// ============================================================================

/// Room session lifecycle event.
#[derive(Debug, Clone)]
pub struct RoomEvent {
    pub room_session_id: Uuid,
    pub run_id: Option<Uuid>,
    pub user_id: Option<Uuid>,
    pub kind: RoomEventKind,
}

/// The specific room event variant with its data.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RoomEventKind {
    SpeakerStart {
        agent_id: Uuid,
        agent_name: String,
        speaker_order: i32,
        turn_number: i32,
    },
    SpeakerToken {
        agent_id: Uuid,
        agent_name: String,
        content: String,
        speaker_order: i32,
        turn_number: i32,
    },
    SpeakerEnd {
        agent_id: Uuid,
        agent_name: String,
        content: String,
        speaker_order: i32,
        turn_number: i32,
    },
    TurnComplete {
        turn_number: i32,
    },
    SessionComplete {
        turn_number: i32,
    },
}

impl RoomEvent {
    fn event_name(&self) -> &'static str {
        match &self.kind {
            RoomEventKind::SpeakerStart { .. } => "speaker_start",
            RoomEventKind::SpeakerToken { .. } => "speaker_token",
            RoomEventKind::SpeakerEnd { .. } => "speaker_end",
            RoomEventKind::TurnComplete { .. } => "turn_complete",
            RoomEventKind::SessionComplete { .. } => "session_complete",
        }
    }

    fn into_wire_message(self) -> WireMessage {
        let event_name = self.event_name().to_string();
        let mut data = extract_variant_data(&self.kind);
        if let serde_json::Value::Object(ref mut map) = data {
            map.insert(
                "room_session_id".into(),
                serde_json::json!(self.room_session_id),
            );
        }
        WireMessage {
            topic: Topic::Room,
            event: event_name,
            ts: Utc::now(),
            run_id: self.run_id,
            user_id: self.user_id,
            data,
        }
    }
}

// ============================================================================
// SessionEvent
// ============================================================================

/// Chat session lifecycle event.
#[derive(Debug, Clone)]
pub struct SessionEvent {
    pub session_id: Uuid,
    pub user_id: Option<Uuid>,
    pub kind: SessionEventKind,
}

/// The specific session event variant with its data.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionEventKind {
    Created {
        title: String,
        mode_id: String,
    },
    Updated {
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        mode_id: Option<String>,
    },
    Deleted,
    /// A background dispatch task was started.
    DispatchStarted {
        execution_id: Uuid,
        step_id: Uuid,
        instruction: String,
    },
    /// Progress update from a running dispatch task.
    DispatchProgress {
        execution_id: Uuid,
        step_id: Uuid,
        message: String,
    },
    /// A dispatch task completed successfully.
    DispatchCompleted {
        execution_id: Uuid,
        step_id: Uuid,
        summary: String,
        /// Question from the builder, if any. Surfaces to manager board state.
        question: Option<String>,
    },
    /// A dispatch task failed.
    DispatchFailed {
        execution_id: Uuid,
        step_id: Uuid,
        error: String,
    },
    /// A dispatch task was cancelled.
    DispatchCancelled {
        execution_id: Uuid,
        step_id: Uuid,
    },
    /// An agent-sourced message was injected into this session.
    AgentMessage {
        message_id: Uuid,
        from_agent: String,
        message_type: String,
        content_preview: String,
    },
    /// Streaming text token from a dispatch agent.
    DispatchStreamToken {
        execution_id: Uuid,
        step_id: Uuid,
        content: String,
    },
    /// A dispatch agent started calling a tool.
    DispatchStreamToolStart {
        execution_id: Uuid,
        step_id: Uuid,
        tool_name: String,
        tool_id: String,
        input: serde_json::Value,
    },
    /// A dispatch agent's tool call completed.
    DispatchStreamToolEnd {
        execution_id: Uuid,
        step_id: Uuid,
        tool_name: String,
        tool_id: String,
        result: serde_json::Value,
    },
    /// An error occurred during dispatch execution.
    DispatchStreamError {
        execution_id: Uuid,
        step_id: Uuid,
        error: String,
    },
}

impl SessionEvent {
    fn event_name(&self) -> &'static str {
        match &self.kind {
            SessionEventKind::Created { .. } => "created",
            SessionEventKind::Updated { .. } => "updated",
            SessionEventKind::Deleted => "deleted",
            SessionEventKind::DispatchStarted { .. } => "dispatch_started",
            SessionEventKind::DispatchProgress { .. } => "dispatch_progress",
            SessionEventKind::DispatchCompleted { .. } => "dispatch_completed",
            SessionEventKind::DispatchFailed { .. } => "dispatch_failed",
            SessionEventKind::DispatchCancelled { .. } => "dispatch_cancelled",
            SessionEventKind::AgentMessage { .. } => "agent_message",
            SessionEventKind::DispatchStreamToken { .. } => "dispatch_stream_token",
            SessionEventKind::DispatchStreamToolStart { .. } => "dispatch_stream_tool_start",
            SessionEventKind::DispatchStreamToolEnd { .. } => "dispatch_stream_tool_end",
            SessionEventKind::DispatchStreamError { .. } => "dispatch_stream_error",
        }
    }

    fn into_wire_message(self) -> WireMessage {
        let event_name = self.event_name().to_string();
        let mut data = extract_variant_data(&self.kind);
        if let serde_json::Value::Object(ref mut map) = data {
            map.insert("session_id".into(), serde_json::json!(self.session_id));
        }
        WireMessage {
            topic: Topic::Session,
            event: event_name,
            ts: Utc::now(),
            run_id: None,
            user_id: self.user_id,
            data,
        }
    }
}

// ============================================================================
// Helpers
// ============================================================================

/// Extract the inner data from an externally-tagged serde enum variant.
///
/// Serde's default (externally tagged) format serializes `Variant { x: 1 }` as
/// `{"variant": {"x": 1}}`. This helper extracts the inner `{"x": 1}` object.
/// For unit variants (serialized as `"variant"`), returns an empty object.
fn extract_variant_data<T: Serialize>(kind: &T) -> serde_json::Value {
    let value = serde_json::to_value(kind).unwrap_or_default();
    match value {
        serde_json::Value::Object(map) => {
            // Externally tagged: {"variant_name": {inner_data}}
            // There should be exactly one key — the variant name.
            map.into_values()
                .next()
                .unwrap_or(serde_json::Value::Object(serde_json::Map::new()))
        }
        serde_json::Value::String(_) => {
            // Unit variant serialized as a string
            serde_json::Value::Object(serde_json::Map::new())
        }
        other => other,
    }
}
