#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use crate::server::hub::dag::pipeline::runner::AgentExecutionInput;
    use crate::server::hub::dag::pipeline::DesignedAgentPrompt;

    fn make_prompt(name: &str, receives_from: Vec<&str>) -> DesignedAgentPrompt {
        DesignedAgentPrompt {
            agent_roster_entry_id: Uuid::nil(),
            agent_name: name.to_string(),
            tools: vec![],
            system_prompt: format!("{name} system prompt"),
            assignment: format!("{name} assignment"),
            expected_output: Some(format!("{name} expected output")),
            execution_order: 0,
            receives_from: receives_from.into_iter().map(String::from).collect(),
            read_only: false,
        }
    }

    #[test]
    fn zero_phase_tokens_pattern() {
        let input = AgentExecutionInput {
            designed_prompts: vec![make_prompt("scanner", vec![])],
            failure_mode: "fail_fast".to_string(),
            upstream_step_output: String::new(),
            original_prompt: "test".to_string(),
            designer_run_id: None,
            phase_tokens_in: 0,
            phase_tokens_out: 0,
            phase_cost: 0.0,
        };

        assert!(input.designer_run_id.is_none());
        assert_eq!(input.phase_tokens_in, 0);
        assert_eq!(input.phase_tokens_out, 0);
        assert_eq!(input.phase_cost, 0.0);
        assert_eq!(input.failure_mode, "fail_fast");
    }

    #[test]
    fn with_phase_tokens_pattern() {
        let run_id = Uuid::new_v4();
        let input = AgentExecutionInput {
            designed_prompts: vec![
                make_prompt("scanner", vec![]),
                make_prompt("analyzer", vec!["scanner"]),
            ],
            failure_mode: "skip_failed".to_string(),
            upstream_step_output: "upstream data".to_string(),
            original_prompt: "Scan codebase".to_string(),
            designer_run_id: Some(run_id),
            phase_tokens_in: 1500,
            phase_tokens_out: 800,
            phase_cost: 0.05,
        };

        assert_eq!(input.designer_run_id, Some(run_id));
        assert_eq!(input.phase_tokens_in, 1500);
        assert_eq!(input.phase_tokens_out, 800);
        assert_eq!(input.designed_prompts.len(), 2);
        assert_eq!(input.designed_prompts[1].receives_from, vec!["scanner"]);
    }
}
