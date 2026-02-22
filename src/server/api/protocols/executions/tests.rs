#[cfg(test)]
mod tests {
    use chrono::Utc;
    use uuid::Uuid;

    use crate::db::ProtocolExecutionRow;
    use crate::server::api::protocols::executions::ProtocolExecutionResponse;

    #[test]
    fn response_from_row_complete() {
        let step_id = Uuid::new_v4();
        let run_id = Uuid::new_v4();
        let now = Utc::now();

        let row = ProtocolExecutionRow {
            id: Uuid::new_v4(),
            protocol_step_id: step_id,
            workflow_run_id: Some(run_id),
            phase: "strategy".to_string(),
            document_def_id: None,
            agent_id: None,
            input_prompt: Some("Plan documents".to_string()),
            output_content: Some("{\"plans\":[]}".to_string()),
            status: "complete".to_string(),
            error_message: None,
            tokens_in: Some(150),
            tokens_out: Some(200),
            cost_usd: Some(0.005),
            model: Some("claude-sonnet-4-5-20250929".to_string()),
            capabilities_used: Some(vec!["file_read".to_string()]),
            created_at: now,
            completed_at: Some(now),
            agent_name: None,
            archetype: None,
            designer_run_id: None,
        };

        let resp = ProtocolExecutionResponse::from_row(row);
        assert_eq!(resp.protocol_step_id, step_id.to_string());
        assert_eq!(resp.workflow_run_id, Some(run_id.to_string()));
        assert_eq!(resp.phase, "strategy");
        assert_eq!(resp.status, "complete");
        assert_eq!(resp.tokens_in, Some(150));
        assert_eq!(resp.tokens_out, Some(200));
        assert!(resp.completed_at.is_some());
    }

    #[test]
    fn response_from_row_minimal() {
        let row = ProtocolExecutionRow {
            id: Uuid::new_v4(),
            protocol_step_id: Uuid::new_v4(),
            workflow_run_id: None,
            phase: "research".to_string(),
            document_def_id: None,
            agent_id: None,
            input_prompt: None,
            output_content: None,
            status: "pending".to_string(),
            error_message: None,
            tokens_in: None,
            tokens_out: None,
            cost_usd: None,
            model: None,
            capabilities_used: None,
            created_at: Utc::now(),
            completed_at: None,
            agent_name: None,
            archetype: None,
            designer_run_id: None,
        };

        let resp = ProtocolExecutionResponse::from_row(row);
        assert_eq!(resp.phase, "research");
        assert_eq!(resp.status, "pending");
        assert!(resp.workflow_run_id.is_none());
        assert!(resp.completed_at.is_none());
        assert!(resp.tokens_in.is_none());
    }
}
