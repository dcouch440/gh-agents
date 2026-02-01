//! PostgreSQL implementation of repository traits.

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::traits::{CostRepo, DependencyRepo, DocumentRepo, MergeQueueRepo, RefactorRepo, SchedulerRepo, ServerRepo, TaskQueueRepo, UserRepo};
use crate::db::{
    AgentRow, ChatMessageRow, ClusterRow, DocumentRow, DocumentSearchResult, PipelineRow, PipelineRunRow, PipelineStageRow, ScheduleRow, SessionRow, StageExecutionRow,
    StageSideTaskRow, ToolRow, TriggerRow, UsageSummaryRow,
};
use crate::github::{PrQueueEntry, QueueError as MergeQueueError};
use crate::orchestration::DependencyError;
use crate::orchestration::QueueError as TaskQueueError;
use crate::types::{AgentId, AgentTier, ChangeId, ChangeStatus, CostRecord, ProductionMode, RefactorChange, RefactorSession, Task, TaskId, TaskStatus, User, UserId};

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
impl DependencyRepo for PgRepo {
    async fn get_task_status(&self, id: TaskId) -> Result<Option<TaskStatus>, DependencyError> {
        let row: Option<(String,)> = sqlx::query_as("SELECT status FROM tasks WHERE id = $1")
            .bind(id.0)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| DependencyError::DatabaseError(e.to_string()))?;

