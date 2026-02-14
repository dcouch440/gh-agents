//! Room step execution within the DAG.
//!
//! Extracted from `hub::dag::mod` — contains `execute_room_step` and the
//! helper functions that build composite per-agent outputs from room sessions.

mod tests;

use std::collections::HashMap;

use anyhow::anyhow;
use serde_json::Value as JsonValue;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::db::WorkflowStepEdgeRow;
use crate::db::WorkflowStepRow;
use crate::server::executors::room::{
    build_dag_room_prompt, execute_room_turn, RoomMemberWithAgent, SpeakerResult,
};
use crate::server::hub::dag::agent_designer::{self, DesignedAgentPrompt};
use crate::server::hub::dag::designer_input::room::build_room_designer_input;
use crate::server::hub::dag::designer_input::RoomDesignerMember;
use crate::server::hub::engine::ExecutionEngine;
use crate::server::hub::error::HubError;
use crate::server::state::AppState;
use crate::server::ws::events::WorkflowEventKind;
use crate::types::{ExecutionMetadata, ExecutionStatus, StepExecutionEnvelope};

use super::{
    broadcast_workflow_event, compose_prompt, resolve_output_key, resolve_port_inputs,
    step_display_name, PortMetadata, StepOutput, WorkflowExecutionContext,
};

