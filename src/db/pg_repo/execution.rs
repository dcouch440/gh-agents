use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::db::traits::{AgentExecutionRepo, CreateAgentExecutionInput};
use crate::db::{AgentExecutionRow, ExecutionMessageRow, TimelineRow};

use super::PgRepo;

#[async_trait]
impl AgentExecutionRepo for PgRepo {
    async fn create_agent_execution(
        &self,
        input: CreateAgentExecutionInput,
    ) -> Result<AgentExecutionRow> {
        use crate::types::ExecutionType;
        let is_interactive = input.execution_type == ExecutionType::InteractiveReview;
        let row = sqlx::query_as::<_, AgentExecutionRow>(
            "INSERT INTO agent_executions \
             (execution_type, agent_id, workflow_step_id, is_interactive, \
              parent_agent_execution_id, system_prompt_rendered, input, \
              room_session_id, speaker_order, workflow_execution_id) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) RETURNING *",
        )
        .bind(input.execution_type.as_str())
        .bind(input.agent_id)
        .bind(input.workflow_step_id)
        .bind(is_interactive)
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
               AND execution_type != 'interactive_review' \
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
               AND execution_type = 'interactive_review' \
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
                 WHERE ae.status = $1 AND ae.execution_type = 'interactive_review' AND we.user_id = $2 \
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
                 WHERE ae.execution_type = 'interactive_review' AND we.user_id = $1 \
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

    async fn update_execution_trace(&self, id: Uuid, trace: serde_json::Value) -> Result<()> {
        sqlx::query("UPDATE agent_executions SET trace = $2 WHERE id = $1")
            .bind(id)
            .bind(trace)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn get_latest_dispatch_execution_for_step(
        &self,
        step_id: Uuid,
    ) -> Result<Option<AgentExecutionRow>> {
        let row = sqlx::query_as::<_, AgentExecutionRow>(
            "SELECT * FROM agent_executions \
             WHERE workflow_step_id = $1 \
               AND execution_type IN ('dispatch', 'manager_dispatch') \
             ORDER BY started_at DESC \
             LIMIT 1",
        )
        .bind(step_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    async fn get_latest_dispatch_executions_for_steps(
        &self,
        step_ids: &[Uuid],
    ) -> Result<Vec<AgentExecutionRow>> {
        if step_ids.is_empty() {
            return Ok(Vec::new());
        }
        let rows = sqlx::query_as::<_, AgentExecutionRow>(
            "SELECT id, execution_type, agent_id, workflow_step_id, \
                    workflow_execution_id, is_interactive, \
                    parent_agent_execution_id, system_prompt_rendered, \
                    input, output, structured_output, room_session_id, \
                    speaker_order, status, started_at, completed_at, \
                    is_exemplary, trace \
             FROM ( \
                 SELECT *, ROW_NUMBER() OVER ( \
                     PARTITION BY workflow_step_id ORDER BY started_at DESC \
                 ) AS rn \
                 FROM agent_executions \
                 WHERE workflow_step_id = ANY($1) \
                   AND execution_type IN ('dispatch', 'manager_dispatch') \
             ) sub WHERE rn = 1",
        )
        .bind(step_ids)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    async fn get_running_step_ids_for_run(&self, workflow_execution_id: Uuid) -> Result<Vec<Uuid>> {
        let rows: Vec<(Uuid,)> = sqlx::query_as(
            "SELECT DISTINCT workflow_step_id FROM agent_executions \
             WHERE workflow_execution_id = $1 \
               AND status IN ('pending', 'running') \
               AND workflow_step_id IS NOT NULL",
        )
        .bind(workflow_execution_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|r| r.0).collect())
    }

    async fn list_execution_timeline(
        &self,
        workflow_execution_id: Uuid,
        limit: i64,
        before: Option<DateTime<Utc>>,
    ) -> Result<Vec<TimelineRow>> {
        let rows = if let Some(before_ts) = before {
            sqlx::query_as::<_, TimelineRow>(
                "SELECT \
                   em.id, \
                   em.created_at AS ts, \
                   em.role, \
                   em.content, \
                   em.tool_call_id, \
                   em.input_tokens, \
                   em.output_tokens, \
                   ae.id AS agent_execution_id, \
                   ae.execution_type, \
                   ae.workflow_step_id AS step_id, \
                   ws.name AS step_name, \
                   CASE WHEN ae.execution_type = 'pipeline_agent' \
                     THEN pe.agent_name \
                     ELSE a.name \
                   END AS agent_name, \
                   ae.status AS agent_status \
                 FROM execution_messages em \
                 JOIN agent_executions ae ON ae.id = em.agent_execution_id \
                 LEFT JOIN workflow_steps ws ON ws.id = ae.workflow_step_id \
                 LEFT JOIN agents a ON a.id = ae.agent_id \
                 LEFT JOIN protocol_executions pe \
                   ON pe.protocol_step_id = ae.workflow_step_id \
                   AND pe.workflow_run_id = $1 \
                   AND pe.phase LIKE 'agent_%' \
                   AND pe.agent_name IS NOT NULL \
                   AND ae.execution_type = 'pipeline_agent' \
                   AND ae.started_at >= pe.created_at \
                   AND (pe.completed_at IS NULL OR ae.started_at <= pe.completed_at) \
                 WHERE ae.workflow_execution_id = $1 \
                   AND em.created_at < $2 \
                 ORDER BY em.created_at DESC \
                 LIMIT $3",
            )
            .bind(workflow_execution_id)
            .bind(before_ts)
            .bind(limit)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, TimelineRow>(
                "SELECT \
                   em.id, \
                   em.created_at AS ts, \
                   em.role, \
                   em.content, \
                   em.tool_call_id, \
                   em.input_tokens, \
                   em.output_tokens, \
                   ae.id AS agent_execution_id, \
                   ae.execution_type, \
                   ae.workflow_step_id AS step_id, \
                   ws.name AS step_name, \
                   CASE WHEN ae.execution_type = 'pipeline_agent' \
                     THEN pe.agent_name \
                     ELSE a.name \
                   END AS agent_name, \
                   ae.status AS agent_status \
                 FROM execution_messages em \
                 JOIN agent_executions ae ON ae.id = em.agent_execution_id \
                 LEFT JOIN workflow_steps ws ON ws.id = ae.workflow_step_id \
                 LEFT JOIN agents a ON a.id = ae.agent_id \
                 LEFT JOIN protocol_executions pe \
                   ON pe.protocol_step_id = ae.workflow_step_id \
                   AND pe.workflow_run_id = $1 \
                   AND pe.phase LIKE 'agent_%' \
                   AND pe.agent_name IS NOT NULL \
                   AND ae.execution_type = 'pipeline_agent' \
                   AND ae.started_at >= pe.created_at \
                   AND (pe.completed_at IS NULL OR ae.started_at <= pe.completed_at) \
                 WHERE ae.workflow_execution_id = $1 \
                 ORDER BY em.created_at DESC \
                 LIMIT $2",
            )
            .bind(workflow_execution_id)
            .bind(limit)
            .fetch_all(&self.pool)
            .await?
        };
        Ok(rows)
    }
}
