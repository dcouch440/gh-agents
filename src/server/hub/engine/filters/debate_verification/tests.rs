#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    use async_trait::async_trait;
    use futures::Stream;
    use std::pin::Pin;
    use uuid::Uuid;

    use crate::db::traits::ServerRepo;
    use crate::db::AgentRow;
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
            "test"
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
    // Tests
    // =========================================================================

    #[tokio::test]
    async fn no_verification_agents_passthrough() {
        let provider: Arc<dyn LLMProvider> = Arc::new(TestLLMProvider::new(vec![]));
        let repo = mock_repo_with_agents(vec![]);
        let filter = DebateVerificationFilter::new(provider, repo, vec![]);
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
        let filter = DebateVerificationFilter::new(provider, repo, vec![agent_a_id, agent_b_id]);
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
        let filter = DebateVerificationFilter::new(provider, repo, vec![agent_id]);
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
                assert!(feedback.contains("Verification Panel Feedback"));
                assert!(feedback.contains("NEEDS REVISION"));
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
        let filter = DebateVerificationFilter::new(provider, repo, vec![agent_id]);
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
        let filter = DebateVerificationFilter::new(provider, repo, vec![approver_id, critic_id]);
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
                // One agent should show APPROVED, the other NEEDS REVISION
                assert!(
                    feedback.contains("APPROVED"),
                    "feedback should contain APPROVED section"
                );
                assert!(
                    feedback.contains("NEEDS REVISION"),
                    "feedback should contain NEEDS REVISION section"
                );
                assert!(
                    feedback.contains("Missing input validation"),
                    "feedback should contain the issue"
                );
            }
            ResponseAction::Accept => panic!("expected Retry, got Accept"),
        }
    }
}
