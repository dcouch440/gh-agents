use anyhow::Result;
use async_trait::async_trait;
use uuid::Uuid;

use crate::db::traits::ContentVersionRepo;
use crate::db::{ContentVersionRow, EnvelopeSnapshotRow, RunSnapshotRow};

use super::PgRepo;

#[async_trait]
impl ContentVersionRepo for PgRepo {
    async fn find_or_create_version(
        &self,
        source_id: Uuid,
        content_type: &str,
        content_hash: &str,
        content: &str,
    ) -> Result<ContentVersionRow> {
        let byte_size = content.len() as i32;

        // Compute the next version number for this source + content_type
        let next_version: i32 = sqlx::query_scalar(
            "SELECT COALESCE(MAX(version_number), 0) + 1 FROM content_versions WHERE source_id = $1 AND content_type = $2",
        )
        .bind(source_id)
        .bind(content_type)
        .fetch_one(&self.pool)
        .await?;

        // Insert with dedup — ON CONFLICT means identical content reuses the row
        sqlx::query(
            "INSERT INTO content_versions (source_id, content_type, content_hash, content, version_number, byte_size) VALUES ($1, $2, $3, $4, $5, $6) ON CONFLICT (source_id, content_type, content_hash) DO NOTHING",
        )
        .bind(source_id)
        .bind(content_type)
        .bind(content_hash)
        .bind(content)
        .bind(next_version)
        .bind(byte_size)
        .execute(&self.pool)
        .await?;

        // Fetch the row (either just-inserted or existing dedup match)
        let row = sqlx::query_as::<_, ContentVersionRow>(
            "SELECT * FROM content_versions WHERE source_id = $1 AND content_type = $2 AND content_hash = $3",
        )
        .bind(source_id)
        .bind(content_type)
        .bind(content_hash)
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }

    async fn create_run_snapshot(
        &self,
        run_id: Uuid,
        step_id: Uuid,
        content_type: &str,
        role: &str,
        content_version_id: Uuid,
        source_id: Uuid,
    ) -> Result<RunSnapshotRow> {
        let row = sqlx::query_as::<_, RunSnapshotRow>(
            "INSERT INTO run_snapshots (run_id, step_id, content_type, role, content_version_id, source_id) VALUES ($1, $2, $3, $4, $5, $6) ON CONFLICT (run_id, step_id, content_type, role) DO UPDATE SET content_version_id = EXCLUDED.content_version_id, source_id = EXCLUDED.source_id RETURNING *",
        )
        .bind(run_id)
        .bind(step_id)
        .bind(content_type)
        .bind(role)
        .bind(content_version_id)
        .bind(source_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn get_run_snapshot(
        &self,
        run_id: Uuid,
        step_id: Uuid,
        content_type: &str,
        role: &str,
    ) -> Result<Option<RunSnapshotRow>> {
        let row = sqlx::query_as::<_, RunSnapshotRow>(
            "SELECT * FROM run_snapshots WHERE run_id = $1 AND step_id = $2 AND content_type = $3 AND role = $4",
        )
        .bind(run_id)
        .bind(step_id)
        .bind(content_type)
        .bind(role)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    async fn list_run_snapshots(&self, run_id: Uuid) -> Result<Vec<RunSnapshotRow>> {
        let rows = sqlx::query_as::<_, RunSnapshotRow>(
            "SELECT * FROM run_snapshots WHERE run_id = $1 ORDER BY created_at",
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn resolve_document_version_by_def(
        &self,
        def_id: Uuid,
        run_id: Uuid,
    ) -> Result<Option<ContentVersionRow>> {
        let row = sqlx::query_as::<_, ContentVersionRow>(
            "SELECT cv.* FROM content_versions cv \
             JOIN run_snapshots rs ON rs.content_version_id = cv.id \
             JOIN protocol_document_defs pdd ON rs.source_id = pdd.document_id \
             WHERE pdd.id = $1 AND rs.run_id = $2 AND rs.content_type = 'document' AND rs.role = 'output'",
        )
        .bind(def_id)
        .bind(run_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    async fn list_envelope_snapshots_for_run(
        &self,
        run_id: Uuid,
    ) -> Result<Vec<EnvelopeSnapshotRow>> {
        let rows = sqlx::query_as::<_, EnvelopeSnapshotRow>(
            "SELECT rs.step_id, cv.content, rs.source_id \
             FROM run_snapshots rs \
             JOIN content_versions cv ON cv.id = rs.content_version_id \
             WHERE rs.run_id = $1 AND rs.content_type = 'envelope' AND rs.role = 'output' \
             ORDER BY rs.created_at ASC",
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn get_latest_envelope_for_step(&self, step_id: Uuid) -> Result<Option<String>> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT cv.content \
             FROM content_versions cv \
             JOIN run_snapshots rs ON cv.id = rs.content_version_id \
             WHERE rs.step_id = $1 AND rs.content_type = 'envelope' AND rs.role = 'output' \
             ORDER BY cv.created_at DESC \
             LIMIT 1",
        )
        .bind(step_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|(content,)| content))
    }
}
