//! Database initialization and connection management

#[cfg(test)]
pub mod fixtures;
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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
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
    pub output_schema_id: Option<Uuid>,
    pub version: i32,
    pub default_reasoning_trace: Option<bool>,
    pub is_system: bool,
}

/// Row type for protocol document definitions (workforce step config).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
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
    /// Agent roster entry that produces this deliverable (workforce archetype).
    pub agent_roster_entry_id: Option<Uuid>,
}

/// Row type for protocol execution audit trail (protocol hidden phases).
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
    /// Human-readable agent name (e.g. "Scanner") for workforce agent phases.
    pub agent_name: Option<String>,
    /// Protocol archetype that produced this phase (e.g. "workforce").
    pub archetype: Option<String>,
    /// Links agent phases back to the designer run that created them.
    pub designer_run_id: Option<Uuid>,
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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
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
    pub board_overview_summary: String,
}

/// Row type for a workflow step (DAG node).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct WorkflowStepRow {
    pub id: Uuid,
    pub workflow_id: Uuid,
    pub agent_id: Option<Uuid>,
    pub execution_mode: String, // "single", "workforce", "context", "input", "sub_workflow", "container"
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
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub name: Option<String>,
    pub system_prompt_suffix: Option<String>,
    pub visible: bool,
    pub description: String,
    // Board context: Haiku-distilled awareness of the workflow board
    pub board_context_cache: String,
    pub board_context_updated_at: Option<DateTime<Utc>>,
    pub goal_summary: String,
    pub goal_summary_updated_at: Option<DateTime<Utc>>,
    /// Template to execute as a child workflow (sub_workflow execution mode).
    pub sub_workflow_template_id: Option<Uuid>,
    /// Live child workflow for workforce steps (edited at design time, snapshotted at execution).
    pub child_workflow_id: Option<Uuid>,
    /// Stable readable identifier for LLM-facing references (e.g. "workforce-1").
    pub ref_id: Option<String>,
    /// Whether this step's output is frozen (replayed instead of re-executed).
    pub pinned: bool,
    /// Haiku-generated summary of this step's last execution output.
    pub run_results_summary: String,
}

/// Row type for a workflow step edge (DAG edge).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
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
    pub execution_mode: String,
    pub template_id: Option<Uuid>,
    /// Parent execution for sub-workflow nesting.
    pub parent_execution_id: Option<Uuid>,
    /// Root execution for O(1) tree traversal (Temporal pattern).
    pub root_execution_id: Option<Uuid>,
    /// Nesting depth: 0 = top-level, 1 = first sub-workflow, etc.
    pub depth: i32,
}

