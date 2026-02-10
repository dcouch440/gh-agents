#[cfg(test)]
mod tests {
    use crate::llm::{TokenUsage, Tool};
    use crate::server::hub::strategies::documenter_research::{
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
