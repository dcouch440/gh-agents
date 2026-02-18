#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use chrono::Utc;
    use uuid::Uuid;

    use crate::db::{BeliefRow, TaskAgentRosterRow, TaskMissionBriefRow, WorkflowStepRow};
    use crate::types::{ExecutionMetadata, ExecutionStatus, StepExecutionEnvelope};

    use super::super::*;

    // ── Test helpers ─────────────────────────────────────────────────────────

    fn make_brief(task: &str, capabilities: Vec<String>) -> TaskMissionBriefRow {
        TaskMissionBriefRow {
            id: Uuid::new_v4(),
            step_id: Uuid::new_v4(),
            task_description: task.to_string(),
            available_capabilities: capabilities,
            failure_mode: "fail_fast".to_string(),
            downstream_context: Some("Engineering lead triages findings.".to_string()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn make_roster_entry(name: &str, role: &str, order: i32) -> TaskAgentRosterRow {
        TaskAgentRosterRow {
            id: Uuid::new_v4(),
            mission_brief_id: Uuid::new_v4(),
            name: name.to_string(),
            role_description: role.to_string(),
            capabilities: vec![],
            execution_order: order,
            created_at: Utc::now(),
            child_step_id: None,
        }
    }

    fn make_envelope(data: serde_json::Value) -> StepExecutionEnvelope {
        StepExecutionEnvelope {
            status: ExecutionStatus::Success,
            data: Some(data),
            metadata: ExecutionMetadata {
                execution_id: Uuid::new_v4(),
                execution_time_ms: 100,
                tokens_in: Some(50),
                tokens_out: Some(25),
                cost_usd: Some(0.001),
                model: Some("test-model".to_string()),
                agent_id: None,
                iteration_index: None,
                iteration_label: None,
                routing_label: None,
                upstream_agent_id: None,
                upstream_routing_label: None,
                room_session_id: None,
                room_id: None,
                total_rounds: None,
                child_workflow_execution_id: None,
            },
            error: None,
        }
    }

    fn make_step() -> WorkflowStepRow {
        WorkflowStepRow {
            id: Uuid::new_v4(),
            workflow_id: Uuid::new_v4(),
            agent_id: None,
            execution_mode: "workforce".to_string(),
            agent_execution_mode: None,
            for_each_ref: None,
            prompt_template_id: None,
            prompt_template: String::new(),
            output_schema_id: None,
            output_variable_name: None,
            interactive_agent_id: None,
            for_each_label_field: None,
            room_id: None,
            routing_mode: None,
            routing_field: None,
            display_order: 0,
            version: 1,
            reasoning_trace: false,
            verification_agent_ids: None,
            position_x: None,
            position_y: None,
            width: None,
            height: None,
            name: Some("Test Step".to_string()),
            system_prompt_suffix: None,
            visible: true,
            description: String::new(),
            board_context_cache: String::new(),
            board_context_updated_at: None,
            goal_summary: String::new(),
            goal_summary_updated_at: None,
            sub_workflow_template_id: None,
            child_workflow_id: None,
            is_designer_step: false,
        }
    }

    fn make_room_member(name: &str, role: &str, perspective: &str) -> RoomDesignerMember {
        RoomDesignerMember {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            role: role.to_string(),
            perspective: perspective.to_string(),
        }
    }

    fn make_belief(content: &str, belief_type: &str, confidence: &str) -> BeliefRow {
        BeliefRow {
            id: Uuid::new_v4(),
            workflow_id: Uuid::new_v4(),
            workflow_execution_id: Some(Uuid::new_v4()),
            source_step_id: Uuid::new_v4(),
            source_document_title: None,
            source_document_def_id: None,
            source_phase: "research".to_string(),
            content: content.to_string(),
            reasoning: "Test reasoning".to_string(),
            belief_type: belief_type.to_string(),
            confidence: confidence.to_string(),
            confidence_justification: None,
            semantic_tags: vec![],
            emotional_tone: None,
            cross_source_tension: None,
            source_step_name: "Test Source".to_string(),
            extraction_model: "test-model".to_string(),
            extraction_tokens_in: 100,
            extraction_tokens_out: 50,
            created_at: Utc::now(),
        }
    }

    // ── Shared utility tests ─────────────────────────────────────────────────

    #[test]
    fn test_build_tool_descriptions_known_capabilities() {
        let tools = build_tool_descriptions(&[
            "file_read".to_string(),
            "content_search".to_string(),
            "shell_execution".to_string(),
        ]);
        assert_eq!(tools.len(), 3);
        assert!(tools[0].description.contains("Read file contents"));
        assert!(tools[1].description.contains("Search file contents"));
        assert!(tools[2].description.contains("Execute shell commands"));
    }

    #[test]
    fn test_build_tool_descriptions_unknown_capability() {
        let tools = build_tool_descriptions(&["custom_tool".to_string()]);
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "custom_tool");
        assert_eq!(tools[0].description, "custom_tool");
    }

    #[test]
    fn test_format_envelopes_as_upstream_empty() {
        let result = format_envelopes_as_upstream(&HashMap::new(), &[]);
        assert_eq!(result.len(), 1);
        assert!(result[0].content.contains("No upstream"));
        assert_eq!(result[0].source_type, "none");
    }

    #[test]
    fn test_format_envelopes_as_upstream_with_data() {
        let mut envelopes = HashMap::new();
        let id = Uuid::new_v4();
        envelopes.insert(id, make_envelope(serde_json::json!({"key": "value"})));

        let result = format_envelopes_as_upstream(&envelopes, &[]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].source_type, "step");
        assert!(result[0].content.contains("key"));
    }

    #[test]
    fn test_truncate_for_context_within_limit() {
        let content = "short string";
        assert_eq!(truncate_for_context(content, 100), content);
    }

    #[test]
    fn test_truncate_for_context_exceeds_limit() {
        let content = "this is a longer string that exceeds the limit";
        let truncated = truncate_for_context(content, 10);
        assert_eq!(truncated.len(), 10);
        assert_eq!(truncated, "this is a ");
    }

    #[test]
    fn test_truncate_for_context_respects_char_boundary() {
        // Multi-byte character: é is 2 bytes in UTF-8
        let content = "café latte";
        // Truncating at byte 4 would split the é — should back up to byte 3
        let truncated = truncate_for_context(content, 4);
        assert!(truncated.len() <= 4);
        assert!(truncated.is_char_boundary(truncated.len()));
    }

    // ── Room formatter tests ─────────────────────────────────────────────────

    #[test]
    fn test_room_input_basic() {
        let members = vec![
            make_room_member(
                "Alice",
                "Security Architect",
                "Evaluates for vulnerabilities",
            ),
            make_room_member("Bob", "Product Manager", "Ensures UX quality"),
        ];

        let input = room::build_room_designer_input(
            "Security Review",
            "moderated",
            12,
            &members,
            &[],
            &HashMap::new(),
            &[],
            None,
        );

        assert_eq!(input.archetype, "room");
        assert_eq!(input.agents.len(), 2);
        assert_eq!(input.agents[0].name, "Alice");
        assert_eq!(input.agents[1].name, "Bob");
        assert!(input.available_tools.is_empty());
        assert!(input.archetype_guidance.contains("Security Review"));
        assert!(input.archetype_guidance.contains("moderated"));
        assert!(input.archetype_guidance.contains("12"));
    }

    #[test]
    fn test_room_input_without_beliefs() {
        let members = vec![make_room_member(
            "Alice",
            "Engineer",
            "Technical perspective",
        )];

        let input = room::build_room_designer_input(
            "Review",
            "moderated",
            12,
            &members,
            &[],
            &HashMap::new(),
            &[],
            None,
        );

        assert!(!input.agents[0]
            .additional_context
            .contains("Beliefs extracted"));
        assert!(input.agents[0]
            .additional_context
            .contains("Technical perspective"));
    }

    #[test]
    fn test_room_input_with_beliefs() {
        let members = vec![
            make_room_member(
                "Alice",
                "Security Architect",
                "Evaluates for vulnerabilities",
            ),
            make_room_member("Bob", "Product Manager", "Ensures UX quality"),
        ];
        let beliefs = vec![
            make_belief(
                "OAuth 2.0 PKCE flow is recommended for mobile clients",
                "fact",
                "high",
            ),
            make_belief("Rate limiting should be per-user", "decision", "high"),
        ];

        let input = room::build_room_designer_input(
            "Security Review",
            "moderated",
            12,
            &members,
            &beliefs,
            &HashMap::new(),
            &[],
            None,
        );

        // Both members should have beliefs in their additional_context
        for agent in &input.agents {
            assert!(agent.additional_context.contains("Beliefs extracted"));
            assert!(agent.additional_context.contains("OAuth 2.0 PKCE"));
            assert!(agent.additional_context.contains("Rate limiting"));
        }
    }

    // ── Assistant notes injection tests ─────────────────────────────────────

    #[test]
    fn test_room_input_includes_perspectives() {
        let members = vec![
            make_room_member(
                "Security Architect",
                "Security",
                "Evaluates for vulnerabilities and attack surfaces",
            ),
            make_room_member(
                "Product Manager",
                "Product",
                "Ensures user experience quality",
            ),
        ];

        let input = room::build_room_designer_input(
            "Review",
            "moderated",
            12,
            &members,
            &[],
            &HashMap::new(),
            &[],
            None,
        );

        assert!(input.agents[0]
            .additional_context
            .contains("Evaluates for vulnerabilities"));
        assert!(input.agents[1]
            .additional_context
            .contains("Ensures user experience quality"));
    }
}
