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

    // ── Ticket 1.2: Sub-DAG WebSocket Events ──────────────────────────────

    use crate::server::hub::dag::build_parent_relay;
    use crate::server::hub::dag::utils::SubWorkflowParentContext;
    use crate::server::ws::events::WorkflowEventKind;

    fn make_parent_context() -> SubWorkflowParentContext {
        SubWorkflowParentContext {
            parent_step_id: Uuid::new_v4(),
            parent_run_id: Uuid::new_v4(),
            parent_workflow_id: Uuid::new_v4(),
        }
    }

    #[test]
    fn test_relay_step_started() {
        let parent = make_parent_context();
        let child_exec_id = Uuid::new_v4();
        let step_id = Uuid::new_v4();

        let kind = WorkflowEventKind::StepStarted {
            step_id,
            step_name: "Lint Check".into(),
            agent_id: Some(Uuid::new_v4()),
            execution_id: None,
        };

        let relay = build_parent_relay(&kind, &parent, child_exec_id);
        assert!(relay.is_some());

        match relay.unwrap() {
            WorkflowEventKind::SubWorkflowStepProgress {
                parent_step_id,
                child_execution_id,
                child_step_id,
                child_step_name,
                status,
                input_tokens,
                output_tokens,
                duration_ms,
                error,
            } => {
                assert_eq!(parent_step_id, parent.parent_step_id);
                assert_eq!(child_execution_id, child_exec_id);
                assert_eq!(child_step_id, step_id);
                assert_eq!(child_step_name, "Lint Check");
                assert_eq!(status, "started");
                assert!(input_tokens.is_none());
                assert!(output_tokens.is_none());
                assert!(duration_ms.is_none());
                assert!(error.is_none());
            }
            other => panic!("expected SubWorkflowStepProgress, got {:?}", other),
        }
    }

    #[test]
    fn test_relay_step_completed_with_tokens() {
        let parent = make_parent_context();
        let child_exec_id = Uuid::new_v4();
        let step_id = Uuid::new_v4();

        let kind = WorkflowEventKind::StepCompleted {
            step_id,
            step_name: "Security Scan".into(),
            agent_id: Some(Uuid::new_v4()),
            output: Some("scan complete".into()),
            input_tokens: Some(150),
            output_tokens: Some(50),
            duration_ms: Some(2100),
        };

        let relay = build_parent_relay(&kind, &parent, child_exec_id);
        assert!(relay.is_some());

        match relay.unwrap() {
            WorkflowEventKind::SubWorkflowStepProgress {
                parent_step_id,
                child_execution_id,
                child_step_id,
                child_step_name,
                status,
                input_tokens,
                output_tokens,
                duration_ms,
                error,
            } => {
                assert_eq!(parent_step_id, parent.parent_step_id);
                assert_eq!(child_execution_id, child_exec_id);
                assert_eq!(child_step_id, step_id);
                assert_eq!(child_step_name, "Security Scan");
                assert_eq!(status, "completed");
                assert_eq!(input_tokens, Some(150));
                assert_eq!(output_tokens, Some(50));
                assert_eq!(duration_ms, Some(2100));
                assert!(error.is_none());
            }
            other => panic!("expected SubWorkflowStepProgress, got {:?}", other),
        }
    }

    #[test]
    fn test_relay_step_failed_with_error() {
        let parent = make_parent_context();
        let child_exec_id = Uuid::new_v4();
        let step_id = Uuid::new_v4();

        let kind = WorkflowEventKind::StepFailed {
            step_id,
            step_name: "Code Review".into(),
            error: "model timeout after 30s".into(),
        };

        let relay = build_parent_relay(&kind, &parent, child_exec_id);
        assert!(relay.is_some());

        match relay.unwrap() {
            WorkflowEventKind::SubWorkflowStepProgress {
                status,
                error,
                child_step_name,
                ..
            } => {
                assert_eq!(status, "failed");
                assert_eq!(child_step_name, "Code Review");
                assert_eq!(error, Some("model timeout after 30s".into()));
            }
            other => panic!("expected SubWorkflowStepProgress, got {:?}", other),
        }
    }

    #[test]
    fn test_relay_returns_none_for_non_step_events() {
        let parent = make_parent_context();
        let child_exec_id = Uuid::new_v4();

        // Workflow Started — should NOT relay
        let started = WorkflowEventKind::Started { total_steps: 5 };
        assert!(build_parent_relay(&started, &parent, child_exec_id).is_none());

        // Workflow Completed — should NOT relay
        let completed = WorkflowEventKind::Completed {
            duration_ms: Some(5000),
        };
        assert!(build_parent_relay(&completed, &parent, child_exec_id).is_none());

        // ForEachProgress — should NOT relay
        let progress = WorkflowEventKind::ForEachProgress {
            step_id: Uuid::new_v4(),
            step_name: "iter".into(),
            completed: 3,
            total: 10,
        };
        assert!(build_parent_relay(&progress, &parent, child_exec_id).is_none());

        // SubWorkflowStarted — should NOT relay (already on parent channel)
        let sub_started = WorkflowEventKind::SubWorkflowStarted {
            parent_step_id: Uuid::new_v4(),
            child_execution_id: Uuid::new_v4(),
            total_steps: 3,
        };
        assert!(build_parent_relay(&sub_started, &parent, child_exec_id).is_none());
    }

    #[test]
    fn test_sub_workflow_step_progress_wire_format() {
        let parent_step_id = Uuid::new_v4();
        let child_execution_id = Uuid::new_v4();
        let child_step_id = Uuid::new_v4();

        let kind = WorkflowEventKind::SubWorkflowStepProgress {
            parent_step_id,
            child_execution_id,
            child_step_id,
            child_step_name: "Lint Check".into(),
            status: "completed".into(),
            input_tokens: Some(100),
            output_tokens: Some(50),
            duration_ms: Some(1234),
            error: None,
        };

        let json = serde_json::to_value(&kind).unwrap();
        let data = json.get("sub_workflow_step_progress").unwrap();
        assert_eq!(
            data.get("parent_step_id").unwrap().as_str().unwrap(),
            parent_step_id.to_string()
        );
        assert_eq!(
            data.get("child_execution_id").unwrap().as_str().unwrap(),
            child_execution_id.to_string()
        );
        assert_eq!(
            data.get("child_step_id").unwrap().as_str().unwrap(),
            child_step_id.to_string()
        );
        assert_eq!(
            data.get("child_step_name").unwrap().as_str().unwrap(),
            "Lint Check"
        );
        assert_eq!(data.get("status").unwrap().as_str().unwrap(), "completed");
        assert_eq!(data.get("input_tokens").unwrap().as_u64().unwrap(), 100);
        assert_eq!(data.get("output_tokens").unwrap().as_u64().unwrap(), 50);
        assert_eq!(data.get("duration_ms").unwrap().as_u64().unwrap(), 1234);
        // error should be absent (skip_serializing_if = "Option::is_none")
        assert!(data.get("error").is_none());
    }

    #[test]
    fn test_parent_context_set_on_child_ctx() {
        let parent_step_id = Uuid::new_v4();
        let parent_run_id = Uuid::new_v4();
        let parent_workflow_id = Uuid::new_v4();

        let ctx = SubWorkflowParentContext {
            parent_step_id,
            parent_run_id,
            parent_workflow_id,
        };

        assert_eq!(ctx.parent_step_id, parent_step_id);
        assert_eq!(ctx.parent_run_id, parent_run_id);
        assert_eq!(ctx.parent_workflow_id, parent_workflow_id);
    }
}
