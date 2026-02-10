//! Database initialization and connection management

pub mod pg_repo;
mod queries;
#[cfg(test)]
pub mod test_utils;
pub mod traits;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
pub use queries::*;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use uuid::Uuid;

/// Row type for persisted agent definitions.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct AgentRow {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub tier: Option<String>,
    pub name: String,
    pub system_prompt: String,
    pub persona_style: Option<String>,
    pub model_provider: String,
    pub model_id: String,
    pub model_max_tokens: i32,
    pub model_temperature: f32,
    pub status: Option<String>,
    pub router_mode: Option<bool>,
    pub router_id: Option<Uuid>,
    pub output_schema_id: Option<Uuid>,
    pub version: i32,
    pub default_reasoning_trace: Option<bool>,
}

/// Row type for protocol document definitions (documenter step config).
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct ProtocolDocumentDefRow {
    pub id: Uuid,
    pub step_id: Option<Uuid>,
    pub name: String,
    pub description: String,
    pub target_length: i32,
    pub display_order: i32,
    pub created_at: DateTime<Utc>,
    pub protocol_id: Option<Uuid>,
    pub document_id: Option<Uuid>,
}

/// Row type for protocol execution audit trail (documenter hidden phases).
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct ProtocolExecutionRow {
    pub id: Uuid,
    pub protocol_step_id: Uuid,
    pub workflow_run_id: Option<Uuid>,
    pub phase: String,
    pub document_def_id: Option<Uuid>,
    pub agent_id: Option<Uuid>,
    pub input_prompt: Option<String>,
    pub output_content: Option<String>,
    pub status: String,
    pub error_message: Option<String>,
    pub tokens_in: Option<i32>,
    pub tokens_out: Option<i32>,
    pub cost_usd: Option<f64>,
    pub model: Option<String>,
    pub capabilities_used: Option<Vec<String>>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Row type for agent guidance (distilled feedback / learned instructions).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AgentGuidanceRow {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub workflow_step_id: Option<Uuid>,
    pub suggestions: serde_json::Value,
    pub source: String,
    pub version: i32,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Row type for persisted tool definitions.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ToolRow {
    pub id: Uuid,
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub parameters: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub version: i32,
}

/// Row type for persisted documents.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct DocumentRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub session_id: Option<Uuid>,
    pub title: String,
    pub content: String,
    pub summary: Option<String>,
    pub doc_type: Option<String>,
    pub ref_tag: Option<String>,
    pub tags: Option<Vec<String>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub workflow_id: Option<Uuid>,
    pub target_length: Option<i32>,
    pub is_static: Option<bool>,
    pub source_protocol_step_id: Option<Uuid>,
}

/// Search result for documents (no full content).
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct DocumentSearchResult {
    pub id: Uuid,
    pub title: String,
    pub summary: Option<String>,
    pub ref_tag: Option<String>,
    pub snippet: String,
}

/// Row type for persisted output schema definitions.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct OutputSchemaRow {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub name: String,
    pub schema: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub version: i32,
}

/// Row type for persisted prompt template definitions.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct PromptTemplateRow {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub name: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub version: i32,
}

/// Row type for persisted workflow definitions.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct WorkflowRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub description: String,
    pub execution_mode: String,
    pub created_at: DateTime<Utc>,
    pub version: i32,
    pub container_enabled: bool,
    pub target_repo_url: Option<String>,
    pub target_branch: Option<String>,
    pub vpn_enabled: bool,
}

/// Row type for a workflow step (DAG node).
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct WorkflowStepRow {
    pub id: Uuid,
    pub workflow_id: Uuid,
    pub agent_id: Option<Uuid>,
    pub execution_mode: String, // "single", "for_each", "room", "documenter", etc.
    pub agent_execution_mode: Option<String>, // "sequential" or "parallel", NULL = inherit from workflow
    pub for_each_ref: Option<String>,
    pub prompt_template_id: Option<Uuid>,
    pub prompt_template: String,
    pub output_schema_id: Option<Uuid>,
    pub output_variable_name: Option<String>,
    pub interactive_agent_id: Option<Uuid>,
    pub for_each_label_field: Option<String>,
    pub room_id: Option<Uuid>,
    pub routing_mode: Option<String>,
    pub routing_field: Option<String>,
    pub display_order: i32,
    pub version: i32,
    pub reasoning_trace: bool,
    pub verification_agent_ids: Option<serde_json::Value>,
    pub position_x: Option<f64>,
    pub position_y: Option<f64>,
    pub name: Option<String>,
    pub system_prompt_suffix: Option<String>,
    pub visible: bool,
}

