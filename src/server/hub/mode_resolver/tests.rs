#[cfg(test)]
mod tests {
    use std::pin::Pin;
    use std::sync::Arc;

    use chrono::Utc;
    use futures::stream;
    use uuid::Uuid;

    use crate::db::traits::{MockServerRepo, MockToolCapabilityRepo, MockToolRouterRepo};
    use crate::db::{AgentRow, ToolCapabilityRow, ToolRouterModeRow, ToolRouterRow};
    use async_trait::async_trait;

    use crate::llm::{
        LLMProvider, LLMRequest, LLMResponse, LLMResult, StopReason, StreamChunk, TokenUsage,
    };

    use crate::server::hub::mode_resolver::ModeResolver;

    // =========================================================================
    // Test LLM provider that returns a fixed mode key
    // =========================================================================

    struct MockLLMProvider {
        response: String,
    }

    impl MockLLMProvider {
        fn returning_mode(mode_key: &str) -> Self {
            Self {
                response: format!(r#"{{"mode": "{}"}}"#, mode_key),
            }
        }
    }

    #[async_trait]
    impl LLMProvider for MockLLMProvider {
        async fn send_message(&self, _request: LLMRequest) -> LLMResult<LLMResponse> {
            Ok(LLMResponse {
                content: self.response.clone(),
                content_blocks: vec![],
                model: "mock".into(),
                stop_reason: StopReason::EndTurn,
                usage: TokenUsage {
                    input_tokens: 10,
                    output_tokens: 5,
                },
            })
        }

        async fn send_message_stream(
            &self,
            _request: LLMRequest,
        ) -> LLMResult<Pin<Box<dyn futures::Stream<Item = LLMResult<StreamChunk>> + Send>>>
        {
            Ok(Box::pin(stream::empty()))
        }

        fn provider_name(&self) -> &'static str {
            "mock"
        }

        fn model_id(&self) -> &str {
            "mock-model"
        }
    }

    // =========================================================================
    // Test data helpers
    // =========================================================================

    fn test_agent(router_id: Option<Uuid>) -> AgentRow {
        AgentRow {
            id: Uuid::new_v4(),
            tier: None,
            name: "test-agent".into(),
            system_prompt: "You are a test agent.".into(),
            persona_style: None,
            model_provider: "anthropic".into(),
            model_id: "claude-3-haiku-20240307".into(),
            model_max_tokens: 4096,
            model_temperature: 0.7,
            status: None,
            router_mode: None,
            router_id,
            output_schema_id: None,
            version: 1,
        }
    }

    fn test_router(id: Uuid) -> ToolRouterRow {
        ToolRouterRow {
            id,
            user_id: Uuid::new_v4(),
            name: "test-router".into(),
            description: None,
            system_prompt: "You are a mode classifier.".into(),
            model_id: "claude-3-haiku-20240307".into(),
            is_active: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            parent_router_id: None,
            level: 1,
        }
    }

