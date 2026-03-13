use anyhow::Result;
use async_trait::async_trait;
use uuid::Uuid;

use crate::db::traits::{SystemFileRepo, UpsertSystemFileInput};
use crate::db::SystemFileRow;

use super::PgRepo;

#[async_trait]
impl SystemFileRepo for PgRepo {
    async fn upsert_file(&self, input: UpsertSystemFileInput) -> Result<SystemFileRow> {
        let row: SystemFileRow = sqlx::query_as(
            r#"
            INSERT INTO system_files (workflow_id, path, media_type, description, tags, produced_by, produced_by_agent, size_bytes)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (workflow_id, path) DO UPDATE SET
                media_type = EXCLUDED.media_type,
                description = EXCLUDED.description,
                tags = EXCLUDED.tags,
                produced_by = EXCLUDED.produced_by,
                produced_by_agent = EXCLUDED.produced_by_agent,
                size_bytes = EXCLUDED.size_bytes,
                version = system_files.version + 1,
                updated_at = NOW()
            RETURNING id, workflow_id, path, media_type, description, tags, produced_by, produced_by_agent, version, size_bytes, created_at, updated_at
            "#,
        )
        .bind(input.workflow_id)
        .bind(&input.path)
        .bind(&input.media_type)
        .bind(&input.description)
        .bind(&input.tags)
        .bind(input.produced_by)
        .bind(&input.produced_by_agent)
        .bind(input.size_bytes)
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }

    async fn get_file(&self, workflow_id: Uuid, path: &str) -> Result<Option<SystemFileRow>> {
        let row: Option<SystemFileRow> = sqlx::query_as(
            r#"
            SELECT id, workflow_id, path, media_type, description, tags, produced_by, produced_by_agent, version, size_bytes, created_at, updated_at
            FROM system_files
            WHERE workflow_id = $1 AND path = $2
            "#,
        )
        .bind(workflow_id)
        .bind(path)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row)
    }

    async fn list_files(&self, workflow_id: Uuid, prefix: &str) -> Result<Vec<SystemFileRow>> {
        let like_pattern = format!("{}%", prefix);
        let rows: Vec<SystemFileRow> = sqlx::query_as(
            r#"
            SELECT id, workflow_id, path, media_type, description, tags, produced_by, produced_by_agent, version, size_bytes, created_at, updated_at
            FROM system_files
            WHERE workflow_id = $1 AND path LIKE $2
            ORDER BY path
            "#,
        )
        .bind(workflow_id)
        .bind(&like_pattern)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    async fn delete_file(&self, workflow_id: Uuid, path: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM system_files WHERE workflow_id = $1 AND path = $2")
            .bind(workflow_id)
            .bind(path)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    async fn delete_by_prefix(&self, workflow_id: Uuid, prefix: &str) -> Result<u64> {
        let like_pattern = format!("{}%", prefix);
        let result =
            sqlx::query("DELETE FROM system_files WHERE workflow_id = $1 AND path LIKE $2")
                .bind(workflow_id)
                .bind(&like_pattern)
                .execute(&self.pool)
                .await?;

        Ok(result.rows_affected())
    }

    async fn list_by_producer(
        &self,
        workflow_id: Uuid,
        step_id: Uuid,
    ) -> Result<Vec<SystemFileRow>> {
        let rows: Vec<SystemFileRow> = sqlx::query_as(
            r#"
            SELECT id, workflow_id, path, media_type, description, tags, produced_by, produced_by_agent, version, size_bytes, created_at, updated_at
            FROM system_files
            WHERE workflow_id = $1 AND produced_by = $2
            ORDER BY path
            "#,
        )
        .bind(workflow_id)
        .bind(step_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }
}
