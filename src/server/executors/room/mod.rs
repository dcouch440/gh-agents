//! Room executor — orchestrates a single user turn in an agent room.
//!
//! Determines speaker order (via gatekeeper or fallback), then runs each
//! speaker sequentially through the `ExecutionEngine` using a `RoomSpeakerStrategy`.

use std::sync::Arc;

use tokio_util::sync::CancellationToken;
use tracing::{info, warn};
use uuid::Uuid;

use crate::agents::gatekeeper::{self, GatekeeperInput, RosterEntry, SpeakerSelection};
use crate::db::{AgentRow, RoomMemberRow, RoomRow, RoomSessionRow, RoomTranscriptEntry};
use crate::llm::{LLMProvider, LLMRequest, Message};
use crate::server::hub::streaming::StreamSink;
use crate::server::hub::{
    construct_agent_defaults, ExecutionEngine, ExecutionRecorder, HubError, RoomSpeakerConfig,
    RoomSpeakerStrategy,
};
use crate::server::state::AppState;
use crate::server::ws::events::{RoomEvent, RoomEventKind};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A room member paired with its full agent row.
pub struct RoomMemberWithAgent {
    pub member: RoomMemberRow,
    pub agent: AgentRow,
}

/// Result of a single speaker's turn.
#[derive(Debug, Clone)]
pub struct SpeakerResult {
    pub agent_id: Uuid,
    pub agent_name: String,
    pub content: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub speaker_order: i32,
}

/// Result of one complete room turn (all speakers).
#[derive(Debug, Clone)]
pub struct RoomTurnResult {
    pub turn_number: i32,
    pub speakers: Vec<SpeakerResult>,
    pub session_completed: bool,
}

// ---------------------------------------------------------------------------
// Room context builder
// ---------------------------------------------------------------------------

/// Build the room context preamble injected into each speaker's system prompt.
pub(crate) fn build_room_context(
    room: &RoomRow,
    member: &RoomMemberRow,
    agent: &AgentRow,
    members: &[RoomMemberWithAgent],
) -> String {
    let mut ctx = String::new();
    ctx.push_str("## Room Context\n\n");
    ctx.push_str(&format!(
        "You are **{}** in the room \"{}\".\n",
        member.display_name.as_deref().unwrap_or(&agent.name),
        room.name
    ));
    ctx.push_str(&format!("Your role: {}\n\n", member.role_description));
    ctx.push_str("Other participants:\n");
    for m in members {
        if m.member.agent_id == member.agent_id {
            continue;
        }
        let name = m.member.display_name.as_deref().unwrap_or(&m.agent.name);
        ctx.push_str(&format!("- **{}**: {}\n", name, m.member.role_description));
    }
    ctx.push_str("\nYou are in a group discussion with other AI agents. ");
    ctx.push_str("Build on what others have said. Be concise and additive — ");
    ctx.push_str("don't repeat points already made by other speakers.\n");
    ctx
}

/// Format transcript entries for injection into a speaker's prompt.
pub(crate) fn format_transcript(
    transcript: &[RoomTranscriptEntry],
    summary: Option<&str>,
) -> String {
    let mut out = String::new();

    if let Some(s) = summary {
        if !s.is_empty() {
            out.push_str("## Earlier Discussion (Summary)\n\n");
            out.push_str(s);
            out.push_str("\n\n");
        }
    }

    if transcript.is_empty() {
        return out;
    }

    out.push_str("## Recent Discussion\n\n");
    for entry in transcript {
        out.push_str(&format!(
            "**{}** ({}): {}\n\n",
            entry.agent_name, entry.role_description, entry.content
        ));
    }
    out
}

/// Build the full user prompt for a speaker: transcript + original message + gatekeeper context.
pub(crate) fn build_speaker_prompt(
    user_message: &str,
    followup_context: &str,
    transcript_block: &str,
) -> String {
    let mut prompt = String::new();
    if !transcript_block.is_empty() {
        prompt.push_str(transcript_block);
        prompt.push_str("---\n\n");
    }
    prompt.push_str(&format!("**User message**: {}\n", user_message));
    if !followup_context.is_empty() {
        prompt.push_str(&format!("\n**Facilitator note**: {}\n", followup_context));
    }
    prompt
}

// ---------------------------------------------------------------------------
// Gatekeeper call
// ---------------------------------------------------------------------------

