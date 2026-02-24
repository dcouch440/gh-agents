//! Shared dispatch orchestration service.
//!
//! Provides `dispatch_to_builder()` — the core dispatch flow used by multiple
//! callers: the chat tool handler, the direct API, and the L2→L4 dispatch tool.
//! Handles session management, task registry, spawning, and WS broadcasting.

use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::server::state::AppState;
use crate::server::ws::events::{SessionEvent, SessionEventKind};
use crate::types::UserId;

mod tests;

// ── Passdown type ─────────────────────────────────────────────────────────

/// Structured output from a builder's `complete_task` tool call.
///
/// - `plan`: feeds into the designer at execution time
/// - `summary`: displayed to the manager/user, captures what was done
/// - `question`: optional, surfaces to manager's board state as `<asking>`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Passdown {
    pub plan: String,
    pub summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub question: Option<String>,
}

// ── Input / Output types ──────────────────────────────────────────────────

/// Everything needed to dispatch an instruction to a builder agent.
pub struct DispatchInput {
    pub step_id: Uuid,
    pub workflow_id: Uuid,
    pub user_id: UserId,
    pub instruction: String,
    /// `"manager"` for L2 builder, anything else for L4 node builder.
    pub execution_mode: String,
}

/// Result of a successful dispatch.
pub struct DispatchOutput {
    pub execution_id: Uuid,
    pub session_id: Uuid,
}

// ── Core dispatch function ────────────────────────────────────────────────

/// Dispatch an instruction to a builder agent.
///
/// 1. Finds or creates the persistent builder session.
/// 2. Registers the task in the in-memory TaskRegistry.
/// 3. Spawns the appropriate background runner (L2 or L4).
/// 4. Broadcasts a `DispatchStarted` WebSocket event.
///
/// Used by the assistant `dispatch` tool, the direct REST API, and the
/// manager `dispatch_to_builders` tool.
pub async fn dispatch_to_builder(state: &AppState, input: DispatchInput) -> DispatchOutput {
    let session_id = find_or_create_builder_session(
        state,
        input.step_id,
        input.workflow_id,
        input.user_id,
        &input.execution_mode,
    )
    .await;

    // Register in task registry
    let (execution_id, _cancel_token) = state.task_registry().spawn_task(
        input.step_id,
        input.workflow_id,
        session_id,
        input.instruction.clone(),
    );

    // Spawn the background runner
    let runner_state = state.clone();
    let runner_step_id = input.step_id;
    let runner_workflow_id = input.workflow_id;
    let runner_instruction = input.instruction.clone();
    let runner_execution_id = execution_id;
    let user_id = input.user_id;

    if input.execution_mode == "manager" {
        tokio::spawn(async move {
            crate::server::executors::manager_dispatch::run_manager_dispatch_task(
                runner_state,
                runner_execution_id,
                runner_workflow_id,
                user_id,
                runner_step_id,
                runner_instruction,
                session_id,
            )
            .await;
        });
    } else if input.execution_mode == "board_dispatch" {
        tokio::spawn(async move {
            crate::server::executors::board_dispatch::run_board_dispatch_task(
                runner_state,
                runner_execution_id,
                runner_workflow_id,
                user_id,
                runner_step_id,
                runner_instruction,
                session_id,
            )
            .await;
        });
    } else {
        tokio::spawn(async move {
            crate::server::executors::dispatch::run_dispatch_task(
                runner_state,
                runner_execution_id,
                runner_step_id,
                runner_workflow_id,
                runner_instruction,
                session_id,
                user_id,
            )
            .await;
        });
    }

    // Broadcast started event
    state.broadcast_session(SessionEvent {
        session_id: Uuid::nil(),
        user_id: None,
        kind: SessionEventKind::DispatchStarted {
            execution_id,
            step_id: input.step_id,
            instruction: input.instruction,
        },
    });

    DispatchOutput {
        execution_id,
        session_id,
    }
}

/// Cancel a running dispatch task. Returns `true` if cancelled.
pub fn cancel_dispatch(state: &AppState, execution_id: Uuid, step_id: Uuid) -> bool {
    let cancelled = state.task_registry().cancel_task(execution_id);

    if cancelled {
        state.broadcast_session(SessionEvent {
            session_id: Uuid::nil(),
            user_id: None,
            kind: SessionEventKind::DispatchCancelled {
                execution_id,
                step_id,
            },
        });
    }

    cancelled
}

// ── L2→L4 batch dispatch tool handler ─────────────────────────────────────

