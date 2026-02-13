//! Room archetype formatter for the Agent Designer.
//!
//! Converts room configuration + members + upstream beliefs into a generic
//! `DesignerInput`. The designer curates which beliefs each room member
//! receives most prominently based on their perspective.

use std::collections::HashMap;

use uuid::Uuid;

use crate::db::{BeliefRow, RoomStepConfigRow, RoomStepMemberRow};
use crate::types::StepExecutionEnvelope;

use super::{format_envelopes_as_upstream, AgentDefinition, DesignerInput};

/// Build `DesignerInput` for room members.
///
/// Includes beliefs from upstream belief_capture nodes when available.
/// The designer generates system prompts for each member; the user prompt
/// (transcript) is built per-turn by the room executor.
pub fn build_room_designer_input(
    room_config: &RoomStepConfigRow,
    members: &[RoomStepMemberRow],
    beliefs: &[BeliefRow],
    completed_envelopes: &HashMap<Uuid, StepExecutionEnvelope>,
) -> DesignerInput {
    let agents = members
        .iter()
        .enumerate()
        .map(|(idx, member)| {
            let belief_context = if beliefs.is_empty() {
                String::new()
            } else {
                let formatted = beliefs
                    .iter()
                    .map(|b| {
                        format!(
                            "- \"{}\" ({}, {} confidence, source: {})",
                            b.content, b.belief_type, b.confidence, b.source_step_name,
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                format!(
                    "Beliefs extracted from upstream analysis:\n{}\n\n\
                     Incorporate these beliefs into this member's prompt based on their \
                     perspective. Not all beliefs are equally relevant to every member — \
                     curate based on the member's role and expertise.",
                    formatted,
                )
            };

            let additional = if belief_context.is_empty() {
                format!("Perspective:\n{}", member.perspective)
            } else {
                format!("Perspective:\n{}\n\n{}", member.perspective, belief_context)
            };

            AgentDefinition {
                id: member.id.to_string(),
                name: member.name.clone(),
                role: member.role.clone(),
                capabilities: vec![],
                execution_order: idx as i32,
                additional_context: additional,
            }
        })
        .collect();

    DesignerInput {
        archetype: "room".to_string(),
        context_description: format!(
            "A room meeting with {} members. Purpose: {}",
            members.len(),
            room_config.meeting_purpose,
        ),
        agents,
        upstream: format_envelopes_as_upstream(completed_envelopes),
        available_tools: vec![],
        archetype_guidance: format!(
            "This is a room — a meeting space where agents discuss, debate, or review.\n\n\
             Meeting purpose: {}\n\
             Interaction mode: {}\n\
             Max turns: {}\n\n\
             Room-specific design guidance:\n\
             - Each member's system prompt should establish their perspective and expertise\n\
             - Members should know who else is in the room and what perspectives they bring\n\
             - Include collaborative framing: \"build on what others have said\", \
               \"be concise and additive\"\n\
             - For the task prompt: write a brief orientation that sets the scene for the \
               discussion. The room executor will append the transcript and user message \
               at runtime — the task prompt here is just the opening framing.\n\
             - If beliefs are provided in a member's additional_context, curate them \
               per-member: a security architect should see security-relevant beliefs \
               prominently, while a product manager should see UX-relevant beliefs \
               prominently. All members can see all beliefs, but emphasis and ordering \
               should match their perspective.\n\
             - Members with \"moderated\" interaction mode should defer to the moderator's \
               direction. Members with \"open\" mode can speak freely.",
            room_config.meeting_purpose, room_config.interaction_mode, room_config.max_turns,
        ),
    }
}
