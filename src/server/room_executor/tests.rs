//! Tests for room executor

use super::*;
use crate::db::{AgentRow, RoomMemberRow, RoomRow, RoomTranscriptEntry};
use chrono::Utc;
use uuid::Uuid;

fn make_agent(name: &str) -> AgentRow {
    AgentRow {
        id: Uuid::new_v4(),
        tier: None,
        name: name.to_string(),
        system_prompt: format!("You are {}", name),
        persona_style: None,
        model_provider: "anthropic".to_string(),
        model_id: "claude-haiku-4-20250414".to_string(),
        model_max_tokens: 4096,
        model_temperature: 0.7,
        status: None,
        router_mode: None,
        output_schema_id: None,
        router_id: None,
        version: 1,
    }
}

fn make_member(agent: &AgentRow, display_name: &str, role: &str, order: i32) -> RoomMemberRow {
    RoomMemberRow {
        room_id: Uuid::nil(),
        agent_id: agent.id,
        display_name: Some(display_name.to_string()),
        role_description: role.to_string(),
        display_order: order,
    }
}

fn make_room() -> RoomRow {
    RoomRow {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        collection_id: None,
        name: "Architecture Review".to_string(),
        gatekeeper_enabled: false,
        gatekeeper_model_id: "claude-haiku-4-20250414".to_string(),
        max_speakers_per_turn: 4,
        max_turns: 20,
        tools_enabled: false,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

#[test]
fn build_room_context_includes_identity_and_roster() {
    let a1 = make_agent("SecurityBot");
    let a2 = make_agent("ArchBot");
    let m1 = make_member(&a1, "SecurityLead", "Security specialist", 0);
    let m2 = make_member(&a2, "ArchLead", "Architecture specialist", 1);
    let room = make_room();

    let members = vec![
        RoomMemberWithAgent {
            member: m1.clone(),
            agent: a1.clone(),
        },
        RoomMemberWithAgent {
            member: m2.clone(),
            agent: a2.clone(),
        },
    ];

    let ctx = build_room_context(&room, &m1, &a1, &members);

    assert!(ctx.contains("**SecurityLead**"));
    assert!(ctx.contains("Architecture Review"));
    assert!(ctx.contains("Security specialist"));
    // Should list other participants
    assert!(ctx.contains("**ArchLead**"));
    // Should NOT list self in other participants
    assert!(!ctx.contains("SecurityLead**: Security specialist\n"));
}

#[test]
fn format_transcript_empty() {
    let result = format_transcript(&[], None);
    assert!(result.is_empty());
}

#[test]
fn format_transcript_with_summary_and_entries() {
    let entries = vec![
        RoomTranscriptEntry {
            agent_name: "SecurityLead".to_string(),
            role_description: "Security specialist".to_string(),
            content: "The auth module has CVEs.".to_string(),
            speaker_order: Some(0),
            created_at: Utc::now(),
        },
        RoomTranscriptEntry {
            agent_name: "ArchLead".to_string(),
            role_description: "Architecture specialist".to_string(),
            content: "We should restructure the module.".to_string(),
            speaker_order: Some(1),
            created_at: Utc::now(),
        },
    ];

    let result = format_transcript(&entries, Some("Earlier they discussed performance."));

    assert!(result.contains("Earlier Discussion (Summary)"));
    assert!(result.contains("Earlier they discussed performance."));
    assert!(result.contains("Recent Discussion"));
    assert!(result.contains("**SecurityLead**"));
    assert!(result.contains("The auth module has CVEs."));
    assert!(result.contains("**ArchLead**"));
}

#[test]
fn format_transcript_no_summary() {
    let entries = vec![RoomTranscriptEntry {
        agent_name: "Bot".to_string(),
        role_description: "Helper".to_string(),
        content: "Hello".to_string(),
        speaker_order: Some(0),
        created_at: Utc::now(),
    }];

    let result = format_transcript(&entries, None);
    assert!(!result.contains("Summary"));
    assert!(result.contains("Recent Discussion"));
    assert!(result.contains("**Bot**"));
}

#[test]
fn build_speaker_prompt_all_parts() {
    let transcript = "## Recent Discussion\n\n**Bot**: Hello\n\n";
    let result = build_speaker_prompt("What should we do?", "Focus on security", transcript);

    assert!(result.contains("Recent Discussion"));
    assert!(result.contains("**User message**: What should we do?"));
    assert!(result.contains("**Facilitator note**: Focus on security"));
}

#[test]
fn build_speaker_prompt_no_followup() {
    let result = build_speaker_prompt("Hello", "", "");
    assert!(result.contains("**User message**: Hello"));
    assert!(!result.contains("Facilitator note"));
}

#[test]
fn build_speaker_prompt_no_transcript() {
    let result = build_speaker_prompt("Hello", "Be concise", "");
    assert!(!result.contains("---"));
    assert!(result.contains("**User message**: Hello"));
    assert!(result.contains("**Facilitator note**: Be concise"));
}