/// Row type for a workflow step edge (DAG edge).
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct WorkflowStepEdgeRow {
    pub id: Uuid,
    pub from_step_id: Uuid,
    pub to_step_id: Uuid,
    pub from_output_port: Option<String>,
    pub to_input_port: Option<String>,
    pub transform_jsonpath: Option<String>,
    pub condition_type: Option<String>,
    pub condition_value: Option<serde_json::Value>,
    pub edge_label: Option<String>,
    pub workflow_id: Uuid,
}

/// Row type for workflow collections (DAG of workflows).
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct WorkflowCollectionRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub execution_mode: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Row type for collection workflows (which workflows belong to a collection).
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct CollectionWorkflowRow {
    pub collection_id: Uuid,
    pub workflow_id: Uuid,
    pub display_order: i32,
    pub execution_mode: Option<String>,
}

/// Row type for collection workflow edges (DAG edges between workflows).
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct CollectionWorkflowEdgeRow {
    pub from_workflow_id: Uuid,
    pub to_workflow_id: Uuid,
    pub collection_id: Uuid,
}

/// Row type for collection runs (execution tracking).
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct CollectionRunRow {
    pub id: Uuid,
    pub collection_id: Uuid,
    pub user_id: Uuid,
    pub status: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
}

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
}

/// Row type for workflow step agents (multi-agent step support).
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct WorkflowStepAgentRow {
    pub step_id: Uuid,
    pub agent_id: Uuid,
    pub execution_strategy: String,
    pub agent_order: i32,
}

/// Row type for a step-document attachment.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct StepDocumentRow {
    pub step_id: Uuid,
    pub document_id: Uuid,
}

/// Row type for agent execution records.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct AgentExecutionRow {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub workflow_step_id: Option<Uuid>,
    pub workflow_execution_id: Option<Uuid>,
    pub is_interactive: bool,
    pub parent_agent_execution_id: Option<Uuid>,
    pub system_prompt_rendered: String,
    pub input: String,
    pub output: Option<String>,
    pub structured_output: Option<serde_json::Value>,
    pub selected_mode_id: Option<Uuid>,
    pub room_session_id: Option<Uuid>,
    pub speaker_order: Option<i32>,
    pub status: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    // Cavernous routing fields
    pub routing_analysis: Option<serde_json::Value>,
    pub selected_routing_document_id: Option<Uuid>,
    // Few-shot exemplar flag
    pub is_exemplary: bool,
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

/// Row type for saved structured results.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct ResultRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub agent_execution_id: Uuid,
    pub output_schema_id: Option<Uuid>,
    pub name: String,
    pub data: serde_json::Value,
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

/// Row type for tool router definitions.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct ToolRouterRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub system_prompt: String,
    pub model_id: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub parent_router_id: Option<Uuid>,
    pub level: i32,
}

