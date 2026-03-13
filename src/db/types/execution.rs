use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Row type for workflow executions (workflow-level execution within a collection run).
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct WorkflowExecutionRow {
    pub id: Uuid,
    pub collection_run_id: Option<Uuid>,
    pub workflow_id: Uuid,
    pub user_id: Uuid,
    pub status: String,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub outputs: Option<serde_json::Value>,
    pub error: Option<String>,
    pub execution_mode: String,
    pub template_id: Option<Uuid>,
    /// Root execution for O(1) tree traversal (Temporal pattern).
    pub root_execution_id: Option<Uuid>,
    /// Nesting depth: 0 = top-level, 1 = first sub-workflow, etc.
    pub depth: i32,
}

/// Row type for agent execution records.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct AgentExecutionRow {
    pub id: Uuid,
    pub execution_type: String,
    pub agent_id: Option<Uuid>,
    pub workflow_step_id: Option<Uuid>,
    pub workflow_execution_id: Option<Uuid>,
    pub is_interactive: bool,
    pub parent_agent_execution_id: Option<Uuid>,
    pub system_prompt_rendered: String,
    pub input: String,
    pub output: Option<String>,
    pub structured_output: Option<serde_json::Value>,
    pub room_session_id: Option<Uuid>,
    pub speaker_order: Option<i32>,
    pub status: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    // Few-shot exemplar flag
    pub is_exemplary: bool,
    /// Serialized dispatch trace (tokens, tool calls, errors) for persistence.
    pub trace: Option<serde_json::Value>,
}

/// Row type for execution message records.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct ExecutionMessageRow {
    pub id: Uuid,
    pub agent_execution_id: Uuid,
    pub role: String,
    pub content: String,
    pub tool_call_id: Option<String>,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub created_at: DateTime<Utc>,
}

/// Row type for token ledger entries.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct TokenLedgerRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub agent_execution_id: Option<Uuid>,
    pub model_id: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cost_usd: f32,
    pub created_at: DateTime<Utc>,
}

/// Flat row for the execution timeline view — joins agent_executions + execution_messages + workflow_steps.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct TimelineRow {
    pub id: Uuid,
    pub ts: DateTime<Utc>,
    pub role: String,
    pub content: String,
    pub tool_call_id: Option<String>,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub agent_execution_id: Uuid,
    pub execution_type: String,
    pub step_id: Option<Uuid>,
    pub step_name: Option<String>,
    pub agent_name: Option<String>,
    pub agent_status: String,
}

impl Default for WorkflowExecutionRow {
    fn default() -> Self {
        Self {
            id: Uuid::nil(),
            collection_run_id: None,
            workflow_id: Uuid::nil(),
            user_id: Uuid::nil(),
            status: "pending".to_string(),
            started_at: None,
            completed_at: None,
            outputs: None,
            error: None,
            execution_mode: "dag".to_string(),
            template_id: None,
            root_execution_id: None,
            depth: 0,
        }
    }
}

impl Default for AgentExecutionRow {
    fn default() -> Self {
        Self {
            id: Uuid::nil(),
            execution_type: "dag_step".to_string(),
            agent_id: None,
            workflow_step_id: None,
            workflow_execution_id: None,
            is_interactive: false,
            parent_agent_execution_id: None,
            system_prompt_rendered: String::new(),
            input: String::new(),
            output: None,
            structured_output: None,
            room_session_id: None,
            speaker_order: None,
            status: "pending".to_string(),
            started_at: Utc::now(),
            completed_at: None,
            is_exemplary: false,
            trace: None,
        }
    }
}

impl Default for ExecutionMessageRow {
    fn default() -> Self {
        Self {
            id: Uuid::nil(),
            agent_execution_id: Uuid::nil(),
            role: "user".to_string(),
            content: String::new(),
            tool_call_id: None,
            input_tokens: 0,
            output_tokens: 0,
            created_at: Utc::now(),
        }
    }
}

impl Default for TokenLedgerRow {
    fn default() -> Self {
        Self {
            id: Uuid::nil(),
            user_id: Uuid::nil(),
            agent_execution_id: None,
            model_id: "test".to_string(),
            input_tokens: 0,
            output_tokens: 0,
            cost_usd: 0.0,
            created_at: Utc::now(),
        }
    }
}