/// Execute a room step within the DAG.
///
/// Two modes:
/// - **Auto-run**: All rounds execute automatically, agents discuss via `execute_room_turn()`.
/// - **Interactive** (`agent_execution_mode = "interactive"`): Agents run an initial round,
///   then the DAG pauses for user participation. The user chats via the normal room API
///   and closes the session to resume the DAG.
///
/// Produces a composite envelope with per-agent outputs: `{"agent:<uuid>": output, ...}`.
#[allow(clippy::too_many_arguments)]
pub(super) async fn execute_room_step(
    engine: &ExecutionEngine,
    state: &AppState,
    ctx: &WorkflowExecutionContext,
    step: &WorkflowStepRow,
    _steps: &[WorkflowStepRow],
    edges: &[WorkflowStepEdgeRow],
    var_outputs: &mut HashMap<String, JsonValue>,
    completed: &mut HashMap<Uuid, StepOutput>,
    completed_envelopes: &mut HashMap<Uuid, StepExecutionEnvelope>,
    port_meta: &PortMetadata,
    total_input_tokens: &mut i64,
    total_output_tokens: &mut i64,
    _total_cost_usd: &mut f32,
    cancel: Option<&CancellationToken>,
) -> Result<(), HubError> {
    // 1. Extract room_id
    let room_id = step.room_id.ok_or_else(|| {
        HubError::Internal(anyhow!(
            "step {} has execution_mode='room' but no room_id",
            step.id
        ))
    })?;

    // 2. Check if this is a resume with a completed session
    if let Some(existing_output) = completed.get(&step.id) {
        if let Some(ref structured) = existing_output.structured_output {
            if structured.get("status").and_then(|v| v.as_str()) == Some("awaiting_room") {
                // This step was paused for user interaction — check if session is completed
                if let Some(session_id_str) =
                    structured.get("room_session_id").and_then(|v| v.as_str())
                {
                    if let Ok(session_id) = session_id_str.parse::<Uuid>() {
                        let room_repo = &state.repos().rooms;
                        if let Ok(Some(session)) = room_repo.get_room_session(session_id).await {
                            if session.status == "completed" {
                                // Session completed — extract outputs from transcript
                                info!(
                                    step_id = %step.id,
                                    session_id = %session_id,
                                    "Resuming room step — extracting outputs from completed session"
                                );
                                let transcript = room_repo
                                    .get_room_transcript(session_id)
                                    .await
                                    .unwrap_or_default();

                                let resolved_key =
                                    resolve_output_key(step, &port_meta.step_outputs);
                                let key_ref = if resolved_key.is_empty() {
                                    None
                                } else {
                                    Some(resolved_key.as_str())
                                };
                                let (envelope_data, output) =
                                    extract_room_outputs_from_transcript(&transcript, key_ref);

                                if !output.variable_name.is_empty() {
                                    if let Some(ref structured) = output.structured_output {
                                        var_outputs.insert(
                                            output.variable_name.clone(),
                                            structured.clone(),
                                        );
                                    }
                                }

                                let envelope = StepExecutionEnvelope {
                                    status: ExecutionStatus::Success,
                                    data: Some(envelope_data),
                                    metadata: ExecutionMetadata {
                                        execution_id: session_id,
                                        execution_time_ms: 0,
                                        tokens_in: None,
                                        tokens_out: None,
                                        cost_usd: None,
                                        model: None,
                                        agent_id: None,
                                        iteration_index: None,
                                        iteration_label: None,
                                        routing_label: None,

                                        upstream_agent_id: None,
                                        upstream_routing_label: None,
                                        room_session_id: Some(session_id),
                                        room_id: Some(room_id),
                                        total_rounds: Some(session.current_turn),
                                    },
                                    error: None,
                                };
                                completed_envelopes.insert(step.id, envelope);
                                completed.insert(step.id, output);
                                return Ok(());
                            }
                        }
                    }
                }
            }
        }
    }

    // 3. Load room configuration
    let room_repo = &state.repos().rooms;
    let room = room_repo
        .get_room(room_id)
        .await
        .map_err(|e| HubError::Internal(anyhow!("failed to load room: {}", e)))?
        .ok_or_else(|| HubError::Internal(anyhow!("room {} not found", room_id)))?;

    // 4. Load members and their agents
    let members = room_repo
        .list_room_members(room_id)
        .await
        .map_err(|e| HubError::Internal(anyhow!("failed to load room members: {}", e)))?;

    let mut members_with_agents: Vec<RoomMemberWithAgent> = Vec::new();
    for member in members {
        let agent = state
            .repo()
            .get_persisted_agent(member.agent_id)
            .await
            .map_err(|e| anyhow::anyhow!("failed to load agent: {}", e))?
            .ok_or_else(|| HubError::AgentNotFound {
                step_id: step.id,
                agent_id: member.agent_id,
            })?;
        members_with_agents.push(RoomMemberWithAgent { member, agent });
    }

    // 4b. Run Agent Designer pre-lifecycle (if design-time config exists)
    let designed_prompts: Option<HashMap<Uuid, DesignedAgentPrompt>> = {
        let wf_repo = &state.repos().workflows;
        let room_step_config = wf_repo.get_room_step_config(step.id).await.ok().flatten();
        let room_step_members = wf_repo
            .list_room_step_members(step.id)
            .await
            .unwrap_or_default();
        let beliefs = wf_repo
            .list_beliefs_for_execution(ctx.run_id)
            .await
            .unwrap_or_default();

        if let Some(ref config) = room_step_config {
            let designer_members: Vec<RoomDesignerMember> = members_with_agents
                .iter()
                .map(|ma| {
                    let name = ma
                        .member
                        .display_name
                        .clone()
                        .unwrap_or_else(|| ma.agent.name.clone());
                    let perspective = room_step_members
                        .iter()
                        .find(|m| m.name == name)
                        .map(|m| m.perspective.clone())
                        .unwrap_or_default();
                    RoomDesignerMember {
                        id: ma.member.agent_id.to_string(),
                        name,
                        role: ma.member.role_description.clone(),
                        perspective,
                    }
                })
                .collect();

            let input = build_room_designer_input(
                &config.meeting_purpose,
                &config.interaction_mode,
                config.max_turns,
                &designer_members,
                &beliefs,
                completed_envelopes,
            );

            match agent_designer::run_agent_designer(
                engine, state, ctx, step, input, "room", cancel,
            )
            .await
            {
                Ok(result) => {
                    info!(
                        step_id = %step.id,
                        run_id = %result.run_id,
                        prompts = result.prompts.len(),
                        "Room Agent Designer completed"
                    );
                    *total_input_tokens += result.input_tokens;
                    *total_output_tokens += result.output_tokens;
                    let lookup: HashMap<Uuid, DesignedAgentPrompt> = result
                        .prompts
                        .into_iter()
                        .filter_map(|p| p.agent_id.parse::<Uuid>().ok().map(|id| (id, p)))
                        .collect();
                    Some(lookup)
                }
                Err(e) => {
                    warn!(
                        "Agent Designer failed for room step {}, using static prompts: {}",
                        step.id, e
                    );
                    None
                }
            }
        } else {
            None
        }
    };

    // 5. Resolve port inputs
    let port_inputs = if let Some(inputs) = port_meta.step_inputs.get(&step.id) {
        match resolve_port_inputs(
            step.id,
            edges,
            inputs,
            &port_meta.step_outputs,
            completed_envelopes,
        ) {
            Ok(resolved) => {
                debug!(step_id = %step.id, ports = resolved.len(), "Resolved port inputs for room step");
                Some(resolved)
            }
            Err(e) => {
                warn!("Port resolution failed for room step {}: {}", step.id, e);
                None
            }
        }
    } else {
        None
    };

    // 6. Compose initial prompt
    let prompt = compose_prompt(
        step,
        state.prompt_template_repo().as_deref(),
        state.doc_repo().as_deref(),
        state.workflow_repo().as_deref(),
        &**state.repo(),
        var_outputs,
        &ctx.prior_outputs,
        None,
        port_inputs.as_ref(),
    )
    .await;

    // 7. Create room session
    let session = room_repo
        .create_room_session(room_id)
        .await
        .map_err(|e| HubError::Internal(anyhow!("failed to create room session: {}", e)))?;

    // Broadcast: step started (room step)
    broadcast_workflow_event(
        state,
        ctx,
        step.workflow_id,
        WorkflowEventKind::StepStarted {
            step_id: step.id,
            step_name: step_display_name(step),
            agent_id: None,
            execution_id: Some(session.id),
        },
    );

    info!(
        step_id = %step.id,
        room_id = %room_id,
        session_id = %session.id,
        "Starting room step execution"
    );

    // 8. Get LLM provider
    let provider = engine.provider();

    // 9. Check execution mode
    let interactive = step.agent_execution_mode.as_deref() == Some("interactive");

    if interactive {
        // ── Interactive path: run initial round, then pause for user ──

        let user_message = build_dag_room_prompt(&prompt, 0, room.max_turns);
        let turn_result = execute_room_turn(
            state,
            provider,
            &room,
            &session,
            &members_with_agents,
            &user_message,
            ctx.user_id,
            cancel,
            designed_prompts.as_ref(),
        )
        .await?;

        for speaker in &turn_result.speakers {
            *total_input_tokens += speaker.input_tokens as i64;
            *total_output_tokens += speaker.output_tokens as i64;
        }

        // Store partial output for resume detection
        let partial = StepOutput {
            variable_name: resolve_output_key(step, &port_meta.step_outputs),
            raw_output: format!(
                "{{\"room_session_id\":\"{}\",\"status\":\"awaiting_room\"}}",
                session.id
            ),
            structured_output: Some(serde_json::json!({
                "room_session_id": session.id.to_string(),
                "status": "awaiting_room"
            })),
        };
        completed.insert(step.id, partial);

        // Broadcast: step paused (awaiting user interaction)
        broadcast_workflow_event(
            state,
            ctx,
            step.workflow_id,
            WorkflowEventKind::StepPaused {
                step_id: step.id,
                step_name: step
                    .output_variable_name
                    .clone()
                    .unwrap_or_else(|| step.id.to_string()),
            },
        );

        info!(
            step_id = %step.id,
            session_id = %session.id,
            "Room step paused — awaiting user interaction"
        );

        return Err(HubError::AwaitingUser {
            step_id: step.id,
            execution_id: session.id,
        });
    }

    // ── Auto-run path: execute all rounds ──

    let mut last_turn_result = None;
    let mut current_session = session.clone();

    for round in 0..room.max_turns {
        if cancel.is_some_and(|t| t.is_cancelled()) {
            return Err(HubError::Cancelled);
        }

        let user_message = build_dag_room_prompt(&prompt, round, room.max_turns);
        let turn_result = execute_room_turn(
            state,
            provider.clone(),
            &room,
            &current_session,
            &members_with_agents,
            &user_message,
            ctx.user_id,
            cancel,
            designed_prompts.as_ref(),
        )
        .await?;

        for speaker in &turn_result.speakers {
            *total_input_tokens += speaker.input_tokens as i64;
            *total_output_tokens += speaker.output_tokens as i64;
        }

        let session_done = turn_result.session_completed;
        last_turn_result = Some(turn_result);
        if session_done {
            break;
        }

        // Reload session for updated turn counter
        if let Ok(Some(updated)) = room_repo.get_room_session(current_session.id).await {
            current_session = updated;
        }
    }

    // 10. Extract per-agent outputs from final turn
    let room_output_key = resolve_output_key(step, &port_meta.step_outputs);
    let room_key_ref = if room_output_key.is_empty() {
        None
    } else {
        Some(room_output_key.as_str())
    };
    let (envelope_data, output) = if let Some(ref final_turn) = last_turn_result {
        extract_room_outputs_from_speakers(&final_turn.speakers, room_key_ref)
    } else {
        // No rounds executed (max_turns = 0)
        (
            JsonValue::Object(serde_json::Map::new()),
            StepOutput {
                variable_name: room_output_key.clone(),
                raw_output: "{}".to_string(),
                structured_output: Some(JsonValue::Object(serde_json::Map::new())),
            },
        )
    };

    // 11. Store results
    if !output.variable_name.is_empty() {
        if let Some(ref structured) = output.structured_output {
            var_outputs.insert(output.variable_name.clone(), structured.clone());
        }
    }

    let final_turn_number = last_turn_result
        .as_ref()
        .map(|t| t.turn_number)
        .unwrap_or(0);
    let envelope = StepExecutionEnvelope {
        status: ExecutionStatus::Success,
        data: Some(envelope_data),
        metadata: ExecutionMetadata {
            execution_id: session.id,
            execution_time_ms: 0,
            tokens_in: Some(*total_input_tokens as i32),
            tokens_out: Some(*total_output_tokens as i32),
            cost_usd: None,
            model: None,
            agent_id: None,
            iteration_index: None,
            iteration_label: None,
            routing_label: None,
            upstream_agent_id: None,
            upstream_routing_label: None,
            room_session_id: Some(session.id),
            room_id: Some(room_id),
            total_rounds: Some(final_turn_number),
        },
        error: None,
    };
    completed_envelopes.insert(step.id, envelope);
    completed.insert(step.id, output);

    // Broadcast: step completed (room step)
    broadcast_workflow_event(
        state,
        ctx,
        step.workflow_id,
        WorkflowEventKind::StepCompleted {
            step_id: step.id,
            step_name: step_display_name(step),
            agent_id: None,
            output: None,
            input_tokens: None,
            output_tokens: None,
            duration_ms: None,
        },
    );

    info!(
        step_id = %step.id,
        session_id = %session.id,
        rounds = final_turn_number,
        "Room step execution completed"
    );

    Ok(())
}

