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
/// (specific historical run). Handles workforce and standard steps.
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
            // Pinned/replayed step: load the latest envelope from content_versions
            if let Ok(Some(envelope_json)) = state
                .repos()
                .content_versions
                .get_latest_envelope_for_step(step.id)
                .await
            {
                if let Ok(envelope) = serde_json::from_str::<serde_json::Value>(&envelope_json) {
                    let output_data = envelope.get("data").cloned();
                    let output_str = output_data.as_ref().map(|d| serde_json::to_string(d).unwrap_or_default());
                    return Ok(StepLastRunResponse {
                        execution_id: execution_id.to_string(),
                        workflow_execution_id: execution_id.to_string(),
                        status: "completed".to_string(),
                        started_at: None,
                        completed_at: None,
                        duration_ms: Some(0),
                        output: output_str,
                        structured_output: output_data,
                        input_tokens: Some(0),
                        output_tokens: Some(0),
                        cost_usd: Some(0.0),
                        error: None,
                        phases: None,
                    });
                }
            }

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

        // Compose workforce output from agent phases (mirrors runtime compose_workforce_output)
        let mut agents_map = serde_json::Map::new();
        for phase in &phases {
            if phase.phase.starts_with("agent") {
                if let (Some(name), Some(content)) = (&phase.agent_name, &phase.output_content) {
                    let key = name.to_lowercase().replace(' ', "_");
                    let value: serde_json::Value = serde_json::from_str(content)
                        .unwrap_or_else(|_| serde_json::Value::String(content.clone()));
                    agents_map.insert(key, value);
                }
            }
        }
        let (output, structured_output) = if agents_map.is_empty() {
            (None, None)
        } else {
            let mut composite = serde_json::Map::new();
            composite.insert("agents".to_string(), serde_json::Value::Object(agents_map));
            let val = serde_json::Value::Object(composite);
            (Some(serde_json::to_string(&val).unwrap_or_default()), Some(val))
        };

        Ok(StepLastRunResponse {
            execution_id: execution_id.to_string(),
            workflow_execution_id: execution_id.to_string(),
            status: overall_status.to_string(),
            started_at: earliest.map(|t| t.to_rfc3339()),
            completed_at: latest_completed.map(|t| t.to_rfc3339()),
            duration_ms,
            output,
            structured_output,
            input_tokens: Some(total_tokens_in),
            output_tokens: Some(total_tokens_out),
            cost_usd: Some(total_cost),
            error: None,
            phases: Some(phases),
        })
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
        })
    }
}
