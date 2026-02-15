#[cfg(test)]
mod tests {
    use crate::db::{EnvelopeSnapshotRow, WorkflowStepEdgeRow, WorkflowStepRow};
    use crate::server::hub::dag::dag_state::{DagExecutionState, PortMetadata};
    use crate::server::hub::dag::resolve_output_key;
    use crate::server::hub::dag::staging::compute_next_executable_steps;
    use crate::server::hub::dag::utils::StepOutput;
    use crate::types::{ExecutionMetadata, ExecutionStatus, StepExecutionEnvelope};
    use serde_json::json;
    use std::collections::HashMap;
    use uuid::Uuid;

    fn make_step(id: Uuid, mode: &str, var_name: Option<&str>, order: i32) -> WorkflowStepRow {
        WorkflowStepRow {
            id,
            workflow_id: Uuid::new_v4(),
            agent_id: Some(Uuid::new_v4()),
            execution_mode: mode.to_string(),
            agent_execution_mode: None,
            for_each_ref: None,
            prompt_template_id: None,
            prompt_template: String::new(),
            output_schema_id: None,
            output_variable_name: var_name.map(String::from),
            interactive_agent_id: None,
            for_each_label_field: None,
            room_id: None,
            routing_mode: None,
            routing_field: None,
            display_order: order,
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
            from_output_port: None,
            to_input_port: None,
            condition_type: None,
            condition_value: None,
            transform_jsonpath: None,
            edge_label: None,
        }
    }

    // ── Reconstruction Tests ───────────────────────────────────────────────

    #[test]
    fn reconstruct_from_empty_produces_empty_state() {
        let snapshots: Vec<EnvelopeSnapshotRow> = vec![];
        let steps: Vec<WorkflowStepRow> = vec![];
        let port_meta = PortMetadata {
            step_inputs: HashMap::new(),
            step_outputs: HashMap::new(),
            routing_rules: HashMap::new(),
        };

        let step_map: HashMap<Uuid, &WorkflowStepRow> = steps.iter().map(|s| (s.id, s)).collect();

        let mut completed = HashMap::new();
        let mut completed_envelopes = HashMap::new();
        let mut var_outputs = HashMap::new();

        for snapshot in &snapshots {
            let envelope: StepExecutionEnvelope = serde_json::from_str(&snapshot.content).unwrap();
            let variable_name = step_map
                .get(&snapshot.step_id)
                .map(|s| resolve_output_key(s, &port_meta.step_outputs))
                .unwrap_or_default();

            let step_output = StepOutput {
                variable_name: variable_name.clone(),
                structured_output: envelope.data.clone(),
                raw_output: String::new(),
            };

            if !variable_name.is_empty() {
                if let Some(ref data) = envelope.data {
                    var_outputs.insert(variable_name, data.clone());
                }
            }

            completed_envelopes.insert(snapshot.step_id, envelope);
            completed.insert(snapshot.step_id, step_output);
        }

        assert!(completed.is_empty());
        assert!(completed_envelopes.is_empty());
        assert!(var_outputs.is_empty());
    }

    #[test]
    fn reconstruct_single_step_populates_all_maps() {
        let step_id = Uuid::new_v4();
        let steps = vec![make_step(step_id, "default", Some("result"), 0)];

        let envelope = make_envelope(Some(json!({"answer": 42})));
        let envelope_json = serde_json::to_string(&envelope).unwrap();

        let snapshots = vec![EnvelopeSnapshotRow {
            step_id,
            content: envelope_json,
            source_id: step_id,
        }];

        let port_meta = PortMetadata {
            step_inputs: HashMap::new(),
            step_outputs: HashMap::new(),
            routing_rules: HashMap::new(),
        };

        let step_map: HashMap<Uuid, &WorkflowStepRow> = steps.iter().map(|s| (s.id, s)).collect();

        let mut completed = HashMap::new();
        let mut completed_envelopes = HashMap::new();
        let mut var_outputs = HashMap::new();

        for snapshot in &snapshots {
            let env: StepExecutionEnvelope = serde_json::from_str(&snapshot.content).unwrap();
            let variable_name = step_map
                .get(&snapshot.step_id)
                .map(|s| resolve_output_key(s, &port_meta.step_outputs))
                .unwrap_or_default();

            let step_output = StepOutput {
                variable_name: variable_name.clone(),
                structured_output: env.data.clone(),
                raw_output: String::new(),
            };

            if !variable_name.is_empty() {
                if let Some(ref data) = env.data {
                    var_outputs.insert(variable_name, data.clone());
                }
            }

            completed_envelopes.insert(snapshot.step_id, env);
            completed.insert(snapshot.step_id, step_output);
        }

        assert_eq!(completed.len(), 1);
        assert_eq!(completed_envelopes.len(), 1);
        assert_eq!(var_outputs.len(), 1);
        assert!(completed.contains_key(&step_id));
        assert!(completed_envelopes.contains_key(&step_id));
        assert_eq!(var_outputs["result"], json!({"answer": 42}));
    }

    #[test]
    fn reconstruct_multiple_steps() {
        let step_a = Uuid::new_v4();
        let step_b = Uuid::new_v4();
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

        let port_meta = PortMetadata {
            step_inputs: HashMap::new(),
            step_outputs: HashMap::new(),
            routing_rules: HashMap::new(),
        };

        let step_map: HashMap<Uuid, &WorkflowStepRow> = steps.iter().map(|s| (s.id, s)).collect();

        let mut completed = HashMap::new();
        let mut completed_envelopes = HashMap::new();
        let mut var_outputs = HashMap::new();

        for snapshot in &snapshots {
            let env: StepExecutionEnvelope = serde_json::from_str(&snapshot.content).unwrap();
            let variable_name = step_map
                .get(&snapshot.step_id)
                .map(|s| resolve_output_key(s, &port_meta.step_outputs))
                .unwrap_or_default();

            let step_output = StepOutput {
                variable_name: variable_name.clone(),
                structured_output: env.data.clone(),
                raw_output: String::new(),
            };

            if !variable_name.is_empty() {
                if let Some(ref data) = env.data {
                    var_outputs.insert(variable_name, data.clone());
                }
            }

            completed_envelopes.insert(snapshot.step_id, env);
            completed.insert(snapshot.step_id, step_output);
        }

        assert_eq!(completed.len(), 2);
        assert_eq!(var_outputs.len(), 2);
        assert!(var_outputs.contains_key("analysis"));
        assert!(var_outputs.contains_key("summary"));
    }

    // ── Next-Step Computation Tests ────────────────────────────────────────

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

        let next = compute_next_executable_steps(&steps, &edges, &dag_state);

        // Only step_a (entry step) should be ready
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

        let dag_state =
            DagExecutionState::from_snapshots(completed, HashMap::new(), completed_envelopes);

        let next = compute_next_executable_steps(&steps, &edges, &dag_state);

        // Now step_b should be ready
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

        let dag_state =
            DagExecutionState::from_snapshots(completed, HashMap::new(), completed_envelopes);

        let next = compute_next_executable_steps(&steps, &edges, &dag_state);
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

        let next = compute_next_executable_steps(&steps, &edges, &dag_state);

        // Only step_a should be ready; step_b blocked by step_a, step_c blocked by step_b
        assert_eq!(next.len(), 1);
        assert_eq!(next[0], step_a);
    }
}
