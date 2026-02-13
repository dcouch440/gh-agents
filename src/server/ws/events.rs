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
    ForEachProgress {
        step_id: Uuid,
        step_name: String,
        completed: usize,
        total: usize,
    },
    DocumenterPhaseProgress {
        step_id: Uuid,
        phase: String,
        completed: usize,
        total: usize,
        #[serde(skip_serializing_if = "Option::is_none")]
        document_name: Option<String>,
    },
    TaskForceAgentProgress {
        step_id: Uuid,
        agent_name: String,
        agent_index: usize,
        total_agents: usize,
        status: String,
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
    /// A document definition was created on a step.
    DocDefCreated {
        step_id: Uuid,
        doc_def_id: Uuid,
        name: String,
    },
    /// A document definition was updated on a step.
    DocDefUpdated {
        step_id: Uuid,
        doc_def_id: Uuid,
        name: String,
    },
    /// A document definition was deleted from a step.
    DocDefDeleted {
        step_id: Uuid,
        doc_def_id: Uuid,
    },
    /// Step configuration was updated (name, description, prompt).
    StepConfigUpdated {
        step_id: Uuid,
    },
    /// Step archetype (execution_mode) was changed.
    ArchetypeChanged {
        step_id: Uuid,
        archetype: String,
    },
    /// Step name was updated.
    StepNameUpdated {
        step_id: Uuid,
        name: String,
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
            WorkflowEventKind::ForEachProgress { .. } => "for_each_progress",
            WorkflowEventKind::DocumenterPhaseProgress { .. } => "documenter_phase_progress",
            WorkflowEventKind::TaskForceAgentProgress { .. } => "task_force_agent_progress",
            WorkflowEventKind::Completed { .. } => "completed",
            WorkflowEventKind::Failed { .. } => "failed",
            WorkflowEventKind::Resumed { .. } => "resumed",
            WorkflowEventKind::DocDefCreated { .. } => "doc_def_created",
            WorkflowEventKind::DocDefUpdated { .. } => "doc_def_updated",
            WorkflowEventKind::DocDefDeleted { .. } => "doc_def_deleted",
            WorkflowEventKind::StepConfigUpdated { .. } => "step_config_updated",
            WorkflowEventKind::ArchetypeChanged { .. } => "archetype_changed",
            WorkflowEventKind::StepNameUpdated { .. } => "step_name_updated",
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
}

impl SessionEvent {
    fn event_name(&self) -> &'static str {
        match &self.kind {
            SessionEventKind::Created { .. } => "created",
            SessionEventKind::Updated { .. } => "updated",
            SessionEventKind::Deleted => "deleted",
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
