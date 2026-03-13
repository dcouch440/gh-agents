//! Step last-run handler — returns execution data for the most recent workflow run.
//!
//! Also contains `build_step_run_response()`, the shared utility for building
//! per-step execution results used by both last-run and run-detail endpoints.

use axum::{
    extract::{Path, State},
    Json,
};
use serde::Serialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::db::pg_repo::PgRepo;
use crate::db::traits::WorkflowCollectionRepo;
use crate::db::WorkflowStepRow;
use crate::server::api::AppError;
use crate::server::auth as auth_utils;
use crate::server::state::AppState;

// ============================================================================
// Types
// ============================================================================

#[derive(Serialize, utoipa::ToSchema)]
pub struct StepLastRunResponse {
    pub execution_id: String,
    pub workflow_execution_id: String,
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
    /// Present for protocol steps (workforce).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phases: Option<Vec<PhaseExecution>>,
    /// Present only for sub_workflow steps.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub child_execution_id: Option<uuid::Uuid>,
    /// Present only for sub_workflow steps.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub child_steps: Option<Vec<super::types::ChildStepResult>>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct PhaseExecution {
    pub id: String,
    pub phase: String,
    pub document_name: Option<String>,
    pub status: String,
    pub output_content: Option<String>,
    pub input_tokens: Option<i32>,
    pub output_tokens: Option<i32>,
    pub cost_usd: Option<f64>,
    pub model: Option<String>,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub error_message: Option<String>,
    /// Human-readable agent name (e.g. "Scanner") for workforce agent phases.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_name: Option<String>,
    /// Protocol archetype that produced this phase.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archetype: Option<String>,
}

// ============================================================================
// Handler
// ============================================================================

/// GET /api/workflows/:wid/steps/:sid/last-run
#[utoipa::path(
    get,
    path = "/api/workflows/{wid}/steps/{sid}/last-run",
    tag = "Workflows",
    security(("bearer_auth" = [])),
    params(
        ("wid" = Uuid, Path, description = "Workflow ID"),
        ("sid" = Uuid, Path, description = "Step ID"),
    ),
    responses(
        (status = 200, description = "Last run data for the step", body = StepLastRunResponse),
        (status = 404, description = "No execution found")
    )
)]
pub async fn get_step_last_run(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path((wid, sid)): Path<(Uuid, Uuid)>,
) -> Result<Json<StepLastRunResponse>, AppError> {
    let workflow_repo = &state.repos().workflows;

    // Verify ownership
    let workflow = workflow_repo
        .get_workflow(wid)
        .await?
        .ok_or(AppError::not_found("Workflow"))?;
    if workflow.user_id != auth.user_id.0 {
        return Err(AppError::not_found("Workflow"));
    }

    // Verify step belongs to workflow
    let step = workflow_repo
        .get_step(sid)
        .await?
        .ok_or(AppError::not_found("Step"))?;
    if step.workflow_id != wid {
        return Err(AppError::not_found("Step"));
    }

    // Get latest workflow execution
    let db = state
        .db()
        .ok_or(AppError::Internal("Database not available".into()))?
        .clone();
    let collection_repo: Arc<dyn WorkflowCollectionRepo> = Arc::new(PgRepo::new(db));
    let executions = collection_repo
        .list_workflow_executions_by_workflow(wid, auth.user_id.0)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let latest_exec = executions
        .first()
        .ok_or(AppError::not_found("No executions found for this workflow"))?;

    build_step_run_response(&state, &step, latest_exec.id)
        .await
        .map(Json)
}

// ============================================================================
// Shared Step-Result Builder
// ============================================================================

