#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::types::StepExecutionEnvelope;
    use serde_json::json;
    use uuid::Uuid;

    #[test]
    fn to_snake_case_basic() {
        assert_eq!(to_snake_case("Hello World"), "hello_world");
        assert_eq!(to_snake_case("  My Step  "), "my_step");
        assert_eq!(to_snake_case("CamelCase"), "camelcase");
        assert_eq!(to_snake_case("kebab-case"), "kebab_case");
        assert_eq!(to_snake_case("dot.separated"), "dot_separated");
    }

    #[test]
    fn to_snake_case_collapses_underscores() {
        assert_eq!(to_snake_case("foo___bar"), "foo_bar");
        assert_eq!(to_snake_case("  a  b  "), "a_b");
    }

    #[test]
    fn dag_state_new_is_empty() {
        let state = DagExecutionState::new();
        assert!(state.var_outputs.is_empty());
        assert!(state.completed.is_empty());
        assert!(state.completed_envelopes.is_empty());
        assert_eq!(state.total_input_tokens, 0);
        assert_eq!(state.total_output_tokens, 0);
        assert!((state.total_cost_usd - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn dag_state_accumulate_tokens() {
        let mut state = DagExecutionState::new();
        state.accumulate_tokens(100, 200, 0.5);
        state.accumulate_tokens(50, 75, 0.25);
        assert_eq!(state.total_input_tokens, 150);
        assert_eq!(state.total_output_tokens, 275);
        assert!((state.total_cost_usd - 0.75).abs() < f32::EPSILON);
    }

    #[test]
    fn dag_state_record_step_output() {
        let mut state = DagExecutionState::new();
        let step_id = Uuid::new_v4();

        let output = StepOutput {
            variable_name: "my_var".to_string(),
            structured_output: Some(json!({"key": "value"})),
            raw_output: "raw".to_string(),
        };

        let envelope = StepExecutionEnvelope {
            data: Some(json!({"key": "value"})),
            metadata: crate::types::ExecutionMetadata::new(step_id),
            ..Default::default()
        };

        state.record_step_output(step_id, output, envelope);

        assert!(state.completed.contains_key(&step_id));
        assert!(state.completed_envelopes.contains_key(&step_id));
        assert_eq!(
            state.var_outputs.get("my_var"),
            Some(&json!({"key": "value"}))
        );
    }

    #[test]
    fn dag_state_record_output_empty_variable_name_skips_var_outputs() {
        let mut state = DagExecutionState::new();
        let step_id = Uuid::new_v4();

        let output = StepOutput {
            variable_name: String::new(),
            structured_output: Some(json!(42)),
            raw_output: "42".to_string(),
        };

        let envelope = StepExecutionEnvelope {
            data: Some(json!(42)),
            metadata: crate::types::ExecutionMetadata::new(step_id),
            ..Default::default()
        };

        state.record_step_output(step_id, output, envelope);

        assert!(state.completed.contains_key(&step_id));
        assert!(state.var_outputs.is_empty());
    }

    #[test]
    fn dag_state_with_completed_preserves_state() {
        let step_id = Uuid::new_v4();
        let mut completed = std::collections::HashMap::new();
        completed.insert(
            step_id,
            StepOutput {
                variable_name: "test_var".to_string(),
                structured_output: Some(json!({"done": true})),
                raw_output: "done".to_string(),
            },
        );

        let mut var_outputs = std::collections::HashMap::new();
        var_outputs.insert("test_var".to_string(), json!({"done": true}));

        let state = DagExecutionState::with_completed(completed, var_outputs);
        assert!(state.completed.contains_key(&step_id));
        assert_eq!(
            state.var_outputs.get("test_var"),
            Some(&json!({"done": true}))
        );
        assert!(state.completed_envelopes.is_empty());
        assert_eq!(state.total_input_tokens, 0);
    }

    #[test]
    fn execution_metadata_new_defaults() {
        let id = Uuid::new_v4();
        let meta = crate::types::ExecutionMetadata::new(id);
        assert_eq!(meta.execution_id, id);
        assert_eq!(meta.execution_time_ms, 0);
        assert!(meta.tokens_in.is_none());
        assert!(meta.tokens_out.is_none());
        assert!(meta.cost_usd.is_none());
        assert!(meta.model.is_none());
        assert!(meta.agent_id.is_none());
        assert!(meta.iteration_index.is_none());
        assert!(meta.routing_label.is_none());
        assert!(meta.room_session_id.is_none());
        assert!(meta.room_id.is_none());
        assert!(meta.total_rounds.is_none());
    }

    #[test]
    fn wrap_in_agentless_envelope_success_when_data_present() {
        let step_id = Uuid::new_v4();
        let data = Some(json!({"result": "ok"}));
        let envelope = wrap_in_agentless_envelope(step_id, data.clone(), 150, 100, 200, 0.5);

        assert_eq!(envelope.status, crate::types::ExecutionStatus::Success);
        assert_eq!(envelope.data, data);
        assert_eq!(envelope.metadata.execution_id, step_id);
        assert_eq!(envelope.metadata.execution_time_ms, 150);
        assert_eq!(envelope.metadata.tokens_in, Some(100));
        assert_eq!(envelope.metadata.tokens_out, Some(200));
        assert!(envelope.metadata.agent_id.is_none());
        assert!(envelope.metadata.model.is_none());
        assert!(envelope.error.is_none());
    }

    #[test]
    fn wrap_in_agentless_envelope_error_when_no_data() {
        let step_id = Uuid::new_v4();
        let envelope = wrap_in_agentless_envelope(step_id, None, 0, 0, 0, 0.0);

        assert_eq!(envelope.status, crate::types::ExecutionStatus::Error);
        assert!(envelope.data.is_none());
    }

    #[test]
    fn step_display_name_uses_variable_name() {
        let mut step = crate::db::WorkflowStepRow {
            id: Uuid::new_v4(),
            workflow_id: Uuid::new_v4(),
            output_variable_name: Some("my_output".to_string()),
            ..Default::default()
        };
        assert_eq!(step_display_name(&step), "my_output");

        step.output_variable_name = None;
        assert_eq!(step_display_name(&step), step.id.to_string());
    }

    // =========================================================================
    // Snapshot / Merge Tests (for parallel DAG execution)
    // =========================================================================

    #[test]
    fn snapshot_for_parallel_clones_completed_state() {
        let mut state = DagExecutionState::new();
        let step_id = Uuid::new_v4();

        let output = StepOutput {
            variable_name: "var_a".to_string(),
            structured_output: Some(json!({"data": 1})),
            raw_output: "data".to_string(),
        };
        let envelope = StepExecutionEnvelope {
            data: Some(json!({"data": 1})),
            metadata: crate::types::ExecutionMetadata::new(step_id),
            ..Default::default()
        };
        state.record_step_output(step_id, output, envelope);
        state.accumulate_tokens(100, 200, 0.5);

        let snapshot = state.snapshot_for_parallel();

        // Completed state is cloned
        assert!(snapshot.completed.contains_key(&step_id));
        assert!(snapshot.completed_envelopes.contains_key(&step_id));
        assert_eq!(snapshot.var_outputs.get("var_a"), Some(&json!({"data": 1})));

        // Accumulators are zeroed
        assert_eq!(snapshot.total_input_tokens, 0);
        assert_eq!(snapshot.total_output_tokens, 0);
        assert!((snapshot.total_cost_usd - 0.0).abs() < f32::EPSILON);
        assert!(snapshot.failed.is_empty());
    }

    #[test]
    fn merge_parallel_result_combines_new_entries() {
        let mut state = DagExecutionState::new();
        let step_a = Uuid::new_v4();
        let step_b = Uuid::new_v4();

        // Pre-existing step in main state
        state.record_step_output(
            step_a,
            StepOutput {
                variable_name: "var_a".to_string(),
                structured_output: Some(json!("a")),
                raw_output: "a".to_string(),
            },
            StepExecutionEnvelope {
                data: Some(json!("a")),
                metadata: crate::types::ExecutionMetadata::new(step_a),
                ..Default::default()
            },
        );

        // Parallel task adds step_b
        let mut task_state = state.snapshot_for_parallel();
        task_state.record_step_output(
            step_b,
            StepOutput {
                variable_name: "var_b".to_string(),
                structured_output: Some(json!("b")),
                raw_output: "b".to_string(),
            },
            StepExecutionEnvelope {
                data: Some(json!("b")),
                metadata: crate::types::ExecutionMetadata::new(step_b),
                ..Default::default()
            },
        );
        task_state.accumulate_tokens(50, 75, 0.25);

        state.merge_parallel_result(task_state);

        // Both steps present
        assert!(state.completed.contains_key(&step_a));
        assert!(state.completed.contains_key(&step_b));
        assert!(state.completed_envelopes.contains_key(&step_b));
        assert_eq!(state.var_outputs.get("var_b"), Some(&json!("b")));

        // Tokens accumulated
        assert_eq!(state.total_input_tokens, 50);
        assert_eq!(state.total_output_tokens, 75);
        assert!((state.total_cost_usd - 0.25).abs() < f32::EPSILON);
    }

    #[test]
    fn merge_parallel_result_skips_preexisting_entries() {
        let mut state = DagExecutionState::new();
        let step_id = Uuid::new_v4();

        state.record_step_output(
            step_id,
            StepOutput {
                variable_name: "original".to_string(),
                structured_output: Some(json!("original")),
                raw_output: "original".to_string(),
            },
            StepExecutionEnvelope {
                data: Some(json!("original")),
                metadata: crate::types::ExecutionMetadata::new(step_id),
                ..Default::default()
            },
        );

        // Parallel task also has step_id from its snapshot — should not overwrite
        let task_state = state.snapshot_for_parallel();
        state.merge_parallel_result(task_state);

        // Original value preserved
        assert_eq!(state.var_outputs.get("original"), Some(&json!("original")));
        assert_eq!(state.completed.len(), 1);
    }

    #[test]
    fn merge_parallel_result_accumulates_tokens_from_multiple() {
        let mut state = DagExecutionState::new();
        state.accumulate_tokens(10, 20, 0.1);

        let mut task_a = state.snapshot_for_parallel();
        task_a.accumulate_tokens(100, 200, 1.0);

        let mut task_b = state.snapshot_for_parallel();
        task_b.accumulate_tokens(50, 75, 0.5);

        state.merge_parallel_result(task_a);
        state.merge_parallel_result(task_b);

        assert_eq!(state.total_input_tokens, 160);
        assert_eq!(state.total_output_tokens, 295);
        assert!((state.total_cost_usd - 1.6).abs() < f32::EPSILON);
    }
}
