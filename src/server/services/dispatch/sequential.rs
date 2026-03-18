//! Sequential design pipeline for board submit.
//!
//! Runs builder + designer for each step in topological order, threading
//! the previous step's handoff to the next step's builder and designer.
//! After each step, if the designer updated the handoff, downstream steps
//! are re-designed (designer-only, no builder) — propagating until a step
//! absorbs the change without updating its own handoff.
//!
//! Spawned as a single background task from the board submit handler.

use std::collections::{HashMap, HashSet};

use uuid::Uuid;

use crate::db::{WorkflowStepEdgeRow, WorkflowStepRow};
use crate::server::services::board::instruction::NodeDispatchInstruction;
use crate::server::services::dispatch::{NextStepText, PreviousStepHandoff};
use crate::server::state::AppState;
use crate::server::ws::events::{SessionEvent, SessionEventKind};
use crate::types::UserId;

/// Run the design pipeline sequentially in topological order.
///
/// For each step in the workflow (topo order):
/// - If it has a dispatch instruction → run builder + designer
/// - If a parent's handoff changed → run designer-only (propagation re-design)
/// - Otherwise → skip
///
/// After each step, compare old vs new handoff. If changed, downstream
/// steps will be re-designed when they're reached in the topo order.
pub async fn run_sequential_design_pipeline(
    state: AppState,
    workflow_id: Uuid,
    user_id: UserId,
    instructions: Vec<NodeDispatchInstruction>,
    steps: Vec<WorkflowStepRow>,
    edges: Vec<WorkflowStepEdgeRow>,
) {
    // Build topo-sorted order over ALL steps in the workflow
    let topo_order = match crate::server::hub::dag::topological_sort(&steps, &edges) {
        Ok(order) => order,
        Err(e) => {
            tracing::warn!(
                workflow_id = %workflow_id,
                error = %e,
                "Failed to topological sort for sequential design — falling back to instruction order"
            );
            instructions.iter().map(|i| i.step_id).collect()
        }
    };

    // Build instruction lookup by step_id
    let instruction_map: HashMap<Uuid, &NodeDispatchInstruction> =
        instructions.iter().map(|i| (i.step_id, i)).collect();

    // Build step lookup (refreshed after each iteration)
    let mut step_map: HashMap<Uuid, WorkflowStepRow> =
        steps.into_iter().map(|s| (s.id, s)).collect();

    // Track which steps had their handoff changed (for propagation)
    let mut handoff_changed: HashSet<Uuid> = HashSet::new();
    let mut dispatched_count: usize = 0;
    let mut propagated_count: usize = 0;

    for step_id in &topo_order {
        let has_instruction = instruction_map.contains_key(step_id);

        // Check if any parent's handoff changed (propagation trigger)
        let parent_ids = crate::server::hub::dag::get_parent_steps(*step_id, &edges);
        let parent_changed = parent_ids.iter().any(|pid| handoff_changed.contains(pid));

        if !has_instruction && !parent_changed {
            continue;
        }

        // Save old handoff for comparison after design
        let old_handoff = step_map
            .get(step_id)
            .map(|s| s.designer_handoff.clone())
            .unwrap_or_default();

        // Build context for this step
        let previous_step_handoff = lookup_previous_handoff(&parent_ids, &step_map);
        let child_ids = crate::server::hub::dag::get_child_steps(*step_id, &edges);
        let next_step_text = lookup_next_step_text(&child_ids, &step_map);

        if has_instruction {
            // Run builder + designer (existing path)
            let instruction = instruction_map[step_id];
            run_builder_and_designer(
                &state,
                *step_id,
                workflow_id,
                user_id,
                instruction,
                previous_step_handoff,
                next_step_text,
            )
            .await;
            dispatched_count += 1;
        } else {
            // Propagation: run designer-only (no builder)
            tracing::info!(
                step_id = %step_id,
                "Propagating re-design: upstream handoff changed"
            );
            run_propagation_redesign(
                &state,
                *step_id,
                workflow_id,
                user_id,
                previous_step_handoff,
                next_step_text,
            )
            .await;
            propagated_count += 1;
        }

        // Re-read step from DB to pick up the designer_handoff written by complete_design
        if let Ok(Some(updated_step)) = state.repos().workflows.get_step(*step_id).await {
            let new_handoff = updated_step.designer_handoff.clone();
            step_map.insert(*step_id, updated_step);

            if old_handoff != new_handoff {
                tracing::info!(
                    step_id = %step_id,
                    old_len = old_handoff.len(),
                    new_len = new_handoff.len(),
                    "Handoff changed — downstream steps will be re-designed"
                );
                handoff_changed.insert(*step_id);
            } else {
                tracing::debug!(
                    step_id = %step_id,
                    handoff_len = new_handoff.len(),
                    "Handoff unchanged — no propagation"
                );
            }
        }
    }

    tracing::info!(
        workflow_id = %workflow_id,
        dispatched = dispatched_count,
        propagated = propagated_count,
        "Sequential design pipeline completed"
    );
}

// ── Helpers ──────────────────────────────────────────────────────────────