/// Call the gatekeeper LLM to determine speaker order.
async fn call_gatekeeper(
    provider: &Arc<dyn LLMProvider>,
    room: &RoomRow,
    members: &[RoomMemberWithAgent],
    user_message: &str,
    mentions: &[String],
    transcript_tail: &str,
) -> Result<Vec<SpeakerSelection>, HubError> {
    let roster: Vec<RosterEntry> = members
        .iter()
        .map(|m| RosterEntry {
            agent_id: m.member.agent_id,
            name: m
                .member
                .display_name
                .clone()
                .unwrap_or_else(|| m.agent.name.clone()),
            role_description: m.member.role_description.clone(),
        })
        .collect();

    let input = GatekeeperInput {
        user_message: user_message.to_string(),
        mentions: mentions.to_vec(),
        transcript_tail: transcript_tail.to_string(),
        roster,
        max_speakers: room.max_speakers_per_turn,
    };

    let prompt_body = gatekeeper::build_gatekeeper_prompt(&input);

    let request = LLMRequest::new(&room.gatekeeper_model_id, vec![Message::user(&prompt_body)])
        .with_system(gatekeeper::GATEKEEPER_SYSTEM_PROMPT)
        .with_max_tokens(1024);

    let response = provider
        .send_message(request)
        .await
        .map_err(|e| HubError::LlmCallFailed {
            round: 0,
            source: e,
        })?;

    match gatekeeper::parse_gatekeeper_response(&response.content) {
        Some(output) => {
            info!(
                speakers = output.speakers.len(),
                "Gatekeeper selected speakers"
            );
            Ok(output.speakers)
        }
        None => {
            warn!("Gatekeeper response parse failed, using fallback order");
            let roster_rows: Vec<RoomMemberRow> =
                members.iter().map(|m| m.member.clone()).collect();
            Ok(gatekeeper::fallback_speaker_order(
                &roster_rows,
                mentions,
                room.max_speakers_per_turn,
            ))
        }
    }
}

// ---------------------------------------------------------------------------
// Main executor
// ---------------------------------------------------------------------------