/// Row type for tool router mode definitions.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct ToolRouterModeRow {
    pub id: Uuid,
    pub router_id: Uuid,
    pub mode_key: String,
    pub display_name: String,
    pub description: String,
    pub system_prompt: String,
    pub temperature: f32,
    pub max_tokens: i32,
    pub append_to_agent_system_prompt: bool,
    pub append_to_agent_tools: bool,
    pub display_order: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Row type for context store entries.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct ContextStoreRow {
    pub id: Uuid,
    pub session_id: Uuid,
    pub source: String,
    pub priority: f32,
    pub content: String,
    pub metadata: Option<serde_json::Value>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

/// Row type for router request logs.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct RouterRequestRow {
    pub id: Uuid,
    pub session_id: Uuid,
    pub agent_execution_id: Option<Uuid>,
    pub intent: String,
    pub priority: String,
    pub callback_hint: Option<String>,
    pub routed_tool: Option<String>,
    pub routed_args: Option<serde_json::Value>,
    pub is_async: bool,
    pub passdown: Option<String>,
    pub chain: Option<serde_json::Value>,
    pub status: String,
    pub result: Option<String>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Row type for room definitions (pipeline-scoped).
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct RoomRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub collection_id: Option<Uuid>,
    pub name: String,
    pub gatekeeper_enabled: bool,
    pub gatekeeper_model_id: String,
    pub max_speakers_per_turn: i32,
    pub max_turns: i32,
    pub tools_enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Row type for room membership (join table).
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct RoomMemberRow {
    pub room_id: Uuid,
    pub agent_id: Uuid,
    pub display_name: Option<String>,
    pub role_description: String,
    pub display_order: i32,
}

/// Row type for room session records (runtime).
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct RoomSessionRow {
    pub id: Uuid,
    pub room_id: Uuid,
    pub status: String,
    pub current_turn: i32,
    pub transcript_summary: Option<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub structured_outputs: Option<serde_json::Value>,
    pub final_decision: Option<serde_json::Value>,
}

/// Labeled entry from a room transcript (cross-execution join).
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct RoomTranscriptEntry {
    pub agent_name: String,
    pub role_description: String,
    pub content: String,
    pub speaker_order: Option<i32>,
    pub created_at: DateTime<Utc>,
}

// ============================================================================
// Phase 2: Port-Based Workflow Row Types
// ============================================================================

/// Input port definition for workflow steps
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct StepInputRow {
    pub id: Uuid,
    pub workflow_step_id: Uuid,
    pub port_name: String,
    pub port_type: String,
    pub required: bool,
    pub default_value: Option<serde_json::Value>,
    pub description: Option<String>,
    pub json_schema: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

/// Output port definition for workflow steps
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct StepOutputRow {
    pub id: Uuid,
    pub workflow_step_id: Uuid,
    pub port_name: String,
    pub port_type: String,
    pub json_path: String,
    pub description: Option<String>,
    pub json_schema: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

/// Routing rule for label-based agent assignment
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct StepRoutingRuleRow {
    pub id: Uuid,
    pub workflow_step_id: Uuid,
    pub label_value: String,
    pub description: Option<String>,
    pub agent_id: Uuid,
    pub display_order: i32,
    pub created_at: DateTime<Utc>,
}

// ============================================================================
// Phase 2: Tool Capability Row Types
// ============================================================================

/// Tool capability taxonomy (semantic capabilities)
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct ToolCapabilityRow {
    pub id: Uuid,
    pub capability_key: String,
    pub display_name: String,
    pub category: String,
    pub safety_level: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
}

/// Tool-to-capability assignment (which capabilities each tool provides)
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct ToolCapabilityAssignmentRow {
    pub tool_id: Uuid,
    pub capability_id: Uuid,
}

/// Mode-to-capability requirement (which capabilities each mode requires)
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct ModeRequiredCapabilityRow {
    pub mode_id: Uuid,
    pub capability_id: Uuid,
    pub is_required: bool,
}

// ============================================================================
// Phase 2: Room Execution Row Types
// ============================================================================

/// Structured output from a room member for agent-to-agent data passing
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct RoomExecutionOutputRow {
    pub id: Uuid,
    pub room_session_id: Uuid,
    pub agent_execution_id: Uuid,
    pub agent_id: Uuid,
    pub speaker_order: i32,
    pub turn_number: i32,
    pub output_name: String,
    pub structured_output: serde_json::Value,
    pub raw_output: String,
    pub schema_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

// ============================================================================
// Phase 3: System Config Row Type
// ============================================================================

/// System configuration entry (admin-controlled)
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct SystemConfigRow {
    pub id: Uuid,
    pub config_type: String,
    pub config_key: String,
    pub config_value: serde_json::Value,
    pub description: Option<String>,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ============================================================================
// Protocol Layer Row Types
// ============================================================================

/// Row type for protocol definitions (reusable execution recipes).
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct ProtocolRow {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub protocol_type: String, // "decomp", "transform", "review", "route", "default"
    pub config: serde_json::Value,
    pub version: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub agent_id: Option<Uuid>,
    pub output_schema_id: Option<Uuid>,
    pub prompt_template_id: Option<Uuid>,
}

/// Row type for protocol port slots (agent assignments within a protocol).
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct ProtocolPortRow {
    pub id: Uuid,
    pub protocol_id: Uuid,
    pub port_name: String,
    pub description: String,
    pub agent_id: Uuid,
    pub display_order: i32,
}

/// Row type for workflow step ↔ protocol linkage.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct WorkflowStepProtocolRow {
    pub id: Uuid,
    pub workflow_step_id: Uuid,
    pub protocol_id: Uuid,
    pub applied_expansion: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

/// Type alias for the database pool
pub type DbPool = PgPool;

/// Initialize the database using DATABASE_URL from environment
pub async fn init_db() -> Result<PgPool> {
    let database_url = std::env::var(crate::constants::ENV_DATABASE_URL).context(format!(
        "{} environment variable not set",
        crate::constants::ENV_DATABASE_URL
    ))?;
    init_db_with_url(&database_url).await
}

/// Initialize the database with an explicit URL
pub async fn init_db_with_url(database_url: &str) -> Result<PgPool> {
    let max_connections: u32 = std::env::var(crate::constants::ENV_DB_MAX_CONNECTIONS)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);
    tracing::info!("DB pool max_connections = {}", max_connections);

    let pool = PgPoolOptions::new()
        .max_connections(max_connections)
        .connect(database_url)
        .await
        .with_context(|| format!("Failed to connect to database at {}", database_url))?;

    tracing::info!("Database connected to PostgreSQL");

    // Run migrations
    sqlx::migrate!()
        .run(&pool)
        .await
        .context("Failed to run database migrations")?;

    tracing::info!("All migrations complete");

    Ok(pool)
}
