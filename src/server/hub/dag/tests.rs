//! Tests for DAG module

use super::{resolve_for_each_array, resolve_variables, topological_sort};
use crate::db::{WorkflowStepEdgeRow, WorkflowStepRow};
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
        reasoning_trace: false,
        verification_agent_ids: None,
        position_x: None,
        position_y: None,
        name: None,
        system_prompt_suffix: None,
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
        reasoning_trace: false,
        verification_agent_ids: None,
        position_x: None,
        position_y: None,
        name: None,
        system_prompt_suffix: None,
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

use super::StepOutput;
use super::{aggregate_subtask_outputs, topo_sort_subtasks};
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
use super::resolve_dot_path;
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

// =========================================================================
// Integration Tests: execute_workflow_via_engine
// =========================================================================
//
// These tests exercise the full DAG execution pipeline end-to-end using mock
// LLM providers and mock repositories. No Postgres required.

use super::execute_workflow_via_engine;
use super::WorkflowExecutionContext;
use crate::db::traits::{
    MockAgentExecutionRepo, MockServerRepo, MockTokenLedgerRepo, MockWorkflowRepo,
};
use crate::db::{AgentExecutionRow, AgentRow, ExecutionMessageRow, TokenLedgerRow};
use crate::llm::{
    LLMError, LLMProvider, LLMRequest, LLMResponse, StopReason, StreamChunk, TokenUsage,
};
use crate::server::hub::engine::ExecutionEngine;
use crate::server::hub::error::HubError;
use crate::server::state::test_helpers::MockReposBuilder;
use crate::server::state::AppStateBuilder;
use crate::types::AppConfig;
use async_trait::async_trait;
use chrono::Utc;
use futures::Stream;
use std::pin::Pin;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

// ---------------------------------------------------------------------------
// Mock LLM Providers
// ---------------------------------------------------------------------------

/// Returns the same response on every call.
struct FixedProvider {
    response: LLMResponse,
}

#[async_trait]
impl LLMProvider for FixedProvider {
    async fn send_message(&self, _req: LLMRequest) -> Result<LLMResponse, LLMError> {
        Ok(self.response.clone())
    }
    async fn send_message_stream(
        &self,
        _req: LLMRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, LLMError>> + Send>>, LLMError> {
        Err(LLMError::StreamError("not implemented".into()))
    }
    fn provider_name(&self) -> &'static str {
        "fixed"
    }
    fn model_id(&self) -> &str {
        "test-model"
    }
}

/// Returns different responses on sequential calls. Wraps to the last response
/// if more calls are made than responses available.
struct SequentialProvider {
    responses: Vec<LLMResponse>,
    call_count: AtomicU32,
}

#[async_trait]
impl LLMProvider for SequentialProvider {
    async fn send_message(&self, _req: LLMRequest) -> Result<LLMResponse, LLMError> {
        let n = self.call_count.fetch_add(1, Ordering::SeqCst) as usize;
        let idx = n.min(self.responses.len() - 1);
        Ok(self.responses[idx].clone())
    }
    async fn send_message_stream(
        &self,
        _req: LLMRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, LLMError>> + Send>>, LLMError> {
        Err(LLMError::StreamError("not implemented".into()))
    }
    fn provider_name(&self) -> &'static str {
        "sequential"
    }
    fn model_id(&self) -> &str {
        "test-model"
    }
}

/// Returns a valid response but cancels a token on the first call.
struct CancellingProvider {
    response: LLMResponse,
    token: CancellationToken,
    call_count: AtomicU32,
}

#[async_trait]
impl LLMProvider for CancellingProvider {
    async fn send_message(&self, _req: LLMRequest) -> Result<LLMResponse, LLMError> {
        let n = self.call_count.fetch_add(1, Ordering::SeqCst);
        if n == 0 {
            self.token.cancel();
        }
        Ok(self.response.clone())
    }
    async fn send_message_stream(
        &self,
        _req: LLMRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, LLMError>> + Send>>, LLMError> {
        Err(LLMError::StreamError("not implemented".into()))
    }
    fn provider_name(&self) -> &'static str {
        "cancelling"
    }
    fn model_id(&self) -> &str {
        "test-model"
    }
}

