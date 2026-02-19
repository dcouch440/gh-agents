//! Tests for the room executor module.

#[cfg(test)]
mod tests {
    use crate::db::{AgentRow, RoomMemberRow, RoomRow, RoomTranscriptEntry};
    use crate::server::executors::room::{
        build_room_context, build_speaker_prompt, format_transcript, RoomMemberWithAgent,
    };
    use chrono::Utc;
    use uuid::Uuid;

    // =========================================================================
    // Fixtures
    // =========================================================================

    fn make_agent(id: Uuid, name: &str) -> AgentRow {
        AgentRow {
            id,
            name: name.to_string(),
            system_prompt: "You are a helpful assistant.".to_string(),
            model_id: "claude-3-sonnet".to_string(),
            status: Some("active".to_string()),
            ..Default::default()
        }
    }

    fn make_room(id: Uuid, name: &str) -> RoomRow {
        RoomRow {
            id,
            user_id: Uuid::new_v4(),
            collection_id: None,
            name: name.to_string(),
            gatekeeper_enabled: true,
            gatekeeper_model_id: "claude-3-haiku".to_string(),
            max_speakers_per_turn: 3,
            max_turns: 10,
            tools_enabled: false,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn make_member(
        room_id: Uuid,
        agent_id: Uuid,
        display_name: Option<&str>,
        role: &str,
    ) -> RoomMemberRow {
        RoomMemberRow {
            room_id,
            agent_id,
            display_name: display_name.map(|s| s.to_string()),
            role_description: role.to_string(),
            display_order: 0,
        }
    }

    fn make_member_with_agent(
        room_id: Uuid,
        agent_name: &str,
        display_name: Option<&str>,
        role: &str,
    ) -> RoomMemberWithAgent {
        let agent = make_agent(Uuid::new_v4(), agent_name);
        let member = make_member(room_id, agent.id, display_name, role);
        RoomMemberWithAgent { member, agent }
    }

    fn make_transcript_entry(agent_name: &str, role: &str, content: &str) -> RoomTranscriptEntry {
        RoomTranscriptEntry {
            agent_name: agent_name.to_string(),
            role_description: role.to_string(),
            content: content.to_string(),
            speaker_order: Some(0),
            created_at: Utc::now(),
        }
    }

    // =========================================================================
    // Room Context Tests
    // =========================================================================

    #[test]
    fn build_room_context_includes_room_name() {
        let room = make_room(Uuid::new_v4(), "Design Review");
        let agent = make_agent(Uuid::new_v4(), "Architect");
        let member = make_member(room.id, agent.id, None, "Lead architect");
        let members = vec![RoomMemberWithAgent {
            member: member.clone(),
            agent: agent.clone(),
        }];

        let ctx = build_room_context(&room, &member, &agent, &members);

        assert!(ctx.contains("Design Review"));
    }

    #[test]
    fn build_room_context_uses_display_name_over_agent_name() {
        let room = make_room(Uuid::new_v4(), "Test Room");
        let agent = make_agent(Uuid::new_v4(), "GenericAgent");
        let member = make_member(room.id, agent.id, Some("The Architect"), "Lead designer");
        let members = vec![RoomMemberWithAgent {
            member: member.clone(),
            agent: agent.clone(),
        }];

        let ctx = build_room_context(&room, &member, &agent, &members);

        assert!(ctx.contains("The Architect"));
        assert!(!ctx.contains("GenericAgent"));
    }

    #[test]
    fn build_room_context_falls_back_to_agent_name() {
        let room = make_room(Uuid::new_v4(), "Test Room");
        let agent = make_agent(Uuid::new_v4(), "FallbackAgent");
        let member = make_member(room.id, agent.id, None, "Helper");
        let members = vec![RoomMemberWithAgent {
            member: member.clone(),
            agent: agent.clone(),
        }];

        let ctx = build_room_context(&room, &member, &agent, &members);

        assert!(ctx.contains("FallbackAgent"));
    }

    #[test]
    fn build_room_context_includes_role_description() {
        let room = make_room(Uuid::new_v4(), "Test Room");
        let agent = make_agent(Uuid::new_v4(), "TestAgent");
        let member = make_member(room.id, agent.id, None, "Security specialist");
        let members = vec![RoomMemberWithAgent {
            member: member.clone(),
            agent: agent.clone(),
        }];

        let ctx = build_room_context(&room, &member, &agent, &members);

        assert!(ctx.contains("Security specialist"));
    }

    #[test]
    fn build_room_context_excludes_self_from_participants() {
        let room_id = Uuid::new_v4();
        let agent1 = make_agent(Uuid::new_v4(), "Agent1");
        let agent2 = make_agent(Uuid::new_v4(), "Agent2");
        let member1 = make_member(room_id, agent1.id, Some("Alice"), "Designer");
        let member2 = make_member(room_id, agent2.id, Some("Bob"), "Developer");

        let members = vec![
            RoomMemberWithAgent {
                member: member1.clone(),
                agent: agent1.clone(),
            },
            RoomMemberWithAgent {
                member: member2.clone(),
                agent: agent2.clone(),
            },
        ];

        let room = make_room(room_id, "Test Room");
        let ctx = build_room_context(&room, &member1, &agent1, &members);

        // Should include Bob but not Alice (self)
        assert!(ctx.contains("Bob"));
        // Alice should only appear once at the top, not in the participant list
        let alice_count = ctx.matches("Alice").count();
        assert_eq!(
            alice_count, 1,
            "Alice should appear exactly once (as 'You are')"
        );
    }

    #[test]
    fn build_room_context_lists_other_participants() {
        let room_id = Uuid::new_v4();
        let members = vec![
            make_member_with_agent(room_id, "Agent1", Some("Alice"), "Designer"),
            make_member_with_agent(room_id, "Agent2", Some("Bob"), "Developer"),
            make_member_with_agent(room_id, "Agent3", Some("Carol"), "Tester"),
        ];

        let room = make_room(room_id, "Test Room");
        let ctx = build_room_context(&room, &members[0].member, &members[0].agent, &members);

        // Bob and Carol should be listed
        assert!(ctx.contains("Bob"));
        assert!(ctx.contains("Carol"));
        assert!(ctx.contains("Developer"));
        assert!(ctx.contains("Tester"));
    }

    #[test]
    fn build_room_context_includes_discussion_guidelines() {
        let room = make_room(Uuid::new_v4(), "Test Room");
        let agent = make_agent(Uuid::new_v4(), "TestAgent");
        let member = make_member(room.id, agent.id, None, "Helper");
        let members = vec![RoomMemberWithAgent {
            member: member.clone(),
            agent: agent.clone(),
        }];

        let ctx = build_room_context(&room, &member, &agent, &members);

        assert!(ctx.contains("group discussion"));
        assert!(ctx.contains("concise"));
    }

    // =========================================================================
    // Transcript Formatting Tests
    // =========================================================================

    #[test]
    fn format_transcript_empty() {
        let transcript: Vec<RoomTranscriptEntry> = vec![];
        let result = format_transcript(&transcript, None);
        assert!(result.is_empty());
    }

    #[test]
    fn format_transcript_with_entries() {
        let transcript = vec![
            make_transcript_entry("Alice", "Designer", "I think we should use React."),
            make_transcript_entry("Bob", "Developer", "I agree, React is a good choice."),
        ];

        let result = format_transcript(&transcript, None);

        assert!(result.contains("Alice"));
        assert!(result.contains("Designer"));
        assert!(result.contains("I think we should use React."));
        assert!(result.contains("Bob"));
        assert!(result.contains("I agree, React is a good choice."));
        assert!(result.contains("Recent Discussion"));
    }

    #[test]
    fn format_transcript_with_summary() {
        let transcript: Vec<RoomTranscriptEntry> = vec![];
        let summary = "The team discussed the tech stack and decided on React.";

        let result = format_transcript(&transcript, Some(summary));

        assert!(result.contains("Earlier Discussion (Summary)"));
        assert!(result.contains("decided on React"));
    }

    #[test]
    fn format_transcript_summary_and_entries() {
        let transcript = vec![make_transcript_entry(
            "Carol",
            "PM",
            "Let's finalize the timeline.",
        )];
        let summary = "Previous discussion covered architecture.";

        let result = format_transcript(&transcript, Some(summary));

        assert!(result.contains("Earlier Discussion (Summary)"));
        assert!(result.contains("Previous discussion covered architecture"));
        assert!(result.contains("Recent Discussion"));
        assert!(result.contains("Carol"));
        assert!(result.contains("finalize the timeline"));
    }

    #[test]
    fn format_transcript_empty_summary_ignored() {
        let transcript: Vec<RoomTranscriptEntry> = vec![];
        let result = format_transcript(&transcript, Some(""));
        assert!(result.is_empty());
    }

    #[test]
    fn format_transcript_preserves_order() {
        let transcript = vec![
            make_transcript_entry("First", "Role1", "Message 1"),
            make_transcript_entry("Second", "Role2", "Message 2"),
            make_transcript_entry("Third", "Role3", "Message 3"),
        ];

        let result = format_transcript(&transcript, None);

        let pos_first = result.find("First").unwrap();
        let pos_second = result.find("Second").unwrap();
        let pos_third = result.find("Third").unwrap();

        assert!(pos_first < pos_second);
        assert!(pos_second < pos_third);
    }

    // =========================================================================
    // Speaker Prompt Tests
    // =========================================================================

    #[test]
    fn build_speaker_prompt_basic() {
        let result = build_speaker_prompt("What's the plan?", "", "");

        assert!(result.contains("What's the plan?"));
        assert!(result.contains("User message"));
    }

    #[test]
    fn build_speaker_prompt_with_transcript() {
        let transcript = "## Recent Discussion\n\n**Alice**: Previous comment.";
        let result = build_speaker_prompt("New question", "", transcript);

        assert!(result.contains("Recent Discussion"));
        assert!(result.contains("Alice"));
        assert!(result.contains("New question"));
        assert!(result.contains("---")); // Separator between transcript and message
    }

    #[test]
    fn build_speaker_prompt_with_followup() {
        let result = build_speaker_prompt("User message", "Focus on security concerns", "");

        assert!(result.contains("User message"));
        assert!(result.contains("Facilitator note"));
        assert!(result.contains("Focus on security concerns"));
    }

    #[test]
    fn build_speaker_prompt_all_components() {
        let transcript = "## Discussion\n\n**Alice**: Earlier point.";
        let followup = "Address the cost question";

        let result = build_speaker_prompt("Main question", followup, transcript);

        assert!(result.contains("Discussion"));
        assert!(result.contains("Alice"));
        assert!(result.contains("Main question"));
        assert!(result.contains("Facilitator note"));
        assert!(result.contains("cost question"));
    }

    #[test]
    fn build_speaker_prompt_no_followup_note_when_empty() {
        let result = build_speaker_prompt("Question", "", "Transcript block");

        assert!(!result.contains("Facilitator note"));
    }

    // =========================================================================
    // Type Tests
    // =========================================================================

    #[test]
    fn speaker_result_is_clone() {
        use crate::server::executors::room::SpeakerResult;

        let result = SpeakerResult {
            agent_id: Uuid::new_v4(),
            agent_name: "Test".to_string(),
            content: "Response".to_string(),
            input_tokens: 100,
            output_tokens: 50,
            speaker_order: 0,
        };

        let cloned = result.clone();
        assert_eq!(cloned.agent_name, "Test");
    }

    #[test]
    fn room_turn_result_is_clone() {
        use crate::server::executors::room::RoomTurnResult;

        let result = RoomTurnResult {
            turn_number: 1,
            speakers: vec![],
            session_completed: false,
        };

        let cloned = result.clone();
        assert_eq!(cloned.turn_number, 1);
        assert!(!cloned.session_completed);
    }

    // =========================================================================
    // DAG Room Prompt Tests
    // =========================================================================

    use crate::server::executors::room::build_dag_room_prompt;

    #[test]
    fn build_dag_room_prompt_first_round() {
        let prompt = "Analyze the architecture of this system.";
        let result = build_dag_room_prompt(prompt, 0, 5);

        // First round of multi-round: returns composed prompt verbatim
        assert_eq!(result, prompt);
    }

    #[test]
    fn build_dag_room_prompt_middle_round() {
        let result = build_dag_room_prompt("Initial prompt", 2, 5);

        // Middle rounds give continuation prompt
        assert!(result.contains("Continue the discussion"));
        assert!(result.contains("perspectives not yet explored"));
        // Should NOT contain the original prompt
        assert!(!result.contains("Initial prompt"));
    }

    #[test]
    fn build_dag_room_prompt_final_round() {
        let result = build_dag_room_prompt("Initial prompt", 4, 5);

        // Final round (index 4 of 5 = last)
        assert!(result.contains("final round"));
        assert!(result.contains("Summarize"));
        assert!(result.contains("final recommendation"));
        assert!(!result.contains("Initial prompt"));
    }

    #[test]
    fn build_dag_room_prompt_single_round() {
        let prompt = "Give your complete analysis.";
        let result = build_dag_room_prompt(prompt, 0, 1);

        // Single round: includes original prompt + "only round" signal
        assert!(result.contains(prompt));
        assert!(result.contains("only round"));
        assert!(result.contains("complete analysis"));
    }
}
