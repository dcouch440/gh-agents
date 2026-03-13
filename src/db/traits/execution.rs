use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::db::{AgentExecutionRow, ExecutionMessageRow, ResultRow, TimelineRow, TokenLedgerRow};

// ============================================================================
// Agent Execution Repository
// ============================================================================

/// Input for creating an agent execution record.
#[derive(Debug, Clone)]
pub struct CreateAgentExecutionInput {
    pub execution_type: crate::types::ExecutionType,
    pub agent_id: Option<Uuid>,
    pub workflow_step_id: Option<Uuid>,
    pub parent_agent_execution_id: Option<Uuid>,
    pub system_prompt_rendered: String,
    pub input: String,
    pub room_session_id: Option<Uuid>,
    pub speaker_order: Option<i32>,
    pub workflow_execution_id: Option<Uuid>,
}

/// Database operations for agent executions and execution messages.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait AgentExecutionRepo: Send + Sync {
    // --- Agent Executions ---
    async fn create_agent_execution(
        &self,
        input: CreateAgentExecutionInput,
    ) -> Result<AgentExecutionRow>;
    async fn get_agent_execution(&self, id: Uuid) -> Result<Option<AgentExecutionRow>>;
    async fn update_agent_execution_status(
        &self,
        id: Uuid,
        status: &str,
        output: Option<String>,
        structured_output: Option<serde_json::Value>,
    ) -> Result<AgentExecutionRow>;

    // --- Execution Messages ---
    async fn create_execution_message(
        &self,
        agent_execution_id: Uuid,
        role: &str,
        content: &str,
        tool_call_id: Option<String>,
        input_tokens: i64,
        output_tokens: i64,
    ) -> Result<ExecutionMessageRow>;
    async fn list_execution_messages(
        &self,
        agent_execution_id: Uuid,
    ) -> Result<Vec<ExecutionMessageRow>>;

    /// List completed non-review agent executions for a set of workflow step IDs.
    /// Filters out `execution_type = 'interactive_review'`. Used to reconstruct DAG state on resume.
    async fn list_completed_executions_for_step_ids(
        &self,
        workflow_step_ids: &[Uuid],
    ) -> Result<Vec<AgentExecutionRow>>;

    /// List interactive review executions (`execution_type = 'interactive_review'`) for a step.
    /// Used to check if all reviews are approved before resuming the DAG.
    async fn list_interactive_executions_for_step(
        &self,
        workflow_step_id: Uuid,
    ) -> Result<Vec<AgentExecutionRow>>;

    /// List interactive review executions for a user, optionally filtered by status.
    /// Matches `execution_type = 'interactive_review'`. Joins through workflow_executions.
    async fn list_agent_executions(
        &self,
        user_id: Uuid,
        status: Option<String>,
    ) -> Result<Vec<AgentExecutionRow>>;

    /// List agent executions for a specific step within a specific workflow run.
    async fn list_agent_executions_for_step_and_run(
        &self,
        workflow_step_id: Uuid,
        workflow_execution_id: Uuid,
    ) -> Result<Vec<AgentExecutionRow>>;

    /// List completed executions marked as exemplary for few-shot injection.
    /// Returns rows ordered by most recent, limited to `limit`.
    async fn list_exemplary_executions(
        &self,
        agent_id: Uuid,
        workflow_step_id: Option<Uuid>,
        limit: u32,
    ) -> Result<Vec<AgentExecutionRow>>;

    /// Toggle the exemplary flag on an execution.
    async fn set_execution_exemplary(
        &self,
        id: Uuid,
        is_exemplary: bool,
    ) -> Result<AgentExecutionRow>;

    /// Update the trace JSONB column on an agent execution.
    async fn update_execution_trace(&self, id: Uuid, trace: serde_json::Value) -> Result<()>;

    /// Get the latest dispatch execution for a workflow step.
    /// Matches `execution_type IN ('dispatch', 'manager_dispatch')`.
    async fn get_latest_dispatch_execution_for_step(
        &self,
        step_id: Uuid,
    ) -> Result<Option<AgentExecutionRow>>;

    /// List a unified execution timeline for a workflow run.
    /// Joins agent_executions + execution_messages + workflow_steps into a flat
    /// chronological stream. Cursor-based pagination: returns entries with
    /// `ts < before`, limited to `limit`, ordered newest-first for pagination
    /// (caller reverses for display).
    async fn list_execution_timeline(
        &self,
        workflow_execution_id: Uuid,
        limit: i64,
        before: Option<DateTime<Utc>>,
    ) -> Result<Vec<TimelineRow>>;
}

// ============================================================================
// Token Ledger Repository
// ============================================================================

/// Aggregated spend by model.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct ModelSpendRow {
    pub model_id: String,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub total_cost_usd: f64,
    pub call_count: i64,
}

/// Database operations for token cost tracking.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait TokenLedgerRepo: Send + Sync {
    async fn insert_ledger_entry(
        &self,
        user_id: Uuid,
        agent_execution_id: Option<Uuid>,
        model_id: &str,
        input_tokens: i64,
        output_tokens: i64,
        cost_usd: f32,
    ) -> Result<TokenLedgerRow>;
    async fn get_user_spend(&self, user_id: Uuid, since: Option<DateTime<Utc>>) -> Result<f64>;
    async fn get_model_breakdown(
        &self,
        user_id: Uuid,
        since: Option<DateTime<Utc>>,
    ) -> Result<Vec<ModelSpendRow>>;
}

// ============================================================================
// Result Repository
// ============================================================================

/// Database operations for saved structured results.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait ResultRepo: Send + Sync {
    async fn save_result(
        &self,
        user_id: Uuid,
        agent_execution_id: Uuid,
        output_schema_id: Option<Uuid>,
        name: &str,
        data: serde_json::Value,
    ) -> Result<ResultRow>;
    async fn get_result(&self, id: Uuid) -> Result<Option<ResultRow>>;
    async fn list_results(&self, user_id: Uuid) -> Result<Vec<ResultRow>>;
    async fn list_results_by_schema(
        &self,
        user_id: Uuid,
        output_schema_id: Uuid,
    ) -> Result<Vec<ResultRow>>;
    async fn delete_result(&self, id: Uuid) -> Result<()>;
}
