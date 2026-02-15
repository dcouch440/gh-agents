#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use chrono::Utc;
    use uuid::Uuid;

    use crate::db::{
        BeliefRow, ProtocolDocumentDefRow, TaskAgentRosterRow, TaskMissionBriefRow, WorkflowStepRow,
    };
    use crate::server::hub::dag::documenter::types::DocumentPlan;
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
            },
            error: None,
        }
    }

    fn make_step() -> WorkflowStepRow {
        WorkflowStepRow {
            id: Uuid::new_v4(),
            workflow_id: Uuid::new_v4(),
            agent_id: None,
            execution_mode: "documenter".to_string(),
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
        }
    }

    fn make_doc_def(name: &str, target_length: i32) -> ProtocolDocumentDefRow {
        ProtocolDocumentDefRow {
            id: Uuid::new_v4(),
            step_id: Some(Uuid::new_v4()),
            name: name.to_string(),
            description: format!("Description for {}", name),
            target_length,
            display_order: 0,
            created_at: Utc::now(),
            protocol_id: None,
            document_id: None,
        }
    }

    fn make_document_plan(name: &str, strategy: &str, writer: &str) -> DocumentPlan {
        DocumentPlan {
            document_name: name.to_string(),
            research_strategy: strategy.to_string(),
            required_capabilities: vec!["file_read".to_string(), "grep".to_string()],
            writer_prompt: writer.to_string(),
            context_document_ids: vec![],
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
            "grep".to_string(),
            "shell".to_string(),
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

    // ── Task force formatter tests ───────────────────────────────────────────

    #[test]
    fn test_task_force_input_basic() {
        let brief = make_brief(
            "Audit codebase for security issues",
            vec!["file_read".to_string(), "grep".to_string()],
        );
        let roster = vec![
            make_roster_entry("Scanner", "Search for vulnerabilities", 0),
            make_roster_entry("Analyzer", "Evaluate findings", 1),
            make_roster_entry("Reporter", "Write audit report", 2),
        ];

        let input = task_force::build_task_force_designer_input(
            &brief,
            &roster,
            &HashMap::new(),
            &[],
            None,
        );

        assert_eq!(input.archetype, "task_force");
        assert_eq!(input.agents.len(), 3);
        assert_eq!(input.agents[0].name, "Scanner");
        assert_eq!(input.agents[1].name, "Analyzer");
        assert_eq!(input.agents[2].name, "Reporter");
        assert_eq!(input.available_tools.len(), 2);
        assert!(input.archetype_guidance.contains("fail_fast"));
        assert!(input.archetype_guidance.contains("Engineering lead"));
    }

    #[test]
    fn test_task_force_input_no_upstream() {
        let brief = make_brief("Test task", vec![]);
        let roster = vec![make_roster_entry("Agent", "Do stuff", 0)];

        let input = task_force::build_task_force_designer_input(
            &brief,
            &roster,
            &HashMap::new(),
            &[],
            None,
        );

        assert_eq!(input.upstream.len(), 1);
        assert!(input.upstream[0].content.contains("No upstream"));
    }

    #[test]
    fn test_task_force_input_preserves_execution_order() {
        let brief = make_brief("Test", vec![]);
        let roster = vec![
            make_roster_entry("Third", "role", 2),
            make_roster_entry("First", "role", 0),
            make_roster_entry("Second", "role", 1),
        ];

        let input = task_force::build_task_force_designer_input(
            &brief,
            &roster,
            &HashMap::new(),
            &[],
            None,
        );

        assert_eq!(input.agents[0].execution_order, 2);
        assert_eq!(input.agents[1].execution_order, 0);
        assert_eq!(input.agents[2].execution_order, 1);
    }

    // ── Documenter formatter tests ───────────────────────────────────────────

    #[test]
    fn test_strategist_input_single_agent() {
        let step = make_step();
        let doc_defs = vec![
            make_doc_def("API Reference", 3000),
            make_doc_def("Data Model", 1500),
        ];

        let input = documenter::build_strategist_designer_input(
            &step,
            &doc_defs,
            &HashMap::new(),
            &["file_read".to_string()],
            &[],
            None,
        );

        assert_eq!(input.archetype, "documenter");
        assert_eq!(input.agents.len(), 1);
        assert_eq!(input.agents[0].name, "Document Strategist");
        assert!(input.agents[0].capabilities.is_empty());
        assert!(input.agents[0].additional_context.contains("API Reference"));
        assert!(input.agents[0].additional_context.contains("Data Model"));
        assert!(input.archetype_guidance.contains("Phase 1"));
        assert!(input.context_description.contains("2 reference documents"));
    }

    #[test]
    fn test_research_write_input_creates_2n_agents() {
        let step = make_step();
        let plans = vec![
            make_document_plan(
                "API Reference",
                "Search for endpoints",
                "Write in reference style",
            ),
            make_document_plan(
                "Data Model",
                "Examine schema files",
                "Write entity descriptions",
            ),
        ];

        let input = documenter::build_research_write_designer_input(
            &step,
            &plans,
            &HashMap::new(),
            &["file_read".to_string(), "grep".to_string()],
            &[],
            None,
        );

        assert_eq!(input.agents.len(), 4);

        // Researchers first
        assert_eq!(input.agents[0].id, "researcher:API Reference");
        assert_eq!(input.agents[0].execution_order, 0);
        assert_eq!(input.agents[1].id, "researcher:Data Model");
        assert_eq!(input.agents[1].execution_order, 1);

        // Writers after
        assert_eq!(input.agents[2].id, "writer:API Reference");
        assert_eq!(input.agents[2].execution_order, 2);
        assert_eq!(input.agents[3].id, "writer:Data Model");
        assert_eq!(input.agents[3].execution_order, 3);
    }

    #[test]
    fn test_research_write_input_researcher_gets_capabilities() {
        let step = make_step();
        let plans = vec![make_document_plan("Doc", "Research it", "Write it")];

        let input = documenter::build_research_write_designer_input(
            &step,
            &plans,
            &HashMap::new(),
            &["file_read".to_string()],
            &[],
            None,
        );

        // Researcher has capabilities from the plan
        assert_eq!(
            input.agents[0].capabilities,
            vec!["file_read".to_string(), "grep".to_string()]
        );
        // Writer has no capabilities
        assert!(input.agents[1].capabilities.is_empty());
    }

    #[test]
    fn test_research_write_input_includes_strategist_guidance() {
        let step = make_step();
        let plans = vec![make_document_plan(
            "API Reference",
            "Search for all REST endpoints in src/api/",
            "Write comprehensive endpoint documentation",
        )];

        let input = documenter::build_research_write_designer_input(
            &step,
            &plans,
            &HashMap::new(),
            &[],
            &[],
            None,
        );

        assert!(input.agents[0]
            .additional_context
            .contains("Search for all REST endpoints"));
        assert!(input.agents[1]
            .additional_context
            .contains("Write comprehensive endpoint documentation"));
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
    fn test_task_force_input_includes_assistant_notes() {
        let brief = make_brief("Test task", vec![]);
        let roster = vec![make_roster_entry("Agent", "Do stuff", 0)];

        let input = task_force::build_task_force_designer_input(
            &brief,
            &roster,
            &HashMap::new(),
            &[],
            Some("## Direction\n- Build auth system"),
        );

        let notes_entry = input
            .upstream
            .iter()
            .find(|u| u.source_type == "agent_notes");
        assert!(notes_entry.is_some());
        let entry = notes_entry.unwrap();
        assert_eq!(entry.source_name, "Assistant's Notes");
        assert!(entry.content.contains("Build auth system"));
    }

    #[test]
    fn test_task_force_input_skips_empty_notes() {
        let brief = make_brief("Test task", vec![]);
        let roster = vec![make_roster_entry("Agent", "Do stuff", 0)];

        let input = task_force::build_task_force_designer_input(
            &brief,
            &roster,
            &HashMap::new(),
            &[],
            Some(""),
        );

        assert!(!input
            .upstream
            .iter()
            .any(|u| u.source_type == "agent_notes"));
    }

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
