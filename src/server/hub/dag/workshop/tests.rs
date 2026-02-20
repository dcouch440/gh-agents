#[cfg(test)]
mod tests {
    use crate::db::traits::MockContentVersionRepo;
    use crate::db::{EnvelopeSnapshotRow, WorkflowStepEdgeRow, WorkflowStepRow};
    use crate::server::hub::dag::dag_state::{DagExecutionState, PortMetadata};
    use crate::server::hub::dag::utils::StepOutput;
    use crate::types::{ExecutionMetadata, ExecutionStatus, StepExecutionEnvelope};
    use serde_json::json;
    use std::collections::HashMap;
    use uuid::Uuid;

    use crate::server::hub::dag::workshop::next_executable_steps;
    use crate::server::hub::dag::workshop::reconstruct_state;

    // ── Helpers ──────────────────────────────────────────────────────────────

    fn make_step(id: Uuid, mode: &str, var_name: Option<&str>, order: i32) -> WorkflowStepRow {
        WorkflowStepRow {
            id,
            workflow_id: Uuid::new_v4(),
            agent_id: Some(Uuid::new_v4()),
            execution_mode: mode.to_string(),
            output_variable_name: var_name.map(String::from),
            display_order: order,
            ..Default::default()
        }
    }

    fn make_envelope(data: Option<serde_json::Value>) -> StepExecutionEnvelope {
        StepExecutionEnvelope {
            status: if data.is_some() {
                ExecutionStatus::Success
            } else {
                ExecutionStatus::Error
            },
            data,
            metadata: ExecutionMetadata::new(Uuid::new_v4()),
            error: None,
        }
    }

    fn make_edge(from: Uuid, to: Uuid) -> WorkflowStepEdgeRow {
        WorkflowStepEdgeRow {
            id: Uuid::new_v4(),
            workflow_id: Uuid::new_v4(),
            from_step_id: from,
            to_step_id: to,
            ..Default::default()
        }
    }

    fn empty_port_meta() -> PortMetadata {
        PortMetadata {
            step_inputs: HashMap::new(),
            step_outputs: HashMap::new(),
            routing_rules: HashMap::new(),
            incoming_edges: HashMap::new(),
        }
    }

    fn mock_repo_returning(snapshots: Vec<EnvelopeSnapshotRow>) -> MockContentVersionRepo {
        let mut mock = MockContentVersionRepo::new();
        mock.expect_list_envelope_snapshots_for_run().returning(
            move |_| -> Result<Vec<EnvelopeSnapshotRow>, anyhow::Error> { Ok(snapshots.clone()) },
        );
        mock
    }

    // ── Reconstruction Tests ─────────────────────────────────────────────────

    #[tokio::test]
    async fn reconstruct_empty_produces_empty_state() {
        let mock = mock_repo_returning(vec![]);
        let port_meta = empty_port_meta();

        let state = reconstruct_state(&mock, &[], &port_meta, Uuid::new_v4())
            .await
            .unwrap();

        assert!(state.completed.is_empty());
        assert!(state.completed_envelopes.is_empty());
        assert!(state.var_outputs.is_empty());
    }

    #[tokio::test]
    async fn reconstruct_single_step_populates_all_maps() {
        let step_id = Uuid::new_v4();
        let run_id = Uuid::new_v4();
        let steps = vec![make_step(step_id, "default", Some("result"), 0)];

        let envelope = make_envelope(Some(json!({"answer": 42})));
        let snapshots = vec![EnvelopeSnapshotRow {
            step_id,
            content: serde_json::to_string(&envelope).unwrap(),
            source_id: step_id,
        }];

        let mock = mock_repo_returning(snapshots);
        let port_meta = empty_port_meta();

        let state = reconstruct_state(&mock, &steps, &port_meta, run_id)
            .await
            .unwrap();

        assert_eq!(state.completed.len(), 1);
        assert_eq!(state.completed_envelopes.len(), 1);
        assert!(state.completed.contains_key(&step_id));
        assert_eq!(state.var_outputs["result"], json!({"answer": 42}));
    }

    #[tokio::test]
    async fn reconstruct_multiple_steps() {
        let step_a = Uuid::new_v4();
        let step_b = Uuid::new_v4();
        let run_id = Uuid::new_v4();
        let steps = vec![
            make_step(step_a, "default", Some("analysis"), 0),
            make_step(step_b, "default", Some("summary"), 1),
        ];

        let env_a = make_envelope(Some(json!({"data": "analysis result"})));
        let env_b = make_envelope(Some(json!({"data": "summary result"})));

        let snapshots = vec![
            EnvelopeSnapshotRow {
                step_id: step_a,
                content: serde_json::to_string(&env_a).unwrap(),
                source_id: step_a,
            },
            EnvelopeSnapshotRow {
                step_id: step_b,
                content: serde_json::to_string(&env_b).unwrap(),
                source_id: step_b,
            },
        ];

        let mock = mock_repo_returning(snapshots);
        let port_meta = empty_port_meta();

        let state = reconstruct_state(&mock, &steps, &port_meta, run_id)
            .await
            .unwrap();

        assert_eq!(state.completed.len(), 2);
        assert_eq!(state.var_outputs.len(), 2);
        assert!(state.var_outputs.contains_key("analysis"));
        assert!(state.var_outputs.contains_key("summary"));
    }