    fn test_mode(id: Uuid, router_id: Uuid, mode_key: &str) -> ToolRouterModeRow {
        ToolRouterModeRow {
            id,
            router_id,
            mode_key: mode_key.into(),
            display_name: mode_key.into(),
            description: format!("Mode for {}", mode_key),
            system_prompt: format!("You are in {} mode.", mode_key),
            temperature: 0.5,
            max_tokens: 2048,
            append_to_agent_system_prompt: false,
            append_to_agent_tools: false,
            display_order: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn test_capability(key: &str) -> ToolCapabilityRow {
        ToolCapabilityRow {
            id: Uuid::new_v4(),
            capability_key: key.into(),
            display_name: key.into(),
            category: "test".into(),
            safety_level: "safe".into(),
            description: format!("{} capability", key),
            created_at: Utc::now(),
        }
    }

    fn test_tool_row(name: &str) -> crate::db::ToolRow {
        crate::db::ToolRow {
            id: Uuid::new_v4(),
            name: name.into(),
            display_name: name.into(),
            description: format!("{} tool", name),
            parameters: serde_json::json!({}),
            created_at: Utc::now(),
            version: 1,
        }
    }

    // =========================================================================
    // Tests
    // =========================================================================

    #[tokio::test]
    async fn resolve_no_router_returns_agent_defaults_with_empty_capabilities() {
        let agent = test_agent(None);

        let mut server_repo = MockServerRepo::new();
        server_repo
            .expect_get_agent_tools()
            .returning(|_| Ok(vec![test_tool_row("read_file")]));

        let tool_router_repo = MockToolRouterRepo::new();
        let tool_cap_repo = MockToolCapabilityRepo::new();
        let provider = MockLLMProvider::returning_mode("coding");

        let resolver = ModeResolver::new(
            Arc::new(server_repo),
            Arc::new(tool_router_repo),
            Arc::new(tool_cap_repo),
            Arc::new(provider),
        );

        let result = resolver
            .resolve(&agent, "hello", None::<&str>)
            .await
            .unwrap();

        assert!(result.selected_mode_id.is_none());
        assert!(result.capabilities.is_empty());
        assert_eq!(result.tools.len(), 1);
        assert_eq!(result.tools[0].name, "read_file");
    }

    #[tokio::test]
    async fn resolve_with_capabilities_auto_selects_tools() {
        let router_id = Uuid::new_v4();
        let mode_id = Uuid::new_v4();
        let agent = test_agent(Some(router_id));
        let mode = test_mode(mode_id, router_id, "coding");

        let mut server_repo = MockServerRepo::new();
        server_repo
            .expect_get_agent_tools()
            .returning(|_| Ok(vec![]));

        let mut tool_router_repo = MockToolRouterRepo::new();
        {
            let router = test_router(router_id);
            tool_router_repo
                .expect_get_tool_router()
                .returning(move |_| Ok(Some(router.clone())));
        }
        {
            let mode_clone = mode.clone();
            tool_router_repo
                .expect_list_router_modes()
                .returning(move |_| Ok(vec![mode_clone.clone()]));
        }
        // No explicit tools on this mode
        tool_router_repo
            .expect_get_mode_tools()
            .returning(|_| Ok(vec![]));

        let mut tool_cap_repo = MockToolCapabilityRepo::new();
        // Mode requires "file_read" capability
        let cap = test_capability("file_read");
        tool_cap_repo
            .expect_get_mode_capabilities()
            .returning(move |_| Ok(vec![cap.clone()]));
        // "file_read" capability is provided by "read_file" tool
        tool_cap_repo
            .expect_get_tools_by_capability()
            .withf(|key| key == "file_read")
            .returning(|_| Ok(vec![test_tool_row("read_file")]));

        let provider = MockLLMProvider::returning_mode("coding");

        let resolver = ModeResolver::new(
            Arc::new(server_repo),
            Arc::new(tool_router_repo),
            Arc::new(tool_cap_repo),
            Arc::new(provider),
        );

        let result = resolver
            .resolve(&agent, "read this file", None::<&str>)
            .await
            .unwrap();

        assert_eq!(result.selected_mode_key.as_deref(), Some("coding"));
        assert_eq!(result.capabilities, vec!["file_read"]);
        assert_eq!(result.tools.len(), 1);
        assert_eq!(result.tools[0].name, "read_file");
    }

    #[tokio::test]
    async fn capabilities_union_with_explicit_tools() {
        let router_id = Uuid::new_v4();
        let mode_id = Uuid::new_v4();
        let agent = test_agent(Some(router_id));
        let mode = test_mode(mode_id, router_id, "coding");

        let mut server_repo = MockServerRepo::new();
        server_repo
            .expect_get_agent_tools()
            .returning(|_| Ok(vec![]));

        let mut tool_router_repo = MockToolRouterRepo::new();
        {
            let router = test_router(router_id);
            tool_router_repo
                .expect_get_tool_router()
                .returning(move |_| Ok(Some(router.clone())));
        }
        {
            let mode_clone = mode.clone();
            tool_router_repo
                .expect_list_router_modes()
                .returning(move |_| Ok(vec![mode_clone.clone()]));
        }
        // Explicit tool: write_file
        tool_router_repo
            .expect_get_mode_tools()
            .returning(|_| Ok(vec![test_tool_row("write_file")]));

        let mut tool_cap_repo = MockToolCapabilityRepo::new();
        // Mode also requires "file_read" capability
        let cap = test_capability("file_read");
        tool_cap_repo
            .expect_get_mode_capabilities()
            .returning(move |_| Ok(vec![cap.clone()]));
        // "file_read" provides read_file
        tool_cap_repo
            .expect_get_tools_by_capability()
            .returning(|_| Ok(vec![test_tool_row("read_file")]));

        let provider = MockLLMProvider::returning_mode("coding");

        let resolver = ModeResolver::new(
            Arc::new(server_repo),
            Arc::new(tool_router_repo),
            Arc::new(tool_cap_repo),
            Arc::new(provider),
        );

        let result = resolver
            .resolve(&agent, "edit code", None::<&str>)
            .await
            .unwrap();

        let tool_names: Vec<&str> = result.tools.iter().map(|t| t.name.as_str()).collect();
        assert!(tool_names.contains(&"write_file"));
        assert!(tool_names.contains(&"read_file"));
        assert_eq!(result.tools.len(), 2);
    }

    #[tokio::test]
    async fn empty_capabilities_returns_explicit_tools_only() {
        let router_id = Uuid::new_v4();
        let mode_id = Uuid::new_v4();
        let agent = test_agent(Some(router_id));
        let mode = test_mode(mode_id, router_id, "coding");

        let mut server_repo = MockServerRepo::new();
        server_repo
            .expect_get_agent_tools()
            .returning(|_| Ok(vec![]));

        let mut tool_router_repo = MockToolRouterRepo::new();
        {
            let router = test_router(router_id);
            tool_router_repo
                .expect_get_tool_router()
                .returning(move |_| Ok(Some(router.clone())));
        }
        {
            let mode_clone = mode.clone();
            tool_router_repo
                .expect_list_router_modes()
                .returning(move |_| Ok(vec![mode_clone.clone()]));
        }
        tool_router_repo
            .expect_get_mode_tools()
            .returning(|_| Ok(vec![test_tool_row("git_status")]));

        let mut tool_cap_repo = MockToolCapabilityRepo::new();
        // No capabilities required
        tool_cap_repo
            .expect_get_mode_capabilities()
            .returning(|_| Ok(vec![]));

        let provider = MockLLMProvider::returning_mode("coding");

        let resolver = ModeResolver::new(
            Arc::new(server_repo),
            Arc::new(tool_router_repo),
            Arc::new(tool_cap_repo),
            Arc::new(provider),
        );

        let result = resolver
            .resolve(&agent, "git status", None::<&str>)
            .await
            .unwrap();

        assert!(result.capabilities.is_empty());
        assert_eq!(result.tools.len(), 1);
        assert_eq!(result.tools[0].name, "git_status");
    }

    #[tokio::test]
    async fn duplicate_tools_from_explicit_and_capability_are_deduplicated() {
        let router_id = Uuid::new_v4();
        let mode_id = Uuid::new_v4();
        let agent = test_agent(Some(router_id));
        let mode = test_mode(mode_id, router_id, "coding");

        let mut server_repo = MockServerRepo::new();
        server_repo
            .expect_get_agent_tools()
            .returning(|_| Ok(vec![]));

        let mut tool_router_repo = MockToolRouterRepo::new();
        {
            let router = test_router(router_id);
            tool_router_repo
                .expect_get_tool_router()
                .returning(move |_| Ok(Some(router.clone())));
        }
        {
            let mode_clone = mode.clone();
            tool_router_repo
                .expect_list_router_modes()
                .returning(move |_| Ok(vec![mode_clone.clone()]));
        }
        // Explicit: read_file
        tool_router_repo
            .expect_get_mode_tools()
            .returning(|_| Ok(vec![test_tool_row("read_file")]));

        let mut tool_cap_repo = MockToolCapabilityRepo::new();
        let cap = test_capability("file_read");
        tool_cap_repo
            .expect_get_mode_capabilities()
            .returning(move |_| Ok(vec![cap.clone()]));
        // Capability also resolves to read_file (duplicate)
        tool_cap_repo
            .expect_get_tools_by_capability()
            .returning(|_| Ok(vec![test_tool_row("read_file")]));

        let provider = MockLLMProvider::returning_mode("coding");

        let resolver = ModeResolver::new(
            Arc::new(server_repo),
            Arc::new(tool_router_repo),
            Arc::new(tool_cap_repo),
            Arc::new(provider),
        );

        let result = resolver
            .resolve(&agent, "read file", None::<&str>)
            .await
            .unwrap();

        // Should only appear once despite being in both explicit and capability sources
        assert_eq!(result.tools.len(), 1);
        assert_eq!(result.tools[0].name, "read_file");
        assert_eq!(result.capabilities, vec!["file_read"]);
    }
}
