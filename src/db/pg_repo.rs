//! PostgreSQL implementation of repository traits.

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::traits::{
    AgentExecutionRepo, ContextStoreRepo, DocumentRepo, MergeQueueRepo, ModelSpendRow, OutputSchemaRepo, PipelineStageMemberRepo, PromptTemplateRepo, ResultRepo, RoomMemberInput, RoomRepo,
    RouterRequestRepo, ServerRepo, TokenLedgerRepo, ToolRouterRepo, UserRepo, WorkflowRepo,
};
use crate::db::{
    AgentExecutionRow, AgentModeRow, AgentRow, ChatMessageRow, ContextStoreRow, DocumentRow, DocumentSearchResult, ExecutionMessageRow, OutputSchemaRow, PipelineRow, PipelineRunRow,
    PipelineStageMemberRow, PipelineStageRow, PromptTemplateRow, ResultRow, RoomMemberRow, RoomRow, RoomSessionRow, RoomTranscriptEntry, RouterRequestRow, SessionRow, StageExecutionRow,
    StepDocumentRow, TokenLedgerRow, ToolRouterRow, ToolRow, WorkflowRow, WorkflowStepEdgeRow, WorkflowStepRow,
};
use crate::github::{PrQueueEntry, QueueError as MergeQueueError};
use crate::types::{Task, User, UserId};

/// Production repository backed by PostgreSQL.
#[derive(Clone)]
pub struct PgRepo {
    pool: PgPool,
}

impl PgRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

#[async_trait]
impl MergeQueueRepo for PgRepo {
    async fn insert_queue_entry(&self, id: Uuid, owner: String, repo: String, pr_number: u32, position: u32, now: DateTime<Utc>) -> Result<(), MergeQueueError> {
        sqlx::query(
            r#"
            INSERT INTO pr_merge_queue (
                id, repo_owner, repo_name, pr_number,
                queue_position, status, created_at, updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (repo_owner, repo_name, pr_number)
            DO UPDATE SET updated_at = excluded.updated_at
            "#,
        )
        .bind(id)
        .bind(&owner)
        .bind(&repo)
        .bind(pr_number as i32)
        .bind(position as i32)
        .bind("pending")
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_next_position(&self, owner: String, repo: String) -> Result<u32, MergeQueueError> {
        let row: Option<(i32,)> = sqlx::query_as(
            r#"
            SELECT COALESCE(MAX(queue_position), 0) + 1
            FROM pr_merge_queue
            WHERE repo_owner = $1 AND repo_name = $2
            "#,
        )
        .bind(&owner)
        .bind(&repo)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|(n,)| n as u32).unwrap_or(1))
    }

    async fn delete_queue_entry(&self, owner: String, repo: String, pr_number: u32) -> Result<bool, MergeQueueError> {
        let result = sqlx::query(
            r#"
            DELETE FROM pr_merge_queue
            WHERE repo_owner = $1 AND repo_name = $2 AND pr_number = $3
            "#,
        )
        .bind(&owner)
        .bind(&repo)
        .bind(pr_number as i32)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    async fn get_queue_entries(&self, owner: String, repo: String) -> Result<Vec<PrQueueEntry>, MergeQueueError> {
        let rows: Vec<(Uuid, String, String, i32, i32, String, Option<String>, Option<String>, DateTime<Utc>, DateTime<Utc>)> = sqlx::query_as(
            r#"
            SELECT
                id, repo_owner, repo_name, pr_number,
                queue_position, status, conflict_info,
                error_message, created_at, updated_at
            FROM pr_merge_queue
            WHERE repo_owner = $1 AND repo_name = $2
            ORDER BY queue_position ASC
            "#,
        )
        .bind(&owner)
        .bind(&repo)
        .fetch_all(&self.pool)
        .await?;

        let entries = rows
            .into_iter()
            .filter_map(|row| {
                Some(PrQueueEntry {
                    id: row.0,
                    repo_owner: row.1,
                    repo_name: row.2,
                    pr_number: row.3 as u32,
                    queue_position: row.4 as u32,
                    status: row.5.parse().ok()?,
                    conflict_info: row.6.and_then(|s| serde_json::from_str(&s).ok()),
                    error_message: row.7,
                    created_at: row.8,
                    updated_at: row.9,
                })
            })
            .collect();

        Ok(entries)
    }

    async fn update_entry_status(&self, owner: String, repo: String, pr_number: u32, status: String, error_message: Option<String>, now: DateTime<Utc>) -> Result<bool, MergeQueueError> {
        let result = sqlx::query(
            r#"
            UPDATE pr_merge_queue
            SET status = $1, error_message = $2, updated_at = $3
            WHERE repo_owner = $4 AND repo_name = $5 AND pr_number = $6
            "#,
        )
        .bind(&status)
        .bind(error_message.as_deref())
        .bind(now)
        .bind(&owner)
        .bind(&repo)
        .bind(pr_number as i32)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    async fn set_entry_conflict(&self, owner: String, repo: String, pr_number: u32, conflict_json: String, now: DateTime<Utc>) -> Result<bool, MergeQueueError> {
        let result = sqlx::query(
            r#"
            UPDATE pr_merge_queue
            SET status = $1, conflict_info = $2, updated_at = $3
            WHERE repo_owner = $4 AND repo_name = $5 AND pr_number = $6
            "#,
        )
        .bind("conflict")
        .bind(&conflict_json)
        .bind(now)
        .bind(&owner)
        .bind(&repo)
        .bind(pr_number as i32)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    async fn update_entry_position(&self, id: Uuid, position: u32, now: DateTime<Utc>) -> Result<(), MergeQueueError> {
        sqlx::query(
            r#"
            UPDATE pr_merge_queue
            SET queue_position = $1, updated_at = $2
            WHERE id = $3
            "#,
        )
        .bind(position as i32)
        .bind(now)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn reset_interrupted(&self, owner: String, repo: String, now: DateTime<Utc>) -> Result<u32, MergeQueueError> {
        let result = sqlx::query(
            r#"
            UPDATE pr_merge_queue
            SET status = 'pending', updated_at = $1
            WHERE repo_owner = $2 AND repo_name = $3
            AND status = 'in_progress'
            "#,
        )
        .bind(now)
        .bind(&owner)
        .bind(&repo)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() as u32)
    }

    async fn cleanup_old(&self, owner: String, repo: String, cutoff: DateTime<Utc>) -> Result<u32, MergeQueueError> {
        let result = sqlx::query(
            r#"
            DELETE FROM pr_merge_queue
            WHERE repo_owner = $1 AND repo_name = $2
            AND status IN ('merged', 'skipped')
            AND updated_at < $3
            "#,
        )
        .bind(&owner)
        .bind(&repo)
        .bind(cutoff)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() as u32)
    }
}

#[async_trait]
impl ServerRepo for PgRepo {
    async fn health_check(&self) -> bool {
        sqlx::query("SELECT 1").fetch_one(&self.pool).await.is_ok()
    }

    async fn list_tasks(&self, user_id: UserId, status: Option<String>, limit: Option<u32>) -> Result<Vec<Task>> {
        crate::db::list_tasks(&self.pool, user_id, status.as_deref(), limit).await
    }

    async fn get_task_by_uuid(&self, user_id: UserId, id: Uuid) -> Result<Option<Task>> {
        crate::db::get_task_by_uuid(&self.pool, user_id, id).await
    }

    async fn insert_task(&self, user_id: UserId, task: Task) -> Result<()> {
        crate::db::insert_task(&self.pool, user_id, &task).await
    }

    async fn insert_chat_message(&self, user_id: UserId, id: Uuid, role: String, content: String) -> Result<()> {
        crate::db::insert_chat_message(&self.pool, user_id, &id, &role, &content).await
    }

    async fn get_chat_history(&self, user_id: UserId, limit: u32, offset: u32) -> Result<Vec<ChatMessageRow>> {
        crate::db::get_chat_history(&self.pool, user_id, limit, offset).await
    }

    async fn clear_chat_history(&self, user_id: UserId) -> Result<()> {
        crate::db::clear_chat_history(&self.pool, user_id).await
    }

    async fn has_password(&self) -> Result<bool> {
        crate::db::has_password(&self.pool).await
    }

    async fn set_password(&self, password_hash: String) -> Result<()> {
        crate::db::set_password(&self.pool, &password_hash).await
    }

    async fn get_password(&self) -> Result<Option<String>> {
        crate::db::get_password(&self.pool).await
    }

    // --- Agent persistence ---

    async fn list_persisted_agents(&self, user_id: UserId) -> Result<Vec<AgentRow>> {
        let rows = sqlx::query_as::<_, PgAgentRow>(
            "SELECT id, tier, name, system_prompt, persona_style, model_provider, model_id, model_max_tokens, model_temperature, status, router_mode, version FROM agents WHERE user_id = $1",
        )
        .bind(user_id.0)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(agent_row_from_pg).collect())
    }

    async fn get_persisted_agent(&self, agent_id: Uuid) -> Result<Option<AgentRow>> {
        let row = sqlx::query_as::<_, PgAgentRow>(
            "SELECT id, tier, name, system_prompt, persona_style, model_provider, model_id, model_max_tokens, model_temperature, status, router_mode, version FROM agents WHERE id = $1",
        )
        .bind(agent_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(agent_row_from_pg))
    }

    async fn upsert_agent(&self, user_id: UserId, agent: AgentRow) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO agents (id, user_id, tier, name, system_prompt, persona_style, model_provider, model_id, model_max_tokens, model_temperature, status, router_mode)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            ON CONFLICT (id) DO UPDATE SET
                tier = EXCLUDED.tier,
                name = EXCLUDED.name,
                system_prompt = EXCLUDED.system_prompt,
                persona_style = EXCLUDED.persona_style,
                model_provider = EXCLUDED.model_provider,
                model_id = EXCLUDED.model_id,
                model_max_tokens = EXCLUDED.model_max_tokens,
                model_temperature = EXCLUDED.model_temperature,
                status = EXCLUDED.status,
                router_mode = EXCLUDED.router_mode,
                version = agents.version + 1
        "#,
        )
        .bind(agent.id)
        .bind(user_id.0)
        .bind(&agent.tier)
        .bind(&agent.name)
        .bind(&agent.system_prompt)
        .bind(&agent.persona_style)
        .bind(&agent.model_provider)
        .bind(&agent.model_id)
        .bind(agent.model_max_tokens)
        .bind(agent.model_temperature)
        .bind(&agent.status)
        .bind(agent.router_mode)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn delete_persisted_agent(&self, agent_id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM agents WHERE id = $1").bind(agent_id).execute(&self.pool).await?;
        Ok(())
    }

