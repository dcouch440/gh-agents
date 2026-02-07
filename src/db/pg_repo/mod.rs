//! PostgreSQL implementation of repository traits.

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::traits::{
    AgentExecutionRepo, ContextStoreRepo, DocumentRepo, MergeQueueRepo, ModelSpendRow,
    OutputSchemaRepo, PromptTemplateRepo, ResultRepo, RoomMemberInput, RoomRepo, RouterRequestRepo,
    ServerRepo, SystemConfigRepo, TokenLedgerRepo, ToolCapabilityRepo, ToolRouterRepo, UserRepo,
    WorkflowCollectionRepo, WorkflowRepo, WorkflowStepAgentRepo,
};
use crate::db::{
    AgentExecutionRow, AgentModeRow, AgentRow, ChatMessageRow, CollectionRunRow,
    CollectionWorkflowEdgeRow, CollectionWorkflowRow, ContextStoreRow, DocumentRow,
    DocumentSearchResult, ExecutionMessageRow, OutputSchemaRow, PromptTemplateRow, ResultRow,
    RoomExecutionOutputRow, RoomMemberRow, RoomRow, RoomSessionRow, RoomTranscriptEntry,
    RouterRequestRow, SessionRow, StepDocumentRow, StepInputRow, StepOutputRow, StepRoutingRuleRow,
    SystemConfigRow, TokenLedgerRow, ToolCapabilityRow, ToolRouterModeRow, ToolRouterRow, ToolRow,
    WorkflowCollectionRow, WorkflowExecutionRow, WorkflowRow, WorkflowStepAgentRow,
    WorkflowStepEdgeRow, WorkflowStepRow,
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
    async fn insert_queue_entry(
        &self,
        id: Uuid,
        owner: String,
        repo: String,
        pr_number: u32,
        position: u32,
        now: DateTime<Utc>,
    ) -> Result<(), MergeQueueError> {
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
impl ServerRepo for PgRepo {
    async fn health_check(&self) -> bool {
        sqlx::query("SELECT 1").fetch_one(&self.pool).await.is_ok()
    }

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
            "SELECT id, name, system_prompt, persona_style, model_provider, model_id, model_max_tokens, model_temperature, status, router_mode, router_id, output_schema_id, version FROM agents WHERE user_id = $1",
        )
        .bind(user_id.0)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(agent_row_from_pg).collect())
    }

    async fn get_persisted_agent(&self, agent_id: Uuid) -> Result<Option<AgentRow>> {
        let row = sqlx::query_as::<_, PgAgentRow>(
            "SELECT id, name, system_prompt, persona_style, model_provider, model_id, model_max_tokens, model_temperature, status, router_mode, router_id, output_schema_id, version FROM agents WHERE id = $1",
        )
        .bind(agent_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(agent_row_from_pg))
    }

    async fn upsert_agent(&self, user_id: UserId, agent: AgentRow) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO agents (id, user_id, name, system_prompt, persona_style, model_provider, model_id, model_max_tokens, model_temperature, status, router_mode, output_schema_id)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            ON CONFLICT (id) DO UPDATE SET
                name = EXCLUDED.name,
                system_prompt = EXCLUDED.system_prompt,
                persona_style = EXCLUDED.persona_style,
                model_provider = EXCLUDED.model_provider,
                model_id = EXCLUDED.model_id,
                model_max_tokens = EXCLUDED.model_max_tokens,
                model_temperature = EXCLUDED.model_temperature,
                status = EXCLUDED.status,
                router_mode = EXCLUDED.router_mode,
                output_schema_id = EXCLUDED.output_schema_id,
                version = agents.version + 1
        "#,
        )
        .bind(agent.id)
        .bind(user_id.0)
        .bind(&agent.name)
        .bind(&agent.system_prompt)
        .bind(&agent.persona_style)
        .bind(&agent.model_provider)
        .bind(&agent.model_id)
        .bind(agent.model_max_tokens)
        .bind(agent.model_temperature)
        .bind(&agent.status)
        .bind(agent.router_mode)
        .bind(agent.output_schema_id)
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

    // --- Tool persistence ---

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
        let mut tx = self.pool.begin().await?;

        sqlx::query("DELETE FROM agent_tools WHERE agent_id = $1")
            .bind(agent_id)
            .execute(&mut *tx)
            .await?;

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

        sqlx::query("DELETE FROM agent_context WHERE agent_id = $1")
            .bind(agent_id)
            .execute(&mut *tx)
            .await?;

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

    // --- Session management ---

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

    async fn link_session_agent(&self, session_id: Uuid, agent_id: Uuid) -> Result<()> {
        crate::db::link_session_agent(&self.pool, session_id, agent_id).await
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
// User Repository
// ============================================================================

#[derive(sqlx::FromRow)]
struct PgAgentRow {
    id: Uuid,
    name: String,
    system_prompt: String,
    persona_style: Option<String>,
    model_provider: String,
    model_id: String,
    model_max_tokens: i32,
    model_temperature: f32,
    status: Option<String>,
    router_mode: Option<bool>,
    router_id: Option<Uuid>,
    output_schema_id: Option<Uuid>,
    version: i32,
}

fn agent_row_from_pg(r: PgAgentRow) -> AgentRow {
    AgentRow {
        id: r.id,
        tier: None,
        name: r.name,
        system_prompt: r.system_prompt,
        persona_style: r.persona_style,
        model_provider: r.model_provider,
        model_id: r.model_id,
        model_max_tokens: r.model_max_tokens,
        model_temperature: r.model_temperature,
        status: r.status,
        router_mode: r.router_mode,
        router_id: r.router_id,
        output_schema_id: r.output_schema_id,
        version: r.version,
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
    async fn create_document(
        &self,
        user_id: Uuid,
        session_id: Option<Uuid>,
        title: String,
        content: String,
        doc_type: String,
        ref_tag: String,
        tags: Vec<String>,
    ) -> Result<DocumentRow> {
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

    async fn search_routing_documents(
        &self,
        user_id: Uuid,
        query: &str,
        limit: i64,
    ) -> Result<Vec<DocumentSearchResult>> {
        let rows: Vec<DocumentSearchResult> = sqlx::query_as(
            r#"
            SELECT id, title, summary, ref_tag,
                   ts_headline('english', content, plainto_tsquery('english', $2),
                       'StartSel=**, StopSel=**, MaxWords=35, MinWords=15') AS snippet
            FROM documents
            WHERE user_id = $1
              AND title LIKE 'routing:%'
              AND to_tsvector('english', title || ' ' || content) @@ plainto_tsquery('english', $2)
            ORDER BY ts_rank(to_tsvector('english', title || ' ' || content), plainto_tsquery('english', $2)) DESC
            LIMIT $3
            "#,
        )
        .bind(user_id)
        .bind(query)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }
}

#[async_trait]
impl OutputSchemaRepo for PgRepo {
    async fn create_output_schema(
        &self,
        user_id: Uuid,
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
        let rows: Vec<OutputSchemaRow> = sqlx::query_as("SELECT id, user_id, name, schema, created_at, version FROM output_schemas WHERE user_id = $1 ORDER BY created_at DESC")
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
        user_id: Uuid,
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
        let rows: Vec<PromptTemplateRow> = sqlx::query_as("SELECT id, user_id, name, content, created_at, version FROM prompt_templates WHERE user_id = $1 ORDER BY created_at DESC")
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

    async fn create_workflow(
        &self,
        user_id: Uuid,
        name: String,
        description: String,
        container_enabled: bool,
        target_repo_url: Option<String>,
        target_branch: Option<String>,
    ) -> Result<WorkflowRow> {
        let row: WorkflowRow = sqlx::query_as(
            "INSERT INTO workflows (user_id, name, description, container_enabled, target_repo_url, target_branch) \
             VALUES ($1, $2, $3, $4, $5, $6) \
             RETURNING id, user_id, name, description, execution_mode, created_at, version, container_enabled, target_repo_url, target_branch",
        )
        .bind(user_id)
        .bind(&name)
        .bind(&description)
        .bind(container_enabled)
        .bind(&target_repo_url)
        .bind(&target_branch)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn get_workflow(&self, id: Uuid) -> Result<Option<WorkflowRow>> {
        let row: Option<WorkflowRow> = sqlx::query_as(
            "SELECT id, user_id, name, description, execution_mode, created_at, version, container_enabled, target_repo_url, target_branch \
             FROM workflows WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    async fn list_workflows(&self, user_id: Uuid) -> Result<Vec<WorkflowRow>> {
        let rows: Vec<WorkflowRow> = sqlx::query_as(
            "SELECT id, user_id, name, description, execution_mode, created_at, version, container_enabled, target_repo_url, target_branch \
             FROM workflows WHERE user_id = $1 ORDER BY created_at DESC",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn update_workflow(
        &self,
        id: Uuid,
        name: Option<String>,
        description: Option<String>,
        container_enabled: Option<bool>,
        target_repo_url: Option<Option<String>>,
        target_branch: Option<Option<String>>,
    ) -> Result<WorkflowRow> {
        // Build dynamic SET clauses for optional container fields
        let row: WorkflowRow = sqlx::query_as(
            "UPDATE workflows SET \
             name = COALESCE($1, name), \
             description = COALESCE($2, description), \
             container_enabled = COALESCE($3, container_enabled), \
             target_repo_url = CASE WHEN $4 THEN $5 ELSE target_repo_url END, \
             target_branch = CASE WHEN $6 THEN $7 ELSE target_branch END, \
             version = version + 1 \
             WHERE id = $8 \
             RETURNING id, user_id, name, description, execution_mode, created_at, version, container_enabled, target_repo_url, target_branch",
        )
        .bind(name)
        .bind(description)
        .bind(container_enabled)
        .bind(target_repo_url.is_some()) // $4: whether to update target_repo_url
        .bind(target_repo_url.unwrap_or(None)) // $5: the value (may be None to clear)
        .bind(target_branch.is_some()) // $6: whether to update target_branch
        .bind(target_branch.unwrap_or(None)) // $7: the value (may be None to clear)
        .bind(id)
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
            INSERT INTO workflow_steps (id, workflow_id, agent_id, execution_mode, for_each_ref, prompt_template_id, prompt_template, output_schema_id, output_variable_name, interactive_agent_id, for_each_label_field, display_order, reasoning_trace)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
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
                reasoning_trace = $11, version = version + 1
            WHERE id = $12
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
                "INSERT INTO workflow_step_edges (from_step_id, to_step_id) VALUES ($1, $2)",
            )
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

    async fn create_step_input(
        &self,
        workflow_step_id: Uuid,
        port_name: &str,
        port_type: &str,
        required: bool,
        default_value: Option<serde_json::Value>,
        description: Option<String>,
        json_schema: Option<serde_json::Value>,
    ) -> Result<StepInputRow> {
        let row = sqlx::query_as::<_, StepInputRow>(
            "INSERT INTO step_inputs (workflow_step_id, port_name, port_type, required, default_value, description, json_schema)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             RETURNING id, workflow_step_id, port_name, port_type, required, default_value, description, json_schema, created_at"
        )
        .bind(workflow_step_id)
        .bind(port_name)
        .bind(port_type)
        .bind(required)
        .bind(default_value)
        .bind(description)
        .bind(json_schema)
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
}

#[async_trait]
impl AgentExecutionRepo for PgRepo {
    async fn create_agent_execution(
        &self,
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
            "INSERT INTO agent_executions (agent_id, workflow_step_id, is_interactive, parent_agent_execution_id, system_prompt_rendered, input, selected_mode_id, room_session_id, speaker_order) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING *",
        )
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

    async fn update_agent_execution_routing(
        &self,
        id: Uuid,
        routing_analysis: &serde_json::Value,
        selected_routing_document_id: Option<Uuid>,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE agent_executions SET routing_analysis = $2, selected_routing_document_id = $3 WHERE id = $1",
        )
        .bind(id)
        .bind(routing_analysis)
        .bind(selected_routing_document_id)
        .execute(&self.pool)
        .await?;
        Ok(())
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
// ToolRouterRepo
// ============================================================================

#[async_trait]
impl ToolRouterRepo for PgRepo {
    async fn list_tool_routers(&self, user_id: Uuid) -> Result<Vec<ToolRouterRow>> {
        let rows: Vec<ToolRouterRow> =
            sqlx::query_as("SELECT id, user_id, name, description, system_prompt, model_id, is_active, created_at, updated_at, parent_router_id, level FROM tool_routers WHERE user_id = $1 ORDER BY created_at DESC")
                .bind(user_id)
                .fetch_all(&self.pool)
                .await?;
        Ok(rows)
    }

    async fn get_tool_router(&self, id: Uuid) -> Result<Option<ToolRouterRow>> {
        let row: Option<ToolRouterRow> = sqlx::query_as("SELECT id, user_id, name, description, system_prompt, model_id, is_active, created_at, updated_at, parent_router_id, level FROM tool_routers WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row)
    }

    async fn create_tool_router(
        &self,
        user_id: Uuid,
        name: &str,
        description: Option<String>,
        system_prompt: &str,
        model_id: &str,
    ) -> Result<ToolRouterRow> {
        let row: ToolRouterRow = sqlx::query_as(
            "INSERT INTO tool_routers (user_id, name, description, system_prompt, model_id) VALUES ($1, $2, $3, $4, $5) RETURNING id, user_id, name, description, system_prompt, model_id, is_active, created_at, updated_at, parent_router_id, level",
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

    async fn update_tool_router(
        &self,
        id: Uuid,
        name: Option<String>,
        description: Option<String>,
        system_prompt: Option<String>,
        model_id: Option<String>,
        is_active: Option<bool>,
    ) -> Result<ToolRouterRow> {
        let row: ToolRouterRow = sqlx::query_as(
            r#"UPDATE tool_routers SET
                name = COALESCE($2, name),
                description = COALESCE($3, description),
                system_prompt = COALESCE($4, system_prompt),
                model_id = COALESCE($5, model_id),
                is_active = COALESCE($6, is_active),
                updated_at = NOW()
            WHERE id = $1
            RETURNING id, user_id, name, description, system_prompt, model_id, is_active, created_at, updated_at, parent_router_id, level"#,
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
        sqlx::query("DELETE FROM tool_routers WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn get_router_tools(&self, router_id: Uuid) -> Result<Vec<ToolRow>> {
        let rows: Vec<ToolRow> = sqlx::query_as(
            "SELECT t.id, t.name, t.display_name, t.description, t.parameters, t.created_at, t.version FROM tools t INNER JOIN tool_router_tools trt ON t.id = trt.tool_id WHERE trt.router_id = $1 ORDER BY t.name",
        )
        .bind(router_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn set_router_tools(&self, router_id: Uuid, tool_ids: &[Uuid]) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        sqlx::query("DELETE FROM tool_router_tools WHERE router_id = $1")
            .bind(router_id)
            .execute(&mut *tx)
            .await?;

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

    // --- Router Modes ---

    async fn list_router_modes(&self, router_id: Uuid) -> Result<Vec<ToolRouterModeRow>> {
        let rows: Vec<ToolRouterModeRow> = sqlx::query_as(
            r#"SELECT id, router_id, mode_key, display_name, description, system_prompt,
               temperature, max_tokens, append_to_agent_system_prompt, append_to_agent_tools,
               display_order, created_at, updated_at
               FROM tool_router_modes
               WHERE router_id = $1
               ORDER BY display_order, mode_key"#,
        )
        .bind(router_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn get_router_mode(&self, id: Uuid) -> Result<Option<ToolRouterModeRow>> {
        let row: Option<ToolRouterModeRow> = sqlx::query_as(
            r#"SELECT id, router_id, mode_key, display_name, description, system_prompt,
               temperature, max_tokens, append_to_agent_system_prompt, append_to_agent_tools,
               display_order, created_at, updated_at
               FROM tool_router_modes
               WHERE id = $1"#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    async fn get_router_mode_by_key(
        &self,
        router_id: Uuid,
        mode_key: &str,
    ) -> Result<Option<ToolRouterModeRow>> {
        let row: Option<ToolRouterModeRow> = sqlx::query_as(
            r#"SELECT id, router_id, mode_key, display_name, description, system_prompt,
               temperature, max_tokens, append_to_agent_system_prompt, append_to_agent_tools,
               display_order, created_at, updated_at
               FROM tool_router_modes
               WHERE router_id = $1 AND mode_key = $2"#,
        )
        .bind(router_id)
        .bind(mode_key)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    async fn create_router_mode(
        &self,
        router_id: Uuid,
        mode_key: &str,
        display_name: &str,
        description: &str,
        system_prompt: &str,
        temperature: f32,
        max_tokens: i32,
        append_to_agent_system_prompt: bool,
        append_to_agent_tools: bool,
        display_order: i32,
    ) -> Result<ToolRouterModeRow> {
        let row: ToolRouterModeRow = sqlx::query_as(
            r#"INSERT INTO tool_router_modes (
                   router_id, mode_key, display_name, description, system_prompt,
                   temperature, max_tokens, append_to_agent_system_prompt,
                   append_to_agent_tools, display_order
               )
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
               RETURNING id, router_id, mode_key, display_name, description, system_prompt,
                         temperature, max_tokens, append_to_agent_system_prompt,
                         append_to_agent_tools, display_order, created_at, updated_at"#,
        )
        .bind(router_id)
        .bind(mode_key)
        .bind(display_name)
        .bind(description)
        .bind(system_prompt)
        .bind(temperature)
        .bind(max_tokens)
        .bind(append_to_agent_system_prompt)
        .bind(append_to_agent_tools)
        .bind(display_order)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn update_router_mode(
        &self,
        id: Uuid,
        mode_key: Option<String>,
        display_name: Option<String>,
        description: Option<String>,
        system_prompt: Option<String>,
        temperature: Option<f32>,
        max_tokens: Option<i32>,
        append_to_agent_system_prompt: Option<bool>,
        append_to_agent_tools: Option<bool>,
        display_order: Option<i32>,
    ) -> Result<ToolRouterModeRow> {
        let row: ToolRouterModeRow = sqlx::query_as(
            r#"UPDATE tool_router_modes SET
                   mode_key = COALESCE($2, mode_key),
                   display_name = COALESCE($3, display_name),
                   description = COALESCE($4, description),
                   system_prompt = COALESCE($5, system_prompt),
                   temperature = COALESCE($6, temperature),
                   max_tokens = COALESCE($7, max_tokens),
                   append_to_agent_system_prompt = COALESCE($8, append_to_agent_system_prompt),
                   append_to_agent_tools = COALESCE($9, append_to_agent_tools),
                   display_order = COALESCE($10, display_order),
                   updated_at = NOW()
               WHERE id = $1
               RETURNING id, router_id, mode_key, display_name, description, system_prompt,
                         temperature, max_tokens, append_to_agent_system_prompt,
                         append_to_agent_tools, display_order, created_at, updated_at"#,
        )
        .bind(id)
        .bind(mode_key)
        .bind(display_name)
        .bind(description)
        .bind(system_prompt)
        .bind(temperature)
        .bind(max_tokens)
        .bind(append_to_agent_system_prompt)
        .bind(append_to_agent_tools)
        .bind(display_order)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    async fn delete_router_mode(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM tool_router_modes WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn get_mode_tools(&self, mode_id: Uuid) -> Result<Vec<ToolRow>> {
        let rows: Vec<ToolRow> = sqlx::query_as(
            r#"SELECT t.id, t.user_id, t.name, t.display_name, t.description,
               t.parameters, t.created_at, t.version
               FROM tools t
               INNER JOIN tool_router_mode_tools trmt ON t.id = trmt.tool_id
               WHERE trmt.mode_id = $1
               ORDER BY t.name"#,
        )
        .bind(mode_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn set_mode_tools(&self, mode_id: Uuid, tool_ids: &[Uuid]) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        // Delete all existing associations
        sqlx::query("DELETE FROM tool_router_mode_tools WHERE mode_id = $1")
            .bind(mode_id)
            .execute(&mut *tx)
            .await?;

        // Insert new associations
        for tool_id in tool_ids {
            sqlx::query("INSERT INTO tool_router_mode_tools (mode_id, tool_id) VALUES ($1, $2)")
                .bind(mode_id)
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
    async fn add_context(
        &self,
        session_id: Uuid,
        source: &str,
        priority: f32,
        content: &str,
        metadata: Option<serde_json::Value>,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<ContextStoreRow> {
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

    async fn get_active_context(
        &self,
        session_id: Uuid,
        limit: u32,
    ) -> Result<Vec<ContextStoreRow>> {
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
    async fn create_router_request(
        &self,
        session_id: Uuid,
        agent_execution_id: Option<Uuid>,
        intent: &str,
        priority: &str,
        callback_hint: Option<String>,
    ) -> Result<RouterRequestRow> {
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

    async fn update_router_request(
        &self,
        id: Uuid,
        routed_tool: Option<String>,
        routed_args: Option<serde_json::Value>,
        is_async: bool,
        passdown: Option<String>,
        chain: Option<serde_json::Value>,
        status: &str,
        result: Option<String>,
    ) -> Result<RouterRequestRow> {
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
        collection_id: Option<Uuid>,
        name: &str,
        gatekeeper_enabled: bool,
        gatekeeper_model_id: &str,
        max_speakers_per_turn: i32,
        max_turns: i32,
        tools_enabled: bool,
    ) -> Result<RoomRow> {
        let row = sqlx::query_as::<_, RoomRow>(
            "INSERT INTO rooms (user_id, collection_id, name, gatekeeper_enabled, gatekeeper_model_id, max_speakers_per_turn, max_turns, tools_enabled) VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING *",
        )
        .bind(user_id)
        .bind(collection_id)
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
        let mut tx = self.pool.begin().await?;

        sqlx::query("DELETE FROM room_members WHERE room_id = $1")
            .bind(room_id)
            .execute(&mut *tx)
            .await?;

        for member in members {
            sqlx::query("INSERT INTO room_members (room_id, agent_id, display_name, role_description, display_order) VALUES ($1, $2, $3, $4, $5)")
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

    async fn create_room_session(
        &self,
        room_id: Uuid,
        run_id: Option<Uuid>,
    ) -> Result<RoomSessionRow> {
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
        room_session_id: Uuid,
        agent_execution_id: Uuid,
        agent_id: Uuid,
        speaker_order: i32,
        turn_number: i32,
        output_name: &str,
        structured_output: &serde_json::Value,
        raw_output: &str,
        schema_id: Option<Uuid>,
    ) -> Result<RoomExecutionOutputRow> {
        let row = sqlx::query_as::<_, RoomExecutionOutputRow>(
            "INSERT INTO room_execution_outputs
             (room_session_id, agent_execution_id, agent_id, speaker_order, turn_number, output_name, structured_output, raw_output, schema_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
             RETURNING id, room_session_id, agent_execution_id, agent_id, speaker_order, turn_number, output_name, structured_output, raw_output, schema_id, created_at"
        )
        .bind(room_session_id)
        .bind(agent_execution_id)
        .bind(agent_id)
        .bind(speaker_order)
        .bind(turn_number)
        .bind(output_name)
        .bind(structured_output)
        .bind(raw_output)
        .bind(schema_id)
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
        let mut tx = self.pool.begin().await?;

        // Delete existing edges
        sqlx::query("DELETE FROM collection_workflow_edges WHERE collection_id = $1")
            .bind(collection_id)
            .execute(&mut *tx)
            .await?;

        // Insert new edges
        for edge in edges {
            sqlx::query("INSERT INTO collection_workflow_edges (from_workflow_id, to_workflow_id, collection_id) VALUES ($1, $2, $3)")
                .bind(edge.from_workflow_id)
                .bind(edge.to_workflow_id)
                .bind(collection_id)
                .execute(&mut *tx)
                .await?;
        }

        tx.commit().await?;
        Ok(())
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
        let row = sqlx::query_as::<_, WorkflowExecutionRow>(
            "INSERT INTO workflow_executions (collection_run_id, workflow_id, user_id, status) \
             VALUES ($1, $2, $3, 'pending') \
             RETURNING *",
        )
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
        let mut tx = self.pool.begin().await?;

        // Delete existing agents
        sqlx::query("DELETE FROM workflow_step_agents WHERE step_id = $1")
            .bind(step_id)
            .execute(&mut *tx)
            .await?;

        // Insert new agents
        for agent in agents {
            sqlx::query("INSERT INTO workflow_step_agents (step_id, agent_id, execution_strategy, agent_order) VALUES ($1, $2, $3, $4)")
                .bind(step_id)
                .bind(agent.agent_id)
                .bind(agent.execution_strategy)
                .bind(agent.agent_order)
                .execute(&mut *tx)
                .await?;
        }

        tx.commit().await?;
        Ok(())
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
            "SELECT t.id, t.name, t.description, t.input_schema, t.created_at, t.updated_at
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

#[cfg(test)]
mod tests;
