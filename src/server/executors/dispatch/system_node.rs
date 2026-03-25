//! System node agent dispatch executor.
//!
//! Runs the SystemNodeStrategy in a Docker container to configure a step's
//! runtime agents. After the agent completes, syncs files to DB and cleans
//! up the container.
//!
//! Replaces the builder + designer pair for workforce steps:
//! - Old: DispatchStrategy → complete_task → run_designer_after_builder
//! - New: SystemNodeStrategy → complete_system → sync_to_db

use uuid::Uuid;

/// Wall-clock timeout for the system node agent. Prevents runaway LLM loops
/// that exhaust max_rounds slowly (~10s per round x 10 rounds = 100s).
const SYSTEM_NODE_TIMEOUT_SECS: u64 = 120;

use crate::db::traits::CreateAgentExecutionInput;
use crate::server::hub::dag::container::{
    create_optional_container, destroy_optional_container, ManagedContainer,
};
use crate::server::hub::dag::ContainerExecutionConfig;
use crate::server::hub::engine::ExecutionEngine;
use crate::server::hub::recorder::ExecutionRecorder;
use crate::server::hub::strategies::SystemNodeStrategy;
use crate::server::hub::strategy::ExecutionStrategy;
use crate::server::hub::streaming::DispatchStreamSink;
use crate::server::services::system_node::sync;
use crate::server::state::AppState;
use crate::server::ws::events::SessionEventKind;
use crate::types::ExecutionType;
use crate::types::UserId;

use super::{broadcast_dispatch_event, persist_outcome, persist_trace};

