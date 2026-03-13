use anyhow::Result;
use async_trait::async_trait;
use uuid::Uuid;

use crate::db::traits::AgentRepo;
use crate::db::{AgentRow, DocumentRow};
use crate::types::UserId;

use super::PgRepo;

#[derive(sqlx::FromRow)]
struct PgAgentRow {
    id: Uuid,
    user_id: Option<Uuid>,
    name: String,
    system_prompt: String,
    persona_style: Option<String>,
    model_provider: String,
    model_id: String,
    model_max_tokens: i32,
    model_temperature: f32,
    status: Option<String>,
    output_schema_id: Option<Uuid>,
    version: i32,
    default_reasoning_trace: Option<bool>,
    is_system: bool,
}

fn agent_row_from_pg(r: PgAgentRow) -> AgentRow {
    AgentRow {
        id: r.id,
        user_id: r.user_id,
        tier: None,
        name: r.name,
        system_prompt: r.system_prompt,
        persona_style: r.persona_style,
        model_provider: r.model_provider,
        model_id: r.model_id,
        model_max_tokens: r.model_max_tokens,
        model_temperature: r.model_temperature,
        status: r.status,
        output_schema_id: r.output_schema_id,
        version: r.version,
        default_reasoning_trace: r.default_reasoning_trace,
        is_system: r.is_system,
    }
}

#[async_trait]
impl AgentRepo for PgRepo {
    async fn list_persisted_agents(&self, user_id: UserId) -> Result<Vec<AgentRow>> {
        let rows = sqlx::query_as::<_, PgAgentRow>(
            "SELECT id, user_id, name, system_prompt, persona_style, model_provider, model_id, model_max_tokens, model_temperature, status, output_schema_id, version, default_reasoning_trace, is_system FROM agents WHERE user_id = $1 OR user_id IS NULL",
        )
        .bind(user_id.0)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(agent_row_from_pg).collect())
    }

    async fn get_persisted_agent(&self, agent_id: Uuid) -> Result<Option<AgentRow>> {
        let row = sqlx::query_as::<_, PgAgentRow>(
            "SELECT id, user_id, name, system_prompt, persona_style, model_provider, model_id, model_max_tokens, model_temperature, status, output_schema_id, version, default_reasoning_trace, is_system FROM agents WHERE id = $1",
        )
        .bind(agent_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(agent_row_from_pg))
    }

    async fn get_agents_by_ids(&self, agent_ids: &[Uuid]) -> Result<Vec<AgentRow>> {
        let rows = sqlx::query_as::<_, PgAgentRow>(
            "SELECT id, user_id, name, system_prompt, persona_style, model_provider, model_id, model_max_tokens, model_temperature, status, output_schema_id, version, default_reasoning_trace, is_system FROM agents WHERE id = ANY($1)",
        )
        .bind(agent_ids)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(agent_row_from_pg).collect())
    }

    async fn upsert_agent(&self, agent: AgentRow) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO agents (id, user_id, name, system_prompt, persona_style, model_provider, model_id, model_max_tokens, model_temperature, status, output_schema_id, default_reasoning_trace, is_system)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            ON CONFLICT (id) DO UPDATE SET
                name = EXCLUDED.name,
                system_prompt = EXCLUDED.system_prompt,
                persona_style = EXCLUDED.persona_style,
                model_provider = EXCLUDED.model_provider,
                model_id = EXCLUDED.model_id,
                model_max_tokens = EXCLUDED.model_max_tokens,
                model_temperature = EXCLUDED.model_temperature,
                status = EXCLUDED.status,
                output_schema_id = EXCLUDED.output_schema_id,
                default_reasoning_trace = EXCLUDED.default_reasoning_trace,
                is_system = EXCLUDED.is_system,
                version = agents.version + 1
        "#,
        )
        .bind(agent.id)
        .bind(agent.user_id)
        .bind(&agent.name)
        .bind(&agent.system_prompt)
        .bind(&agent.persona_style)
        .bind(&agent.model_provider)
        .bind(&agent.model_id)
        .bind(agent.model_max_tokens)
        .bind(agent.model_temperature)
        .bind(&agent.status)
        .bind(agent.output_schema_id)
        .bind(agent.default_reasoning_trace)
        .bind(agent.is_system)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn delete_persisted_agent(&self, agent_id: Uuid) -> Result<()> {
        // First, nullify agent_id in sessions that reference this agent
        sqlx::query("UPDATE chat_sessions SET agent_id = NULL WHERE agent_id = $1")
            .bind(agent_id)
            .execute(&self.pool)
            .await?;

        // Now safe to delete the agent
        sqlx::query("DELETE FROM agents WHERE id = $1")
            .bind(agent_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn get_agent_context(&self, agent_id: Uuid) -> Result<Vec<DocumentRow>> {
        let rows = sqlx::query_as::<_, DocumentRow>(
            "SELECT d.id, d.user_id, d.session_id, d.title, d.content, d.summary, d.doc_type, d.ref_tag, d.tags, d.created_at, d.updated_at, d.workflow_id, d.target_length, d.is_static, d.source_protocol_step_id FROM documents d INNER JOIN agent_context ac ON d.id = ac.document_id WHERE ac.agent_id = $1 ORDER BY d.title",
        )
        .bind(agent_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn set_agent_context(&self, agent_id: Uuid, document_ids: Vec<Uuid>) -> Result<()> {
        run_serializable!(self.pool, |tx| {
            sqlx::query("DELETE FROM agent_context WHERE agent_id = $1")
                .bind(agent_id)
                .execute(&mut *tx)
                .await?;

            for doc_id in &document_ids {
                sqlx::query("INSERT INTO agent_context (agent_id, document_id) VALUES ($1, $2)")
                    .bind(agent_id)
                    .bind(doc_id)
                    .execute(&mut *tx)
                    .await?;
            }
            Ok(())
        })
    }

    async fn get_agent_guidances(
        &self,
        agent_id: Uuid,
        step_id: Option<Uuid>,
    ) -> Result<Vec<crate::db::AgentGuidanceRow>> {
        let rows = sqlx::query_as::<_, crate::db::AgentGuidanceRow>(
            "SELECT id, agent_id, workflow_step_id, suggestions, source, version, \
                    is_active, created_at, updated_at \
             FROM agent_guidances \
             WHERE agent_id = $1 AND is_active = true \
               AND (workflow_step_id IS NULL OR workflow_step_id = $2) \
             ORDER BY workflow_step_id NULLS FIRST, version DESC",
        )
        .bind(agent_id)
        .bind(step_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }
}
