#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};
    use tokio_util::sync::CancellationToken;
    use uuid::Uuid;

    use crate::db::{AgentExecutionRow, WorkflowStepRow};
    use crate::server::services::workflow_state::{
        is_generating, merge_dispatches, resolve_baseline_status, DispatchSource,
    };
    use crate::server::state::task_registry::{TaskEntry, TaskStatus};

    // ── Helpers ────────────────────────────────────────────────────────

    fn base_step() -> WorkflowStepRow {
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
            name: None,
            system_prompt_suffix: None,
            visible: true,
            description: String::new(),
            board_context_cache: String::new(),
            board_context_updated_at: None,
            goal_summary: String::new(),
            goal_summary_updated_at: None,
            child_workflow_id: None,
            ref_id: Some("test".to_string()),
            pinned: false,
            run_results_summary: String::new(),
            designer_handoff: String::new(),
        }
    }

    fn make_task(step_id: Uuid, status: TaskStatus, age_secs: i64) -> TaskEntry {
        TaskEntry {
            execution_id: Uuid::new_v4(),
            step_id,
            workflow_id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            status,
            instruction: "do the thing".to_string(),
            cancel_token: CancellationToken::new(),
            created_at: Utc::now() - Duration::seconds(age_secs),
            result: None,
            trace: Vec::new(),
        }
    }

    fn make_dispatch(step_id: Uuid, status: &str) -> AgentExecutionRow {
        AgentExecutionRow {
            workflow_step_id: Some(step_id),
            execution_type: "dispatch".to_string(),
            status: status.to_string(),
            ..AgentExecutionRow::default()
        }
    }

    // ── resolve_baseline_status ────────────────────────────────────────

    #[test]
    fn baseline_idle_by_default() {
        assert_eq!(resolve_baseline_status(&base_step(), None), "idle");
    }

    #[test]
    fn baseline_described_when_description_present() {
        let mut step = base_step();
        step.description = "Crawl the docs".to_string();
        assert_eq!(resolve_baseline_status(&step, None), "described");
    }

    #[test]
    fn baseline_configured_when_child_workflow_present() {
        let mut step = base_step();
        step.description = "Crawl the docs".to_string();
        step.child_workflow_id = Some(Uuid::new_v4());
        assert_eq!(resolve_baseline_status(&step, None), "configured");
    }

    #[test]
    fn baseline_completed_when_pinned() {
        let mut step = base_step();
        step.child_workflow_id = Some(Uuid::new_v4());
        step.pinned = true;
        assert_eq!(resolve_baseline_status(&step, None), "completed");
    }

    #[test]
    fn baseline_completed_when_run_results_summary_present() {
        let mut step = base_step();
        step.run_results_summary = "Found 12 issues".to_string();
        assert_eq!(resolve_baseline_status(&step, None), "completed");
    }

    #[test]
    fn baseline_error_outranks_completed() {
        let mut step = base_step();
        step.pinned = true;
        let failed = make_dispatch(step.id, "failed");
        assert_eq!(resolve_baseline_status(&step, Some(&failed)), "error");
    }

    #[test]
    fn baseline_ignores_non_failed_dispatch() {
        let step = base_step();
        let ok = make_dispatch(step.id, "completed");
        assert_eq!(resolve_baseline_status(&step, Some(&ok)), "idle");
    }

    // ── merge_dispatches ───────────────────────────────────────────────

    #[test]
    fn merge_empty_inputs_yields_nothing() {
        assert!(merge_dispatches(&[], &[]).is_empty());
    }

    #[test]
    fn merge_registry_entry_wins_over_persisted_for_same_step() {
        let step_id = Uuid::new_v4();
        let task = make_task(step_id, TaskStatus::Running, 0);
        let persisted = make_dispatch(step_id, "completed");

        let merged = merge_dispatches(&[task.clone()], &[persisted]);

        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].execution_id, task.execution_id);
        assert_eq!(merged[0].source, DispatchSource::Registry);
        assert_eq!(merged[0].status, "running");
    }

    #[test]
    fn merge_takes_first_registry_entry_per_step() {
        // Registry arrives newest-first; the first entry seen must win. This is
        // the regression guard for the frontend having read tasks[len - 1].
        let step_id = Uuid::new_v4();
        let newest = make_task(step_id, TaskStatus::Running, 0);
        let older = make_task(step_id, TaskStatus::Completed, 600);

        let merged = merge_dispatches(&[newest.clone(), older], &[]);

        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].execution_id, newest.execution_id);
    }

    #[test]
    fn merge_falls_back_to_persisted_when_registry_empty() {
        // The server-restart path: TaskRegistry is in-memory, agent_executions is not.
        let step_id = Uuid::new_v4();
        let persisted = make_dispatch(step_id, "completed");

        let merged = merge_dispatches(&[], &[persisted.clone()]);

        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].execution_id, persisted.id);
        assert_eq!(merged[0].source, DispatchSource::Persisted);
    }

    #[test]
    fn merge_keeps_distinct_steps_from_both_sources() {
        let registry_step = Uuid::new_v4();
        let persisted_step = Uuid::new_v4();

        let merged = merge_dispatches(
            &[make_task(registry_step, TaskStatus::Running, 0)],
            &[make_dispatch(persisted_step, "completed")],
        );

        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].step_id, registry_step);
        assert_eq!(merged[1].step_id, persisted_step);
    }

    #[test]
    fn merge_skips_persisted_rows_without_a_step() {
        let orphan = AgentExecutionRow {
            workflow_step_id: None,
            execution_type: "dispatch".to_string(),
            status: "completed".to_string(),
            ..AgentExecutionRow::default()
        };
        assert!(merge_dispatches(&[], &[orphan]).is_empty());
    }

    #[test]
    fn merge_counts_persisted_trace_length() {
        let step_id = Uuid::new_v4();
        let mut persisted = make_dispatch(step_id, "completed");
        persisted.trace = Some(serde_json::json!([{ "type": "token" }, { "type": "token" }]));

        let merged = merge_dispatches(&[], &[persisted]);

        assert_eq!(merged[0].trace_len, 2);
    }

    // ── is_generating ──────────────────────────────────────────────────

    #[test]
    fn is_generating_true_only_with_a_running_dispatch() {
        let step_id = Uuid::new_v4();
        let running = merge_dispatches(&[make_task(step_id, TaskStatus::Running, 0)], &[]);
        let done = merge_dispatches(&[make_task(step_id, TaskStatus::Completed, 0)], &[]);

        assert!(is_generating(&running));
        assert!(!is_generating(&done));
        assert!(!is_generating(&[]));
    }
}
