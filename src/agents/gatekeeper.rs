//! Gatekeeper agent for room turn management.
//!
//! The gatekeeper is a system-managed agent with a hardcoded prompt that decides
//! which room members should speak and in what order for each user message.

use regex::Regex;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::RoomMemberRow;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

pub const GATEKEEPER_SYSTEM_PROMPT: &str =
    crate::config::protocols::roles::MEETING_GATEKEEPER.system;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A member of the room roster as presented to the gatekeeper.
#[derive(Debug, Clone, Serialize)]
pub struct RosterEntry {
    pub agent_id: Uuid,
    pub name: String,
    pub role_description: String,
}

/// Input assembled for a gatekeeper LLM call.
#[derive(Debug, Clone, Serialize)]
pub struct GatekeeperInput {
    pub user_message: String,
    pub mentions: Vec<String>,
    pub transcript_tail: String,
    pub roster: Vec<RosterEntry>,
    pub max_speakers: i32,
}

/// A single speaker selection from the gatekeeper.
#[derive(Debug, Clone, Deserialize)]
pub struct SpeakerSelection {
    pub agent_id: Uuid,
    pub followup_context: String,
}

/// Parsed gatekeeper output.
#[derive(Debug, Clone, Deserialize)]
pub struct GatekeeperOutput {
    pub speakers: Vec<SpeakerSelection>,
}

// ---------------------------------------------------------------------------
// @ mention parsing
// ---------------------------------------------------------------------------

/// Parse `@Name` mentions from a user message.
/// Returns display names (lowercased) that matched against the roster.
pub fn parse_mentions(message: &str, roster: &[RoomMemberRow]) -> Vec<String> {
    let re = Regex::new(r"@(\w+)").unwrap();
    let mut mentions = Vec::new();
    for cap in re.captures_iter(message) {
        let name = &cap[1];
        let name_lower = name.to_lowercase();
        for member in roster {
            let member_name = member.display_name.as_deref().unwrap_or("").to_lowercase();
            if member_name == name_lower || member_name.starts_with(&name_lower) {
                mentions.push(member_name);
                break;
            }
        }
    }
    mentions
}

/// Build the user prompt for the gatekeeper LLM call.
pub fn build_gatekeeper_prompt(input: &GatekeeperInput) -> String {
    serde_json::to_string_pretty(input).unwrap_or_else(|_| input.user_message.clone())
}

/// Parse gatekeeper response JSON. Returns None on parse failure.
pub fn parse_gatekeeper_response(response: &str) -> Option<GatekeeperOutput> {
    // Try to parse directly
    if let Ok(output) = serde_json::from_str::<GatekeeperOutput>(response) {
        return Some(output);
    }
    // Try to extract JSON from markdown code fence
    let trimmed = response.trim();
    if let Some(start) = trimmed.find('{') {
        if let Some(end) = trimmed.rfind('}') {
            let json_str = &trimmed[start..=end];
            return serde_json::from_str::<GatekeeperOutput>(json_str).ok();
        }
    }
    None
}

/// Fallback speaker order: all members in display_order, with mentioned agents first.
pub fn fallback_speaker_order(
    roster: &[RoomMemberRow],
    mentions: &[String],
    max_speakers: i32,
) -> Vec<SpeakerSelection> {
    let mut mentioned = Vec::new();
    let mut rest = Vec::new();

    for member in roster {
        let member_name = member.display_name.as_deref().unwrap_or("").to_lowercase();
        let selection = SpeakerSelection {
            agent_id: member.agent_id,
            followup_context: String::new(),
        };
        if mentions.iter().any(|m| m == &member_name) {
            mentioned.push(selection);
        } else {
            rest.push(selection);
        }
    }

    mentioned.extend(rest);
    mentioned.truncate(max_speakers as usize);
    mentioned
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_roster() -> Vec<RoomMemberRow> {
        vec![
            RoomMemberRow {
                room_id: Uuid::nil(),
                agent_id: Uuid::from_u128(1),
                display_name: Some("SecurityLead".to_string()),
                role_description: "Security specialist".to_string(),
                display_order: 0,
            },
            RoomMemberRow {
                room_id: Uuid::nil(),
                agent_id: Uuid::from_u128(2),
                display_name: Some("ArchLead".to_string()),
                role_description: "Architecture specialist".to_string(),
                display_order: 1,
            },
            RoomMemberRow {
                room_id: Uuid::nil(),
                agent_id: Uuid::from_u128(3),
                display_name: Some("FrontendLead".to_string()),
                role_description: "Frontend specialist".to_string(),
                display_order: 2,
            },
        ]
    }

    #[test]
    fn parse_mentions_finds_matching_names() {
        let roster = make_roster();
        let mentions = parse_mentions("Hey @SecurityLead what do you think?", &roster);
        assert_eq!(mentions, vec!["securitylead"]);
    }

    #[test]
    fn parse_mentions_handles_multiple() {
        let roster = make_roster();
        let mentions = parse_mentions("@SecurityLead and @ArchLead discuss this", &roster);
        assert_eq!(mentions.len(), 2);
    }

    #[test]
    fn parse_mentions_ignores_unknown() {
        let roster = make_roster();
        let mentions = parse_mentions("@UnknownAgent help me", &roster);
        assert!(mentions.is_empty());
    }

    #[test]
    fn parse_gatekeeper_response_valid_json() {
        let json = r#"{"speakers": [{"agent_id": "00000000-0000-0000-0000-000000000001", "followup_context": "Focus on CVEs"}]}"#;
        let result = parse_gatekeeper_response(json);
        assert!(result.is_some());
        assert_eq!(result.unwrap().speakers.len(), 1);
    }

    #[test]
    fn parse_gatekeeper_response_with_code_fence() {
        let json = "```json\n{\"speakers\": [{\"agent_id\": \"00000000-0000-0000-0000-000000000001\", \"followup_context\": \"test\"}]}\n```";
        let result = parse_gatekeeper_response(json);
        assert!(result.is_some());
    }

    #[test]
    fn parse_gatekeeper_response_invalid() {
        let result = parse_gatekeeper_response("not json at all");
        assert!(result.is_none());
    }

    #[test]
    fn fallback_speaker_order_mentions_first() {
        let roster = make_roster();
        let mentions = vec!["archlead".to_string()];
        let speakers = fallback_speaker_order(&roster, &mentions, 4);
        assert_eq!(speakers[0].agent_id, Uuid::from_u128(2)); // ArchLead first
    }

    #[test]
    fn fallback_speaker_order_respects_max() {
        let roster = make_roster();
        let speakers = fallback_speaker_order(&roster, &[], 2);
        assert_eq!(speakers.len(), 2);
    }

    #[test]
    fn fallback_speaker_order_single_member() {
        let roster = vec![RoomMemberRow {
            room_id: Uuid::nil(),
            agent_id: Uuid::from_u128(42),
            display_name: Some("SoloAgent".to_string()),
            role_description: "Does everything".to_string(),
            display_order: 0,
        }];
        let speakers = fallback_speaker_order(&roster, &[], 3);
        assert_eq!(speakers.len(), 1);
        assert_eq!(speakers[0].agent_id, Uuid::from_u128(42));
    }
}
