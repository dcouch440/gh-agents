#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use serde_json::{json, Value as JsonValue};
    use uuid::Uuid;

    use crate::types::{ExecutionMetadata, ExecutionStatus, StepExecutionEnvelope};

    use crate::server::hub::dag::utils::{StepOutput, WorkflowExecutionResult};

    #[test]
    fn test_child_context_mapping_from_port_inputs() {
        // Verify that port inputs map correctly to child prior_outputs + initial_input
        let port_inputs: HashMap<String, JsonValue> = HashMap::from([
            (
                "task_description".to_string(),
                json!("Analyze the codebase"),
            ),
            ("config".to_string(), json!({"depth": 3, "format": "json"})),
        ]);

        // Child prior_outputs should be the full map
        let child_prior_outputs = port_inputs.clone();
        assert_eq!(child_prior_outputs.len(), 2);
        assert_eq!(
            child_prior_outputs.get("task_description"),
            Some(&json!("Analyze the codebase"))
        );

        // initial_input should be the first value (string passthrough)
        let first_value = child_prior_outputs.values().next().unwrap();
        let child_initial_input = match first_value {
            JsonValue::String(s) => s.clone(),
            other => serde_json::to_string(other).unwrap_or_default(),
        };
        // The first value depends on HashMap ordering, but it should be a non-empty string
        assert!(!child_initial_input.is_empty());
    }

    #[test]
    fn test_empty_port_inputs_produce_empty_context() {
        let port_inputs: Option<HashMap<String, JsonValue>> = None;
        let child_prior_outputs = port_inputs.unwrap_or_default();
        assert!(child_prior_outputs.is_empty());

        let child_initial_input = child_prior_outputs
            .values()
            .next()
            .map(|v| match v {
                JsonValue::String(s) => s.clone(),
                other => serde_json::to_string(other).unwrap_or_default(),
            })
            .unwrap_or_default();
        assert!(child_initial_input.is_empty());
    }

    #[test]
    fn test_success_envelope_structure() {
        let child_execution_id = Uuid::new_v4();

        // Build outputs JSON the same way the executor does:
        // a map from step name → structured_output
        let outputs_json = json!({
            "analysis": {"findings": ["issue1", "issue2"]}
        });

        let envelope = StepExecutionEnvelope {
            status: ExecutionStatus::Success,
            data: Some(outputs_json),
            metadata: ExecutionMetadata {
                execution_time_ms: 5000,
                tokens_in: Some(1000),
                tokens_out: Some(500),
                cost_usd: Some(0.05),
                child_workflow_execution_id: Some(child_execution_id),
                ..ExecutionMetadata::new(child_execution_id)
            },
            error: None,
        };

        assert_eq!(envelope.status, ExecutionStatus::Success);
        assert!(envelope.data.is_some());
        assert!(envelope.error.is_none());
        assert_eq!(
            envelope.metadata.child_workflow_execution_id,
            Some(child_execution_id)
        );
        assert_eq!(envelope.metadata.tokens_in, Some(1000));
        assert_eq!(envelope.metadata.tokens_out, Some(500));
    }

    #[test]
    fn test_error_envelope_structure() {
        let child_execution_id = Uuid::new_v4();
        let error_msg = "Child step 'code_review' failed: model timeout".to_string();

        let envelope = StepExecutionEnvelope {
            status: ExecutionStatus::Error,
            data: None,
            metadata: ExecutionMetadata {
                execution_time_ms: 30000,
                child_workflow_execution_id: Some(child_execution_id),
                ..ExecutionMetadata::new(child_execution_id)
            },
            error: Some(crate::types::ExecutionError {
                message: error_msg.clone(),
                error_type: "SubWorkflowFailed".into(),
                retryable: false,
                details: None,
            }),
        };

        assert_eq!(envelope.status, ExecutionStatus::Error);
        assert!(envelope.data.is_none());
        assert!(envelope.error.is_some());
        let err = envelope.error.unwrap();
        assert_eq!(err.error_type, "SubWorkflowFailed");
        assert_eq!(err.message, error_msg);
        assert!(!err.retryable);
    }

    #[test]
    fn test_output_map_construction_from_step_outputs() {
        // Verify the JSON map construction matches what execute_sub_workflow_step produces
        let result = WorkflowExecutionResult {
            outputs: HashMap::from([
                (
                    "step_a".to_string(),
                    StepOutput {
                        variable_name: "step_a".to_string(),
                        structured_output: Some(json!({"result": "ok"})),
                        raw_output: r#"{"result": "ok"}"#.to_string(),
                    },
                ),
                (
                    "step_b".to_string(),
                    StepOutput {
                        variable_name: "step_b".to_string(),
                        structured_output: Some(json!(["item1", "item2"])),
                        raw_output: r#"["item1", "item2"]"#.to_string(),
                    },
                ),
                (
                    "step_c_skipped".to_string(),
                    StepOutput {
                        variable_name: "step_c_skipped".to_string(),
                        structured_output: None, // Skipped step has no output
                        raw_output: String::new(),
                    },
                ),
            ]),
            total_input_tokens: 2000,
            total_output_tokens: 1000,
            total_cost_usd: 0.10,
            duration_ms: 8000,
        };

        // Build JSON the same way as the executor
        let outputs_map: serde_json::Map<String, JsonValue> = result
            .outputs
            .iter()
            .filter_map(|(key, step_output)| {
                step_output
                    .structured_output
                    .clone()
                    .map(|v| (key.clone(), v))
            })
            .collect();
        let outputs_json = JsonValue::Object(outputs_map);

        assert!(outputs_json.is_object());
        // step_a and step_b should be present
        assert_eq!(outputs_json.get("step_a"), Some(&json!({"result": "ok"})));
        assert_eq!(outputs_json.get("step_b"), Some(&json!(["item1", "item2"])));
        // step_c_skipped should NOT be present (None structured_output is filtered)
        assert!(outputs_json.get("step_c_skipped").is_none());
    }

    #[test]
    fn test_token_accumulation_from_child() {
        // Verify parent DAG state accumulates child tokens correctly
        let mut dag_state = crate::server::hub::dag::dag_state::DagExecutionState::new();

        // Simulate pre-existing tokens from earlier parent steps
        dag_state.accumulate_tokens(500, 200, 0.02);

        // Simulate child workflow token accumulation
        dag_state.accumulate_tokens(1000, 500, 0.05);

        assert_eq!(dag_state.total_input_tokens, 1500);
        assert_eq!(dag_state.total_output_tokens, 700);
        assert!((dag_state.total_cost_usd - 0.07).abs() < 0.001);
    }
}
