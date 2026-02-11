#[cfg(test)]
mod tests {
    mod coordinator {
        use crate::llm::TokenUsage;
        use crate::server::hub::strategies::documenter::coordinator::{
            DocumenterCoordinatorConfig, DocumenterCoordinatorStrategy,
        };
        use crate::server::hub::strategy::ExecutionStrategy;

        fn make_strategy() -> DocumenterCoordinatorStrategy {
            DocumenterCoordinatorStrategy::new(DocumenterCoordinatorConfig {
                system_prompt: "You are a document strategist.".into(),
                model_id: "claude-sonnet-4-20250514".into(),
                temperature: 0.3,
                max_rounds: 1,
                context_budget: 100_000,
                state: None,
                user_id: None,
            })
        }

        #[test]
        fn coordinator_properties() {
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
        async fn coordinator_build_messages() {
            let strategy = make_strategy();
            let messages = strategy
                .build_messages("Plan documents for this project")
                .await
                .unwrap();

            assert_eq!(messages.len(), 1);
            assert_eq!(messages[0].text(), "Plan documents for this project");
        }

        #[tokio::test]
        async fn coordinator_on_complete_noop_without_state() {
            let strategy = make_strategy();
            let usage = TokenUsage {
                input_tokens: 500,
                output_tokens: 200,
            };
            // Should succeed without state (no token ledger write)
            strategy.on_complete("{}", &usage).await.unwrap();
        }
    }

    mod research {
        use crate::llm::{TokenUsage, Tool};
        use crate::server::hub::strategies::documenter::research::{
            DocumenterResearchConfig, DocumenterResearchStrategy,
        };
        use crate::server::hub::strategy::ExecutionStrategy;

        fn make_tools() -> Vec<Tool> {
            vec![
                Tool {
                    name: "web_research".into(),
                    description: "Search the web".into(),
                    input_schema: serde_json::json!({"type": "object", "properties": {"query": {"type": "string"}}}),
                },
                Tool {
                    name: "read_file".into(),
                    description: "Read a file".into(),
                    input_schema: serde_json::json!({"type": "object", "properties": {"path": {"type": "string"}}}),
                },
            ]
        }

        fn make_strategy() -> DocumenterResearchStrategy {
            DocumenterResearchStrategy::new(DocumenterResearchConfig {
                system_prompt: "You are a research assistant.".into(),
                model_id: "claude-sonnet-4-20250514".into(),
                temperature: 0.2,
                max_rounds: 15,
                context_budget: 480_000,
                tools: make_tools(),
                tool_names: vec!["web_research".into(), "read_file".into()],
                execution_context: None,
                state: None,
                user_id: None,
            })
        }

        #[test]
        fn research_properties() {
            let strategy = make_strategy();

            assert_eq!(strategy.system_prompt(), "You are a research assistant.");
            assert_eq!(strategy.model_id(), "claude-sonnet-4-20250514");
            assert_eq!(strategy.max_rounds(), 15);
            assert_eq!(strategy.context_budget(), 480_000);
            assert!(!strategy.streaming());
            assert!((strategy.temperature() - 0.2).abs() < f32::EPSILON);
        }

        #[test]
        fn research_tools_returned() {
            let strategy = make_strategy();
            let tools = strategy.tools();

            assert_eq!(tools.len(), 2);
            assert_eq!(tools[0].name, "web_research");
            assert_eq!(tools[1].name, "read_file");
        }

        #[tokio::test]
        async fn research_build_messages() {
            let strategy = make_strategy();
            let messages = strategy
                .build_messages("Research the API endpoints")
                .await
                .unwrap();

            assert_eq!(messages.len(), 1);
            assert_eq!(messages[0].text(), "Research the API endpoints");
        }

        #[tokio::test]
        async fn research_on_complete_noop_without_state() {
            let strategy = make_strategy();
            let usage = TokenUsage {
                input_tokens: 1000,
                output_tokens: 500,
            };
            strategy
                .on_complete("research results", &usage)
                .await
                .unwrap();
        }
    }

    mod writer {
        use crate::llm::TokenUsage;
        use crate::server::hub::strategies::documenter::writer::{
            DocumenterWriterConfig, DocumenterWriterStrategy,
        };
        use crate::server::hub::strategy::ExecutionStrategy;

        fn make_strategy() -> DocumenterWriterStrategy {
            DocumenterWriterStrategy::new(DocumenterWriterConfig {
                system_prompt: "You are a technical writer.".into(),
                model_id: "claude-sonnet-4-20250514".into(),
                temperature: 0.5,
                max_rounds: 1,
                context_budget: 480_000,
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
}
