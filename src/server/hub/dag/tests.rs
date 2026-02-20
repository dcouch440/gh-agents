#[cfg(test)]
mod tests {
    //! Integration tests for the DAG executor.
    //!
    //! Unit tests for specific modules live in colocated test files:
    //! - resolve_output_key / to_snake_case → `dag_state/tests.rs`

    use super::super::{resolve_variables, topological_sort};
    use crate::db::fixtures::fixtures::*;
    use crate::db::WorkflowStepRow;
    use serde_json::Value as JsonValue;
    use std::collections::HashMap;
    use uuid::Uuid;

    // =========================================================================
    // Topological Sort
    // =========================================================================

    #[test]
    fn topo_sort_linear() {
        let s1 = Uuid::new_v4();
        let s2 = Uuid::new_v4();
        let steps = vec![
            step_with(s1, "p1", Some("v1"), 0),
            step_with(s2, "p2", Some("v2"), 1),
        ];
        let edges = vec![edge(s1, s2)];

        let sorted = topological_sort(&steps, &edges).unwrap();
        assert_eq!(sorted[0], s1);
        assert_eq!(sorted[1], s2);
    }

    #[test]
    fn topo_sort_cycle_detected() {
        let s1 = Uuid::new_v4();
        let s2 = Uuid::new_v4();
        let steps = vec![step_with(s1, "p", None, 0), step_with(s2, "p", None, 1)];
        let edges = vec![edge(s1, s2), edge(s2, s1)];

        assert!(topological_sort(&steps, &edges).is_err());
    }

    // =========================================================================
    // Variable Resolution
    // =========================================================================

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

    // =========================================================================
    // Integration Tests: execute_workflow_via_engine
    // =========================================================================
    //
    // These tests exercise the full DAG execution pipeline end-to-end using mock
    // LLM providers and mock repositories. No Postgres required.

    use super::super::execute_workflow_via_engine;
    use super::super::WorkflowExecutionContext;
    use crate::db::traits::{
        MockAgentExecutionRepo, MockAgentRepo, MockContentVersionRepo, MockTokenLedgerRepo,
        MockToolRepo, MockWorkflowRepo,
    };
    use crate::db::{ContentVersionRow, RunSnapshotRow};
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

    /// Convert an LLMResponse into a stream of chunks the StreamAccumulator can reconstruct.
    fn response_to_stream(
        resp: &LLMResponse,
    ) -> Pin<Box<dyn Stream<Item = Result<StreamChunk, LLMError>> + Send>> {
        let mut chunks: Vec<Result<StreamChunk, LLMError>> = vec![
            Ok(StreamChunk::MessageStart {
                model: resp.model.clone(),
                input_tokens: resp.usage.input_tokens,
            }),
            Ok(StreamChunk::ContentBlockStart { index: 0 }),
            Ok(StreamChunk::ContentDelta {
                text: resp.content.clone(),
                index: 0,
            }),
            Ok(StreamChunk::ContentBlockStop { index: 0 }),
        ];
        // Emit tool use blocks if present
        for block in &resp.content_blocks {
            if let crate::llm::ContentBlock::ToolUse { id, name, input } = block {
                chunks.push(Ok(StreamChunk::ToolUseStart {
                    index: chunks.len(),
                    id: id.clone(),
                    name: name.clone(),
                }));
                chunks.push(Ok(StreamChunk::InputJsonDelta {
                    index: chunks.len(),
                    partial_json: input.to_string(),
                }));
                chunks.push(Ok(StreamChunk::ContentBlockStop {
                    index: chunks.len(),
                }));
            }
        }
        chunks.push(Ok(StreamChunk::MessageDelta {
            stop_reason: Some(resp.stop_reason),
            output_tokens: Some(resp.usage.output_tokens),
        }));
        chunks.push(Ok(StreamChunk::MessageStop));
        Box::pin(futures::stream::iter(chunks))
    }

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
        ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, LLMError>> + Send>>, LLMError>
        {
            Ok(response_to_stream(&self.response))
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
        ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, LLMError>> + Send>>, LLMError>
        {
            let n = self.call_count.fetch_add(1, Ordering::SeqCst) as usize;
            let idx = n.min(self.responses.len() - 1);
            Ok(response_to_stream(&self.responses[idx]))
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
        ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, LLMError>> + Send>>, LLMError>
        {
            let n = self.call_count.fetch_add(1, Ordering::SeqCst);
            if n == 0 {
                self.token.cancel();
            }
            Ok(response_to_stream(&self.response))
        }
        fn provider_name(&self) -> &'static str {
            "cancelling"
        }
        fn model_id(&self) -> &str {
            "test-model"
        }
    }

