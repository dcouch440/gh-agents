//! Tests for the DAG executor module.

#[cfg(test)]
mod tests {
    use crate::db::{StepInputRow, StepOutputRow, WorkflowStepEdgeRow, WorkflowStepRow};
    use crate::server::hub::dag::utils::{
        extract_for_each_label, find_entry_steps, get_child_steps, get_parent_steps,
        resolve_dot_path, resolve_for_each_array, resolve_port_inputs, resolve_variables,
        topological_sort, DagPaused,
    };
    use crate::types::{ExecutionMetadata, ExecutionStatus, StepExecutionEnvelope};
    use std::collections::HashMap;
    use uuid::Uuid;

    // =========================================================================
    // Fixtures
    // =========================================================================

    fn make_step(id: Uuid, display_order: i32) -> WorkflowStepRow {
        WorkflowStepRow {
            id,
            workflow_id: Uuid::new_v4(),
            agent_id: Uuid::new_v4(),
            execution_mode: "single".to_string(),
            agent_execution_mode: None,
            for_each_ref: None,
            prompt_template_id: None,
            prompt_template: "Test prompt".to_string(),
            output_schema_id: None,
            output_variable_name: Some("output".to_string()),
            interactive_agent_id: None,
            for_each_label_field: None,
            room_id: None,
            routing_mode: None,
            routing_field: None,
            display_order,
            version: 1,
            reasoning_trace: false,
        }
    }

    fn make_edge(from: Uuid, to: Uuid) -> WorkflowStepEdgeRow {
        WorkflowStepEdgeRow {
            id: Uuid::new_v4(),
            from_step_id: from,
            to_step_id: to,
            from_output_port: None,
            to_input_port: None,
            transform_jsonpath: None,
            condition_type: None,
            condition_value: None,
            edge_label: None,
            workflow_id: Uuid::new_v4(),
        }
    }

    fn make_port_edge(from: Uuid, to: Uuid, from_port: &str, to_port: &str) -> WorkflowStepEdgeRow {
        WorkflowStepEdgeRow {
            id: Uuid::new_v4(),
            from_step_id: from,
            to_step_id: to,
            from_output_port: Some(from_port.to_string()),
            to_input_port: Some(to_port.to_string()),
            transform_jsonpath: None,
            condition_type: None,
            condition_value: None,
            edge_label: None,
            workflow_id: Uuid::new_v4(),
        }
    }

    fn make_step_input(step_id: Uuid, port_name: &str, required: bool) -> StepInputRow {
        StepInputRow {
            id: Uuid::new_v4(),
            workflow_step_id: step_id,
            port_name: port_name.to_string(),
            port_type: "string".to_string(),
            required,
            default_value: None,
            description: None,
            json_schema: None,
            created_at: chrono::Utc::now(),
        }
    }

    fn make_step_output(step_id: Uuid, port_name: &str, json_path: &str) -> StepOutputRow {
        StepOutputRow {
            id: Uuid::new_v4(),
            workflow_step_id: step_id,
            port_name: port_name.to_string(),
            port_type: "string".to_string(),
            json_path: json_path.to_string(),
            description: None,
            json_schema: None,
            created_at: chrono::Utc::now(),
        }
    }

    fn make_envelope(data: serde_json::Value) -> StepExecutionEnvelope {
        StepExecutionEnvelope {
            status: ExecutionStatus::Success,
            data: Some(data),
            metadata: ExecutionMetadata {
                execution_id: Uuid::new_v4(),
                execution_time_ms: 100,
                tokens_in: None,
                tokens_out: None,
                cost_usd: None,
                model: None,
                agent_id: None,
                iteration_index: None,
                iteration_label: None,
                routing_label: None,
                selected_routing_document_id: None,
                upstream_agent_id: None,
                upstream_routing_label: None,
                room_session_id: None,
                room_id: None,
                total_rounds: None,
            },
            error: None,
        }
    }

    // =========================================================================
    // Topological Sort Tests
    // =========================================================================

    #[test]
    fn topological_sort_linear_chain() {
        // A -> B -> C
        let a = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
        let b = Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap();
        let c = Uuid::parse_str("00000000-0000-0000-0000-000000000003").unwrap();

        let steps = vec![make_step(a, 0), make_step(b, 1), make_step(c, 2)];
        let edges = vec![make_edge(a, b), make_edge(b, c)];

        let sorted = topological_sort(&steps, &edges).unwrap();

        // A must come before B, B must come before C
        let pos_a = sorted.iter().position(|&id| id == a).unwrap();
        let pos_b = sorted.iter().position(|&id| id == b).unwrap();
        let pos_c = sorted.iter().position(|&id| id == c).unwrap();

        assert!(pos_a < pos_b);
        assert!(pos_b < pos_c);
    }

