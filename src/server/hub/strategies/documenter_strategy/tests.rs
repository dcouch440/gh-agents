#[cfg(test)]
mod tests {
    use crate::llm::TokenUsage;
    use crate::server::hub::strategies::documenter_strategy::{
        DocumenterStrategyConfig, DocumenterStrategyStrategy,
    };
    use crate::server::hub::strategy::ExecutionStrategy;

    fn make_strategy() -> DocumenterStrategyStrategy {
        DocumenterStrategyStrategy::new(DocumenterStrategyConfig {
            system_prompt: "You are a document strategist.".into(),
            model_id: "claude-sonnet-4-20250514".into(),
            state: None,
            user_id: None,
        })
    }

    #[test]
    fn strategy_properties() {
        let strategy = make_strategy();

        assert_eq!(strategy.system_prompt(), "You are a document strategist.");
        assert_eq!(strategy.model_id(), "claude-sonnet-4-20250514");
        assert_eq!(strategy.max_rounds(), 1);
        assert_eq!(strategy.context_budget(), 100_000);
        assert!(!strategy.streaming());
        assert!(strategy.tools().is_empty());
        assert!((strategy.temperature() - 0.3).abs() < f32::EPSILON);
    }

    #[tokio::test]
    async fn strategy_build_messages() {
        let strategy = make_strategy();
        let messages = strategy
            .build_messages("Plan documents for this project")
            .await
            .unwrap();

        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].text(), "Plan documents for this project");
    }

    #[tokio::test]
    async fn strategy_on_complete_noop_without_state() {
        let strategy = make_strategy();
        let usage = TokenUsage {
            input_tokens: 500,
            output_tokens: 200,
        };
        // Should succeed without state (no token ledger write)
        strategy.on_complete("{}", &usage).await.unwrap();
    }
}
