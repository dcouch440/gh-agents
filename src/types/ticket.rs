//! Ticket and GitHub integration types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::task::SliceId;

/// Unique identifier for a ticket
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TicketId(pub Uuid);

impl TicketId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for TicketId {
    fn default() -> Self {
        Self::new()
    }
}

/// Source of a ticket
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
#[derive(Default)]
pub enum TicketSource {
    GitHub {
        owner: String,
        repo: String,
        issue_number: u32,
    },
    #[default]
    Manual,
}

/// Ticket lifecycle status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TicketStatus {
    #[default]
    New,
    Planning,
    InProgress,
    Review,
    Completed,
    Closed,
}

/// A ticket (issue) from GitHub or manual entry
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ticket {
    pub id: TicketId,
    pub source: TicketSource,
    pub title: String,
    pub description: String,
    pub labels: Vec<String>,
    pub slices: Vec<SliceId>,
    pub status: TicketStatus,
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ticket_status_default_is_new() {
        assert_eq!(TicketStatus::default(), TicketStatus::New);
    }

    #[test]
    fn ticket_source_default_is_manual() {
        assert!(matches!(TicketSource::default(), TicketSource::Manual));
    }
}
