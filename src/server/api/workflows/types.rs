//! Request/response types and helpers for workflow endpoints

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ============================================================================
// Workflow Types
// ============================================================================

#[derive(Serialize, utoipa::ToSchema)]
pub struct WorkflowResponse {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub container_enabled: bool,
    pub target_repo_url: Option<String>,
    pub target_branch: Option<String>,
    pub vpn_enabled: bool,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateWorkflowRequest {
    pub name: String,
    pub description: Option<String>,
    pub container_enabled: Option<bool>,
    pub target_repo_url: Option<String>,
    pub target_branch: Option<String>,
    pub vpn_enabled: Option<bool>,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpdateWorkflowRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub container_enabled: Option<bool>,
    pub target_repo_url: Option<Option<String>>,
    pub target_branch: Option<Option<String>>,
    pub vpn_enabled: Option<bool>,
}

// ============================================================================
// Workflow Step Types
// ============================================================================

#[derive(Serialize, utoipa::ToSchema)]
pub struct WorkflowStepResponse {
    pub id: Uuid,
    pub workflow_id: Uuid,
    pub agent_id: Option<Uuid>,
    pub execution_mode: String,
    pub for_each_ref: Option<String>,
    pub prompt_template_id: Option<Uuid>,
    pub prompt_template: String,
    pub output_schema_id: Option<Uuid>,
    pub output_variable_name: Option<String>,
    pub interactive_agent_id: Option<Uuid>,
    pub for_each_label_field: Option<String>,
    pub display_order: i32,
    pub version: i32,
    pub reasoning_trace: bool,
    pub verification_agent_ids: Vec<Uuid>,
    pub position_x: Option<f64>,
    pub position_y: Option<f64>,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub name: Option<String>,
    pub system_prompt_suffix: Option<String>,
    pub room_id: Option<Uuid>,
    pub visible: bool,
    pub description: String,
    pub sub_workflow_template_id: Option<Uuid>,
    pub pinned: bool,
    pub run_results_summary: String,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateStepRequest {
    #[serde(flatten)]
    pub payload: crate::server::services::steps::StepPayload,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpdateStepRequest {
    #[serde(flatten)]
    pub payload: crate::server::services::steps::StepPayload,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct WorkflowStepPath {
    pub wid: Uuid,
    pub sid: Uuid,
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct TogglePinRequest {
    pub pinned: bool,
}

// ============================================================================
// Edge Types
// ============================================================================

#[derive(Deserialize, utoipa::ToSchema)]
pub struct EdgeRequest {
    pub from_step_id: Uuid,
    pub to_step_id: Uuid,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct EdgeResponse {
    pub id: Uuid,
    pub from_step_id: Uuid,
    pub to_step_id: Uuid,
}

// ============================================================================
// Step Document Types
// ============================================================================

#[derive(Deserialize, utoipa::ToSchema)]
pub struct StepDocumentRequest {
    pub document_id: Uuid,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct StepDocumentResponse {
    pub step_id: Uuid,
    pub document_id: Uuid,
}

// ============================================================================
// Run Workflow Types
// ============================================================================

#[derive(Deserialize, utoipa::ToSchema)]
pub struct RunWorkflowRequest {
    pub initial_input: Option<String>,
    pub template_id: Option<Uuid>,
}

// ============================================================================
// Run Template Types
// ============================================================================

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateTemplateRequest {
    #[schema(max_length = 200)]
    pub name: String,
    pub description: Option<String>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct RunTemplateResponse {
    pub id: Uuid,
    pub workflow_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct RunTemplateDetailResponse {
    pub id: Uuid,
    pub workflow_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub snapshot: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct TemplatePath {
    pub id: Uuid,
    pub template_id: Uuid,
}

// ============================================================================
// Rebase Types
// ============================================================================

#[derive(Deserialize, utoipa::ToSchema)]
pub struct RebaseRequest {
    pub template_id: Uuid,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct RebaseResponse {
    pub backup_template_id: Uuid,
    pub template_id: Uuid,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct WorkflowRunResponse {
    pub execution_id: Uuid,
    pub workflow_id: Uuid,
    pub status: String,
}

// ============================================================================
// Workflow Execution History Types
// ============================================================================

#[derive(Serialize, utoipa::ToSchema)]
pub struct WorkflowExecutionResponse {
    pub id: Uuid,
    pub workflow_id: Uuid,
    pub status: String,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub outputs: Option<serde_json::Value>,
    pub error: Option<String>,
    pub execution_mode: String,
    pub template_id: Option<Uuid>,
}

// ============================================================================
// Run Detail Types
// ============================================================================

#[derive(Serialize, utoipa::ToSchema)]
pub struct RunStepResultResponse {
    pub step_id: Uuid,
    pub step_name: Option<String>,
    pub execution_mode: String,
    pub execution_id: Option<String>,
    pub status: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub duration_ms: Option<u64>,
    pub output: Option<String>,
    pub structured_output: Option<serde_json::Value>,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub cost_usd: Option<f64>,
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phases: Option<Vec<super::last_run_handlers::PhaseExecution>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub child_execution_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub child_steps: Option<Vec<ChildStepResult>>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct ChildStepResult {
    pub step_name: Option<String>,
    pub execution_mode: String,
    pub status: String,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub duration_ms: Option<u64>,
    pub error: Option<String>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct RunDetailResponse {
    pub execution: WorkflowExecutionResponse,
    pub steps: Vec<RunStepResultResponse>,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub total_cost_usd: f64,
    pub duration_ms: Option<u64>,
    pub template_name: Option<String>,
}

#[derive(Deserialize)]
pub struct ExecutionStepPath {
    pub wid: Uuid,
    pub eid: Uuid,
    pub sid: Uuid,
}

#[derive(Deserialize)]
pub struct ExecutionPath {
    pub wid: Uuid,
    pub eid: Uuid,
}

// ============================================================================
// Workshop Types
// ============================================================================

#[derive(Deserialize, utoipa::ToSchema)]
pub struct CreateWorkshopRequest {
    pub initial_input: Option<String>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct WorkshopResponse {
    pub run_id: Uuid,
    pub workflow_id: Uuid,
    pub status: String,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct WorkshopStepResponse {
    pub step_id: Uuid,
    pub status: String,
    pub output: Option<serde_json::Value>,
    pub tokens_in: i64,
    pub tokens_out: i64,
    pub cost_usd: f32,
    pub duration_ms: u64,
    pub next_executable_steps: Vec<Uuid>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct WorkshopStatusResponse {
    pub run_id: Uuid,
    pub workflow_id: Uuid,
    pub status: String,
    pub completed_steps: Vec<WorkshopStepSummary>,
    pub next_executable_steps: Vec<Uuid>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct WorkshopStepSummary {
    pub step_id: Uuid,
    pub status: String,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
}

/// Path params for workshop step execution.
#[derive(Deserialize)]
pub struct WorkshopStepPath {
    pub id: Uuid,
    pub step_id: Uuid,
}

// ============================================================================
// Helper Functions
// ============================================================================

/// A single step's assistant notes.
#[derive(Serialize)]
pub struct WorkflowNoteEntry {
    pub step_id: String,
    pub content: String,
}

/// Question state for a single step.
#[derive(Serialize)]
pub struct StepQuestionStateEntry {
    pub step_id: String,
    pub status_text: String,
    pub question_text: Option<String>,
    pub updated_at: String,
}

pub fn step_response(r: crate::db::WorkflowStepRow) -> WorkflowStepResponse {
    WorkflowStepResponse {
        id: r.id,
        workflow_id: r.workflow_id,
        agent_id: r.agent_id,
        execution_mode: r.execution_mode,
        for_each_ref: r.for_each_ref,
        prompt_template_id: r.prompt_template_id,
        prompt_template: r.prompt_template,
        output_schema_id: r.output_schema_id,
        output_variable_name: r.output_variable_name,
        interactive_agent_id: r.interactive_agent_id,
        for_each_label_field: r.for_each_label_field,
        display_order: r.display_order,
        version: r.version,
        reasoning_trace: r.reasoning_trace,
        verification_agent_ids: r
            .verification_agent_ids
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default(),
        position_x: r.position_x,
        position_y: r.position_y,
        width: r.width,
        height: r.height,
        name: r.name,
        system_prompt_suffix: r.system_prompt_suffix,
        room_id: r.room_id,
        visible: r.visible,
        description: r.description,
        sub_workflow_template_id: r.sub_workflow_template_id,
        pinned: r.pinned,
        run_results_summary: r.run_results_summary,
    }
}