// ---------------------------------------------------------------------------
// Dummy Row Factories
// ---------------------------------------------------------------------------

fn dummy_ae_row() -> AgentExecutionRow {
    AgentExecutionRow {
        id: Uuid::new_v4(),
        agent_id: Uuid::new_v4(),
        workflow_step_id: None,
        workflow_execution_id: None,
        is_interactive: false,
        parent_agent_execution_id: None,
        system_prompt_rendered: String::new(),
        input: String::new(),
        output: None,
        structured_output: None,
        selected_mode_id: None,
        room_session_id: None,
        speaker_order: None,
        status: "running".into(),
        started_at: Utc::now(),
        completed_at: None,
        routing_analysis: None,
        selected_routing_document_id: None,
        is_exemplary: false,
    }
}

fn dummy_msg_row() -> ExecutionMessageRow {
    ExecutionMessageRow {
        id: Uuid::new_v4(),
        agent_execution_id: Uuid::new_v4(),
        role: "system".into(),
        content: String::new(),
        tool_call_id: None,
        input_tokens: 0,
        output_tokens: 0,
        created_at: Utc::now(),
    }
}

fn dummy_tl_row() -> TokenLedgerRow {
    TokenLedgerRow {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        agent_execution_id: None,
        model_id: "test-model".into(),
        input_tokens: 0,
        output_tokens: 0,
        cost_usd: 0.0,
        created_at: Utc::now(),
    }
}

// ---------------------------------------------------------------------------
// Test Helpers
// ---------------------------------------------------------------------------

fn make_test_agent(id: Uuid) -> AgentRow {
    AgentRow {
        id,
        user_id: None,
        tier: None,
        name: "Test Agent".into(),
        system_prompt: "You are a test agent.".into(),
        persona_style: None,
        model_provider: "anthropic".into(),
        model_id: "claude-sonnet-4-20250514".into(),
        model_max_tokens: 4096,
        model_temperature: 0.7,
        status: Some("active".into()),
        router_mode: None,
        router_id: None,
        output_schema_id: None,
        version: 1,
    }
}

fn make_integration_step(
    id: Uuid,
    agent_id: Uuid,
    prompt: &str,
    var_name: Option<&str>,
    order: i32,
) -> WorkflowStepRow {
    WorkflowStepRow {
        id,
        workflow_id: Uuid::new_v4(),
        agent_id,
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
        display_order: order,
        version: 1,
        reasoning_trace: false,
        verification_agent_ids: None,
        position_x: None,
        position_y: None,
        name: None,
        system_prompt_suffix: None,
    }
}

fn make_for_each_integration_step(
    id: Uuid,
    agent_id: Uuid,
    for_each_ref: &str,
    var_name: Option<&str>,
    order: i32,
) -> WorkflowStepRow {
    WorkflowStepRow {
        id,
        workflow_id: Uuid::new_v4(),
        agent_id,
        execution_mode: "for_each".into(),
        agent_execution_mode: Some("parallel".into()),
        for_each_ref: Some(for_each_ref.into()),
        prompt_template_id: None,
        prompt_template: "Process item".into(),
        output_schema_id: None,
        output_variable_name: var_name.map(|s| s.into()),
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
        name: None,
        system_prompt_suffix: None,
    }
}

fn make_ctx() -> WorkflowExecutionContext {
    WorkflowExecutionContext {
        stage_execution_id: Uuid::new_v4(),
        run_id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        initial_input: "test input".into(),
        prior_outputs: HashMap::new(),
        execution_context: None,
        container_config: None,
        wg_client: None,
    }
}

fn make_llm_response(content: &str, input_tokens: u32, output_tokens: u32) -> LLMResponse {
    LLMResponse {
        content: content.into(),
        content_blocks: vec![],
        model: "test-model".into(),
        stop_reason: StopReason::EndTurn,
        usage: TokenUsage {
            input_tokens,
            output_tokens,
        },
    }
}

// ---------------------------------------------------------------------------
// Harness Builder
// ---------------------------------------------------------------------------

struct TestHarness {
    engine: ExecutionEngine,
    state: crate::server::state::AppState,
    _rx: tokio::sync::mpsc::Receiver<crate::server::state::ConsumerMessage>,
}

