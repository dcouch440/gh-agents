//! Pipeline stage advancement state machine.
//!
//! Extracted from the orchestrator's response consumer. Determines what
//! happens after a stage completes: advance to next stage, wait for
//! approval, mark completed, or mark failed.

use chrono::Utc;
use tracing::warn;
use uuid::Uuid;

use crate::server::state::AppState;
use crate::server::ws::{FeedUpdate, PipelineUpdate};

use super::error::HubError;

/// The result of attempting to advance a pipeline after a stage completes.
#[derive(Debug, Clone)]
pub enum PipelineAdvanceAction {
    /// Advanced to the next stage, which is now running.
    NextStage {
        stage_number: i32,
        stage_name: String,
    },
    /// Next stage requires approval before running.
    AwaitingApproval {
        stage_number: i32,
        stage_name: String,
    },
    /// All stages are complete — the pipeline run is finished.
    Completed,
    /// The pipeline failed at the given stage.
    Failed { reason: String },
}

/// Advance a pipeline after a stage completes (or fails).
///
/// This is the pure state machine. It:
/// 1. Records stage outputs
/// 2. Tries to advance to the next stage
/// 3. If next stage needs approval → sets waiting state
/// 4. If next stage is auto → returns NextStage (caller dispatches)
/// 5. If no more stages → marks run completed
pub async fn advance_pipeline(
    state: &AppState,
    run_id: Uuid,
    completed_stage_number: i32,
    stage_output: Option<String>,
    succeeded: bool,
    input_tokens: i64,
    output_tokens: i64,
    duration_ms: i64,
) -> Result<PipelineAdvanceAction, HubError> {
    let now = Utc::now();

    // Update stage execution row
    if let Ok(execs) = state.repo.list_stage_executions(run_id).await {
        if let Some(exec) = execs
            .into_iter()
            .find(|e| e.stage_number == completed_stage_number)
        {
            let mut updated = exec;
            updated.status = if succeeded {
                "completed".to_string()
            } else {
                "failed".to_string()
            };
            updated.output = if succeeded {
                stage_output.clone()
            } else {
                None
            };
            updated.input_tokens = input_tokens;
            updated.output_tokens = output_tokens;
            updated.duration_ms = duration_ms;
            updated.completed_at = Some(now);
            let _ = state.repo.update_stage_execution(&updated).await;
        }
    }

    // Update run token totals
    if let Ok(Some(mut run_row)) = state.repo.get_pipeline_run(run_id).await {
        *run_row.total_input_tokens.get_or_insert(0) += input_tokens;
        *run_row.total_output_tokens.get_or_insert(0) += output_tokens;
        run_row.current_stage = completed_stage_number;
        if !succeeded {
            run_row.status = "failed".to_string();
            run_row.completed_at = Some(now);
        }
        let _ = state.repo.update_pipeline_run(&run_row).await;
    }

    // Get pipeline_id for broadcasts
    let pipeline_id = {
        let mgr = state.pipeline_manager.read().await;
        mgr.get_run_pipeline_id(run_id)
            .map(|p| p.0)
            .unwrap_or(run_id)
    };

    // Broadcast stage completion/failure
    state.broadcast_pipeline(PipelineUpdate {
        run_id,
        pipeline_id,
        event: if succeeded {
            "stage_completed".into()
        } else {
            "stage_failed".into()
        },
        stage_number: Some(completed_stage_number),
        stage_name: None,
        agent_id: None,
        output: if succeeded { stage_output.clone() } else { None },
        input_tokens: Some(input_tokens as u64),
        output_tokens: Some(output_tokens as u64),
        duration_ms: Some(duration_ms as u64),
        user_input: None,
        timestamp: now,
        user_id: None,
    });

    if !succeeded {
        let mut mgr = state.pipeline_manager.write().await;
        let _ = mgr.fail_run(run_id, "Stage task failed");

        state.broadcast_feed(FeedUpdate {
            id: run_id,
            agent_id: "pipeline".into(),
            content: "Pipeline failed due to stage failure".into(),
            item_type: "pipeline_failed".into(),
            timestamp: now,
            user_id: None,
        });

        state.broadcast_pipeline(PipelineUpdate {
            run_id,
            pipeline_id,
            event: "run_failed".into(),
            stage_number: Some(completed_stage_number),
            stage_name: None,
            agent_id: None,
            output: None,
            input_tokens: None,
            output_tokens: None,
            duration_ms: None,
            user_input: None,
            timestamp: now,
            user_id: None,
        });

        return Ok(PipelineAdvanceAction::Failed {
            reason: "Stage task failed".to_string(),
        });
    }

    // Record structured output
    if let Some(ref output_str) = stage_output {
        let mut mgr = state.pipeline_manager.write().await;
        let p_id = mgr.get_run_pipeline_id(run_id);
        let s_name = mgr.get_stage_name(run_id, completed_stage_number as u32);

        if let (Some(pid), Some(sname)) = (p_id, s_name) {
            let output_schema = mgr
                .get_pipeline(&pid)
                .and_then(|p| {
                    p.stages
                        .get(completed_stage_number as usize)
                        .map(|s| s.output_schema.clone())
                })
                .unwrap_or_else(|| serde_json::json!({"fields": []}));
            let parsed =
                crate::agents::pipeline::parse_stage_output(output_str, &output_schema);
            mgr.record_stage_output(run_id, sname, parsed);
        }
    }

    // Try to advance
    let next_stage = {
        let mut mgr = state.pipeline_manager.write().await;
        mgr.advance_stage(run_id).ok().flatten()
    };

    match next_stage {
        Some(next) if next.approval_required => {
            let mut mgr = state.pipeline_manager.write().await;
            mgr.set_waiting_for_approval(run_id);

            // Persist waiting status
            if let Ok(Some(mut run_row)) = state.repo.get_pipeline_run(run_id).await {
                run_row.status = "waiting_for_approval".to_string();
                let _ = state.repo.update_pipeline_run(&run_row).await;
            }

            // Create stage execution for the gate
            let gate_exec = crate::db::StageExecutionRow {
                id: Uuid::new_v4(),
                run_id,
                stage_number: next.stage_number as i32,
                stage_name: next.stage_name.clone(),
                agent_id: next.agent_id.as_ref().map(|a| a.0),
                status: "waiting_for_approval".to_string(),
                rendered_prompt: None,
                output: None,
                structured_output: None,
                user_input: None,
                input_tokens: 0,
                output_tokens: 0,
                started_at: now,
                completed_at: None,
                duration_ms: 0,
                stage_member_id: None,
                pipeline_id: None,
            };
            let _ = state.repo.create_stage_execution(&gate_exec).await;

            if let Some(ae_repo) = &state.agent_execution_repo {
                if let Some(aid) = gate_exec.agent_id {
                    let _ = ae_repo
                        .create_agent_execution(gate_exec.id, aid, None, false, None, "", "", None)
                        .await;
                }
            }

            state.broadcast_feed(FeedUpdate {
                id: run_id,
                agent_id: "pipeline".into(),
                content: format!(
                    "Pipeline waiting for approval at stage {}",
                    next.stage_number
                ),
                item_type: "pipeline_approval".into(),
                timestamp: now,
                user_id: None,
            });

            state.broadcast_pipeline(PipelineUpdate {
                run_id,
                pipeline_id,
                event: "gate_waiting".into(),
                stage_number: Some(next.stage_number as i32),
                stage_name: Some(next.stage_name.clone()),
                agent_id: next.agent_id.as_ref().map(|a| a.0.to_string()),
                output: None,
                input_tokens: None,
                output_tokens: None,
                duration_ms: None,
                user_input: None,
                timestamp: now,
                user_id: None,
            });

            Ok(PipelineAdvanceAction::AwaitingApproval {
                stage_number: next.stage_number as i32,
                stage_name: next.stage_name,
            })
        }
        Some(next) => Ok(PipelineAdvanceAction::NextStage {
            stage_number: next.stage_number as i32,
            stage_name: next.stage_name,
        }),
        None => {
            // No more stages — pipeline is complete
            if let Ok(Some(mut run_row)) = state.repo.get_pipeline_run(run_id).await {
                run_row.status = "completed".to_string();
                run_row.completed_at = Some(now);
                // Persist final stage outputs
                let mgr = state.pipeline_manager.read().await;
                if let Some(outputs) = mgr.get_stage_outputs(run_id) {
                    run_row.stage_outputs =
                        Some(serde_json::to_value(outputs).unwrap_or_default());
                }
                drop(mgr);
                let _ = state.repo.update_pipeline_run(&run_row).await;
            }

            state.broadcast_pipeline(PipelineUpdate {
                run_id,
                pipeline_id,
                event: "run_completed".into(),
                stage_number: None,
                stage_name: None,
                agent_id: None,
                output: None,
                input_tokens: None,
                output_tokens: None,
                duration_ms: None,
                user_input: None,
                timestamp: now,
                user_id: None,
            });

            state.broadcast_feed(FeedUpdate {
                id: run_id,
                agent_id: "pipeline".into(),
                content: "Pipeline completed successfully".into(),
                item_type: "pipeline_completed".into(),
                timestamp: now,
                user_id: None,
            });

            Ok(PipelineAdvanceAction::Completed)
        }
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_variants() {
        let next = PipelineAdvanceAction::NextStage {
            stage_number: 2,
            stage_name: "Build".into(),
        };
        assert!(matches!(next, PipelineAdvanceAction::NextStage { stage_number: 2, .. }));

        let gate = PipelineAdvanceAction::AwaitingApproval {
            stage_number: 3,
            stage_name: "Review".into(),
        };
        assert!(matches!(gate, PipelineAdvanceAction::AwaitingApproval { .. }));

        let done = PipelineAdvanceAction::Completed;
        assert!(matches!(done, PipelineAdvanceAction::Completed));

        let fail = PipelineAdvanceAction::Failed {
            reason: "timeout".into(),
        };
        assert!(matches!(fail, PipelineAdvanceAction::Failed { .. }));
    }
}
