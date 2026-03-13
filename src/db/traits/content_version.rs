use anyhow::Result;
use async_trait::async_trait;
use uuid::Uuid;

use crate::db::{ContentVersionRow, EnvelopeSnapshotRow, RunSnapshotRow};

// ============================================================================
// Content Version Repository
// ============================================================================

/// Database operations for content versioning and run snapshots.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait ContentVersionRepo: Send + Sync {
    /// Find an existing version by (source_id, content_type, content_hash)
    /// or create a new one. Returns the version row (existing or newly created).
    async fn find_or_create_version(
        &self,
        source_id: Uuid,
        content_type: &str,
        content_hash: &str,
        content: &str,
    ) -> Result<ContentVersionRow>;

    /// Create a run snapshot linking (run_id, step_id, content_type, role) to a version.
    async fn create_run_snapshot(
        &self,
        run_id: Uuid,
        step_id: Uuid,
        content_type: &str,
        role: &str,
        content_version_id: Uuid,
        source_id: Uuid,
    ) -> Result<RunSnapshotRow>;

    /// Get the content version for a specific (run_id, step_id, content_type, role).
    async fn get_run_snapshot(
        &self,
        run_id: Uuid,
        step_id: Uuid,
        content_type: &str,
        role: &str,
    ) -> Result<Option<RunSnapshotRow>>;

    /// List all snapshots for a given run.
    async fn list_run_snapshots(&self, run_id: Uuid) -> Result<Vec<RunSnapshotRow>>;

    /// Resolve a document def_id to its versioned content for a specific run.
    async fn resolve_document_version_by_def(
        &self,
        def_id: Uuid,
        run_id: Uuid,
    ) -> Result<Option<ContentVersionRow>>;

    /// List all envelope output snapshots for a run (JOIN with content_versions).
    /// Returns (step_id, envelope_json, source_id) for DagState reconstruction.
    async fn list_envelope_snapshots_for_run(
        &self,
        run_id: Uuid,
    ) -> Result<Vec<EnvelopeSnapshotRow>>;

    /// Get the latest envelope snapshot content for a step (across all runs).
    /// Used by pinned node replay to load the most recent execution output.
    async fn get_latest_envelope_for_step(&self, step_id: Uuid) -> Result<Option<String>>;
}