/// Row type for run templates (frozen workflow snapshots).
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct RunTemplateRow {
    pub id: Uuid,
    pub workflow_id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub snapshot: serde_json::Value,
    pub created_at: DateTime<Utc>,
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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
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
    pub protocol_type: String, // e.g. "workforce"
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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct WorkflowStepProtocolRow {
    pub id: Uuid,
    pub workflow_step_id: Uuid,
    pub protocol_id: Uuid,
    pub applied_expansion: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

// ============================================================================
// Workforce Row Types
// ============================================================================

/// Row type for workforce mission briefs (one per step).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct TaskMissionBriefRow {
    pub id: Uuid,
    pub step_id: Uuid,
    pub task_description: String,
    pub available_capabilities: Vec<String>,
    pub failure_mode: String,
    pub downstream_context: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Row type for task force agent roster entries.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct TaskAgentRosterRow {
    pub id: Uuid,
    pub mission_brief_id: Uuid,
    pub name: String,
    pub role_description: String,
    pub capabilities: Vec<String>,
    pub execution_order: i32,
    pub created_at: DateTime<Utc>,
    /// Corresponding visual step in the workforce child workflow.
    pub child_step_id: Option<Uuid>,
}

// ============================================================================
// Agent Designer Row Types
// ============================================================================

/// Row type for agent designer execution runs.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct AgentDesignerRunRow {
    pub id: Uuid,
    pub workflow_execution_id: Uuid,
    pub stage_execution_id: Uuid,
    pub step_id: Uuid,
    pub mission_brief_id: Option<Uuid>,
    pub archetype: String,
    pub phase: String,
    pub model_id: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cost_usd: f32,
    pub created_at: DateTime<Utc>,
}

/// Row type for designer-generated agent prompt outputs.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct AgentDesignerOutputRow {
    pub id: Uuid,
    pub designer_run_id: Uuid,
    pub agent_roster_entry_id: Option<Uuid>,
    pub agent_name: String,
    pub assigned_tools: Vec<String>,
    pub generated_system_prompt: String,
    pub generated_task_prompt: String,
    pub design_reasoning: String,
    pub execution_order: i32,
    pub source_entity_id: String,
    pub source_archetype: String,
    /// Direct link to the protocol execution phase that triggered this designer output.
    pub protocol_execution_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

// ============================================================================
// Belief Capture Row Types
// ============================================================================

/// Row type for belief extraction plans (one per step, design-time config).
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct BeliefExtractionPlanRow {
    pub id: Uuid,
    pub step_id: Uuid,
    pub extraction_focus: String,
    pub tag_vocabulary: Vec<String>,
    pub contradiction_handling: String,
    pub confidence_threshold: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Row type for extracted beliefs (populated at runtime by the gatekeeper).
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct BeliefRow {
    pub id: Uuid,
    pub workflow_id: Uuid,
    pub workflow_execution_id: Option<Uuid>,
    pub source_step_id: Uuid,
    pub source_document_title: Option<String>,
    pub source_document_def_id: Option<Uuid>,
    pub source_phase: String,
    pub content: String,
    pub reasoning: String,
    pub belief_type: String,
    pub confidence: String,
    pub confidence_justification: Option<String>,
    pub semantic_tags: Vec<String>,
    pub emotional_tone: Option<String>,
    pub cross_source_tension: Option<String>,
    pub source_step_name: String,
    pub extraction_model: String,
    pub extraction_tokens_in: i32,
    pub extraction_tokens_out: i32,
    pub created_at: DateTime<Utc>,
}

// ============================================================================
// Room Step Config Row Types (Design-Time)
// ============================================================================

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct RoomStepConfigRow {
    pub id: Uuid,
    pub step_id: Uuid,
    pub meeting_purpose: String,
    pub max_turns: i32,
    pub interaction_mode: String,
    pub gatekeeper_enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct RoomStepMemberRow {
    pub id: Uuid,
    pub step_id: Uuid,
    pub name: String,
    pub role: String,
    pub perspective: String,
    pub display_order: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Row type for immutable content version snapshots.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct ContentVersionRow {
    pub id: Uuid,
    pub source_id: Uuid,
    pub content_type: String,
    pub content_hash: String,
    pub content: String,
    pub version_number: i32,
    pub byte_size: i32,
    pub created_at: DateTime<Utc>,
}

/// Row type for run snapshot linkage (run → content version).
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct RunSnapshotRow {
    pub id: Uuid,
    pub run_id: Uuid,
    pub step_id: Uuid,
    pub content_type: String,
    pub role: String,
    pub content_version_id: Uuid,
    pub source_id: Uuid,
    pub created_at: DateTime<Utc>,
}

/// Lightweight row for reconstructing envelopes from snapshots (JOIN result).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct EnvelopeSnapshotRow {
    pub step_id: Uuid,
    pub content: String,
    pub source_id: Uuid,
}

/// Row type for persisted canvas snapshots (one per workflow, upserted on board submit).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CanvasSnapshotRow {
    pub workflow_id: Uuid,
    pub snapshot_json: String,
    pub elements_json: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Maps Excalidraw element IDs to workflow step or edge UUIDs.
/// Exactly one of `step_id` or `edge_id` is populated (XOR constraint in DB).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CanvasElementMapRow {
    pub workflow_id: Uuid,
    pub element_id: String,
    pub step_id: Option<Uuid>,
    pub edge_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

// ============================================================================
// Default Implementations
// ============================================================================
//
// Manual `impl Default` (not derive) because DateTime<Utc> doesn't implement
// Default and some fields need non-zero defaults (e.g. visible=true, version=1).
// These enable `..Default::default()` spread in tests so that adding a new
// Option field to a struct requires zero test file changes.

impl Default for WorkflowStepRow {
    fn default() -> Self {
        Self {
            id: Uuid::nil(),
            workflow_id: Uuid::nil(),
            agent_id: None,
            execution_mode: "single".to_string(),
            agent_execution_mode: None,
            for_each_ref: None,
            prompt_template_id: None,
            prompt_template: String::new(),
            output_schema_id: None,
            output_variable_name: None,
            interactive_agent_id: None,
            for_each_label_field: None,
            room_id: None,
            routing_mode: None,
            routing_field: None,
            display_order: 0,
            version: 1,
            reasoning_trace: false,
            verification_agent_ids: None,
            position_x: None,
            position_y: None,
            width: None,
            height: None,
            name: None,
            system_prompt_suffix: None,
            visible: true,
            description: String::new(),
            board_context_cache: String::new(),
            board_context_updated_at: None,
            goal_summary: String::new(),
            goal_summary_updated_at: None,
            sub_workflow_template_id: None,
            child_workflow_id: None,
            ref_id: None,
            pinned: false,
            run_results_summary: String::new(),
        }
    }
}

impl Default for WorkflowStepEdgeRow {
    fn default() -> Self {
        Self {
            id: Uuid::nil(),
            from_step_id: Uuid::nil(),
            to_step_id: Uuid::nil(),
            from_output_port: None,
            to_input_port: None,
            transform_jsonpath: None,
            condition_type: None,
            condition_value: None,
            edge_label: None,
            workflow_id: Uuid::nil(),
        }
    }
}

impl Default for AgentRow {
    fn default() -> Self {
        Self {
            id: Uuid::nil(),
            user_id: None,
            tier: None,
            name: String::new(),
            system_prompt: String::new(),
            persona_style: None,
            model_provider: crate::constants::ACTIVE_PROVIDER.to_string(),
            model_id: crate::constants::MODEL_TIER2.to_string(),
            model_max_tokens: 4096,
            model_temperature: 0.7,
            status: None,
            output_schema_id: None,
            version: 1,
            default_reasoning_trace: None,
            is_system: false,
        }
    }
}

impl Default for WorkflowRow {
    fn default() -> Self {
        Self {
            id: Uuid::nil(),
            user_id: Uuid::nil(),
            name: String::new(),
            description: String::new(),
            execution_mode: "dag".to_string(),
            created_at: Utc::now(),
            version: 1,
            container_enabled: false,
            target_repo_url: None,
            target_branch: None,
            vpn_enabled: false,
            board_overview_summary: String::new(),
        }
    }
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
            parent_execution_id: None,
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

impl Default for TaskMissionBriefRow {
    fn default() -> Self {
        Self {
            id: Uuid::nil(),
            step_id: Uuid::nil(),
            task_description: String::new(),
            available_capabilities: vec![],
            failure_mode: "fail_fast".to_string(),
            downstream_context: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

impl Default for TaskAgentRosterRow {
    fn default() -> Self {
        Self {
            id: Uuid::nil(),
            mission_brief_id: Uuid::nil(),
            name: String::new(),
            role_description: String::new(),
            capabilities: vec![],
            execution_order: 0,
            created_at: Utc::now(),
            child_step_id: None,
        }
    }
}

impl Default for BeliefRow {
    fn default() -> Self {
        Self {
            id: Uuid::nil(),
            workflow_id: Uuid::nil(),
            workflow_execution_id: None,
            source_step_id: Uuid::nil(),
            source_document_title: None,
            source_document_def_id: None,
            source_phase: String::new(),
            content: String::new(),
            reasoning: String::new(),
            belief_type: String::new(),
            confidence: String::new(),
            confidence_justification: None,
            semantic_tags: vec![],
            emotional_tone: None,
            cross_source_tension: None,
            source_step_name: String::new(),
            extraction_model: String::new(),
            extraction_tokens_in: 0,
            extraction_tokens_out: 0,
            created_at: Utc::now(),
        }
    }
}

impl Default for StepInputRow {
    fn default() -> Self {
        Self {
            id: Uuid::nil(),
            workflow_step_id: Uuid::nil(),
            port_name: String::new(),
            port_type: "any".to_string(),
            required: false,
            default_value: None,
            description: None,
            json_schema: None,
            created_at: Utc::now(),
        }
    }
}

impl Default for StepOutputRow {
    fn default() -> Self {
        Self {
            id: Uuid::nil(),
            workflow_step_id: Uuid::nil(),
            port_name: String::new(),
            port_type: "any".to_string(),
            json_path: "$".to_string(),
            description: None,
            json_schema: None,
            created_at: Utc::now(),
        }
    }
}

/// Row type for step question state (compressed status + pending question).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct StepQuestionStateRow {
    pub step_id: Uuid,
    pub status_text: String,
    pub question_text: Option<String>,
    pub updated_at: DateTime<Utc>,
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
        .unwrap_or(50);
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
