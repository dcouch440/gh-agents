#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::llm::{ContentBlock, LLMResponse, StopReason, TokenUsage};
    use crate::server::hub::engine::filters::FilterContext;
    use uuid::Uuid;

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
                ..Default::default()
            },
        }
    }

    fn schema_ctx() -> FilterContext {
        FilterContext::new("m", Uuid::new_v4()).with_schema(serde_json::json!({}))
    }

    #[tokio::test]
    async fn accepts_when_no_schema() {
        let filter = SchemaValidationRetryFilter::new();
        let ctx = FilterContext::new("m", Uuid::new_v4());
        let action = filter
            .on_response(&ctx, &make_response("anything"))
            .await
            .unwrap();
        assert!(matches!(action, ResponseAction::Accept));
    }

    #[tokio::test]
    async fn accepts_valid_json_object() {
        let filter = SchemaValidationRetryFilter::new();
        let ctx = schema_ctx();
        let action = filter
            .on_response(&ctx, &make_response(r#"{"key": "val"}"#))
            .await
            .unwrap();
        assert!(matches!(action, ResponseAction::Accept));
    }

    #[tokio::test]
    async fn accepts_valid_json_array() {
        let filter = SchemaValidationRetryFilter::new();
        let ctx = schema_ctx();
        let action = filter
            .on_response(&ctx, &make_response(r#"[{"a": 1}]"#))
            .await
            .unwrap();
        assert!(matches!(action, ResponseAction::Accept));
    }

    #[tokio::test]
    async fn retries_on_plain_text() {
        let filter = SchemaValidationRetryFilter::new();
        let ctx = schema_ctx();
        let action = filter
            .on_response(&ctx, &make_response("Here is my analysis..."))
            .await
            .unwrap();
        match action {
            ResponseAction::Retry { feedback } => {
                assert!(feedback.contains("failed to parse"));
            }
            ResponseAction::Accept => panic!("expected Retry"),
        }
    }

    #[tokio::test]
    async fn retries_on_code_fence_wrapped_json() {
        let filter = SchemaValidationRetryFilter::new();
        let ctx = schema_ctx();
        let content = "```json\n{\"key\": \"val\"}\n```";
        let action = filter
            .on_response(&ctx, &make_response(content))
            .await
            .unwrap();
        match action {
            ResponseAction::Retry { feedback } => {
                assert!(feedback.contains("code fences"));
            }
            ResponseAction::Accept => panic!("expected Retry"),
        }
    }

    #[tokio::test]
    async fn retries_on_primitive_json() {
        let filter = SchemaValidationRetryFilter::new();
        let ctx = schema_ctx();
        let action = filter
            .on_response(&ctx, &make_response("42"))
            .await
            .unwrap();
        match action {
            ResponseAction::Retry { feedback } => {
                assert!(feedback.contains("primitive"));
                assert!(feedback.contains("number"));
            }
            ResponseAction::Accept => panic!("expected Retry"),
        }
    }

    #[tokio::test]
    async fn retries_on_string_json() {
        let filter = SchemaValidationRetryFilter::new();
        let ctx = schema_ctx();
        let action = filter
            .on_response(&ctx, &make_response(r#""just a string""#))
            .await
            .unwrap();
        match action {
            ResponseAction::Retry { feedback } => {
                assert!(feedback.contains("primitive"));
                assert!(feedback.contains("string"));
            }
            ResponseAction::Accept => panic!("expected Retry"),
        }
    }

    #[test]
    fn extract_from_json_fence() {
        let input = "```json\n{\"a\": 1}\n```";
        assert_eq!(
            try_extract_from_fence(input),
            Some("{\"a\": 1}".to_string())
        );
    }

    #[test]
    fn extract_from_plain_fence() {
        let input = "```\n{\"a\": 1}\n```";
        assert_eq!(
            try_extract_from_fence(input),
            Some("{\"a\": 1}".to_string())
        );
    }

    #[test]
    fn extract_no_fence() {
        assert_eq!(try_extract_from_fence(r#"{"a": 1}"#), None);
    }
}
