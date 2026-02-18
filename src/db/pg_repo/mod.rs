//! PostgreSQL implementation of repository traits.

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::traits::{
    AgentExecutionRepo, AgentRepo, AuthConfigRepo, ChatMessageRepo, ContentVersionRepo,
    CreateAgentExecutionInput, CreateDesignerOutputGenericInput, CreateDesignerOutputInput,
    CreateDocumentInput, CreateProtocolInput, CreateRoomInput, CreateStepInputPort,
    CreateWorkflowInput, DocumentRepo, InsertQueueEntryInput, MergeQueueRepo, ModelSpendRow,
    OutputSchemaRepo, PromptTemplateRepo, ProtocolRepo, ResultRepo, RoomMemberInput, RoomRepo,
    SaveRoomExecutionOutputInput, SessionRepo, SystemConfigRepo, TaskRepo, TokenLedgerRepo,
    ToolCapabilityRepo, ToolRepo, UpdateProtocolExecutionStatusInput, UpdateProtocolInput,
    UpdateRoomInput, UpdateWorkflowInput, UserRepo, WorkflowCollectionRepo, WorkflowRepo,
    WorkflowStepAgentRepo,
};
use crate::db::{
    AgentDesignerOutputRow, AgentDesignerRunRow, AgentExecutionRow, AgentRow,
    BeliefExtractionPlanRow, BeliefRow, ChatMessageRow, CollectionRunRow,
    CollectionWorkflowEdgeRow, CollectionWorkflowRow, ContentVersionRow, DocumentRow,
    DocumentSearchResult, EnvelopeSnapshotRow, ExecutionMessageRow, OutputSchemaRow,
    PromptTemplateRow, ProtocolDocumentDefRow, ProtocolExecutionRow, ProtocolPortRow, ProtocolRow,
    ResultRow, RoomExecutionOutputRow, RoomMemberRow, RoomRow, RoomSessionRow, RoomStepConfigRow,
    RoomStepMemberRow, RoomTranscriptEntry, RunSnapshotRow, RunTemplateRow, SessionRow,
    StepDocumentRow, StepInputRow, StepOutputRow, StepRoutingRuleRow, SystemConfigRow,
    TaskAgentRosterRow, TaskMissionBriefRow, TokenLedgerRow, ToolCapabilityRow, ToolRow,
    WorkflowCollectionRow, WorkflowExecutionRow, WorkflowRow, WorkflowStepAgentRow,
    WorkflowStepEdgeRow, WorkflowStepProtocolRow, WorkflowStepRow,
};
use crate::github::{PrQueueEntry, QueueError as MergeQueueError};
use crate::types::{Task, User, UserId};

/// Maximum retries on serialization failure (Postgres error 40001).
const SERIALIZABLE_MAX_RETRIES: u32 = 3;

/// Check whether a sqlx error is a Postgres serialization failure (40001).
fn is_serialization_failure(e: &sqlx::Error) -> bool {
    matches!(
        e,
        sqlx::Error::Database(db_err) if db_err.code().as_deref() == Some("40001")
    )
}

/// Check whether an anyhow error wraps a serialization failure.
#[allow(dead_code)]
fn is_serialization_failure_anyhow(e: &anyhow::Error) -> bool {
    e.downcast_ref::<sqlx::Error>()
        .is_some_and(is_serialization_failure)
}