/// Extract per-agent outputs from SpeakerResult records (auto-run path).
pub(crate) fn extract_room_outputs_from_speakers(
    speakers: &[SpeakerResult],
    variable_name: Option<&str>,
) -> (JsonValue, StepOutput) {
    let mut composite = serde_json::Map::new();
    for speaker in speakers {
        let key = format!("agent:{}", speaker.agent_id);
        let value: JsonValue = serde_json::from_str(&speaker.content)
            .unwrap_or_else(|_| JsonValue::String(speaker.content.clone()));
        composite.insert(key, value);
    }
    let envelope_data = JsonValue::Object(composite);

    let output = StepOutput {
        variable_name: variable_name.unwrap_or_default().to_string(),
        raw_output: serde_json::to_string(&envelope_data).unwrap_or_default(),
        structured_output: Some(envelope_data.clone()),
    };

    (envelope_data, output)
}

/// Extract per-agent outputs from a room transcript (resume path).
///
/// Groups transcript entries by agent, takes each agent's last assistant message.
pub(super) fn extract_room_outputs_from_transcript(
    transcript: &[crate::db::RoomTranscriptEntry],
    variable_name: Option<&str>,
) -> (JsonValue, StepOutput) {
    let mut last_by_agent: HashMap<String, String> = HashMap::new();

    for entry in transcript {
        // Transcript entries include all messages; we want the last content per agent
        last_by_agent.insert(entry.agent_name.clone(), entry.content.clone());
    }

    let mut composite = serde_json::Map::new();
    for (agent_name, content) in &last_by_agent {
        let key = agent_name.to_lowercase().replace(' ', "_");
        let value: JsonValue =
            serde_json::from_str(content).unwrap_or_else(|_| JsonValue::String(content.clone()));
        composite.insert(key, value);
    }
    let envelope_data = JsonValue::Object(composite);

    let output = StepOutput {
        variable_name: variable_name.unwrap_or_default().to_string(),
        raw_output: serde_json::to_string(&envelope_data).unwrap_or_default(),
        structured_output: Some(envelope_data.clone()),
    };

    (envelope_data, output)
}