    // ---------------------------------------------------------------------------
    // Test Helpers
    // ---------------------------------------------------------------------------

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
            agent_id: Some(agent_id),
            prompt_template: prompt.into(),
            output_variable_name: var_name.map(|s| s.into()),
            display_order: order,
            ..Default::default()
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
            snapshot: None,
            parent_context: None,
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
    /// used for the ExecutionEngine. MockAgentRepo is configured to return the
    /// agent for any `get_persisted_agent` call matching the given `agent_id`.
    fn build_test_harness(
        agent_id: Uuid,
        provider: Arc<dyn LLMProvider + Send + Sync>,
    ) -> TestHarness {
        let agent = agent(agent_id);

        // MockAgentRepo
        let mut agent_repo = MockAgentRepo::new();
        let agent_clone = agent.clone();
        agent_repo
            .expect_get_persisted_agent()
            .returning(move |id| {
                if id == agent_id {
                    Ok(Some(agent_clone.clone()))
                } else {
                    Ok(None)
                }
            });
        agent_repo
            .expect_get_agent_guidances()
            .returning(|_, _| Ok(vec![]));
        agent_repo
            .expect_get_agent_context()
            .returning(|_| Ok(vec![]));

        // MockToolRepo
        let mut tool_repo = MockToolRepo::new();
        tool_repo.expect_get_agent_tools().returning(|_| Ok(vec![]));

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
            .returning(|_| Ok(agent_execution()));
        ae_repo
            .expect_create_execution_message()
            .returning(|_, _, _, _, _, _| Ok(execution_message(Uuid::new_v4())));
        ae_repo
            .expect_update_agent_execution_status()
            .returning(|_, _, _, _| Ok(agent_execution()));
        ae_repo
            .expect_list_exemplary_executions()
            .returning(|_, _, _| Ok(vec![]));

        // MockTokenLedgerRepo
        let mut tl_repo = MockTokenLedgerRepo::new();
        tl_repo
            .expect_insert_ledger_entry()
            .returning(|_, _, _, _, _, _| Ok(token_ledger(Uuid::new_v4())));

        // MockContentVersionRepo — permissive (snapshotting is fire-and-forget)
        let mut cv_repo = MockContentVersionRepo::new();
        cv_repo.expect_find_or_create_version().returning(
            |source_id, content_type, content_hash, _content| {
                Ok(ContentVersionRow {
                    id: Uuid::new_v4(),
                    source_id,
                    content_type: content_type.to_string(),
                    content_hash: content_hash.to_string(),
                    content: String::new(),
                    version_number: 1,
                    byte_size: 0,
                    created_at: Utc::now(),
                })
            },
        );
        cv_repo
            .expect_get_latest_envelope_for_step()
            .returning(|_| Ok(None));
        cv_repo.expect_create_run_snapshot().returning(
            |run_id, step_id, content_type, role, cv_id, source_id| {
                Ok(RunSnapshotRow {
                    id: Uuid::new_v4(),
                    run_id,
                    step_id,
                    content_type: content_type.to_string(),
                    role: role.to_string(),
                    content_version_id: cv_id,
                    source_id,
                    created_at: Utc::now(),
                })
            },
        );

        let repos = MockReposBuilder::new()
            .with_agents(Arc::new(agent_repo))
            .with_tools(Arc::new(tool_repo))
            .with_workflows(Arc::new(wf_repo))
            .with_agent_executions(Arc::new(ae_repo))
            .with_token_ledger(Arc::new(tl_repo))
            .with_content_versions(Arc::new(cv_repo))
            .build();

        let engine = ExecutionEngine::new(provider.clone());

        let (state, rx) = AppStateBuilder::new()
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

        let result = execute_workflow_via_engine(
            &harness.engine,
            &harness.state,
            &ctx,
            &steps,
            &edges,
            None,
        )
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
        let edges = vec![edge(s1, s2)];

        let result = execute_workflow_via_engine(
            &harness.engine,
            &harness.state,
            &ctx,
            &steps,
            &edges,
            None,
        )
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
        let edges = vec![edge(s1, s2), edge(s1, s3), edge(s2, s4), edge(s3, s4)];

        let result = execute_workflow_via_engine(
            &harness.engine,
            &harness.state,
            &ctx,
            &steps,
            &edges,
            None,
        )
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
        let edges = vec![edge(s1, s2), edge(s2, s1)];

        let result = execute_workflow_via_engine(
            &harness.engine,
            &harness.state,
            &ctx,
            &steps,
            &edges,
            None,
        )
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

        let result = execute_workflow_via_engine(
            &harness.engine,
            &harness.state,
            &ctx,
            &steps,
            &edges,
            None,
        )
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
        let edges = vec![edge(s1, s2)];

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

    // ---------------------------------------------------------------------------
    // Pinned Step Tests
    // ---------------------------------------------------------------------------

    fn make_context_step(
        id: Uuid,
        prompt: &str,
        var_name: Option<&str>,
        order: i32,
    ) -> WorkflowStepRow {
        WorkflowStepRow {
            execution_mode: "context".into(),
            agent_id: None,
            ..make_integration_step(id, Uuid::new_v4(), prompt, var_name, order)
        }
    }

    /// Build a test harness where `get_latest_envelope_for_step` returns
    /// a specific envelope JSON for any step.
    fn build_pinned_harness(
        agent_id: Uuid,
        provider: Arc<dyn LLMProvider + Send + Sync>,
        envelope_json: Option<String>,
    ) -> TestHarness {
        let agent = agent(agent_id);

        let mut agent_repo = MockAgentRepo::new();
        let agent_clone = agent.clone();
        agent_repo
            .expect_get_persisted_agent()
            .returning(move |id| {
                if id == agent_id {
                    Ok(Some(agent_clone.clone()))
                } else {
                    Ok(None)
                }
            });
        agent_repo
            .expect_get_agent_guidances()
            .returning(|_, _| Ok(vec![]));
        agent_repo
            .expect_get_agent_context()
            .returning(|_| Ok(vec![]));

        let mut tool_repo = MockToolRepo::new();
        tool_repo.expect_get_agent_tools().returning(|_| Ok(vec![]));

        let mut wf_repo = MockWorkflowRepo::new();
        wf_repo.expect_get_step_inputs().returning(|_| Ok(vec![]));
        wf_repo.expect_get_step_outputs().returning(|_| Ok(vec![]));
        wf_repo
            .expect_get_step_routing_rules()
            .returning(|_| Ok(vec![]));
        wf_repo
            .expect_list_step_documents()
            .returning(|_| Ok(vec![]));

        let mut ae_repo = MockAgentExecutionRepo::new();
        ae_repo
            .expect_create_agent_execution()
            .returning(|_| Ok(agent_execution()));
        ae_repo
            .expect_create_execution_message()
            .returning(|_, _, _, _, _, _| Ok(execution_message(Uuid::new_v4())));
        ae_repo
            .expect_update_agent_execution_status()
            .returning(|_, _, _, _| Ok(agent_execution()));
        ae_repo
            .expect_list_exemplary_executions()
            .returning(|_, _, _| Ok(vec![]));

        let mut tl_repo = MockTokenLedgerRepo::new();
        tl_repo
            .expect_insert_ledger_entry()
            .returning(|_, _, _, _, _, _| Ok(token_ledger(Uuid::new_v4())));

        let mut cv_repo = MockContentVersionRepo::new();
        cv_repo.expect_find_or_create_version().returning(
            |source_id, content_type, content_hash, _content| {
                Ok(ContentVersionRow {
                    id: Uuid::new_v4(),
                    source_id,
                    content_type: content_type.to_string(),
                    content_hash: content_hash.to_string(),
                    content: String::new(),
                    version_number: 1,
                    byte_size: 0,
                    created_at: Utc::now(),
                })
            },
        );
        cv_repo
            .expect_get_latest_envelope_for_step()
            .returning(move |_| Ok(envelope_json.clone()));
        cv_repo.expect_create_run_snapshot().returning(
            |run_id, step_id, content_type, role, cv_id, source_id| {
                Ok(RunSnapshotRow {
                    id: Uuid::new_v4(),
                    run_id,
                    step_id,
                    content_type: content_type.to_string(),
                    role: role.to_string(),
                    content_version_id: cv_id,
                    source_id,
                    created_at: Utc::now(),
                })
            },
        );

        let repos = MockReposBuilder::new()
            .with_agents(Arc::new(agent_repo))
            .with_tools(Arc::new(tool_repo))
            .with_workflows(Arc::new(wf_repo))
            .with_agent_executions(Arc::new(ae_repo))
            .with_token_ledger(Arc::new(tl_repo))
            .with_content_versions(Arc::new(cv_repo))
            .build();

        let engine = ExecutionEngine::new(provider.clone());

        let (state, rx) = AppStateBuilder::new()
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

    #[tokio::test]
    async fn pinned_context_step_replays_output() {
        let agent_id = Uuid::new_v4();
        let s1 = Uuid::new_v4();

        // Provider should never be called — pinned context is pure pass-through
        let provider = Arc::new(FixedProvider {
            response: make_llm_response("should not be called", 999, 999),
        });
        let harness = build_test_harness(agent_id, provider);
        let ctx = make_ctx();

        let mut step = make_context_step(s1, "pinned context data", Some("ctx_out"), 0);
        step.pinned = true;

        let steps = vec![step];
        let edges = vec![];

        let result = execute_workflow_via_engine(
            &harness.engine,
            &harness.state,
            &ctx,
            &steps,
            &edges,
            None,
        )
        .await
        .unwrap();

        // Context step pass-through: 0 tokens (pinned or not, context steps don't use LLM)
        assert_eq!(result.total_input_tokens, 0);
        assert_eq!(result.total_output_tokens, 0);
        assert_eq!(result.outputs.len(), 1);
    }

    #[tokio::test]
    async fn pinned_single_step_replays_last_envelope() {
        let agent_id = Uuid::new_v4();
        let s1 = Uuid::new_v4();

        // Prepare a stored envelope that the pinned step should replay
        let stored_envelope = serde_json::json!({
            "status": "success",
            "data": {"result": "previously computed"},
            "metadata": {
                "execution_id": Uuid::new_v4().to_string(),
                "execution_time_ms": 500,
                "tokens_in": 100,
                "tokens_out": 50,
                "cost_usd": 0.001
            },
            "error": null
        });
        let envelope_json = serde_json::to_string(&stored_envelope).unwrap();

        // Provider should NOT be called — pinned step replays from stored envelope
        let provider = Arc::new(FixedProvider {
            response: make_llm_response("should not be called", 999, 999),
        });
        let harness = build_pinned_harness(agent_id, provider, Some(envelope_json));
        let ctx = make_ctx();

        let mut step = make_integration_step(s1, agent_id, "Compute something", Some("output"), 0);
        step.pinned = true;

        let steps = vec![step];
        let edges = vec![];

        let result = execute_workflow_via_engine(
            &harness.engine,
            &harness.state,
            &ctx,
            &steps,
            &edges,
            None,
        )
        .await
        .unwrap();

        // Pinned replay: 0 tokens charged
        assert_eq!(result.total_input_tokens, 0);
        assert_eq!(result.total_output_tokens, 0);
        assert_eq!(result.outputs.len(), 1);
    }

    #[tokio::test]
    async fn pinned_single_step_no_prior_output_falls_through() {
        let agent_id = Uuid::new_v4();
        let s1 = Uuid::new_v4();

        // No stored envelope — pinned step should fall through to normal execution
        let provider = Arc::new(FixedProvider {
            response: make_llm_response(r#"{"fresh":"result"}"#, 15, 8),
        });
        let harness = build_pinned_harness(agent_id, provider, None);
        let ctx = make_ctx();

        let mut step = make_integration_step(s1, agent_id, "Compute something", Some("output"), 0);
        step.pinned = true;

        let steps = vec![step];
        let edges = vec![];

        let result = execute_workflow_via_engine(
            &harness.engine,
            &harness.state,
            &ctx,
            &steps,
            &edges,
            None,
        )
        .await
        .unwrap();

        // Falls through to normal execution — tokens charged
        assert_eq!(result.total_input_tokens, 15);
        assert_eq!(result.total_output_tokens, 8);
        assert_eq!(result.outputs.len(), 1);
    }

    #[tokio::test]
    async fn pinned_step_in_chain_skips_downstream() {
        // A → B(pinned) → C — B replays, C still executes against B's replayed output
        let agent_id = Uuid::new_v4();
        let s_a = Uuid::new_v4();
        let s_b = Uuid::new_v4();
        let s_c = Uuid::new_v4();

        let stored_envelope = serde_json::json!({
            "status": "success",
            "data": {"cached": "value"},
            "metadata": {
                "execution_id": Uuid::new_v4().to_string(),
                "execution_time_ms": 100,
                "tokens_in": 50,
                "tokens_out": 25,
                "cost_usd": 0.0005
            },
            "error": null
        });
        let envelope_json = serde_json::to_string(&stored_envelope).unwrap();

        let provider = Arc::new(FixedProvider {
            response: make_llm_response(r#"{"result":"computed"}"#, 10, 5),
        });
        let harness = build_pinned_harness(agent_id, provider, Some(envelope_json));
        let ctx = make_ctx();

        let s_a_step = make_integration_step(s_a, agent_id, "Step A", Some("a_out"), 0);
        let mut s_b_step = make_integration_step(s_b, agent_id, "Step B", Some("b_out"), 1);
        s_b_step.pinned = true;
        let s_c_step =
            make_integration_step(s_c, agent_id, "Step C uses {b_out}", Some("c_out"), 2);

        let steps = vec![s_a_step, s_b_step, s_c_step];
        let edges = vec![edge(s_a, s_b), edge(s_b, s_c)];

        let result = execute_workflow_via_engine(
            &harness.engine,
            &harness.state,
            &ctx,
            &steps,
            &edges,
            None,
        )
        .await
        .unwrap();

        // A: dead-path skipped (only child B is pinned), B: pinned (0), C: 10 in + 5 out
        assert_eq!(result.total_input_tokens, 10);
        assert_eq!(result.total_output_tokens, 5);
        assert_eq!(result.outputs.len(), 3);
    }
}