/// Execute a single user turn in a room session.
///
/// 1. Parse @ mentions
/// 2. Determine speaker order (gatekeeper or fallback)
/// 3. For each speaker: assemble context, create agent_execution, call LLM via engine
/// 4. Increment turn, check limits
pub async fn execute_room_turn(
    state: &AppState,
    provider: Arc<dyn LLMProvider>,
    room: &RoomRow,
    session: &RoomSessionRow,
    members: &[RoomMemberWithAgent],
    user_message: &str,
    user_id: Uuid,
    cancel: Option<&CancellationToken>,
) -> Result<RoomTurnResult, HubError> {
    let room_repo = &state.repos().rooms;
    let ae_repo = &state.repos().agent_executions;

    // 1. Parse @ mentions
    let roster_rows: Vec<RoomMemberRow> = members.iter().map(|m| m.member.clone()).collect();
    let mentions = gatekeeper::parse_mentions(user_message, &roster_rows);

    // 2. Load transcript so far
    let transcript = room_repo
        .get_room_transcript(session.id)
        .await
        .unwrap_or_default();

    let transcript_block = format_transcript(&transcript, session.transcript_summary.as_deref());

    // 3. Determine speaker order
    let speakers = if room.gatekeeper_enabled {
        call_gatekeeper(
            &provider,
            room,
            members,
            user_message,
            &mentions,
            &transcript_block,
        )
        .await?
    } else {
        gatekeeper::fallback_speaker_order(&roster_rows, &mentions, room.max_speakers_per_turn)
    };

    // 4. Execute each speaker sequentially
    let engine = ExecutionEngine::new(provider.clone());
    let mut results = Vec::new();

    for (i, selection) in speakers.iter().enumerate() {
        if cancel.is_some_and(|t| t.is_cancelled()) {
            return Err(HubError::Cancelled);
        }

        // Find the member+agent for this speaker
        let member_agent = members
            .iter()
            .find(|m| m.member.agent_id == selection.agent_id);
        let Some(ma) = member_agent else {
            warn!(agent_id = %selection.agent_id, "Gatekeeper selected unknown agent, skipping");
            continue;
        };

        let speaker_name = ma
            .member
            .display_name
            .as_deref()
            .unwrap_or(&ma.agent.name)
            .to_string();

        // Broadcast speaker_start
        state.broadcast_room(RoomEvent {
            room_session_id: session.id,
            run_id: None,
            user_id: Some(user_id),
            kind: RoomEventKind::SpeakerStart {
                agent_id: selection.agent_id,
                agent_name: speaker_name.clone(),
                speaker_order: i as i32,
                turn_number: session.current_turn + 1,
            },
        });

        // Resolve mode with transcript as context
        let mode = if let Some(resolver) = state.mode_resolver() {
            resolver
                .resolve(&ma.agent, user_message, Some(&transcript_block))
                .await
                .map_err(|e| HubError::Internal(anyhow::anyhow!("Mode resolution failed: {}", e)))?
        } else {
            // Fallback: construct agent defaults for backward compatibility
            construct_agent_defaults(&ma.agent, &state.repo())
                .await
                .map_err(HubError::Internal)?
        };

        // Build system prompt: mode result + room context + agent docs
        let room_context = build_room_context(room, &ma.member, &ma.agent, members);
        let mut system_prompt = mode.system_prompt; // agent + mode already merged
        system_prompt.push_str("\n\n");
        system_prompt.push_str(&room_context);

        // Append agent context documents (global knowledge for this agent)
        if let Ok(agent_docs) = state.repo().get_agent_context(selection.agent_id).await {
            for doc in &agent_docs {
                system_prompt.push_str(&format!(
                    "\n\n---\n## {} (Agent Context)\n{}",
                    doc.title, doc.content
                ));
            }
        }

        // Build user prompt: transcript + user message + gatekeeper followup
        let user_prompt =
            build_speaker_prompt(user_message, &selection.followup_context, &transcript_block);

        // Apply room tools_enabled override
        let (tools, tool_names) = if room.tools_enabled {
            // Use resolved tools from mode (or agent defaults)
            (mode.tools, mode.tool_names)
        } else {
            // Room master switch OFF → force disable
            (vec![], vec![])
        };

        // Create agent_execution record
        let ae_row = ae_repo
            .create_agent_execution(
                selection.agent_id,
                None,  // workflow_step_id
                false, // is_interactive
                None,  // parent_agent_execution_id
                &system_prompt,
                &user_prompt,
                mode.selected_mode_id, // Track which mode was used
                Some(session.id),      // room_session_id
                Some(i as i32),        // speaker_order
                None,                  // workflow_execution_id
            )
            .await
            .map_err(HubError::Internal)?;

        // Record initial messages
        let _ = ae_repo
            .create_execution_message(ae_row.id, "system", &system_prompt, None, 0, 0)
            .await;
        let _ = ae_repo
            .create_execution_message(ae_row.id, "user", &user_prompt, None, 0, 0)
            .await;

        // Build strategy and execute
        let strategy = RoomSpeakerStrategy::new(
            RoomSpeakerConfig {
                agent: ma.agent.clone(),
                system_prompt,
                user_prompt,
                tools,
                tool_names,
                temperature: mode.temperature, // Use mode temperature
                execution_context: None,       // TODO: wire up if tools_enabled
                user_id,
                agent_execution_id: ae_row.id,
            },
            state.clone(),
        );

        // Use a room-aware sink that broadcasts tokens via WS
        let sink = RoomStreamSink {
            state: state.clone(),
            room_session_id: session.id,
            run_id: None,
            agent_id: selection.agent_id,
            agent_name: speaker_name.clone(),
            speaker_order: i as i32,
            turn_number: session.current_turn + 1,
            user_id,
        };

        let ae_repo2 = state.agent_execution_repo();
        let tl_repo = state.token_ledger_repo();
        let recorder = ExecutionRecorder::new(
            state.repo().as_ref(),
            ae_repo2.as_deref(),
            tl_repo.as_deref(),
        );

        let exec_result = engine
            .execute(&strategy, user_message, &sink, &recorder, cancel)
            .await;

        match exec_result {
            Ok(result) => {
                // Record the assistant response as an execution message
                let _ = ae_repo
                    .create_execution_message(
                        ae_row.id,
                        "assistant",
                        &result.content,
                        None,
                        result.input_tokens as i64,
                        result.output_tokens as i64,
                    )
                    .await;

                // Broadcast speaker_end
                state.broadcast_room(RoomEvent {
                    room_session_id: session.id,
                    run_id: None,
                    user_id: Some(user_id),
                    kind: RoomEventKind::SpeakerEnd {
                        agent_id: selection.agent_id,
                        agent_name: speaker_name.clone(),
                        content: result.content.clone(),
                        speaker_order: i as i32,
                        turn_number: session.current_turn + 1,
                    },
                });

                results.push(SpeakerResult {
                    agent_id: selection.agent_id,
                    agent_name: speaker_name,
                    content: result.content,
                    input_tokens: result.input_tokens,
                    output_tokens: result.output_tokens,
                    speaker_order: i as i32,
                });
            }
            Err(HubError::Cancelled) => return Err(HubError::Cancelled),
            Err(e) => {
                warn!(agent = %speaker_name, error = %e, "Speaker execution failed, continuing");
                // Update execution as failed
                let _ = ae_repo
                    .update_agent_execution_status(ae_row.id, "failed", None, None)
                    .await;
            }
        }
    }

    // 5. Increment turn counter
    let new_turn = room_repo
        .increment_room_session_turn(session.id)
        .await
        .unwrap_or(session.current_turn + 1);

    // 6. Check turn limit
    let session_completed = new_turn >= room.max_turns;
    if session_completed {
        let _ = room_repo
            .update_room_session_status(session.id, "completed")
            .await;
    }

    // Broadcast turn_complete or session_complete
    let kind = if session_completed {
        RoomEventKind::SessionComplete {
            turn_number: new_turn,
        }
    } else {
        RoomEventKind::TurnComplete {
            turn_number: new_turn,
        }
    };
    state.broadcast_room(RoomEvent {
        room_session_id: session.id,
        run_id: None,
        user_id: Some(user_id),
        kind,
    });

    Ok(RoomTurnResult {
        turn_number: new_turn,
        speakers: results,
        session_completed,
    })
}