    // --- Tool persistence ---

    async fn list_tools(&self, user_id: UserId) -> Result<Vec<ToolRow>> {
        let rows = sqlx::query_as::<_, PgToolRow>("SELECT id, user_id, name, display_name, description, parameters, created_at, version FROM tools WHERE user_id = $1 ORDER BY name")
            .bind(user_id.0)
            .fetch_all(&self.pool)
            .await?;

        Ok(rows.into_iter().map(tool_row_from_pg).collect())
    }

    async fn get_tool(&self, tool_id: Uuid) -> Result<Option<ToolRow>> {
        let row = sqlx::query_as::<_, PgToolRow>("SELECT id, user_id, name, display_name, description, parameters, created_at, version FROM tools WHERE id = $1")
            .bind(tool_id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.map(tool_row_from_pg))
    }

    async fn upsert_tool(&self, user_id: UserId, tool: ToolRow) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO tools (id, user_id, name, display_name, description, parameters)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (id) DO UPDATE SET
                name = EXCLUDED.name,
                display_name = EXCLUDED.display_name,
                description = EXCLUDED.description,
                parameters = EXCLUDED.parameters,
                version = tools.version + 1
        "#,
        )
        .bind(tool.id)
        .bind(user_id.0)
        .bind(&tool.name)
        .bind(&tool.display_name)
        .bind(&tool.description)
        .bind(&tool.parameters)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn delete_tool(&self, tool_id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM tools WHERE id = $1").bind(tool_id).execute(&self.pool).await?;
        Ok(())
    }

    async fn get_agent_tools(&self, agent_id: Uuid) -> Result<Vec<ToolRow>> {
        let rows = sqlx::query_as::<_, PgToolRow>(
            "SELECT t.id, t.user_id, t.name, t.display_name, t.description, t.parameters, t.created_at, t.version FROM tools t INNER JOIN agent_tools at ON t.id = at.tool_id WHERE at.agent_id = $1 ORDER BY t.name",
        )
        .bind(agent_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(tool_row_from_pg).collect())
    }

    async fn set_agent_tools(&self, agent_id: Uuid, tool_ids: Vec<Uuid>) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        sqlx::query("DELETE FROM agent_tools WHERE agent_id = $1").bind(agent_id).execute(&mut *tx).await?;

        for tool_id in tool_ids {
            sqlx::query("INSERT INTO agent_tools (agent_id, tool_id) VALUES ($1, $2)")
                .bind(agent_id)
                .bind(tool_id)
                .execute(&mut *tx)
                .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn seed_builtin_tools(&self, user_id: UserId) -> Result<()> {
        for tool in crate::agents::execution_tools::builtin_tool_rows() {
            sqlx::query(
                r#"
                INSERT INTO tools (id, user_id, name, display_name, description, parameters)
                VALUES ($1, $2, $3, $4, $5, $6)
                ON CONFLICT (user_id, name) DO NOTHING
            "#,
            )
            .bind(tool.id)
            .bind(user_id.0)
            .bind(&tool.name)
            .bind(&tool.display_name)
            .bind(&tool.description)
            .bind(&tool.parameters)
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    // --- Agent context (document linkage) ---

    async fn get_agent_context(&self, agent_id: Uuid) -> Result<Vec<DocumentRow>> {
        let rows = sqlx::query_as::<_, DocumentRow>(
            "SELECT d.id, d.user_id, d.session_id, d.title, d.content, d.summary, d.doc_type, d.ref_tag, d.tags, d.created_at, d.updated_at FROM documents d INNER JOIN agent_context ac ON d.id = ac.document_id WHERE ac.agent_id = $1 ORDER BY d.title",
        )
        .bind(agent_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn set_agent_context(&self, agent_id: Uuid, document_ids: Vec<Uuid>) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        sqlx::query("DELETE FROM agent_context WHERE agent_id = $1").bind(agent_id).execute(&mut *tx).await?;

        for doc_id in document_ids {
            sqlx::query("INSERT INTO agent_context (agent_id, document_id) VALUES ($1, $2)")
                .bind(agent_id)
                .bind(doc_id)
                .execute(&mut *tx)
                .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    // --- Pipeline persistence ---

    async fn list_pipelines(&self, user_id: UserId) -> Result<Vec<PipelineRow>> {
        let rows: Vec<(Uuid, String)> = sqlx::query_as("SELECT id, name FROM pipelines WHERE user_id = $1").bind(user_id.0).fetch_all(&self.pool).await?;

        Ok(rows.into_iter().map(|(id, name)| PipelineRow { id, name }).collect())
    }

    async fn upsert_pipeline(&self, user_id: UserId, pipeline: PipelineRow) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO pipelines (id, user_id, name)
            VALUES ($1, $2, $3)
            ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name
        "#,
        )
        .bind(pipeline.id)
        .bind(user_id.0)
        .bind(&pipeline.name)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn delete_pipeline(&self, pipeline_id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM pipelines WHERE id = $1").bind(pipeline_id).execute(&self.pool).await?;
        Ok(())
    }

    async fn list_pipeline_stages(&self, pipeline_id: Uuid) -> Result<Vec<PipelineStageRow>> {
        let rows: Vec<(Uuid, i32, Option<Uuid>, Option<Uuid>, Option<String>, Option<bool>, Option<bool>, String, Option<serde_json::Value>, Option<String>, Option<serde_json::Value>)> = sqlx::query_as(
            "SELECT pipeline_id, stage_number, agent_id, cluster_id, role, approval_required, fan_out, stage_name, input_definitions, output_description, output_schema FROM pipeline_stages WHERE pipeline_id = $1 ORDER BY stage_number"
        )
        .bind(pipeline_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(
                |(pipeline_id, stage_number, agent_id, cluster_id, role, approval_required, fan_out, stage_name, input_definitions, output_description, output_schema)| PipelineStageRow {
                    pipeline_id,
                    stage_number,
                    agent_id,
                    cluster_id,
                    role,
                    approval_required,
                    fan_out,
                    stage_name,
                    input_definitions,
                    output_description,
                    output_schema,
                },
            )
            .collect())
    }

    async fn upsert_pipeline_stage(&self, stage: PipelineStageRow) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO pipeline_stages (pipeline_id, stage_number, agent_id, cluster_id, role, approval_required, fan_out, stage_name, input_definitions, output_description, output_schema)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            ON CONFLICT (pipeline_id, stage_number) DO UPDATE SET
                agent_id = EXCLUDED.agent_id,
                cluster_id = EXCLUDED.cluster_id,
                role = EXCLUDED.role,
                approval_required = EXCLUDED.approval_required,
                fan_out = EXCLUDED.fan_out,
                stage_name = EXCLUDED.stage_name,
                input_definitions = EXCLUDED.input_definitions,
                output_description = EXCLUDED.output_description,
                output_schema = EXCLUDED.output_schema
        "#,
        )
        .bind(stage.pipeline_id)
        .bind(stage.stage_number)
        .bind(stage.agent_id)
        .bind(stage.cluster_id)
        .bind(&stage.role)
        .bind(stage.approval_required)
        .bind(stage.fan_out)
        .bind(&stage.stage_name)
        .bind(&stage.input_definitions)
        .bind(&stage.output_description)
        .bind(&stage.output_schema)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // --- Session management ---

    async fn create_session(&self, user_id: UserId, session_id: Uuid, mode_id: &str, title: &str, agent_id: Option<Uuid>) -> Result<()> {
        crate::db::create_session(&self.pool, user_id, session_id, mode_id, title, agent_id).await
    }

    async fn list_sessions(&self, user_id: UserId) -> Result<Vec<SessionRow>> {
        crate::db::list_sessions(&self.pool, user_id).await
    }

    async fn get_session(&self, session_id: Uuid) -> Result<Option<SessionRow>> {
        crate::db::get_session(&self.pool, session_id).await
    }

    async fn delete_session(&self, session_id: Uuid) -> Result<()> {
        crate::db::delete_session(&self.pool, session_id).await
    }

    async fn insert_session_message(&self, user_id: UserId, session_id: Uuid, id: Uuid, role: String, content: String) -> Result<()> {
        crate::db::insert_session_message(&self.pool, user_id, session_id, &id, &role, &content).await
    }

    async fn get_session_history(&self, session_id: Uuid, limit: u32) -> Result<Vec<ChatMessageRow>> {
        crate::db::get_session_history(&self.pool, session_id, limit).await
    }

    async fn update_session_title(&self, session_id: Uuid, title: &str) -> Result<()> {
        crate::db::update_session_title(&self.pool, session_id, title).await
    }

    async fn update_session_summary(&self, session_id: Uuid, summary: &str) -> Result<()> {
        sqlx::query("UPDATE chat_sessions SET summary = $1 WHERE id = $2")
            .bind(summary)
            .bind(session_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn count_session_messages(&self, session_id: Uuid) -> Result<u32> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM chat_messages WHERE session_id = $1")
            .bind(session_id)
            .fetch_one(&self.pool)
            .await?;
        Ok(row.0 as u32)
    }

    // --- Token usage tracking ---

    async fn create_pipeline_run(&self, run: &PipelineRunRow) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO pipeline_runs (id, pipeline_id, user_id, status, initial_task, stage_outputs, current_stage, started_at, completed_at, total_input_tokens, total_output_tokens)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            "#,
        )
        .bind(run.id)
        .bind(run.pipeline_id)
        .bind(run.user_id)
        .bind(&run.status)
        .bind(&run.initial_task)
        .bind(&run.stage_outputs)
        .bind(run.current_stage)
        .bind(run.started_at)
        .bind(run.completed_at)
        .bind(run.total_input_tokens)
        .bind(run.total_output_tokens)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn update_pipeline_run(&self, run: &PipelineRunRow) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE pipeline_runs
            SET status = $2, stage_outputs = $3, current_stage = $4, completed_at = $5, total_input_tokens = $6, total_output_tokens = $7
            WHERE id = $1
            "#,
        )
        .bind(run.id)
        .bind(&run.status)
        .bind(&run.stage_outputs)
        .bind(run.current_stage)
        .bind(run.completed_at)
        .bind(run.total_input_tokens)
        .bind(run.total_output_tokens)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get_pipeline_run(&self, run_id: Uuid) -> Result<Option<PipelineRunRow>> {
        let row = sqlx::query_as::<_, PipelineRunRow>("SELECT * FROM pipeline_runs WHERE id = $1")
            .bind(run_id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row)
    }

    async fn list_pipeline_runs(&self, pipeline_id: Uuid) -> Result<Vec<PipelineRunRow>> {
        let rows = sqlx::query_as::<_, PipelineRunRow>("SELECT * FROM pipeline_runs WHERE pipeline_id = $1 ORDER BY started_at DESC")
            .bind(pipeline_id)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows)
    }

    async fn create_stage_execution(&self, exec: &StageExecutionRow) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO stage_executions (id, run_id, stage_number, stage_name, agent_id, status, rendered_prompt, output, structured_output, user_input, input_tokens, output_tokens, started_at, completed_at, duration_ms, stage_member_id, pipeline_id)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
            "#,
        )
        .bind(exec.id)
        .bind(exec.run_id)
        .bind(exec.stage_number)
        .bind(&exec.stage_name)
        .bind(exec.agent_id)
        .bind(&exec.status)
        .bind(exec.rendered_prompt.as_deref())
        .bind(exec.output.as_deref())
        .bind(&exec.structured_output)
        .bind(exec.user_input.as_deref())
        .bind(exec.input_tokens)
        .bind(exec.output_tokens)
        .bind(exec.started_at)
        .bind(exec.completed_at)
        .bind(exec.duration_ms)
        .bind(exec.stage_member_id)
        .bind(exec.pipeline_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn update_stage_execution(&self, exec: &StageExecutionRow) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE stage_executions
            SET status = $2, output = $3, structured_output = $4, user_input = $5, input_tokens = $6, output_tokens = $7, completed_at = $8, duration_ms = $9
            WHERE id = $1
            "#,
        )
        .bind(exec.id)
        .bind(&exec.status)
        .bind(exec.output.as_deref())
        .bind(&exec.structured_output)
        .bind(exec.user_input.as_deref())
        .bind(exec.input_tokens)
        .bind(exec.output_tokens)
        .bind(exec.completed_at)
        .bind(exec.duration_ms)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn list_stage_executions(&self, run_id: Uuid) -> Result<Vec<StageExecutionRow>> {
        let rows = sqlx::query_as::<_, StageExecutionRow>("SELECT * FROM stage_executions WHERE run_id = $1 ORDER BY stage_number")
            .bind(run_id)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows)
    }

    // --- Agent modes ---

    async fn get_agent_modes(&self, agent_id: Uuid) -> Result<Vec<AgentModeRow>> {
        crate::db::list_agent_modes(&self.pool, agent_id).await
    }

    async fn create_agent_mode(&self, mode: &AgentModeRow) -> Result<()> {
        crate::db::create_agent_mode(&self.pool, mode).await
    }

    async fn delete_agent_mode(&self, mode_id: Uuid) -> Result<()> {
        crate::db::delete_agent_mode(&self.pool, mode_id).await
    }
}