/// Build a `StepLastRunResponse` for a given step + execution pair.
///
/// Shared by `get_step_last_run` (latest run) and the run-detail endpoints
/// (specific historical run). Handles workforce, sub_workflow, and standard steps.
pub(super) async fn build_step_run_response(
    state: &AppState,
    step: &WorkflowStepRow,
    execution_id: Uuid,
) -> Result<StepLastRunResponse, AppError> {
    if step.execution_mode == "workforce" {
        // Workforce: use protocol_executions for phase-level detail (designer + agents)
        let all_phases = state
            .repos()
            .protocols
            .list_protocol_executions_by_run(execution_id)
            .await?;

        let step_phases: Vec<_> = all_phases
            .into_iter()
            .filter(|p| p.protocol_step_id == step.id)
            .collect();

        if step_phases.is_empty() {
            // Fall through to agent_executions for older runs without protocol tracking
            let agent_execs = state
                .repos()
                .agent_executions
                .list_agent_executions_for_step_and_run(step.id, execution_id)
                .await?;

            let exec = agent_execs
                .first()
                .ok_or(AppError::not_found("No execution found for this step"))?;

            let duration_ms = exec
                .completed_at
                .map(|end| (end - exec.started_at).num_milliseconds().unsigned_abs());

            return Ok(StepLastRunResponse {
                execution_id: exec.id.to_string(),
                workflow_execution_id: execution_id.to_string(),
                status: exec.status.clone(),
                started_at: Some(exec.started_at.to_rfc3339()),
                completed_at: exec.completed_at.map(|t| t.to_rfc3339()),
                duration_ms,
                output: exec.output.clone(),
                structured_output: exec.structured_output.clone(),
                input_tokens: None,
                output_tokens: None,
                cost_usd: None,
                error: None,
                phases: None,
                child_execution_id: None,
                child_steps: None,
            });
        }

        // Aggregate totals across phases
        let total_tokens_in: i64 = step_phases
            .iter()
            .filter_map(|p| p.tokens_in.map(i64::from))
            .sum();
        let total_tokens_out: i64 = step_phases
            .iter()
            .filter_map(|p| p.tokens_out.map(i64::from))
            .sum();
        let total_cost: f64 = step_phases.iter().filter_map(|p| p.cost_usd).sum();

        let has_error = step_phases.iter().any(|p| p.status == "failed");
        let all_complete = step_phases.iter().all(|p| p.status == "complete");
        let overall_status = if has_error {
            "failed"
        } else if all_complete {
            "completed"
        } else {
            "running"
        };

        let earliest = step_phases.iter().map(|p| p.created_at).min();
        let latest_completed = step_phases.iter().filter_map(|p| p.completed_at).max();
        let duration_ms = match (earliest, latest_completed) {
            (Some(start), Some(end)) => Some((end - start).num_milliseconds().unsigned_abs()),
            _ => None,
        };

        let phases: Vec<PhaseExecution> = step_phases
            .into_iter()
            .map(|p| PhaseExecution {
                id: p.id.to_string(),
                phase: p.phase,
                document_name: None,
                status: p.status,
                output_content: p.output_content,
                input_tokens: p.tokens_in,
                output_tokens: p.tokens_out,
                cost_usd: p.cost_usd,
                model: p.model,
                started_at: p.created_at.to_rfc3339(),
                completed_at: p.completed_at.map(|t| t.to_rfc3339()),
                error_message: p.error_message,
                agent_name: p.agent_name,
                archetype: p.archetype,
            })
            .collect();

        Ok(StepLastRunResponse {
            execution_id: execution_id.to_string(),
            workflow_execution_id: execution_id.to_string(),
            status: overall_status.to_string(),
            started_at: earliest.map(|t| t.to_rfc3339()),
            completed_at: latest_completed.map(|t| t.to_rfc3339()),
            duration_ms,
            output: None,
            structured_output: None,
            input_tokens: Some(total_tokens_in),
            output_tokens: Some(total_tokens_out),
            cost_usd: Some(total_cost),
            error: None,
            phases: Some(phases),
            child_execution_id: None,
            child_steps: None,
        })
    } else if step.execution_mode == "sub_workflow" {
        // Sub-workflow (legacy, see tickets/remove-sub-workflow.md): look up child execution and its step results
        let db = state
            .db()
            .ok_or(AppError::Internal("Database not available".into()))?
            .clone();
        let collection_repo: Arc<dyn WorkflowCollectionRepo> = Arc::new(PgRepo::new(db));
        let child_executions = collection_repo
            .list_child_executions(execution_id)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        // Find the child execution that matches this step's template
        let child_exec = child_executions.first();

        match child_exec {
            Some(child) => {
                let child_workflow_repo = &state.repos().workflows;
                let child_steps = child_workflow_repo
                    .list_steps(child.workflow_id)
                    .await
                    .unwrap_or_default();

                let mut child_step_results = Vec::new();
                let mut total_in: i64 = 0;
                let mut total_out: i64 = 0;
                let mut total_cost: f64 = 0.0;

                for cs in &child_steps {
                    if cs.execution_mode == "context" || cs.execution_mode == "input" {
                        continue;
                    }
                    match Box::pin(build_step_run_response(state, cs, child.id)).await {
                        Ok(cr) => {
                            total_in += cr.input_tokens.unwrap_or(0);
                            total_out += cr.output_tokens.unwrap_or(0);
                            total_cost += cr.cost_usd.unwrap_or(0.0);
                            child_step_results.push(super::types::ChildStepResult {
                                step_name: cs.name.clone(),
                                execution_mode: cs.execution_mode.clone(),
                                status: cr.status,
                                input_tokens: cr.input_tokens,
                                output_tokens: cr.output_tokens,
                                duration_ms: cr.duration_ms,
                                error: cr.error,
                            });
                        }
                        Err(_) => {
                            child_step_results.push(super::types::ChildStepResult {
                                step_name: cs.name.clone(),
                                execution_mode: cs.execution_mode.clone(),
                                status: "skipped".to_string(),
                                input_tokens: None,
                                output_tokens: None,
                                duration_ms: None,
                                error: None,
                            });
                        }
                    }
                }

                let duration_ms = match (child.started_at, child.completed_at) {
                    (Some(start), Some(end)) => {
                        Some((end - start).num_milliseconds().unsigned_abs())
                    }
                    _ => None,
                };

                Ok(StepLastRunResponse {
                    execution_id: child.id.to_string(),
                    workflow_execution_id: execution_id.to_string(),
                    status: child.status.clone(),
                    started_at: child.started_at.map(|t| t.to_rfc3339()),
                    completed_at: child.completed_at.map(|t| t.to_rfc3339()),
                    duration_ms,
                    output: None,
                    structured_output: child.outputs.clone(),
                    input_tokens: Some(total_in),
                    output_tokens: Some(total_out),
                    cost_usd: Some(total_cost),
                    error: child.error.clone(),
                    phases: None,
                    child_execution_id: Some(child.id),
                    child_steps: Some(child_step_results),
                })
            }
            None => Err(AppError::not_found(
                "No child execution found for sub_workflow step",
            )),
        }
    } else {
        // Standard steps: use agent_executions
        let agent_execs = state
            .repos()
            .agent_executions
            .list_agent_executions_for_step_and_run(step.id, execution_id)
            .await?;

        let exec = agent_execs
            .first()
            .ok_or(AppError::not_found("No execution found for this step"))?;

        let duration_ms = exec
            .completed_at
            .map(|end| (end - exec.started_at).num_milliseconds().unsigned_abs());

        Ok(StepLastRunResponse {
            execution_id: exec.id.to_string(),
            workflow_execution_id: execution_id.to_string(),
            status: exec.status.clone(),
            started_at: Some(exec.started_at.to_rfc3339()),
            completed_at: exec.completed_at.map(|t| t.to_rfc3339()),
            duration_ms,
            output: exec.output.clone(),
            structured_output: exec.structured_output.clone(),
            input_tokens: None,
            output_tokens: None,
            cost_usd: None,
            error: None,
            phases: None,
            child_execution_id: None,
            child_steps: None,
        })
    }
}