// ---------------------------------------------------------------------------
// Room-aware stream sink
// ---------------------------------------------------------------------------

/// Streams tokens to WebSocket room subscribers.
struct RoomStreamSink {
    state: AppState,
    room_session_id: Uuid,
    run_id: Option<Uuid>,
    agent_id: Uuid,
    agent_name: String,
    speaker_order: i32,
    turn_number: i32,
    user_id: Uuid,
}

#[async_trait::async_trait]
impl StreamSink for RoomStreamSink {
    async fn token(&self, text: &str) {
        self.state.broadcast_room(RoomEvent {
            room_session_id: self.room_session_id,
            run_id: self.run_id,
            user_id: Some(self.user_id),
            kind: RoomEventKind::SpeakerToken {
                agent_id: self.agent_id,
                agent_name: self.agent_name.clone(),
                content: text.to_string(),
                speaker_order: self.speaker_order,
                turn_number: self.turn_number,
            },
        });
    }

    async fn tool_start(&self, _name: &str, _tool_id: &str) {}
    async fn tool_end(&self, _name: &str, _tool_id: &str) {}

    async fn error(&self, msg: &str) {
        warn!(
            room_session = %self.room_session_id,
            agent = %self.agent_name,
            "Room speaker error: {}",
            msg
        );
    }

    async fn done(&self) {
        // Speaker completion is handled by the executor, not the sink.
    }
}

// ---------------------------------------------------------------------------
// DAG room prompt builder
// ---------------------------------------------------------------------------

/// Build the "user message" for a DAG-driven room round.
///
/// Round 0: returns the composed workflow prompt directly.
/// Middle rounds: continuation prompt encouraging new perspectives.
/// Final round: closing prompt requesting final positions.
pub fn build_dag_room_prompt(composed_prompt: &str, round: i32, max_rounds: i32) -> String {
    if round == 0 {
        // First round or single-round room: use the composed prompt as-is
        if max_rounds <= 1 {
            // Single round — also signal this is the final round
            format!(
                "{}\n\nThis is the only round. Provide your complete analysis and final recommendation.",
                composed_prompt
            )
        } else {
            composed_prompt.to_string()
        }
    } else if round >= max_rounds - 1 {
        // Final round
        "This is the final round. Summarize your key findings and provide your final recommendation.".to_string()
    } else {
        // Middle round
        "Continue the discussion. Build on previous points and consider perspectives not yet explored.".to_string()
    }
}

#[cfg(test)]
mod tests;
