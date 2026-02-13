#[cfg(test)]
mod tests {
    use super::super::{AgentDesignerConfig, AgentDesignerStrategy};
    use crate::server::hub::strategy::ExecutionStrategy;

    fn make_config() -> AgentDesignerConfig {
        AgentDesignerConfig {
            system_prompt: "You are the Agent Designer.".to_string(),
            model_id: "claude-sonnet-4-20250514".to_string(),
            temperature: 0.4,
            max_rounds: 1,
            context_budget: 480_000,
            state: None,
            user_id: None,
        }
    }

    #[test]
    fn strategy_returns_no_tools() {
        let strategy = AgentDesignerStrategy::new(make_config());
        assert!(strategy.tools().is_empty());
    }

    #[test]
    fn strategy_not_streaming() {
        let strategy = AgentDesignerStrategy::new(make_config());
        assert!(!strategy.streaming());
    }

    #[test]
    fn strategy_model_id() {
        let strategy = AgentDesignerStrategy::new(make_config());
        assert_eq!(strategy.model_id(), "claude-sonnet-4-20250514");
    }

    #[test]
    fn strategy_max_rounds_is_one() {
        let strategy = AgentDesignerStrategy::new(make_config());
        assert_eq!(strategy.max_rounds(), 1);
    }

    #[test]
    fn strategy_temperature() {
        let strategy = AgentDesignerStrategy::new(make_config());
        assert!((strategy.temperature() - 0.4).abs() < f32::EPSILON);
    }

    #[test]
    fn strategy_context_budget() {
        let strategy = AgentDesignerStrategy::new(make_config());
        assert_eq!(strategy.context_budget(), 480_000);
    }

    #[test]
    fn strategy_system_prompt() {
        let strategy = AgentDesignerStrategy::new(make_config());
        assert_eq!(strategy.system_prompt(), "You are the Agent Designer.");
    }

    #[tokio::test]
    async fn strategy_build_messages() {
        let strategy = AgentDesignerStrategy::new(make_config());
        let messages = strategy
            .build_messages("Design prompts for these agents.")
            .await
            .unwrap();
        assert_eq!(messages.len(), 1);
    }
}
