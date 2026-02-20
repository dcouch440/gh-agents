#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use uuid::Uuid;

    use crate::db::fixtures::fixtures::*;
    use crate::db::{TaskAgentRosterRow, TaskMissionBriefRow, WorkflowStepEdgeRow};

    use super::super::*;

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
        envelopes.insert(id, envelope(serde_json::json!({"key": "value"})));

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

    // ── Workforce dependency tests ───────────────────────────────────────────

    use crate::db::traits::MockToolCapabilityRepo;

    fn make_brief(step_id: Uuid) -> TaskMissionBriefRow {
        TaskMissionBriefRow {
            task_description: "Test mission".to_string(),
            failure_mode: "fail_fast".to_string(),
            ..brief(step_id)
        }
    }

    fn make_roster_agent(
        brief_id: Uuid,
        name: &str,
        order: i32,
        child_step_id: Option<Uuid>,
    ) -> TaskAgentRosterRow {
        TaskAgentRosterRow {
            child_step_id,
            ..roster_agent(brief_id, name, order)
        }
    }

    #[tokio::test]
    async fn workforce_input_includes_dependencies() {
        let step_id = Uuid::new_v4();
        let brief = make_brief(step_id);
        let scanner_child = Uuid::new_v4();
        let analyzer_child = Uuid::new_v4();

        let roster = vec![
            make_roster_agent(brief.id, "Scanner", 0, Some(scanner_child)),
            make_roster_agent(brief.id, "Analyzer", 1, Some(analyzer_child)),
        ];

        let child_edges = vec![WorkflowStepEdgeRow {
            id: Uuid::new_v4(),
            from_step_id: scanner_child,
            to_step_id: analyzer_child,
            workflow_id: Uuid::new_v4(),
            ..Default::default()
        }];

        let mut tool_repo = MockToolCapabilityRepo::new();
        tool_repo
            .expect_get_tool_capabilities()
            .returning(|| Ok(vec![]));

        let input = workforce::build_workforce_designer_input(
            &brief,
            &roster,
            &HashMap::new(),
            &[],
            None,
            &tool_repo,
            &child_edges,
        )
        .await;

        assert_eq!(input.dependencies.len(), 1);
        assert_eq!(input.dependencies[0].from_agent_name, "Scanner");
        assert_eq!(input.dependencies[0].to_agent_name, "Analyzer");
        assert!(input.archetype_guidance.contains("Scanner"));
        assert!(input.archetype_guidance.contains("Analyzer"));
        assert!(input.archetype_guidance.contains("receives_from"));
    }

    #[tokio::test]
    async fn workforce_input_filters_designer_edges() {
        let step_id = Uuid::new_v4();
        let brief = make_brief(step_id);
        let designer_step = Uuid::new_v4();
        let scanner_child = Uuid::new_v4();

        let roster = vec![make_roster_agent(
            brief.id,
            "Scanner",
            0,
            Some(scanner_child),
        )];

        // Designer → Scanner edge (should be filtered out)
        let child_edges = vec![WorkflowStepEdgeRow {
            id: Uuid::new_v4(),
            from_step_id: designer_step,
            to_step_id: scanner_child,
            workflow_id: Uuid::new_v4(),
            ..Default::default()
        }];

        let mut tool_repo = MockToolCapabilityRepo::new();
        tool_repo
            .expect_get_tool_capabilities()
            .returning(|| Ok(vec![]));

        let input = workforce::build_workforce_designer_input(
            &brief,
            &roster,
            &HashMap::new(),
            &[],
            None,
            &tool_repo,
            &child_edges,
        )
        .await;

        assert!(input.dependencies.is_empty());
        assert!(input
            .archetype_guidance
            .contains("No inter-agent dependencies"));
    }

    #[tokio::test]
    async fn workforce_input_no_dependencies() {
        let step_id = Uuid::new_v4();
        let brief = make_brief(step_id);

        let roster = vec![
            make_roster_agent(brief.id, "Scanner", 0, Some(Uuid::new_v4())),
            make_roster_agent(brief.id, "Analyzer", 1, Some(Uuid::new_v4())),
        ];

        let mut tool_repo = MockToolCapabilityRepo::new();
        tool_repo
            .expect_get_tool_capabilities()
            .returning(|| Ok(vec![]));

        let input = workforce::build_workforce_designer_input(
            &brief,
            &roster,
            &HashMap::new(),
            &[],
            None,
            &tool_repo,
            &[], // No edges
        )
        .await;

        assert!(input.dependencies.is_empty());
        assert!(input
            .archetype_guidance
            .contains("No inter-agent dependencies"));
    }
}
