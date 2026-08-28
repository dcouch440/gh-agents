#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use uuid::Uuid;

    use crate::db::fixtures::fixtures::*;
    use crate::execution::diagnostics::types::{ChangeType, FileChange};
    use crate::server::hub::dag::pipeline::{
        build_filtered_outputs_block, build_upstream_outputs_block, compose_workforce_output,
        compute_execution_levels, filter_outputs_for_agent, passdown_entries, DesignedAgentPrompt,
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
            read_only: false,
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
        let outputs = [
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
            has_container: true,
        }
        .build();

        assert!(prompt.contains("<previous_step>\nPrior output text\n</previous_step>"));
        assert!(prompt.contains("<assignment>\nDo the thing\n</assignment>"));
        assert!(prompt.contains("<deliverable>\nDescribe what you did\n</deliverable>"));
        // The builder appends no directive about where output goes — that is
        // the designer's `expected_output`, which is the <deliverable> body.
        assert!(prompt.trim_end().ends_with("</deliverable>"));
    }

    #[test]
    fn task_prompt_builder_omits_empty_previous_step() {
        use super::super::agent_executor::TaskPromptBuilder;

        let prompt = TaskPromptBuilder {
            previous_step: String::new(),
            assignment: "Do the thing".to_string(),
            expected_output: None,
            has_container: true,
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
            has_container: true,
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
            has_container: true,
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
            has_container: true,
        }
        .build();

        let prev_pos = prompt.find("<previous_step>").unwrap();
        let assign_pos = prompt.find("<assignment>").unwrap();
        let expect_pos = prompt.find("<deliverable>").unwrap();
        assert!(prev_pos < assign_pos);
        assert!(assign_pos < expect_pos);
    }

    /// The container is the one fact the designer cannot know, so it is the one
    /// fact the builder still states. Without a container there are no workspace
    /// tools at all, which makes a `<deliverable>` describing a saved file
    /// unachievable — the agent has to be told before it tries.
    #[test]
    fn task_prompt_builder_states_missing_workspace() {
        use super::super::agent_executor::TaskPromptBuilder;

        let prompt = TaskPromptBuilder {
            previous_step: String::new(),
            assignment: "Do the thing".to_string(),
            expected_output: Some("A summary of the findings".to_string()),
            has_container: false,
        }
        .build();

        assert!(prompt.contains("<deliverable>\nA summary of the findings\n</deliverable>"));
        assert!(prompt.contains("has to be in your response"));
    }

    /// With a container the builder appends nothing after `</deliverable>`.
    /// Where the output goes is `expected_output`'s job — the designer knows
    /// whether it is one file, several, or a report from a read_only agent, and
    /// a hardcoded "save this to a file" contradicted it every time.
    #[test]
    fn task_prompt_builder_appends_no_directive_with_container() {
        use super::super::agent_executor::TaskPromptBuilder;

        let prompt = TaskPromptBuilder {
            previous_step: String::new(),
            assignment: "Do the thing".to_string(),
            expected_output: Some("A summary of the findings".to_string()),
            has_container: true,
        }
        .build();

        assert!(prompt.trim_end().ends_with("</deliverable>"));
        assert!(!prompt.contains("write_file"));
        assert!(!prompt.contains("receipt"));
    }

    // ── Failure handling ──────────────────────────────────────────────────────

    /// Run dd27d008: the QA agent had finished its verification pass 67/67 green
    /// and was doing unprompted refactoring when round 60 hit. `fail_fast`
    /// turned that into an Abort that killed a five-step run and discarded seven
    /// successful agents' deliverable.
    #[test]
    fn round_exhaustion_skips_even_under_fail_fast() {
        use crate::server::hub::dag::pipeline::agent_executor::handle_agent_failure;
        use crate::server::hub::dag::pipeline::types::AgentFailureAction;
        use crate::server::hub::error::HubError;

        let action = handle_agent_failure(
            HubError::MaxRoundsExhausted { max: 60 },
            "Frontend QA engineer",
            "fail_fast",
        );

        match action {
            AgentFailureAction::Skip { name, error_output } => {
                assert_eq!(name, "Frontend QA engineer");
                assert!(error_output.contains("60"));
                assert!(
                    error_output.contains("on disk"),
                    "downstream agents must be told the work is partial, not just that it failed"
                );
            }
            AgentFailureAction::Abort(_) => panic!("exhaustion must not abort the step"),
        }
    }

    /// A targeted exception, not a blanket downgrade — every other failure under
    /// `fail_fast` still aborts.
    #[test]
    fn other_failures_still_abort_under_fail_fast() {
        use crate::server::hub::dag::pipeline::agent_executor::handle_agent_failure;
        use crate::server::hub::dag::pipeline::types::AgentFailureAction;
        use crate::server::hub::error::HubError;

        let action = handle_agent_failure(
            HubError::ToolFailed {
                tool_name: "write_file".to_string(),
                reason: "disk full".to_string(),
            },
            "Builder",
            "fail_fast",
        );

        assert!(matches!(action, AgentFailureAction::Abort(_)));
    }

    /// `skip_failed` behaviour is unchanged for ordinary failures.
    #[test]
    fn skip_failed_mode_still_skips_ordinary_failures() {
        use crate::server::hub::dag::pipeline::agent_executor::handle_agent_failure;
        use crate::server::hub::dag::pipeline::types::AgentFailureAction;
        use crate::server::hub::error::HubError;

        let action = handle_agent_failure(
            HubError::ToolFailed {
                tool_name: "run_command".to_string(),
                reason: "boom".to_string(),
            },
            "Builder",
            "skip_failed",
        );

        match action {
            AgentFailureAction::Skip { error_output, .. } => {
                assert!(error_output.contains("AGENT FAILED"));
            }
            AgentFailureAction::Abort(_) => panic!("skip_failed must not abort"),
        }
    }

    // ── Tool assembly ─────────────────────────────────────────────────────

    /// `let baseline = ["run_command"];` was the entire implicit tool set.
    /// Shell execution is `safety_level: unsafe, default_enabled: false` in
    /// capabilities.yaml and was the only tool injected; file_read (safe) and
    /// file_write (caution) required an opt-in the designer prompt discouraged.
    #[test]
    fn containerized_agents_get_the_file_tools_without_being_assigned_them() {
        use crate::server::hub::dag::pipeline::agent_executor::{
            inject_baseline_tools, CONTAINER_BASELINE_TOOLS,
        };

        let (mut tools, mut names) = (Vec::new(), Vec::new());
        inject_baseline_tools(&mut tools, &mut names, CONTAINER_BASELINE_TOOLS);

        for expected in [
            "read_file",
            "write_file",
            "edit_file",
            "list_files",
            "run_command",
        ] {
            assert!(
                names.iter().any(|n| n == expected),
                "{expected} missing from the baseline"
            );
        }
        assert_eq!(tools.len(), names.len());
    }

    /// A designer that explicitly resolved a baseline tool through a capability
    /// must not end up with it twice — duplicate tool names are a provider
    /// error, not a warning.
    #[test]
    fn baseline_injection_does_not_duplicate_an_already_resolved_tool() {
        use crate::server::hub::dag::pipeline::agent_executor::{
            inject_baseline_tools, CONTAINER_BASELINE_TOOLS,
        };

        let mut names = vec!["read_file".to_string()];
        let mut tools = vec![crate::tools::registry::get_tool_definition("read_file").unwrap()];
        inject_baseline_tools(&mut tools, &mut names, CONTAINER_BASELINE_TOOLS);

        assert_eq!(names.iter().filter(|n| *n == "read_file").count(), 1);
        assert_eq!(names.len(), 5);
    }

    /// The QA agent in run dd27d008 finished verification 67/67 green, then
    /// spent its remaining rounds `sed -i`-ing styles.css and rewriting
    /// script.js. A verifier that can write stops verifying — the read_only
    /// flag has to actually remove the tools, not just say so in a prompt.
    #[test]
    fn a_read_only_agent_gets_no_writing_tools() {
        use crate::server::hub::dag::pipeline::agent_executor::{
            inject_baseline_tools, restrict_to_read_only, READ_ONLY_BASELINE_TOOLS,
        };

        let (mut tools, mut names) = (Vec::new(), Vec::new());
        inject_baseline_tools(&mut tools, &mut names, READ_ONLY_BASELINE_TOOLS);

        for banned in [
            "write_file",
            "edit_file",
            "run_command",
            "git_add",
            "git_commit",
        ] {
            assert!(
                !names.iter().any(|n| n == banned),
                "{banned} reached a read_only agent"
            );
        }
        assert!(names.iter().any(|n| n == "read_file"));
        assert!(names.iter().any(|n| n == "list_files"));

        restrict_to_read_only(&mut tools, &mut names);
        assert_eq!(
            names.len(),
            2,
            "restriction must be idempotent over the read-only baseline"
        );
    }

    /// The designer can still hand a read_only agent a writing capability by
    /// mistake. Restriction runs before baseline injection so both routes are
    /// covered — and web tools it legitimately assigned must survive.
    #[test]
    fn read_only_restriction_strips_write_tools_but_keeps_web() {
        use crate::server::hub::dag::pipeline::agent_executor::restrict_to_read_only;

        let mut names: Vec<String> = ["brave_search", "read_webpage", "write_file", "run_command"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let mut tools: Vec<_> = names
            .iter()
            .filter_map(|n| crate::tools::registry::get_tool_definition(n))
            .collect();

        restrict_to_read_only(&mut tools, &mut names);

        assert_eq!(
            names,
            vec!["brave_search".to_string(), "read_webpage".to_string()]
        );
    }

    /// A read_only agent's output IS its deliverable, and saying so is now the
    /// designer's job — it sets the flag, so it is the layer that knows. The
    /// builder's part of the contract is to carry `expected_output` through
    /// untouched and add nothing that could contradict it.
    #[test]
    fn read_only_deliverable_reaches_the_agent_verbatim() {
        use super::super::agent_executor::TaskPromptBuilder;

        let designed = "A pass or fail for every requirement in the brief, with concrete \
                        evidence under each failure. This is your reply, not a file — you \
                        have no write access in this step.";

        let prompt = TaskPromptBuilder {
            previous_step: String::new(),
            assignment: "Verify the build against the spec".to_string(),
            expected_output: Some(designed.to_string()),
            has_container: true,
        }
        .build();

        assert!(prompt.contains(designed));
        assert!(prompt.trim_end().ends_with("</deliverable>"));
        assert!(!prompt.contains("write_file"));
    }

    // ── Failed-agent row termination ──────────────────────────────────────

    /// `agent_executions.5734fed9-…` sat at `status = 'running'`,
    /// `completed_at = NULL` under a `workflow_executions` row that had been
    /// `failed` for hours, because the workforce failure path wrote
    /// `protocol_executions` instead. The UI reads `agent_executions` via
    /// `get_running_step_ids_for_run`, so the node spun forever.
    #[tokio::test]
    #[ignore = "requires running Postgres"]
    async fn failed_agent_leaves_a_terminal_row_not_a_running_one() {
        use crate::db::pg_repo::PgRepo;
        use crate::db::test_utils::TestDb;
        use crate::db::traits::{AgentExecutionRepo, CreateAgentExecutionInput};
        use crate::server::hub::dag::pipeline::agent_executor::fail_agent_execution;
        use crate::types::ExecutionType;

        let db = TestDb::new().await;
        let repo = PgRepo::new(db.pool.clone());

        let row = repo
            .create_agent_execution(CreateAgentExecutionInput {
                execution_type: ExecutionType::PipelineAgent,
                agent_id: None,
                workflow_step_id: None,
                parent_agent_execution_id: None,
                system_prompt_rendered: "Frontend QA engineer".to_string(),
                input: "Verify the build against the spec".to_string(),
                room_session_id: None,
                speaker_order: None,
                workflow_execution_id: None,
            })
            .await
            .unwrap();
        assert_eq!(row.status, "running");

        fail_agent_execution(&repo, Some(row.id), "max tool rounds (60) exhausted").await;

        let after = repo.get_agent_execution(row.id).await.unwrap().unwrap();
        assert_eq!(after.status, "failed");
        assert!(
            after.completed_at.is_some(),
            "completed_at must be stamped so the row reads as terminal"
        );
        assert!(after.output.unwrap().contains("max tool rounds"));

        db.cleanup().await;
    }

    /// Row creation can itself fail, leaving `ae_id` as None. That must be a
    /// no-op rather than a panic on an already-failing path.
    #[tokio::test]
    #[ignore = "requires running Postgres"]
    async fn fail_agent_execution_tolerates_a_missing_row_id() {
        use crate::db::pg_repo::PgRepo;
        use crate::db::test_utils::TestDb;
        use crate::server::hub::dag::pipeline::agent_executor::fail_agent_execution;

        let db = TestDb::new().await;
        let repo = PgRepo::new(db.pool.clone());

        fail_agent_execution(&repo, None, "boom").await;

        db.cleanup().await;
    }

    // ── passdown manifest ───────────────────────────────────────────────
    //
    // The `files:` line is the only objective half of the handoff. Before
    // directories were rolled up it was sorted by size and capped at ten, so a
    // deliverable that is a tree arrived as whichever ten of its files were
    // biggest — the shape the agent chose was the one thing it could not say.

    fn change(path: &str, size: u64) -> FileChange {
        FileChange {
            path: std::path::PathBuf::from(path),
            change_type: ChangeType::Created,
            size,
        }
    }

    #[test]
    fn passdown_names_root_files_individually() {
        let (entries, dropped) =
            passdown_entries(&[change("pricing.md", 4_000), change("notes.md", 900)]);

        assert_eq!(dropped, 0);
        assert_eq!(
            entries,
            vec![
                "pricing.md (created, 3KB)".to_string(),
                "notes.md (created, 900B)".to_string(),
            ]
        );
    }

    #[test]
    fn passdown_rolls_a_directory_into_one_entry() {
        let files: Vec<FileChange> = (0..12)
            .map(|i| change(&format!("tally/src/mod_{i}.py"), 1_000))
            .collect();

        let (entries, dropped) = passdown_entries(&files);

        // Twelve files, one deliverable, one line — and nothing dropped.
        assert_eq!(dropped, 0);
        assert_eq!(entries.len(), 1);
        assert!(
            entries[0].starts_with("tally/ (12 files,"),
            "{:?}",
            entries[0]
        );
    }

    #[test]
    fn passdown_keeps_a_small_deliverable_beside_a_large_tree() {
        // The exact case the old size-sorted cap dropped: one small file that
        // matters, written next to a directory of larger ones.
        let mut files: Vec<FileChange> = (0..30)
            .map(|i| change(&format!("build/asset_{i}.bin"), 500_000))
            .collect();
        files.push(change("README.md", 800));

        let (entries, dropped) = passdown_entries(&files);

        assert_eq!(dropped, 0);
        assert_eq!(entries.len(), 2);
        assert!(
            entries[0].starts_with("build/ (30 files,"),
            "{:?}",
            entries[0]
        );
        assert_eq!(entries[1], "README.md (created, 800B)");
    }

    #[test]
    fn passdown_names_the_file_when_a_directory_holds_one() {
        // `docs/ (1 file, …)` would be strictly less information than the path.
        let (entries, _) = passdown_entries(&[change("docs/design.md", 2_048)]);
        assert_eq!(entries, vec!["docs/design.md (created, 2KB)".to_string()]);
    }

    #[test]
    fn passdown_caps_after_grouping_and_reports_the_remainder() {
        let files: Vec<FileChange> = (0..14)
            .map(|i| change(&format!("dir_{i:02}/a.txt"), (14 - i) as u64 * 1_000))
            .collect();

        let (entries, dropped) = passdown_entries(&files);

        assert_eq!(entries.len(), 10);
        assert_eq!(dropped, 4);
    }
}
