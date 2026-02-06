//! Tests for DAG module

use crate::db::{WorkflowStepEdgeRow, WorkflowStepRow};
use crate::server::executors::dag::{resolve_for_each_array, resolve_variables, topological_sort};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use uuid::Uuid;

fn make_step(
    id: Uuid,
    prompt: &str,
    var_name: Option<&str>,
    display_order: i32,
) -> WorkflowStepRow {
    WorkflowStepRow {
        id,
        workflow_id: Uuid::new_v4(),
        agent_id: Uuid::new_v4(),
        execution_mode: "single".into(),
        agent_execution_mode: None,
        for_each_ref: None,
        prompt_template_id: None,
        prompt_template: prompt.into(),
        output_schema_id: None,
        output_variable_name: var_name.map(|s| s.into()),
        interactive_agent_id: None,
        for_each_label_field: None,
        room_id: None,
        routing_mode: None,
        routing_field: None,
        display_order,
        version: 1,
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

#[test]
fn topo_sort_linear() {
    let s1 = Uuid::new_v4();
    let s2 = Uuid::new_v4();
    let steps = vec![
        make_step(s1, "p1", Some("v1"), 0),
        make_step(s2, "p2", Some("v2"), 1),
    ];
    let edges = vec![make_edge(s1, s2)];

    let sorted = topological_sort(&steps, &edges).unwrap();
    assert_eq!(sorted[0], s1);
    assert_eq!(sorted[1], s2);
}

#[test]
fn topo_sort_cycle_detected() {
    let s1 = Uuid::new_v4();
    let s2 = Uuid::new_v4();
    let steps = vec![make_step(s1, "p", None, 0), make_step(s2, "p", None, 1)];
    let edges = vec![make_edge(s1, s2), make_edge(s2, s1)];

    assert!(topological_sort(&steps, &edges).is_err());
}

#[test]
fn resolve_variables_basic() {
    let mut outputs = HashMap::new();
    outputs.insert("name".to_string(), JsonValue::String("Alice".to_string()));

    let result = resolve_variables("Hello {name}!", &outputs, &HashMap::new());
    assert_eq!(result, "Hello Alice!");
}

#[test]
fn resolve_variables_dot_path() {
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
fn resolve_variables_unresolved_left_as_is() {
    let result = resolve_variables("Hello {unknown}!", &HashMap::new(), &HashMap::new());
    assert_eq!(result, "Hello {unknown}!");
}

#[test]
fn resolve_for_each_array_basic() {
    let mut outputs = HashMap::new();
    outputs.insert(
        "items".to_string(),
        serde_json::json!([{"name": "a"}, {"name": "b"}]),
    );

    let arr = resolve_for_each_array("items", &outputs, &HashMap::new()).unwrap();
    assert_eq!(arr.len(), 2);
}

#[test]
fn resolve_for_each_array_nested() {
    let mut outputs = HashMap::new();
    outputs.insert(
        "result".to_string(),
        serde_json::json!({"data": {"items": [1, 2, 3]}}),
    );

    let arr = resolve_for_each_array("result.data.items", &outputs, &HashMap::new()).unwrap();
    assert_eq!(arr.len(), 3);
}
