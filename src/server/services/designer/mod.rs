//! Standalone Designer service — run the Agent Designer outside the workforce
//! pipeline so users can preview and debug generated agent prompts.
//!
//! Note: `run_standalone_design` is currently disabled — the underlying
//! agent_designer and designer_input modules have been removed. The
//! `get_latest_design` query endpoint remains functional.

use anyhow::anyhow;
use uuid::Uuid;

use crate::db::traits::WorkflowRepo;
use crate::db::{AgentDesignerOutputRow, AgentDesignerRunRow};
use crate::server::state::AppState;

use super::ServiceError;

// ── Result types ────────────────────────────────────────────────────────────

/// Result of a standalone design run.
#[derive(Debug, Clone)]
pub struct StandaloneDesignResult {
    pub execution_id: Uuid,
    pub is_preview: bool,
    pub run_id: Uuid,
    pub agents: Vec<DesignedAgent>,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cost_usd: f32,
}

/// One designed agent output (API-friendly shape).
#[derive(Debug, Clone)]
pub struct DesignedAgent {
    pub agent_name: String,
    pub system_prompt: String,
    pub assignment: String,
    pub assigned_tools: Vec<String>,
    pub reasoning: String,
    pub execution_order: i32,
}

/// Latest design for a step (if any).
#[derive(Debug, Clone)]
pub struct LatestDesign {
    pub run: AgentDesignerRunRow,
    pub outputs: Vec<AgentDesignerOutputRow>,
}

// ── run_standalone_design ───────────────────────────────────────────────────

/// Run the Designer for a workforce step outside the pipeline.
///
/// Currently disabled — the agent_designer and designer_input modules have been
/// removed as part of the system-node-agent migration. Returns a service error.
pub async fn run_standalone_design(
    _state: &AppState,
    _workflow_id: Uuid,
    _step_id: Uuid,
    _user_id: Uuid,
) -> Result<StandaloneDesignResult, ServiceError> {
    Err(ServiceError::Internal(anyhow!(
        "Standalone designer is not available — agent design is now handled by system node agents"
    )))
}

// ── get_latest_design ───────────────────────────────────────────────────────

/// Get the most recent Designer output for a step (across all executions).
pub async fn get_latest_design(
    repo: &dyn WorkflowRepo,
    workflow_id: Uuid,
    step_id: Uuid,
    user_id: Uuid,
) -> Result<Option<LatestDesign>, ServiceError> {
    // Verify ownership
    let workflow = repo
        .get_workflow(workflow_id)
        .await?
        .ok_or_else(|| ServiceError::not_found("Workflow"))?;
    if workflow.user_id != user_id {
        return Err(ServiceError::not_found("Workflow"));
    }

    // Check step belongs to workflow
    let step = repo
        .get_step(step_id)
        .await?
        .ok_or_else(|| ServiceError::not_found("Step"))?;
    if step.workflow_id != workflow_id {
        return Err(ServiceError::not_found("Step"));
    }

    let run = repo.get_latest_designer_run_for_step(step_id).await?;
    let Some(run) = run else {
        return Ok(None);
    };

    let outputs = repo.list_designer_outputs(run.id).await?;

    Ok(Some(LatestDesign { run, outputs }))
}

mod tests;
