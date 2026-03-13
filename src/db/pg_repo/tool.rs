use anyhow::Result;
use async_trait::async_trait;
use uuid::Uuid;

use crate::db::traits::ToolRepo;
use crate::db::ToolRow;

use super::PgRepo;

#[derive(sqlx::FromRow)]
struct PgToolRow {
    id: Uuid,
    name: String,
    display_name: String,
    description: String,
    parameters: serde_json::Value,
    created_at: chrono::DateTime<chrono::Utc>,
    version: i32,
}

fn tool_row_from_pg(r: PgToolRow) -> ToolRow {
    ToolRow {
        id: r.id,
        name: r.name,
        display_name: r.display_name,
        description: r.description,
        parameters: r.parameters,
        created_at: r.created_at,
        version: r.version,
    }
}

/// Intermediate row for the agent-tools JOIN query (includes owning agent_id).
#[derive(sqlx::FromRow)]
struct PgAgentToolJoinRow {
    agent_id: Uuid,
    id: Uuid,
    name: String,
    display_name: String,
    description: String,
    parameters: serde_json::Value,
    created_at: chrono::DateTime<chrono::Utc>,
    version: i32,
}

#[async_trait]
impl ToolRepo for PgRepo {
    async fn list_tools(&self) -> Result<Vec<ToolRow>> {
        let rows = sqlx::query_as::<_, PgToolRow>("SELECT id, name, display_name, description, parameters, created_at, version FROM tools ORDER BY name")
            .fetch_all(&self.pool)
            .await?;

        Ok(rows.into_iter().map(tool_row_from_pg).collect())
    }

    async fn get_tool(&self, tool_id: Uuid) -> Result<Option<ToolRow>> {
        let row = sqlx::query_as::<_, PgToolRow>("SELECT id, name, display_name, description, parameters, created_at, version FROM tools WHERE id = $1")
            .bind(tool_id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.map(tool_row_from_pg))
    }

    async fn upsert_tool(&self, tool: ToolRow) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO tools (id, name, display_name, description, parameters)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (id) DO UPDATE SET
                name = EXCLUDED.name,
                display_name = EXCLUDED.display_name,
                description = EXCLUDED.description,
                parameters = EXCLUDED.parameters,
                version = tools.version + 1
        "#,
        )
        .bind(tool.id)
        .bind(&tool.name)
        .bind(&tool.display_name)
        .bind(&tool.description)
        .bind(&tool.parameters)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn delete_tool(&self, tool_id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM tools WHERE id = $1")
            .bind(tool_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn get_agent_tools(&self, agent_id: Uuid) -> Result<Vec<ToolRow>> {
        let rows = sqlx::query_as::<_, PgToolRow>(
            "SELECT t.id, t.name, t.display_name, t.description, t.parameters, t.created_at, t.version FROM tools t INNER JOIN agent_tools at ON t.id = at.tool_id WHERE at.agent_id = $1 ORDER BY t.name",
        )
        .bind(agent_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(tool_row_from_pg).collect())
    }

    async fn get_tools_for_agents(&self, agent_ids: &[Uuid]) -> Result<Vec<(Uuid, ToolRow)>> {
        let rows = sqlx::query_as::<_, PgAgentToolJoinRow>(
            "SELECT at.agent_id, t.id, t.name, t.display_name, t.description, t.parameters, t.created_at, t.version \
             FROM tools t INNER JOIN agent_tools at ON t.id = at.tool_id \
             WHERE at.agent_id = ANY($1) ORDER BY t.name",
        )
        .bind(agent_ids)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| {
                let agent_id = r.agent_id;
                let tool = ToolRow {
                    id: r.id,
                    name: r.name,
                    display_name: r.display_name,
                    description: r.description,
                    parameters: r.parameters,
                    created_at: r.created_at,
                    version: r.version,
                };
                (agent_id, tool)
            })
            .collect())
    }

    async fn set_agent_tools(&self, agent_id: Uuid, tool_ids: Vec<Uuid>) -> Result<()> {
        run_serializable!(self.pool, |tx| {
            sqlx::query("DELETE FROM agent_tools WHERE agent_id = $1")
                .bind(agent_id)
                .execute(&mut *tx)
                .await?;

            for tool_id in &tool_ids {
                sqlx::query("INSERT INTO agent_tools (agent_id, tool_id) VALUES ($1, $2)")
                    .bind(agent_id)
                    .bind(tool_id)
                    .execute(&mut *tx)
                    .await?;
            }
            Ok(())
        })
    }

    async fn seed_builtin_tools(&self) -> Result<()> {
        for tool in crate::server::tools::execution::builtin_tool_rows() {
            sqlx::query(
                r#"
                INSERT INTO tools (id, name, display_name, description, parameters)
                VALUES ($1, $2, $3, $4, $5)
                ON CONFLICT (name) DO NOTHING
            "#,
            )
            .bind(tool.id)
            .bind(&tool.name)
            .bind(&tool.display_name)
            .bind(&tool.description)
            .bind(&tool.parameters)
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }
}
