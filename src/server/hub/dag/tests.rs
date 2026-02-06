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

// =========================================================================
// Phase 6B: Chain Detection Tests
// =========================================================================

use super::detect_for_each_chains;

fn make_for_each_step(id: Uuid, var_name: Option<&str>, display_order: i32) -> WorkflowStepRow {
    WorkflowStepRow {
        id,
        workflow_id: Uuid::new_v4(),
        agent_id: Uuid::new_v4(),
        execution_mode: "for_each".into(),
        agent_execution_mode: Some("parallel".into()),
        for_each_ref: Some("items".into()),
        prompt_template_id: None,
        prompt_template: "Process item".into(),
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

#[test]
fn detect_chains_two_for_each() {
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let c = Uuid::new_v4();

    let steps = vec![
        make_for_each_step(a, Some("specialists"), 0),
        make_for_each_step(b, Some("reviewers"), 1),
        make_step(c, "Synthesize", Some("final"), 2),
    ];
    let edges = vec![make_edge(a, b), make_edge(b, c)];

    let chains = detect_for_each_chains(&steps, &edges);
    assert_eq!(chains.len(), 1);
    assert_eq!(chains[0].step_ids, vec![a, b]);
}

#[test]
fn detect_chains_three_for_each() {
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let c = Uuid::new_v4();
    let d = Uuid::new_v4();

    let steps = vec![
        make_for_each_step(a, Some("stage1"), 0),
        make_for_each_step(b, Some("stage2"), 1),
        make_for_each_step(c, Some("stage3"), 2),
        make_step(d, "Synthesize", Some("final"), 3),
    ];
    let edges = vec![make_edge(a, b), make_edge(b, c), make_edge(c, d)];

    let chains = detect_for_each_chains(&steps, &edges);
    assert_eq!(chains.len(), 1);
    assert_eq!(chains[0].step_ids, vec![a, b, c]);
}

#[test]
fn detect_chains_none_single_for_each() {
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();

    let steps = vec![
        make_for_each_step(a, Some("items"), 0),
        make_step(b, "Done", Some("result"), 1),
    ];
    let edges = vec![make_edge(a, b)];

    let chains = detect_for_each_chains(&steps, &edges);
    assert!(chains.is_empty());
}

#[test]
fn detect_chains_broken_by_single() {
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let c = Uuid::new_v4();

    let steps = vec![
        make_for_each_step(a, Some("stage1"), 0),
        make_step(b, "Middle", Some("mid"), 1),
        make_for_each_step(c, Some("stage2"), 2),
    ];
    let edges = vec![make_edge(a, b), make_edge(b, c)];

    let chains = detect_for_each_chains(&steps, &edges);
    assert!(chains.is_empty());
}

#[test]
fn detect_chains_fan_out_breaks() {
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let c = Uuid::new_v4();

    let steps = vec![
        make_for_each_step(a, Some("source"), 0),
        make_for_each_step(b, Some("branch1"), 1),
        make_for_each_step(c, Some("branch2"), 2),
    ];
    // a fans out to both b and c
    let edges = vec![make_edge(a, b), make_edge(a, c)];

    let chains = detect_for_each_chains(&steps, &edges);
    // a has 2 for-each children, so no chain forms
    assert!(chains.is_empty());
}

#[test]
fn detect_chains_independent() {
    let a1 = Uuid::new_v4();
    let a2 = Uuid::new_v4();
    let b1 = Uuid::new_v4();
    let b2 = Uuid::new_v4();

    let steps = vec![
        make_for_each_step(a1, Some("chain1_s1"), 0),
        make_for_each_step(a2, Some("chain1_s2"), 1),
        make_for_each_step(b1, Some("chain2_s1"), 2),
        make_for_each_step(b2, Some("chain2_s2"), 3),
    ];
    let edges = vec![make_edge(a1, a2), make_edge(b1, b2)];

    let chains = detect_for_each_chains(&steps, &edges);
    assert_eq!(chains.len(), 2);

    // Both chains should have length 2
    for chain in &chains {
        assert_eq!(chain.step_ids.len(), 2);
    }
}

#[test]
fn detect_chains_fan_in_breaks() {
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let c = Uuid::new_v4();

    let steps = vec![
        make_for_each_step(a, Some("source1"), 0),
        make_for_each_step(b, Some("source2"), 1),
        make_for_each_step(c, Some("merged"), 2),
    ];
    // Both a and b feed into c (fan-in)
    let edges = vec![make_edge(a, c), make_edge(b, c)];

    let chains = detect_for_each_chains(&steps, &edges);
    // c has 2 parents, so no chain forms with c
    assert!(chains.is_empty());
}
