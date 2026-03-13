use anyhow::Result;
use async_trait::async_trait;
use uuid::Uuid;

use crate::db::{
    CollectionRunRow, CollectionWorkflowEdgeRow, CollectionWorkflowRow, WorkflowCollectionRow,
    WorkflowExecutionRow, WorkflowStepAgentRow,
};

// ============================================================================
// Workflow Collection Repository
// ============================================================================

/// Database operations for workflow collections (multi-tier DAG architecture).
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait WorkflowCollectionRepo: Send + Sync {
    // --- Collections ---
    async fn create_collection(
        &self,
        user_id: Uuid,
        name: String,
        description: Option<String>,
        execution_mode: String,
    ) -> Result<WorkflowCollectionRow>;
    async fn get_collection(&self, id: Uuid) -> Result<Option<WorkflowCollectionRow>>;
    async fn list_collections(&self, user_id: Uuid) -> Result<Vec<WorkflowCollectionRow>>;
    async fn update_collection(
        &self,
        id: Uuid,
        name: Option<String>,
        description: Option<String>,
        execution_mode: Option<String>,
    ) -> Result<WorkflowCollectionRow>;
    async fn delete_collection(&self, id: Uuid) -> Result<()>;

    // --- Collection Workflows (Membership) ---
    async fn add_collection_workflow(
        &self,
        collection_id: Uuid,
        workflow_id: Uuid,
        display_order: i32,
        execution_mode: Option<String>,
    ) -> Result<CollectionWorkflowRow>;
    async fn list_collection_workflows(
        &self,
        collection_id: Uuid,
    ) -> Result<Vec<CollectionWorkflowRow>>;
    async fn remove_collection_workflow(
        &self,
        collection_id: Uuid,
        workflow_id: Uuid,
    ) -> Result<()>;
    async fn update_collection_workflow(
        &self,
        collection_id: Uuid,
        workflow_id: Uuid,
        display_order: Option<i32>,
        execution_mode: Option<String>,
    ) -> Result<CollectionWorkflowRow>;

    // --- Collection Workflow Edges (DAG edges between workflows) ---
    async fn set_collection_edges(
        &self,
        collection_id: Uuid,
        edges: Vec<CollectionWorkflowEdgeRow>,
    ) -> Result<()>;
    async fn list_collection_edges(
        &self,
        collection_id: Uuid,
    ) -> Result<Vec<CollectionWorkflowEdgeRow>>;
    async fn add_collection_edge(
        &self,
        collection_id: Uuid,
        from_workflow_id: Uuid,
        to_workflow_id: Uuid,
    ) -> Result<()>;
    async fn remove_collection_edge(
        &self,
        collection_id: Uuid,
        from_workflow_id: Uuid,
        to_workflow_id: Uuid,
    ) -> Result<()>;

    // --- Collection Runs (Execution Tracking) ---
    async fn create_collection_run(
        &self,
        collection_id: Uuid,
        user_id: Uuid,
    ) -> Result<CollectionRunRow>;
    async fn get_collection_run(&self, id: Uuid) -> Result<Option<CollectionRunRow>>;
    async fn list_collection_runs(&self, collection_id: Uuid) -> Result<Vec<CollectionRunRow>>;
    async fn update_collection_run_status(
        &self,
        id: Uuid,
        status: &str,
        error: Option<String>,
    ) -> Result<CollectionRunRow>;

    // --- Workflow Executions (Workflow-level execution within a collection run) ---
    async fn create_workflow_execution(
        &self,
        collection_run_id: Uuid,
        workflow_id: Uuid,
        user_id: Uuid,
    ) -> Result<WorkflowExecutionRow>;
    async fn get_workflow_execution(&self, id: Uuid) -> Result<Option<WorkflowExecutionRow>>;
    async fn list_workflow_executions(
        &self,
        collection_run_id: Uuid,
    ) -> Result<Vec<WorkflowExecutionRow>>;
    async fn update_workflow_execution_status(
        &self,
        id: Uuid,
        status: &str,
        outputs: Option<serde_json::Value>,
        error: Option<String>,
    ) -> Result<WorkflowExecutionRow>;

    async fn list_workflow_executions_by_workflow(
        &self,
        workflow_id: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<WorkflowExecutionRow>>;

    // --- Standalone Workflow Execution (no collection) ---
    async fn create_standalone_workflow_execution(
        &self,
        workflow_id: Uuid,
        user_id: Uuid,
    ) -> Result<WorkflowExecutionRow>;

    /// List the full execution tree rooted at `root_id` (O(1) via root_execution_id index).
    async fn list_execution_tree(&self, root_id: Uuid) -> Result<Vec<WorkflowExecutionRow>>;

    // --- Workshop (persistent per-workflow execution context) ---
    async fn get_or_create_workshop(
        &self,
        workflow_id: Uuid,
        user_id: Uuid,
    ) -> Result<WorkflowExecutionRow>;
}

// ============================================================================
// Workflow Step Agent Repository
// ============================================================================

/// Database operations for workflow step agents (multi-agent support).
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait WorkflowStepAgentRepo: Send + Sync {
    /// Add an agent to a workflow step.
    async fn add_step_agent(
        &self,
        step_id: Uuid,
        agent_id: Uuid,
        execution_strategy: String,
        agent_order: i32,
    ) -> Result<WorkflowStepAgentRow>;

    /// List all agents for a workflow step.
    async fn list_step_agents(&self, step_id: Uuid) -> Result<Vec<WorkflowStepAgentRow>>;

    /// Remove an agent from a workflow step.
    async fn remove_step_agent(&self, step_id: Uuid, agent_id: Uuid) -> Result<()>;

    /// Update agent configuration for a step.
    async fn update_step_agent(
        &self,
        step_id: Uuid,
        agent_id: Uuid,
        execution_strategy: Option<String>,
        agent_order: Option<i32>,
    ) -> Result<WorkflowStepAgentRow>;

    /// Replace all agents for a step (for bulk updates).
    async fn set_step_agents(&self, step_id: Uuid, agents: Vec<WorkflowStepAgentRow>)
        -> Result<()>;
}
