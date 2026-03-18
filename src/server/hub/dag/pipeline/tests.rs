#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use uuid::Uuid;

    use crate::db::fixtures::fixtures::*;
    use crate::db::TaskAgentRosterRow;
    use crate::server::hub::dag::pipeline::{
        build_filtered_outputs_block, build_upstream_outputs_block, compose_workforce_output,
        compute_execution_levels, filter_outputs_for_agent, DesignedAgentPrompt,
    };

    // ── Output Composition ────────────────────────────────────────────────────

    fn make_designed_prompt(name: &str, receives_from: &[&str]) -> DesignedAgentPrompt {
        DesignedAgentPrompt {
            agent_roster_entry_id: Uuid::new_v4(),
            agent_name: name.to_string(),
            tools: vec![],
            system_prompt: String::new(),
            assignment: String::new(),
            expected_output: None,
            execution_order: 0,
            receives_from: receives_from.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn compose_workforce_output_includes_agents() {
        let agent_outputs = vec![
            ("Scanner".to_string(), "scan results".to_string()),
            ("Writer".to_string(), "written docs".to_string()),
        ];

        let result = compose_workforce_output(&agent_outputs);

        assert!(result["agents"]["scanner"].is_string());
        assert_eq!(result["agents"]["scanner"], "scan results");
        assert_eq!(result["agents"]["writer"], "written docs");
    }

    #[test]
    fn filter_outputs_empty_receives_from_returns_all() {
        let outputs = vec![
            ("A".to_string(), "a_out".to_string()),
            ("B".to_string(), "b_out".to_string()),
        ];
        let filtered = filter_outputs_for_agent(&outputs, &[]);
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn filter_outputs_with_receives_from_filters() {
        let outputs = vec![
            ("Scanner".to_string(), "scan".to_string()),
            ("Writer".to_string(), "write".to_string()),
        ];
        let filtered = filter_outputs_for_agent(&outputs, &["Scanner".to_string()]);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].0, "Scanner");
    }

    #[test]
    fn build_filtered_outputs_block_empty() {
        let result = build_filtered_outputs_block(&[]);
        assert!(result.contains("No previous agent outputs"));
    }

    #[test]
    fn build_filtered_outputs_block_with_outputs() {
        let outputs = vec![
            ("Agent A".to_string(), "output a".to_string()),
            ("Agent B".to_string(), "output b".to_string()),
        ];
        let refs: Vec<&(String, String)> = outputs.iter().collect();
        let result = build_filtered_outputs_block(&refs);
        assert!(result.contains("### Agent A"));
        assert!(result.contains("output a"));
        assert!(result.contains("### Agent B"));
    }

    // ── Execution Level Scheduling ────────────────────────────────────────────

    #[test]
    fn compute_levels_parallel_researchers() {
        // 3 researchers (no receives_from) + 1 synthesizer (receives from all 3)
        let prompts = vec![
            make_designed_prompt("FewShotResearcher", &[]),
            make_designed_prompt("PersonalityResearcher", &[]),
            make_designed_prompt("BestPracticesResearcher", &[]),
            make_designed_prompt(
                "Synthesizer",
                &[
                    "FewShotResearcher",
                    "PersonalityResearcher",
                    "BestPracticesResearcher",
                ],
            ),
        ];

        let levels = compute_execution_levels(&prompts);

        assert_eq!(levels.len(), 2);
        assert_eq!(levels[0].len(), 3); // All 3 researchers at level 0
        assert_eq!(levels[1], vec![3]); // Synthesizer at level 1
                                        // Researchers should be indices 0, 1, 2 in some order
        let mut level_0 = levels[0].clone();
        level_0.sort();
        assert_eq!(level_0, vec![0, 1, 2]);
    }

    #[test]
    fn compute_levels_linear_pipeline() {
        // A → B → C
        let prompts = vec![
            make_designed_prompt("Scanner", &[]),
            make_designed_prompt("Analyzer", &["Scanner"]),
            make_designed_prompt("Reporter", &["Analyzer"]),
        ];

        let levels = compute_execution_levels(&prompts);

        assert_eq!(levels.len(), 3);
        assert_eq!(levels[0], vec![0]); // Scanner
        assert_eq!(levels[1], vec![1]); // Analyzer
        assert_eq!(levels[2], vec![2]); // Reporter
    }

    #[test]
    fn compute_levels_diamond() {
        // A → B, A → C, B → D, C → D
        let prompts = vec![
            make_designed_prompt("A", &[]),
            make_designed_prompt("B", &["A"]),
            make_designed_prompt("C", &["A"]),
            make_designed_prompt("D", &["B", "C"]),
        ];

        let levels = compute_execution_levels(&prompts);

        assert_eq!(levels.len(), 3);
        assert_eq!(levels[0], vec![0]); // A
        let mut level_1 = levels[1].clone();
        level_1.sort();
        assert_eq!(level_1, vec![1, 2]); // B, C in parallel
        assert_eq!(levels[2], vec![3]); // D
    }

    #[test]
    fn compute_levels_no_dependencies() {
        // All agents independent
        let prompts = vec![
            make_designed_prompt("A", &[]),
            make_designed_prompt("B", &[]),
            make_designed_prompt("C", &[]),
        ];

        let levels = compute_execution_levels(&prompts);

        assert_eq!(levels.len(), 1);
        let mut level_0 = levels[0].clone();
        level_0.sort();
        assert_eq!(level_0, vec![0, 1, 2]);
    }

    #[test]
    fn compute_levels_empty() {
        let levels = compute_execution_levels(&[]);
        assert!(levels.is_empty());
    }

    #[test]
    fn compute_levels_single_agent() {
        let prompts = vec![make_designed_prompt("Solo", &[])];
        let levels = compute_execution_levels(&prompts);
        assert_eq!(levels.len(), 1);
        assert_eq!(levels[0], vec![0]);
    }

    #[test]
    fn compute_levels_sorts_within_level_by_execution_order() {
        // Three parallel agents with different execution_order, added in reverse
        let prompts = vec![
            DesignedAgentPrompt {
                execution_order: 2,
                ..make_designed_prompt("C", &[])
            },
            DesignedAgentPrompt {
                execution_order: 0,
                ..make_designed_prompt("A", &[])
            },
            DesignedAgentPrompt {
                execution_order: 1,
                ..make_designed_prompt("B", &[])
            },
        ];

        let levels = compute_execution_levels(&prompts);

        assert_eq!(levels.len(), 1);
        // Sorted by execution_order: A(idx=1, order=0), B(idx=2, order=1), C(idx=0, order=2)
        assert_eq!(levels[0], vec![1, 2, 0]);
    }

    // ── Static Fallback ───────────────────────────────────────────────────────

    #[test]
    fn static_fallback_creates_sequential_receives_from() {
        use super::super::designer::build_static_fallback_prompts;
        use crate::db::TaskMissionBriefRow;

        let brief_id = Uuid::new_v4();
        let brief = TaskMissionBriefRow {
            id: brief_id,
            task_description: "Test task".into(),
            ..Default::default()
        };
        let roster = vec![
            roster_agent(brief_id, "Scanner", 0),
            roster_agent(brief_id, "Analyzer", 1),
            roster_agent(brief_id, "Reporter", 2),
        ];

        let prompts = build_static_fallback_prompts(&brief, &roster, "base prompt");

        assert_eq!(prompts.len(), 3);
        // First agent has no receives_from (root)
        assert!(prompts[0].receives_from.is_empty());
        // Second receives from first
        assert_eq!(prompts[1].receives_from, vec!["Scanner"]);
        // Third receives from second
        assert_eq!(prompts[2].receives_from, vec!["Analyzer"]);

        // System prompt is simple: "You are {name}. {role}"
        assert!(prompts[0].system_prompt.starts_with("You are Scanner."));
        assert!(!prompts[0].system_prompt.contains("<role>"));
        assert!(!prompts[0].system_prompt.contains("<mission>"));
        assert!(!prompts[0].system_prompt.contains("<team>"));

        // Assignment includes task description
        assert!(prompts[0].assignment.contains("Test task"));
    }

    // ── Edge Routing Enforcement ──────────────────────────────────────────────

    #[test]
    fn enforce_edge_routing_overrides_designer_receives_from() {
        use super::super::designer::enforce_edge_routing;
        use crate::db::{TaskAgentRosterRow, WorkflowStepEdgeRow};

        let step_a = Uuid::new_v4();
        let step_b = Uuid::new_v4();
        let step_c = Uuid::new_v4();

        let roster = vec![
            TaskAgentRosterRow {
                name: "A".into(),
                child_step_id: Some(step_a),
                execution_order: 0,
                ..Default::default()
            },
            TaskAgentRosterRow {
                name: "B".into(),
                child_step_id: Some(step_b),
                execution_order: 1,
                ..Default::default()
            },
            TaskAgentRosterRow {
                name: "C".into(),
                child_step_id: Some(step_c),
                execution_order: 2,
                ..Default::default()
            },
        ];

        // Edges: A → B → C
        let edges = vec![
            WorkflowStepEdgeRow {
                from_step_id: step_a,
                to_step_id: step_b,
                ..Default::default()
            },
            WorkflowStepEdgeRow {
                from_step_id: step_b,
                to_step_id: step_c,
                ..Default::default()
            },
        ];

        // Designer gave wrong receives_from (all parallel)
        let mut prompts = vec![
            make_designed_prompt("A", &[]),
            make_designed_prompt("B", &[]),
            make_designed_prompt("C", &[]),
        ];

        enforce_edge_routing(&mut prompts, &roster, &edges);

        // A has no incoming edges → root
        assert!(prompts[0].receives_from.is_empty());
        // B receives from A
        assert_eq!(prompts[1].receives_from, vec!["A"]);
        // C receives from B
        assert_eq!(prompts[2].receives_from, vec!["B"]);
    }

    #[test]
    fn enforce_edge_routing_noop_when_no_edges() {
        use super::super::designer::enforce_edge_routing;
        use crate::db::TaskAgentRosterRow;

        let roster = vec![TaskAgentRosterRow {
            name: "A".into(),
            ..Default::default()
        }];

        // Designer set receives_from — should be preserved when no edges
        let mut prompts = vec![make_designed_prompt("A", &["SomeAgent"])];

        enforce_edge_routing(&mut prompts, &roster, &[]);

        assert_eq!(prompts[0].receives_from, vec!["SomeAgent"]);
    }

    // ── Upstream Outputs Block ────────────────────────────────────────────────

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

    // ── TaskPromptBuilder (A5 — 3-block format) ──────────────────────────────

    #[test]
    fn task_prompt_builder_three_blocks() {
        use super::super::agent_executor::TaskPromptBuilder;

        let prompt = TaskPromptBuilder {
            previous_step: "Prior output text".to_string(),
            assignment: "Do the thing".to_string(),
            expected_output: Some("Describe what you did".to_string()),
        }
        .build();

        assert!(prompt.contains("<previous_step>\nPrior output text\n</previous_step>"));
        assert!(prompt.contains("<assignment>\nDo the thing\n</assignment>"));
        assert!(prompt.contains("<expected_output>\nDescribe what you did\n</expected_output>"));
    }

    #[test]
    fn task_prompt_builder_omits_empty_previous_step() {
        use super::super::agent_executor::TaskPromptBuilder;

        let prompt = TaskPromptBuilder {
            previous_step: String::new(),
            assignment: "Do the thing".to_string(),
            expected_output: None,
        }
        .build();

        assert!(!prompt.contains("<previous_step>"));
        assert!(prompt.starts_with("<assignment>"));
    }

    #[test]
    fn task_prompt_builder_omits_empty_expected_output() {
        use super::super::agent_executor::TaskPromptBuilder;

        let prompt = TaskPromptBuilder {
            previous_step: String::new(),
            assignment: "Do the thing".to_string(),
            expected_output: Some(String::new()),
        }
        .build();

        assert!(!prompt.contains("<expected_output>"));
    }

    #[test]
    fn task_prompt_builder_no_old_blocks() {
        use super::super::agent_executor::TaskPromptBuilder;

        let prompt = TaskPromptBuilder {
            previous_step: "output".to_string(),
            assignment: "task".to_string(),
            expected_output: Some("result".to_string()),
        }
        .build();

        // None of the old block tags should appear
        assert!(!prompt.contains("<context>"));
        assert!(!prompt.contains("<upstream_artifacts>"));
        assert!(!prompt.contains("<previous_agent_outputs>"));
        assert!(!prompt.contains("<upstream_step_outputs>"));
        assert!(!prompt.contains("<user_notes>"));
    }

    #[test]
    fn task_prompt_builder_block_order() {
        use super::super::agent_executor::TaskPromptBuilder;

        let prompt = TaskPromptBuilder {
            previous_step: "prev".to_string(),
            assignment: "assign".to_string(),
            expected_output: Some("expect".to_string()),
        }
        .build();

        let prev_pos = prompt.find("<previous_step>").unwrap();
        let assign_pos = prompt.find("<assignment>").unwrap();
        let expect_pos = prompt.find("<expected_output>").unwrap();
        assert!(prev_pos < assign_pos);
        assert!(assign_pos < expect_pos);
    }

    #[test]
    fn static_fallback_simple_system_prompt() {
        use super::super::designer::build_static_fallback_prompts;
        use crate::db::TaskMissionBriefRow;

        let brief_id = Uuid::new_v4();
        let brief = TaskMissionBriefRow {
            id: brief_id,
            task_description: "Scan for vulnerabilities".into(),
            ..Default::default()
        };
        let roster = vec![roster_agent(brief_id, "Scanner", 0)];

        let prompts = build_static_fallback_prompts(&brief, &roster, "base prompt");

        // System prompt is short: "You are {name}. {role}"
        assert!(prompts[0].system_prompt.starts_with("You are Scanner."));
        assert!(!prompts[0].system_prompt.contains("<role>"));
        assert!(!prompts[0].system_prompt.contains("<mission>"));
        assert!(!prompts[0].system_prompt.contains("<team>"));
        assert!(!prompts[0].system_prompt.contains("<upstream_outputs>"));
        assert!(!prompts[0].system_prompt.contains("<instructions>"));
    }

    #[test]
    fn static_fallback_assignment_includes_task_description() {
        use super::super::designer::build_static_fallback_prompts;
        use crate::db::TaskMissionBriefRow;

        let brief_id = Uuid::new_v4();
        let brief = TaskMissionBriefRow {
            id: brief_id,
            task_description: "Scan for vulnerabilities".into(),
            ..Default::default()
        };
        let roster = vec![roster_agent(brief_id, "Scanner", 0)];

        let prompts = build_static_fallback_prompts(&brief, &roster, "base prompt");

        // Assignment should contain both task_description and role_description
        assert!(prompts[0].assignment.contains("Scan for vulnerabilities"));
        assert!(prompts[0].assignment.contains(&roster[0].role_description));
    }
}
