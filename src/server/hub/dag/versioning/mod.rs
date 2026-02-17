//! Content versioning utilities for immutable run snapshots.
//!
//! Provides SHA-256 content hashing and deduped snapshot creation.
//! All snapshot operations are fire-and-forget — failures never block execution.

use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::db::traits::ContentVersionRepo;
use crate::db::ContentVersionRow;

mod tests;

/// Content type constants for the `content_versions` table.
pub(crate) mod content_types {
    pub const PROMPT: &str = "prompt";
    pub const SYSTEM_PROMPT: &str = "system_prompt";
    pub const ENVELOPE: &str = "envelope";
}

/// Compute the SHA-256 hex digest of content.
pub(crate) fn compute_content_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Create a content version (deduped) and link it to a run as a snapshot.
///
/// - If identical content already exists for this source, reuses the existing version.
/// - Creates a `run_snapshots` row linking the run to the version.
/// - Returns the content version row.
pub(crate) async fn snapshot_content(
    repo: &dyn ContentVersionRepo,
    run_id: Uuid,
    step_id: Uuid,
    source_id: Uuid,
    content_type: &str,
    role: &str,
    content: &str,
) -> anyhow::Result<ContentVersionRow> {
    let hash = compute_content_hash(content);
    let version = repo
        .find_or_create_version(source_id, content_type, &hash, content)
        .await?;
    repo.create_run_snapshot(run_id, step_id, content_type, role, version.id, source_id)
        .await?;
    Ok(version)
}