    #[tokio::test]
    async fn reconstruct_error_then_retry_overwrites() {
        let step_id = Uuid::new_v4();
        let run_id = Uuid::new_v4();
        let steps = vec![make_step(step_id, "default", Some("result"), 0)];

        let error_env = StepExecutionEnvelope {
            status: ExecutionStatus::Error,
            data: None,
            metadata: ExecutionMetadata::new(Uuid::new_v4()),
            error: Some(crate::types::ExecutionError {
                message: "boom".to_string(),
                error_type: "test".to_string(),
                retryable: true,
                details: None,
            }),
        };
        let success_env = make_envelope(Some(json!({"retried": true})));

        // Snapshots ordered chronologically: error first, then success retry
        let snapshots = vec![
            EnvelopeSnapshotRow {
                step_id,
                content: serde_json::to_string(&error_env).unwrap(),
                source_id: step_id,
            },
            EnvelopeSnapshotRow {
                step_id,
                content: serde_json::to_string(&success_env).unwrap(),
                source_id: step_id,
            },
        ];

        let mock = mock_repo_returning(snapshots);
        let port_meta = empty_port_meta();

        let state = reconstruct_state(&mock, &steps, &port_meta, run_id)
            .await
            .unwrap();

        // Retry overwrites the error — step is completed, not failed
        assert_eq!(state.completed.len(), 1);
        assert!(state.failed.is_empty());
        assert_eq!(state.var_outputs["result"], json!({"retried": true}));
    }

    // ── Next-Step Computation Tests ──────────────────────────────────────────

    #[test]
    fn next_steps_with_empty_state_returns_entry_steps() {
        let step_a = Uuid::new_v4();
        let step_b = Uuid::new_v4();
        let steps = vec![
            make_step(step_a, "context", None, 0),
            make_step(step_b, "default", Some("out"), 1),
        ];
        let edges = vec![make_edge(step_a, step_b)];
        let dag_state = DagExecutionState::new();

        let next = next_executable_steps(&steps, &edges, &dag_state);

        assert_eq!(next.len(), 1);
        assert_eq!(next[0], step_a);
    }

    #[test]
    fn next_steps_after_first_completes() {
        let step_a = Uuid::new_v4();
        let step_b = Uuid::new_v4();
        let steps = vec![
            make_step(step_a, "context", None, 0),
            make_step(step_b, "default", Some("out"), 1),
        ];
        let edges = vec![make_edge(step_a, step_b)];

        let envelope = make_envelope(Some(json!("context data")));
        let mut completed = HashMap::new();
        completed.insert(
            step_a,
            StepOutput {
                variable_name: String::new(),
                structured_output: Some(json!("context data")),
                raw_output: String::new(),
            },
        );
        let mut completed_envelopes = HashMap::new();
        completed_envelopes.insert(step_a, envelope);

        let dag_state = DagExecutionState::from_snapshots(
            completed,
            HashMap::new(),
            completed_envelopes,
            HashMap::new(),
        );

        let next = next_executable_steps(&steps, &edges, &dag_state);

        assert_eq!(next.len(), 1);
        assert_eq!(next[0], step_b);
    }

    #[test]
    fn next_steps_all_completed_returns_empty() {
        let step_a = Uuid::new_v4();
        let steps = vec![make_step(step_a, "context", None, 0)];
        let edges = vec![];

        let envelope = make_envelope(Some(json!("data")));
        let mut completed = HashMap::new();
        completed.insert(
            step_a,
            StepOutput {
                variable_name: String::new(),
                structured_output: Some(json!("data")),
                raw_output: String::new(),
            },
        );
        let mut completed_envelopes = HashMap::new();
        completed_envelopes.insert(step_a, envelope);

        let dag_state = DagExecutionState::from_snapshots(
            completed,
            HashMap::new(),
            completed_envelopes,
            HashMap::new(),
        );

        let next = next_executable_steps(&steps, &edges, &dag_state);
        assert!(next.is_empty());
    }

    #[test]
    fn readiness_blocks_on_missing_upstream() {
        let step_a = Uuid::new_v4();
        let step_b = Uuid::new_v4();
        let step_c = Uuid::new_v4();
        let steps = vec![
            make_step(step_a, "context", None, 0),
            make_step(step_b, "default", Some("middle"), 1),
            make_step(step_c, "default", Some("end"), 2),
        ];
        let edges = vec![make_edge(step_a, step_b), make_edge(step_b, step_c)];
        let dag_state = DagExecutionState::new();

        let next = next_executable_steps(&steps, &edges, &dag_state);

        assert_eq!(next.len(), 1);
        assert_eq!(next[0], step_a);
    }
}
