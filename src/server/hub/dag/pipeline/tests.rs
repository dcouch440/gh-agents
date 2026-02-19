#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::db::{TaskAgentRosterRow, WorkflowStepRow};
    use crate::server::hub::dag::pipeline::{
        build_filtered_outputs_block, build_team_roster_string, compose_workforce_output,
        compute_execution_levels, filter_outputs_for_agent, DesignedAgentPrompt,
    };

    // ── Designer Detection ────────────────────────────────────────────────────

    /// Verify that Designer detection logic works correctly.
    /// A child workflow with an `is_designer_step = true` step is "designed",
    /// one without is "static".
    #[test]
    fn designer_detection_with_designer() {
        let child_wf_id = Uuid::new_v4();
        let designer = WorkflowStepRow {
            id: Uuid::new_v4(),
            workflow_id: child_wf_id,
            is_designer_step: true,
            ..Default::default()
        };
        let agent = WorkflowStepRow {
            id: Uuid::new_v4(),
            workflow_id: child_wf_id,
            ..Default::default()
        };

        let steps = vec![designer, agent];
        assert!(steps.iter().any(|s| s.is_designer_step));
    }

    #[test]
    fn designer_detection_without_designer() {
        let child_wf_id = Uuid::new_v4();
        let step_a = WorkflowStepRow {
            id: Uuid::new_v4(),
            workflow_id: child_wf_id,
            ..Default::default()
        };
        let step_b = WorkflowStepRow {
            id: Uuid::new_v4(),
            workflow_id: child_wf_id,
            ..Default::default()
        };

        let steps = vec![step_a, step_b];
        assert!(!steps.iter().any(|s| s.is_designer_step));
    }

    #[test]
    fn designer_detection_empty_steps() {
        let steps: Vec<WorkflowStepRow> = vec![];
        assert!(!steps.iter().any(|s| s.is_designer_step));
    }

    // ── Output Composition ────────────────────────────────────────────────────

    fn make_roster_agent(name: &str, order: i32) -> TaskAgentRosterRow {
        TaskAgentRosterRow {
            id: Uuid::new_v4(),
            mission_brief_id: Uuid::new_v4(),
            name: name.to_string(),
            role_description: format!("{} role", name),
            capabilities: vec!["file_read".to_string()],
            execution_order: order,
            ..Default::default()
        }
    }

    fn make_designed_prompt(name: &str, receives_from: &[&str]) -> DesignedAgentPrompt {
        DesignedAgentPrompt {
            agent_roster_entry_id: Uuid::new_v4(),
            agent_name: name.to_string(),
            tools: vec![],
            system_prompt: String::new(),
            task_prompt: String::new(),
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
    fn team_roster_string_includes_agents() {
        let agent = make_roster_agent("Scanner", 0);
        let roster = vec![agent.clone()];

        let result = build_team_roster_string(&roster);
        assert!(result.contains("Scanner"));
        assert!(result.contains("file_read"));
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
}
