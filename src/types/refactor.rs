//! Refactor mode types for mid-stream plan modifications.
//!
//! The refactor mode allows users to pause production, modify the project plan,
//! and resume with updated specs. This module defines the types for tracking
//! refactor sessions and proposed changes.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for a refactor session
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RefactorId(pub Uuid);

impl RefactorId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for RefactorId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for RefactorId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// System production mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProductionMode {
    /// Normal operation - work is being assigned and executed
    #[default]
    Running,
    /// In refactor conversation - no new work assigned
    RefactorMode,
    /// Production halted - waiting for refactor completion
    Paused,
    /// Transitioning back to running after refactor
    Resuming,
}

impl ProductionMode {
    /// Returns true if production is actively running
    pub fn is_active(&self) -> bool {
        matches!(self, ProductionMode::Running)
    }

    /// Returns true if in any refactor-related state
    pub fn is_refactoring(&self) -> bool {
        matches!(self, ProductionMode::RefactorMode | ProductionMode::Paused)
    }

    /// Convert to string for database storage
    pub fn as_str(&self) -> &'static str {
        match self {
            ProductionMode::Running => "running",
            ProductionMode::RefactorMode => "refactor_mode",
            ProductionMode::Paused => "paused",
            ProductionMode::Resuming => "resuming",
        }
    }

    /// Parse from database string
    pub fn from_str(s: &str) -> Self {
        match s {
            "running" => ProductionMode::Running,
            "refactor_mode" => ProductionMode::RefactorMode,
            "paused" => ProductionMode::Paused,
            "resuming" => ProductionMode::Resuming,
            _ => ProductionMode::Running,
        }
    }
}

/// Intent detected from user message during refactor conversation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RefactorIntent {
    /// User explicitly wants to halt production immediately
    HaltNow,
    /// User describes changes that affect existing tickets
    RefactorNeeded,
    /// User is exploring options, no action yet
    Clarifying,
    /// Casual conversation, no refactor intent
    JustChatting,
    /// User wants to exit refactor mode
    ExitRefactor,
}

impl RefactorIntent {
    /// Returns true if this intent should halt production
    pub fn should_halt(&self) -> bool {
        matches!(self, RefactorIntent::HaltNow | RefactorIntent::RefactorNeeded)
    }

    /// Returns true if this is a conversational intent (no action needed)
    pub fn is_conversational(&self) -> bool {
        matches!(self, RefactorIntent::Clarifying | RefactorIntent::JustChatting)
    }
}

/// Type of change to a planning file
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeType {
    /// Creating a new file
    Create,
    /// Modifying an existing file
    Modify,
    /// Deleting a file
    Delete,
    /// Renaming/moving a file
    Rename,
}

/// Status of a proposed change
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ChangeStatus {
    /// Change has been proposed but not reviewed
    #[default]
    Proposed,
    /// User approved the change
    Approved,
    /// User rejected the change
    Rejected,
    /// Change has been applied to filesystem
    Applied,
}

impl ChangeStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ChangeStatus::Proposed => "proposed",
            ChangeStatus::Approved => "approved",
            ChangeStatus::Rejected => "rejected",
            ChangeStatus::Applied => "applied",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "proposed" => ChangeStatus::Proposed,
            "approved" => ChangeStatus::Approved,
            "rejected" => ChangeStatus::Rejected,
            "applied" => ChangeStatus::Applied,
            _ => ChangeStatus::Proposed,
        }
    }
}

/// Unique identifier for a refactor change
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChangeId(pub Uuid);

impl ChangeId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ChangeId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ChangeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// A proposed change to a planning file
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RefactorChange {
    /// Unique identifier
    pub id: ChangeId,
    /// Session this change belongs to
    pub session_id: RefactorId,
    /// Path to the file (e.g., "decomp/M2/2.3.md")
    pub file_path: String,
    /// Type of change
    pub change_type: ChangeType,
    /// Original content (None for new files)
    pub before_content: Option<String>,
    /// New content (None for deletions)
    pub after_content: Option<String>,
    /// Reason for the change
    pub reason: String,
    /// Current status
    pub status: ChangeStatus,
    /// When the change was proposed
    pub created_at: DateTime<Utc>,
}