// ============================================================================
// User Repository
// ============================================================================

#[derive(sqlx::FromRow)]
struct PgAgentRow {
    id: Uuid,
    tier: Option<String>,
    name: String,
    system_prompt: String,
    persona_style: Option<String>,
    model_provider: String,
    model_id: String,
    model_max_tokens: i32,
    model_temperature: f32,
    status: Option<String>,
    router_mode: Option<bool>,
    version: i32,
}

fn agent_row_from_pg(r: PgAgentRow) -> AgentRow {
    AgentRow {
        id: r.id,
        tier: r.tier,
        name: r.name,
        system_prompt: r.system_prompt,
        persona_style: r.persona_style,
        model_provider: r.model_provider,
        model_id: r.model_id,
        model_max_tokens: r.model_max_tokens,
        model_temperature: r.model_temperature,
        status: r.status,
        router_mode: r.router_mode,
        version: r.version,
    }
}

#[derive(sqlx::FromRow)]
struct PgToolRow {
    id: Uuid,
    user_id: Uuid,
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
        user_id: r.user_id,
        name: r.name,
        display_name: r.display_name,
        description: r.description,
        parameters: r.parameters,
        created_at: r.created_at,
        version: r.version,
    }
}

#[derive(sqlx::FromRow)]
struct UserRow {
    id: Uuid,
    email: String,
    password_hash: Option<String>,
    github_id: Option<i64>,
    github_login: Option<String>,
    github_token_encrypted: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<UserRow> for User {
    fn from(row: UserRow) -> Self {
        User {
            id: UserId(row.id),
            email: row.email,
            password_hash: row.password_hash,
            github_id: row.github_id,
            github_login: row.github_login,
            github_token_encrypted: row.github_token_encrypted,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

#[async_trait]
impl UserRepo for PgRepo {
    async fn create_user(&self, email: &str, password_hash: &str) -> Result<User> {
        let row: UserRow = sqlx::query_as(
            r#"
            INSERT INTO users (id, email, password_hash, created_at, updated_at)
            VALUES (gen_random_uuid(), $1, $2, NOW(), NOW())
            RETURNING id, email, password_hash, github_id, github_login, github_token_encrypted, created_at, updated_at
            "#,
        )
        .bind(email)
        .bind(password_hash)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.into())
    }

    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>> {
        let row: Option<UserRow> = sqlx::query_as("SELECT id, email, password_hash, github_id, github_login, github_token_encrypted, created_at, updated_at FROM users WHERE email = $1")
            .bind(email)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.map(|r| r.into()))
    }

    async fn get_user_by_id(&self, id: UserId) -> Result<Option<User>> {
        let row: Option<UserRow> = sqlx::query_as("SELECT id, email, password_hash, github_id, github_login, github_token_encrypted, created_at, updated_at FROM users WHERE id = $1")
            .bind(id.0)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.map(|r| r.into()))
    }

    async fn get_user_by_github_id(&self, github_id: i64) -> Result<Option<User>> {
        let row: Option<UserRow> = sqlx::query_as("SELECT id, email, password_hash, github_id, github_login, github_token_encrypted, created_at, updated_at FROM users WHERE github_id = $1")
            .bind(github_id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.map(|r| r.into()))
    }

    async fn link_github(&self, user_id: UserId, github_id: i64, github_login: &str, token_encrypted: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE users
            SET github_id = $1, github_login = $2, github_token_encrypted = $3, updated_at = NOW()
            WHERE id = $4
            "#,
        )
        .bind(github_id)
        .bind(github_login)
        .bind(token_encrypted)
        .bind(user_id.0)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn create_github_user(&self, email: &str, github_id: i64, github_login: &str, token_encrypted: &str) -> Result<User> {
        let row: UserRow = sqlx::query_as(
            r#"
            INSERT INTO users (id, email, github_id, github_login, github_token_encrypted, created_at, updated_at)
            VALUES (gen_random_uuid(), $1, $2, $3, $4, NOW(), NOW())
            RETURNING id, email, password_hash, github_id, github_login, github_token_encrypted, created_at, updated_at
            "#,
        )
        .bind(email)
        .bind(github_id)
        .bind(github_login)
        .bind(token_encrypted)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.into())
    }
}

// ============================================================================
// Document Repository
// ============================================================================

#[async_trait]
impl DocumentRepo for PgRepo {
    async fn create_document(&self, user_id: Uuid, session_id: Option<Uuid>, title: String, content: String, doc_type: String, ref_tag: String, tags: Vec<String>) -> Result<DocumentRow> {
        let id = Uuid::new_v4();
        let row: DocumentRow = sqlx::query_as(
            r#"
            INSERT INTO documents (id, user_id, session_id, title, content, doc_type, ref_tag, tags)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id, user_id, session_id, title, content, summary, doc_type, ref_tag, tags, created_at, updated_at
            "#,
        )
        .bind(id)
        .bind(user_id)
        .bind(session_id)
        .bind(&title)
        .bind(&content)
        .bind(&doc_type)
        .bind(&ref_tag)
        .bind(&tags)
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }

