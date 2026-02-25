//! Run results service: build per-step execution results.
//!
//! Shared by `get_step_last_run` (latest run) and the run-detail endpoints
//! (specific historical run). Handles workforce, sub_workflow, and standard steps.

use serde::Serialize;
use uuid::Uuid;

use crate::db::WorkflowStepRow;
use crate::server::state::AppState;

use super::error::ServiceError;

// ── Domain types ─────────────────────────────────────────────────────────────

/// Per-step execution result across all execution modes.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct StepRunResult {
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
    pub phases: Option<Vec<PhaseResult>>,
    /// Present only for sub_workflow steps.
    pub child_execution_id: Option<Uuid>,
    /// Present only for sub_workflow steps.
    pub child_steps: Option<Vec<ChildStepResult>>,
}

/// Phase-level execution detail (workforce pipeline phases, designer + agents).
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct PhaseResult {
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
    pub agent_name: Option<String>,
    /// Protocol archetype that produced this phase.
    pub archetype: Option<String>,
}

/// Summary of a child step within a sub_workflow execution.
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ChildStepResult {
    pub step_name: Option<String>,
    pub execution_mode: String,
    pub status: String,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub duration_ms: Option<u64>,
    pub error: Option<String>,
}

// ── Core function ────────────────────────────────────────────────────────────

/// Build a `StepRunResult` for a given step + execution pair.
///
/// Handles workforce (protocol phases), sub_workflow (recursive child steps),
/// and standard steps (agent executions).
pub async fn build_step_run_result(
    state: &AppState,
    step: &WorkflowStepRow,
    execution_id: Uuid,
) -> Result<StepRunResult, ServiceError> {
    if step.execution_mode == "workforce" {
        build_workforce_result(state, step, execution_id).await
    } else if step.execution_mode == "sub_workflow" {
        build_sub_workflow_result(state, step, execution_id).await
    } else {
        build_standard_result(state, step, execution_id).await
    }
}

// ── Private helpers ──────────────────────────────────────────────────────────

async fn build_workforce_result(
    state: &AppState,
    step: &WorkflowStepRow,
    execution_id: Uuid,
) -> Result<StepRunResult, ServiceError> {
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
        return build_standard_result(state, step, execution_id).await;
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

    let phases: Vec<PhaseResult> = step_phases
        .into_iter()
        .map(|p| PhaseResult {
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

    Ok(StepRunResult {
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
}

async fn build_sub_workflow_result(
    state: &AppState,
    _step: &WorkflowStepRow,
    execution_id: Uuid,
) -> Result<StepRunResult, ServiceError> {
    let collection_repo = &state.repos().collections;
    let child_executions = collection_repo
        .list_child_executions(execution_id)
        .await
        .map_err(|e| ServiceError::Internal(anyhow::anyhow!("{}", e)))?;

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
                match Box::pin(build_step_run_result(state, cs, child.id)).await {
                    Ok(cr) => {
                        total_in += cr.input_tokens.unwrap_or(0);
                        total_out += cr.output_tokens.unwrap_or(0);
                        total_cost += cr.cost_usd.unwrap_or(0.0);
                        child_step_results.push(ChildStepResult {
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
                        child_step_results.push(ChildStepResult {
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

            Ok(StepRunResult {
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
        None => Err(ServiceError::not_found(
            "No child execution found for sub_workflow step",
        )),
    }
}

async fn build_standard_result(
    state: &AppState,
    step: &WorkflowStepRow,
    execution_id: Uuid,
) -> Result<StepRunResult, ServiceError> {
    let agent_execs = state
        .repos()
        .agent_executions
        .list_agent_executions_for_step_and_run(step.id, execution_id)
        .await?;

    let exec = agent_execs
        .first()
        .ok_or_else(|| ServiceError::not_found("No execution found for this step"))?;

    let duration_ms = exec
        .completed_at
        .map(|end| (end - exec.started_at).num_milliseconds().unsigned_abs());

    Ok(StepRunResult {
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

mod tests;
