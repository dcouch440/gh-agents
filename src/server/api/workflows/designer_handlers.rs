//! Standalone Designer handlers — trigger and retrieve Designer runs
//! outside the workforce pipeline.

use axum::{
    extract::{Path, State},
    Json,
};
use serde::Serialize;
use uuid::Uuid;

use crate::server::api::AppError;
use crate::server::auth as auth_utils;
use crate::server::services::designer;
use crate::server::state::AppState;

use super::sub_dag_handlers::{DesignerAgentOutput, DesignerDetail};

// ── Response types ──────────────────────────────────────────────────────────

#[derive(Serialize, utoipa::ToSchema)]
pub struct DesignResponse {
    pub execution_id: String,
    pub is_preview: bool,
    pub run_id: String,
    pub agents: Vec<DesignerAgentOutput>,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cost_usd: f32,
}

// ── Handlers ────────────────────────────────────────────────────────────────

/// POST /api/workflows/:wid/steps/:sid/design — run the Designer standalone.
///
/// Calls the Designer LLM for a workforce step and returns the generated agent
/// prompts, tool assignments, and reasoning. Debug events stream through the
/// WebSocket debug channel in real time during the call.
#[utoipa::path(
    post,
    path = "/api/workflows/{wid}/steps/{sid}/design",
    tag = "Workflows",
    security(("bearer_auth" = [])),
    params(
        ("wid" = Uuid, Path, description = "Workflow ID"),
        ("sid" = Uuid, Path, description = "Step ID"),
    ),
    responses(
        (status = 200, description = "Designer result", body = DesignResponse),
        (status = 400, description = "Step is not a workforce step or has no agents"),
        (status = 404, description = "Workflow or step not found")
    )
)]
pub async fn design_step(
    State(_state): State<AppState>,
    _auth: auth_utils::AuthUser,
    Path((_wid, _sid)): Path<(Uuid, Uuid)>,
) -> Result<Json<DesignResponse>, AppError> {
    Err(AppError::not_found(
        "Standalone designer is not available — agent design is now handled by system node agents",
    ))
}

/// GET /api/workflows/:wid/steps/:sid/design/latest — fetch the most recent
/// Designer output for a step across all executions.
#[utoipa::path(
    get,
    path = "/api/workflows/{wid}/steps/{sid}/design/latest",
    tag = "Workflows",
    security(("bearer_auth" = [])),
    params(
        ("wid" = Uuid, Path, description = "Workflow ID"),
        ("sid" = Uuid, Path, description = "Step ID"),
    ),
    responses(
        (status = 200, description = "Latest designer output", body = DesignerDetail),
        (status = 404, description = "No design found for this step")
    )
)]
pub async fn get_latest_step_design(
    State(state): State<AppState>,
    auth: auth_utils::AuthUser,
    Path((wid, sid)): Path<(Uuid, Uuid)>,
) -> Result<Json<DesignerDetail>, AppError> {
    let result =
        designer::get_latest_design(&*state.repos().workflows, wid, sid, auth.user_id.0).await?;

    let design = result.ok_or_else(|| AppError::not_found("Design"))?;

    Ok(Json(DesignerDetail {
        run_id: design.run.id.to_string(),
        archetype: design.run.archetype,
        phase: design.run.phase,
        model_id: design.run.model_id,
        input_tokens: design.run.input_tokens,
        output_tokens: design.run.output_tokens,
        cost_usd: design.run.cost_usd,
        agents: design
            .outputs
            .into_iter()
            .map(|o| DesignerAgentOutput {
                agent_name: o.agent_name,
                system_prompt: o.generated_system_prompt,
                assignment: o.generated_task_prompt,
                assigned_tools: o.assigned_tools,
                reasoning: o.design_reasoning,
                execution_order: o.execution_order,
            })
            .collect(),
    }))
}
