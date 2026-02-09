#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    use async_trait::async_trait;
    use chrono::Utc;
    use futures::Stream;
    use std::pin::Pin;
    use uuid::Uuid;

    use crate::db::traits::{AgentExecutionRepo, ServerRepo, TokenLedgerRepo};
    use crate::db::{AgentExecutionRow, AgentRow, TokenLedgerRow};
    use crate::llm::{
        LLMError, LLMProvider, LLMRequest, LLMResponse, StopReason, StreamChunk, TokenUsage,
    };
    use crate::server::hub::engine::filters::{ExecutionFilter, FilterContext, ResponseAction};

    use super::super::DebateVerificationFilter;

    // =========================================================================
    // Test LLM Provider — returns canned responses
    // =========================================================================

    struct TestLLMProvider {
        responses: Vec<String>,
        call_count: AtomicU32,
    }

    impl TestLLMProvider {
        fn new(responses: Vec<String>) -> Self {
            Self {
                responses,
                call_count: AtomicU32::new(0),
            }
        }

        fn calls(&self) -> u32 {
            self.call_count.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl LLMProvider for TestLLMProvider {
        async fn send_message(&self, _request: LLMRequest) -> Result<LLMResponse, LLMError> {
            let idx = self.call_count.fetch_add(1, Ordering::SeqCst) as usize;
            let content = self
                .responses
                .get(idx)
                .cloned()
                .unwrap_or_else(|| r#"{"approved": true, "issues": []}"#.to_string());
            Ok(LLMResponse {
                content,
                content_blocks: vec![],
                model: "test-model".to_string(),
                stop_reason: StopReason::EndTurn,
                usage: TokenUsage {
                    input_tokens: 100,
                    output_tokens: 50,
                },
            })
        }

        async fn send_message_stream(
            &self,
            _request: LLMRequest,
        ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, LLMError>> + Send>>, LLMError>
        {
            Err(LLMError::ApiError {
                status: 501,
                message: "not implemented".into(),
            })
        }

        fn provider_name(&self) -> &'static str {
            "test"
        }

        fn model_id(&self) -> &str {
            "test-model"
        }
    }

    // =========================================================================
    // Slow LLM Provider — sleeps longer than the verification timeout
    // =========================================================================

    struct SlowLLMProvider;

    #[async_trait]
    impl LLMProvider for SlowLLMProvider {
        async fn send_message(&self, _request: LLMRequest) -> Result<LLMResponse, LLMError> {
            // Sleep longer than VERIFICATION_AGENT_TIMEOUT_SECS (60s).
            // The filter's tokio::time::timeout will fire first.
            tokio::time::sleep(std::time::Duration::from_secs(120)).await;
            Ok(LLMResponse {
                content: r#"{"approved": false, "issues": [{"severity": "high", "description": "too slow"}]}"#.to_string(),
                content_blocks: vec![],
                model: "test-model".to_string(),
                stop_reason: StopReason::EndTurn,
                usage: TokenUsage::default(),
            })
        }

        async fn send_message_stream(
            &self,
            _request: LLMRequest,
        ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, LLMError>> + Send>>, LLMError>
        {
            Err(LLMError::ApiError {
                status: 501,
                message: "not implemented".into(),
            })
        }

        fn provider_name(&self) -> &'static str {
            "test-slow"
        }

        fn model_id(&self) -> &str {
            "test-model"
        }
    }

    // =========================================================================
    // Helpers
    // =========================================================================

    fn make_agent(id: Uuid, name: &str) -> AgentRow {
        AgentRow {
            id,
            user_id: None,
            tier: None,
            name: name.to_string(),
            system_prompt: format!("You are {name}, an expert reviewer."),
            persona_style: None,
            model_provider: "anthropic".to_string(),
            model_id: "claude-3".to_string(),
            model_max_tokens: 4096,
            model_temperature: 0.0,
            status: None,
            router_mode: None,
            router_id: None,
            output_schema_id: None,
            version: 1,
        }
    }

    fn make_ae_row(agent_id: Uuid) -> AgentExecutionRow {
        AgentExecutionRow {
            id: Uuid::new_v4(),
            agent_id,
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
            status: "pending".to_string(),
            started_at: Utc::now(),
            completed_at: None,
            routing_analysis: None,
            selected_routing_document_id: None,
            is_exemplary: false,
        }
    }

    fn make_tl_row(user_id: Uuid) -> TokenLedgerRow {
        TokenLedgerRow {
            id: Uuid::new_v4(),
            user_id,
            agent_execution_id: None,
            model_id: "claude-3".to_string(),
            input_tokens: 100,
            output_tokens: 50,
            cost_usd: 0.001,
            created_at: Utc::now(),
        }
    }

    fn mock_response() -> LLMResponse {
        LLMResponse {
            content: "Some response from primary agent".to_string(),
            content_blocks: vec![],
            model: "claude-3".to_string(),
            stop_reason: StopReason::EndTurn,
            usage: TokenUsage::default(),
        }
    }

    fn mock_repo_with_agents(agents: Vec<AgentRow>) -> Arc<dyn ServerRepo> {
        let mut mock = crate::db::traits::MockServerRepo::new();
        mock.expect_get_persisted_agent()
            .returning(move |id| Ok(agents.iter().find(|a| a.id == id).cloned()));
        Arc::new(mock)
    }

    // =========================================================================
    // Tests — existing (updated for 5-param constructor)
    // =========================================================================

    #[tokio::test]
    async fn no_verification_agents_passthrough() {
        let provider: Arc<dyn LLMProvider> = Arc::new(TestLLMProvider::new(vec![]));
        let repo = mock_repo_with_agents(vec![]);
        let filter = DebateVerificationFilter::new(provider, repo, vec![], None, None);
        let filter: &dyn ExecutionFilter = &filter;

        let ctx = FilterContext::new("m", Uuid::new_v4());
        let response = mock_response();

        let action = filter.on_response(&ctx, &response).await.unwrap();
        assert!(matches!(action, ResponseAction::Accept));
    }

    #[tokio::test]
    async fn all_agents_approve() {
        let agent_a_id = Uuid::new_v4();
        let agent_b_id = Uuid::new_v4();
        let agent_a = make_agent(agent_a_id, "Security Expert");
        let agent_b = make_agent(agent_b_id, "Perf Optimizer");

        let provider: Arc<dyn LLMProvider> = Arc::new(TestLLMProvider::new(vec![
            r#"{"approved": true, "issues": []}"#.to_string(),
            r#"{"approved": true, "issues": []}"#.to_string(),
        ]));
        let repo = mock_repo_with_agents(vec![agent_a, agent_b]);
        let filter =
            DebateVerificationFilter::new(provider, repo, vec![agent_a_id, agent_b_id], None, None);
        let filter: &dyn ExecutionFilter = &filter;

        let ctx = FilterContext::new("m", Uuid::new_v4());

        // on_start to capture prompt context
        let msgs = vec![crate::llm::Message::user("Write a function")];
        filter
            .on_start(&ctx, String::from("System prompt"), msgs)
            .await
            .unwrap();

        let response = mock_response();
        let action = filter.on_response(&ctx, &response).await.unwrap();
        assert!(matches!(action, ResponseAction::Accept));
    }

    #[tokio::test]
    async fn critique_triggers_retry() {
        let agent_id = Uuid::new_v4();
        let agent = make_agent(agent_id, "Security Expert");

        let critique = r#"{
            "approved": false,
            "issues": [
                {
                    "severity": "high",
                    "description": "SQL injection risk on line 5",
                    "suggestion": "Use parameterized queries"
                }
            ]
        }"#;

        let provider: Arc<dyn LLMProvider> =
            Arc::new(TestLLMProvider::new(vec![critique.to_string()]));
        let repo = mock_repo_with_agents(vec![agent]);
        let filter = DebateVerificationFilter::new(provider, repo, vec![agent_id], None, None);
        let filter: &dyn ExecutionFilter = &filter;

        let ctx = FilterContext::new("m", Uuid::new_v4());
        let msgs = vec![crate::llm::Message::user("Write SQL handler")];
        filter
            .on_start(&ctx, String::from("System prompt"), msgs)
            .await
            .unwrap();

        let response = mock_response();
        let action = filter.on_response(&ctx, &response).await.unwrap();

        match action {
            ResponseAction::Retry { feedback } => {
                assert!(feedback.contains("<verification_feedback>"));
                assert!(feedback.contains("needs_revision"));
                assert!(feedback.contains("SQL injection risk on line 5"));
                assert!(feedback.contains("Use parameterized queries"));
            }
            ResponseAction::Accept => panic!("expected Retry, got Accept"),
        }
    }

    #[tokio::test]
    async fn unparseable_critique_treated_as_approved() {
        let agent_id = Uuid::new_v4();
        let agent = make_agent(agent_id, "Confused Agent");

        let provider: Arc<dyn LLMProvider> = Arc::new(TestLLMProvider::new(vec![
            "This is not valid JSON at all, just rambling text.".to_string(),
        ]));
        let repo = mock_repo_with_agents(vec![agent]);
        let filter = DebateVerificationFilter::new(provider, repo, vec![agent_id], None, None);
        let filter: &dyn ExecutionFilter = &filter;

        let ctx = FilterContext::new("m", Uuid::new_v4());
        let msgs = vec![crate::llm::Message::user("Do something")];
        filter
            .on_start(&ctx, String::from("System"), msgs)
            .await
            .unwrap();

        let response = mock_response();
        let action = filter.on_response(&ctx, &response).await.unwrap();
        assert!(matches!(action, ResponseAction::Accept));
    }

    #[tokio::test]
    async fn captures_prompt_in_on_start() {
        let provider: Arc<dyn LLMProvider> = Arc::new(TestLLMProvider::new(vec![]));
        let repo = mock_repo_with_agents(vec![]);
        let filter = DebateVerificationFilter::new(
            provider,
            repo,
            vec![Uuid::new_v4()], // Non-empty so capture runs
            None,
            None,
        );

        let ctx = FilterContext::new("m", Uuid::new_v4());
        let msgs = vec![crate::llm::Message::user("Build me a server")];
        let filter_ref: &dyn ExecutionFilter = &filter;
        filter_ref
            .on_start(&ctx, String::from("You are a developer."), msgs)
            .await
            .unwrap();

        let capture = filter.prompt_context.lock().await;
        let capture = capture.as_ref().expect("prompt should be captured");
        assert_eq!(capture.system_prompt, "You are a developer.");
        assert_eq!(capture.user_prompt, "Build me a server");
    }

    #[tokio::test]
    async fn parallel_execution_calls_all_agents() {
        let agent_a_id = Uuid::new_v4();
        let agent_b_id = Uuid::new_v4();
        let agent_c_id = Uuid::new_v4();
        let agents = vec![
            make_agent(agent_a_id, "Agent A"),
            make_agent(agent_b_id, "Agent B"),
            make_agent(agent_c_id, "Agent C"),
        ];

        let provider = Arc::new(TestLLMProvider::new(vec![
            r#"{"approved": true, "issues": []}"#.to_string(),
            r#"{"approved": true, "issues": []}"#.to_string(),
            r#"{"approved": true, "issues": []}"#.to_string(),
        ]));
        let call_counter = Arc::clone(&provider);
        let provider_trait: Arc<dyn LLMProvider> = provider;
        let repo = mock_repo_with_agents(agents);
        let filter = DebateVerificationFilter::new(
            provider_trait,
            repo,
            vec![agent_a_id, agent_b_id, agent_c_id],
            None,
            None,
        );
        let filter: &dyn ExecutionFilter = &filter;

        let ctx = FilterContext::new("m", Uuid::new_v4());
        let msgs = vec![crate::llm::Message::user("Task")];
        filter
            .on_start(&ctx, String::from("System"), msgs)
            .await
            .unwrap();

        let response = mock_response();
        filter.on_response(&ctx, &response).await.unwrap();

        assert_eq!(
            call_counter.calls(),
            3,
            "all 3 verification agents should be called"
        );
    }

    #[tokio::test]
    async fn feedback_format_includes_agent_names() {
        let approver_id = Uuid::new_v4();
        let critic_id = Uuid::new_v4();
        let approver = make_agent(approver_id, "Performance Guru");
        let critic = make_agent(critic_id, "Security Auditor");

        let provider: Arc<dyn LLMProvider> = Arc::new(TestLLMProvider::new(vec![
            r#"{"approved": true, "issues": []}"#.to_string(),
            r#"{"approved": false, "issues": [{"severity": "medium", "description": "Missing input validation"}]}"#.to_string(),
        ]));
        let repo = mock_repo_with_agents(vec![approver, critic]);
        let filter =
            DebateVerificationFilter::new(provider, repo, vec![approver_id, critic_id], None, None);
        let filter: &dyn ExecutionFilter = &filter;

        let ctx = FilterContext::new("m", Uuid::new_v4());
        let msgs = vec![crate::llm::Message::user("Handle user input")];
        filter
            .on_start(&ctx, String::from("System"), msgs)
            .await
            .unwrap();

        let response = mock_response();
        let action = filter.on_response(&ctx, &response).await.unwrap();

        match action {
            ResponseAction::Retry { feedback } => {
                // One agent should show approved, the other needs_revision
                assert!(
                    feedback.contains("verdict=\"approved\""),
                    "feedback should contain approved reviewer section"
                );
                assert!(
                    feedback.contains("verdict=\"needs_revision\""),
                    "feedback should contain needs_revision reviewer section"
                );
                assert!(
                    feedback.contains("Missing input validation"),
                    "feedback should contain the issue"
                );
            }
            ResponseAction::Accept => panic!("expected Retry, got Accept"),
        }
    }

    // =========================================================================
    // New tests — production hardening
    // =========================================================================

    #[tokio::test]
    async fn timeout_treated_as_approved() {
        let agent_id = Uuid::new_v4();
        let agent = make_agent(agent_id, "Slow Agent");

        let provider: Arc<dyn LLMProvider> = Arc::new(SlowLLMProvider);
        let repo = mock_repo_with_agents(vec![agent]);
        let filter = DebateVerificationFilter::new(provider, repo, vec![agent_id], None, None);
        let filter: &dyn ExecutionFilter = &filter;

        let ctx = FilterContext::new("m", Uuid::new_v4());
        let msgs = vec![crate::llm::Message::user("Task")];
        filter
            .on_start(&ctx, String::from("System"), msgs)
            .await
            .unwrap();

        let response = mock_response();

        // The filter should timeout (60s) and treat as approved, not hang forever.
        let action = tokio::time::timeout(
            std::time::Duration::from_secs(90),
            filter.on_response(&ctx, &response),
        )
        .await
        .expect("filter should not hang — timeout should fire")
        .unwrap();

        assert!(
            matches!(action, ResponseAction::Accept),
            "timed-out verification should be treated as approved"
        );
    }

    #[tokio::test]
    async fn records_verification_executions() {
        let agent_id = Uuid::new_v4();
        let agent = make_agent(agent_id, "Auditor");
        let parent_ae_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let provider: Arc<dyn LLMProvider> = Arc::new(TestLLMProvider::new(vec![
            r#"{"approved": true, "issues": []}"#.to_string(),
        ]));
        let repo = mock_repo_with_agents(vec![agent.clone()]);

        // Mock AgentExecutionRepo — expect create + update per verifier.
        let mut ae_mock = crate::db::traits::MockAgentExecutionRepo::new();
        let _ae_row = make_ae_row(agent_id);

        ae_mock
            .expect_create_agent_execution()
            .withf(move |aid, _, _, parent_id, _, _, _, _, _, _| {
                *aid == agent_id && *parent_id == Some(parent_ae_id)
            })
            .times(1)
            .returning(move |_, _, _, _, _, _, _, _, _, _| Ok(make_ae_row(agent_id)));

        ae_mock
            .expect_update_agent_execution_status()
            .withf(move |_, status, _, _| status == "completed")
            .times(1)
            .returning(move |_, _, _, _| Ok(make_ae_row(agent_id)));

        let ae_repo: Arc<dyn AgentExecutionRepo> = Arc::new(ae_mock);

        let filter =
            DebateVerificationFilter::new(provider, repo, vec![agent_id], Some(ae_repo), None);
        let filter: &dyn ExecutionFilter = &filter;

        let mut ctx = FilterContext::new("m", Uuid::new_v4());
        ctx.metadata.insert(
            "agent_execution_id".into(),
            serde_json::to_value(parent_ae_id).unwrap(),
        );
        ctx.metadata
            .insert("user_id".into(), serde_json::to_value(user_id).unwrap());

        let msgs = vec![crate::llm::Message::user("Review this")];
        filter
            .on_start(&ctx, String::from("System"), msgs)
            .await
            .unwrap();

        let response = mock_response();
        let action = filter.on_response(&ctx, &response).await.unwrap();
        assert!(matches!(action, ResponseAction::Accept));
        // Mock expectations are automatically verified on drop.
    }

    #[tokio::test]
    async fn records_token_usage() {
        let agent_id = Uuid::new_v4();
        let agent = make_agent(agent_id, "Token Tracker");
        let user_id = Uuid::new_v4();

        let provider: Arc<dyn LLMProvider> = Arc::new(TestLLMProvider::new(vec![
            r#"{"approved": true, "issues": []}"#.to_string(),
        ]));
        let repo = mock_repo_with_agents(vec![agent.clone()]);

        // Mock TokenLedgerRepo — expect insert_ledger_entry called once.
        let mut tl_mock = crate::db::traits::MockTokenLedgerRepo::new();
        tl_mock
            .expect_insert_ledger_entry()
            .withf(move |uid, _, model, in_tok, out_tok, _| {
                *uid == user_id && model == "claude-3" && *in_tok == 100 && *out_tok == 50
            })
            .times(1)
            .returning(move |uid, _, _, _, _, _| Ok(make_tl_row(uid)));

        let tl_repo: Arc<dyn TokenLedgerRepo> = Arc::new(tl_mock);

        let filter =
            DebateVerificationFilter::new(provider, repo, vec![agent_id], None, Some(tl_repo));
        let filter: &dyn ExecutionFilter = &filter;

        let mut ctx = FilterContext::new("m", Uuid::new_v4());
        ctx.metadata
            .insert("user_id".into(), serde_json::to_value(user_id).unwrap());

        let msgs = vec![crate::llm::Message::user("Track tokens")];
        filter
            .on_start(&ctx, String::from("System"), msgs)
            .await
            .unwrap();

        let response = mock_response();
        let action = filter.on_response(&ctx, &response).await.unwrap();
        assert!(matches!(action, ResponseAction::Accept));
        // Mock expectations are automatically verified on drop.
    }
}
