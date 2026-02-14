//! Refresh orchestrator for board context and goal summaries.
//!
//! - `refresh_board_context()` — renders the board once, distills per-node concurrently
//! - `refresh_node_goal()` — distills a single node's conversational intent

use tokio::task::JoinSet;
use uuid::Uuid;

use crate::server::hub::error::HubError;
use crate::server::state::AppState;

use super::distiller;
use super::renderer;

/// Minimum turns before re-distilling a node's goal.
/// Prevents thrashing on early conversation turns.
const GOAL_REFRESH_MIN_TURNS: u32 = 3;

/// Refresh board context for all nodes in a workflow.
///
/// Renders the board once (2 queries), then distills per-node concurrently
/// via Haiku. Results are stored in `board_context_cache` with a fresh
/// `board_context_updated_at` timestamp.
pub async fn refresh_board_context(
    state: &AppState,
    workflow_id: Uuid,
) -> Result<(), HubError> {
    let repo = state.repos().workflows.clone();

    // 1. Render the full board
    let board_render = renderer::render_board(repo.as_ref(), workflow_id).await?;

    // 2. Load steps for per-node distillation
    let steps = repo
        .list_steps(workflow_id)
        .await
        .map_err(HubError::Internal)?;

    // 3. Distill per-node concurrently
    let mut join_set = JoinSet::new();

    for step in &steps {
        let step_id = step.id;
        let node_name = step.name.clone().unwrap_or_else(|| "(unnamed)".to_string());
        let node_archetype = step.execution_mode.clone();
        let board = board_render.clone();
        let wf_repo = repo.clone();

        join_set.spawn(async move {
            let context =
                distiller::distill_board_for_node(&board, &node_name, &node_archetype).await;

            if let Some(ctx) = context {
                if let Err(e) = wf_repo.update_step_board_context(step_id, &ctx).await {
                    tracing::error!("Failed to store board context for step {step_id}: {e}");
                }
            }
        });
    }

    // Wait for all distillations to complete
    while let Some(result) = join_set.join_next().await {
        if let Err(e) = result {
            tracing::error!("Board context distillation task panicked: {e}");
        }
    }

    Ok(())
}

/// Refresh a single node's goal summary from its conversation history.
///
/// Loads recent session messages, distills the conversational intent via Haiku,
/// and stores the result. If the goal changed, marks the board context stale
/// so neighbors pick up the change on their next read.
pub async fn refresh_node_goal(
    state: &AppState,
    step_id: Uuid,
    session_id: Uuid,
    workflow_id: Uuid,
) -> Result<(), HubError> {
    let repo = state.repos().workflows.clone();

    // Load step info
    let step = repo
        .get_step(step_id)
        .await
        .map_err(HubError::Internal)?
        .ok_or_else(|| HubError::Internal(anyhow::anyhow!("Step {step_id} not found")))?;

    // Check if we have enough conversation to distill
    let message_count = state
        .repo()
        .count_session_messages(session_id)
        .await
        .map_err(HubError::Internal)?;

    if message_count < GOAL_REFRESH_MIN_TURNS {
        return Ok(());
    }

    // Load recent messages
    let messages = state
        .repo()
        .get_session_history(session_id, 10)
        .await
        .map_err(HubError::Internal)?;

    if messages.is_empty() {
        return Ok(());
    }

    // Format conversation for distillation
    let conversation: String = messages
        .iter()
        .map(|m| format!("{}: {}", m.role, m.content))
        .collect::<Vec<_>>()
        .join("\n");

    let node_name = step.name.as_deref().unwrap_or("(unnamed)");

    // Distill goal
    let new_goal = distiller::distill_node_goal(
        &conversation,
        node_name,
        &step.execution_mode,
        &step.goal_summary,
    )
    .await;

    if let Some(goal) = new_goal {
        let goal_changed = goal != step.goal_summary;

        // Store the new goal
        repo.update_step_goal_summary(step_id, &goal)
            .await
            .map_err(HubError::Internal)?;

        // If goal changed, mark board context stale so neighbors refresh
        if goal_changed {
            repo.mark_board_context_stale(workflow_id)
                .await
                .map_err(HubError::Internal)?;
        }
    }

    Ok(())
}