impl RefactorChange {
    /// Create a new proposed change
    pub fn new(session_id: RefactorId, file_path: String, change_type: ChangeType, before_content: Option<String>, after_content: Option<String>, reason: String) -> Self {
        Self {
            id: ChangeId::new(),
            session_id,
            file_path,
            change_type,
            before_content,
            after_content,
            reason,
            status: ChangeStatus::Proposed,
            created_at: Utc::now(),
        }
    }

    /// Create a modification change
    pub fn modify(session_id: RefactorId, file_path: String, before: String, after: String, reason: String) -> Self {
        Self::new(session_id, file_path, ChangeType::Modify, Some(before), Some(after), reason)
    }

    /// Create a new file change
    pub fn create(session_id: RefactorId, file_path: String, content: String, reason: String) -> Self {
        Self::new(session_id, file_path, ChangeType::Create, None, Some(content), reason)
    }

    /// Create a delete change
    pub fn delete(session_id: RefactorId, file_path: String, original_content: String, reason: String) -> Self {
        Self::new(session_id, file_path, ChangeType::Delete, Some(original_content), None, reason)
    }
}

/// A refactor session tracking conversation and changes
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RefactorSession {
    /// Unique identifier
    pub id: RefactorId,
    /// When the session started
    pub started_at: DateTime<Utc>,
    /// When the session ended (None if active)
    pub ended_at: Option<DateTime<Utc>>,
    /// Whether production was halted during this session
    pub production_halted: bool,
    /// Whether changes have been applied
    pub changes_applied: bool,
    /// Proposed changes in this session
    pub proposed_changes: Vec<RefactorChange>,
}

impl RefactorSession {
    /// Create a new refactor session
    pub fn new() -> Self {
        Self {
            id: RefactorId::new(),
            started_at: Utc::now(),
            ended_at: None,
            production_halted: false,
            changes_applied: false,
            proposed_changes: Vec::new(),
        }
    }

    /// Check if the session is active
    pub fn is_active(&self) -> bool {
        self.ended_at.is_none()
    }

    /// End the session
    pub fn end(&mut self) {
        self.ended_at = Some(Utc::now());
    }

    /// Mark production as halted
    pub fn halt_production(&mut self) {
        self.production_halted = true;
    }

    /// Add a proposed change
    pub fn add_change(&mut self, change: RefactorChange) {
        self.proposed_changes.push(change);
    }

    /// Get pending (proposed) changes
    pub fn pending_changes(&self) -> Vec<&RefactorChange> {
        self.proposed_changes.iter().filter(|c| c.status == ChangeStatus::Proposed).collect()
    }

    /// Get approved changes
    pub fn approved_changes(&self) -> Vec<&RefactorChange> {
        self.proposed_changes.iter().filter(|c| c.status == ChangeStatus::Approved).collect()
    }

    /// Mark changes as applied
    pub fn mark_changes_applied(&mut self) {
        self.changes_applied = true;
    }
}

impl Default for RefactorSession {
    fn default() -> Self {
        Self::new()
    }
}

/// Context provided to the refactor agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefactorContext {
    /// Current production mode
    pub production_mode: ProductionMode,
    /// Active refactor session if any
    pub session: Option<RefactorSession>,
    /// Summary of in-progress work
    pub in_progress_work: Vec<String>,
    /// Files that can be modified (decomp/, PROGRESS.md, etc.)
    pub modifiable_files: Vec<String>,
}

impl RefactorContext {
    /// Create a new refactor context
    pub fn new(production_mode: ProductionMode) -> Self {
        Self {
            production_mode,
            session: None,
            in_progress_work: Vec::new(),
            modifiable_files: vec!["PROGRESS.md".to_string(), "ROADMAP.md".to_string(), "decomp/".to_string()],
        }
    }

    /// Add an active session
    pub fn with_session(mut self, session: RefactorSession) -> Self {
        self.session = Some(session);
        self
    }

