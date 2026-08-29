use anyhow::Result;
use async_trait::async_trait;
use uuid::Uuid;

use crate::db::traits::{WorkflowCollectionRepo, WorkflowStepAgentRepo};
use crate::db::{
    CollectionRunRow, CollectionWorkflowEdgeRow, CollectionWorkflowRow, WorkflowCollectionRow,
    WorkflowExecutionRow, WorkflowStepAgentRow,
};

use super::PgRepo;

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
                 completed_at = CASE WHEN $2 IN ('completed', 'failed', 'cancelled') THEN NOW() ELSE completed_at END \
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
             AND execution_mode <> 'workshop' \
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