/// Execute the `dispatch_to_builders` batch tool call.
///
/// Resolves each node ref → step, dispatches configuration instructions
/// directly to node builders (L4), bypassing node assistants (L3).
pub async fn execute_dispatch_to_builders_tool(
    state: &AppState,
    input: &serde_json::Value,
    user_id: UserId,
    workflow_id: Uuid,
) -> serde_json::Value {
    let Some(messages) = input["messages"].as_array() else {
        return serde_json::json!({ "error": "Missing required parameter: messages" });
    };

    if messages.is_empty() {
        return serde_json::json!({ "error": "messages array must not be empty" });
    }

    let mut results = Vec::new();

    for msg in messages {
        let Some(node_ref) = msg["node"].as_str() else {
            results.push(serde_json::json!({
                "node": "unknown",
                "status": "error",
                "error": "Missing 'node' field",
            }));
            continue;
        };
        let Some(instruction) = msg["instruction"].as_str() else {
            results.push(serde_json::json!({
                "node": node_ref,
                "status": "error",
                "error": "Missing 'instruction' field",
            }));
            continue;
        };

        // Resolve node by name or ref ID → step
        let step = match crate::server::tools::manager::resolve::resolve_node(
            state.repos().workflows.as_ref(),
            workflow_id,
            node_ref,
        )
        .await
        {
            Ok(s) => s,
            Err(e) => {
                results.push(serde_json::json!({
                    "node": node_ref,
                    "status": "error",
                    "error": e,
                }));
                continue;
            }
        };

        // Dispatch directly to the node builder
        let output = dispatch_to_builder(
            state,
            DispatchInput {
                step_id: step.id,
                workflow_id,
                user_id,
                instruction: instruction.to_string(),
                execution_mode: step.execution_mode.clone(),
            },
        )
        .await;

        results.push(serde_json::json!({
            "node": node_ref,
            "execution_id": output.execution_id.to_string(),
            "status": "dispatched",
        }));
    }

    serde_json::json!({
        "dispatched": results.len(),
        "results": results,
    })
}

// ── User ID resolution ────────────────────────────────────────────────────

/// Resolve the user_id for a dispatch task by looking up the workflow owner.
///
/// Falls back to a nil UUID — the executor's ownership checks will catch
/// this safely.
pub async fn resolve_dispatch_user_id(state: &AppState, workflow_id: Uuid) -> UserId {
    match state.repos().workflows.get_workflow(workflow_id).await {
        Ok(Some(wf)) => UserId(wf.user_id),
        _ => UserId(Uuid::nil()),
    }
}

// ── Session management ────────────────────────────────────────────────────

/// Find or create the persistent builder session for a dispatch agent.
///
/// L2 (manager mode): Looks up by `workflow_id` + `role = manager_builder`.
/// L4 (node mode): Looks up by `step_id` + `role = builder`.
///
/// Creates the session if it doesn't exist.
async fn find_or_create_builder_session(
    state: &AppState,
    step_id: Uuid,
    workflow_id: Uuid,
    user_id: UserId,
    execution_mode: &str,
) -> Uuid {
    let sessions = state.repos().sessions.as_ref();

    if execution_mode == "manager" {
        // L2: manager builder session, keyed by workflow_id
        match sessions.find_manager_builder_session(workflow_id).await {
            Ok(Some(session)) => return session.id,
            Ok(None) => {}
            Err(e) => {
                tracing::warn!(
                    workflow_id = %workflow_id,
                    error = %e,
                    "Failed to find manager builder session, creating new"
                );
            }
        }

        let session_id = Uuid::new_v4();
        let draft_config = json!({
            "workflow_id": workflow_id.to_string(),
            "role": "manager_builder",
        });

        if let Err(e) = sessions
            .create_session(
                user_id,
                session_id,
                "dispatch",
                "Manager Builder",
                None,
                Some(draft_config),
            )
            .await
        {
            tracing::error!(
                workflow_id = %workflow_id,
                error = %e,
                "Failed to create manager builder session"
            );
        }

        session_id
    } else if execution_mode == "board_dispatch" {
        // Board dispatcher session, keyed by workflow_id
        match sessions.find_board_dispatcher_session(workflow_id).await {
            Ok(Some(session)) => return session.id,
            Ok(None) => {}
            Err(e) => {
                tracing::warn!(
                    workflow_id = %workflow_id,
                    error = %e,
                    "Failed to find board dispatcher session, creating new"
                );
            }
        }

        let session_id = Uuid::new_v4();
        let draft_config = json!({
            "workflow_id": workflow_id.to_string(),
            "role": "board_dispatcher",
        });

        if let Err(e) = sessions
            .create_session(
                user_id,
                session_id,
                "dispatch",
                "Board Dispatcher",
                None,
                Some(draft_config),
            )
            .await
        {
            tracing::error!(
                workflow_id = %workflow_id,
                error = %e,
                "Failed to create board dispatcher session"
            );
        }

        session_id
    } else {
        // L4: node builder session, keyed by step_id
        match sessions.find_builder_session_by_step_id(step_id).await {
            Ok(Some(session)) => return session.id,
            Ok(None) => {}
            Err(e) => {
                tracing::warn!(
                    step_id = %step_id,
                    error = %e,
                    "Failed to find builder session, creating new"
                );
            }
        }

        let session_id = Uuid::new_v4();
        let draft_config = json!({
            "step_id": step_id.to_string(),
            "workflow_id": workflow_id.to_string(),
            "role": "builder",
        });

        if let Err(e) = sessions
            .create_session(
                user_id,
                session_id,
                "dispatch",
                "Node Builder",
                None,
                Some(draft_config),
            )
            .await
        {
            tracing::error!(
                step_id = %step_id,
                error = %e,
                "Failed to create builder session"
            );
        }

        session_id
    }
}
