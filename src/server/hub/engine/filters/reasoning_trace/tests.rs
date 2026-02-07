#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::server::hub::engine::filters::FilterContext;
    use uuid::Uuid;

    fn schema_ctx() -> FilterContext {
        FilterContext::new("m", Uuid::new_v4()).with_schema(serde_json::json!({}))
    }

    #[tokio::test]
    async fn skips_when_no_schema_on_start() {
        let filter = ReasoningTraceFilter::new();
        let ctx = FilterContext::new("m", Uuid::new_v4());
        let (sys, _) = filter
            .on_start(&ctx, "Base prompt.".into(), vec![])
            .await
            .unwrap();
        assert_eq!(sys, "Base prompt.");
    }

    #[tokio::test]
    async fn skips_when_no_schema_on_output() {
        let filter = ReasoningTraceFilter::new();
        let ctx = FilterContext::new("m", Uuid::new_v4());
        let out = filter.on_output(&ctx, "anything".into()).await.unwrap();
        assert_eq!(out, "anything");
    }

    #[tokio::test]
    async fn augments_prompt_when_schema_present() {
        let filter = ReasoningTraceFilter::new();
        let ctx = schema_ctx();
        let (sys, _) = filter
            .on_start(&ctx, "Base prompt.".into(), vec![])
            .await
            .unwrap();
        assert!(sys.starts_with("Base prompt."));
        assert!(sys.contains("Reasoning Trace"));
        assert!(sys.contains("reasoning"));
        assert!(sys.contains("result"));
    }

    #[tokio::test]
    async fn strips_reasoning_from_output() {
        let filter = ReasoningTraceFilter::new();
        let ctx = schema_ctx();
        let input = r#"{"reasoning": "I thought about it", "result": {"key": "val"}}"#;
        let out = filter.on_output(&ctx, input.into()).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed, serde_json::json!({"key": "val"}));
    }

    #[tokio::test]
    async fn passes_through_when_no_result_key() {
        let filter = ReasoningTraceFilter::new();
        let ctx = schema_ctx();
        let input = r#"{"key": "val"}"#;
        let out = filter.on_output(&ctx, input.into()).await.unwrap();
        assert_eq!(out, r#"{"key": "val"}"#);
    }

    #[tokio::test]
    async fn passes_through_non_json() {
        let filter = ReasoningTraceFilter::new();
        let ctx = schema_ctx();
        let out = filter
            .on_output(&ctx, "not json at all".into())
            .await
            .unwrap();
        assert_eq!(out, "not json at all");
    }

    #[tokio::test]
    async fn handles_result_as_array() {
        let filter = ReasoningTraceFilter::new();
        let ctx = schema_ctx();
        let input = r#"{"reasoning": "checking items", "result": [1, 2, 3]}"#;
        let out = filter.on_output(&ctx, input.into()).await.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed, serde_json::json!([1, 2, 3]));
    }

    #[tokio::test]
    async fn handles_result_as_string() {
        let filter = ReasoningTraceFilter::new();
        let ctx = schema_ctx();
        let input = r#"{"reasoning": "simple answer", "result": "hello"}"#;
        let out = filter.on_output(&ctx, input.into()).await.unwrap();
        assert_eq!(out, r#""hello""#);
    }
}
