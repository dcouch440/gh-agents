#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::server::hub::engine::filters::FilterContext;
    use uuid::Uuid;

    fn schema_ctx() -> FilterContext {
        FilterContext::new("m", Uuid::new_v4()).with_schema(serde_json::json!({}))
    }

    #[tokio::test]
    async fn passthrough_when_no_schema() {
        let filter = PartialJsonRecoveryFilter::new();
        let ctx = FilterContext::new("m", Uuid::new_v4());
        let out = filter.on_output(&ctx, "not json".into()).await.unwrap();
        assert_eq!(out, "not json");
    }

    #[tokio::test]
    async fn passthrough_when_valid_json() {
        let filter = PartialJsonRecoveryFilter::new();
        let ctx = schema_ctx();
        let out = filter
            .on_output(&ctx, r#"{"key": "value"}"#.into())
            .await
            .unwrap();
        assert_eq!(out, r#"{"key": "value"}"#);
    }

    #[tokio::test]
    async fn recovers_truncated_object() {
        let filter = PartialJsonRecoveryFilter::new();
        let ctx = schema_ctx();
        let out = filter
            .on_output(&ctx, r#"{"key": "value""#.into())
            .await
            .unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed["key"], "value");
    }

    #[tokio::test]
    async fn recovers_truncated_nested() {
        let filter = PartialJsonRecoveryFilter::new();
        let ctx = schema_ctx();
        let input = r#"{"items": [{"name": "a"}, {"name": "b""#;
        let out = filter.on_output(&ctx, input.into()).await.unwrap();
        assert!(serde_json::from_str::<serde_json::Value>(&out).is_ok());
    }

    #[tokio::test]
    async fn recovers_truncated_array() {
        let filter = PartialJsonRecoveryFilter::new();
        let ctx = schema_ctx();
        let input = r#"[{"a": 1}, {"b": 2"#;
        let out = filter.on_output(&ctx, input.into()).await.unwrap();
        assert!(serde_json::from_str::<serde_json::Value>(&out).is_ok());
    }

    #[test]
    fn recover_unclosed_string() {
        let result = recover_truncated_json(r#"{"key": "value that is cut"#);
        assert!(result.is_some());
        assert!(serde_json::from_str::<serde_json::Value>(&result.unwrap()).is_ok());
    }

    #[test]
    fn recover_returns_none_for_balanced() {
        // Balanced delimiters but invalid JSON for other reasons
        let result = recover_truncated_json(r#"{"key": }"#);
        assert!(result.is_none());
    }

    #[test]
    fn recover_empty_input() {
        assert!(recover_truncated_json("").is_none());
        assert!(recover_truncated_json("   ").is_none());
    }

    #[test]
    fn recover_no_json_start() {
        assert!(recover_truncated_json("just some text").is_none());
    }

    #[test]
    fn recover_with_escaped_quotes() {
        let input = r#"{"msg": "he said \"hello"#;
        let result = recover_truncated_json(input);
        assert!(result.is_some());
        assert!(serde_json::from_str::<serde_json::Value>(&result.unwrap()).is_ok());
    }

    #[test]
    fn recover_deeply_nested() {
        let input = r#"{"a": {"b": {"c": [1, 2"#;
        let result = recover_truncated_json(input);
        assert!(result.is_some());
        let parsed = serde_json::from_str::<serde_json::Value>(&result.unwrap());
        assert!(parsed.is_ok());
    }
}
