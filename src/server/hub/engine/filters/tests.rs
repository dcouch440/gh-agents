#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::llm::{ContentBlock, LLMResponse, StopReason, TokenUsage};

    struct NoopFilter;

    #[async_trait]
    impl ExecutionFilter for NoopFilter {
        fn name(&self) -> &str {
            "noop"
        }
    }

    #[test]
    fn filter_context_new() {
        let id = Uuid::new_v4();
        let ctx = FilterContext::new("claude-sonnet-4-20250514", id);
        assert_eq!(ctx.model_id, "claude-sonnet-4-20250514");
        assert_eq!(ctx.agent_id, id);
        assert!(ctx.step_id.is_none());
        assert_eq!(ctx.round, 0);
        assert!(!ctx.has_output_schema);
        assert!(ctx.output_schema.is_none());
        assert!(ctx.metadata.is_empty());
    }

    #[test]
    fn filter_context_with_schema() {
        let schema = serde_json::json!({"type": "object", "properties": {}});
        let ctx = FilterContext::new("m", Uuid::new_v4()).with_schema(schema.clone());
        assert!(ctx.has_output_schema);
        assert_eq!(ctx.output_schema, Some(schema));
    }

    #[test]
    fn filter_context_with_step_id() {
        let step_id = Uuid::new_v4();
        let ctx = FilterContext::new("m", Uuid::new_v4()).with_step_id(step_id);
        assert_eq!(ctx.step_id, Some(step_id));
    }

    fn make_response(content: &str) -> LLMResponse {
        LLMResponse {
            content: content.to_string(),
            content_blocks: vec![ContentBlock::Text {
                text: content.to_string(),
            }],
            model: "m".into(),
            stop_reason: StopReason::EndTurn,
            usage: TokenUsage {
                input_tokens: 10,
                output_tokens: 5,
            },
        }
    }

    #[tokio::test]
    async fn default_on_start_is_passthrough() {
        let filter = NoopFilter;
        let ctx = FilterContext::new("m", Uuid::new_v4());
        let (sys, msgs) = filter.on_start(&ctx, "hello".into(), vec![]).await.unwrap();
        assert_eq!(sys, "hello");
        assert!(msgs.is_empty());
    }

    #[tokio::test]
    async fn default_on_response_accepts() {
        let filter = NoopFilter;
        let ctx = FilterContext::new("m", Uuid::new_v4());
        let response = make_response("anything");
        let action = filter.on_response(&ctx, &response).await.unwrap();
        assert!(matches!(action, ResponseAction::Accept));
    }

    #[tokio::test]
    async fn default_on_output_is_passthrough() {
        let filter = NoopFilter;
        let ctx = FilterContext::new("m", Uuid::new_v4());
        let out = filter.on_output(&ctx, "content".into()).await.unwrap();
        assert_eq!(out, "content");
    }
}
