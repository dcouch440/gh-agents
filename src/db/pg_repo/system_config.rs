use anyhow::Result;
use async_trait::async_trait;
use uuid::Uuid;

use crate::db::traits::SystemConfigRepo;
use crate::db::SystemConfigRow;

use super::PgRepo;

#[async_trait]
impl SystemConfigRepo for PgRepo {
    async fn get_system_config(&self, config_key: &str) -> Result<Option<SystemConfigRow>> {
        let row = sqlx::query_as::<_, SystemConfigRow>(
            "SELECT id, config_type, config_key, config_value, description, created_by, created_at, updated_at
             FROM system_config
             WHERE config_key = $1"
        )
        .bind(config_key)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    async fn list_system_configs(
        &self,
        config_type: Option<String>,
    ) -> Result<Vec<SystemConfigRow>> {
        let rows = if let Some(ct) = config_type {
            sqlx::query_as::<_, SystemConfigRow>(
                "SELECT id, config_type, config_key, config_value, description, created_by, created_at, updated_at
                 FROM system_config
                 WHERE config_type = $1
                 ORDER BY config_key"
            )
            .bind(ct)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, SystemConfigRow>(
                "SELECT id, config_type, config_key, config_value, description, created_by, created_at, updated_at
                 FROM system_config
                 ORDER BY config_type, config_key"
            )
            .fetch_all(&self.pool)
            .await?
        };
        Ok(rows)
    }

    async fn upsert_system_config(
        &self,
        config_type: &str,
        config_key: &str,
        config_value: &serde_json::Value,
        description: Option<String>,
        created_by: Option<Uuid>,
    ) -> Result<SystemConfigRow> {
        let row = sqlx::query_as::<_, SystemConfigRow>(
            "INSERT INTO system_config (config_type, config_key, config_value, description, created_by)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (config_key) DO UPDATE SET
                config_value = EXCLUDED.config_value,
                description = COALESCE(EXCLUDED.description, system_config.description),
                updated_at = NOW()
             RETURNING id, config_type, config_key, config_value, description, created_by, created_at, updated_at"
        )
        .bind(config_type)
        .bind(config_key)
        .bind(config_value)
        .bind(description)
        .bind(created_by)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn delete_system_config(&self, config_key: &str) -> Result<()> {
        sqlx::query("DELETE FROM system_config WHERE config_key = $1")
            .bind(config_key)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn get_execution_constraints(
        &self,
    ) -> Result<std::collections::HashMap<String, serde_json::Value>> {
        let rows = sqlx::query_as::<_, SystemConfigRow>(
            "SELECT id, config_type, config_key, config_value, description, created_by, created_at, updated_at
             FROM system_config
             WHERE config_type = 'constraint'"
        )
        .fetch_all(&self.pool)
        .await?;

        let mut constraints = std::collections::HashMap::new();
        for row in rows {
            constraints.insert(row.config_key, row.config_value);
        }
        Ok(constraints)
    }

    async fn get_unsafe_operations_enabled(&self) -> Result<bool> {
        let row = sqlx::query_as::<_, SystemConfigRow>(
            "SELECT id, config_type, config_key, config_value, description, created_by, created_at, updated_at
             FROM system_config
             WHERE config_key = 'unsafe_operations_enabled'"
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.and_then(|r| r.config_value.as_bool()).unwrap_or(false))
    }
}
