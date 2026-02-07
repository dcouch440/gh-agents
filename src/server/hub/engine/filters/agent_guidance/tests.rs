#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::db::traits::ServerRepo;
    use crate::db::AgentGuidanceRow;
    use crate::server::hub::engine::filters::FilterContext;
    use chrono::Utc;
    use std::sync::Arc;
    use uuid::Uuid;

    fn make_guidance(agent_id: Uuid, suggestions: serde_json::Value) -> AgentGuidanceRow {
        AgentGuidanceRow {
            id: Uuid::new_v4(),
            agent_id,
            workflow_step_id: None,
            suggestions,
            source: "manual".into(),
            version: 1,
            is_active: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    /// Build a mock repo that returns the given guidance rows.
    fn mock_repo_with_guidances(rows: Vec<AgentGuidanceRow>) -> Arc<dyn ServerRepo> {
        let mut mock = crate::db::traits::MockServerRepo::new();
        mock.expect_get_agent_guidances()
            .returning(move |_agent_id, _step_id| Ok(rows.clone()));
        Arc::new(mock)
    }

    #[tokio::test]
    async fn no_guidance_passthrough() {
        let repo = mock_repo_with_guidances(vec![]);
        let filter = AgentGuidanceFilter::new(repo);
        let ctx = FilterContext::new("m", Uuid::new_v4());
        let (sys, _) = filter
            .on_start(&ctx, "Base prompt.".into(), vec![])
            .await
            .unwrap();
        assert_eq!(sys, "Base prompt.");
    }

    #[tokio::test]
    async fn appends_global_guidance() {
        let agent_id = Uuid::new_v4();
        let row = make_guidance(
            agent_id,
            serde_json::json!(["Always include sources", "Be concise"]),
        );
        let repo = mock_repo_with_guidances(vec![row]);
        let filter = AgentGuidanceFilter::new(repo);
        let ctx = FilterContext::new("m", agent_id);

        let (sys, _) = filter
            .on_start(&ctx, "Base prompt.".into(), vec![])
            .await
            .unwrap();
        assert!(sys.contains("<guidance>"));
        assert!(sys.contains("</guidance>"));
        assert!(sys.contains("Always include sources"));
        assert!(sys.contains("Be concise"));
    }

    #[tokio::test]
    async fn stacks_multiple_guidance_rows() {
        let agent_id = Uuid::new_v4();
        let row1 = make_guidance(agent_id, serde_json::json!(["Rule A"]));
        let row2 = make_guidance(agent_id, serde_json::json!(["Rule B", "Rule C"]));
        let repo = mock_repo_with_guidances(vec![row1, row2]);
        let filter = AgentGuidanceFilter::new(repo);
        let ctx = FilterContext::new("m", agent_id);

        let (sys, _) = filter.on_start(&ctx, "Base.".into(), vec![]).await.unwrap();
        assert!(sys.contains("Rule A"));
        assert!(sys.contains("Rule B"));
        assert!(sys.contains("Rule C"));
    }

    #[tokio::test]
    async fn empty_suggestions_array_passthrough() {
        let agent_id = Uuid::new_v4();
        let row = make_guidance(agent_id, serde_json::json!([]));
        let repo = mock_repo_with_guidances(vec![row]);
        let filter = AgentGuidanceFilter::new(repo);
        let ctx = FilterContext::new("m", agent_id);

        let (sys, _) = filter.on_start(&ctx, "Base.".into(), vec![]).await.unwrap();
        assert_eq!(sys, "Base.");
    }
}