        Ok(row.map(|r| match r.0.as_str() {
            "pending" => TaskStatus::Pending,
            "inprogress" | "in_progress" => TaskStatus::InProgress,
            "review" => TaskStatus::Review,
            "completed" => TaskStatus::Completed,
            "failed" => TaskStatus::Failed,
            _ => TaskStatus::Pending,
        }))
    }

    async fn get_blocked_by(&self, task_id: TaskId) -> Result<Vec<TaskId>, DependencyError> {
        let rows: Vec<(Uuid,)> = sqlx::query_as("SELECT task_id FROM task_dependencies WHERE depends_on_id = $1")
            .bind(task_id.0)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DependencyError::DatabaseError(e.to_string()))?;

        Ok(rows.into_iter().map(|r| TaskId(r.0)).collect())
    }

    async fn get_task_dependencies(&self, task_id: TaskId) -> Result<Vec<TaskId>, DependencyError> {
        let rows: Vec<(Uuid,)> = sqlx::query_as("SELECT depends_on_id FROM task_dependencies WHERE task_id = $1")
            .bind(task_id.0)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| DependencyError::DatabaseError(e.to_string()))?;

        Ok(rows.into_iter().map(|r| TaskId(r.0)).collect())
    }

    async fn save_dependency(&self, task_id: TaskId, depends_on: TaskId, now: DateTime<Utc>) -> Result<(), DependencyError> {
        sqlx::query(
            r#"
            INSERT INTO task_dependencies (task_id, depends_on_id, created_at)
            VALUES ($1, $2, $3)
            ON CONFLICT DO NOTHING
            "#,
        )
        .bind(task_id.0)
        .bind(depends_on.0)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| DependencyError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    async fn remove_dependency(&self, task_id: TaskId, depends_on: TaskId) -> Result<(), DependencyError> {
        sqlx::query("DELETE FROM task_dependencies WHERE task_id = $1 AND depends_on_id = $2")
            .bind(task_id.0)
            .bind(depends_on.0)
            .execute(&self.pool)
            .await
            .map_err(|e| DependencyError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    async fn get_ready_task_ids(&self) -> Result<Vec<TaskId>, DependencyError> {
        let rows: Vec<(Uuid,)> = sqlx::query_as(
            r#"
            SELECT t.id FROM tasks t
            WHERE t.status = 'pending'
            AND NOT EXISTS (
                SELECT 1 FROM task_dependencies td
                JOIN tasks dep ON td.depends_on_id = dep.id
                WHERE td.task_id = t.id
                AND dep.status != 'completed'
            )
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DependencyError::DatabaseError(e.to_string()))?;

        Ok(rows.into_iter().map(|r| TaskId(r.0)).collect())
    }
}

#[async_trait]
impl TaskQueueRepo for PgRepo {
    async fn list_tasks_by_status(&self, status: TaskStatus) -> Result<Vec<Task>, TaskQueueError> {
        crate::db::list_tasks_by_status(&self.pool, status)
            .await
            .map_err(|e| TaskQueueError::DatabaseError(e.to_string()))
    }

    async fn update_task_status(&self, id: TaskId, status: TaskStatus) -> Result<(), TaskQueueError> {
        crate::db::update_task_status(&self.pool, &id, status)
            .await
            .map_err(|e| TaskQueueError::DatabaseError(e.to_string()))
    }

    async fn update_task_for_requeue(&self, task_id: TaskId, priority_str: String, policy_description: String, now: DateTime<Utc>) -> Result<(), TaskQueueError> {
        sqlx::query(
            r#"
            UPDATE tasks
            SET status = 'pending',
                priority = $1,
                updated_at = $2
            WHERE id = $3
            "#,
        )
        .bind(&priority_str)
        .bind(now)
        .bind(task_id.0)
        .execute(&self.pool)
        .await
        .map_err(|e| TaskQueueError::DatabaseError(e.to_string()))?;

        sqlx::query(
            r#"
            INSERT INTO task_events (id, task_id, event_type, details, timestamp)
            VALUES ($1, $2, 'requeued', $3, $4)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(task_id.0)
        .bind(policy_description)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| TaskQueueError::DatabaseError(e.to_string()))?;

        Ok(())
    }
}

#[async_trait]
impl SchedulerRepo for PgRepo {
    async fn get_production_mode(&self) -> Result<ProductionMode, anyhow::Error> {
        let row: Option<(String,)> = sqlx::query_as("SELECT value FROM system_state WHERE key = 'production_mode'")
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to fetch production mode: {}", e))?;

        Ok(row.map(|(v,)| ProductionMode::from_db_str(&v)).unwrap_or_default())
    }

    async fn set_production_mode(&self, mode: ProductionMode) -> Result<(), anyhow::Error> {
        let value = mode.as_str();

        sqlx::query(
            r#"
            INSERT INTO system_state (key, value, updated_at)
            VALUES ('production_mode', $1, $2)
            ON CONFLICT(key) DO UPDATE SET
                value = excluded.value,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(value)
        .bind(Utc::now())
        .execute(&self.pool)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to set production mode: {}", e))?;

        Ok(())
    }
}

#[async_trait]
impl CostRepo for PgRepo {
    async fn persist_cost_record(&self, record: CostRecord) -> Result<(), String> {
        let task_id = record.task_id.as_ref().map(|id| id.0);
        let tier_str = format!("{:?}", record.agent_tier);

        sqlx::query(
            r#"
            INSERT INTO cost_records (
                id, task_id, agent_id, agent_tier, model_id,
                input_tokens, output_tokens, cost_usd, timestamp
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
        )
        .bind(record.id)
        .bind(task_id)
        .bind(record.agent_id.0)
        .bind(tier_str)
        .bind(&record.model_id)
        .bind(record.input_tokens as i32)
        .bind(record.output_tokens as i32)
        .bind(record.cost_usd as f32)
        .bind(record.timestamp)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        Ok(())
    }

    async fn get_cost_records(&self, since: Option<DateTime<Utc>>) -> Result<Vec<CostRecord>, String> {
        let rows: Vec<CostRecordPgRow> = if let Some(since_time) = since {
            sqlx::query_as(
                r#"
                SELECT id, task_id, agent_id, agent_tier, model_id,
                       input_tokens, output_tokens, cost_usd, timestamp
                FROM cost_records
                WHERE timestamp >= $1
                ORDER BY timestamp DESC
                "#,
            )
            .bind(since_time)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?
        } else {
            sqlx::query_as(
                r#"
                SELECT id, task_id, agent_id, agent_tier, model_id,
                       input_tokens, output_tokens, cost_usd, timestamp
                FROM cost_records
                ORDER BY timestamp DESC
                "#,
            )
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?
        };

        let cost_records: Vec<CostRecord> = rows.into_iter().filter_map(|row| row.try_into().ok()).collect();

        Ok(cost_records)
    }
}

#[async_trait]
impl RefactorRepo for PgRepo {
    async fn get_active_refactor_session(&self) -> Result<Option<RefactorSession>> {
        crate::db::get_active_refactor_session(&self.pool).await
    }

    async fn insert_refactor_session(&self, session: RefactorSession) -> Result<()> {
        crate::db::insert_refactor_session(&self.pool, &session).await
    }

    async fn update_refactor_session(&self, session: RefactorSession) -> Result<()> {
        crate::db::update_refactor_session(&self.pool, &session).await
    }

    async fn insert_refactor_change(&self, change: RefactorChange) -> Result<()> {
        crate::db::insert_refactor_change(&self.pool, &change).await
    }

    async fn update_change_status(&self, id: ChangeId, status: ChangeStatus) -> Result<()> {
        crate::db::update_change_status(&self.pool, &id, status).await
    }
}

/// Database row for cost records (used by CostRepo impl)
#[derive(Debug, sqlx::FromRow)]
struct CostRecordPgRow {
    id: Uuid,
    task_id: Option<Uuid>,
    agent_id: Uuid,
    agent_tier: String,
    model_id: String,
    input_tokens: i32,
    output_tokens: i32,
    cost_usd: f32,
    timestamp: DateTime<Utc>,
}

impl TryFrom<CostRecordPgRow> for CostRecord {
    type Error = String;

    fn try_from(row: CostRecordPgRow) -> Result<Self, Self::Error> {
        let agent_tier = match row.agent_tier.as_str() {
            "Orchestrator" => AgentTier::Orchestrator,
            "Worker" => AgentTier::Worker,
            "Utility" => AgentTier::Utility,
            _ => AgentTier::Worker,
        };

        Ok(CostRecord {
            id: row.id,
            task_id: row.task_id.map(TaskId),
            agent_id: AgentId(row.agent_id),
            agent_tier,
            model_id: row.model_id,
            input_tokens: row.input_tokens as u32,
            output_tokens: row.output_tokens as u32,
            cost_usd: row.cost_usd as f64,
            timestamp: row.timestamp,
        })
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
            "SELECT id, tier, persona_name, persona_prompt, persona_style, model_provider, model_id, model_max_tokens, model_temperature, status, router_mode FROM agents WHERE user_id = $1"
        )
        .bind(user_id.0)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(agent_row_from_pg).collect())
    }

    async fn get_persisted_agent(&self, agent_id: Uuid) -> Result<Option<AgentRow>> {
        let row = sqlx::query_as::<_, PgAgentRow>(
            "SELECT id, tier, persona_name, persona_prompt, persona_style, model_provider, model_id, model_max_tokens, model_temperature, status, router_mode FROM agents WHERE id = $1"
        )
        .bind(agent_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(agent_row_from_pg))
    }

    async fn upsert_agent(&self, user_id: UserId, agent: AgentRow) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO agents (id, user_id, tier, persona_name, persona_prompt, persona_style, model_provider, model_id, model_max_tokens, model_temperature, status, router_mode)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            ON CONFLICT (id) DO UPDATE SET
                tier = EXCLUDED.tier,
                persona_name = EXCLUDED.persona_name,
                persona_prompt = EXCLUDED.persona_prompt,
                persona_style = EXCLUDED.persona_style,
                model_provider = EXCLUDED.model_provider,
                model_id = EXCLUDED.model_id,
                model_max_tokens = EXCLUDED.model_max_tokens,
                model_temperature = EXCLUDED.model_temperature,
                status = EXCLUDED.status,
                router_mode = EXCLUDED.router_mode
        "#,
        )
        .bind(agent.id)
        .bind(user_id.0)
        .bind(&agent.tier)
        .bind(&agent.persona_name)
        .bind(&agent.persona_prompt)
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
        let rows = sqlx::query_as::<_, PgToolRow>(
            "SELECT id, name, description, category, parameter_schema, output_schema, enabled, cluster_id, is_builtin FROM tools WHERE user_id = $1 ORDER BY category, name",
        )
        .bind(user_id.0)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(tool_row_from_pg).collect())
    }

    async fn get_tool(&self, tool_id: Uuid) -> Result<Option<ToolRow>> {
        let row =
            sqlx::query_as::<_, PgToolRow>("SELECT id, name, description, category, parameter_schema, output_schema, enabled, cluster_id, is_builtin FROM tools WHERE id = $1")
                .bind(tool_id)
                .fetch_optional(&self.pool)
                .await?;

        Ok(row.map(tool_row_from_pg))
    }

    async fn upsert_tool(&self, user_id: UserId, tool: ToolRow) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO tools (id, user_id, name, description, category, parameter_schema, output_schema, enabled, cluster_id, is_builtin)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT (id) DO UPDATE SET
                name = EXCLUDED.name,
                description = EXCLUDED.description,
                category = EXCLUDED.category,
                parameter_schema = EXCLUDED.parameter_schema,
                output_schema = EXCLUDED.output_schema,
                enabled = EXCLUDED.enabled,
                cluster_id = EXCLUDED.cluster_id,
                is_builtin = EXCLUDED.is_builtin
        "#,
        )
        .bind(tool.id)
        .bind(user_id.0)
        .bind(&tool.name)
        .bind(&tool.description)
        .bind(&tool.category)
        .bind(&tool.parameter_schema)
        .bind(&tool.output_schema)
        .bind(tool.enabled)
        .bind(tool.cluster_id)
        .bind(tool.is_builtin)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn delete_tool(&self, tool_id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM tools WHERE id = $1 AND is_builtin = false")
            .bind(tool_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn get_agent_tools(&self, agent_id: Uuid) -> Result<Vec<ToolRow>> {
        let rows = sqlx::query_as::<_, PgToolRow>(
            "SELECT t.id, t.name, t.description, t.category, t.parameter_schema, t.output_schema, t.enabled, t.cluster_id, t.is_builtin FROM tools t INNER JOIN agent_tools at ON t.id = at.tool_id WHERE at.agent_id = $1 ORDER BY t.category, t.name",
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
                INSERT INTO tools (id, user_id, name, description, category, parameter_schema, output_schema, enabled, cluster_id, is_builtin)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                ON CONFLICT (user_id, name) DO NOTHING
            "#,
            )
            .bind(tool.id)
            .bind(user_id.0)
            .bind(&tool.name)
            .bind(&tool.description)
            .bind(&tool.category)
            .bind(&tool.parameter_schema)
            .bind(&tool.output_schema)
            .bind(tool.enabled)
            .bind(tool.cluster_id)
            .bind(tool.is_builtin)
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

    // --- Cluster persistence ---

    async fn list_persisted_clusters(&self, user_id: UserId) -> Result<Vec<ClusterRow>> {
        let rows = sqlx::query_as::<_, PgClusterRow>("SELECT id, name, description, conventions, shared_files FROM clusters WHERE user_id = $1")
            .bind(user_id.0)
            .fetch_all(&self.pool)
            .await?;

        Ok(rows
            .into_iter()
            .map(|r| ClusterRow {
                id: r.id,
                name: r.name,
                description: r.description,
                conventions: r.conventions,
                shared_files: r.shared_files,
            })
            .collect())
    }

    async fn upsert_cluster(&self, user_id: UserId, cluster: ClusterRow) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO clusters (id, user_id, name, description, conventions, shared_files)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (id) DO UPDATE SET
                name = EXCLUDED.name,
                description = EXCLUDED.description,
                conventions = EXCLUDED.conventions,
                shared_files = EXCLUDED.shared_files
        "#,
        )
        .bind(cluster.id)
        .bind(user_id.0)
        .bind(&cluster.name)
        .bind(&cluster.description)
        .bind(&cluster.conventions)
        .bind(&cluster.shared_files)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn delete_cluster(&self, cluster_id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM clusters WHERE id = $1").bind(cluster_id).execute(&self.pool).await?;
        Ok(())
    }

    async fn list_cluster_members(&self, cluster_id: Uuid) -> Result<Vec<Uuid>> {
        let rows: Vec<(Uuid,)> = sqlx::query_as("SELECT agent_id FROM cluster_members WHERE cluster_id = $1")
            .bind(cluster_id)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows.into_iter().map(|r| r.0).collect())
    }

    async fn add_cluster_member(&self, cluster_id: Uuid, agent_id: Uuid) -> Result<()> {
        sqlx::query("INSERT INTO cluster_members (cluster_id, agent_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
            .bind(cluster_id)
            .bind(agent_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn remove_cluster_member(&self, cluster_id: Uuid, agent_id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM cluster_members WHERE cluster_id = $1 AND agent_id = $2")
            .bind(cluster_id)
            .bind(agent_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // --- Pipeline persistence ---

    async fn list_pipelines(&self, user_id: UserId) -> Result<Vec<PipelineRow>> {
        let rows: Vec<(Uuid, String)> = sqlx::query_as("SELECT id, name FROM pipelines WHERE user_id = $1")
            .bind(user_id.0)
            .fetch_all(&self.pool)
            .await?;

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
        let rows: Vec<(Uuid, i32, Option<Uuid>, Option<Uuid>, Option<String>, bool, bool, String, serde_json::Value, String, serde_json::Value)> = sqlx::query_as(
            "SELECT pipeline_id, stage_number, agent_id, cluster_id, role, approval_required, fan_out, stage_name, input_definitions, output_description, output_schema FROM pipeline_stages WHERE pipeline_id = $1 ORDER BY stage_number"
        )
        .bind(pipeline_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(
                |(pipeline_id, stage_number, agent_id, cluster_id, role, approval_required, fan_out, stage_name, input_definitions, output_description, output_schema)| {
                    PipelineStageRow {
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
                    }
                },
            )
            .collect())
    }

    async fn upsert_pipeline_stage(&self, stage: PipelineStageRow) -> Result<()> {
        sqlx::query(r#"
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
        "#)
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

    // --- Stage side task persistence ---

    async fn list_stage_side_tasks(&self, pipeline_id: Uuid, stage_number: i32) -> Result<Vec<StageSideTaskRow>> {
        let rows: Vec<(Uuid, Uuid, i32, Uuid, serde_json::Value, String, bool, serde_json::Value)> = sqlx::query_as(
            "SELECT id, pipeline_id, stage_number, agent_id, input_definitions, output_name, blocking, output_schema FROM stage_side_tasks WHERE pipeline_id = $1 AND stage_number = $2"
        )
        .bind(pipeline_id)
        .bind(stage_number)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(
                |(id, pipeline_id, stage_number, agent_id, input_definitions, output_name, blocking, output_schema)| StageSideTaskRow {
                    id,
                    pipeline_id,
                    stage_number,
                    agent_id,
                    input_definitions,
                    output_name,
                    blocking,
                    output_schema,
                },
            )
            .collect())
    }

    async fn upsert_stage_side_task(&self, side_task: StageSideTaskRow) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO stage_side_tasks (id, pipeline_id, stage_number, agent_id, input_definitions, output_name, blocking, output_schema)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (id) DO UPDATE SET
                agent_id = EXCLUDED.agent_id,
                input_definitions = EXCLUDED.input_definitions,
                output_name = EXCLUDED.output_name,
                blocking = EXCLUDED.blocking,
                output_schema = EXCLUDED.output_schema
        "#,
        )
        .bind(side_task.id)
        .bind(side_task.pipeline_id)
        .bind(side_task.stage_number)
        .bind(side_task.agent_id)
        .bind(&side_task.input_definitions)
        .bind(&side_task.output_name)
        .bind(side_task.blocking)
        .bind(&side_task.output_schema)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn delete_stage_side_task(&self, side_task_id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM stage_side_tasks WHERE id = $1").bind(side_task_id).execute(&self.pool).await?;
        Ok(())
    }

    // --- Schedule persistence ---

    async fn list_schedules(&self, user_id: UserId) -> Result<Vec<ScheduleRow>> {
        let rows: Vec<(Uuid, String, Uuid, i32, String, String, Option<String>, bool, Option<DateTime<Utc>>)> =
            sqlx::query_as("SELECT id, name, agent_id, interval_seconds, task_title, task_description, role, enabled, last_run_at FROM schedules WHERE user_id = $1")
                .bind(user_id.0)
                .fetch_all(&self.pool)
                .await?;

        Ok(rows
            .into_iter()
            .map(
                |(id, name, agent_id, interval_seconds, task_title, task_description, role, enabled, last_run_at)| ScheduleRow {
                    id,
                    name,
                    agent_id,
                    interval_seconds,
                    task_title,
                    task_description,
                    role,
                    enabled,
                    last_run_at,
                },
            )
            .collect())
    }

    async fn upsert_schedule(&self, user_id: UserId, schedule: ScheduleRow) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO schedules (id, user_id, name, agent_id, interval_seconds, task_title, task_description, role, enabled, last_run_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT (id) DO UPDATE SET
                name = EXCLUDED.name,
                agent_id = EXCLUDED.agent_id,
                interval_seconds = EXCLUDED.interval_seconds,
                task_title = EXCLUDED.task_title,
                task_description = EXCLUDED.task_description,
                role = EXCLUDED.role,
                enabled = EXCLUDED.enabled,
                last_run_at = EXCLUDED.last_run_at
        "#,
        )
        .bind(schedule.id)
        .bind(user_id.0)
        .bind(&schedule.name)
        .bind(schedule.agent_id)
        .bind(schedule.interval_seconds)
        .bind(&schedule.task_title)
        .bind(&schedule.task_description)
        .bind(&schedule.role)
        .bind(schedule.enabled)
        .bind(schedule.last_run_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn delete_schedule(&self, schedule_id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM schedules WHERE id = $1").bind(schedule_id).execute(&self.pool).await?;
        Ok(())
    }

    async fn update_schedule_last_run(&self, schedule_id: Uuid, last_run_at: DateTime<Utc>) -> Result<()> {
        sqlx::query("UPDATE schedules SET last_run_at = $1 WHERE id = $2")
            .bind(last_run_at)
            .bind(schedule_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // --- Trigger persistence ---

    async fn list_triggers(&self, user_id: UserId) -> Result<Vec<TriggerRow>> {
        let rows: Vec<(Uuid, String, String, Uuid, String, String, Option<String>)> =
            sqlx::query_as("SELECT id, name, event_type, agent_id, task_title, task_description, role FROM triggers WHERE user_id = $1")
                .bind(user_id.0)
                .fetch_all(&self.pool)
                .await?;

        Ok(rows
            .into_iter()
            .map(|(id, name, event_type, agent_id, task_title, task_description, role)| TriggerRow {
                id,
                name,
                event_type,
                agent_id,
                task_title,
                task_description,
                role,
            })
            .collect())
    }

    async fn upsert_trigger(&self, user_id: UserId, trigger: TriggerRow) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO triggers (id, user_id, name, event_type, agent_id, task_title, task_description, role)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (id) DO UPDATE SET
                name = EXCLUDED.name,
                event_type = EXCLUDED.event_type,
                agent_id = EXCLUDED.agent_id,
                task_title = EXCLUDED.task_title,
                task_description = EXCLUDED.task_description,
                role = EXCLUDED.role
        "#,
        )
        .bind(trigger.id)
        .bind(user_id.0)
        .bind(&trigger.name)
        .bind(&trigger.event_type)
        .bind(trigger.agent_id)
        .bind(&trigger.task_title)
        .bind(&trigger.task_description)
        .bind(&trigger.role)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn delete_trigger(&self, trigger_id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM triggers WHERE id = $1").bind(trigger_id).execute(&self.pool).await?;
        Ok(())
    }

    // --- Session management ---

    async fn create_session(&self, user_id: UserId, session_id: Uuid, mode_id: &str, title: &str) -> Result<()> {
        crate::db::create_session(&self.pool, user_id, session_id, mode_id, title).await
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

    async fn insert_token_usage(&self, session_id: Option<Uuid>, agent_id: Option<Uuid>, tier: &str, model_id: &str, input_tokens: i64, output_tokens: i64) -> Result<()> {
        sqlx::query("INSERT INTO token_usage (session_id, agent_id, tier, model_id, input_tokens, output_tokens) VALUES ($1, $2, $3, $4, $5, $6)")
            .bind(session_id)
            .bind(agent_id)
            .bind(tier)
            .bind(model_id)
            .bind(input_tokens)
            .bind(output_tokens)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn get_usage_summary(&self, since_hours: u32) -> Result<Vec<UsageSummaryRow>> {
        let rows = sqlx::query_as::<_, UsageSummaryRow>(
            r#"
            SELECT tier, model_id,
                   COALESCE(SUM(input_tokens), 0)::bigint AS total_input,
                   COALESCE(SUM(output_tokens), 0)::bigint AS total_output,
                   COUNT(*) AS call_count
            FROM token_usage
            WHERE created_at > NOW() - make_interval(hours => $1::int)
            GROUP BY tier, model_id
            ORDER BY SUM(input_tokens + output_tokens) DESC
            "#,
        )
        .bind(since_hours as i32)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

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
        let row: Option<(Uuid, Uuid, Uuid, String, String, serde_json::Value, i32, chrono::DateTime<chrono::Utc>, Option<chrono::DateTime<chrono::Utc>>, i64, i64)> = sqlx::query_as(
            "SELECT id, pipeline_id, user_id, status, initial_task, stage_outputs, current_stage, started_at, completed_at, total_input_tokens, total_output_tokens FROM pipeline_runs WHERE id = $1"
        )
        .bind(run_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(
            |(id, pipeline_id, user_id, status, initial_task, stage_outputs, current_stage, started_at, completed_at, total_input_tokens, total_output_tokens)| PipelineRunRow {
                id,
                pipeline_id,
                user_id,
                status,
                initial_task,
                stage_outputs,
                current_stage,
                started_at,
                completed_at,
                total_input_tokens,
                total_output_tokens,
            },
        ))
    }

    async fn list_pipeline_runs(&self, pipeline_id: Uuid) -> Result<Vec<PipelineRunRow>> {
        let rows: Vec<(Uuid, Uuid, Uuid, String, String, serde_json::Value, i32, chrono::DateTime<chrono::Utc>, Option<chrono::DateTime<chrono::Utc>>, i64, i64)> = sqlx::query_as(
            "SELECT id, pipeline_id, user_id, status, initial_task, stage_outputs, current_stage, started_at, completed_at, total_input_tokens, total_output_tokens FROM pipeline_runs WHERE pipeline_id = $1 ORDER BY started_at DESC"
        )
        .bind(pipeline_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(
                |(id, pipeline_id, user_id, status, initial_task, stage_outputs, current_stage, started_at, completed_at, total_input_tokens, total_output_tokens)| {
                    PipelineRunRow {
                        id,
                        pipeline_id,
                        user_id,
                        status,
                        initial_task,
                        stage_outputs,
                        current_stage,
                        started_at,
                        completed_at,
                        total_input_tokens,
                        total_output_tokens,
                    }
                },
            )
            .collect())
    }

    async fn create_stage_execution(&self, exec: &StageExecutionRow) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO stage_executions (id, run_id, stage_number, stage_name, agent_id, status, rendered_prompt, output, structured_output, user_input, input_tokens, output_tokens, started_at, completed_at, duration_ms)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
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
        let rows: Vec<(Uuid, Uuid, i32, String, Option<Uuid>, String, Option<String>, Option<String>, Option<serde_json::Value>, Option<String>, i64, i64, chrono::DateTime<chrono::Utc>, Option<chrono::DateTime<chrono::Utc>>, i64)> = sqlx::query_as(
            "SELECT id, run_id, stage_number, stage_name, agent_id, status, rendered_prompt, output, structured_output, user_input, input_tokens, output_tokens, started_at, completed_at, duration_ms FROM stage_executions WHERE run_id = $1 ORDER BY stage_number"
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(
                |(
                    id,
                    run_id,
                    stage_number,
                    stage_name,
                    agent_id,
                    status,
                    rendered_prompt,
                    output,
                    structured_output,
                    user_input,
                    input_tokens,
                    output_tokens,
                    started_at,
                    completed_at,
                    duration_ms,
                )| StageExecutionRow {
                    id,
                    run_id,
                    stage_number,
                    stage_name,
                    agent_id,
                    status,
                    rendered_prompt,
                    output,
                    structured_output,
                    user_input,
                    input_tokens,
                    output_tokens,
                    started_at,
                    completed_at,
                    duration_ms,
                },
            )
            .collect())
    }

    async fn insert_tool_call(&self, call: crate::db::traits::ToolCallInput) -> Result<()> {
        sqlx::query("INSERT INTO tool_calls (id, session_id, message_id, round, tool_name, tool_use_id, input, output, latency_ms) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)")
            .bind(Uuid::new_v4())
            .bind(call.session_id)
            .bind(call.message_id)
            .bind(call.round)
            .bind(&call.tool_name)
            .bind(&call.tool_use_id)
            .bind(&call.input)
            .bind(&call.output)
            .bind(call.latency_ms)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

// ============================================================================
// User Repository
// ============================================================================

#[derive(sqlx::FromRow)]
struct PgAgentRow {
    id: Uuid,
    tier: String,
    persona_name: String,
    persona_prompt: String,
    persona_style: String,
    model_provider: String,
    model_id: String,
    model_max_tokens: i32,
    model_temperature: f32,
    status: String,
    router_mode: bool,
}

fn agent_row_from_pg(r: PgAgentRow) -> AgentRow {
    AgentRow {
        id: r.id,
        tier: r.tier,
        persona_name: r.persona_name,
        persona_prompt: r.persona_prompt,
        persona_style: r.persona_style,
        model_provider: r.model_provider,
        model_id: r.model_id,
        model_max_tokens: r.model_max_tokens,
        model_temperature: r.model_temperature,
        status: r.status,
        router_mode: r.router_mode,
    }
}

#[derive(sqlx::FromRow)]
struct PgToolRow {
    id: Uuid,
    name: String,
    description: String,
    category: String,
    parameter_schema: serde_json::Value,
    output_schema: serde_json::Value,
    enabled: bool,
    cluster_id: Option<Uuid>,
    is_builtin: bool,
}

fn tool_row_from_pg(r: PgToolRow) -> ToolRow {
    ToolRow {
        id: r.id,
        name: r.name,
        description: r.description,
        category: r.category,
        parameter_schema: r.parameter_schema,
        output_schema: r.output_schema,
        enabled: r.enabled,
        cluster_id: r.cluster_id,
        is_builtin: r.is_builtin,
    }
}

#[derive(sqlx::FromRow)]
struct PgClusterRow {
    id: Uuid,
    name: String,
    description: String,
    conventions: String,
    shared_files: serde_json::Value,
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
        let row: Option<UserRow> =
            sqlx::query_as("SELECT id, email, password_hash, github_id, github_login, github_token_encrypted, created_at, updated_at FROM users WHERE email = $1")
                .bind(email)
                .fetch_optional(&self.pool)
                .await?;

        Ok(row.map(|r| r.into()))
    }

    async fn get_user_by_id(&self, id: UserId) -> Result<Option<User>> {
        let row: Option<UserRow> =
            sqlx::query_as("SELECT id, email, password_hash, github_id, github_login, github_token_encrypted, created_at, updated_at FROM users WHERE id = $1")
                .bind(id.0)
                .fetch_optional(&self.pool)
                .await?;

        Ok(row.map(|r| r.into()))
    }

    async fn get_user_by_github_id(&self, github_id: i64) -> Result<Option<User>> {
        let row: Option<UserRow> =
            sqlx::query_as("SELECT id, email, password_hash, github_id, github_login, github_token_encrypted, created_at, updated_at FROM users WHERE github_id = $1")
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
        let row: Option<DocumentRow> =
            sqlx::query_as("SELECT id, user_id, session_id, title, content, summary, doc_type, ref_tag, tags, created_at, updated_at FROM documents WHERE id = $1")
                .bind(doc_id)
                .fetch_optional(&self.pool)
                .await?;
        Ok(row)
    }

    async fn get_document_by_ref_tag(&self, ref_tag: &str) -> Result<Option<DocumentRow>> {
        let row: Option<DocumentRow> =
            sqlx::query_as("SELECT id, user_id, session_id, title, content, summary, doc_type, ref_tag, tags, created_at, updated_at FROM documents WHERE ref_tag = $1")
                .bind(ref_tag)
                .fetch_optional(&self.pool)
                .await?;
        Ok(row)
    }

    async fn list_documents(&self, user_id: Uuid) -> Result<Vec<DocumentRow>> {
        let rows: Vec<DocumentRow> = sqlx::query_as(
            "SELECT id, user_id, session_id, title, content, summary, doc_type, ref_tag, tags, created_at, updated_at FROM documents WHERE user_id = $1 ORDER BY updated_at DESC",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn list_session_documents(&self, session_id: Uuid) -> Result<Vec<DocumentRow>> {
        let rows: Vec<DocumentRow> = sqlx::query_as(
            "SELECT id, user_id, session_id, title, content, summary, doc_type, ref_tag, tags, created_at, updated_at FROM documents WHERE session_id = $1 ORDER BY updated_at DESC",
        )
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
