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

        if has_instruction {
            // Run system node agent (replaces builder + designer)
            let instruction = instruction_map[step_id];
            run_system_node_dispatch(
                &state,
                *step_id,
                workflow_id,
                user_id,
                instruction,
                previous_step_handoff,
            )
            .await;
            dispatched_count += 1;
        } else {
            // Propagation: re-run system node agent with upstream context
            tracing::info!(
                step_id = %step_id,
                "Propagating re-design: upstream handoff changed"
            );
            run_system_node_propagation(
                &state,
                *step_id,
                workflow_id,
                user_id,
                previous_step_handoff,
            )
            .await;
            propagated_count += 1;
        }

        // Re-read step from DB to pick up the designer_handoff written by complete_design
        match state.repos().workflows.get_step(*step_id).await {
            Ok(Some(updated_step)) => {
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
                    tracing::info!(
                        step_id = %step_id,
                        handoff_len = new_handoff.len(),
                        had_instruction = has_instruction,
                        "Handoff unchanged after design — no downstream propagation"
                    );
                }
            }
            Ok(None) => {
                tracing::warn!(
                    step_id = %step_id,
                    "Step not found after design — cannot check handoff propagation"
                );
            }
            Err(e) => {
                tracing::warn!(
                    step_id = %step_id,
                    error = %e,
                    "Failed to re-read step after design — cannot check handoff propagation"
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
///
/// Retained for fallback (used by run_builder_and_designer).
#[allow(dead_code)]
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
///
/// Retained for fallback — the system node agent path replaces this.
/// Will be removed in slice 6 (cleanup).
#[allow(dead_code)]
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

/// Run the system node agent for a step with an instruction.
///
/// Replaces `run_builder_and_designer` — the system node agent handles both
/// configuration (builder) and prompt design (designer) in a single pass.
async fn run_system_node_dispatch(
    state: &AppState,
    step_id: Uuid,
    workflow_id: Uuid,
    user_id: UserId,
    instruction: &NodeDispatchInstruction,
    previous_step_handoff: Vec<PreviousStepHandoff>,
) {
    // Enrich instruction with previous step context
    let enriched_instruction =
        enrich_with_previous_step(&instruction.instruction, &previous_step_handoff);

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

    crate::server::executors::dispatch::system_node::run_system_node_task(
        state.clone(),
        execution_id,
        step_id,
        workflow_id,
        enriched_instruction,
        session_id,
        user_id,
    )
    .await;
}

/// Re-run the system node agent when an upstream step's description changed.
///
/// Replaces `run_propagation_redesign` — formats an instruction with the
/// updated upstream context and re-runs the system node agent.
async fn run_system_node_propagation(
    state: &AppState,
    step_id: Uuid,
    workflow_id: Uuid,
    user_id: UserId,
    previous_step_handoff: Vec<PreviousStepHandoff>,
) {
    // Use the step's box text (prompt_template) as context for the re-run.
    // prompt_template is the user's raw canvas text; description may be empty.
    let step = state
        .repos()
        .workflows
        .get_step(step_id)
        .await
        .ok()
        .flatten();

    let task_text = step
        .as_ref()
        .map(|s| {
            if !s.prompt_template.is_empty() {
                s.prompt_template.clone()
            } else {
                s.description.clone()
            }
        })
        .unwrap_or_default();

    if task_text.is_empty() && previous_step_handoff.is_empty() {
        tracing::debug!(
            step_id = %step_id,
            "Skipping system node propagation — no task text or upstream context"
        );
        return;
    }

    let base_instruction = format!(
        "The upstream step changed what it produces.\n\n<task>\n{}\n</task>",
        task_text
    );
    let instruction_text = enrich_with_previous_step(&base_instruction, &previous_step_handoff);

    let session_id =
        super::find_or_create_builder_session(state, step_id, workflow_id, user_id, "workforce")
            .await;

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

    crate::server::executors::dispatch::system_node::run_system_node_task(
        state.clone(),
        execution_id,
        step_id,
        workflow_id,
        instruction_text,
        session_id,
        user_id,
    )
    .await;
}

/// Enrich an instruction with `<previous_step>` context blocks.
fn enrich_with_previous_step(
    instruction: &str,
    previous_step_handoff: &[PreviousStepHandoff],
) -> String {
    if previous_step_handoff.is_empty() {
        return instruction.to_string();
    }

    let blocks: Vec<String> = previous_step_handoff
        .iter()
        .map(|h| {
            format!(
                "<previous_step name=\"{}\">\n{}\n</previous_step>",
                h.step_name, h.handoff_description
            )
        })
        .collect();

    format!("{}\n\n{}", instruction, blocks.join("\n\n"))
}

/// Run designer-only re-design for a step whose parent's handoff changed.
///
/// Retained for fallback — the system node propagation path replaces this.
/// Will be removed in slice 6 (cleanup).
#[allow(dead_code)]
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
