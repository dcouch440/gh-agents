//! Message and feed types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::agent::AgentId;
use super::task::TaskId;

/// Unique identifier for a message
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MessageId(pub Uuid);

impl MessageId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for MessageId {
    fn default() -> Self {
        Self::new()
    }
}

/// Types of inter-agent messages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    TaskAssignment,
    TaskResult,
    ReviewRequest,
    ReviewFeedback,
    ContextRequest,
    ContextResponse,
    Escalation,
    StatusUpdate,
}

/// Task context passed with messages
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct TaskContext {
    pub files: Vec<String>,
    pub history: Vec<String>,
    pub conventions: String,
}

/// Message between agents
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentMessage {
    pub id: MessageId,
    pub from: AgentId,
    pub to: AgentId,
    pub message_type: MessageType,
    pub content: String,
    pub task_id: Option<TaskId>,
    pub context: Option<TaskContext>,
    pub timestamp: DateTime<Utc>,
}

/// Types of feed items for the activity view
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedItemType {
    AgentReport,
    TaskStarted,
    TaskCompleted,
    Error,
    UserMessage,
    SystemNotice,
    Milestone,
}

/// Verbosity levels for filtering feed output
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum VerbosityLevel {
    Quiet,
    #[default]
    Normal,
    Verbose,
}

/// Unique identifier for a feed item
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FeedItemId(pub Uuid);

impl FeedItemId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for FeedItemId {
    fn default() -> Self {
        Self::new()
    }
}

/// An item in the activity feed
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeedItem {
    pub id: FeedItemId,
    pub agent_id: AgentId,
    pub content: String,
    pub item_type: FeedItemType,
    pub verbosity_level: VerbosityLevel,
    pub timestamp: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verbosity_default_is_normal() {
        assert_eq!(VerbosityLevel::default(), VerbosityLevel::Normal);
    }

    #[test]
    fn message_id_generates_unique() {
        let id1 = MessageId::new();
        let id2 = MessageId::new();
        assert_ne!(id1, id2);
    }
}
