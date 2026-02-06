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

// =========================================================================
// Phase 7: Cavernous Routing Tests
// =========================================================================

use super::{aggregate_subtask_outputs, topo_sort_subtasks};
use crate::server::executors::dag::StepOutput;
use crate::types::Subtask;

fn make_subtask(id: &str, depends_on: Vec<&str>) -> Subtask {
    Subtask {
        id: id.into(),
        task_name: format!("Task {}", id),
        agent_id: Uuid::new_v4(),
        tools: vec![],
        prompt_template: format!("Do {}", id),
        depends_on: depends_on.into_iter().map(|s| s.into()).collect(),
        input_mapping: std::collections::HashMap::new(),
        output_schema: None,
    }
}

fn make_subtask_output(raw: &str) -> StepOutput {
    StepOutput {
        variable_name: String::new(),
        structured_output: serde_json::from_str(raw).ok(),
        raw_output: raw.into(),
    }
}

#[test]
fn topo_sort_subtasks_linear() {
    // A -> B -> C
    let subtasks = vec![
        make_subtask("a", vec![]),
        make_subtask("b", vec!["a"]),
        make_subtask("c", vec!["b"]),
    ];

    let layers = topo_sort_subtasks(&subtasks).unwrap();
    assert_eq!(layers.len(), 3);
    assert_eq!(layers[0].len(), 1);
    assert_eq!(layers[0][0].id, "a");
    assert_eq!(layers[1][0].id, "b");
    assert_eq!(layers[2][0].id, "c");
}

#[test]
fn topo_sort_subtasks_parallel() {
    // A and B independent, C depends on both
    let subtasks = vec![
        make_subtask("a", vec![]),
        make_subtask("b", vec![]),
        make_subtask("c", vec!["a", "b"]),
    ];

    let layers = topo_sort_subtasks(&subtasks).unwrap();
    assert_eq!(layers.len(), 2);
    assert_eq!(layers[0].len(), 2); // a and b in parallel
    let first_layer_ids: Vec<&str> = layers[0].iter().map(|s| s.id.as_str()).collect();
    assert!(first_layer_ids.contains(&"a"));
    assert!(first_layer_ids.contains(&"b"));
    assert_eq!(layers[1].len(), 1);
    assert_eq!(layers[1][0].id, "c");
}

#[test]
fn topo_sort_subtasks_cycle_detected() {
    // A depends on B, B depends on A
    let subtasks = vec![make_subtask("a", vec!["b"]), make_subtask("b", vec!["a"])];

    assert!(topo_sort_subtasks(&subtasks).is_err());
}

#[test]
fn topo_sort_subtasks_unknown_dep() {
    let subtasks = vec![make_subtask("a", vec!["nonexistent"])];

    assert!(topo_sort_subtasks(&subtasks).is_err());
}

#[test]
fn topo_sort_subtasks_empty() {
    let layers = topo_sort_subtasks(&[]).unwrap();
    assert!(layers.is_empty());
}

#[test]
fn aggregate_all_outputs_mode() {
    let mut results = HashMap::new();
    results.insert(
        "db_schema".into(),
        make_subtask_output(r#"{"tables": ["users", "posts"]}"#),
    );
    results.insert(
        "api".into(),
        make_subtask_output(r#"{"endpoints": ["/users", "/posts"]}"#),
    );

    let order = vec!["db_schema".into(), "api".into()];
    let aggregated = aggregate_subtask_outputs(&results, "all_outputs", &order);

    assert!(aggregated.is_object());
    let obj = aggregated.as_object().unwrap();
    assert!(obj.contains_key("db_schema"));
    assert!(obj.contains_key("api"));
    assert_eq!(obj["db_schema"]["tables"][0].as_str().unwrap(), "users");
}

#[test]
fn aggregate_final_output_mode() {
    let mut results = HashMap::new();
    results.insert("first".into(), make_subtask_output(r#"{"step": 1}"#));
    results.insert("last".into(), make_subtask_output(r#"{"step": 2}"#));

    let order = vec!["first".into(), "last".into()];
    let aggregated = aggregate_subtask_outputs(&results, "final_output", &order);

    assert_eq!(aggregated["step"].as_i64().unwrap(), 2);
}

#[test]
fn aggregate_merge_mode() {
    let mut results = HashMap::new();
    results.insert(
        "a".into(),
        make_subtask_output(r#"{"color": "red", "size": 10}"#),
    );
    results.insert(
        "b".into(),
        make_subtask_output(r#"{"shape": "circle", "size": 20}"#),
    );

    let order = vec!["a".into(), "b".into()];
    let aggregated = aggregate_subtask_outputs(&results, "merge", &order);

    let obj = aggregated.as_object().unwrap();
    assert_eq!(obj["color"].as_str().unwrap(), "red");
    assert_eq!(obj["shape"].as_str().unwrap(), "circle");
    // Later value wins for "size"
    assert_eq!(obj["size"].as_i64().unwrap(), 20);
}

#[test]
fn aggregate_final_output_skips_missing() {
    let mut results = HashMap::new();
    results.insert("first".into(), make_subtask_output(r#"{"data": "ok"}"#));
    // "second" is missing (simulating a failed subtask)

    let order = vec!["first".into(), "second".into()];
    let aggregated = aggregate_subtask_outputs(&results, "final_output", &order);

    // Falls back to "first" since "second" is missing
    assert_eq!(aggregated["data"].as_str().unwrap(), "ok");
}

// =========================================================================
// Room Composite Envelope Tests
// =========================================================================

use super::extract_room_outputs_from_speakers;
use crate::server::executors::dag::resolve_dot_path;
use crate::server::executors::room::SpeakerResult;

#[test]
fn room_composite_envelope_structure() {
    let agent_a = Uuid::new_v4();
    let agent_b = Uuid::new_v4();

    let speakers = vec![
        SpeakerResult {
            agent_id: agent_a,
            agent_name: "Architect".into(),
            content: r#"{"recommendation": "use microservices"}"#.into(),
            input_tokens: 100,
            output_tokens: 50,
            speaker_order: 0,
        },
        SpeakerResult {
            agent_id: agent_b,
            agent_name: "Reviewer".into(),
            content: "I agree with the approach.".into(),
            input_tokens: 80,
            output_tokens: 30,
            speaker_order: 1,
        },
    ];

    let (envelope_data, output) = extract_room_outputs_from_speakers(&speakers, Some("room_out"));

    // Verify output variable name
    assert_eq!(output.variable_name, "room_out");

    // Verify composite structure has per-agent keys
    let key_a = format!("agent:{}", agent_a);
    let key_b = format!("agent:{}", agent_b);

    // Agent A returned valid JSON — should be parsed as object
    let val_a = resolve_dot_path(&envelope_data, &key_a).unwrap();
    assert_eq!(val_a["recommendation"], "use microservices");

    // Agent B returned plain text — should be stored as string
    let val_b = resolve_dot_path(&envelope_data, &key_b).unwrap();
    assert_eq!(val_b.as_str().unwrap(), "I agree with the approach.");

    // Nested access works through the port system
    let nested_path = format!("{}.recommendation", key_a);
    let nested = resolve_dot_path(&envelope_data, &nested_path).unwrap();
    assert_eq!(nested, "use microservices");
}