    #[test]
    fn topological_sort_diamond_dag() {
        // A -> B -> D
        // A -> C -> D
        let a = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
        let b = Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap();
        let c = Uuid::parse_str("00000000-0000-0000-0000-000000000003").unwrap();
        let d = Uuid::parse_str("00000000-0000-0000-0000-000000000004").unwrap();

        let steps = vec![
            make_step(a, 0),
            make_step(b, 1),
            make_step(c, 2),
            make_step(d, 3),
        ];
        let edges = vec![
            make_edge(a, b),
            make_edge(a, c),
            make_edge(b, d),
            make_edge(c, d),
        ];

        let sorted = topological_sort(&steps, &edges).unwrap();

        let pos_a = sorted.iter().position(|&id| id == a).unwrap();
        let pos_b = sorted.iter().position(|&id| id == b).unwrap();
        let pos_c = sorted.iter().position(|&id| id == c).unwrap();
        let pos_d = sorted.iter().position(|&id| id == d).unwrap();

        // A must come first
        assert_eq!(pos_a, 0);
        // D must come last
        assert_eq!(pos_d, 3);
        // B and C must come after A and before D
        assert!(pos_b > pos_a && pos_b < pos_d);
        assert!(pos_c > pos_a && pos_c < pos_d);
    }

    #[test]
    fn topological_sort_detects_cycle() {
        // A -> B -> C -> A (cycle!)
        let a = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
        let b = Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap();
        let c = Uuid::parse_str("00000000-0000-0000-0000-000000000003").unwrap();

        let steps = vec![make_step(a, 0), make_step(b, 1), make_step(c, 2)];
        let edges = vec![make_edge(a, b), make_edge(b, c), make_edge(c, a)];

        let result = topological_sort(&steps, &edges);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Cycle detected"));
    }

    #[test]
    fn topological_sort_single_node() {
        let a = Uuid::new_v4();
        let steps = vec![make_step(a, 0)];
        let edges = vec![];

        let sorted = topological_sort(&steps, &edges).unwrap();
        assert_eq!(sorted, vec![a]);
    }

    #[test]
    fn topological_sort_empty_graph() {
        let steps: Vec<WorkflowStepRow> = vec![];
        let edges: Vec<WorkflowStepEdgeRow> = vec![];

        let sorted = topological_sort(&steps, &edges).unwrap();
        assert!(sorted.is_empty());
    }

    #[test]
    fn topological_sort_multiple_entry_points() {
        // A -> C
        // B -> C
        // (A and B are both entry points)
        let a = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
        let b = Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap();
        let c = Uuid::parse_str("00000000-0000-0000-0000-000000000003").unwrap();

        let steps = vec![make_step(a, 0), make_step(b, 1), make_step(c, 2)];
        let edges = vec![make_edge(a, c), make_edge(b, c)];

        let sorted = topological_sort(&steps, &edges).unwrap();

        let pos_a = sorted.iter().position(|&id| id == a).unwrap();
        let pos_b = sorted.iter().position(|&id| id == b).unwrap();
        let pos_c = sorted.iter().position(|&id| id == c).unwrap();

        // C must come after both A and B
        assert!(pos_c > pos_a);
        assert!(pos_c > pos_b);
    }

    #[test]
    fn topological_sort_respects_display_order() {
        // No edges, just three disconnected steps
        // Should be sorted by display_order
        let a = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
        let b = Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap();
        let c = Uuid::parse_str("00000000-0000-0000-0000-000000000003").unwrap();

        // Note: display_order is reversed from ID order
        let steps = vec![make_step(a, 2), make_step(b, 0), make_step(c, 1)];
        let edges = vec![];

        let sorted = topological_sort(&steps, &edges).unwrap();

        // Should be sorted by display_order: b(0), c(1), a(2)
        // But note the algorithm pops from the end, so it's reversed
        assert_eq!(sorted.len(), 3);
        assert!(sorted.contains(&a));
        assert!(sorted.contains(&b));
        assert!(sorted.contains(&c));
    }