    /// Add in-progress work summary
    pub fn with_in_progress(mut self, work: Vec<String>) -> Self {
        self.in_progress_work = work;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refactor_id_generates_unique() {
        let id1 = RefactorId::new();
        let id2 = RefactorId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn production_mode_default_is_running() {
        assert_eq!(ProductionMode::default(), ProductionMode::Running);
    }

    #[test]
    fn production_mode_is_active() {
        assert!(ProductionMode::Running.is_active());
        assert!(!ProductionMode::RefactorMode.is_active());
        assert!(!ProductionMode::Paused.is_active());
        assert!(!ProductionMode::Resuming.is_active());
    }

    #[test]
    fn production_mode_is_refactoring() {
        assert!(!ProductionMode::Running.is_refactoring());
        assert!(ProductionMode::RefactorMode.is_refactoring());
        assert!(ProductionMode::Paused.is_refactoring());
        assert!(!ProductionMode::Resuming.is_refactoring());
    }

    #[test]
    fn production_mode_roundtrip() {
        for mode in [ProductionMode::Running, ProductionMode::RefactorMode, ProductionMode::Paused, ProductionMode::Resuming] {
            let s = mode.as_str();
            let parsed = ProductionMode::from_str(s);
            assert_eq!(mode, parsed);
        }
    }

    #[test]
    fn refactor_intent_should_halt() {
        assert!(RefactorIntent::HaltNow.should_halt());
        assert!(RefactorIntent::RefactorNeeded.should_halt());
        assert!(!RefactorIntent::Clarifying.should_halt());
        assert!(!RefactorIntent::JustChatting.should_halt());
        assert!(!RefactorIntent::ExitRefactor.should_halt());
    }

    #[test]
    fn refactor_intent_is_conversational() {
        assert!(!RefactorIntent::HaltNow.is_conversational());
        assert!(!RefactorIntent::RefactorNeeded.is_conversational());
        assert!(RefactorIntent::Clarifying.is_conversational());
        assert!(RefactorIntent::JustChatting.is_conversational());
        assert!(!RefactorIntent::ExitRefactor.is_conversational());
    }

    #[test]
    fn change_status_roundtrip() {
        for status in [ChangeStatus::Proposed, ChangeStatus::Approved, ChangeStatus::Rejected, ChangeStatus::Applied] {
            let s = status.as_str();
            let parsed = ChangeStatus::from_str(s);
            assert_eq!(status, parsed);
        }
    }

    #[test]
    fn refactor_change_create() {
        let session_id = RefactorId::new();
        let change = RefactorChange::create(
            session_id.clone(),
            "decomp/M2/2.7.md".to_string(),
            "# Ticket 2.7".to_string(),
            "Adding new ticket".to_string(),
        );

        assert_eq!(change.session_id, session_id);
        assert_eq!(change.change_type, ChangeType::Create);
        assert!(change.before_content.is_none());
        assert!(change.after_content.is_some());
        assert_eq!(change.status, ChangeStatus::Proposed);
    }

    #[test]
    fn refactor_change_modify() {
        let session_id = RefactorId::new();
        let change = RefactorChange::modify(
            session_id.clone(),
            "PROGRESS.md".to_string(),
            "# Old content".to_string(),
            "# New content".to_string(),
            "Updating progress".to_string(),
        );

        assert_eq!(change.change_type, ChangeType::Modify);
        assert!(change.before_content.is_some());
        assert!(change.after_content.is_some());
    }

    #[test]
    fn refactor_change_delete() {
        let session_id = RefactorId::new();
        let change = RefactorChange::delete(
            session_id.clone(),
            "decomp/M2/2.3.md".to_string(),
            "# Old ticket".to_string(),
            "Ticket no longer needed".to_string(),
        );

        assert_eq!(change.change_type, ChangeType::Delete);
        assert!(change.before_content.is_some());
        assert!(change.after_content.is_none());
    }

    #[test]
    fn refactor_session_lifecycle() {
        let mut session = RefactorSession::new();
        assert!(session.is_active());
        assert!(!session.production_halted);
        assert!(!session.changes_applied);

        session.halt_production();
        assert!(session.production_halted);

        let change = RefactorChange::create(session.id.clone(), "test.md".to_string(), "content".to_string(), "test".to_string());
        session.add_change(change);
        assert_eq!(session.pending_changes().len(), 1);

        session.mark_changes_applied();
        assert!(session.changes_applied);

        session.end();
        assert!(!session.is_active());
        assert!(session.ended_at.is_some());
    }

    #[test]
    fn refactor_context_builder() {
        let ctx = RefactorContext::new(ProductionMode::RefactorMode)
            .with_session(RefactorSession::new())
            .with_in_progress(vec!["Task 2.1".to_string()]);

        assert_eq!(ctx.production_mode, ProductionMode::RefactorMode);
        assert!(ctx.session.is_some());
        assert_eq!(ctx.in_progress_work.len(), 1);
        assert!(ctx.modifiable_files.contains(&"PROGRESS.md".to_string()));
    }
}
