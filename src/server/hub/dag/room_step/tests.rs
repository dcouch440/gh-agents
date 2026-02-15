#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use uuid::Uuid;

    use super::super::extract_room_outputs_from_speakers;
    use crate::server::executors::room::SpeakerResult;
    use crate::server::hub::dag::agent_designer::DesignedAgentPrompt;
    use crate::server::hub::dag::designer_input::room::build_room_designer_input;
    use crate::server::hub::dag::designer_input::RoomDesignerMember;
    use crate::server::hub::dag::resolve_dot_path;

    #[test]
    fn room_composite_envelope_structure() {
        let agent_a = Uuid::new_v4();
        let agent_b = Uuid::new_v4();

        let speakers = vec![
            SpeakerResult {
                agent_id: agent_a,
                agent_name: "Architect".into(),
                content: r#"{"recommendation": "use microservices"}"#.into(),
                input_tokens: 100,
                output_tokens: 50,
                speaker_order: 0,
            },
            SpeakerResult {
                agent_id: agent_b,
                agent_name: "Reviewer".into(),
                content: "I agree with the approach.".into(),
                input_tokens: 80,
                output_tokens: 30,
                speaker_order: 1,
            },
        ];

        let (envelope_data, output) =
            extract_room_outputs_from_speakers(&speakers, Some("room_out"));

        // Verify output variable name
        assert_eq!(output.variable_name, "room_out");

        // Verify composite structure has per-agent keys
        let key_a = format!("agent:{}", agent_a);
        let key_b = format!("agent:{}", agent_b);

        // Agent A returned valid JSON — should be parsed as object
        let val_a = resolve_dot_path(&envelope_data, &key_a).unwrap();
        assert_eq!(val_a["recommendation"], "use microservices");

        // Agent B returned plain text — should be stored as string
        let val_b = resolve_dot_path(&envelope_data, &key_b).unwrap();
        assert_eq!(val_b.as_str().unwrap(), "I agree with the approach.");

        // Nested access works through the port system
        let nested_path = format!("{}.recommendation", key_a);
        let nested = resolve_dot_path(&envelope_data, &nested_path).unwrap();
        assert_eq!(nested, "use microservices");
    }

    // ── Designer input formatter integration tests ──────────────────────────

    #[test]
    fn room_designer_input_maps_members_to_agents() {
        let agent_a = Uuid::new_v4();
        let agent_b = Uuid::new_v4();

        let members = vec![
            RoomDesignerMember {
                id: agent_a.to_string(),
                name: "Alice".to_string(),
                role: "Security Architect".to_string(),
                perspective: "Evaluates for vulnerabilities".to_string(),
            },
            RoomDesignerMember {
                id: agent_b.to_string(),
                name: "Bob".to_string(),
                role: "Product Manager".to_string(),
                perspective: "Ensures UX quality".to_string(),
            },
        ];

        let input = build_room_designer_input(
            "Code Review",
            "moderated",
            8,
            &members,
            &[],
            &HashMap::new(),
            &[],
            None,
        );

        assert_eq!(input.archetype, "room");
        assert_eq!(input.agents.len(), 2);
        assert_eq!(input.agents[0].id, agent_a.to_string());
        assert_eq!(input.agents[0].name, "Alice");
        assert_eq!(input.agents[1].id, agent_b.to_string());
        assert_eq!(input.agents[1].name, "Bob");
        assert!(input.archetype_guidance.contains("Code Review"));
        assert!(input.archetype_guidance.contains("moderated"));
        assert!(input.archetype_guidance.contains("8"));
    }

    // ── Designed prompts lookup tests ───────────────────────────────────────

    #[test]
    fn designed_prompts_lookup_by_agent_id() {
        let agent_a = Uuid::new_v4();
        let agent_b = Uuid::new_v4();

        let prompts = vec![
            DesignedAgentPrompt {
                agent_id: agent_a.to_string(),
                agent_name: "Alice".to_string(),
                tools: vec![],
                system_prompt: "You are a security architect.".to_string(),
                task_prompt: "Review the code for vulnerabilities.".to_string(),
                reasoning: "Security perspective needed.".to_string(),
                execution_order: 0,
                receives_from: vec![],
            },
            DesignedAgentPrompt {
                agent_id: agent_b.to_string(),
                agent_name: "Bob".to_string(),
                tools: vec![],
                system_prompt: "You are a product manager.".to_string(),
                task_prompt: "Evaluate UX implications.".to_string(),
                reasoning: "Product perspective needed.".to_string(),
                execution_order: 1,
                receives_from: vec![],
            },
        ];

        // Build lookup the same way room_step does
        let lookup: HashMap<Uuid, DesignedAgentPrompt> = prompts
            .into_iter()
            .filter_map(|p| p.agent_id.parse::<Uuid>().ok().map(|id| (id, p)))
            .collect();

        assert_eq!(lookup.len(), 2);
        assert_eq!(
            lookup.get(&agent_a).unwrap().system_prompt,
            "You are a security architect."
        );
        assert_eq!(
            lookup.get(&agent_b).unwrap().task_prompt,
            "Evaluate UX implications."
        );
    }

    #[test]
    fn designed_prompts_lookup_skips_invalid_uuid() {
        let prompts = vec![DesignedAgentPrompt {
            agent_id: "not-a-uuid".to_string(),
            agent_name: "Broken".to_string(),
            tools: vec![],
            system_prompt: String::new(),
            task_prompt: String::new(),
            reasoning: String::new(),
            execution_order: 0,
            receives_from: vec![],
        }];

        let lookup: HashMap<Uuid, DesignedAgentPrompt> = prompts
            .into_iter()
            .filter_map(|p| p.agent_id.parse::<Uuid>().ok().map(|id| (id, p)))
            .collect();

        assert!(lookup.is_empty());
    }

    #[test]
    fn designed_prompts_fallback_when_none() {
        let designed_prompts: Option<&HashMap<Uuid, DesignedAgentPrompt>> = None;
        let agent_id = Uuid::new_v4();

        // Same pattern as execute_room_turn: and_then + get
        let result = designed_prompts.and_then(|m| m.get(&agent_id));
        assert!(result.is_none());
    }

    #[test]
    fn designed_prompts_found_when_present() {
        let agent_id = Uuid::new_v4();
        let mut lookup = HashMap::new();
        lookup.insert(
            agent_id,
            DesignedAgentPrompt {
                agent_id: agent_id.to_string(),
                agent_name: "Alice".to_string(),
                tools: vec![],
                system_prompt: "Designed system prompt".to_string(),
                task_prompt: "Designed task prompt".to_string(),
                reasoning: String::new(),
                execution_order: 0,
                receives_from: vec![],
            },
        );

        let designed_prompts = Some(&lookup);
        let result = designed_prompts.and_then(|m| m.get(&agent_id));
        assert!(result.is_some());
        assert_eq!(result.unwrap().system_prompt, "Designed system prompt");
    }
}