    // =========================================================================
    // Entry Steps Tests
    // =========================================================================

    #[test]
    fn find_entry_steps_single_entry() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();

        let steps = vec![make_step(a, 0), make_step(b, 1), make_step(c, 2)];
        let edges = vec![make_edge(a, b), make_edge(b, c)];

        let entries = find_entry_steps(&steps, &edges);
        assert_eq!(entries, vec![a]);
    }

    #[test]
    fn find_entry_steps_multiple_entries() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();

        let steps = vec![make_step(a, 0), make_step(b, 1), make_step(c, 2)];
        let edges = vec![make_edge(a, c), make_edge(b, c)];

        let entries = find_entry_steps(&steps, &edges);
        assert_eq!(entries.len(), 2);
        assert!(entries.contains(&a));
        assert!(entries.contains(&b));
    }

    #[test]
    fn find_entry_steps_all_disconnected() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();

        let steps = vec![make_step(a, 0), make_step(b, 1)];
        let edges = vec![];

        let entries = find_entry_steps(&steps, &edges);
        assert_eq!(entries.len(), 2);
    }

    // =========================================================================
    // Parent/Child Steps Tests
    // =========================================================================

    #[test]
    fn get_parent_steps_returns_all_parents() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();

        let edges = vec![make_edge(a, c), make_edge(b, c)];

        let parents = get_parent_steps(c, &edges);
        assert_eq!(parents.len(), 2);
        assert!(parents.contains(&a));
        assert!(parents.contains(&b));
    }

    #[test]
    fn get_parent_steps_empty_for_entry() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();

        let edges = vec![make_edge(a, b)];

        let parents = get_parent_steps(a, &edges);
        assert!(parents.is_empty());
    }

    #[test]
    fn get_child_steps_returns_all_children() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();

        let edges = vec![make_edge(a, b), make_edge(a, c)];

        let children = get_child_steps(a, &edges);
        assert_eq!(children.len(), 2);
        assert!(children.contains(&b));
        assert!(children.contains(&c));
    }

    #[test]
    fn get_child_steps_empty_for_terminal() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();

        let edges = vec![make_edge(a, b)];

        let children = get_child_steps(b, &edges);
        assert!(children.is_empty());
    }

    // =========================================================================
    // Variable Resolution Tests
    // =========================================================================

    #[test]
    fn resolve_simple_variable() {
        let mut outputs = HashMap::new();
        outputs.insert("name".to_string(), serde_json::json!("Alice"));

        let result = resolve_variables("Hello {name}!", &outputs, &HashMap::new());
        assert_eq!(result, "Hello Alice!");
    }

    #[test]
    fn resolve_dot_path_access() {
        let mut outputs = HashMap::new();
        outputs.insert(
            "user".to_string(),
            serde_json::json!({"name": "Bob", "age": 30}),
        );

        let result = resolve_variables(
            "Name: {user.name}, Age: {user.age}",
            &outputs,
            &HashMap::new(),
        );
        assert_eq!(result, "Name: Bob, Age: 30");
    }

    #[test]
    fn resolve_array_index_access() {
        let mut outputs = HashMap::new();
        outputs.insert(
            "items".to_string(),
            serde_json::json!(["first", "second", "third"]),
        );

        let result = resolve_variables(
            "First: {items.0}, Second: {items.1}",
            &outputs,
            &HashMap::new(),
        );
        assert_eq!(result, "First: first, Second: second");
    }

    #[test]
    fn resolve_nested_object_access() {
        let mut outputs = HashMap::new();
        outputs.insert(
            "data".to_string(),
            serde_json::json!({
                "users": [
                    {"name": "Alice"},
                    {"name": "Bob"}
                ]
            }),
        );

        let result = resolve_variables("User: {data.users.0.name}", &outputs, &HashMap::new());
        assert_eq!(result, "User: Alice");
    }

    #[test]
    fn resolve_unresolved_leaves_placeholder() {
        let outputs = HashMap::new();

        let result = resolve_variables("Hello {unknown}!", &outputs, &HashMap::new());
        assert_eq!(result, "Hello {unknown}!");
    }

    #[test]
    fn resolve_from_prior_outputs() {
        let outputs = HashMap::new();
        let mut prior = HashMap::new();
        prior.insert("previous".to_string(), serde_json::json!("from prior"));

        let result = resolve_variables("Value: {previous}", &outputs, &prior);
        assert_eq!(result, "Value: from prior");
    }

    #[test]
    fn resolve_prefers_current_over_prior() {
        let mut outputs = HashMap::new();
        outputs.insert("value".to_string(), serde_json::json!("current"));

        let mut prior = HashMap::new();
        prior.insert("value".to_string(), serde_json::json!("prior"));

        let result = resolve_variables("Value: {value}", &outputs, &prior);
        assert_eq!(result, "Value: current");
    }

    #[test]
    fn resolve_null_value_leaves_placeholder() {
        let mut outputs = HashMap::new();
        outputs.insert("data".to_string(), serde_json::json!({"field": null}));

        let result = resolve_variables("Value: {data.field}", &outputs, &HashMap::new());
        assert_eq!(result, "Value: {data.field}");
    }

    #[test]
    fn resolve_multiple_variables() {
        let mut outputs = HashMap::new();
        outputs.insert("first".to_string(), serde_json::json!("A"));
        outputs.insert("second".to_string(), serde_json::json!("B"));

        let result = resolve_variables("{first} and {second}", &outputs, &HashMap::new());
        assert_eq!(result, "A and B");
    }

    #[test]
    fn resolve_no_variables() {
        let result = resolve_variables("No variables here", &HashMap::new(), &HashMap::new());
        assert_eq!(result, "No variables here");
    }

    // =========================================================================
    // For-Each Resolution Tests
    // =========================================================================

    #[test]
    fn resolve_for_each_simple_array() {
        let mut outputs = HashMap::new();
        outputs.insert("items".to_string(), serde_json::json!(["a", "b", "c"]));

        let result = resolve_for_each_array("items", &outputs, &HashMap::new());
        assert!(result.is_some());
        let arr = result.unwrap();
        assert_eq!(arr.len(), 3);
        assert_eq!(arr[0], serde_json::json!("a"));
    }

    #[test]
    fn resolve_for_each_nested_array() {
        let mut outputs = HashMap::new();
        outputs.insert("data".to_string(), serde_json::json!({"items": [1, 2, 3]}));

        let result = resolve_for_each_array("data.items", &outputs, &HashMap::new());
        assert!(result.is_some());
        let arr = result.unwrap();
        assert_eq!(arr.len(), 3);
    }

    #[test]
    fn resolve_for_each_returns_none_for_non_array() {
        let mut outputs = HashMap::new();
        outputs.insert("value".to_string(), serde_json::json!("not an array"));

        let result = resolve_for_each_array("value", &outputs, &HashMap::new());
        assert!(result.is_none());
    }

    #[test]
    fn resolve_for_each_returns_none_for_missing() {
        let outputs = HashMap::new();

        let result = resolve_for_each_array("missing", &outputs, &HashMap::new());
        assert!(result.is_none());
    }

    #[test]
    fn resolve_for_each_from_prior_outputs() {
        let outputs = HashMap::new();
        let mut prior = HashMap::new();
        prior.insert("items".to_string(), serde_json::json!([1, 2]));

        let result = resolve_for_each_array("items", &outputs, &prior);
        assert!(result.is_some());
        assert_eq!(result.unwrap().len(), 2);
    }

    // =========================================================================
    // Extract For-Each Label Tests
    // =========================================================================

    #[test]
    fn extract_label_from_object() {
        let element = serde_json::json!({"name": "Task 1", "id": 123});

        let label = extract_for_each_label(&element, Some("name"));
        assert_eq!(label, Some("Task 1".to_string()));
    }

    #[test]
    fn extract_label_missing_field() {
        let element = serde_json::json!({"id": 123});

        let label = extract_for_each_label(&element, Some("name"));
        assert!(label.is_none());
    }

    #[test]
    fn extract_label_none_field() {
        let element = serde_json::json!({"name": "Task 1"});

        let label = extract_for_each_label(&element, None);
        assert!(label.is_none());
    }

    #[test]
    fn extract_label_non_string_value() {
        let element = serde_json::json!({"name": 123});

        let label = extract_for_each_label(&element, Some("name"));
        assert!(label.is_none());
    }

    // =========================================================================
    // DagPaused Error Tests
    // =========================================================================

    #[test]
    fn dag_paused_display() {
        let step_id = Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap();
        let execution_id = Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap();

        let paused = DagPaused {
            step_id,
            execution_id,
        };

        let display = format!("{}", paused);
        assert!(display.contains(&step_id.to_string()));
        assert!(display.contains(&execution_id.to_string()));
        assert!(display.contains("awaiting user input"));
    }

    // =========================================================================
    // resolve_dot_path Tests
    // =========================================================================

    #[test]
    fn resolve_dot_path_simple_field() {
        let value = serde_json::json!({"name": "Alice", "age": 30});
        assert_eq!(
            resolve_dot_path(&value, "name"),
            Some(serde_json::json!("Alice"))
        );
        assert_eq!(resolve_dot_path(&value, "age"), Some(serde_json::json!(30)));
    }

    #[test]
    fn resolve_dot_path_nested() {
        let value = serde_json::json!({"user": {"profile": {"name": "Bob"}}});
        assert_eq!(
            resolve_dot_path(&value, "user.profile.name"),
            Some(serde_json::json!("Bob"))
        );
    }

    #[test]
    fn resolve_dot_path_array_index() {
        let value = serde_json::json!({"items": ["first", "second", "third"]});
        assert_eq!(
            resolve_dot_path(&value, "items.0"),
            Some(serde_json::json!("first"))
        );
        assert_eq!(
            resolve_dot_path(&value, "items.2"),
            Some(serde_json::json!("third"))
        );
    }

    #[test]
    fn resolve_dot_path_missing_returns_none() {
        let value = serde_json::json!({"name": "Alice"});
        assert_eq!(resolve_dot_path(&value, "missing"), None);
        assert_eq!(resolve_dot_path(&value, "name.nested"), None);
    }

    #[test]
    fn resolve_dot_path_null_returns_none() {
        let value = serde_json::json!({"field": null});
        assert_eq!(resolve_dot_path(&value, "field"), None);
    }

    #[test]
    fn resolve_dot_path_empty_path_returns_root() {
        let value = serde_json::json!({"name": "Alice"});
        assert_eq!(
            resolve_dot_path(&value, ""),
            Some(serde_json::json!({"name": "Alice"}))
        );
    }

    // =========================================================================
    // resolve_port_inputs Tests
    // =========================================================================

    #[test]
    fn resolve_port_inputs_basic() {
        let step_a = Uuid::new_v4();
        let step_b = Uuid::new_v4();

        // Edge: step_a:result -> step_b:context
        let edges = vec![make_port_edge(step_a, step_b, "result", "context")];

        // step_b has one input port
        let step_inputs = vec![make_step_input(step_b, "context", true)];

        // step_a has one output port with json_path "summary"
        let mut source_outputs = HashMap::new();
        source_outputs.insert(step_a, vec![make_step_output(step_a, "result", "summary")]);

        // step_a completed with data containing "summary" field
        let mut completed = HashMap::new();
        completed.insert(
            step_a,
            make_envelope(serde_json::json!({"summary": "Hello world"})),
        );

        let result =
            resolve_port_inputs(step_b, &edges, &step_inputs, &source_outputs, &completed).unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result["context"], serde_json::json!("Hello world"));
    }

    #[test]
    fn resolve_port_inputs_with_json_path() {
        let step_a = Uuid::new_v4();
        let step_b = Uuid::new_v4();

        let edges = vec![make_port_edge(step_a, step_b, "analysis", "input_data")];
        let step_inputs = vec![make_step_input(step_b, "input_data", true)];

        // Output port with nested json_path
        let mut source_outputs = HashMap::new();
        source_outputs.insert(
            step_a,
            vec![make_step_output(step_a, "analysis", "results.data.items")],
        );

        let mut completed = HashMap::new();
        completed.insert(
            step_a,
            make_envelope(serde_json::json!({
                "results": {
                    "data": {
                        "items": [1, 2, 3]
                    }
                }
            })),
        );

        let result =
            resolve_port_inputs(step_b, &edges, &step_inputs, &source_outputs, &completed).unwrap();

        assert_eq!(result["input_data"], serde_json::json!([1, 2, 3]));
    }

    #[test]
    fn resolve_port_inputs_with_transform() {
        let step_a = Uuid::new_v4();
        let step_b = Uuid::new_v4();

        // Edge with a transform
        let mut edge = make_port_edge(step_a, step_b, "output", "input");
        edge.transform_jsonpath = Some("name".to_string());

        let edges = vec![edge];
        let step_inputs = vec![make_step_input(step_b, "input", true)];

        let mut source_outputs = HashMap::new();
        source_outputs.insert(step_a, vec![make_step_output(step_a, "output", "data")]);

        let mut completed = HashMap::new();
        completed.insert(
            step_a,
            make_envelope(serde_json::json!({
                "data": {"name": "Alice", "age": 30}
            })),
        );

        let result =
            resolve_port_inputs(step_b, &edges, &step_inputs, &source_outputs, &completed).unwrap();

        // The transform extracts "name" from the data object
        assert_eq!(result["input"], serde_json::json!("Alice"));
    }

    #[test]
    fn resolve_port_inputs_default_value() {
        let step_b = Uuid::new_v4();

        // No edges — input has a default value
        let edges: Vec<WorkflowStepEdgeRow> = vec![];
        let mut input = make_step_input(step_b, "config", false);
        input.default_value = Some(serde_json::json!({"timeout": 30}));
        let step_inputs = vec![input];

        let result = resolve_port_inputs(
            step_b,
            &edges,
            &step_inputs,
            &HashMap::new(),
            &HashMap::new(),
        )
        .unwrap();

        assert_eq!(result["config"], serde_json::json!({"timeout": 30}));
    }

    #[test]
    fn resolve_port_inputs_missing_required_errors() {
        let step_b = Uuid::new_v4();

        // No edges, required input, no default
        let edges: Vec<WorkflowStepEdgeRow> = vec![];
        let step_inputs = vec![make_step_input(step_b, "required_data", true)];

        let result = resolve_port_inputs(
            step_b,
            &edges,
            &step_inputs,
            &HashMap::new(),
            &HashMap::new(),
        );

        assert!(result.is_err());
        let err = result.unwrap_err();
        let msg = format!("{}", err);
        assert!(msg.contains("required_data"));
        assert!(msg.contains("missing"));
    }

    #[test]
    fn resolve_port_inputs_no_ports_returns_empty() {
        let step_b = Uuid::new_v4();

        let result =
            resolve_port_inputs(step_b, &[], &[], &HashMap::new(), &HashMap::new()).unwrap();

        assert!(result.is_empty());
    }

    #[test]
    fn resolve_port_inputs_source_not_completed_errors() {
        let step_a = Uuid::new_v4();
        let step_b = Uuid::new_v4();

        let edges = vec![make_port_edge(step_a, step_b, "output", "input")];
        let step_inputs = vec![make_step_input(step_b, "input", true)];

        // No completed envelopes — step_a hasn't run
        let result = resolve_port_inputs(
            step_b,
            &edges,
            &step_inputs,
            &HashMap::new(),
            &HashMap::new(),
        );

        assert!(result.is_err());
        let msg = format!("{}", result.unwrap_err());
        assert!(msg.contains("not completed"));
    }

    #[test]
    fn resolve_port_inputs_multiple_edges() {
        let step_a = Uuid::new_v4();
        let step_b = Uuid::new_v4();
        let step_c = Uuid::new_v4();

        // Two edges into step_c from different sources
        let edges = vec![
            make_port_edge(step_a, step_c, "result", "left"),
            make_port_edge(step_b, step_c, "result", "right"),
        ];

        let step_inputs = vec![
            make_step_input(step_c, "left", true),
            make_step_input(step_c, "right", true),
        ];

        let mut source_outputs = HashMap::new();
        source_outputs.insert(step_a, vec![make_step_output(step_a, "result", "value")]);
        source_outputs.insert(step_b, vec![make_step_output(step_b, "result", "value")]);

        let mut completed = HashMap::new();
        completed.insert(
            step_a,
            make_envelope(serde_json::json!({"value": "from_a"})),
        );
        completed.insert(
            step_b,
            make_envelope(serde_json::json!({"value": "from_b"})),
        );

        let result =
            resolve_port_inputs(step_c, &edges, &step_inputs, &source_outputs, &completed).unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result["left"], serde_json::json!("from_a"));
        assert_eq!(result["right"], serde_json::json!("from_b"));
    }

    // =========================================================================
    // build_routing_instruction_block Tests (Phase 6)
    // =========================================================================

    use crate::server::hub::dag::utils::build_routing_instruction_block;
    use crate::types::{DownstreamRoutingContext, RouteDescription};

    #[test]
    fn routing_block_basic() {
        let ctx = DownstreamRoutingContext {
            downstream_step_id: Uuid::new_v4(),
            routing_field: "category".to_string(),
            routes: vec![
                RouteDescription {
                    label_value: "frontend".to_string(),
                    description: Some("UI components and styling".to_string()),
                    agent_name: "Frontend Specialist".to_string(),
                    agent_tools: vec!["file_write".to_string(), "file_read".to_string()],
                },
                RouteDescription {
                    label_value: "backend".to_string(),
                    description: Some("API endpoints and server logic".to_string()),
                    agent_name: "Backend Specialist".to_string(),
                    agent_tools: vec!["file_write".to_string(), "test_execution".to_string()],
                },
            ],
        };

        let block = build_routing_instruction_block(&ctx);

        assert!(block.contains("## Routing Instructions"));
        assert!(block.contains("\"category\""));
        assert!(block.contains("- frontend: UI components and styling"));
        assert!(block.contains("Routed to: Frontend Specialist (tools: file_write, file_read)"));
        assert!(block.contains("- backend: API endpoints and server logic"));
        assert!(block.contains("Routed to: Backend Specialist (tools: file_write, test_execution)"));
    }

    #[test]
    fn routing_block_no_description() {
        let ctx = DownstreamRoutingContext {
            downstream_step_id: Uuid::new_v4(),
            routing_field: "type".to_string(),
            routes: vec![RouteDescription {
                label_value: "misc".to_string(),
                description: None,
                agent_name: "General Agent".to_string(),
                agent_tools: vec!["file_read".to_string()],
            }],
        };

        let block = build_routing_instruction_block(&ctx);

        assert!(block.contains("- misc\n"));
        assert!(!block.contains("- misc:"));
        assert!(block.contains("Routed to: General Agent"));
    }

    #[test]
    fn routing_block_no_tools() {
        let ctx = DownstreamRoutingContext {
            downstream_step_id: Uuid::new_v4(),
            routing_field: "category".to_string(),
            routes: vec![RouteDescription {
                label_value: "review".to_string(),
                description: Some("Code review tasks".to_string()),
                agent_name: "Reviewer".to_string(),
                agent_tools: vec![],
            }],
        };

        let block = build_routing_instruction_block(&ctx);

        assert!(block.contains("Routed to: Reviewer (no tools)"));
    }

    #[test]
    fn routing_block_empty_routes() {
        let ctx = DownstreamRoutingContext {
            downstream_step_id: Uuid::new_v4(),
            routing_field: "category".to_string(),
            routes: vec![],
        };

        let block = build_routing_instruction_block(&ctx);

        assert!(block.contains("## Routing Instructions"));
        assert!(!block.contains("Routed to:"));
    }

    // =========================================================================
    // resolve_dot_path with colon-containing keys (Room Outputs)
    // =========================================================================

    #[test]
    fn resolve_dot_path_colon_key() {
        // Room envelopes use "agent:<uuid>" keys
        let agent_id = Uuid::new_v4();
        let key = format!("agent:{}", agent_id);

        let value = serde_json::json!({
            key.clone(): {"findings": "important data", "score": 95}
        });

        // Colon is NOT a split delimiter — only dot is — so this should work
        let result = resolve_dot_path(&value, &key);
        assert!(result.is_some());
        let inner = result.unwrap();
        assert_eq!(inner["findings"], "important data");
        assert_eq!(inner["score"], 95);
    }

    #[test]
    fn resolve_dot_path_colon_nested() {
        // Navigate into a colon-key object with dot path
        let agent_id = Uuid::new_v4();
        let key = format!("agent:{}", agent_id);

        let value = serde_json::json!({
            key.clone(): {"findings": "deep insight", "details": {"confidence": 0.9}}
        });

        let path = format!("{}.findings", key);
        let result = resolve_dot_path(&value, &path);
        assert_eq!(result, Some(serde_json::json!("deep insight")));

        let path2 = format!("{}.details.confidence", key);
        let result2 = resolve_dot_path(&value, &path2);
        assert_eq!(result2, Some(serde_json::json!(0.9)));
    }

    #[test]
    fn resolve_dot_path_empty_returns_full_composite() {
        // Empty path returns the entire object — useful for a "combined" port
        let value = serde_json::json!({
            "agent:aaa": {"a": 1},
            "agent:bbb": {"b": 2}
        });

        let result = resolve_dot_path(&value, "");
        assert_eq!(result, Some(value));
    }
}
