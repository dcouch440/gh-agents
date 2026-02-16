#[cfg(test)]
mod tests {
    use crate::server::api::agent_roster::RosterAgentResponse;

    #[test]
    fn roster_agent_response_from_row() {
        let now = chrono::Utc::now();
        let id = uuid::Uuid::new_v4();
        let brief_id = uuid::Uuid::new_v4();

        let row = crate::db::TaskAgentRosterRow {
            id,
            mission_brief_id: brief_id,
            name: "Planner".to_string(),
            role_description: "Plans the work".to_string(),
            capabilities: vec!["planning".to_string(), "analysis".to_string()],
            execution_order: 1,
            created_at: now,
            child_step_id: None,
        };

        let dep_id = uuid::Uuid::new_v4().to_string();
        let resp = RosterAgentResponse::from_row(row, vec![dep_id.clone()]);

        assert_eq!(resp.id, id.to_string());
        assert_eq!(resp.name, "Planner");
        assert_eq!(resp.role_description, "Plans the work");
        assert_eq!(resp.capabilities, vec!["planning", "analysis"]);
        assert_eq!(resp.execution_order, 1);
        assert_eq!(resp.created_at, now.to_rfc3339());
        assert_eq!(resp.child_step_id, None);
        assert_eq!(resp.depends_on, vec![dep_id]);
    }
}