/// Run a block inside a SERIALIZABLE transaction with automatic retry on
/// serialization failure (Postgres error code 40001). The block receives `$tx`
/// as a mutable transaction — use `&mut *$tx` for query execution. The macro
/// handles commit; do NOT commit inside the block. Use `?` normally — errors
/// are caught and classified (serialization failures trigger retry, others
/// propagate immediately).
macro_rules! run_serializable {
    ($pool:expr, |$tx:ident| { $($body:tt)* }) => {{
        let mut _last_err: Option<anyhow::Error> = None;
        let mut _succeeded = false;
        for _attempt in 0..SERIALIZABLE_MAX_RETRIES {
            let mut $tx = $pool.begin().await?;
            sqlx::query("SET TRANSACTION ISOLATION LEVEL SERIALIZABLE")
                .execute(&mut *$tx)
                .await?;

            let _body_result: Result<()> = (async {
                $($body)*
            }).await;

            match _body_result {
                Ok(()) => match $tx.commit().await {
                    Ok(()) => {
                        _succeeded = true;
                        break;
                    }
                    Err(e) if is_serialization_failure(&e) => {
                        tracing::warn!(
                            attempt = _attempt,
                            "serialization failure on commit, retrying"
                        );
                        _last_err = Some(anyhow::Error::from(e));
                        continue;
                    }
                    Err(e) => return Err(e.into()),
                },
                Err(e) if is_serialization_failure_anyhow(&e) => {
                    tracing::warn!(attempt = _attempt, "serialization failure, retrying");
                    _last_err = Some(e);
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
        if !_succeeded {
            return Err(_last_err.unwrap_or_else(|| {
                anyhow::anyhow!("serializable transaction failed after max retries")
            }));
        }
        Ok(())
    }};
}

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
    async fn insert_queue_entry(
        &self,
        input: InsertQueueEntryInput,
    ) -> Result<(), MergeQueueError> {
        sqlx::query(
            r#"
            INSERT INTO pr_merge_queue (
                id, repo_owner, repo_name, pr_number,
                queue_position, status, created_at, updated_at, user_id
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ON CONFLICT (repo_owner, repo_name, pr_number)
            DO UPDATE SET updated_at = excluded.updated_at
            "#,
        )
        .bind(input.id)
        .bind(&input.owner)
        .bind(&input.repo)
        .bind(input.pr_number as i32)
        .bind(input.position as i32)
        .bind("pending")
        .bind(input.now)
        .bind(input.now)
        .bind(input.user_id)
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

    async fn delete_queue_entry(
        &self,
        owner: String,
        repo: String,
        pr_number: u32,
    ) -> Result<bool, MergeQueueError> {
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

    async fn get_queue_entries(
        &self,
        owner: String,
        repo: String,
    ) -> Result<Vec<PrQueueEntry>, MergeQueueError> {
        let rows: Vec<(
            Uuid,
            String,
            String,
            i32,
            i32,
            String,
            Option<String>,
            Option<String>,
            DateTime<Utc>,
            DateTime<Utc>,
        )> = sqlx::query_as(
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

    async fn update_entry_status(
        &self,
        owner: String,
        repo: String,
        pr_number: u32,
        status: String,
        error_message: Option<String>,
        now: DateTime<Utc>,
    ) -> Result<bool, MergeQueueError> {
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

    async fn set_entry_conflict(
        &self,
        owner: String,
        repo: String,
        pr_number: u32,
        conflict_json: String,
        now: DateTime<Utc>,
    ) -> Result<bool, MergeQueueError> {
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

    async fn update_entry_position(
        &self,
        id: Uuid,
        position: u32,
        now: DateTime<Utc>,
    ) -> Result<(), MergeQueueError> {
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

    async fn reset_interrupted(
        &self,
        owner: String,
        repo: String,
        now: DateTime<Utc>,
    ) -> Result<u32, MergeQueueError> {
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

    async fn cleanup_old(
        &self,
        owner: String,
        repo: String,
        cutoff: DateTime<Utc>,
    ) -> Result<u32, MergeQueueError> {
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
// ============================================================================
// Auth Config Repository
// ============================================================================
#[async_trait]
impl AuthConfigRepo for PgRepo {
    async fn health_check(&self) -> bool {
        sqlx::query("SELECT 1").fetch_one(&self.pool).await.is_ok()
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
}

// ============================================================================
// Task Repository
// ============================================================================

#[async_trait]
impl TaskRepo for PgRepo {
    async fn list_tasks(
        &self,
        user_id: UserId,
        status: Option<String>,
        limit: Option<u32>,
    ) -> Result<Vec<Task>> {
        crate::db::list_tasks(&self.pool, user_id, status.as_deref(), limit).await
    }

    async fn get_task_by_uuid(&self, user_id: UserId, id: Uuid) -> Result<Option<Task>> {
        crate::db::get_task_by_uuid(&self.pool, user_id, id).await
    }

    async fn insert_task(&self, user_id: UserId, task: Task) -> Result<()> {
        crate::db::insert_task(&self.pool, user_id, &task).await
    }
}

// ============================================================================
// Chat Message Repository
// ============================================================================

#[async_trait]
impl ChatMessageRepo for PgRepo {
    async fn insert_chat_message(
        &self,
        user_id: UserId,
        id: Uuid,
        role: String,
        content: String,
    ) -> Result<()> {
        crate::db::insert_chat_message(&self.pool, user_id, &id, &role, &content).await
    }

    async fn get_chat_history(
        &self,
        user_id: UserId,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<ChatMessageRow>> {
        crate::db::get_chat_history(&self.pool, user_id, limit, offset).await
    }

    async fn clear_chat_history(&self, user_id: UserId) -> Result<()> {
        crate::db::clear_chat_history(&self.pool, user_id).await
    }
}

// ============================================================================
// Agent Repository
// ============================================================================

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

// ============================================================================
// Tool Repository
// ============================================================================

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
        for tool in crate::agents::execution_tools::builtin_tool_rows() {
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

// ============================================================================
// Session Repository
// ============================================================================

#[async_trait]
impl SessionRepo for PgRepo {
    async fn create_session(
        &self,
        user_id: UserId,
        session_id: Uuid,
        mode_id: &str,
        title: &str,
        agent_id: Option<Uuid>,
        draft_config: Option<serde_json::Value>,
    ) -> Result<()> {
        crate::db::create_session(
            &self.pool,
            user_id,
            session_id,
            mode_id,
            title,
            agent_id,
            draft_config,
        )
        .await
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

    async fn insert_session_message(
        &self,
        user_id: UserId,
        session_id: Uuid,
        id: Uuid,
        role: String,
        content: String,
    ) -> Result<()> {
        crate::db::insert_session_message(&self.pool, user_id, session_id, &id, &role, &content)
            .await
    }

    async fn get_session_history(
        &self,
        session_id: Uuid,
        limit: u32,
    ) -> Result<Vec<ChatMessageRow>> {
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
        let row: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM chat_messages WHERE session_id = $1")
                .bind(session_id)
                .fetch_one(&self.pool)
                .await?;
        Ok(row.0 as u32)
    }

    async fn update_session_draft_config(
        &self,
        session_id: Uuid,
        draft_config: Option<serde_json::Value>,
    ) -> Result<()> {
        crate::db::update_session_draft_config(&self.pool, session_id, draft_config).await
    }

    async fn clear_session_messages(&self, session_id: Uuid) -> Result<()> {
        crate::db::clear_session_messages(&self.pool, session_id).await
    }

    async fn find_session_by_step_id(&self, step_id: Uuid) -> Result<Option<SessionRow>> {
        crate::db::find_session_by_step_id(&self.pool, step_id).await
    }

    async fn link_session_agent(&self, session_id: Uuid, agent_id: Uuid) -> Result<()> {
        crate::db::link_session_agent(&self.pool, session_id, agent_id).await
    }
}

// ============================================================================
// User Repository
// ============================================================================

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

#[derive(sqlx::FromRow)]
struct UserRow {
    id: Uuid,
    email: String,
    password_hash: Option<String>,
    github_id: Option<i64>,
    github_login: Option<String>,
    github_token_encrypted: Option<String>,
    is_admin: bool,
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
            is_admin: row.is_admin,
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
            RETURNING id, email, password_hash, github_id, github_login, github_token_encrypted, is_admin, created_at, updated_at
            "#,
        )
        .bind(email)
        .bind(password_hash)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.into())
    }

    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>> {
        let row: Option<UserRow> = sqlx::query_as("SELECT id, email, password_hash, github_id, github_login, github_token_encrypted, is_admin, created_at, updated_at FROM users WHERE email = $1")
            .bind(email)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.map(|r| r.into()))
    }

    async fn get_user_by_id(&self, id: UserId) -> Result<Option<User>> {
        let row: Option<UserRow> = sqlx::query_as("SELECT id, email, password_hash, github_id, github_login, github_token_encrypted, is_admin, created_at, updated_at FROM users WHERE id = $1")
            .bind(id.0)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.map(|r| r.into()))
    }

    async fn get_user_by_github_id(&self, github_id: i64) -> Result<Option<User>> {
        let row: Option<UserRow> = sqlx::query_as("SELECT id, email, password_hash, github_id, github_login, github_token_encrypted, is_admin, created_at, updated_at FROM users WHERE github_id = $1")
            .bind(github_id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.map(|r| r.into()))
    }

    async fn link_github(
        &self,
        user_id: UserId,
        github_id: i64,
        github_login: &str,
        token_encrypted: &str,
    ) -> Result<()> {
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

    async fn create_github_user(
        &self,
        email: &str,
        github_id: i64,
        github_login: &str,
        token_encrypted: &str,
    ) -> Result<User> {
        let row: UserRow = sqlx::query_as(
            r#"
            INSERT INTO users (id, email, github_id, github_login, github_token_encrypted, created_at, updated_at)
            VALUES (gen_random_uuid(), $1, $2, $3, $4, NOW(), NOW())
            RETURNING id, email, password_hash, github_id, github_login, github_token_encrypted, is_admin, created_at, updated_at
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
    async fn create_document(&self, input: CreateDocumentInput) -> Result<DocumentRow> {
        let id = Uuid::new_v4();
        let row: DocumentRow = sqlx::query_as(
            r#"
            INSERT INTO documents (id, user_id, session_id, title, content, doc_type, ref_tag, tags)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id, user_id, session_id, title, content, summary, doc_type, ref_tag, tags, created_at, updated_at, workflow_id, target_length, is_static, source_protocol_step_id
            "#,
        )
        .bind(id)
        .bind(input.user_id)
        .bind(input.session_id)
        .bind(&input.title)
        .bind(&input.content)
        .bind(&input.doc_type)
        .bind(&input.ref_tag)
        .bind(&input.tags)
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }

    async fn create_workflow_document(
        &self,
        user_id: Uuid,
        title: String,
        workflow_id: Uuid,
        target_length: Option<i32>,
        source_protocol_step_id: Option<Uuid>,
    ) -> Result<DocumentRow> {
        let id = Uuid::new_v4();
        let row: DocumentRow = sqlx::query_as(
            r#"
            INSERT INTO documents (id, user_id, title, content, doc_type, workflow_id, target_length, is_static, source_protocol_step_id)
            VALUES ($1, $2, $3, '', 'protocol', $4, $5, false, $6)
            RETURNING id, user_id, session_id, title, content, summary, doc_type, ref_tag, tags, created_at, updated_at, workflow_id, target_length, is_static, source_protocol_step_id
            "#,
        )
        .bind(id)
        .bind(user_id)
        .bind(&title)
        .bind(workflow_id)
        .bind(target_length)
        .bind(source_protocol_step_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }

    async fn update_document(
        &self,
        doc_id: Uuid,
        content: Option<String>,
        title: Option<String>,
        tags: Option<Vec<String>>,
    ) -> Result<DocumentRow> {
        let row: DocumentRow = sqlx::query_as(
            r#"
            UPDATE documents
            SET
                content = COALESCE($1, content),
                title = COALESCE($2, title),
                tags = COALESCE($3, tags),
                updated_at = NOW()
            WHERE id = $4
            RETURNING id, user_id, session_id, title, content, summary, doc_type, ref_tag, tags, created_at, updated_at, workflow_id, target_length, is_static, source_protocol_step_id
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
        let row: Option<DocumentRow> = sqlx::query_as("SELECT id, user_id, session_id, title, content, summary, doc_type, ref_tag, tags, created_at, updated_at, workflow_id, target_length, is_static, source_protocol_step_id FROM documents WHERE id = $1")
            .bind(doc_id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row)
    }

    async fn get_document_by_ref_tag(&self, ref_tag: &str) -> Result<Option<DocumentRow>> {
        let row: Option<DocumentRow> = sqlx::query_as("SELECT id, user_id, session_id, title, content, summary, doc_type, ref_tag, tags, created_at, updated_at, workflow_id, target_length, is_static, source_protocol_step_id FROM documents WHERE ref_tag = $1")
            .bind(ref_tag)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row)
    }

    async fn list_documents(&self, user_id: Uuid) -> Result<Vec<DocumentRow>> {
        let rows: Vec<DocumentRow> =
            sqlx::query_as("SELECT id, user_id, session_id, title, content, summary, doc_type, ref_tag, tags, created_at, updated_at, workflow_id, target_length, is_static, source_protocol_step_id FROM documents WHERE user_id = $1 ORDER BY updated_at DESC")
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

    async fn search_documents(
        &self,
        user_id: Uuid,
        query: &str,
    ) -> Result<Vec<DocumentSearchResult>> {
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
        sqlx::query("DELETE FROM documents WHERE id = $1")
            .bind(doc_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

#[async_trait]
impl OutputSchemaRepo for PgRepo {
    async fn create_output_schema(
        &self,
        user_id: Option<Uuid>,
        name: String,
        schema: serde_json::Value,
    ) -> Result<OutputSchemaRow> {
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
        let rows: Vec<OutputSchemaRow> = sqlx::query_as("SELECT id, user_id, name, schema, created_at, version FROM output_schemas WHERE user_id = $1 OR user_id IS NULL ORDER BY created_at DESC")
            .bind(user_id)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows)
    }

    async fn update_output_schema(
        &self,
        id: Uuid,
        name: Option<String>,
        schema: Option<serde_json::Value>,
    ) -> Result<OutputSchemaRow> {
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
        sqlx::query("DELETE FROM output_schemas WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

#[async_trait]
impl PromptTemplateRepo for PgRepo {
    async fn create_prompt_template(
        &self,
        user_id: Option<Uuid>,
        name: String,
        content: String,
    ) -> Result<PromptTemplateRow> {
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
        let rows: Vec<PromptTemplateRow> = sqlx::query_as("SELECT id, user_id, name, content, created_at, version FROM prompt_templates WHERE user_id = $1 OR user_id IS NULL ORDER BY created_at DESC")
            .bind(user_id)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows)
    }

    async fn update_prompt_template(
        &self,
        id: Uuid,
        name: Option<String>,
        content: Option<String>,
    ) -> Result<PromptTemplateRow> {
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
        sqlx::query("DELETE FROM prompt_templates WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

#[async_trait]
impl WorkflowRepo for PgRepo {
    // --- Workflows ---

    async fn create_workflow(&self, input: CreateWorkflowInput) -> Result<WorkflowRow> {
        let row: WorkflowRow = sqlx::query_as(
            "INSERT INTO workflows (user_id, name, description, container_enabled, target_repo_url, target_branch, vpn_enabled) \
             VALUES ($1, $2, $3, $4, $5, $6, $7) \
             RETURNING id, user_id, name, description, execution_mode, created_at, version, container_enabled, target_repo_url, target_branch, vpn_enabled, board_overview_summary",
        )
        .bind(input.user_id)
        .bind(&input.name)
        .bind(&input.description)
        .bind(input.container_enabled)
        .bind(&input.target_repo_url)
        .bind(&input.target_branch)
        .bind(input.vpn_enabled)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn get_workflow(&self, id: Uuid) -> Result<Option<WorkflowRow>> {
        let row: Option<WorkflowRow> = sqlx::query_as(
            "SELECT id, user_id, name, description, execution_mode, created_at, version, container_enabled, target_repo_url, target_branch, vpn_enabled, board_overview_summary \
             FROM workflows WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    async fn list_workflows(&self, user_id: Uuid) -> Result<Vec<WorkflowRow>> {
        let rows: Vec<WorkflowRow> = sqlx::query_as(
            "SELECT id, user_id, name, description, execution_mode, created_at, version, container_enabled, target_repo_url, target_branch, vpn_enabled, board_overview_summary \
             FROM workflows WHERE user_id = $1 ORDER BY created_at DESC",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn update_workflow(&self, input: UpdateWorkflowInput) -> Result<WorkflowRow> {
        // Build dynamic SET clauses for optional container fields
        let row: WorkflowRow = sqlx::query_as(
            "UPDATE workflows SET \
             name = COALESCE($1, name), \
             description = COALESCE($2, description), \
             container_enabled = COALESCE($3, container_enabled), \
             target_repo_url = CASE WHEN $4 THEN $5 ELSE target_repo_url END, \
             target_branch = CASE WHEN $6 THEN $7 ELSE target_branch END, \
             vpn_enabled = COALESCE($8, vpn_enabled), \
             version = version + 1 \
             WHERE id = $9 \
             RETURNING id, user_id, name, description, execution_mode, created_at, version, container_enabled, target_repo_url, target_branch, vpn_enabled, board_overview_summary",
        )
        .bind(input.name)
        .bind(input.description)
        .bind(input.container_enabled)
        .bind(input.target_repo_url.is_some()) // $4: whether to update target_repo_url
        .bind(input.target_repo_url.unwrap_or(None)) // $5: the value (may be None to clear)
        .bind(input.target_branch.is_some()) // $6: whether to update target_branch
        .bind(input.target_branch.unwrap_or(None)) // $7: the value (may be None to clear)
        .bind(input.vpn_enabled)
        .bind(input.id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn delete_workflow(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM workflows WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // --- Steps ---

    async fn create_step(&self, step: WorkflowStepRow) -> Result<WorkflowStepRow> {
        let row: WorkflowStepRow = sqlx::query_as(
            r#"
            INSERT INTO workflow_steps (id, workflow_id, agent_id, execution_mode, for_each_ref, prompt_template_id, prompt_template, output_schema_id, output_variable_name, interactive_agent_id, for_each_label_field, display_order, reasoning_trace, verification_agent_ids, position_x, position_y, width, height, name, system_prompt_suffix, visible, description, child_workflow_id, is_designer_step)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23, $24)
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
        .bind(step.reasoning_trace)
        .bind(&step.verification_agent_ids)
        .bind(step.position_x)
        .bind(step.position_y)
        .bind(step.width)
        .bind(step.height)
        .bind(&step.name)
        .bind(&step.system_prompt_suffix)
        .bind(step.visible)
        .bind(&step.description)
        .bind(step.child_workflow_id)
        .bind(step.is_designer_step)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn get_step(&self, id: Uuid) -> Result<Option<WorkflowStepRow>> {
        let row: Option<WorkflowStepRow> =
            sqlx::query_as("SELECT * FROM workflow_steps WHERE id = $1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;
        Ok(row)
    }

    async fn list_steps(&self, workflow_id: Uuid) -> Result<Vec<WorkflowStepRow>> {
        let rows: Vec<WorkflowStepRow> = sqlx::query_as(
            "SELECT * FROM workflow_steps WHERE workflow_id = $1 ORDER BY display_order",
        )
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
                reasoning_trace = $11, verification_agent_ids = $12, position_x = $13, position_y = $14, width = $15, height = $16,
                name = $17, system_prompt_suffix = $18, visible = $19, description = $20, child_workflow_id = $21, is_designer_step = $22,
                version = version + 1
            WHERE id = $23
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
        .bind(step.reasoning_trace)
        .bind(&step.verification_agent_ids)
        .bind(step.position_x)
        .bind(step.position_y)
        .bind(step.width)
        .bind(step.height)
        .bind(&step.name)
        .bind(&step.system_prompt_suffix)
        .bind(step.visible)
        .bind(&step.description)
        .bind(step.child_workflow_id)
        .bind(step.is_designer_step)
        .bind(step.id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn delete_step(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM workflow_steps WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
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
            sqlx::query(
                "INSERT INTO workflow_step_edges (workflow_id, from_step_id, to_step_id) VALUES ($1, $2, $3)",
            )
            .bind(workflow_id)
            .bind(edge.from_step_id)
            .bind(edge.to_step_id)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    async fn list_edges(&self, workflow_id: Uuid) -> Result<Vec<WorkflowStepEdgeRow>> {
        let rows: Vec<WorkflowStepEdgeRow> = sqlx::query_as(
            "SELECT e.id, e.from_step_id, e.to_step_id, e.from_output_port, e.to_input_port, \
             e.transform_jsonpath, e.condition_type, e.condition_value, e.edge_label, e.workflow_id \
             FROM workflow_step_edges e \
             JOIN workflow_steps s ON e.from_step_id = s.id \
             WHERE s.workflow_id = $1"
        )
            .bind(workflow_id)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows)
    }

    async fn add_edge(
        &self,
        workflow_id: Uuid,
        from_step_id: Uuid,
        to_step_id: Uuid,
    ) -> Result<WorkflowStepEdgeRow> {
        let row: WorkflowStepEdgeRow = sqlx::query_as(
            "INSERT INTO workflow_step_edges (workflow_id, from_step_id, to_step_id) VALUES ($1, $2, $3) ON CONFLICT (workflow_id, from_step_id, to_step_id) DO UPDATE SET from_step_id = EXCLUDED.from_step_id RETURNING *",
        )
        .bind(workflow_id)
        .bind(from_step_id)
        .bind(to_step_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn remove_edge(&self, from_step_id: Uuid, to_step_id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM workflow_step_edges WHERE from_step_id = $1 AND to_step_id = $2")
            .bind(from_step_id)
            .bind(to_step_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn delete_edge_by_id(&self, edge_id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM workflow_step_edges WHERE id = $1")
            .bind(edge_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // --- Step documents ---

    async fn list_step_documents(&self, step_id: Uuid) -> Result<Vec<StepDocumentRow>> {
        let rows: Vec<StepDocumentRow> =
            sqlx::query_as("SELECT step_id, document_id FROM step_documents WHERE step_id = $1")
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

    // --- Protocol Document Definitions ---

    async fn get_document_def(&self, id: Uuid) -> Result<Option<ProtocolDocumentDefRow>> {
        let row = sqlx::query_as::<_, ProtocolDocumentDefRow>(
            "SELECT id, step_id, name, description, target_length, display_order, created_at, protocol_id, document_id, agent_roster_entry_id FROM protocol_document_defs WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    async fn list_document_defs(&self, step_id: Uuid) -> Result<Vec<ProtocolDocumentDefRow>> {
        let rows = sqlx::query_as::<_, ProtocolDocumentDefRow>(
            "SELECT id, step_id, name, description, target_length, display_order, created_at, protocol_id, document_id, agent_roster_entry_id FROM protocol_document_defs WHERE step_id = $1 ORDER BY display_order, created_at",
        )
        .bind(step_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn create_document_def(
        &self,
        def: ProtocolDocumentDefRow,
    ) -> Result<ProtocolDocumentDefRow> {
        let row = sqlx::query_as::<_, ProtocolDocumentDefRow>(
            "INSERT INTO protocol_document_defs (id, step_id, name, description, target_length, display_order, protocol_id, document_id, agent_roster_entry_id) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING id, step_id, name, description, target_length, display_order, created_at, protocol_id, document_id, agent_roster_entry_id",
        )
        .bind(def.id)
        .bind(def.step_id)
        .bind(&def.name)
        .bind(&def.description)
        .bind(def.target_length)
        .bind(def.display_order)
        .bind(def.protocol_id)
        .bind(def.document_id)
        .bind(def.agent_roster_entry_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn update_document_def(
        &self,
        id: Uuid,
        name: String,
        description: String,
        target_length: i32,
    ) -> Result<ProtocolDocumentDefRow> {
        let row = sqlx::query_as::<_, ProtocolDocumentDefRow>(
            "UPDATE protocol_document_defs SET name = $2, description = $3, target_length = $4 WHERE id = $1 RETURNING id, step_id, name, description, target_length, display_order, created_at, protocol_id, document_id, agent_roster_entry_id",
        )
        .bind(id)
        .bind(&name)
        .bind(&description)
        .bind(target_length)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn link_document_to_def(&self, def_id: Uuid, document_id: Uuid) -> Result<()> {
        sqlx::query("UPDATE protocol_document_defs SET document_id = $1 WHERE id = $2")
            .bind(document_id)
            .bind(def_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn delete_document_def(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM protocol_document_defs WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // --- Port Management (Phase 3) ---

    async fn get_step_inputs(&self, workflow_step_id: Uuid) -> Result<Vec<StepInputRow>> {
        let rows = sqlx::query_as::<_, StepInputRow>(
            "SELECT id, workflow_step_id, port_name, port_type, required, default_value, description, json_schema, created_at
             FROM step_inputs
             WHERE workflow_step_id = $1
             ORDER BY port_name"
        )
        .bind(workflow_step_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn get_step_outputs(&self, workflow_step_id: Uuid) -> Result<Vec<StepOutputRow>> {
        let rows = sqlx::query_as::<_, StepOutputRow>(
            "SELECT id, workflow_step_id, port_name, port_type, json_path, description, json_schema, created_at
             FROM step_outputs
             WHERE workflow_step_id = $1
             ORDER BY port_name"
        )
        .bind(workflow_step_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn create_step_input(&self, input: CreateStepInputPort) -> Result<StepInputRow> {
        let row = sqlx::query_as::<_, StepInputRow>(
            "INSERT INTO step_inputs (workflow_step_id, port_name, port_type, required, default_value, description, json_schema)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             RETURNING id, workflow_step_id, port_name, port_type, required, default_value, description, json_schema, created_at"
        )
        .bind(input.workflow_step_id)
        .bind(&input.port_name)
        .bind(&input.port_type)
        .bind(input.required)
        .bind(input.default_value)
        .bind(input.description)
        .bind(input.json_schema)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn create_step_output(
        &self,
        workflow_step_id: Uuid,
        port_name: &str,
        port_type: &str,
        json_path: &str,
        description: Option<String>,
        json_schema: Option<serde_json::Value>,
    ) -> Result<StepOutputRow> {
        let row = sqlx::query_as::<_, StepOutputRow>(
            "INSERT INTO step_outputs (workflow_step_id, port_name, port_type, json_path, description, json_schema)
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING id, workflow_step_id, port_name, port_type, json_path, description, json_schema, created_at"
        )
        .bind(workflow_step_id)
        .bind(port_name)
        .bind(port_type)
        .bind(json_path)
        .bind(description)
        .bind(json_schema)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn delete_step_input(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM step_inputs WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn delete_step_output(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM step_outputs WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // --- Routing Rules (Phase 3) ---

    async fn get_step_routing_rules(
        &self,
        workflow_step_id: Uuid,
    ) -> Result<Vec<StepRoutingRuleRow>> {
        let rows = sqlx::query_as::<_, StepRoutingRuleRow>(
            "SELECT id, workflow_step_id, label_value, description, agent_id, display_order, created_at
             FROM step_routing_rules
             WHERE workflow_step_id = $1
             ORDER BY display_order, label_value"
        )
        .bind(workflow_step_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn create_routing_rule(
        &self,
        workflow_step_id: Uuid,
        label_value: &str,
        agent_id: Uuid,
        description: Option<String>,
        display_order: i32,
    ) -> Result<StepRoutingRuleRow> {
        let row = sqlx::query_as::<_, StepRoutingRuleRow>(
            "INSERT INTO step_routing_rules (workflow_step_id, label_value, agent_id, description, display_order)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING id, workflow_step_id, label_value, description, agent_id, display_order, created_at"
        )
        .bind(workflow_step_id)
        .bind(label_value)
        .bind(agent_id)
        .bind(description)
        .bind(display_order)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn update_routing_rule(
        &self,
        id: Uuid,
        agent_id: Option<Uuid>,
        description: Option<String>,
        display_order: Option<i32>,
    ) -> Result<StepRoutingRuleRow> {
        let row = sqlx::query_as::<_, StepRoutingRuleRow>(
            "UPDATE step_routing_rules SET
                agent_id = COALESCE($2, agent_id),
                description = COALESCE($3, description),
                display_order = COALESCE($4, display_order)
             WHERE id = $1
             RETURNING id, workflow_step_id, label_value, description, agent_id, display_order, created_at"
        )
        .bind(id)
        .bind(agent_id)
        .bind(description)
        .bind(display_order)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn delete_routing_rule(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM step_routing_rules WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn find_step_by_room_id(&self, room_id: Uuid) -> Result<Option<WorkflowStepRow>> {
        let row = sqlx::query_as::<_, WorkflowStepRow>(
            "SELECT * FROM workflow_steps WHERE room_id = $1 LIMIT 1",
        )
        .bind(room_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    // --- Task Force (Mission Briefs + Agent Roster) ---

    async fn get_mission_brief(&self, step_id: Uuid) -> Result<Option<TaskMissionBriefRow>> {
        let row = sqlx::query_as::<_, TaskMissionBriefRow>(
            "SELECT * FROM task_mission_briefs WHERE step_id = $1",
        )
        .bind(step_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    async fn upsert_mission_brief(
        &self,
        step_id: Uuid,
        task_description: &str,
        available_capabilities: &[String],
        failure_mode: &str,
        downstream_context: Option<String>,
    ) -> Result<TaskMissionBriefRow> {
        let row = sqlx::query_as::<_, TaskMissionBriefRow>(
            "INSERT INTO task_mission_briefs (step_id, task_description, available_capabilities, failure_mode, downstream_context)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (step_id) DO UPDATE SET
                task_description = EXCLUDED.task_description,
                available_capabilities = EXCLUDED.available_capabilities,
                failure_mode = EXCLUDED.failure_mode,
                downstream_context = EXCLUDED.downstream_context,
                updated_at = now()
             RETURNING *",
        )
        .bind(step_id)
        .bind(task_description)
        .bind(available_capabilities)
        .bind(failure_mode)
        .bind(downstream_context)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn list_agent_roster(&self, mission_brief_id: Uuid) -> Result<Vec<TaskAgentRosterRow>> {
        let rows = sqlx::query_as::<_, TaskAgentRosterRow>(
            "SELECT * FROM task_agent_roster WHERE mission_brief_id = $1 ORDER BY execution_order",
        )
        .bind(mission_brief_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn add_roster_agent(
        &self,
        mission_brief_id: Uuid,
        name: &str,
        role_description: &str,
        capabilities: &[String],
        execution_order: i32,
    ) -> Result<TaskAgentRosterRow> {
        let row = sqlx::query_as::<_, TaskAgentRosterRow>(
            "INSERT INTO task_agent_roster (mission_brief_id, name, role_description, capabilities, execution_order)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING *",
        )
        .bind(mission_brief_id)
        .bind(name)
        .bind(role_description)
        .bind(capabilities)
        .bind(execution_order)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn update_roster_agent(
        &self,
        agent_id: Uuid,
        name: Option<String>,
        role_description: Option<String>,
        capabilities: Option<Vec<String>>,
    ) -> Result<TaskAgentRosterRow> {
        let row = sqlx::query_as::<_, TaskAgentRosterRow>(
            "UPDATE task_agent_roster SET
                name = COALESCE($2, name),
                role_description = COALESCE($3, role_description),
                capabilities = COALESCE($4, capabilities)
             WHERE id = $1
             RETURNING *",
        )
        .bind(agent_id)
        .bind(name)
        .bind(role_description)
        .bind(capabilities)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn remove_roster_agent(&self, agent_id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM task_agent_roster WHERE id = $1")
            .bind(agent_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn link_roster_agent_to_child_step(
        &self,
        agent_id: Uuid,
        child_step_id: Option<Uuid>,
    ) -> Result<()> {
        sqlx::query("UPDATE task_agent_roster SET child_step_id = $1 WHERE id = $2")
            .bind(child_step_id)
            .bind(agent_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // --- Belief Capture (Extraction Plans) ---

    async fn get_extraction_plan(&self, step_id: Uuid) -> Result<Option<BeliefExtractionPlanRow>> {
        let row = sqlx::query_as::<_, BeliefExtractionPlanRow>(
            "SELECT * FROM belief_extraction_plans WHERE step_id = $1",
        )
        .bind(step_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    async fn upsert_extraction_plan(
        &self,
        step_id: Uuid,
        extraction_focus: &str,
        tag_vocabulary: &[String],
        contradiction_handling: &str,
        confidence_threshold: &str,
    ) -> Result<BeliefExtractionPlanRow> {
        let row = sqlx::query_as::<_, BeliefExtractionPlanRow>(
            "INSERT INTO belief_extraction_plans (step_id, extraction_focus, tag_vocabulary, contradiction_handling, confidence_threshold)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (step_id) DO UPDATE SET
                extraction_focus = EXCLUDED.extraction_focus,
                tag_vocabulary = EXCLUDED.tag_vocabulary,
                contradiction_handling = EXCLUDED.contradiction_handling,
                confidence_threshold = EXCLUDED.confidence_threshold,
                updated_at = now()
             RETURNING *",
        )
        .bind(step_id)
        .bind(extraction_focus)
        .bind(tag_vocabulary)
        .bind(contradiction_handling)
        .bind(confidence_threshold)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    // --- Belief Capture (Runtime Beliefs) ---

    async fn insert_belief(&self, belief: &BeliefRow) -> Result<BeliefRow> {
        let row = sqlx::query_as::<_, BeliefRow>(
            "INSERT INTO beliefs (
                id, workflow_id, workflow_execution_id, source_step_id,
                source_document_title, source_document_def_id, source_phase,
                content, reasoning, belief_type, confidence,
                confidence_justification, semantic_tags, emotional_tone,
                cross_source_tension, source_step_name, extraction_model,
                extraction_tokens_in, extraction_tokens_out
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
                $11, $12, $13, $14, $15, $16, $17, $18, $19
            ) RETURNING *",
        )
        .bind(belief.id)
        .bind(belief.workflow_id)
        .bind(belief.workflow_execution_id)
        .bind(belief.source_step_id)
        .bind(&belief.source_document_title)
        .bind(belief.source_document_def_id)
        .bind(&belief.source_phase)
        .bind(&belief.content)
        .bind(&belief.reasoning)
        .bind(&belief.belief_type)
        .bind(&belief.confidence)
        .bind(&belief.confidence_justification)
        .bind(&belief.semantic_tags)
        .bind(&belief.emotional_tone)
        .bind(&belief.cross_source_tension)
        .bind(&belief.source_step_name)
        .bind(&belief.extraction_model)
        .bind(belief.extraction_tokens_in)
        .bind(belief.extraction_tokens_out)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn list_beliefs_for_execution(
        &self,
        workflow_execution_id: Uuid,
    ) -> Result<Vec<BeliefRow>> {
        let rows = sqlx::query_as::<_, BeliefRow>(
            "SELECT * FROM beliefs WHERE workflow_execution_id = $1 ORDER BY created_at",
        )
        .bind(workflow_execution_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    // --- Chat Beliefs ---

    async fn replace_chat_beliefs(
        &self,
        step_id: Uuid,
        beliefs: &[BeliefRow],
    ) -> Result<Vec<BeliefRow>> {
        let mut tx = self.pool.begin().await?;

        sqlx::query("DELETE FROM beliefs WHERE source_step_id = $1 AND source_phase = 'chat'")
            .bind(step_id)
            .execute(&mut *tx)
            .await?;

        let mut inserted = Vec::with_capacity(beliefs.len());
        for belief in beliefs {
            let row = sqlx::query_as::<_, BeliefRow>(
                "INSERT INTO beliefs (
                    id, workflow_id, workflow_execution_id, source_step_id,
                    source_document_title, source_document_def_id, source_phase,
                    content, reasoning, belief_type, confidence,
                    confidence_justification, semantic_tags, emotional_tone,
                    cross_source_tension, source_step_name, extraction_model,
                    extraction_tokens_in, extraction_tokens_out
                ) VALUES (
                    $1, $2, $3, $4, $5, $6, $7, $8, $9, $10,
                    $11, $12, $13, $14, $15, $16, $17, $18, $19
                ) RETURNING *",
            )
            .bind(belief.id)
            .bind(belief.workflow_id)
            .bind(belief.workflow_execution_id)
            .bind(belief.source_step_id)
            .bind(&belief.source_document_title)
            .bind(belief.source_document_def_id)
            .bind(&belief.source_phase)
            .bind(&belief.content)
            .bind(&belief.reasoning)
            .bind(&belief.belief_type)
            .bind(&belief.confidence)
            .bind(&belief.confidence_justification)
            .bind(&belief.semantic_tags)
            .bind(&belief.emotional_tone)
            .bind(&belief.cross_source_tension)
            .bind(&belief.source_step_name)
            .bind(&belief.extraction_model)
            .bind(belief.extraction_tokens_in)
            .bind(belief.extraction_tokens_out)
            .fetch_one(&mut *tx)
            .await?;
            inserted.push(row);
        }

        tx.commit().await?;
        Ok(inserted)
    }

    async fn get_beliefs_for_connected_steps(
        &self,
        workflow_id: Uuid,
        step_id: Uuid,
    ) -> Result<Vec<BeliefRow>> {
        let rows = sqlx::query_as::<_, BeliefRow>(
            "SELECT b.* FROM beliefs b
             WHERE b.source_phase = 'chat'
             AND b.source_step_id IN (
                 SELECT e.from_step_id FROM workflow_step_edges e
                 WHERE e.to_step_id = $1 AND e.workflow_id = $2
                 UNION
                 SELECT e.to_step_id FROM workflow_step_edges e
                 WHERE e.from_step_id = $1 AND e.workflow_id = $2
             )
             ORDER BY b.source_step_name, b.belief_type, b.created_at",
        )
        .bind(step_id)
        .bind(workflow_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    // --- Room Step Config (Design-Time) ---

    async fn get_room_step_config(&self, step_id: Uuid) -> Result<Option<RoomStepConfigRow>> {
        let row = sqlx::query_as::<_, RoomStepConfigRow>(
            "SELECT * FROM room_step_configs WHERE step_id = $1",
        )
        .bind(step_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    async fn upsert_room_step_config(
        &self,
        step_id: Uuid,
        meeting_purpose: &str,
        max_turns: i32,
        interaction_mode: &str,
        gatekeeper_enabled: bool,
    ) -> Result<RoomStepConfigRow> {
        let row = sqlx::query_as::<_, RoomStepConfigRow>(
            "INSERT INTO room_step_configs (step_id, meeting_purpose, max_turns, interaction_mode, gatekeeper_enabled)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (step_id) DO UPDATE SET
                meeting_purpose = EXCLUDED.meeting_purpose,
                max_turns = EXCLUDED.max_turns,
                interaction_mode = EXCLUDED.interaction_mode,
                gatekeeper_enabled = EXCLUDED.gatekeeper_enabled,
                updated_at = now()
             RETURNING *",
        )
        .bind(step_id)
        .bind(meeting_purpose)
        .bind(max_turns)
        .bind(interaction_mode)
        .bind(gatekeeper_enabled)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn list_room_step_members(&self, step_id: Uuid) -> Result<Vec<RoomStepMemberRow>> {
        let rows = sqlx::query_as::<_, RoomStepMemberRow>(
            "SELECT * FROM room_step_members WHERE step_id = $1 ORDER BY display_order",
        )
        .bind(step_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn add_room_step_member(
        &self,
        step_id: Uuid,
        name: &str,
        role: &str,
        perspective: &str,
        display_order: i32,
    ) -> Result<RoomStepMemberRow> {
        let row = sqlx::query_as::<_, RoomStepMemberRow>(
            "INSERT INTO room_step_members (step_id, name, role, perspective, display_order)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING *",
        )
        .bind(step_id)
        .bind(name)
        .bind(role)
        .bind(perspective)
        .bind(display_order)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn update_room_step_member(
        &self,
        member_id: Uuid,
        name: Option<String>,
        role: Option<String>,
        perspective: Option<String>,
    ) -> Result<RoomStepMemberRow> {
        let row = sqlx::query_as::<_, RoomStepMemberRow>(
            "UPDATE room_step_members SET
                name = COALESCE($2, name),
                role = COALESCE($3, role),
                perspective = COALESCE($4, perspective),
                updated_at = now()
             WHERE id = $1
             RETURNING *",
        )
        .bind(member_id)
        .bind(name)
        .bind(role)
        .bind(perspective)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn remove_room_step_member(&self, member_id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM room_step_members WHERE id = $1")
            .bind(member_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // --- Agent Designer ---

    async fn create_designer_run(
        &self,
        workflow_execution_id: Uuid,
        stage_execution_id: Uuid,
        step_id: Uuid,
        mission_brief_id: Uuid,
        model_id: &str,
    ) -> Result<AgentDesignerRunRow> {
        let row = sqlx::query_as::<_, AgentDesignerRunRow>(
            "INSERT INTO agent_designer_runs \
             (workflow_execution_id, stage_execution_id, step_id, mission_brief_id, archetype, phase, model_id) \
             VALUES ($1, $2, $3, $4, 'task_force', '', $5) \
             RETURNING *",
        )
        .bind(workflow_execution_id)
        .bind(stage_execution_id)
        .bind(step_id)
        .bind(mission_brief_id)
        .bind(model_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn create_designer_run_generic(
        &self,
        workflow_execution_id: Uuid,
        stage_execution_id: Uuid,
        step_id: Uuid,
        archetype: &str,
        phase: &str,
        model_id: &str,
    ) -> Result<AgentDesignerRunRow> {
        let row = sqlx::query_as::<_, AgentDesignerRunRow>(
            "INSERT INTO agent_designer_runs \
             (workflow_execution_id, stage_execution_id, step_id, archetype, phase, model_id) \
             VALUES ($1, $2, $3, $4, $5, $6) \
             RETURNING *",
        )
        .bind(workflow_execution_id)
        .bind(stage_execution_id)
        .bind(step_id)
        .bind(archetype)
        .bind(phase)
        .bind(model_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn update_designer_run_tokens(
        &self,
        run_id: Uuid,
        input_tokens: i64,
        output_tokens: i64,
        cost_usd: f32,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE agent_designer_runs SET input_tokens = $2, output_tokens = $3, cost_usd = $4 WHERE id = $1",
        )
        .bind(run_id)
        .bind(input_tokens)
        .bind(output_tokens)
        .bind(cost_usd)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn create_designer_output(
        &self,
        input: CreateDesignerOutputInput,
    ) -> Result<AgentDesignerOutputRow> {
        let row = sqlx::query_as::<_, AgentDesignerOutputRow>(
            "INSERT INTO agent_designer_outputs \
             (designer_run_id, agent_roster_entry_id, agent_name, assigned_tools, \
              generated_system_prompt, generated_task_prompt, design_reasoning, execution_order, \
              source_entity_id, source_archetype) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'task_force') \
             RETURNING *",
        )
        .bind(input.designer_run_id)
        .bind(input.agent_roster_entry_id)
        .bind(&input.agent_name)
        .bind(&input.assigned_tools)
        .bind(&input.generated_system_prompt)
        .bind(&input.generated_task_prompt)
        .bind(&input.design_reasoning)
        .bind(input.execution_order)
        .bind(input.agent_roster_entry_id.to_string())
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn create_designer_output_generic(
        &self,
        input: CreateDesignerOutputGenericInput,
    ) -> Result<AgentDesignerOutputRow> {
        let row = sqlx::query_as::<_, AgentDesignerOutputRow>(
            "INSERT INTO agent_designer_outputs \
             (designer_run_id, source_entity_id, source_archetype, agent_name, assigned_tools, \
              generated_system_prompt, generated_task_prompt, design_reasoning, execution_order, \
              protocol_execution_id) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) \
             RETURNING *",
        )
        .bind(input.designer_run_id)
        .bind(&input.source_entity_id)
        .bind(&input.source_archetype)
        .bind(&input.agent_name)
        .bind(&input.assigned_tools)
        .bind(&input.generated_system_prompt)
        .bind(&input.generated_task_prompt)
        .bind(&input.design_reasoning)
        .bind(input.execution_order)
        .bind(input.protocol_execution_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn list_designer_outputs(
        &self,
        designer_run_id: Uuid,
    ) -> Result<Vec<AgentDesignerOutputRow>> {
        let rows = sqlx::query_as::<_, AgentDesignerOutputRow>(
            "SELECT * FROM agent_designer_outputs WHERE designer_run_id = $1 ORDER BY execution_order",
        )
        .bind(designer_run_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn list_designer_outputs_by_protocol_execution(
        &self,
        protocol_execution_id: Uuid,
    ) -> Result<Vec<AgentDesignerOutputRow>> {
        let rows = sqlx::query_as::<_, AgentDesignerOutputRow>(
            "SELECT * FROM agent_designer_outputs \
             WHERE protocol_execution_id = $1 \
             ORDER BY execution_order",
        )
        .bind(protocol_execution_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn list_designer_runs_for_step(
        &self,
        step_id: Uuid,
        workflow_execution_id: Uuid,
    ) -> Result<Vec<AgentDesignerRunRow>> {
        let rows = sqlx::query_as::<_, AgentDesignerRunRow>(
            "SELECT * FROM agent_designer_runs \
             WHERE step_id = $1 AND workflow_execution_id = $2 \
             ORDER BY created_at",
        )
        .bind(step_id)
        .bind(workflow_execution_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    // --- Assistant Notes ---

    async fn get_assistant_notes(&self, step_id: Uuid) -> Result<Option<String>> {
        let row = sqlx::query_scalar::<_, String>(
            "SELECT content FROM assistant_notes WHERE step_id = $1",
        )
        .bind(step_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    async fn upsert_assistant_notes(&self, step_id: Uuid, content: &str) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO assistant_notes (step_id, content, updated_at)
            VALUES ($1, $2, now())
            ON CONFLICT (step_id) DO UPDATE
            SET content = $2, updated_at = now()
            "#,
        )
        .bind(step_id)
        .bind(content)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get_all_assistant_notes_for_workflow(
        &self,
        workflow_id: Uuid,
    ) -> Result<Vec<(Uuid, Option<String>, String, String)>> {
        let rows = sqlx::query_as::<_, (Uuid, Option<String>, String, String)>(
            r#"
            SELECT ws.id, ws.name, ws.execution_mode, an.content
            FROM workflow_steps ws
            JOIN assistant_notes an ON an.step_id = ws.id
            WHERE ws.workflow_id = $1 AND an.content != ''
            ORDER BY ws.name
            "#,
        )
        .bind(workflow_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    // --- Board Overview Summary ---

    async fn get_board_overview_summary(&self, workflow_id: Uuid) -> Result<String> {
        let summary = sqlx::query_scalar::<_, String>(
            "SELECT board_overview_summary FROM workflows WHERE id = $1",
        )
        .bind(workflow_id)
        .fetch_optional(&self.pool)
        .await?
        .unwrap_or_default();
        Ok(summary)
    }

    async fn update_board_overview_summary(&self, workflow_id: Uuid, summary: &str) -> Result<()> {
        sqlx::query("UPDATE workflows SET board_overview_summary = $1 WHERE id = $2")
            .bind(summary)
            .bind(workflow_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // --- Run Templates ---

    async fn create_template(
        &self,
        workflow_id: Uuid,
        user_id: Uuid,
        name: &str,
        description: Option<String>,
        snapshot: serde_json::Value,
    ) -> Result<RunTemplateRow> {
        let row = sqlx::query_as::<_, RunTemplateRow>(
            "INSERT INTO run_templates (workflow_id, user_id, name, description, snapshot) \
             VALUES ($1, $2, $3, $4, $5) RETURNING *",
        )
        .bind(workflow_id)
        .bind(user_id)
        .bind(name)
        .bind(description)
        .bind(&snapshot)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn get_template(&self, template_id: Uuid) -> Result<Option<RunTemplateRow>> {
        let row = sqlx::query_as::<_, RunTemplateRow>("SELECT * FROM run_templates WHERE id = $1")
            .bind(template_id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row)
    }

    async fn list_templates(&self, workflow_id: Uuid) -> Result<Vec<RunTemplateRow>> {
        let rows = sqlx::query_as::<_, RunTemplateRow>(
            "SELECT id, workflow_id, user_id, name, description, \
             '{}'::jsonb AS snapshot, created_at \
             FROM run_templates WHERE workflow_id = $1 ORDER BY created_at DESC",
        )
        .bind(workflow_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn delete_template(&self, template_id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM run_templates WHERE id = $1")
            .bind(template_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

#[async_trait]
impl AgentExecutionRepo for PgRepo {
    async fn create_agent_execution(
        &self,
        input: CreateAgentExecutionInput,
    ) -> Result<AgentExecutionRow> {
        let row = sqlx::query_as::<_, AgentExecutionRow>(
            "INSERT INTO agent_executions (agent_id, workflow_step_id, is_interactive, parent_agent_execution_id, system_prompt_rendered, input, room_session_id, speaker_order, workflow_execution_id) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING *",
        )
        .bind(input.agent_id)
        .bind(input.workflow_step_id)
        .bind(input.is_interactive)
        .bind(input.parent_agent_execution_id)
        .bind(&input.system_prompt_rendered)
        .bind(&input.input)
        .bind(input.room_session_id)
        .bind(input.speaker_order)
        .bind(input.workflow_execution_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn get_agent_execution(&self, id: Uuid) -> Result<Option<AgentExecutionRow>> {
        let row =
            sqlx::query_as::<_, AgentExecutionRow>("SELECT * FROM agent_executions WHERE id = $1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;
        Ok(row)
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

    async fn create_execution_message(
        &self,
        agent_execution_id: Uuid,
        role: &str,
        content: &str,
        tool_call_id: Option<String>,
        input_tokens: i64,
        output_tokens: i64,
    ) -> Result<ExecutionMessageRow> {
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

    async fn list_execution_messages(
        &self,
        agent_execution_id: Uuid,
    ) -> Result<Vec<ExecutionMessageRow>> {
        let rows = sqlx::query_as::<_, ExecutionMessageRow>("SELECT * FROM execution_messages WHERE agent_execution_id = $1 ORDER BY created_at ASC")
            .bind(agent_execution_id)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows)
    }

    async fn list_completed_executions_for_step_ids(
        &self,
        workflow_step_ids: &[Uuid],
    ) -> Result<Vec<AgentExecutionRow>> {
        let rows = sqlx::query_as::<_, AgentExecutionRow>(
            "SELECT * FROM agent_executions \
             WHERE workflow_step_id = ANY($1) \
               AND status = 'completed' \
               AND is_interactive = false \
             ORDER BY started_at ASC",
        )
        .bind(workflow_step_ids)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn list_interactive_executions_for_step(
        &self,
        workflow_step_id: Uuid,
    ) -> Result<Vec<AgentExecutionRow>> {
        let rows = sqlx::query_as::<_, AgentExecutionRow>(
            "SELECT * FROM agent_executions \
             WHERE workflow_step_id = $1 \
               AND is_interactive = true \
             ORDER BY started_at ASC",
        )
        .bind(workflow_step_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn list_agent_executions_for_step_and_run(
        &self,
        workflow_step_id: Uuid,
        workflow_execution_id: Uuid,
    ) -> Result<Vec<AgentExecutionRow>> {
        let rows = sqlx::query_as::<_, AgentExecutionRow>(
            "SELECT * FROM agent_executions \
             WHERE workflow_step_id = $1 AND workflow_execution_id = $2 \
             ORDER BY started_at DESC",
        )
        .bind(workflow_step_id)
        .bind(workflow_execution_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn list_agent_executions(
        &self,
        user_id: Uuid,
        status: Option<String>,
    ) -> Result<Vec<AgentExecutionRow>> {
        let rows = if let Some(ref s) = status {
            sqlx::query_as::<_, AgentExecutionRow>(
                "SELECT ae.* FROM agent_executions ae \
                 JOIN workflow_executions we ON ae.workflow_execution_id = we.id \
                 WHERE ae.status = $1 AND ae.is_interactive = true AND we.user_id = $2 \
                 ORDER BY ae.started_at DESC LIMIT 100",
            )
            .bind(s.as_str())
            .bind(user_id)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, AgentExecutionRow>(
                "SELECT ae.* FROM agent_executions ae \
                 JOIN workflow_executions we ON ae.workflow_execution_id = we.id \
                 WHERE ae.is_interactive = true AND we.user_id = $1 \
                 ORDER BY ae.started_at DESC LIMIT 100",
            )
            .bind(user_id)
            .fetch_all(&self.pool)
            .await?
        };
        Ok(rows)
    }

    async fn list_exemplary_executions(
        &self,
        agent_id: Uuid,
        workflow_step_id: Option<Uuid>,
        limit: u32,
    ) -> Result<Vec<AgentExecutionRow>> {
        let rows = sqlx::query_as::<_, AgentExecutionRow>(
            "SELECT * FROM agent_executions \
             WHERE agent_id = $1 \
               AND ($2::uuid IS NULL OR workflow_step_id = $2) \
               AND is_exemplary = true \
               AND status = 'completed' \
             ORDER BY completed_at DESC \
             LIMIT $3",
        )
        .bind(agent_id)
        .bind(workflow_step_id)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn set_execution_exemplary(
        &self,
        id: Uuid,
        is_exemplary: bool,
    ) -> Result<AgentExecutionRow> {
        let row = sqlx::query_as::<_, AgentExecutionRow>(
            "UPDATE agent_executions SET is_exemplary = $2 WHERE id = $1 RETURNING *",
        )
        .bind(id)
        .bind(is_exemplary)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }
}

#[async_trait]
impl TokenLedgerRepo for PgRepo {
    async fn insert_ledger_entry(
        &self,
        user_id: Uuid,
        agent_execution_id: Option<Uuid>,
        model_id: &str,
        input_tokens: i64,
        output_tokens: i64,
        cost_usd: f32,
    ) -> Result<TokenLedgerRow> {
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

    async fn get_model_breakdown(
        &self,
        user_id: Uuid,
        since: Option<DateTime<Utc>>,
    ) -> Result<Vec<ModelSpendRow>> {
        let rows = match since {
            Some(t) => {
                sqlx::query_as::<_, ModelSpendRow>("SELECT model_id, CAST(SUM(input_tokens) AS INT8) AS total_input_tokens, CAST(SUM(output_tokens) AS INT8) AS total_output_tokens, CAST(SUM(cost_usd) AS FLOAT8) AS total_cost_usd, COUNT(*) AS call_count FROM token_ledger WHERE user_id = $1 AND created_at >= $2 GROUP BY model_id ORDER BY total_cost_usd DESC")
                    .bind(user_id)
                    .bind(t)
                    .fetch_all(&self.pool)
                    .await?
            }
            None => {
                sqlx::query_as::<_, ModelSpendRow>("SELECT model_id, CAST(SUM(input_tokens) AS INT8) AS total_input_tokens, CAST(SUM(output_tokens) AS INT8) AS total_output_tokens, CAST(SUM(cost_usd) AS FLOAT8) AS total_cost_usd, COUNT(*) AS call_count FROM token_ledger WHERE user_id = $1 GROUP BY model_id ORDER BY total_cost_usd DESC")
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
    async fn save_result(
        &self,
        user_id: Uuid,
        agent_execution_id: Uuid,
        output_schema_id: Option<Uuid>,
        name: &str,
        data: serde_json::Value,
    ) -> Result<ResultRow> {
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
        let row = sqlx::query_as::<_, ResultRow>("SELECT * FROM results WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row)
    }

    async fn list_results(&self, user_id: Uuid) -> Result<Vec<ResultRow>> {
        let rows = sqlx::query_as::<_, ResultRow>(
            "SELECT * FROM results WHERE user_id = $1 ORDER BY created_at DESC",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn list_results_by_schema(
        &self,
        user_id: Uuid,
        output_schema_id: Uuid,
    ) -> Result<Vec<ResultRow>> {
        let rows = sqlx::query_as::<_, ResultRow>("SELECT * FROM results WHERE user_id = $1 AND output_schema_id = $2 ORDER BY created_at DESC")
            .bind(user_id)
            .bind(output_schema_id)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows)
    }

    async fn delete_result(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM results WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

// ============================================================================
// Room Repository
// ============================================================================

#[async_trait]
impl RoomRepo for PgRepo {
    async fn create_room(&self, input: CreateRoomInput) -> Result<RoomRow> {
        let row = sqlx::query_as::<_, RoomRow>(
            "INSERT INTO rooms (user_id, collection_id, name, gatekeeper_enabled, gatekeeper_model_id, max_speakers_per_turn, max_turns, tools_enabled) VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING *",
        )
        .bind(input.user_id)
        .bind(input.collection_id)
        .bind(&input.name)
        .bind(input.gatekeeper_enabled)
        .bind(&input.gatekeeper_model_id)
        .bind(input.max_speakers_per_turn)
        .bind(input.max_turns)
        .bind(input.tools_enabled)
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

    async fn update_room(&self, input: UpdateRoomInput) -> Result<RoomRow> {
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
        .bind(input.id)
        .bind(input.name)
        .bind(input.gatekeeper_enabled)
        .bind(input.gatekeeper_model_id)
        .bind(input.max_speakers_per_turn)
        .bind(input.max_turns)
        .bind(input.tools_enabled)
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
        let rows = sqlx::query_as::<_, RoomMemberRow>("SELECT room_id, agent_id, display_name, role_description, display_order FROM room_members WHERE room_id = $1 ORDER BY display_order ASC")
            .bind(room_id)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows)
    }

    async fn add_room_member(
        &self,
        room_id: Uuid,
        agent_id: Uuid,
        display_name: Option<String>,
        role_description: String,
        display_order: i32,
    ) -> Result<()> {
        sqlx::query("INSERT INTO room_members (room_id, agent_id, display_name, role_description, display_order) VALUES ($1, $2, $3, $4, $5) ON CONFLICT DO NOTHING")
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
        let members = members.to_vec();
        run_serializable!(self.pool, |tx| {
            sqlx::query("DELETE FROM room_members WHERE room_id = $1")
                .bind(room_id)
                .execute(&mut *tx)
                .await?;

            for member in &members {
                sqlx::query("INSERT INTO room_members (room_id, agent_id, display_name, role_description, display_order) VALUES ($1, $2, $3, $4, $5)")
                    .bind(room_id)
                    .bind(member.agent_id)
                    .bind(member.display_name.as_deref())
                    .bind(&member.role_description)
                    .bind(member.display_order)
                    .execute(&mut *tx)
                    .await?;
            }
            Ok(())
        })
    }

    // --- Room sessions ---

    async fn create_room_session(&self, room_id: Uuid) -> Result<RoomSessionRow> {
        let row = sqlx::query_as::<_, RoomSessionRow>(
            "INSERT INTO room_sessions (room_id) VALUES ($1) RETURNING *",
        )
        .bind(room_id)
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
        let row: (i32,) = sqlx::query_as("UPDATE room_sessions SET current_turn = current_turn + 1 WHERE id = $1 RETURNING current_turn")
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

    // --- Room Execution Outputs (Phase 3) ---

    async fn save_room_execution_output(
        &self,
        input: SaveRoomExecutionOutputInput,
    ) -> Result<RoomExecutionOutputRow> {
        let row = sqlx::query_as::<_, RoomExecutionOutputRow>(
            "INSERT INTO room_execution_outputs
             (room_session_id, agent_execution_id, agent_id, speaker_order, turn_number, output_name, structured_output, raw_output, schema_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
             RETURNING id, room_session_id, agent_execution_id, agent_id, speaker_order, turn_number, output_name, structured_output, raw_output, schema_id, created_at"
        )
        .bind(input.room_session_id)
        .bind(input.agent_execution_id)
        .bind(input.agent_id)
        .bind(input.speaker_order)
        .bind(input.turn_number)
        .bind(&input.output_name)
        .bind(&input.structured_output)
        .bind(&input.raw_output)
        .bind(input.schema_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn get_room_execution_outputs(
        &self,
        room_session_id: Uuid,
        turn_number: Option<i32>,
    ) -> Result<Vec<RoomExecutionOutputRow>> {
        let rows = if let Some(turn) = turn_number {
            sqlx::query_as::<_, RoomExecutionOutputRow>(
                "SELECT id, room_session_id, agent_execution_id, agent_id, speaker_order, turn_number, output_name, structured_output, raw_output, schema_id, created_at
                 FROM room_execution_outputs
                 WHERE room_session_id = $1 AND turn_number = $2
                 ORDER BY speaker_order"
            )
            .bind(room_session_id)
            .bind(turn)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, RoomExecutionOutputRow>(
                "SELECT id, room_session_id, agent_execution_id, agent_id, speaker_order, turn_number, output_name, structured_output, raw_output, schema_id, created_at
                 FROM room_execution_outputs
                 WHERE room_session_id = $1
                 ORDER BY turn_number, speaker_order"
            )
            .bind(room_session_id)
            .fetch_all(&self.pool)
            .await?
        };
        Ok(rows)
    }

    async fn get_room_outputs_by_schema(
        &self,
        room_session_id: Uuid,
        schema_id: Uuid,
    ) -> Result<Vec<RoomExecutionOutputRow>> {
        let rows = sqlx::query_as::<_, RoomExecutionOutputRow>(
            "SELECT id, room_session_id, agent_execution_id, agent_id, speaker_order, turn_number, output_name, structured_output, raw_output, schema_id, created_at
             FROM room_execution_outputs
             WHERE room_session_id = $1 AND schema_id = $2
             ORDER BY turn_number, speaker_order"
        )
        .bind(room_session_id)
        .bind(schema_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }
}

// ============================================================================
// WorkflowCollectionRepo implementation
// ============================================================================

#[async_trait]
impl WorkflowCollectionRepo for PgRepo {
    // --- Collections ---

    async fn create_collection(
        &self,
        user_id: Uuid,
        name: String,
        description: Option<String>,
        execution_mode: String,
    ) -> Result<WorkflowCollectionRow> {
        let row = sqlx::query_as::<_, WorkflowCollectionRow>(
            "INSERT INTO workflow_collections (user_id, name, description, execution_mode) \
             VALUES ($1, $2, $3, $4) \
             RETURNING *",
        )
        .bind(user_id)
        .bind(name)
        .bind(description)
        .bind(execution_mode)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn get_collection(&self, id: Uuid) -> Result<Option<WorkflowCollectionRow>> {
        let row = sqlx::query_as::<_, WorkflowCollectionRow>(
            "SELECT * FROM workflow_collections WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    async fn list_collections(&self, user_id: Uuid) -> Result<Vec<WorkflowCollectionRow>> {
        let rows = sqlx::query_as::<_, WorkflowCollectionRow>(
            "SELECT * FROM workflow_collections WHERE user_id = $1 ORDER BY created_at DESC",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn update_collection(
        &self,
        id: Uuid,
        name: Option<String>,
        description: Option<String>,
        execution_mode: Option<String>,
    ) -> Result<WorkflowCollectionRow> {
        let row = sqlx::query_as::<_, WorkflowCollectionRow>(
            "UPDATE workflow_collections \
             SET name = COALESCE($2, name), \
                 description = COALESCE($3, description), \
                 execution_mode = COALESCE($4, execution_mode), \
                 updated_at = NOW() \
             WHERE id = $1 \
             RETURNING *",
        )
        .bind(id)
        .bind(name)
        .bind(description)
        .bind(execution_mode)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn delete_collection(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM workflow_collections WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // --- Collection Workflows (Membership) ---

    async fn add_collection_workflow(
        &self,
        collection_id: Uuid,
        workflow_id: Uuid,
        display_order: i32,
        execution_mode: Option<String>,
    ) -> Result<CollectionWorkflowRow> {
        let row = sqlx::query_as::<_, CollectionWorkflowRow>(
            "INSERT INTO collection_workflows (collection_id, workflow_id, display_order, execution_mode) \
             VALUES ($1, $2, $3, $4) \
             RETURNING *",
        )
        .bind(collection_id)
        .bind(workflow_id)
        .bind(display_order)
        .bind(execution_mode)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn list_collection_workflows(
        &self,
        collection_id: Uuid,
    ) -> Result<Vec<CollectionWorkflowRow>> {
        let rows = sqlx::query_as::<_, CollectionWorkflowRow>(
            "SELECT * FROM collection_workflows WHERE collection_id = $1 ORDER BY display_order",
        )
        .bind(collection_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn remove_collection_workflow(
        &self,
        collection_id: Uuid,
        workflow_id: Uuid,
    ) -> Result<()> {
        sqlx::query(
            "DELETE FROM collection_workflows WHERE collection_id = $1 AND workflow_id = $2",
        )
        .bind(collection_id)
        .bind(workflow_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn update_collection_workflow(
        &self,
        collection_id: Uuid,
        workflow_id: Uuid,
        display_order: Option<i32>,
        execution_mode: Option<String>,
    ) -> Result<CollectionWorkflowRow> {
        let row = sqlx::query_as::<_, CollectionWorkflowRow>(
            "UPDATE collection_workflows \
             SET display_order = COALESCE($3, display_order), \
                 execution_mode = COALESCE($4, execution_mode) \
             WHERE collection_id = $1 AND workflow_id = $2 \
             RETURNING *",
        )
        .bind(collection_id)
        .bind(workflow_id)
        .bind(display_order)
        .bind(execution_mode)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    // --- Collection Workflow Edges (DAG edges between workflows) ---

    async fn set_collection_edges(
        &self,
        collection_id: Uuid,
        edges: Vec<CollectionWorkflowEdgeRow>,
    ) -> Result<()> {
        run_serializable!(self.pool, |tx| {
            sqlx::query("DELETE FROM collection_workflow_edges WHERE collection_id = $1")
                .bind(collection_id)
                .execute(&mut *tx)
                .await?;

            for edge in &edges {
                sqlx::query("INSERT INTO collection_workflow_edges (from_workflow_id, to_workflow_id, collection_id) VALUES ($1, $2, $3)")
                    .bind(edge.from_workflow_id)
                    .bind(edge.to_workflow_id)
                    .bind(collection_id)
                    .execute(&mut *tx)
                    .await?;
            }
            Ok(())
        })
    }

    async fn list_collection_edges(
        &self,
        collection_id: Uuid,
    ) -> Result<Vec<CollectionWorkflowEdgeRow>> {
        let rows = sqlx::query_as::<_, CollectionWorkflowEdgeRow>(
            "SELECT * FROM collection_workflow_edges WHERE collection_id = $1",
        )
        .bind(collection_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn add_collection_edge(
        &self,
        collection_id: Uuid,
        from_workflow_id: Uuid,
        to_workflow_id: Uuid,
    ) -> Result<()> {
        sqlx::query("INSERT INTO collection_workflow_edges (from_workflow_id, to_workflow_id, collection_id) VALUES ($1, $2, $3)")
            .bind(from_workflow_id)
            .bind(to_workflow_id)
            .bind(collection_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn remove_collection_edge(
        &self,
        collection_id: Uuid,
        from_workflow_id: Uuid,
        to_workflow_id: Uuid,
    ) -> Result<()> {
        sqlx::query("DELETE FROM collection_workflow_edges WHERE collection_id = $1 AND from_workflow_id = $2 AND to_workflow_id = $3")
            .bind(collection_id)
            .bind(from_workflow_id)
            .bind(to_workflow_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // --- Collection Runs (Execution Tracking) ---

    async fn create_collection_run(
        &self,
        collection_id: Uuid,
        user_id: Uuid,
    ) -> Result<CollectionRunRow> {
        let row = sqlx::query_as::<_, CollectionRunRow>(
            "INSERT INTO collection_runs (collection_id, user_id, status) \
             VALUES ($1, $2, 'running') \
             RETURNING *",
        )
        .bind(collection_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn get_collection_run(&self, id: Uuid) -> Result<Option<CollectionRunRow>> {
        let row =
            sqlx::query_as::<_, CollectionRunRow>("SELECT * FROM collection_runs WHERE id = $1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;
        Ok(row)
    }

    async fn list_collection_runs(&self, collection_id: Uuid) -> Result<Vec<CollectionRunRow>> {
        let rows = sqlx::query_as::<_, CollectionRunRow>(
            "SELECT * FROM collection_runs WHERE collection_id = $1 ORDER BY started_at DESC",
        )
        .bind(collection_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn update_collection_run_status(
        &self,
        id: Uuid,
        status: &str,
        error: Option<String>,
    ) -> Result<CollectionRunRow> {
        let row = sqlx::query_as::<_, CollectionRunRow>(
            "UPDATE collection_runs \
             SET status = $2, \
                 error = $3, \
                 completed_at = CASE WHEN $2 IN ('completed', 'failed', 'cancelled') THEN NOW() ELSE completed_at END \
             WHERE id = $1 \
             RETURNING *",
        )
        .bind(id)
        .bind(status)
        .bind(error)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    // --- Workflow Executions (Workflow-level execution within a collection run) ---

    async fn create_workflow_execution(
        &self,
        collection_run_id: Uuid,
        workflow_id: Uuid,
        user_id: Uuid,
    ) -> Result<WorkflowExecutionRow> {
        let id = Uuid::new_v4();
        let row = sqlx::query_as::<_, WorkflowExecutionRow>(
            "INSERT INTO workflow_executions \
             (id, collection_run_id, workflow_id, user_id, status, root_execution_id, depth) \
             VALUES ($1, $2, $3, $4, 'pending', $1, 0) \
             RETURNING *",
        )
        .bind(id)
        .bind(collection_run_id)
        .bind(workflow_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn get_workflow_execution(&self, id: Uuid) -> Result<Option<WorkflowExecutionRow>> {
        let row = sqlx::query_as::<_, WorkflowExecutionRow>(
            "SELECT * FROM workflow_executions WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    async fn list_workflow_executions(
        &self,
        collection_run_id: Uuid,
    ) -> Result<Vec<WorkflowExecutionRow>> {
        let rows = sqlx::query_as::<_, WorkflowExecutionRow>(
            "SELECT * FROM workflow_executions WHERE collection_run_id = $1 ORDER BY started_at",
        )
        .bind(collection_run_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn update_workflow_execution_status(
        &self,
        id: Uuid,
        status: &str,
        outputs: Option<serde_json::Value>,
        error: Option<String>,
    ) -> Result<WorkflowExecutionRow> {
        let row = sqlx::query_as::<_, WorkflowExecutionRow>(
            "UPDATE workflow_executions \
             SET status = $2, \
                 outputs = COALESCE($3, outputs), \
                 error = $4, \
                 started_at = COALESCE(started_at, CASE WHEN $2 = 'running' THEN NOW() ELSE started_at END), \
                 completed_at = CASE WHEN $2 IN ('completed', 'failed') THEN NOW() ELSE completed_at END \
             WHERE id = $1 \
             RETURNING *",
        )
        .bind(id)
        .bind(status)
        .bind(outputs)
        .bind(error)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn list_workflow_executions_by_workflow(
        &self,
        workflow_id: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<WorkflowExecutionRow>> {
        let rows = sqlx::query_as::<_, WorkflowExecutionRow>(
            "SELECT * FROM workflow_executions \
             WHERE workflow_id = $1 AND user_id = $2 \
             ORDER BY started_at DESC NULLS LAST",
        )
        .bind(workflow_id)
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn create_standalone_workflow_execution(
        &self,
        workflow_id: Uuid,
        user_id: Uuid,
    ) -> Result<WorkflowExecutionRow> {
        let id = Uuid::new_v4();
        let row = sqlx::query_as::<_, WorkflowExecutionRow>(
            "INSERT INTO workflow_executions \
             (id, workflow_id, user_id, status, root_execution_id, depth) \
             VALUES ($1, $2, $3, 'pending', $1, 0) \
             RETURNING *",
        )
        .bind(id)
        .bind(workflow_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn create_child_workflow_execution(
        &self,
        parent_execution_id: Uuid,
        workflow_id: Uuid,
        user_id: Uuid,
        template_id: Uuid,
    ) -> Result<WorkflowExecutionRow> {
        let row = sqlx::query_as::<_, WorkflowExecutionRow>(
            "INSERT INTO workflow_executions \
             (parent_execution_id, workflow_id, user_id, template_id, status, execution_mode, \
              root_execution_id, depth) \
             VALUES ($1, $2, $3, $4, 'pending', 'sub_workflow', \
                     (SELECT COALESCE(root_execution_id, id) FROM workflow_executions WHERE id = $1), \
                     (SELECT depth + 1 FROM workflow_executions WHERE id = $1)) \
             RETURNING *",
        )
        .bind(parent_execution_id)
        .bind(workflow_id)
        .bind(user_id)
        .bind(template_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn list_child_executions(
        &self,
        parent_execution_id: Uuid,
    ) -> Result<Vec<WorkflowExecutionRow>> {
        let rows = sqlx::query_as::<_, WorkflowExecutionRow>(
            "SELECT * FROM workflow_executions \
             WHERE parent_execution_id = $1 \
             ORDER BY started_at",
        )
        .bind(parent_execution_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn list_execution_tree(&self, root_id: Uuid) -> Result<Vec<WorkflowExecutionRow>> {
        let rows = sqlx::query_as::<_, WorkflowExecutionRow>(
            "SELECT * FROM workflow_executions \
             WHERE root_execution_id = $1 \
             ORDER BY depth, started_at",
        )
        .bind(root_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn get_or_create_workshop(
        &self,
        workflow_id: Uuid,
        user_id: Uuid,
    ) -> Result<WorkflowExecutionRow> {
        // Try to find existing workshop for this workflow
        let existing = sqlx::query_as::<_, WorkflowExecutionRow>(
            "SELECT * FROM workflow_executions \
             WHERE workflow_id = $1 AND execution_mode = 'workshop' \
             LIMIT 1",
        )
        .bind(workflow_id)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = existing {
            return Ok(row);
        }

        // Create new workshop (unique index prevents duplicates under races)
        let id = Uuid::new_v4();
        let row = sqlx::query_as::<_, WorkflowExecutionRow>(
            "INSERT INTO workflow_executions \
             (id, workflow_id, user_id, status, execution_mode, root_execution_id, depth) \
             VALUES ($1, $2, $3, 'workshop', 'workshop', $1, 0) \
             ON CONFLICT (workflow_id) WHERE execution_mode = 'workshop' \
             DO UPDATE SET workflow_id = EXCLUDED.workflow_id \
             RETURNING *",
        )
        .bind(id)
        .bind(workflow_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }
}

// ============================================================================
// WorkflowStepAgentRepo implementation
// ============================================================================

#[async_trait]
impl WorkflowStepAgentRepo for PgRepo {
    async fn add_step_agent(
        &self,
        step_id: Uuid,
        agent_id: Uuid,
        execution_strategy: String,
        agent_order: i32,
    ) -> Result<WorkflowStepAgentRow> {
        let row = sqlx::query_as::<_, WorkflowStepAgentRow>(
            "INSERT INTO workflow_step_agents (step_id, agent_id, execution_strategy, agent_order) \
             VALUES ($1, $2, $3, $4) \
             RETURNING *",
        )
        .bind(step_id)
        .bind(agent_id)
        .bind(execution_strategy)
        .bind(agent_order)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn list_step_agents(&self, step_id: Uuid) -> Result<Vec<WorkflowStepAgentRow>> {
        let rows = sqlx::query_as::<_, WorkflowStepAgentRow>(
            "SELECT * FROM workflow_step_agents WHERE step_id = $1 ORDER BY agent_order",
        )
        .bind(step_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn remove_step_agent(&self, step_id: Uuid, agent_id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM workflow_step_agents WHERE step_id = $1 AND agent_id = $2")
            .bind(step_id)
            .bind(agent_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn update_step_agent(
        &self,
        step_id: Uuid,
        agent_id: Uuid,
        execution_strategy: Option<String>,
        agent_order: Option<i32>,
    ) -> Result<WorkflowStepAgentRow> {
        let row = sqlx::query_as::<_, WorkflowStepAgentRow>(
            "UPDATE workflow_step_agents \
             SET execution_strategy = COALESCE($3, execution_strategy), \
                 agent_order = COALESCE($4, agent_order) \
             WHERE step_id = $1 AND agent_id = $2 \
             RETURNING *",
        )
        .bind(step_id)
        .bind(agent_id)
        .bind(execution_strategy)
        .bind(agent_order)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn set_step_agents(
        &self,
        step_id: Uuid,
        agents: Vec<WorkflowStepAgentRow>,
    ) -> Result<()> {
        run_serializable!(self.pool, |tx| {
            sqlx::query("DELETE FROM workflow_step_agents WHERE step_id = $1")
                .bind(step_id)
                .execute(&mut *tx)
                .await?;

            for agent in &agents {
                sqlx::query("INSERT INTO workflow_step_agents (step_id, agent_id, execution_strategy, agent_order) VALUES ($1, $2, $3, $4)")
                    .bind(step_id)
                    .bind(agent.agent_id)
                    .bind(&agent.execution_strategy)
                    .bind(agent.agent_order)
                    .execute(&mut *tx)
                    .await?;
            }
            Ok(())
        })
    }
}

// ============================================================================
// Tool Capability Repository Implementation (Phase 3)
// ============================================================================

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

// ============================================================================
// System Configuration Repository Implementation (Phase 3)
// ============================================================================

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

// ============================================================================
// Protocol Repository
// ============================================================================

#[async_trait]
impl ProtocolRepo for PgRepo {
    async fn create_protocol(&self, input: CreateProtocolInput) -> Result<ProtocolRow> {
        let row = sqlx::query_as::<_, ProtocolRow>(
            "INSERT INTO protocols (name, description, protocol_type, config, agent_id, output_schema_id, prompt_template_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             RETURNING id, name, description, protocol_type, config, version, created_at, updated_at, agent_id, output_schema_id, prompt_template_id",
        )
        .bind(&input.name)
        .bind(&input.description)
        .bind(&input.protocol_type)
        .bind(&input.config)
        .bind(input.agent_id)
        .bind(input.output_schema_id)
        .bind(input.prompt_template_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn get_protocol(&self, id: Uuid) -> Result<Option<ProtocolRow>> {
        let row = sqlx::query_as::<_, ProtocolRow>(
            "SELECT id, name, description, protocol_type, config, version, created_at, updated_at, agent_id, output_schema_id, prompt_template_id
             FROM protocols WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    async fn get_protocol_by_type(&self, protocol_type: &str) -> Result<Option<ProtocolRow>> {
        let row = sqlx::query_as::<_, ProtocolRow>(
            "SELECT id, name, description, protocol_type, config, version, created_at, updated_at, agent_id, output_schema_id, prompt_template_id
             FROM protocols WHERE protocol_type = $1 LIMIT 1",
        )
        .bind(protocol_type)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    async fn list_protocols(&self) -> Result<Vec<ProtocolRow>> {
        let rows = sqlx::query_as::<_, ProtocolRow>(
            "SELECT id, name, description, protocol_type, config, version, created_at, updated_at, agent_id, output_schema_id, prompt_template_id
             FROM protocols ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn seed_builtin_protocols(&self) -> Result<()> {
        for p in crate::server::hub::protocols::builtins::builtin_protocol_definitions() {
            // Upsert protocol row. Agent/schema/template FKs are NULL for builtins —
            // the compilers generate schemas and prompts dynamically from port config.
            sqlx::query(
                r#"
                INSERT INTO protocols (id, name, description, protocol_type, config)
                VALUES ($1, $2, $3, $4, $5)
                ON CONFLICT (name) DO UPDATE SET
                    description = EXCLUDED.description,
                    protocol_type = EXCLUDED.protocol_type,
                    config = EXCLUDED.config,
                    version = protocols.version + 1,
                    updated_at = now()
                "#,
            )
            .bind(p.id)
            .bind(&p.name)
            .bind(&p.description)
            .bind(&p.protocol_type)
            .bind(&p.config)
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    async fn update_protocol(&self, input: UpdateProtocolInput) -> Result<ProtocolRow> {
        let row = sqlx::query_as::<_, ProtocolRow>(
            "UPDATE protocols SET
                name = COALESCE($2, name),
                description = COALESCE($3, description),
                config = COALESCE($4, config),
                agent_id = COALESCE($5, agent_id),
                output_schema_id = COALESCE($6, output_schema_id),
                prompt_template_id = COALESCE($7, prompt_template_id),
                version = version + 1,
                updated_at = now()
             WHERE id = $1
             RETURNING id, name, description, protocol_type, config, version, created_at, updated_at, agent_id, output_schema_id, prompt_template_id",
        )
        .bind(input.id)
        .bind(input.name)
        .bind(input.description)
        .bind(input.config)
        .bind(input.agent_id)
        .bind(input.output_schema_id)
        .bind(input.prompt_template_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn delete_protocol(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM protocols WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // --- Protocol Ports ---

    async fn list_protocol_ports(&self, protocol_id: Uuid) -> Result<Vec<ProtocolPortRow>> {
        let rows = sqlx::query_as::<_, ProtocolPortRow>(
            "SELECT id, protocol_id, port_name, description, agent_id, display_order
             FROM protocol_ports WHERE protocol_id = $1 ORDER BY display_order",
        )
        .bind(protocol_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn create_protocol_port(
        &self,
        protocol_id: Uuid,
        port_name: String,
        description: String,
        agent_id: Uuid,
        display_order: i32,
    ) -> Result<ProtocolPortRow> {
        let row = sqlx::query_as::<_, ProtocolPortRow>(
            "INSERT INTO protocol_ports (protocol_id, port_name, description, agent_id, display_order)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING id, protocol_id, port_name, description, agent_id, display_order",
        )
        .bind(protocol_id)
        .bind(&port_name)
        .bind(&description)
        .bind(agent_id)
        .bind(display_order)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn update_protocol_port(
        &self,
        id: Uuid,
        port_name: Option<String>,
        description: Option<String>,
        agent_id: Option<Uuid>,
        display_order: Option<i32>,
    ) -> Result<ProtocolPortRow> {
        let row = sqlx::query_as::<_, ProtocolPortRow>(
            "UPDATE protocol_ports SET
                port_name = COALESCE($2, port_name),
                description = COALESCE($3, description),
                agent_id = COALESCE($4, agent_id),
                display_order = COALESCE($5, display_order)
             WHERE id = $1
             RETURNING id, protocol_id, port_name, description, agent_id, display_order",
        )
        .bind(id)
        .bind(port_name)
        .bind(description)
        .bind(agent_id)
        .bind(display_order)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn delete_protocol_port(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM protocol_ports WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // --- Workflow Step Protocol Linkage ---

    async fn get_step_protocol(
        &self,
        workflow_step_id: Uuid,
    ) -> Result<Option<WorkflowStepProtocolRow>> {
        let row = sqlx::query_as::<_, WorkflowStepProtocolRow>(
            "SELECT id, workflow_step_id, protocol_id, applied_expansion, created_at
             FROM workflow_step_protocols WHERE workflow_step_id = $1",
        )
        .bind(workflow_step_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    async fn create_step_protocol(
        &self,
        workflow_step_id: Uuid,
        protocol_id: Uuid,
        applied_expansion: serde_json::Value,
    ) -> Result<WorkflowStepProtocolRow> {
        let row = sqlx::query_as::<_, WorkflowStepProtocolRow>(
            "INSERT INTO workflow_step_protocols (workflow_step_id, protocol_id, applied_expansion)
             VALUES ($1, $2, $3)
             RETURNING id, workflow_step_id, protocol_id, applied_expansion, created_at",
        )
        .bind(workflow_step_id)
        .bind(protocol_id)
        .bind(&applied_expansion)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn delete_step_protocol(&self, workflow_step_id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM workflow_step_protocols WHERE workflow_step_id = $1")
            .bind(workflow_step_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // --- Protocol-scoped Document Definitions ---

    async fn list_protocol_document_defs(
        &self,
        protocol_id: Uuid,
    ) -> Result<Vec<ProtocolDocumentDefRow>> {
        let rows = sqlx::query_as::<_, ProtocolDocumentDefRow>(
            "SELECT id, step_id, name, description, target_length, display_order, created_at, protocol_id, document_id, agent_roster_entry_id \
             FROM protocol_document_defs WHERE protocol_id = $1 ORDER BY display_order, created_at",
        )
        .bind(protocol_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn create_protocol_document_def(
        &self,
        def: ProtocolDocumentDefRow,
    ) -> Result<ProtocolDocumentDefRow> {
        let row = sqlx::query_as::<_, ProtocolDocumentDefRow>(
            "INSERT INTO protocol_document_defs (id, step_id, name, description, target_length, display_order, protocol_id, document_id, agent_roster_entry_id) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) \
             RETURNING id, step_id, name, description, target_length, display_order, created_at, protocol_id, document_id, agent_roster_entry_id",
        )
        .bind(def.id)
        .bind(def.step_id)
        .bind(&def.name)
        .bind(&def.description)
        .bind(def.target_length)
        .bind(def.display_order)
        .bind(def.protocol_id)
        .bind(def.document_id)
        .bind(def.agent_roster_entry_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn update_protocol_document_def(
        &self,
        id: Uuid,
        name: String,
        description: String,
        target_length: i32,
    ) -> Result<ProtocolDocumentDefRow> {
        let row = sqlx::query_as::<_, ProtocolDocumentDefRow>(
            "UPDATE protocol_document_defs SET name = $2, description = $3, target_length = $4 \
             WHERE id = $1 \
             RETURNING id, step_id, name, description, target_length, display_order, created_at, protocol_id, document_id, agent_roster_entry_id",
        )
        .bind(id)
        .bind(&name)
        .bind(&description)
        .bind(target_length)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn delete_protocol_document_def(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM protocol_document_defs WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // --- Protocol Executions ---

    async fn create_protocol_execution(
        &self,
        row: ProtocolExecutionRow,
    ) -> Result<ProtocolExecutionRow> {
        let result = sqlx::query_as::<_, ProtocolExecutionRow>(
            "INSERT INTO protocol_executions \
             (id, protocol_step_id, workflow_run_id, phase, document_def_id, agent_id, \
              input_prompt, output_content, status, error_message, \
              tokens_in, tokens_out, cost_usd, model, capabilities_used, created_at, completed_at, \
              agent_name, archetype, designer_run_id) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, \
                     $18, $19, $20) \
             RETURNING *",
        )
        .bind(row.id)
        .bind(row.protocol_step_id)
        .bind(row.workflow_run_id)
        .bind(&row.phase)
        .bind(row.document_def_id)
        .bind(row.agent_id)
        .bind(&row.input_prompt)
        .bind(&row.output_content)
        .bind(&row.status)
        .bind(&row.error_message)
        .bind(row.tokens_in)
        .bind(row.tokens_out)
        .bind(row.cost_usd)
        .bind(&row.model)
        .bind(&row.capabilities_used)
        .bind(row.created_at)
        .bind(row.completed_at)
        .bind(&row.agent_name)
        .bind(&row.archetype)
        .bind(row.designer_run_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(result)
    }

    async fn update_protocol_execution_status(
        &self,
        input: UpdateProtocolExecutionStatusInput,
    ) -> Result<ProtocolExecutionRow> {
        let row = sqlx::query_as::<_, ProtocolExecutionRow>(
            "UPDATE protocol_executions \
             SET status = $2, output_content = $3, error_message = $4, \
                 tokens_in = $5, tokens_out = $6, cost_usd = $7, model = $8, \
                 completed_at = CASE WHEN $2 IN ('complete', 'failed') THEN now() ELSE completed_at END \
             WHERE id = $1 \
             RETURNING *",
        )
        .bind(input.id)
        .bind(&input.status)
        .bind(&input.output_content)
        .bind(&input.error_message)
        .bind(input.tokens_in)
        .bind(input.tokens_out)
        .bind(input.cost_usd)
        .bind(&input.model)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn list_protocol_executions_by_step(
        &self,
        step_id: Uuid,
    ) -> Result<Vec<ProtocolExecutionRow>> {
        let rows = sqlx::query_as::<_, ProtocolExecutionRow>(
            "SELECT * FROM protocol_executions WHERE protocol_step_id = $1 ORDER BY created_at",
        )
        .bind(step_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn list_protocol_executions_by_run(
        &self,
        run_id: Uuid,
    ) -> Result<Vec<ProtocolExecutionRow>> {
        let rows = sqlx::query_as::<_, ProtocolExecutionRow>(
            "SELECT * FROM protocol_executions WHERE workflow_run_id = $1 ORDER BY created_at",
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }
}

// ============================================================================
// ContentVersionRepo
// ============================================================================

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
             WHERE rs.run_id = $1 AND rs.content_type = 'envelope' AND rs.role = 'output'",
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }
}

#[cfg(test)]
mod tests;
