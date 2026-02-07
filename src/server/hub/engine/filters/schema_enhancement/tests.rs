#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::server::hub::engine::filters::FilterContext;
    use uuid::Uuid;

    #[tokio::test]
    async fn skips_when_no_schema() {
        let filter = SchemaEnhancementFilter::new();
        let ctx = FilterContext::new("m", Uuid::new_v4());
        let (sys, _) = filter
            .on_start(&ctx, "You are helpful.".into(), vec![])
            .await
            .unwrap();
        assert_eq!(sys, "You are helpful.");
    }

    #[tokio::test]
    async fn augments_when_schema_present() {
        let filter = SchemaEnhancementFilter::new();
        let ctx = FilterContext::new("m", Uuid::new_v4())
            .with_schema(serde_json::json!({"type": "object"}));
        let (sys, _) = filter
            .on_start(&ctx, "Base prompt.".into(), vec![])
            .await
            .unwrap();
        assert!(sys.starts_with("Base prompt."));
        assert!(sys.contains("<output_rules>"));
        assert!(sys.contains("</output_rules>"));
        assert!(sys.contains("Do NOT wrap"));
        assert!(sys.contains("Do NOT include any text"));
    }

    #[tokio::test]
    async fn preserves_messages() {
        let filter = SchemaEnhancementFilter::new();
        let ctx = FilterContext::new("m", Uuid::new_v4())
            .with_schema(serde_json::json!({"type": "object"}));
        let msgs = vec![crate::llm::Message::user("hello")];
        let (_, returned_msgs) = filter.on_start(&ctx, "sys".into(), msgs).await.unwrap();
        assert_eq!(returned_msgs.len(), 1);
    }
}