    async fn update_document(&self, doc_id: Uuid, content: Option<String>, title: Option<String>, tags: Option<Vec<String>>) -> Result<DocumentRow> {
        let row: DocumentRow = sqlx::query_as(
            r#"
            UPDATE documents
            SET
                content = COALESCE($1, content),
                title = COALESCE($2, title),
                tags = COALESCE($3, tags),
                updated_at = NOW()
            WHERE id = $4
            RETURNING id, user_id, session_id, title, content, summary, doc_type, ref_tag, tags, created_at, updated_at
            "#,
        )
        .bind(content)
        .bind(title)
        .bind(tags)
        .bind(doc_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }

    async fn update_document_summary(&self, doc_id: Uuid, summary: String) -> Result<()> {
        sqlx::query("UPDATE documents SET summary = $1, updated_at = NOW() WHERE id = $2")
            .bind(&summary)
            .bind(doc_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn get_document(&self, doc_id: Uuid) -> Result<Option<DocumentRow>> {
        let row: Option<DocumentRow> = sqlx::query_as("SELECT id, user_id, session_id, title, content, summary, doc_type, ref_tag, tags, created_at, updated_at FROM documents WHERE id = $1")
            .bind(doc_id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row)
    }

    async fn get_document_by_ref_tag(&self, ref_tag: &str) -> Result<Option<DocumentRow>> {
        let row: Option<DocumentRow> = sqlx::query_as("SELECT id, user_id, session_id, title, content, summary, doc_type, ref_tag, tags, created_at, updated_at FROM documents WHERE ref_tag = $1")
            .bind(ref_tag)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row)
    }

    async fn list_documents(&self, user_id: Uuid) -> Result<Vec<DocumentRow>> {
        let rows: Vec<DocumentRow> =
            sqlx::query_as("SELECT id, user_id, session_id, title, content, summary, doc_type, ref_tag, tags, created_at, updated_at FROM documents WHERE user_id = $1 ORDER BY updated_at DESC")
                .bind(user_id)
                .fetch_all(&self.pool)
                .await?;
        Ok(rows)
    }

    async fn list_session_documents(&self, session_id: Uuid) -> Result<Vec<DocumentRow>> {
        let rows: Vec<DocumentRow> =
            sqlx::query_as("SELECT id, user_id, session_id, title, content, summary, doc_type, ref_tag, tags, created_at, updated_at FROM documents WHERE session_id = $1 ORDER BY updated_at DESC")
                .bind(session_id)
                .fetch_all(&self.pool)
                .await?;
        Ok(rows)
    }

    async fn search_documents(&self, user_id: Uuid, query: &str) -> Result<Vec<DocumentSearchResult>> {
        let rows: Vec<DocumentSearchResult> = sqlx::query_as(
            r#"
            SELECT id, title, summary, ref_tag,
                   ts_headline('english', content, plainto_tsquery('english', $2),
                       'StartSel=**, StopSel=**, MaxWords=35, MinWords=15') AS snippet
            FROM documents
            WHERE user_id = $1
              AND to_tsvector('english', title || ' ' || content) @@ plainto_tsquery('english', $2)
            ORDER BY ts_rank(to_tsvector('english', title || ' ' || content), plainto_tsquery('english', $2)) DESC
            LIMIT 50
            "#,
        )
        .bind(user_id)
        .bind(query)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn delete_document(&self, doc_id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM documents WHERE id = $1").bind(doc_id).execute(&self.pool).await?;
        Ok(())
    }
}

#[async_trait]
impl OutputSchemaRepo for PgRepo {
    async fn create_output_schema(&self, user_id: Uuid, name: String, schema: serde_json::Value) -> Result<OutputSchemaRow> {
        let row: OutputSchemaRow = sqlx::query_as(
            r#"
            INSERT INTO output_schemas (user_id, name, schema)
            VALUES ($1, $2, $3)
            RETURNING id, user_id, name, schema, created_at, version
            "#,
        )
        .bind(user_id)
        .bind(&name)
        .bind(&schema)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn get_output_schema(&self, id: Uuid) -> Result<Option<OutputSchemaRow>> {
        let row: Option<OutputSchemaRow> = sqlx::query_as("SELECT id, user_id, name, schema, created_at, version FROM output_schemas WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row)
    }

    async fn list_output_schemas(&self, user_id: Uuid) -> Result<Vec<OutputSchemaRow>> {
        let rows: Vec<OutputSchemaRow> = sqlx::query_as("SELECT id, user_id, name, schema, created_at, version FROM output_schemas WHERE user_id = $1 ORDER BY created_at DESC")
            .bind(user_id)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows)
    }

    async fn update_output_schema(&self, id: Uuid, name: Option<String>, schema: Option<serde_json::Value>) -> Result<OutputSchemaRow> {
        let row: OutputSchemaRow = sqlx::query_as(
            r#"
            UPDATE output_schemas
            SET name = COALESCE($1, name),
                schema = COALESCE($2, schema),
                version = version + 1
            WHERE id = $3
            RETURNING id, user_id, name, schema, created_at, version
            "#,
        )
        .bind(name)
        .bind(schema)
        .bind(id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn delete_output_schema(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM output_schemas WHERE id = $1").bind(id).execute(&self.pool).await?;
        Ok(())
    }
}

#[async_trait]
impl PromptTemplateRepo for PgRepo {
    async fn create_prompt_template(&self, user_id: Uuid, name: String, content: String) -> Result<PromptTemplateRow> {
        let row: PromptTemplateRow = sqlx::query_as(
            r#"
            INSERT INTO prompt_templates (user_id, name, content)
            VALUES ($1, $2, $3)
            RETURNING id, user_id, name, content, created_at, version
            "#,
        )
        .bind(user_id)
        .bind(&name)
        .bind(&content)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn get_prompt_template(&self, id: Uuid) -> Result<Option<PromptTemplateRow>> {
        let row: Option<PromptTemplateRow> = sqlx::query_as("SELECT id, user_id, name, content, created_at, version FROM prompt_templates WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row)
    }

    async fn list_prompt_templates(&self, user_id: Uuid) -> Result<Vec<PromptTemplateRow>> {
        let rows: Vec<PromptTemplateRow> = sqlx::query_as("SELECT id, user_id, name, content, created_at, version FROM prompt_templates WHERE user_id = $1 ORDER BY created_at DESC")
            .bind(user_id)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows)
    }

    async fn update_prompt_template(&self, id: Uuid, name: Option<String>, content: Option<String>) -> Result<PromptTemplateRow> {
        let row: PromptTemplateRow = sqlx::query_as(
            r#"
            UPDATE prompt_templates
            SET name = COALESCE($1, name),
                content = COALESCE($2, content),
                version = version + 1
            WHERE id = $3
            RETURNING id, user_id, name, content, created_at, version
            "#,
        )
        .bind(name)
        .bind(content)
        .bind(id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn delete_prompt_template(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM prompt_templates WHERE id = $1").bind(id).execute(&self.pool).await?;
        Ok(())
    }
}

#[async_trait]
impl WorkflowRepo for PgRepo {
    // --- Workflows ---

    async fn create_workflow(&self, user_id: Uuid, name: String, description: String) -> Result<WorkflowRow> {
        let row: WorkflowRow = sqlx::query_as("INSERT INTO workflows (user_id, name, description) VALUES ($1, $2, $3) RETURNING id, user_id, name, description, created_at, version")
            .bind(user_id)
            .bind(&name)
            .bind(&description)
            .fetch_one(&self.pool)
            .await?;
        Ok(row)
    }

    async fn get_workflow(&self, id: Uuid) -> Result<Option<WorkflowRow>> {
        let row: Option<WorkflowRow> = sqlx::query_as("SELECT id, user_id, name, description, created_at, version FROM workflows WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row)
    }

    async fn list_workflows(&self, user_id: Uuid) -> Result<Vec<WorkflowRow>> {
        let rows: Vec<WorkflowRow> = sqlx::query_as("SELECT id, user_id, name, description, created_at, version FROM workflows WHERE user_id = $1 ORDER BY created_at DESC")
            .bind(user_id)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows)
    }

    async fn update_workflow(&self, id: Uuid, name: Option<String>, description: Option<String>) -> Result<WorkflowRow> {
        let row: WorkflowRow =
            sqlx::query_as("UPDATE workflows SET name = COALESCE($1, name), description = COALESCE($2, description), version = version + 1 WHERE id = $3 RETURNING id, user_id, name, description, created_at, version")
                .bind(name)
                .bind(description)
                .bind(id)
                .fetch_one(&self.pool)
                .await?;
        Ok(row)
    }

    async fn delete_workflow(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM workflows WHERE id = $1").bind(id).execute(&self.pool).await?;
        Ok(())
    }

    // --- Steps ---

    async fn create_step(&self, step: WorkflowStepRow) -> Result<WorkflowStepRow> {
        let row: WorkflowStepRow = sqlx::query_as(
            r#"
            INSERT INTO workflow_steps (id, workflow_id, agent_id, execution_mode, for_each_ref, prompt_template_id, prompt_template, output_schema_id, output_variable_name, interactive_agent_id, for_each_label_field, display_order)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            RETURNING *
            "#,
        )
        .bind(step.id)
        .bind(step.workflow_id)
        .bind(step.agent_id)
        .bind(&step.execution_mode)
        .bind(&step.for_each_ref)
        .bind(step.prompt_template_id)
        .bind(&step.prompt_template)
        .bind(step.output_schema_id)
        .bind(&step.output_variable_name)
        .bind(step.interactive_agent_id)
        .bind(&step.for_each_label_field)
        .bind(step.display_order)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn get_step(&self, id: Uuid) -> Result<Option<WorkflowStepRow>> {
        let row: Option<WorkflowStepRow> = sqlx::query_as("SELECT * FROM workflow_steps WHERE id = $1").bind(id).fetch_optional(&self.pool).await?;
        Ok(row)
    }

    async fn list_steps(&self, workflow_id: Uuid) -> Result<Vec<WorkflowStepRow>> {
        let rows: Vec<WorkflowStepRow> = sqlx::query_as("SELECT * FROM workflow_steps WHERE workflow_id = $1 ORDER BY display_order")
            .bind(workflow_id)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows)
    }

    async fn update_step(&self, step: WorkflowStepRow) -> Result<WorkflowStepRow> {
        let row: WorkflowStepRow = sqlx::query_as(
            r#"
            UPDATE workflow_steps
            SET agent_id = $1, execution_mode = $2, for_each_ref = $3, prompt_template_id = $4, prompt_template = $5,
                output_schema_id = $6, output_variable_name = $7, interactive_agent_id = $8, for_each_label_field = $9, display_order = $10,
                version = version + 1
            WHERE id = $11
            RETURNING *
            "#,
        )
        .bind(step.agent_id)
        .bind(&step.execution_mode)
        .bind(&step.for_each_ref)
        .bind(step.prompt_template_id)
        .bind(&step.prompt_template)
        .bind(step.output_schema_id)
        .bind(&step.output_variable_name)
        .bind(step.interactive_agent_id)
        .bind(&step.for_each_label_field)
        .bind(step.display_order)
        .bind(step.id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn delete_step(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM workflow_steps WHERE id = $1").bind(id).execute(&self.pool).await?;
        Ok(())
    }

    // --- Edges ---

    async fn set_edges(&self, workflow_id: Uuid, edges: Vec<WorkflowStepEdgeRow>) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM workflow_step_edges WHERE from_step_id IN (SELECT id FROM workflow_steps WHERE workflow_id = $1)")
            .bind(workflow_id)
            .execute(&mut *tx)
            .await?;
        for edge in &edges {
            sqlx::query("INSERT INTO workflow_step_edges (from_step_id, to_step_id) VALUES ($1, $2)")
                .bind(edge.from_step_id)
                .bind(edge.to_step_id)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    async fn list_edges(&self, workflow_id: Uuid) -> Result<Vec<WorkflowStepEdgeRow>> {
        let rows: Vec<WorkflowStepEdgeRow> = sqlx::query_as("SELECT e.from_step_id, e.to_step_id FROM workflow_step_edges e JOIN workflow_steps s ON e.from_step_id = s.id WHERE s.workflow_id = $1")
            .bind(workflow_id)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows)
    }

    async fn add_edge(&self, from_step_id: Uuid, to_step_id: Uuid) -> Result<()> {
        sqlx::query("INSERT INTO workflow_step_edges (from_step_id, to_step_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
            .bind(from_step_id)
            .bind(to_step_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn remove_edge(&self, from_step_id: Uuid, to_step_id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM workflow_step_edges WHERE from_step_id = $1 AND to_step_id = $2")
            .bind(from_step_id)
            .bind(to_step_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // --- Step documents ---

    async fn list_step_documents(&self, step_id: Uuid) -> Result<Vec<StepDocumentRow>> {
        let rows: Vec<StepDocumentRow> = sqlx::query_as("SELECT step_id, document_id FROM step_documents WHERE step_id = $1")
            .bind(step_id)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows)
    }

    async fn add_step_document(&self, step_id: Uuid, document_id: Uuid) -> Result<()> {
        sqlx::query("INSERT INTO step_documents (step_id, document_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
            .bind(step_id)
            .bind(document_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn remove_step_document(&self, step_id: Uuid, document_id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM step_documents WHERE step_id = $1 AND document_id = $2")
            .bind(step_id)
            .bind(document_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

#[async_trait]
impl PipelineStageMemberRepo for PgRepo {
    async fn list_stage_members(&self, pipeline_id: Uuid, stage_number: i32) -> Result<Vec<PipelineStageMemberRow>> {
        let rows: Vec<PipelineStageMemberRow> =
            sqlx::query_as("SELECT id, pipeline_id, stage_number, workflow_id, display_order FROM pipeline_stage_members WHERE pipeline_id = $1 AND stage_number = $2 ORDER BY display_order")
                .bind(pipeline_id)
                .bind(stage_number)
                .fetch_all(&self.pool)
                .await?;
        Ok(rows)
    }

    async fn add_stage_member(&self, pipeline_id: Uuid, stage_number: i32, workflow_id: Uuid, display_order: i32) -> Result<PipelineStageMemberRow> {
        let row: PipelineStageMemberRow = sqlx::query_as(
            "INSERT INTO pipeline_stage_members (pipeline_id, stage_number, workflow_id, display_order) VALUES ($1, $2, $3, $4) RETURNING id, pipeline_id, stage_number, workflow_id, display_order",
        )
        .bind(pipeline_id)
        .bind(stage_number)
        .bind(workflow_id)
        .bind(display_order)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn remove_stage_member(&self, member_id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM pipeline_stage_members WHERE id = $1").bind(member_id).execute(&self.pool).await?;
        Ok(())
    }

    async fn update_stage_member(&self, member_id: Uuid, display_order: i32) -> Result<PipelineStageMemberRow> {
        let row: PipelineStageMemberRow = sqlx::query_as("UPDATE pipeline_stage_members SET display_order = $1 WHERE id = $2 RETURNING id, pipeline_id, stage_number, workflow_id, display_order")
            .bind(display_order)
            .bind(member_id)
            .fetch_one(&self.pool)
            .await?;
        Ok(row)
    }
}

#[async_trait]
impl AgentExecutionRepo for PgRepo {
    async fn create_agent_execution(
        &self,
        stage_execution_id: Uuid,
        agent_id: Uuid,
        workflow_step_id: Option<Uuid>,
        is_interactive: bool,
        parent_agent_execution_id: Option<Uuid>,
        system_prompt_rendered: &str,
        input: &str,
        selected_mode_id: Option<Uuid>,
        room_session_id: Option<Uuid>,
        speaker_order: Option<i32>,
    ) -> Result<AgentExecutionRow> {
        let row = sqlx::query_as::<_, AgentExecutionRow>(
            "INSERT INTO agent_executions (stage_execution_id, agent_id, workflow_step_id, is_interactive, parent_agent_execution_id, system_prompt_rendered, input, selected_mode_id, room_session_id, speaker_order) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) RETURNING *",
        )
        .bind(stage_execution_id)
        .bind(agent_id)
        .bind(workflow_step_id)
        .bind(is_interactive)
        .bind(parent_agent_execution_id)
        .bind(system_prompt_rendered)
        .bind(input)
        .bind(selected_mode_id)
        .bind(room_session_id)
        .bind(speaker_order)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn get_agent_execution(&self, id: Uuid) -> Result<Option<AgentExecutionRow>> {
        let row = sqlx::query_as::<_, AgentExecutionRow>("SELECT * FROM agent_executions WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row)
    }

    async fn list_agent_executions_by_stage(&self, stage_execution_id: Uuid) -> Result<Vec<AgentExecutionRow>> {
        let rows = sqlx::query_as::<_, AgentExecutionRow>("SELECT * FROM agent_executions WHERE stage_execution_id = $1 ORDER BY started_at ASC")
            .bind(stage_execution_id)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows)
    }

    async fn update_agent_execution_status(
        &self,
        id: Uuid,
        status: &str,
        output: Option<String>,
        structured_output: Option<serde_json::Value>,
    ) -> Result<AgentExecutionRow> {
        let row = sqlx::query_as::<_, AgentExecutionRow>(
            "UPDATE agent_executions SET status = $2, output = COALESCE($3, output), structured_output = COALESCE($4, structured_output), completed_at = CASE WHEN $2 IN ('completed', 'failed', 'cancelled') THEN NOW() ELSE completed_at END WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .bind(status)
        .bind(output)
        .bind(structured_output)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn create_execution_message(&self, agent_execution_id: Uuid, role: &str, content: &str, tool_call_id: Option<String>, input_tokens: i64, output_tokens: i64) -> Result<ExecutionMessageRow> {
        let row = sqlx::query_as::<_, ExecutionMessageRow>(
            "INSERT INTO execution_messages (agent_execution_id, role, content, tool_call_id, input_tokens, output_tokens) VALUES ($1, $2, $3, $4, $5, $6) RETURNING *",
        )
        .bind(agent_execution_id)
        .bind(role)
        .bind(content)
        .bind(tool_call_id)
        .bind(input_tokens)
        .bind(output_tokens)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn list_execution_messages(&self, agent_execution_id: Uuid) -> Result<Vec<ExecutionMessageRow>> {
        let rows = sqlx::query_as::<_, ExecutionMessageRow>("SELECT * FROM execution_messages WHERE agent_execution_id = $1 ORDER BY created_at ASC")
            .bind(agent_execution_id)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows)
    }
}

#[async_trait]
impl TokenLedgerRepo for PgRepo {
    async fn insert_ledger_entry(&self, user_id: Uuid, agent_execution_id: Option<Uuid>, model_id: &str, input_tokens: i64, output_tokens: i64, cost_usd: f32) -> Result<TokenLedgerRow> {
        let row =
            sqlx::query_as::<_, TokenLedgerRow>("INSERT INTO token_ledger (user_id, agent_execution_id, model_id, input_tokens, output_tokens, cost_usd) VALUES ($1, $2, $3, $4, $5, $6) RETURNING *")
                .bind(user_id)
                .bind(agent_execution_id)
                .bind(model_id)
                .bind(input_tokens)
                .bind(output_tokens)
                .bind(cost_usd)
                .fetch_one(&self.pool)
                .await?;
        Ok(row)
    }

    async fn get_user_spend(&self, user_id: Uuid, since: Option<DateTime<Utc>>) -> Result<f64> {
        let row: (Option<f64>,) = match since {
            Some(t) => {
                sqlx::query_as("SELECT CAST(COALESCE(SUM(cost_usd), 0) AS FLOAT8) FROM token_ledger WHERE user_id = $1 AND created_at >= $2")
                    .bind(user_id)
                    .bind(t)
                    .fetch_one(&self.pool)
                    .await?
            }
            None => {
                sqlx::query_as("SELECT CAST(COALESCE(SUM(cost_usd), 0) AS FLOAT8) FROM token_ledger WHERE user_id = $1")
                    .bind(user_id)
                    .fetch_one(&self.pool)
                    .await?
            }
        };
        Ok(row.0.unwrap_or(0.0))
    }

    async fn get_run_spend(&self, run_id: Uuid) -> Result<f64> {
        let row: (Option<f64>,) = sqlx::query_as(
            "SELECT CAST(COALESCE(SUM(tl.cost_usd), 0) AS FLOAT8) FROM token_ledger tl JOIN agent_executions ae ON tl.agent_execution_id = ae.id JOIN stage_executions se ON ae.stage_execution_id = se.id WHERE se.run_id = $1",
        )
        .bind(run_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0.unwrap_or(0.0))
    }

    async fn get_model_breakdown(&self, user_id: Uuid, since: Option<DateTime<Utc>>) -> Result<Vec<ModelSpendRow>> {
        let rows = match since {
            Some(t) => {
                sqlx::query_as::<_, ModelSpendRow>("SELECT model_id, SUM(input_tokens) AS total_input_tokens, SUM(output_tokens) AS total_output_tokens, CAST(SUM(cost_usd) AS FLOAT8) AS total_cost_usd, COUNT(*) AS call_count FROM token_ledger WHERE user_id = $1 AND created_at >= $2 GROUP BY model_id ORDER BY total_cost_usd DESC")
                    .bind(user_id)
                    .bind(t)
                    .fetch_all(&self.pool)
                    .await?
            }
            None => {
                sqlx::query_as::<_, ModelSpendRow>("SELECT model_id, SUM(input_tokens) AS total_input_tokens, SUM(output_tokens) AS total_output_tokens, CAST(SUM(cost_usd) AS FLOAT8) AS total_cost_usd, COUNT(*) AS call_count FROM token_ledger WHERE user_id = $1 GROUP BY model_id ORDER BY total_cost_usd DESC")
                    .bind(user_id)
                    .fetch_all(&self.pool)
                    .await?
            }
        };
        Ok(rows)
    }
}

#[async_trait]
impl ResultRepo for PgRepo {
    async fn save_result(&self, user_id: Uuid, agent_execution_id: Uuid, output_schema_id: Option<Uuid>, name: &str, data: serde_json::Value) -> Result<ResultRow> {
        let row = sqlx::query_as::<_, ResultRow>("INSERT INTO results (user_id, agent_execution_id, output_schema_id, name, data) VALUES ($1, $2, $3, $4, $5) RETURNING *")
            .bind(user_id)
            .bind(agent_execution_id)
            .bind(output_schema_id)
            .bind(name)
            .bind(data)
            .fetch_one(&self.pool)
            .await?;
        Ok(row)
    }

    async fn get_result(&self, id: Uuid) -> Result<Option<ResultRow>> {
        let row = sqlx::query_as::<_, ResultRow>("SELECT * FROM results WHERE id = $1").bind(id).fetch_optional(&self.pool).await?;
        Ok(row)
    }

    async fn list_results(&self, user_id: Uuid) -> Result<Vec<ResultRow>> {
        let rows = sqlx::query_as::<_, ResultRow>("SELECT * FROM results WHERE user_id = $1 ORDER BY created_at DESC")
            .bind(user_id)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows)
    }

    async fn list_results_by_schema(&self, user_id: Uuid, output_schema_id: Uuid) -> Result<Vec<ResultRow>> {
        let rows = sqlx::query_as::<_, ResultRow>("SELECT * FROM results WHERE user_id = $1 AND output_schema_id = $2 ORDER BY created_at DESC")
            .bind(user_id)
            .bind(output_schema_id)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows)
    }

    async fn delete_result(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM results WHERE id = $1").bind(id).execute(&self.pool).await?;
        Ok(())
    }
}

// ============================================================================
// ToolRouterRepo
// ============================================================================

#[async_trait]
impl ToolRouterRepo for PgRepo {
    async fn list_tool_routers(&self, user_id: Uuid) -> Result<Vec<ToolRouterRow>> {
        let rows: Vec<ToolRouterRow> = sqlx::query_as(
            "SELECT id, user_id, name, description, system_prompt, model_id, is_active, created_at, updated_at FROM tool_routers WHERE user_id = $1 ORDER BY created_at DESC",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn get_tool_router(&self, id: Uuid) -> Result<Option<ToolRouterRow>> {
        let row: Option<ToolRouterRow> = sqlx::query_as(
            "SELECT id, user_id, name, description, system_prompt, model_id, is_active, created_at, updated_at FROM tool_routers WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    async fn create_tool_router(&self, user_id: Uuid, name: &str, description: Option<String>, system_prompt: &str, model_id: &str) -> Result<ToolRouterRow> {
        let row: ToolRouterRow = sqlx::query_as(
            "INSERT INTO tool_routers (user_id, name, description, system_prompt, model_id) VALUES ($1, $2, $3, $4, $5) RETURNING id, user_id, name, description, system_prompt, model_id, is_active, created_at, updated_at",
        )
        .bind(user_id)
        .bind(name)
        .bind(description)
        .bind(system_prompt)
        .bind(model_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn update_tool_router(&self, id: Uuid, name: Option<String>, description: Option<String>, system_prompt: Option<String>, model_id: Option<String>, is_active: Option<bool>) -> Result<ToolRouterRow> {
        let row: ToolRouterRow = sqlx::query_as(
            r#"UPDATE tool_routers SET
                name = COALESCE($2, name),
                description = COALESCE($3, description),
                system_prompt = COALESCE($4, system_prompt),
                model_id = COALESCE($5, model_id),
                is_active = COALESCE($6, is_active),
                updated_at = NOW()
            WHERE id = $1
            RETURNING id, user_id, name, description, system_prompt, model_id, is_active, created_at, updated_at"#,
        )
        .bind(id)
        .bind(name)
        .bind(description)
        .bind(system_prompt)
        .bind(model_id)
        .bind(is_active)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn delete_tool_router(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM tool_routers WHERE id = $1").bind(id).execute(&self.pool).await?;
        Ok(())
    }

    async fn get_router_tools(&self, router_id: Uuid) -> Result<Vec<ToolRow>> {
        let rows: Vec<ToolRow> = sqlx::query_as(
            "SELECT t.id, t.user_id, t.name, t.display_name, t.description, t.parameters, t.created_at FROM tools t INNER JOIN tool_router_tools trt ON t.id = trt.tool_id WHERE trt.router_id = $1 ORDER BY t.name",
        )
        .bind(router_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn set_router_tools(&self, router_id: Uuid, tool_ids: &[Uuid]) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        sqlx::query("DELETE FROM tool_router_tools WHERE router_id = $1").bind(router_id).execute(&mut *tx).await?;

        for tool_id in tool_ids {
            sqlx::query("INSERT INTO tool_router_tools (router_id, tool_id) VALUES ($1, $2)")
                .bind(router_id)
                .bind(tool_id)
                .execute(&mut *tx)
                .await?;
        }

        tx.commit().await?;
        Ok(())
    }
}

// ============================================================================
// ContextStoreRepo
// ============================================================================

#[async_trait]
impl ContextStoreRepo for PgRepo {
    async fn add_context(&self, session_id: Uuid, source: &str, priority: f32, content: &str, metadata: Option<serde_json::Value>, expires_at: Option<DateTime<Utc>>) -> Result<ContextStoreRow> {
        let row: ContextStoreRow = sqlx::query_as(
            "INSERT INTO context_store (session_id, source, priority, content, metadata, expires_at) VALUES ($1, $2, $3, $4, $5, $6) RETURNING id, session_id, source, priority, content, metadata, status, created_at, expires_at",
        )
        .bind(session_id)
        .bind(source)
        .bind(priority)
        .bind(content)
        .bind(metadata)
        .bind(expires_at)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn get_active_context(&self, session_id: Uuid, limit: u32) -> Result<Vec<ContextStoreRow>> {
        let rows: Vec<ContextStoreRow> = sqlx::query_as(
            "SELECT id, session_id, source, priority, content, metadata, status, created_at, expires_at FROM context_store WHERE session_id = $1 AND status = 'active' AND (expires_at IS NULL OR expires_at > NOW()) ORDER BY priority DESC LIMIT $2",
        )
        .bind(session_id)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn update_context_status(&self, id: Uuid, status: &str) -> Result<()> {
        sqlx::query("UPDATE context_store SET status = $2 WHERE id = $1")
            .bind(id)
            .bind(status)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn expire_stale_context(&self, session_id: Uuid) -> Result<u32> {
        let result = sqlx::query("UPDATE context_store SET status = 'expired' WHERE session_id = $1 AND status = 'active' AND expires_at IS NOT NULL AND expires_at <= NOW()")
            .bind(session_id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() as u32)
    }
}

// ============================================================================
// RouterRequestRepo
// ============================================================================

#[async_trait]
impl RouterRequestRepo for PgRepo {
    async fn create_router_request(&self, session_id: Uuid, agent_execution_id: Option<Uuid>, intent: &str, priority: &str, callback_hint: Option<String>) -> Result<RouterRequestRow> {
        let row: RouterRequestRow = sqlx::query_as(
            "INSERT INTO router_requests (session_id, agent_execution_id, intent, priority, callback_hint) VALUES ($1, $2, $3, $4, $5) RETURNING id, session_id, agent_execution_id, intent, priority, callback_hint, routed_tool, routed_args, is_async, passdown, chain, status, result, created_at, completed_at",
        )
        .bind(session_id)
        .bind(agent_execution_id)
        .bind(intent)
        .bind(priority)
        .bind(callback_hint)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn update_router_request(&self, id: Uuid, routed_tool: Option<String>, routed_args: Option<serde_json::Value>, is_async: bool, passdown: Option<String>, chain: Option<serde_json::Value>, status: &str, result: Option<String>) -> Result<RouterRequestRow> {
        let row: RouterRequestRow = sqlx::query_as(
            r#"UPDATE router_requests SET
                routed_tool = $2, routed_args = $3, is_async = $4, passdown = $5,
                chain = $6, status = $7, result = $8,
                completed_at = CASE WHEN $7 IN ('completed', 'failed', 'cancelled') THEN NOW() ELSE completed_at END
            WHERE id = $1
            RETURNING id, session_id, agent_execution_id, intent, priority, callback_hint, routed_tool, routed_args, is_async, passdown, chain, status, result, created_at, completed_at"#,
        )
        .bind(id)
        .bind(routed_tool)
        .bind(routed_args)
        .bind(is_async)
        .bind(passdown)
        .bind(chain)
        .bind(status)
        .bind(result)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn get_router_request(&self, id: Uuid) -> Result<Option<RouterRequestRow>> {
        let row: Option<RouterRequestRow> = sqlx::query_as(
            "SELECT id, session_id, agent_execution_id, intent, priority, callback_hint, routed_tool, routed_args, is_async, passdown, chain, status, result, created_at, completed_at FROM router_requests WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    async fn list_session_requests(&self, session_id: Uuid) -> Result<Vec<RouterRequestRow>> {
        let rows: Vec<RouterRequestRow> = sqlx::query_as(
            "SELECT id, session_id, agent_execution_id, intent, priority, callback_hint, routed_tool, routed_args, is_async, passdown, chain, status, result, created_at, completed_at FROM router_requests WHERE session_id = $1 ORDER BY created_at DESC",
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }
}

// ============================================================================
// Room Repository
// ============================================================================

#[async_trait]
impl RoomRepo for PgRepo {
    async fn create_room(
        &self,
        user_id: Uuid,
        pipeline_id: Uuid,
        name: &str,
        gatekeeper_enabled: bool,
        gatekeeper_model_id: &str,
        max_speakers_per_turn: i32,
        max_turns: i32,
        tools_enabled: bool,
    ) -> Result<RoomRow> {
        let row = sqlx::query_as::<_, RoomRow>(
            "INSERT INTO rooms (user_id, pipeline_id, name, gatekeeper_enabled, gatekeeper_model_id, max_speakers_per_turn, max_turns, tools_enabled) VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING *",
        )
        .bind(user_id)
        .bind(pipeline_id)
        .bind(name)
        .bind(gatekeeper_enabled)
        .bind(gatekeeper_model_id)
        .bind(max_speakers_per_turn)
        .bind(max_turns)
        .bind(tools_enabled)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn get_room(&self, id: Uuid) -> Result<Option<RoomRow>> {
        let row = sqlx::query_as::<_, RoomRow>("SELECT * FROM rooms WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row)
    }

    async fn list_rooms_for_pipeline(&self, pipeline_id: Uuid) -> Result<Vec<RoomRow>> {
        let rows = sqlx::query_as::<_, RoomRow>("SELECT * FROM rooms WHERE pipeline_id = $1 ORDER BY created_at ASC")
            .bind(pipeline_id)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows)
    }

    async fn update_room(
        &self,
        id: Uuid,
        name: Option<String>,
        gatekeeper_enabled: Option<bool>,
        gatekeeper_model_id: Option<String>,
        max_speakers_per_turn: Option<i32>,
        max_turns: Option<i32>,
        tools_enabled: Option<bool>,
    ) -> Result<RoomRow> {
        let row = sqlx::query_as::<_, RoomRow>(
            "UPDATE rooms SET \
                name = COALESCE($2, name), \
                gatekeeper_enabled = COALESCE($3, gatekeeper_enabled), \
                gatekeeper_model_id = COALESCE($4, gatekeeper_model_id), \
                max_speakers_per_turn = COALESCE($5, max_speakers_per_turn), \
                max_turns = COALESCE($6, max_turns), \
                tools_enabled = COALESCE($7, tools_enabled), \
                updated_at = NOW() \
            WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .bind(name)
        .bind(gatekeeper_enabled)
        .bind(gatekeeper_model_id)
        .bind(max_speakers_per_turn)
        .bind(max_turns)
        .bind(tools_enabled)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn delete_room(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM rooms WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // --- Room members ---

    async fn list_room_members(&self, room_id: Uuid) -> Result<Vec<RoomMemberRow>> {
        let rows = sqlx::query_as::<_, RoomMemberRow>(
            "SELECT room_id, agent_id, display_name, role_description, display_order FROM room_members WHERE room_id = $1 ORDER BY display_order ASC",
        )
        .bind(room_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn add_room_member(&self, room_id: Uuid, agent_id: Uuid, display_name: Option<String>, role_description: String, display_order: i32) -> Result<()> {
        sqlx::query(
            "INSERT INTO room_members (room_id, agent_id, display_name, role_description, display_order) VALUES ($1, $2, $3, $4, $5) ON CONFLICT DO NOTHING",
        )
        .bind(room_id)
        .bind(agent_id)
        .bind(display_name)
        .bind(role_description)
        .bind(display_order)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn remove_room_member(&self, room_id: Uuid, agent_id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM room_members WHERE room_id = $1 AND agent_id = $2")
            .bind(room_id)
            .bind(agent_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn set_room_members(&self, room_id: Uuid, members: &[RoomMemberInput]) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        sqlx::query("DELETE FROM room_members WHERE room_id = $1")
            .bind(room_id)
            .execute(&mut *tx)
            .await?;

        for member in members {
            sqlx::query(
                "INSERT INTO room_members (room_id, agent_id, display_name, role_description, display_order) VALUES ($1, $2, $3, $4, $5)",
            )
            .bind(room_id)
            .bind(member.agent_id)
            .bind(member.display_name.as_deref())
            .bind(&member.role_description)
            .bind(member.display_order)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    // --- Room sessions ---

    async fn create_room_session(&self, room_id: Uuid, run_id: Option<Uuid>) -> Result<RoomSessionRow> {
        let row = sqlx::query_as::<_, RoomSessionRow>(
            "INSERT INTO room_sessions (room_id, run_id) VALUES ($1, $2) RETURNING *",
        )
        .bind(room_id)
        .bind(run_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn get_room_session(&self, id: Uuid) -> Result<Option<RoomSessionRow>> {
        let row = sqlx::query_as::<_, RoomSessionRow>("SELECT * FROM room_sessions WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row)
    }

    async fn update_room_session_status(&self, id: Uuid, status: &str) -> Result<()> {
        let completed_at = if status == "completed" || status == "cancelled" {
            Some(Utc::now())
        } else {
            None
        };
        sqlx::query("UPDATE room_sessions SET status = $2, completed_at = COALESCE($3, completed_at) WHERE id = $1")
            .bind(id)
            .bind(status)
            .bind(completed_at)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn increment_room_session_turn(&self, id: Uuid) -> Result<i32> {
        let row: (i32,) = sqlx::query_as(
            "UPDATE room_sessions SET current_turn = current_turn + 1 WHERE id = $1 RETURNING current_turn",
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0)
    }

    async fn set_transcript_summary(&self, id: Uuid, summary: &str) -> Result<()> {
        sqlx::query("UPDATE room_sessions SET transcript_summary = $2 WHERE id = $1")
            .bind(id)
            .bind(summary)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // --- Room transcript ---

    async fn get_room_transcript(&self, room_session_id: Uuid) -> Result<Vec<RoomTranscriptEntry>> {
        let rows = sqlx::query_as::<_, RoomTranscriptEntry>(
            "SELECT \
                COALESCE(rm.display_name, a.name) AS agent_name, \
                COALESCE(rm.role_description, '') AS role_description, \
                em.content, \
                ae.speaker_order, \
                em.created_at \
            FROM execution_messages em \
            JOIN agent_executions ae ON em.agent_execution_id = ae.id \
            JOIN agents a ON ae.agent_id = a.id \
            LEFT JOIN room_members rm ON rm.agent_id = ae.agent_id \
                AND rm.room_id = (SELECT room_id FROM room_sessions WHERE id = $1) \
            WHERE ae.room_session_id = $1 \
                AND em.role IN ('user', 'assistant') \
            ORDER BY em.created_at ASC",
        )
        .bind(room_session_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::test_utils::TestDb;
    use crate::types::{AgentTier, Priority, Task, TaskId, TaskStatus};

    #[tokio::test]
    #[ignore = "requires running Postgres"]
    async fn merge_queue_insert_and_get() {
        let db = TestDb::new().await;
        let repo = PgRepo::new(db.pool.clone());

        let id = Uuid::new_v4();
        let owner = "testowner".to_string();
        let repo_name = "testrepo".to_string();
        let pr_number = 42;
        let position = 1;
        let now = Utc::now();

        // Insert entry
        repo.insert_queue_entry(id, owner.clone(), repo_name.clone(), pr_number, position, now).await.unwrap();

        // Get entries
        let entries = repo.get_queue_entries(owner.clone(), repo_name.clone()).await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].pr_number, pr_number);
        assert_eq!(entries[0].queue_position, position);

        db.cleanup().await;
    }

    #[tokio::test]
    #[ignore = "requires running Postgres"]
    async fn merge_queue_get_next_position() {
        let db = TestDb::new().await;
        let repo = PgRepo::new(db.pool.clone());

        let owner = "testowner".to_string();
        let repo_name = "testrepo".to_string();
        let now = Utc::now();

        // Get next position (should be 1 for empty queue)
        let pos1 = repo.get_next_position(owner.clone(), repo_name.clone()).await.unwrap();
        assert_eq!(pos1, 1);

        // Insert entry at position 1
        repo.insert_queue_entry(Uuid::new_v4(), owner.clone(), repo_name.clone(), 1, 1, now).await.unwrap();

        // Get next position (should be 2)
        let pos2 = repo.get_next_position(owner.clone(), repo_name.clone()).await.unwrap();
        assert_eq!(pos2, 2);

        db.cleanup().await;
    }

    #[tokio::test]
    #[ignore = "requires running Postgres"]
    async fn merge_queue_delete_entry() {
        let db = TestDb::new().await;
        let repo = PgRepo::new(db.pool.clone());

        let owner = "testowner".to_string();
        let repo_name = "testrepo".to_string();
        let pr_number = 42;
        let now = Utc::now();

        // Insert entry
        repo.insert_queue_entry(Uuid::new_v4(), owner.clone(), repo_name.clone(), pr_number, 1, now).await.unwrap();

        // Delete entry
        let deleted = repo.delete_queue_entry(owner.clone(), repo_name.clone(), pr_number).await.unwrap();
        assert!(deleted);

        // Verify deletion
        let entries = repo.get_queue_entries(owner.clone(), repo_name.clone()).await.unwrap();
        assert_eq!(entries.len(), 0);

        db.cleanup().await;
    }

    #[tokio::test]
    #[ignore = "requires running Postgres"]
    async fn merge_queue_update_status() {
        let db = TestDb::new().await;
        let repo = PgRepo::new(db.pool.clone());

        let owner = "testowner".to_string();
        let repo_name = "testrepo".to_string();
        let pr_number = 42;
        let now = Utc::now();

        // Insert entry
        repo.insert_queue_entry(Uuid::new_v4(), owner.clone(), repo_name.clone(), pr_number, 1, now).await.unwrap();

        // Update status
        let updated = repo
            .update_entry_status(owner.clone(), repo_name.clone(), pr_number, "in_progress".to_string(), None, now)
            .await
            .unwrap();
        assert!(updated);

        // Verify update
        let entries = repo.get_queue_entries(owner.clone(), repo_name.clone()).await.unwrap();
        assert_eq!(entries[0].status.to_string(), "in_progress");

        db.cleanup().await;
    }

    #[tokio::test]
    #[ignore = "requires running Postgres"]
    async fn merge_queue_reset_interrupted() {
        let db = TestDb::new().await;
        let repo = PgRepo::new(db.pool.clone());

        let owner = "testowner".to_string();
        let repo_name = "testrepo".to_string();
        let now = Utc::now();

        // Insert entries with in_progress status
        repo.insert_queue_entry(Uuid::new_v4(), owner.clone(), repo_name.clone(), 1, 1, now).await.unwrap();
        repo.update_entry_status(owner.clone(), repo_name.clone(), 1, "in_progress".to_string(), None, now).await.unwrap();

        // Reset interrupted
        let count = repo.reset_interrupted(owner.clone(), repo_name.clone(), now).await.unwrap();
        assert_eq!(count, 1);

        // Verify reset
        let entries = repo.get_queue_entries(owner.clone(), repo_name.clone()).await.unwrap();
        assert_eq!(entries[0].status.to_string(), "pending");

        db.cleanup().await;
    }

    #[tokio::test]
    #[ignore = "requires running Postgres"]
    async fn user_repo_create_and_get_by_email() {
        let db = TestDb::new().await;
        let repo = PgRepo::new(db.pool.clone());

        let email = "test@example.com";
        let password_hash = "hashed_password";

        // Create user
        let user = repo.create_user(email, password_hash).await.unwrap();
        assert_eq!(user.email, email);
        assert_eq!(user.password_hash, Some(password_hash.to_string()));

        // Get user by email
        let fetched = repo.get_user_by_email(email).await.unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().email, email);

        db.cleanup().await;
    }

    #[tokio::test]
    #[ignore = "requires running Postgres"]
    async fn user_repo_get_by_id() {
        let db = TestDb::new().await;
        let repo = PgRepo::new(db.pool.clone());

        let email = "test@example.com";
        let password_hash = "hashed_password";

        // Create user
        let user = repo.create_user(email, password_hash).await.unwrap();

        // Get user by ID
        let fetched = repo.get_user_by_id(user.id).await.unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().id, user.id);

        db.cleanup().await;
    }

    #[tokio::test]
    #[ignore = "requires running Postgres"]
    async fn user_repo_create_github_user() {
        let db = TestDb::new().await;
        let repo = PgRepo::new(db.pool.clone());

        let email = "github@example.com";
        let github_id = 123456;
        let github_login = "testuser";
        let token = "encrypted_token";

        // Create GitHub user
        let user = repo.create_github_user(email, github_id, github_login, token).await.unwrap();
        assert_eq!(user.email, email);
        assert_eq!(user.github_id, Some(github_id));
        assert_eq!(user.github_login, Some(github_login.to_string()));

        // Get by GitHub ID
        let fetched = repo.get_user_by_github_id(github_id).await.unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().github_id, Some(github_id));

        db.cleanup().await;
    }

    #[tokio::test]
    #[ignore = "requires running Postgres"]
    async fn user_repo_link_github() {
        let db = TestDb::new().await;
        let repo = PgRepo::new(db.pool.clone());

        // Create regular user
        let user = repo.create_user("test@example.com", "password_hash").await.unwrap();

        // Link GitHub
        let github_id = 789;
        let github_login = "linkeduser";
        let token = "encrypted_token";
        repo.link_github(user.id, github_id, github_login, token).await.unwrap();

        // Verify link
        let fetched = repo.get_user_by_id(user.id).await.unwrap().unwrap();
        assert_eq!(fetched.github_id, Some(github_id));
        assert_eq!(fetched.github_login, Some(github_login.to_string()));

        db.cleanup().await;
    }

    #[tokio::test]
    #[ignore = "requires running Postgres"]
    async fn server_repo_health_check() {
        let db = TestDb::new().await;
        let repo = PgRepo::new(db.pool.clone());

        let healthy = repo.health_check().await;
        assert!(healthy);

        db.cleanup().await;
    }

    #[tokio::test]
    #[ignore = "requires running Postgres"]
    async fn server_repo_task_operations() {
        let db = TestDb::new().await;
        let repo = PgRepo::new(db.pool.clone());

        // Create a user first
        let user = repo.create_user("taskuser@example.com", "hash").await.unwrap();

        // Create a task
        let task = Task {
            id: TaskId(Uuid::new_v4()),
            slice_id: None,
            title: "Test Task".to_string(),
            description: "Test Description".to_string(),
            assigned_tier: AgentTier::Worker,
            assigned_agent: None,
            status: TaskStatus::Pending,
            priority: Priority::Normal,
            context_files: vec![],
            metadata: None,
            depends_on: vec![],
            retry_count: 0,
            max_retries: 3,
            last_error: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // Insert task
        repo.insert_task(user.id, task.clone()).await.unwrap();

        // Get task by UUID
        let fetched = repo.get_task_by_uuid(user.id, task.id.0).await.unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().title, "Test Task");

        // List tasks
        let tasks = repo.list_tasks(user.id, None, None).await.unwrap();
        assert!(!tasks.is_empty());

        db.cleanup().await;
    }

    #[tokio::test]
    #[ignore = "requires running Postgres"]
    async fn document_repo_create_and_get() {
        let db = TestDb::new().await;
        let repo = PgRepo::new(db.pool.clone());

        let user_id = Uuid::new_v4();
        let title = "Test Document".to_string();
        let content = "This is test content".to_string();
        let doc_type = "note".to_string();
        let ref_tag = "test-ref".to_string();
        let tags = vec!["tag1".to_string(), "tag2".to_string()];

        // Create document
        let doc = repo.create_document(user_id, None, title.clone(), content.clone(), doc_type, ref_tag.clone(), tags).await.unwrap();

        assert_eq!(doc.title, title);
        assert_eq!(doc.content, content);

        // Get by ID
        let fetched = repo.get_document(doc.id).await.unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().title, title);

        // Get by ref_tag
        let fetched_by_ref = repo.get_document_by_ref_tag(&ref_tag).await.unwrap();
        assert!(fetched_by_ref.is_some());
        assert_eq!(fetched_by_ref.unwrap().ref_tag.unwrap_or_default(), ref_tag);

        db.cleanup().await;
    }

    #[tokio::test]
    #[ignore = "requires running Postgres"]
    async fn document_repo_update() {
        let db = TestDb::new().await;
        let repo = PgRepo::new(db.pool.clone());

        let user_id = Uuid::new_v4();

        // Create document
        let doc = repo
            .create_document(
                user_id,
                None,
                "Original Title".to_string(),
                "Original Content".to_string(),
                "note".to_string(),
                "ref".to_string(),
                vec![],
            )
            .await
            .unwrap();

        // Update document
        let updated = repo
            .update_document(doc.id, Some("Updated Content".to_string()), Some("Updated Title".to_string()), None)
            .await
            .unwrap();

        assert_eq!(updated.title, "Updated Title");
        assert_eq!(updated.content, "Updated Content");

        db.cleanup().await;
    }

    #[tokio::test]
    #[ignore = "requires running Postgres"]
    async fn document_repo_delete() {
        let db = TestDb::new().await;
        let repo = PgRepo::new(db.pool.clone());

        let user_id = Uuid::new_v4();

        // Create document
        let doc = repo
            .create_document(user_id, None, "Title".to_string(), "Content".to_string(), "note".to_string(), "ref".to_string(), vec![])
            .await
            .unwrap();

        // Delete document
        repo.delete_document(doc.id).await.unwrap();

        // Verify deletion
        let fetched = repo.get_document(doc.id).await.unwrap();
        assert!(fetched.is_none());

        db.cleanup().await;
    }

    #[tokio::test]
    #[ignore = "requires running Postgres"]
    async fn document_repo_list_by_user() {
        let db = TestDb::new().await;
        let repo = PgRepo::new(db.pool.clone());

        let user_id = Uuid::new_v4();

        // Create multiple documents
        for i in 1..=3 {
            repo.create_document(user_id, None, format!("Doc {}", i), "Content".to_string(), "note".to_string(), format!("ref-{}", i), vec![])
                .await
                .unwrap();
        }

        // List documents
        let docs = repo.list_documents(user_id).await.unwrap();
        assert_eq!(docs.len(), 3);

        db.cleanup().await;
    }
}
