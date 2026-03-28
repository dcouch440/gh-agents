#[cfg(test)]
mod tests {
    use chrono::Utc;
    use tokio_util::sync::CancellationToken;
    use uuid::Uuid;

    use crate::db::{AgentExecutionRow, WorkflowStepRow};
    use crate::server::services::workflow_agent::state::{
        format_pipeline_summary, resolve_node_status,
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

    fn make_task(step_id: Uuid, status: TaskStatus) -> TaskEntry {
        TaskEntry {
            execution_id: Uuid::new_v4(),
            step_id,
            workflow_id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            status,
            instruction: String::new(),
            cancel_token: CancellationToken::new(),
            created_at: Utc::now(),
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

    // ── resolve_node_status: static statuses ───────────────────────────

    #[test]
    fn status_idle() {
        let step = base_step();
        assert_eq!(resolve_node_status(&step, &[], None), "idle");
    }

    #[test]
    fn status_described() {
        let mut step = base_step();
        step.description = "Some description".to_string();
        assert_eq!(resolve_node_status(&step, &[], None), "described");
    }

    #[test]
    fn status_configured() {
        let mut step = base_step();
        step.child_workflow_id = Some(Uuid::new_v4());
        assert_eq!(resolve_node_status(&step, &[], None), "configured");
    }

    #[test]
    fn status_completed_pinned() {
        let mut step = base_step();
        step.pinned = true;
        assert_eq!(resolve_node_status(&step, &[], None), "completed");
    }

    #[test]
    fn status_completed_has_results() {
        let mut step = base_step();
        step.run_results_summary = "some results".to_string();
        assert_eq!(resolve_node_status(&step, &[], None), "completed");
    }

    // ── resolve_node_status: active statuses ───────────────────────────

    #[test]
    fn status_configuring_active_task() {
        let step = base_step();
        let task = make_task(step.id, TaskStatus::Running);
        assert_eq!(resolve_node_status(&step, &[task], None), "configuring");
    }

    #[test]
    fn status_error_failed_dispatch() {
        let step = base_step();
        let dispatch = make_dispatch(step.id, "failed");
        assert_eq!(
            resolve_node_status(&step, &[], Some(&dispatch)),
            "error"
        );
    }

    // ── resolve_node_status: priority ordering ─────────────────────────

    #[test]
    fn configuring_beats_configured() {
        let mut step = base_step();
        step.child_workflow_id = Some(Uuid::new_v4());
        let task = make_task(step.id, TaskStatus::Running);
        // Active task overrides the configured status
        assert_eq!(resolve_node_status(&step, &[task], None), "configuring");
    }

    #[test]
    fn configuring_beats_error() {
        let step = base_step();
        let task = make_task(step.id, TaskStatus::Running);
        let dispatch = make_dispatch(step.id, "failed");
        // Active re-dispatch overrides previous failure
        assert_eq!(
            resolve_node_status(&step, &[task], Some(&dispatch)),
            "configuring"
        );
    }

    #[test]
    fn error_beats_configured() {
        let mut step = base_step();
        step.child_workflow_id = Some(Uuid::new_v4());
        let dispatch = make_dispatch(step.id, "failed");
        assert_eq!(
            resolve_node_status(&step, &[], Some(&dispatch)),
            "error"
        );
    }

    #[test]
    fn completed_task_not_configuring() {
        let step = base_step();
        let task = make_task(step.id, TaskStatus::Completed);
        // Completed task should NOT show as configuring
        assert_eq!(resolve_node_status(&step, &[task], None), "idle");
    }

    #[test]
    fn completed_dispatch_not_error() {
        let mut step = base_step();
        step.child_workflow_id = Some(Uuid::new_v4());
        let dispatch = make_dispatch(step.id, "completed");
        // Completed dispatch → configured (from child_workflow_id), not error
        assert_eq!(
            resolve_node_status(&step, &[], Some(&dispatch)),
            "configured"
        );
    }

    #[test]
    fn cancelled_task_not_configuring() {
        let step = base_step();
        let task = make_task(step.id, TaskStatus::Cancelled);
        assert_eq!(resolve_node_status(&step, &[task], None), "idle");
    }

    // ── format_pipeline_summary ────────────────────────────────────────

    #[test]
    fn pipeline_single_agent() {
        let step = WorkflowStepRow {
            name: Some("Scanner".to_string()),
            ..base_step()
        };
        let summary = format_pipeline_summary(&[step], &[]);
        assert_eq!(summary, "Scanner");
    }

    #[test]
    fn pipeline_linear_chain() {
        let mut a = base_step();
        a.name = Some("Scanner".to_string());
        a.id = Uuid::from_u128(1);

        let mut b = base_step();
        b.name = Some("Analyzer".to_string());
        b.id = Uuid::from_u128(2);

        let edge = crate::db::WorkflowStepEdgeRow {
            id: Uuid::new_v4(),
            from_step_id: a.id,
            to_step_id: b.id,
            from_output_port: None,
            to_input_port: None,
            transform_jsonpath: None,
            condition_type: None,
            condition_value: None,
            edge_label: None,
            workflow_id: Uuid::new_v4(),
        };

        let summary = format_pipeline_summary(&[a, b], &[edge]);
        assert_eq!(summary, "Scanner \u{2192} Analyzer");
    }

    #[test]
    fn pipeline_parallel_then_sequential() {
        let mut a = base_step();
        a.name = Some("Scanner".to_string());
        a.id = Uuid::from_u128(1);

        let mut b = base_step();
        b.name = Some("Crawler".to_string());
        b.id = Uuid::from_u128(2);

        let mut c = base_step();
        c.name = Some("Analyzer".to_string());
        c.id = Uuid::from_u128(3);

        let wf = Uuid::new_v4();
        let edges = vec![
            crate::db::WorkflowStepEdgeRow {
                id: Uuid::new_v4(),
                from_step_id: a.id,
                to_step_id: c.id,
                from_output_port: None,
                to_input_port: None,
                transform_jsonpath: None,
                condition_type: None,
                condition_value: None,
                edge_label: None,
                workflow_id: wf,
            },
            crate::db::WorkflowStepEdgeRow {
                id: Uuid::new_v4(),
                from_step_id: b.id,
                to_step_id: c.id,
                from_output_port: None,
                to_input_port: None,
                transform_jsonpath: None,
                condition_type: None,
                condition_value: None,
                edge_label: None,
                workflow_id: wf,
            },
        ];

        let summary = format_pipeline_summary(&[a, b, c], &edges);
        assert_eq!(summary, "(Scanner, Crawler) \u{2192} Analyzer");
    }
}
