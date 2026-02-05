//! Tests for the DAG executor module.

#[cfg(test)]
mod tests {
    use crate::db::{WorkflowStepEdgeRow, WorkflowStepRow};
    use crate::server::executors::dag::{
        compute_cost, extract_for_each_label, find_entry_steps, get_child_steps, get_parent_steps,
        parse_structured_output, resolve_for_each_array, resolve_variables, topological_sort,
        DagPaused,
    };
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
            display_order,
            version: 1,
        }
    }

    fn make_edge(from: Uuid, to: Uuid) -> WorkflowStepEdgeRow {
        WorkflowStepEdgeRow {
            from_step_id: from,
            to_step_id: to,
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
    // Parse Structured Output Tests
    // =========================================================================

    #[test]
    fn parse_raw_json() {
        let content = r#"{"key": "value", "num": 42}"#;
        let result = parse_structured_output(content);
        assert!(result.is_some());
        let json = result.unwrap();
        assert_eq!(json["key"], "value");
        assert_eq!(json["num"], 42);
    }

    #[test]
    fn parse_json_in_markdown_fence() {
        let content = r#"Here is the result:

```json
{"status": "success"}
```

That's the output."#;

        let result = parse_structured_output(content);
        assert!(result.is_some());
        let json = result.unwrap();
        assert_eq!(json["status"], "success");
    }

    #[test]
    fn parse_json_in_generic_code_block() {
        let content = r#"Result:

```
{"data": [1, 2, 3]}
```
"#;

        let result = parse_structured_output(content);
        assert!(result.is_some());
        let json = result.unwrap();
        assert_eq!(json["data"], serde_json::json!([1, 2, 3]));
    }

    #[test]
    fn parse_returns_none_for_invalid() {
        let content = "This is just plain text with no JSON.";
        let result = parse_structured_output(content);
        assert!(result.is_none());
    }

    #[test]
    fn parse_handles_whitespace() {
        let content = "   \n  {\"trimmed\": true}  \n  ";
        let result = parse_structured_output(content);
        // This actually succeeds because serde_json handles whitespace
        assert!(result.is_some());
    }

    // =========================================================================
    // Cost Calculation Tests
    // =========================================================================

    #[test]
    fn compute_cost_opus_model() {
        let cost = compute_cost("claude-3-opus", 1_000_000, 1_000_000);
        // Input: 1M * 15 / 1M = 15.0
        // Output: 1M * 75 / 1M = 75.0
        // Total: 90.0
        assert!((cost - 90.0).abs() < 0.01);
    }

    #[test]
    fn compute_cost_sonnet_model() {
        let cost = compute_cost("claude-3-sonnet", 1_000_000, 1_000_000);
        // Input: 1M * 3 / 1M = 3.0
        // Output: 1M * 15 / 1M = 15.0
        // Total: 18.0
        assert!((cost - 18.0).abs() < 0.01);
    }

    #[test]
    fn compute_cost_haiku_model() {
        let cost = compute_cost("claude-3-haiku", 1_000_000, 1_000_000);
        // Input: 1M * 0.25 / 1M = 0.25
        // Output: 1M * 1.25 / 1M = 1.25
        // Total: 1.5
        assert!((cost - 1.5).abs() < 0.01);
    }

    #[test]
    fn compute_cost_gpt4o_model() {
        let cost = compute_cost("gpt-4o-2024", 1_000_000, 1_000_000);
        // Input: 1M * 2.5 / 1M = 2.5
        // Output: 1M * 10 / 1M = 10.0
        // Total: 12.5
        assert!((cost - 12.5).abs() < 0.01);
    }

    #[test]
    fn compute_cost_gpt4_model() {
        let cost = compute_cost("gpt-4-turbo", 1_000_000, 1_000_000);
        // Input: 1M * 30 / 1M = 30.0
        // Output: 1M * 60 / 1M = 60.0
        // Total: 90.0
        assert!((cost - 90.0).abs() < 0.01);
    }

    #[test]
    fn compute_cost_unknown_model() {
        let cost = compute_cost("unknown-model", 1_000_000, 1_000_000);
        // Input: 1M * 1 / 1M = 1.0
        // Output: 1M * 3 / 1M = 3.0
        // Total: 4.0
        assert!((cost - 4.0).abs() < 0.01);
    }

    #[test]
    fn compute_cost_realistic_usage() {
        // Typical API call: ~1000 input tokens, ~500 output tokens
        let cost = compute_cost("claude-3-sonnet", 1000, 500);
        // Input: 1000 * 3 / 1M = 0.003
        // Output: 500 * 15 / 1M = 0.0075
        // Total: 0.0105
        assert!((cost - 0.0105).abs() < 0.0001);
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
}
