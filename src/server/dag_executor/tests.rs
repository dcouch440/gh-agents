//! Tests for DAG executor

use super::*;
use crate::db::{WorkflowStepEdgeRow, WorkflowStepRow};
use std::collections::HashMap;
use uuid::Uuid;

fn make_step(id: Uuid, workflow_id: Uuid, order: i32) -> WorkflowStepRow {
    WorkflowStepRow {
        id,
        workflow_id,
        agent_id: Uuid::new_v4(),
        execution_mode: "single".to_string(),
        agent_execution_mode: None,
        for_each_ref: None,
        prompt_template_id: None,
        prompt_template: String::new(),
        output_schema_id: None,
        output_variable_name: None,
        interactive_agent_id: None,
        for_each_label_field: None,
        display_order: order,
        version: 1,
        room_id: None,
    }
}

#[test]
fn topological_sort_linear() {
    let wf = Uuid::new_v4();
    let s1 = Uuid::new_v4();
    let s2 = Uuid::new_v4();
    let steps = vec![make_step(s1, wf, 0), make_step(s2, wf, 1)];
    let edges = vec![WorkflowStepEdgeRow { from_step_id: s1, to_step_id: s2 }];

    let result = topological_sort(&steps, &edges).unwrap();
    assert_eq!(result, vec![s1, s2]);
}

#[test]
fn topological_sort_detects_cycle() {
    let wf = Uuid::new_v4();
    let s1 = Uuid::new_v4();
    let s2 = Uuid::new_v4();
    let steps = vec![make_step(s1, wf, 0), make_step(s2, wf, 1)];
    let edges = vec![WorkflowStepEdgeRow { from_step_id: s1, to_step_id: s2 }, WorkflowStepEdgeRow { from_step_id: s2, to_step_id: s1 }];

    assert!(topological_sort(&steps, &edges).is_err());
}

#[test]
fn resolve_variables_simple() {
    let mut outputs = HashMap::new();
    outputs.insert("name".to_string(), JsonValue::String("Alice".to_string()));

    let result = resolve_variables("Hello {name}!", &outputs, &HashMap::new());
    assert_eq!(result, "Hello Alice!");
}

#[test]
fn resolve_variables_nested() {
    let mut outputs = HashMap::new();
    outputs.insert("user".to_string(), serde_json::json!({"name": "Bob", "age": 30}));

    let result = resolve_variables("Name: {user.name}, Age: {user.age}", &outputs, &HashMap::new());
    assert_eq!(result, "Name: Bob, Age: 30");
}

#[test]
fn resolve_variables_unresolved_left_as_is() {
    let result = resolve_variables("Hello {unknown}!", &HashMap::new(), &HashMap::new());
    assert_eq!(result, "Hello {unknown}!");
}
