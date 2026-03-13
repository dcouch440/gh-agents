use anyhow::Result;
use async_trait::async_trait;
use uuid::Uuid;

use crate::db::SystemFileRow;

/// Input for upserting a system file metadata row.
#[derive(Debug, Clone)]
pub struct UpsertSystemFileInput {
    pub workflow_id: Uuid,
    pub path: String,
    pub media_type: String,
    pub description: String,
    pub tags: Vec<String>,
    pub produced_by: Option<Uuid>,
    pub produced_by_agent: Option<String>,
    pub size_bytes: i64,
}

/// Database operations for system store file metadata.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait SystemFileRepo: Send + Sync {
    /// Insert or update a file's metadata. Bumps version on conflict.
    async fn upsert_file(&self, input: UpsertSystemFileInput) -> Result<SystemFileRow>;

    /// Get a single file by workflow + path.
    async fn get_file(&self, workflow_id: Uuid, path: &str) -> Result<Option<SystemFileRow>>;

    /// List files whose path starts with the given prefix.
    async fn list_files(&self, workflow_id: Uuid, prefix: &str) -> Result<Vec<SystemFileRow>>;

    /// Delete a single file by workflow + path. Returns true if a row was deleted.
    async fn delete_file(&self, workflow_id: Uuid, path: &str) -> Result<bool>;

    /// Delete all files whose path starts with the given prefix. Returns count deleted.
    async fn delete_by_prefix(&self, workflow_id: Uuid, prefix: &str) -> Result<u64>;

    /// List all files produced by a specific step.
    async fn list_by_producer(
        &self,
        workflow_id: Uuid,
        step_id: Uuid,
    ) -> Result<Vec<SystemFileRow>>;
}
