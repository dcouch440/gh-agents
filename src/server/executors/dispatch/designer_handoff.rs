//! Designer handoff — triggers the ReAct designer after the builder completes.
//!
//! Per the vision doc, the designer runs as part of the board submit pipeline:
//! Phase 0 → Builder → Designer (async). This module bridges the gap between
//! the builder dispatch completing and the designer writing agent configs.

use uuid::Uuid;

use crate::config::protocols::DESIGNER;
use crate::db::traits::CreateAgentExecutionInput;
use crate::server::hub::engine::filters::FilterContext;
use crate::server::hub::engine::ExecutionEngine;
use crate::server::hub::execution::strategies::react_designer::{
    ReactDesignerConfig, ReactDesignerStrategy,
};
use crate::server::hub::recorder::ExecutionRecorder;
use crate::server::hub::streaming::DispatchStreamSink;
use crate::server::state::AppState;
use crate::server::ws::events::{SessionEvent, SessionEventKind, WorkflowEventKind};
use crate::types::{ExecutionType, UserId};

/// Run the designer after the builder dispatch completes.
///
/// Loads the roster and plan from the DB, creates a ReactDesignerStrategy,
/// and executes it. Configs are written to the system store. Non-fatal —
/// logs errors but doesn't propagate them.
///
/// `execution_id` is the dispatch execution that triggered this handoff.
/// Designer tool calls stream through the same dispatch trace via
/// `DispatchStreamSink`, giving the frontend a continuous builder → designer
/// trace in the dispatch panel.
pub async fn run_designer_after_builder(
    state: &AppState,
    step_id: Uuid,
    workflow_id: Uuid,
    user_id: UserId,
    execution_id: Uuid,
    dispatch_instruction: &str,
    changed_agents: Vec<String>,
) {
    // Gate: S3 must be available
    let Some(_s3) = state.s3() else {
        tracing::debug!(step_id = %step_id, "Skipping designer handoff — S3 not available");
        return;
    };

    // Load mission brief + roster
    let repos = state.repos();
    let brief = match repos.workflows.get_mission_brief(step_id).await {
        Ok(Some(b)) => b,
        Ok(None) => {
            tracing::debug!(step_id = %step_id, "No mission brief — skipping designer handoff");
            return;
        }
        Err(e) => {
            tracing::warn!(step_id = %step_id, error = %e, "Failed to load mission brief for designer handoff");
            return;
        }
    };

    let roster = match repos.workflows.list_agent_roster(brief.id).await {
        Ok(r) if !r.is_empty() => r,
        Ok(_) => {
            tracing::debug!(step_id = %step_id, "Empty roster — skipping designer handoff");
            return;
        }
        Err(e) => {
            tracing::warn!(step_id = %step_id, error = %e, "Failed to load roster for designer handoff");
            return;
        }
    };

    tracing::info!(
        step_id = %step_id,
        agents = roster.len(),
        "Starting designer handoff after builder"
    );

    // Broadcast designer started (workflow topic — for StepTree status)
    broadcast_designer_progress(state, workflow_id, step_id, "started");

    // Broadcast phase marker (session topic — for dispatch panel trace)
    broadcast_dispatch_progress(
        state,
        execution_id,
        step_id,
        &format!("Designer phase: configuring {} agent(s)...", roster.len()),
    );

    // Create agent execution record
    let designer_ae_id = repos
        .agent_executions
        .create_agent_execution(CreateAgentExecutionInput {
            execution_type: ExecutionType::AgentDesigner,
            agent_id: None,
            workflow_step_id: Some(step_id),
            parent_agent_execution_id: None,
            system_prompt_rendered: String::new(),
            input: String::new(),
            room_session_id: None,
            speaker_order: None,
            workflow_execution_id: None,
        })
        .await
        .ok()
        .map(|row| row.id);

    // Deterministic session ID for designer persistence
    let designer_session_id = Uuid::new_v5(
        &Uuid::NAMESPACE_OID,
        format!("designer:{}", step_id).as_bytes(),
    );

    let upstream_topology =
        crate::server::services::dispatch::build_upstream_topology(state, step_id, workflow_id)
            .await;

    // Build enriched board_state with design status
    let board_state_xml = match crate::server::hub::board_state::build_snapshot(
        repos.workflows.as_ref(),
        None,
        crate::server::hub::board_state::BoardStateVariant::Dispatch,
        workflow_id,
        step_id,
    )
    .await
    {
        Ok(mut snapshot) => {
            crate::server::hub::board_state::enrich_design_status(
                &mut snapshot,
                repos.system_files.as_ref(),
                step_id,
                workflow_id,
                &changed_agents,
            )
            .await;
            crate::server::hub::board_state::render(
                &snapshot,
                crate::server::hub::board_state::BoardStateVariant::Dispatch,
            )
        }
        Err(e) => {
            tracing::warn!(step_id = %step_id, error = %e, "Failed to build board state for designer");
            String::new()
        }
    };

    let strategy = ReactDesignerStrategy::new(ReactDesignerConfig {
        state: state.clone(),
        step_id,
        workflow_id,
        roster: roster.clone(),
        session_id: Some(designer_session_id),
        agent_execution_id: designer_ae_id,
        board_state_xml,
        upstream_topology,
        dispatch_instruction: dispatch_instruction.to_string(),
        changed_agents,
    });

    let designer_cfg = DESIGNER.agent("react_designer");
    let filter_ctx = FilterContext::new(&designer_cfg.model_id, step_id);
    let recorder = ExecutionRecorder::new(
        &*repos.sessions,
        &*repos.chat_messages,
        Some(&*repos.agent_executions),
        Some(&*repos.token_ledger),
    );

    // Get or create a provider for the designer
    let provider = match state.provider() {
        Some(p) => p.clone(),
        None => {
            tracing::warn!(step_id = %step_id, "No LLM provider — skipping designer handoff");
            broadcast_designer_progress(state, workflow_id, step_id, "failed");
            return;
        }
    };

    let engine = ExecutionEngine::new(provider, state.env().debug_stream);

    // Stream designer tool calls through the same dispatch trace
    let sink = DispatchStreamSink::new(state.clone(), execution_id, step_id);

    match engine
        .with_filter_context(filter_ctx)
        .execute(&strategy, strategy.instruction(), &sink, &recorder, None)
        .await
    {
        Ok(result) => {
            tracing::info!(
                step_id = %step_id,
                rounds = result.rounds_used,
                tokens_in = result.input_tokens,
                tokens_out = result.output_tokens,
                "Designer handoff completed"
            );

            // Persist design summary to session
            if let Some(summary) = strategy.take_design_summary() {
                let _ = repos
                    .sessions
                    .insert_session_message(
                        user_id,
                        designer_session_id,
                        Uuid::new_v4(),
                        "assistant".to_string(),
                        summary,
                    )
                    .await;
            }

            broadcast_designer_progress(state, workflow_id, step_id, "completed");
        }
        Err(e) => {
            tracing::warn!(
                step_id = %step_id,
                error = %e,
                "Designer handoff failed"
            );
            broadcast_designer_progress(state, workflow_id, step_id, "failed");
        }
    }
}

fn broadcast_designer_progress(state: &AppState, workflow_id: Uuid, step_id: Uuid, status: &str) {
    state.broadcast_workflow(crate::server::ws::events::WorkflowEvent {
        workflow_id,
        run_id: None,
        user_id: None,
        kind: WorkflowEventKind::WorkforceDesignerProgress {
            step_id,
            status: status.to_string(),
        },
    });
}

fn broadcast_dispatch_progress(state: &AppState, execution_id: Uuid, step_id: Uuid, message: &str) {
    state.broadcast_session(SessionEvent {
        session_id: Uuid::nil(),
        user_id: None,
        kind: SessionEventKind::DispatchProgress {
            execution_id,
            step_id,
            message: message.to_string(),
        },
    });
}
