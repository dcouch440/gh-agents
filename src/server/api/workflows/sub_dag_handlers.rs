//! Step execution sub-DAG handler — returns the internal execution pipeline
//! for protocol steps (designer phases, agent phases, designer outputs).
//!
//! Used to visualize the internal execution flow:
//!   Designer → Agent1 → Agent2 → ...

use axum::{
    extract::{Path, State},
    Json,
};
use serde::Serialize;
use uuid::Uuid;

use crate::server::api::AppError;
use crate::server::auth as auth_utils;
use crate::server::state::AppState;

// ============================================================================
// Types
// ============================================================================

#[derive(Serialize, utoipa::ToSchema)]
pub struct SubDagResponse {
    /// The protocol archetype (e.g. "task_force", "documenter").
    pub archetype: Option<String>,
    /// Designer run details (if a designer was involved).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub designer: Option<DesignerDetail>,
    /// Execution phases in chronological order.
    pub phases: Vec<SubDagPhase>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct DesignerDetail {
    pub run_id: String,
    pub archetype: String,
    pub phase: String,
    pub model_id: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cost_usd: f32,
    pub agents: Vec<DesignerAgentOutput>,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct DesignerAgentOutput {
    pub agent_name: String,
    pub system_prompt: String,
    pub task_prompt: String,
    pub assigned_tools: Vec<String>,
    pub reasoning: String,
    pub execution_order: i32,
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct SubDagPhase {
    pub id: String,
    pub phase: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_tokens: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_usd: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    pub started_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

// ============================================================================
// Handler
// ============================================================================

/// GET /api/workflows/:wid/steps/:sid/executions/:eid/sub-dag
#[utoipa::path(
    get,
    path = "/api/workflows/{wid}/steps/{sid}/executions/{eid}/sub-dag",
    tag = "Workflows",
    security(("bearer_auth" = [])),
    params(
        ("wid" = Uuid, Path, description = "Workflow ID"),
        ("sid" = Uuid, Path, description = "Step ID"),
        ("eid" = Uuid, Path, description = "Workflow execution ID"),
    ),
    responses(
        (status = 200, description = "Sub-DAG execution data", body = SubDagResponse),
        (status = 404, description = "Step or execution not found")
    )
)]
pub async fn get_step_sub_dag(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path((wid, sid, eid)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<Json<SubDagResponse>, AppError> {
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

    // Load protocol execution phases for this step + execution
    let all_phases = state
        .repos()
        .protocols
        .list_protocol_executions_by_run(eid)
        .await?;

    let step_phases: Vec<_> = all_phases
        .into_iter()
        .filter(|p| p.protocol_step_id == step.id)
        .collect();

    // Determine archetype from phases
    let archetype = step_phases
        .iter()
        .find_map(|p| p.archetype.clone())
        .or_else(|| {
            // Fallback: infer from execution_mode
            match step.execution_mode.as_str() {
                "task_force" => Some("task_force".to_string()),
                "documenter" => Some("documenter".to_string()),
                _ => None,
            }
        });

    // Load designer runs for this step + execution
    let designer_runs = workflow_repo
        .list_designer_runs_for_step(sid, eid)
        .await
        .unwrap_or_default();

    // Build designer detail from the first (or most recent) run.
    // When a designer phase has a protocol_execution linked, use direct lookup;
    // otherwise fall back to the designer_run_id path.
    let designer = if let Some(run) = designer_runs.first() {
        // Try direct protocol_execution_id lookup first (fast path via index).
        let designer_phase_exec = step_phases
            .iter()
            .find(|p| p.designer_run_id == Some(run.id));
        let outputs = if let Some(phase) = designer_phase_exec {
            workflow_repo
                .list_designer_outputs_by_protocol_execution(phase.id)
                .await
                .unwrap_or_default()
        } else {
            workflow_repo
                .list_designer_outputs(run.id)
                .await
                .unwrap_or_default()
        };

        Some(DesignerDetail {
            run_id: run.id.to_string(),
            archetype: run.archetype.clone(),
            phase: run.phase.clone(),
            model_id: run.model_id.clone(),
            input_tokens: run.input_tokens,
            output_tokens: run.output_tokens,
            cost_usd: run.cost_usd,
            agents: outputs
                .into_iter()
                .map(|o| DesignerAgentOutput {
                    agent_name: o.agent_name,
                    system_prompt: o.generated_system_prompt,
                    task_prompt: o.generated_task_prompt,
                    assigned_tools: o.assigned_tools,
                    reasoning: o.design_reasoning,
                    execution_order: o.execution_order,
                })
                .collect(),
        })
    } else {
        None
    };

    // Build phases list
    let phases: Vec<SubDagPhase> = step_phases
        .into_iter()
        .map(|p| SubDagPhase {
            id: p.id.to_string(),
            phase: p.phase,
            status: p.status,
            agent_name: p.agent_name,
            output_content: p.output_content,
            input_tokens: p.tokens_in,
            output_tokens: p.tokens_out,
            cost_usd: p.cost_usd,
            model: p.model,
            started_at: p.created_at.to_rfc3339(),
            completed_at: p.completed_at.map(|t| t.to_rfc3339()),
            error_message: p.error_message,
        })
        .collect();

    Ok(Json(SubDagResponse {
        archetype,
        designer,
        phases,
    }))
}
