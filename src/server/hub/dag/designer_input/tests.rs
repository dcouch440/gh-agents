#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use uuid::Uuid;

    use crate::config::capability_registry::CapabilityRegistry;
    use crate::db::fixtures::fixtures::*;
    use crate::db::{TaskAgentRosterRow, TaskMissionBriefRow, WorkflowStepEdgeRow};

    use super::super::*;

    // ── Shared utility tests ─────────────────────────────────────────────────

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

    #[test]
    fn workforce_input_includes_dependencies() {
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

        let registry = CapabilityRegistry::empty();

        let input = workforce::build_workforce_designer_input(
            &brief,
            &roster,
            &HashMap::new(),
            &[],
            None,
            &registry,
            &child_edges,
        );

        assert!(input.archetype_guidance.contains("Scanner"));
        assert!(input.archetype_guidance.contains("Analyzer"));
        assert!(input.archetype_guidance.contains("runs before"));
    }

    #[test]
    fn workforce_input_filters_designer_edges() {
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

        let registry = CapabilityRegistry::empty();

        let input = workforce::build_workforce_designer_input(
            &brief,
            &roster,
            &HashMap::new(),
            &[],
            None,
            &registry,
            &child_edges,
        );

        assert!(input
            .archetype_guidance
            .contains("all agents run in parallel"));
    }

    // ── Upstream outputs block tests ─────────────────────────────────────────

    #[test]
    fn upstream_outputs_block_empty_envelopes() {
        let result = build_upstream_outputs_block(&HashMap::new(), &[]);
        assert!(result.is_empty());
    }

    #[test]
    fn upstream_outputs_block_excludes_context_steps() {
        use crate::db::WorkflowStepRow;

        let step_id = Uuid::new_v4();
        let step = WorkflowStepRow {
            id: step_id,
            execution_mode: "context".to_string(),
            name: Some("My Context".to_string()),
            ..Default::default()
        };

        let mut envelopes = HashMap::new();
        envelopes.insert(step_id, envelope(serde_json::json!({"key": "value"})));

        let result = build_upstream_outputs_block(&envelopes, &[step]);
        assert!(result.is_empty());
    }

    #[test]
    fn upstream_outputs_block_excludes_input_steps() {
        use crate::db::WorkflowStepRow;

        let step_id = Uuid::new_v4();
        let step = WorkflowStepRow {
            id: step_id,
            execution_mode: "input".to_string(),
            name: Some("User Input".to_string()),
            ..Default::default()
        };

        let mut envelopes = HashMap::new();
        envelopes.insert(step_id, envelope(serde_json::json!("some input")));

        let result = build_upstream_outputs_block(&envelopes, &[step]);
        assert!(result.is_empty());
    }

    #[test]
    fn upstream_outputs_block_includes_workforce_step() {
        use crate::db::WorkflowStepRow;

        let step_id = Uuid::new_v4();
        let step = WorkflowStepRow {
            id: step_id,
            execution_mode: "workforce".to_string(),
            name: Some("Research Team".to_string()),
            ..Default::default()
        };

        let mut envelopes = HashMap::new();
        envelopes.insert(
            step_id,
            envelope(serde_json::json!({"agents": {"scanner": "scan results", "writer": "written content"}})),
        );

        let result = build_upstream_outputs_block(&envelopes, &[step]);
        assert!(result.contains("### Research Team"));
        assert!(result.contains("**scanner**:"));
        assert!(result.contains("scan results"));
        assert!(result.contains("**writer**:"));
        assert!(result.contains("written content"));
    }

    #[test]
    fn upstream_outputs_block_uses_output_variable_name_fallback() {
        use crate::db::WorkflowStepRow;

        let step_id = Uuid::new_v4();
        let step = WorkflowStepRow {
            id: step_id,
            execution_mode: "single".to_string(),
            name: None,
            output_variable_name: Some("research_output".to_string()),
            ..Default::default()
        };

        let mut envelopes = HashMap::new();
        envelopes.insert(step_id, envelope(serde_json::json!("some output")));

        let result = build_upstream_outputs_block(&envelopes, &[step]);
        assert!(result.contains("### research_output"));
    }

    #[test]
    fn upstream_outputs_block_mixed_steps() {
        use crate::db::WorkflowStepRow;

        let wf_id = Uuid::new_v4();
        let ctx_id = Uuid::new_v4();
        let single_id = Uuid::new_v4();

        let steps = vec![
            WorkflowStepRow {
                id: wf_id,
                execution_mode: "workforce".to_string(),
                name: Some("Research".to_string()),
                ..Default::default()
            },
            WorkflowStepRow {
                id: ctx_id,
                execution_mode: "context".to_string(),
                name: Some("Context Node".to_string()),
                ..Default::default()
            },
            WorkflowStepRow {
                id: single_id,
                execution_mode: "single".to_string(),
                name: Some("Fetcher".to_string()),
                ..Default::default()
            },
        ];

        let mut envelopes = HashMap::new();
        envelopes.insert(wf_id, envelope(serde_json::json!({"agents": {"a": "out"}})));
        envelopes.insert(ctx_id, envelope(serde_json::json!("context data")));
        envelopes.insert(single_id, envelope(serde_json::json!("fetched data")));

        let result = build_upstream_outputs_block(&envelopes, &steps);

        // Workforce and single steps included
        assert!(result.contains("### Research"));
        assert!(result.contains("### Fetcher"));
        // Context step excluded
        assert!(!result.contains("Context Node"));
    }

    #[test]
    fn upstream_outputs_block_skips_none_data() {
        use crate::db::WorkflowStepRow;

        let step_id = Uuid::new_v4();
        let step = WorkflowStepRow {
            id: step_id,
            execution_mode: "workforce".to_string(),
            name: Some("Empty".to_string()),
            ..Default::default()
        };

        let mut envelopes = HashMap::new();
        envelopes.insert(step_id, empty_envelope());

        let result = build_upstream_outputs_block(&envelopes, &[step]);
        assert!(result.is_empty());
    }

    #[test]
    fn upstream_outputs_block_truncates_large_output() {
        use crate::db::WorkflowStepRow;

        let step_id = Uuid::new_v4();
        let step = WorkflowStepRow {
            id: step_id,
            execution_mode: "single".to_string(),
            name: Some("Big Step".to_string()),
            ..Default::default()
        };

        let big_data = "x".repeat(5000);
        let mut envelopes = HashMap::new();
        envelopes.insert(step_id, envelope(serde_json::Value::String(big_data)));

        let result = build_upstream_outputs_block(&envelopes, &[step]);
        assert!(result.contains("### Big Step"));
        // Header + truncated content should be well under the raw 5000 chars
        assert!(result.len() < 4200);
    }

    #[test]
    fn workforce_input_no_dependencies() {
        let step_id = Uuid::new_v4();
        let brief = make_brief(step_id);

        let roster = vec![
            make_roster_agent(brief.id, "Scanner", 0, Some(Uuid::new_v4())),
            make_roster_agent(brief.id, "Analyzer", 1, Some(Uuid::new_v4())),
        ];

        let registry = CapabilityRegistry::empty();

        let input = workforce::build_workforce_designer_input(
            &brief,
            &roster,
            &HashMap::new(),
            &[],
            None,
            &registry,
            &[], // No edges
        );

        assert!(input
            .archetype_guidance
            .contains("all agents run in parallel"));
    }
}
