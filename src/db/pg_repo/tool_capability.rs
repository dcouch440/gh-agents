use anyhow::Result;
use async_trait::async_trait;
use uuid::Uuid;

use crate::db::traits::ToolCapabilityRepo;
use crate::db::{ToolCapabilityRow, ToolRow};

use super::PgRepo;

#[async_trait]
impl ToolCapabilityRepo for PgRepo {
    async fn get_tool_capabilities(&self) -> Result<Vec<ToolCapabilityRow>> {
        let rows = sqlx::query_as::<_, ToolCapabilityRow>(
            "SELECT id, capability_key, display_name, category, safety_level, description, created_at
             FROM tool_capabilities
             ORDER BY category, capability_key"
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn get_tool_capability(&self, id: Uuid) -> Result<Option<ToolCapabilityRow>> {
        let row = sqlx::query_as::<_, ToolCapabilityRow>(
            "SELECT id, capability_key, display_name, category, safety_level, description, created_at
             FROM tool_capabilities
             WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    async fn get_tool_capability_by_key(&self, key: &str) -> Result<Option<ToolCapabilityRow>> {
        let row = sqlx::query_as::<_, ToolCapabilityRow>(
            "SELECT id, capability_key, display_name, category, safety_level, description, created_at
             FROM tool_capabilities
             WHERE capability_key = $1"
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    async fn get_capabilities_by_tool(&self, tool_id: Uuid) -> Result<Vec<ToolCapabilityRow>> {
        let rows = sqlx::query_as::<_, ToolCapabilityRow>(
            "SELECT tc.id, tc.capability_key, tc.display_name, tc.category, tc.safety_level, tc.description, tc.created_at
             FROM tool_capabilities tc
             JOIN tool_capability_assignments tca ON tc.id = tca.capability_id
             WHERE tca.tool_id = $1
             ORDER BY tc.category, tc.capability_key"
        )
        .bind(tool_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn get_tools_by_capability(&self, capability_key: &str) -> Result<Vec<ToolRow>> {
        let rows = sqlx::query_as::<_, ToolRow>(
            "SELECT t.id, t.name, t.display_name, t.description, t.parameters, t.created_at, t.version
             FROM tools t
             JOIN tool_capability_assignments tca ON t.id = tca.tool_id
             JOIN tool_capabilities tc ON tc.id = tca.capability_id
             WHERE tc.capability_key = $1
             ORDER BY t.name",
        )
        .bind(capability_key)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn get_tools_by_capabilities(&self, capability_keys: &[String]) -> Result<Vec<ToolRow>> {
        let rows = sqlx::query_as::<_, ToolRow>(
            "SELECT DISTINCT t.id, t.name, t.display_name, t.description, t.parameters, t.created_at, t.version
             FROM tools t
             JOIN tool_capability_assignments tca ON t.id = tca.tool_id
             JOIN tool_capabilities tc ON tc.id = tca.capability_id
             WHERE tc.capability_key = ANY($1)
             ORDER BY t.name",
        )
        .bind(capability_keys)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn assign_capability_to_tool(&self, tool_id: Uuid, capability_id: Uuid) -> Result<()> {
        sqlx::query(
            "INSERT INTO tool_capability_assignments (tool_id, capability_id)
             VALUES ($1, $2)
             ON CONFLICT DO NOTHING",
        )
        .bind(tool_id)
        .bind(capability_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn remove_capability_from_tool(&self, tool_id: Uuid, capability_id: Uuid) -> Result<()> {
        sqlx::query(
            "DELETE FROM tool_capability_assignments
             WHERE tool_id = $1 AND capability_id = $2",
        )
        .bind(tool_id)
        .bind(capability_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn set_tool_capabilities(&self, tool_id: Uuid, capability_ids: &[Uuid]) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        // Delete existing assignments
        sqlx::query("DELETE FROM tool_capability_assignments WHERE tool_id = $1")
            .bind(tool_id)
            .execute(&mut *tx)
            .await?;

        // Insert new assignments
        for capability_id in capability_ids {
            sqlx::query(
                "INSERT INTO tool_capability_assignments (tool_id, capability_id)
                 VALUES ($1, $2)",
            )
            .bind(tool_id)
            .bind(capability_id)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn get_mode_capabilities(&self, mode_id: Uuid) -> Result<Vec<ToolCapabilityRow>> {
        let rows = sqlx::query_as::<_, ToolCapabilityRow>(
            "SELECT tc.id, tc.capability_key, tc.display_name, tc.category, tc.safety_level, tc.description, tc.created_at
             FROM tool_capabilities tc
             JOIN mode_required_capabilities mrc ON tc.id = mrc.capability_id
             WHERE mrc.mode_id = $1 AND mrc.is_required = true
             ORDER BY tc.category, tc.capability_key"
        )
        .bind(mode_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn set_mode_capabilities(
        &self,
        mode_id: Uuid,
        capability_ids: &[Uuid],
        is_required: bool,
    ) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        // Delete existing requirements
        sqlx::query("DELETE FROM mode_required_capabilities WHERE mode_id = $1")
            .bind(mode_id)
            .execute(&mut *tx)
            .await?;

        // Insert new requirements
        for capability_id in capability_ids {
            sqlx::query(
                "INSERT INTO mode_required_capabilities (mode_id, capability_id, is_required)
                 VALUES ($1, $2, $3)",
            )
            .bind(mode_id)
            .bind(capability_id)
            .bind(is_required)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }
}