/// Look up handoffs from ALL parent steps (fan-in support).
fn lookup_previous_handoff(
    parent_ids: &[Uuid],
    step_map: &HashMap<Uuid, WorkflowStepRow>,
) -> Vec<PreviousStepHandoff> {
    parent_ids
        .iter()
        .filter_map(|pid| {
            let parent = step_map.get(pid)?;
            if parent.designer_handoff.is_empty() {
                return None;
            }
            Some(PreviousStepHandoff {
                step_name: parent.name.clone().unwrap_or_default(),
                handoff_description: parent.designer_handoff.clone(),
            })
        })
        .collect()
}

/// Look up box text from ALL child steps (fan-out support).
fn lookup_next_step_text(
    child_ids: &[Uuid],
    step_map: &HashMap<Uuid, WorkflowStepRow>,
) -> Vec<NextStepText> {
    child_ids
        .iter()
        .filter_map(|cid| {
            let child = step_map.get(cid)?;
            if child.description.is_empty() {
                return None;
            }
            Some(NextStepText {
                step_name: child.name.clone().unwrap_or_default(),
                description: child.description.clone(),
            })
        })
        .collect()
}

/// Run the full builder + designer dispatch for a step with an instruction.
async fn run_builder_and_designer(
    state: &AppState,
    step_id: Uuid,
    workflow_id: Uuid,
    user_id: UserId,
    instruction: &NodeDispatchInstruction,
    previous_step_handoff: Vec<PreviousStepHandoff>,
    next_step_text: Vec<NextStepText>,
) {
    // Enrich instruction with previous step context for the builder
    let enriched_instruction = if previous_step_handoff.is_empty() {
        instruction.instruction.clone()
    } else {
        let blocks: Vec<String> = previous_step_handoff
            .iter()
            .map(|h| {
                format!(
                    "<previous_step name=\"{}\">\n<handoff>\n{}\n</handoff>\n</previous_step>",
                    h.step_name, h.handoff_description
                )
            })
            .collect();
        format!("{}\n\n{}", instruction.instruction, blocks.join("\n\n"))
    };

    let session_id = super::find_or_create_builder_session(
        state,
        step_id,
        workflow_id,
        user_id,
        &instruction.execution_mode,
    )
    .await;

    let (execution_id, _cancel_token) = state.task_registry().spawn_task(
        step_id,
        workflow_id,
        session_id,
        enriched_instruction.clone(),
    );

    state.broadcast_session(SessionEvent {
        session_id: Uuid::nil(),
        user_id: None,
        kind: SessionEventKind::DispatchStarted {
            execution_id,
            step_id,
            instruction: enriched_instruction.clone(),
        },
    });

    crate::server::executors::dispatch::run_dispatch_task(
        state.clone(),
        execution_id,
        step_id,
        workflow_id,
        enriched_instruction,
        session_id,
        user_id,
        previous_step_handoff,
        next_step_text,
    )
    .await;
}

/// Run designer-only re-design for a step whose parent's handoff changed.
///
/// Skips the builder entirely — the roster and plan are already set.
/// The designer sees all agents as "designed" and uses verify-and-skip:
/// reads existing configs, checks against new `<previous_step>`, updates
/// if stale.
async fn run_propagation_redesign(
    state: &AppState,
    step_id: Uuid,
    workflow_id: Uuid,
    user_id: UserId,
    previous_step_handoff: Vec<PreviousStepHandoff>,
    next_step_text: Vec<NextStepText>,
) {
    // Use the step's task description as the dispatch instruction context
    let task_desc = state
        .repos()
        .workflows
        .get_mission_brief(step_id)
        .await
        .ok()
        .flatten()
        .map(|b| b.task_description)
        .unwrap_or_default();

    if task_desc.is_empty() {
        tracing::debug!(
            step_id = %step_id,
            "Skipping propagation re-design — no mission brief"
        );
        return;
    }

    // Create a session for the designer (reuses the workforce execution_mode)
    let session_id =
        super::find_or_create_builder_session(state, step_id, workflow_id, user_id, "workforce")
            .await;

    let instruction_text = format!("Re-design: upstream handoff changed. Task: {}", task_desc);

    let (execution_id, _cancel_token) = state.task_registry().spawn_task(
        step_id,
        workflow_id,
        session_id,
        instruction_text.clone(),
    );

    state.broadcast_session(SessionEvent {
        session_id: Uuid::nil(),
        user_id: None,
        kind: SessionEventKind::DispatchStarted {
            execution_id,
            step_id,
            instruction: instruction_text.clone(),
        },
    });

    // Run designer directly — no builder needed
    crate::server::executors::dispatch::designer_handoff::run_designer_after_builder(
        state,
        step_id,
        workflow_id,
        user_id,
        execution_id,
        &instruction_text,
        vec![], // no changed_agents — all agents show as "designed"
        previous_step_handoff,
        next_step_text,
    )
    .await;

    // Mark task as completed in the registry
    state.task_registry().mark_completed(
        execution_id,
        Some("Propagation re-design completed".to_string()),
    );

    state.broadcast_session(SessionEvent {
        session_id: Uuid::nil(),
        user_id: None,
        kind: SessionEventKind::DispatchCompleted {
            execution_id,
            step_id,
            summary: "Propagation re-design completed".to_string(),
            question: None,
        },
    });
}