/// Build a test harness with a single agent. The provided LLM provider is
/// used for the ExecutionEngine. MockServerRepo is configured to return the
/// agent for any `get_persisted_agent` call matching the given `agent_id`.
fn build_test_harness(agent_id: Uuid, provider: Arc<dyn LLMProvider + Send + Sync>) -> TestHarness {
    let agent = make_test_agent(agent_id);

    // MockServerRepo
    let mut server_repo = MockServerRepo::new();
    let agent_clone = agent.clone();
    server_repo
        .expect_get_persisted_agent()
        .returning(move |id| {
            if id == agent_id {
                Ok(Some(agent_clone.clone()))
            } else {
                Ok(None)
            }
        });
    server_repo
        .expect_get_agent_tools()
        .returning(|_| Ok(vec![]));
    server_repo
        .expect_get_agent_guidances()
        .returning(|_, _| Ok(vec![]));
    server_repo
        .expect_get_agent_context()
        .returning(|_| Ok(vec![]));

    // MockWorkflowRepo
    let mut wf_repo = MockWorkflowRepo::new();
    wf_repo.expect_get_step_inputs().returning(|_| Ok(vec![]));
    wf_repo.expect_get_step_outputs().returning(|_| Ok(vec![]));
    wf_repo
        .expect_get_step_routing_rules()
        .returning(|_| Ok(vec![]));
    wf_repo
        .expect_list_step_documents()
        .returning(|_| Ok(vec![]));

    // MockAgentExecutionRepo
    let mut ae_repo = MockAgentExecutionRepo::new();
    ae_repo
        .expect_create_agent_execution()
        .returning(|_, _, _, _, _, _, _, _, _| Ok(dummy_ae_row()));
    ae_repo
        .expect_create_execution_message()
        .returning(|_, _, _, _, _, _| Ok(dummy_msg_row()));
    ae_repo
        .expect_update_agent_execution_status()
        .returning(|_, _, _, _| Ok(dummy_ae_row()));
    ae_repo
        .expect_list_exemplary_executions()
        .returning(|_, _, _| Ok(vec![]));

    // MockTokenLedgerRepo
    let mut tl_repo = MockTokenLedgerRepo::new();
    tl_repo
        .expect_insert_ledger_entry()
        .returning(|_, _, _, _, _, _| Ok(dummy_tl_row()));

    let repos = MockReposBuilder::new()
        .with_workflows(Arc::new(wf_repo))
        .with_agent_executions(Arc::new(ae_repo))
        .with_token_ledger(Arc::new(tl_repo))
        .build();

    let engine = ExecutionEngine::new(provider.clone());

    let (state, rx) = AppStateBuilder::new()
        .with_server_repo(Arc::new(server_repo))
        .with_repos(repos)
        .with_config(AppConfig::default())
        .with_provider(provider)
        .build_for_test();

    TestHarness {
        engine,
        state,
        _rx: rx,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn single_step_workflow_executes() {
    let agent_id = Uuid::new_v4();
    let step_id = Uuid::new_v4();
    let provider = Arc::new(FixedProvider {
        response: make_llm_response(r#"{"result":"hello"}"#, 10, 5),
    });
    let harness = build_test_harness(agent_id, provider);
    let ctx = make_ctx();

    let steps = vec![make_integration_step(
        step_id,
        agent_id,
        "Generate output",
        Some("output"),
        0,
    )];
    let edges = vec![];

    let result =
        execute_workflow_via_engine(&harness.engine, &harness.state, &ctx, &steps, &edges, None)
            .await;

    let result = result.unwrap();
    assert_eq!(result.total_input_tokens, 10);
    assert_eq!(result.total_output_tokens, 5);
    assert_eq!(result.outputs.len(), 1);
    // Outputs are keyed by step UUID (not variable name)
    assert!(result.outputs.contains_key(&step_id.to_string()));
}

#[tokio::test]
async fn two_step_linear_workflow() {
    let agent_id = Uuid::new_v4();
    let s1 = Uuid::new_v4();
    let s2 = Uuid::new_v4();
    let provider = Arc::new(FixedProvider {
        response: make_llm_response(r#"{"data":"test"}"#, 10, 5),
    });
    let harness = build_test_harness(agent_id, provider);
    let ctx = make_ctx();

    let steps = vec![
        make_integration_step(s1, agent_id, "Step one", Some("step1_out"), 0),
        make_integration_step(
            s2,
            agent_id,
            "Step two uses {step1_out}",
            Some("step2_out"),
            1,
        ),
    ];
    let edges = vec![make_edge(s1, s2)];

    let result =
        execute_workflow_via_engine(&harness.engine, &harness.state, &ctx, &steps, &edges, None)
            .await
            .unwrap();

    assert_eq!(result.outputs.len(), 2);
    // Outputs are keyed by step UUID
    assert!(result.outputs.contains_key(&s1.to_string()));
    assert!(result.outputs.contains_key(&s2.to_string()));
    assert_eq!(result.total_input_tokens, 20);
    assert_eq!(result.total_output_tokens, 10);
}

#[tokio::test]
async fn three_step_diamond_dag() {
    let agent_id = Uuid::new_v4();
    let s1 = Uuid::new_v4();
    let s2 = Uuid::new_v4();
    let s3 = Uuid::new_v4();
    let s4 = Uuid::new_v4();
    let provider = Arc::new(FixedProvider {
        response: make_llm_response(r#"{"ok":true}"#, 10, 5),
    });
    let harness = build_test_harness(agent_id, provider);
    let ctx = make_ctx();

    let steps = vec![
        make_integration_step(s1, agent_id, "Start", Some("start"), 0),
        make_integration_step(s2, agent_id, "Branch A", Some("branch_a"), 1),
        make_integration_step(s3, agent_id, "Branch B", Some("branch_b"), 2),
        make_integration_step(s4, agent_id, "Merge", Some("merged"), 3),
    ];
    let edges = vec![
        make_edge(s1, s2),
        make_edge(s1, s3),
        make_edge(s2, s4),
        make_edge(s3, s4),
    ];

    let result =
        execute_workflow_via_engine(&harness.engine, &harness.state, &ctx, &steps, &edges, None)
            .await
            .unwrap();

    assert_eq!(result.outputs.len(), 4);
    assert_eq!(result.total_input_tokens, 40);
    assert_eq!(result.total_output_tokens, 20);
}

#[tokio::test]
async fn dag_cycle_returns_error() {
    let agent_id = Uuid::new_v4();
    let s1 = Uuid::new_v4();
    let s2 = Uuid::new_v4();
    // Provider should never be called — cycle detected before execution
    let provider = Arc::new(FixedProvider {
        response: make_llm_response("unused", 0, 0),
    });
    let harness = build_test_harness(agent_id, provider);
    let ctx = make_ctx();

    let steps = vec![
        make_integration_step(s1, agent_id, "A", None, 0),
        make_integration_step(s2, agent_id, "B", None, 1),
    ];
    let edges = vec![make_edge(s1, s2), make_edge(s2, s1)];

    let result =
        execute_workflow_via_engine(&harness.engine, &harness.state, &ctx, &steps, &edges, None)
            .await;

    assert!(matches!(result, Err(HubError::DagCycle)));
}

#[tokio::test]
async fn missing_agent_returns_error() {
    let real_agent_id = Uuid::new_v4();
    let missing_agent_id = Uuid::new_v4();
    let step_id = Uuid::new_v4();
    let provider = Arc::new(FixedProvider {
        response: make_llm_response("unused", 0, 0),
    });
    // Harness is built for real_agent_id, but step references missing_agent_id
    let harness = build_test_harness(real_agent_id, provider);
    let ctx = make_ctx();

    let steps = vec![make_integration_step(
        step_id,
        missing_agent_id,
        "Prompt",
        None,
        0,
    )];
    let edges = vec![];

    let result =
        execute_workflow_via_engine(&harness.engine, &harness.state, &ctx, &steps, &edges, None)
            .await;

    assert!(matches!(
        result,
        Err(HubError::AgentNotFound {
            step_id: _,
            agent_id: _
        })
    ));
}

#[tokio::test]
async fn cancellation_before_execution() {
    let agent_id = Uuid::new_v4();
    let step_id = Uuid::new_v4();
    let provider = Arc::new(FixedProvider {
        response: make_llm_response("unused", 0, 0),
    });
    let harness = build_test_harness(agent_id, provider);
    let ctx = make_ctx();

    let steps = vec![make_integration_step(step_id, agent_id, "Prompt", None, 0)];
    let edges = vec![];

    let token = CancellationToken::new();
    token.cancel();

    let result = execute_workflow_via_engine(
        &harness.engine,
        &harness.state,
        &ctx,
        &steps,
        &edges,
        Some(&token),
    )
    .await;

    assert!(matches!(result, Err(HubError::Cancelled)));
}

#[tokio::test]
async fn cancellation_between_steps() {
    let agent_id = Uuid::new_v4();
    let s1 = Uuid::new_v4();
    let s2 = Uuid::new_v4();
    let token = CancellationToken::new();

    let provider = Arc::new(CancellingProvider {
        response: make_llm_response(r#"{"done":true}"#, 10, 5),
        token: token.clone(),
        call_count: AtomicU32::new(0),
    });
    let harness = build_test_harness(agent_id, provider);
    let ctx = make_ctx();

    let steps = vec![
        make_integration_step(s1, agent_id, "Step 1", Some("s1_out"), 0),
        make_integration_step(s2, agent_id, "Step 2", Some("s2_out"), 1),
    ];
    let edges = vec![make_edge(s1, s2)];

    let result = execute_workflow_via_engine(
        &harness.engine,
        &harness.state,
        &ctx,
        &steps,
        &edges,
        Some(&token),
    )
    .await;

    assert!(matches!(result, Err(HubError::Cancelled)));
}

#[tokio::test]
async fn for_each_step_iterates_array() {
    let agent_id = Uuid::new_v4();
    let s1 = Uuid::new_v4();
    let s2 = Uuid::new_v4();

    let provider = Arc::new(SequentialProvider {
        responses: vec![
            // Step 1: returns a JSON array
            make_llm_response(r#"[{"item":"a"},{"item":"b"},{"item":"c"}]"#, 10, 5),
            // Step 2 iterations: each returns a simple object
            make_llm_response(r#"{"processed":"ok"}"#, 8, 4),
        ],
        call_count: AtomicU32::new(0),
    });
    let harness = build_test_harness(agent_id, provider);
    let ctx = make_ctx();

    let steps = vec![
        make_integration_step(s1, agent_id, "Generate items", Some("items"), 0),
        make_for_each_integration_step(s2, agent_id, "items", Some("processed"), 1),
    ];
    let edges = vec![make_edge(s1, s2)];

    let result =
        execute_workflow_via_engine(&harness.engine, &harness.state, &ctx, &steps, &edges, None)
            .await
            .unwrap();

    assert_eq!(result.outputs.len(), 2);
    // Outputs are keyed by step UUID
    assert!(result.outputs.contains_key(&s1.to_string()));
    assert!(result.outputs.contains_key(&s2.to_string()));
    // 1 call for step 1 + 3 calls for step 2 (3 items)
    // Step 1: 10 input, 5 output
    // Step 2: 3 * 8 input = 24, 3 * 4 output = 12
    assert_eq!(result.total_input_tokens, 34);
    assert_eq!(result.total_output_tokens, 17);
}

#[tokio::test]
async fn for_each_not_array_returns_error() {
    let agent_id = Uuid::new_v4();
    let s1 = Uuid::new_v4();
    let s2 = Uuid::new_v4();

    // Step 1 returns a plain string, not a JSON array
    let provider = Arc::new(FixedProvider {
        response: make_llm_response("just a string, not an array", 10, 5),
    });
    let harness = build_test_harness(agent_id, provider);
    let ctx = make_ctx();

    let steps = vec![
        make_integration_step(s1, agent_id, "Generate", Some("items"), 0),
        make_for_each_integration_step(s2, agent_id, "items", Some("processed"), 1),
    ];
    let edges = vec![make_edge(s1, s2)];

    let result =
        execute_workflow_via_engine(&harness.engine, &harness.state, &ctx, &steps, &edges, None)
            .await;

    assert!(matches!(result, Err(HubError::ForEachNotArray { .. })));
}
