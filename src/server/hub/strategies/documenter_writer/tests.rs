#[cfg(test)]
mod tests {
    use crate::llm::TokenUsage;
    use crate::server::hub::strategies::documenter_writer::{
        DocumenterWriterConfig, DocumenterWriterStrategy,
    };
    use crate::server::hub::strategy::ExecutionStrategy;

    fn make_strategy() -> DocumenterWriterStrategy {
        DocumenterWriterStrategy::new(DocumenterWriterConfig {
            system_prompt: "You are a technical writer.".into(),
            model_id: "claude-sonnet-4-20250514".into(),
            state: None,
            user_id: None,
        })
    }

    #[test]
    fn writer_properties() {
        let strategy = make_strategy();

        assert_eq!(strategy.system_prompt(), "You are a technical writer.");
        assert_eq!(strategy.model_id(), "claude-sonnet-4-20250514");
        assert_eq!(strategy.max_rounds(), 1);
        assert_eq!(strategy.context_budget(), 480_000);
        assert!(!strategy.streaming());
        assert!(strategy.tools().is_empty());
        assert!((strategy.temperature() - 0.5).abs() < f32::EPSILON);
    }

    #[tokio::test]
    async fn writer_build_messages() {
        let strategy = make_strategy();
        let messages = strategy
            .build_messages("Write the API reference document")
            .await
            .unwrap();

        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].text(), "Write the API reference document");
    }

    #[tokio::test]
    async fn writer_on_complete_noop_without_state() {
        let strategy = make_strategy();
        let usage = TokenUsage {
            input_tokens: 800,
            output_tokens: 3000,
        };
        strategy
            .on_complete("# API Reference\n\n...", &usage)
            .await
            .unwrap();
    }
}
