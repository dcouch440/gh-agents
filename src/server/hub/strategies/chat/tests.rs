#[cfg(test)]
mod tests {
    //! Tests for chat strategy

    use crate::db::traits::MockServerRepo;
    use crate::server::hub::strategies::chat::{ChatConfig, ChatStrategy};
    use crate::server::hub::strategy::ExecutionStrategy;
    use crate::server::state::test_helpers::default_mock_repos;
    use crate::server::state::AppState;
    use crate::types::{AppConfig, UserId};
    use std::sync::Arc;
    use uuid::Uuid;

    fn make_state() -> AppState {
        let mut mock = MockServerRepo::new();
        mock.expect_health_check().returning(|| true);
        let repo: Arc<dyn crate::db::traits::ServerRepo> = Arc::new(mock);
        let repos = default_mock_repos();
        let (state, _rx) = AppState::with_repo(None, repo, repos, AppConfig::default());
        state
    }

    #[test]
    fn chat_config_defaults() {
        let config = ChatConfig::default();
        assert_eq!(config.max_rounds, 10);
        assert_eq!(config.context_budget, 480_000);
        assert!(config.tool_names.is_empty());
    }

    #[test]
    fn strategy_properties() {
        let state = make_state();
        let config = ChatConfig {
            system_prompt: "You are helpful.".into(),
            tool_names: vec!["think".into()],
            model_id: "claude-sonnet-4-20250514".into(),
            ..Default::default()
        };
        let strategy = ChatStrategy::new(config, state, UserId::new(), None, Uuid::new_v4());

        assert_eq!(strategy.system_prompt(), "You are helpful.");
        assert_eq!(strategy.model_id(), "claude-sonnet-4-20250514");
        assert_eq!(strategy.max_rounds(), 10);
        assert!(strategy.streaming());
    }

    #[tokio::test]
    async fn build_messages_no_session() {
        let state = make_state();
        let config = ChatConfig {
            system_prompt: "sys".into(),
            model_id: "m".into(),
            ..Default::default()
        };
        let strategy = ChatStrategy::new(config, state, UserId::new(), None, Uuid::new_v4());

        let messages = strategy.build_messages("hello").await.unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].text(), "hello");
    }
}
