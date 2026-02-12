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
            status: crate::types::ExecutionStatus::Success,
            data: Some(json!({"key": "value"})),
            metadata: crate::types::ExecutionMetadata {
                execution_id: step_id,
                execution_time_ms: 0,
                tokens_in: None,
                tokens_out: None,
                cost_usd: None,
                model: None,
                agent_id: None,
                iteration_index: None,
                iteration_label: None,
                routing_label: None,

                upstream_agent_id: None,
                upstream_routing_label: None,
                room_session_id: None,
                room_id: None,
                total_rounds: None,
            },
            error: None,
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
            status: crate::types::ExecutionStatus::Success,
            data: Some(json!(42)),
            metadata: crate::types::ExecutionMetadata {
                execution_id: step_id,
                execution_time_ms: 0,
                tokens_in: None,
                tokens_out: None,
                cost_usd: None,
                model: None,
                agent_id: None,
                iteration_index: None,
                iteration_label: None,
                routing_label: None,

                upstream_agent_id: None,
                upstream_routing_label: None,
                room_session_id: None,
                room_id: None,
                total_rounds: None,
            },
            error: None,
        };

        state.record_step_output(step_id, output, envelope);

        assert!(state.completed.contains_key(&step_id));
        assert!(state.var_outputs.is_empty());
    }

    #[test]
    fn step_display_name_uses_variable_name() {
        let mut step = crate::db::WorkflowStepRow {
            id: Uuid::new_v4(),
            workflow_id: Uuid::new_v4(),
            agent_id: None,
            execution_mode: "single".into(),
            agent_execution_mode: None,
            for_each_ref: None,
            prompt_template_id: None,
            prompt_template: String::new(),
            output_schema_id: None,
            output_variable_name: Some("my_output".to_string()),
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
            name: None,
            system_prompt_suffix: None,
            visible: true,
            description: String::new(),
        };
        assert_eq!(step_display_name(&step), "my_output");

        step.output_variable_name = None;
        assert_eq!(step_display_name(&step), step.id.to_string());
    }
}