/// Run a system node agent dispatch task to completion.
///
/// Creates a container, runs the SystemNodeStrategy (which writes JSON config
/// files via `run_command`), syncs the files to DB, and destroys the container.
///
/// Called from the sequential design pipeline in place of the old
/// `run_dispatch_task` + `run_designer_after_builder` pair.
#[allow(clippy::too_many_arguments)]
pub async fn run_system_node_task(
    state: AppState,
    execution_id: Uuid,
    step_id: Uuid,
    workflow_id: Uuid,
    instruction: String,
    session_id: Uuid,
    user_id: UserId,
) {
    // Get the cancel token from the registry
    let cancel_token = match state.task_registry().get_task(execution_id) {
        Some(entry) => entry.cancel_token,
        None => {
            tracing::error!(
                execution_id = %execution_id,
                "System node task not found in registry"
            );
            return;
        }
    };

    // Persist the instruction as a user message in the builder session
    if let Err(e) = state
        .repos()
        .sessions
        .insert_session_message(
            user_id,
            session_id,
            Uuid::new_v4(),
            "user".to_string(),
            instruction.clone(),
        )
        .await
    {
        tracing::warn!(
            session_id = %session_id,
            error = %e,
            "Failed to persist system node instruction"
        );
    }

    // Resolve base_dir FIRST — it's keyed by step_id so files persist across
    // dispatches. The agent sees its previous config.json, topology.json, and
    // agents/*.json on re-runs.
    let base_dir =
        crate::server::services::system_node::resolve_base_dir(&state, workflow_id, step_id);
    if let Err(e) = std::fs::create_dir_all(&base_dir) {
        tracing::warn!(
            base_dir = %base_dir.display(),
            error = %e,
            "Failed to create system node base_dir"
        );
    }

    // Build container config. Uses step_id (not execution_id) for workspace
    // volume path alignment — the container sees the same pinned directory.
    let container_config = build_container_config(&state, workflow_id, step_id).await;

    // Create container with the pinned workspace volume
    let managed_container = match create_optional_container(
        container_config.as_ref(),
        None, // no VPN for system node agents
        "system_node",
        state.workspace(),
    )
    .await
    {
        Ok(mc) => mc,
        Err(e) => {
            tracing::error!(
                execution_id = %execution_id,
                error = %e,
                "Failed to create container for system node agent"
            );
            persist_outcome(&state, session_id, user_id, &format!("Error: {e}")).await;
            state
                .task_registry()
                .mark_failed(execution_id, e.to_string());
            broadcast_dispatch_event(
                &state,
                SessionEventKind::DispatchFailed {
                    execution_id,
                    step_id,
                    error: e.to_string(),
                },
            );
            return;
        }
    };

    // Extract container handle for the strategy
    let container_handle = managed_container.as_ref().map(|mc| mc.agent_handle.clone());

    // Build the strategy
    let mut strategy = SystemNodeStrategy::new(
        state.clone(),
        step_id,
        workflow_id,
        instruction.clone(),
        Some(session_id),
        container_handle,
        base_dir.clone(),
    );

    // Get the LLM provider
    let provider = match state.provider() {
        Some(p) => p.clone(),
        None => {
            let err = "No LLM provider configured";
            tracing::error!(execution_id = %execution_id, err);
            persist_outcome(&state, session_id, user_id, err).await;
            cleanup_container(&managed_container).await;
            state
                .task_registry()
                .mark_failed(execution_id, err.to_string());
            broadcast_dispatch_event(
                &state,
                SessionEventKind::DispatchFailed {
                    execution_id,
                    step_id,
                    error: err.to_string(),
                },
            );
            return;
        }
    };

    // Create agent execution record for persistence
    let ae_id = match state
        .repos()
        .agent_executions
        .create_agent_execution(CreateAgentExecutionInput {
            execution_type: ExecutionType::Dispatch,
            agent_id: None,
            workflow_step_id: Some(step_id),
            parent_agent_execution_id: None,
            system_prompt_rendered: strategy.system_prompt().to_string(),
            input: instruction.clone(),
            room_session_id: None,
            speaker_order: None,
            workflow_execution_id: None,
        })
        .await
    {
        Ok(row) => Some(row.id),
        Err(e) => {
            tracing::warn!(
                execution_id = %execution_id,
                error = %e,
                "Failed to create agent execution record for system node"
            );
            None
        }
    };

    strategy.set_agent_execution_id(ae_id);

    // Run the engine
    let engine = ExecutionEngine::new(provider, state.env().debug_stream);
    let recorder = ExecutionRecorder::new(
        &*state.repos().sessions,
        &*state.repos().chat_messages,
        Some(&*state.repos().agent_executions),
        Some(&*state.repos().token_ledger),
    );
    let sink = DispatchStreamSink::new(state.clone(), execution_id, step_id);

    let result = match tokio::time::timeout(
        std::time::Duration::from_secs(SYSTEM_NODE_TIMEOUT_SECS),
        engine.execute(
            &strategy,
            &instruction,
            &sink,
            &recorder,
            Some(&cancel_token),
        ),
    )
    .await
    {
        Ok(inner) => inner,
        Err(_) => Err(crate::server::hub::error::HubError::Internal(
            anyhow::anyhow!(
                "System node agent timed out after {}s",
                SYSTEM_NODE_TIMEOUT_SECS
            ),
        )),
    };

    match result {
        Ok(_exec_result) => {
            // Retrieve captured summary from strategy
            let summary = strategy
                .take_summary()
                .unwrap_or_else(|| "System node agent completed".to_string());

            // Persist summary as assistant message in session
            persist_outcome(&state, session_id, user_id, &summary).await;

            // Sync files to DB
            let sync_result = sync::sync_to_db(
                &base_dir,
                step_id,
                workflow_id,
                state.repos().workflows.as_ref(),
                user_id.0,
            )
            .await;

            match &sync_result {
                Ok(sr) => {
                    tracing::info!(
                        execution_id = %execution_id,
                        step_id = %step_id,
                        created = sr.agents_created.len(),
                        updated = sr.agents_updated.len(),
                        removed = sr.agents_removed.len(),
                        description_changed = sr.description_changed,
                        "System node sync completed"
                    );
                }
                Err(e) => {
                    tracing::error!(
                        execution_id = %execution_id,
                        step_id = %step_id,
                        error = %e,
                        "System node sync failed"
                    );
                }
            }

            // Persist trace
            let passdown_json = serde_json::json!({
                "summary": summary,
                "sync": sync_result.as_ref().map(|sr| serde_json::json!({
                    "agents_created": sr.agents_created,
                    "agents_updated": sr.agents_updated,
                    "agents_removed": sr.agents_removed,
                    "description_changed": sr.description_changed,
                })).ok(),
            })
            .to_string();

            persist_trace(
                &state,
                execution_id,
                ae_id,
                "completed",
                Some(&passdown_json),
            )
            .await;

            state
                .task_registry()
                .mark_completed(execution_id, Some(summary.clone()));

            broadcast_dispatch_event(
                &state,
                SessionEventKind::DispatchCompleted {
                    execution_id,
                    step_id,
                    summary,
                    question: None,
                },
            );
        }
        Err(e) => {
            let error_msg = e.to_string();
            persist_outcome(&state, session_id, user_id, &format!("Error: {error_msg}")).await;

            if cancel_token.is_cancelled() {
                persist_trace(&state, execution_id, ae_id, "cancelled", None).await;
                state.task_registry().cancel_task(execution_id);
                broadcast_dispatch_event(
                    &state,
                    SessionEventKind::DispatchCancelled {
                        execution_id,
                        step_id,
                    },
                );
            } else {
                persist_trace(&state, execution_id, ae_id, "failed", Some(&error_msg)).await;
                state
                    .task_registry()
                    .mark_failed(execution_id, error_msg.clone());
                broadcast_dispatch_event(
                    &state,
                    SessionEventKind::DispatchFailed {
                        execution_id,
                        step_id,
                        error: error_msg,
                    },
                );
            }
        }
    }

    // Always clean up the container
    cleanup_container(&managed_container).await;
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Build container config for a system node agent.
///
/// Uses `workspace_subpath_override` to mount the pinned step directory
/// (`workflows/{wf_id}/pinned/{step_id}`) instead of a run directory.
/// Pinned paths survive run garbage collection by design.
///
/// Returns `None` if the workflow is missing.
async fn build_container_config(
    state: &AppState,
    workflow_id: Uuid,
    step_id: Uuid,
) -> Option<ContainerExecutionConfig> {
    let wf_row = match state.repos().workflows.get_workflow(workflow_id).await {
        Ok(Some(wf)) => wf,
        _ => return None,
    };

    let github_token =
        crate::execution::RedactedString::new(state.env().github_token.clone().unwrap_or_default());

    Some(ContainerExecutionConfig {
        clone_url: wf_row.target_repo_url.unwrap_or_default(),
        branch: wf_row.target_branch,
        github_token,
        image: None,
        memory_limit: None,
        cpu_limit: None,
        vpn_enabled: false,
        workflow_id: Some(workflow_id),
        run_id: None, // not used — workspace_subpath_override provides the path
        overlay_enabled: false,
        workspace_subpath_override: Some(format!(
            "{}/{}/pinned/{}",
            crate::constants::WORKSPACE_PREFIX,
            workflow_id,
            step_id,
        )),
    })
}

/// Clean up container after execution.
async fn cleanup_container(managed_container: &Option<ManagedContainer>) {
    destroy_optional_container(managed_container, None).await;
}
