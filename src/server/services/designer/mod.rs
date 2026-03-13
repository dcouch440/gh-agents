//! Standalone Designer service — run the Agent Designer outside the workforce
//! pipeline so users can preview and debug generated agent prompts.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::anyhow;
use tracing::{info, warn};
use uuid::Uuid;

use crate::db::traits::{ContentVersionRepo, WorkflowCollectionRepo, WorkflowRepo};
use crate::db::{AgentDesignerOutputRow, AgentDesignerRunRow, WorkflowStepRow};
use crate::server::hub::dag::agent_designer;
use crate::server::hub::dag::broadcast_workflow_event;
use crate::server::hub::dag::designer_input::workforce::build_workforce_designer_input;
use crate::server::hub::dag::WorkflowExecutionContext;
use crate::server::hub::engine::ExecutionEngine;
use crate::server::hub::streaming::DagStreamSink;
use crate::server::state::AppState;
use crate::server::ws::events::WorkflowEventKind;
use crate::types::StepExecutionEnvelope;

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
/// Creates a lightweight `workflow_execution` (mode: "design") as the execution
/// context, builds the Designer input from current DB state, and invokes the
/// Designer LLM call. Debug events are broadcast via the WS debug channel.
///
/// If upstream steps have prior execution output, it's fed into the Designer
/// for richer prompts. Otherwise `is_preview = true` — the Designer still runs
/// but with structural context only.
pub async fn run_standalone_design(
    state: &AppState,
    workflow_id: Uuid,
    step_id: Uuid,
    user_id: Uuid,
) -> Result<StandaloneDesignResult, ServiceError> {
    let workflow_repo = &*state.repos().workflows;

    // 1. Verify ownership
    let workflow = workflow_repo
        .get_workflow(workflow_id)
        .await?
        .ok_or_else(|| ServiceError::not_found("Workflow"))?;
    if workflow.user_id != user_id {
        return Err(ServiceError::not_found("Workflow"));
    }

    // 2. Load step, validate execution mode
    let step = workflow_repo
        .get_step(step_id)
        .await?
        .ok_or_else(|| ServiceError::not_found("Step"))?;
    if step.workflow_id != workflow_id {
        return Err(ServiceError::not_found("Step"));
    }
    if step.execution_mode != "workforce" {
        return Err(ServiceError::validation(
            "Designer is only available for workforce steps",
        ));
    }

    // 3. Load mission brief + roster
    let brief = workflow_repo
        .get_mission_brief(step_id)
        .await?
        .ok_or_else(|| ServiceError::not_found("Mission brief"))?;
    let roster = workflow_repo.list_agent_roster(brief.id).await?;
    if roster.is_empty() {
        return Err(ServiceError::validation("Step has no agents in its roster"));
    }

    // 4. Create a design-only workflow execution
    let db = state
        .db()
        .ok_or_else(|| ServiceError::Internal(anyhow!("Database not available")))?
        .clone();
    let collection_repo: Arc<dyn WorkflowCollectionRepo> =
        Arc::new(crate::db::pg_repo::PgRepo::new(db));
    let execution = collection_repo
        .create_standalone_workflow_execution(workflow_id, user_id)
        .await
        .map_err(|e| ServiceError::Internal(anyhow!(e.to_string())))?;

    // Update execution_mode to "design" so it's distinguishable from real runs
    collection_repo
        .update_workflow_execution_status(execution.id, "running", None, None)
        .await
        .map_err(|e| ServiceError::Internal(anyhow!(e.to_string())))?;

    // 5. Build minimal execution context
    let ctx = WorkflowExecutionContext {
        stage_execution_id: execution.id,
        run_id: execution.id,
        user_id,
        initial_input: String::new(),
        prior_outputs: HashMap::new(),
        execution_context: None,
        container_config: None,
        wg_client: None,
        snapshot: None,
    };

    // 6. Gather upstream envelopes from last execution (if any)
    let (completed_envelopes, is_preview) =
        gather_upstream_envelopes(workflow_repo, &*state.repos().content_versions, &step).await;

    // 7. Load supporting data
    let steps = workflow_repo.list_steps(workflow_id).await?;
    let child_edges = if let Some(child_wf_id) = step.child_workflow_id {
        workflow_repo
            .list_edges(child_wf_id)
            .await
            .unwrap_or_default()
    } else {
        vec![]
    };
    let plan = workflow_repo.get_plan(step_id).await.unwrap_or_default();

    // 8. Build designer input
    let designer_input = build_workforce_designer_input(
        &brief,
        &roster,
        &completed_envelopes,
        &steps,
        plan.as_deref(),
        state.capability_registry(),
        &child_edges,
    );

    // 9. Build engine
    let provider = state
        .provider()
        .ok_or_else(|| ServiceError::Internal(anyhow!("LLM provider not configured")))?
        .clone();
    let engine = ExecutionEngine::new(provider, true);

    // 10. Build DagStreamSink for debug channel
    let sink = DagStreamSink::new(
        state.clone(),
        ctx.clone(),
        workflow_id,
        step_id,
        execution.id,
        "Designer".to_string(),
    )
    .with_agent_name(Some("Designer".to_string()));

    // 11. Broadcast started
    broadcast_workflow_event(
        state,
        &ctx,
        workflow_id,
        WorkflowEventKind::WorkforceDesignerProgress {
            step_id,
            status: "started".to_string(),
        },
    );

    info!(
        step_id = %step_id,
        execution_id = %execution.id,
        is_preview = is_preview,
        "Running standalone designer"
    );

    // 12. Call the designer
    let result = agent_designer::run_agent_designer(
        &engine,
        state,
        &ctx,
        &step,
        designer_input,
        "standalone",
        None,
        None,
        &sink,
    )
    .await;

    match result {
        Ok(designer_result) => {
            // Broadcast completed
            broadcast_workflow_event(
                state,
                &ctx,
                workflow_id,
                WorkflowEventKind::WorkforceDesignerProgress {
                    step_id,
                    status: "completed".to_string(),
                },
            );

            // Mark execution as completed
            let _ = collection_repo
                .update_workflow_execution_status(execution.id, "completed", None, None)
                .await;

            let agents = designer_result
                .prompts
                .iter()
                .map(|p| DesignedAgent {
                    agent_name: p.agent_name.clone(),
                    system_prompt: p.system_prompt.clone(),
                    assignment: p.assignment.clone(),
                    assigned_tools: p.tools.clone(),
                    reasoning: p.reasoning.clone(),
                    execution_order: p.execution_order,
                })
                .collect();

            Ok(StandaloneDesignResult {
                execution_id: execution.id,
                is_preview,
                run_id: designer_result.run_id,
                agents,
                input_tokens: designer_result.input_tokens,
                output_tokens: designer_result.output_tokens,
                cost_usd: designer_result.cost_usd,
            })
        }
        Err(e) => {
            warn!(
                step_id = %step_id,
                execution_id = %execution.id,
                error = %e,
                "Standalone designer failed"
            );

            broadcast_workflow_event(
                state,
                &ctx,
                workflow_id,
                WorkflowEventKind::WorkforceDesignerProgress {
                    step_id,
                    status: "failed".to_string(),
                },
            );

            let _ = collection_repo
                .update_workflow_execution_status(execution.id, "failed", None, Some(e.to_string()))
                .await;

            Err(ServiceError::Internal(anyhow!("Designer failed: {e}")))
        }
    }
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

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Gather upstream step envelopes from the most recent execution.
///
/// Returns `(envelopes, is_preview)` where `is_preview = true` when no upstream
/// output was found (the Designer will run with structural context only).
async fn gather_upstream_envelopes(
    workflow_repo: &dyn WorkflowRepo,
    cv_repo: &dyn ContentVersionRepo,
    step: &WorkflowStepRow,
) -> (HashMap<Uuid, StepExecutionEnvelope>, bool) {
    let edges = workflow_repo
        .list_edges(step.workflow_id)
        .await
        .unwrap_or_default();

    // Find upstream step IDs (steps that feed into this one)
    let upstream_step_ids: Vec<Uuid> = edges
        .iter()
        .filter(|e| e.to_step_id == step.id)
        .map(|e| e.from_step_id)
        .collect();

    if upstream_step_ids.is_empty() {
        return (HashMap::new(), true);
    }

    let mut envelopes = HashMap::new();
    for upstream_id in &upstream_step_ids {
        if let Ok(Some(json_str)) = cv_repo.get_latest_envelope_for_step(*upstream_id).await {
            if let Ok(envelope) = serde_json::from_str::<StepExecutionEnvelope>(&json_str) {
                envelopes.insert(*upstream_id, envelope);
            }
        }
    }

    let is_preview = envelopes.is_empty();
    (envelopes, is_preview)
}

mod tests;
