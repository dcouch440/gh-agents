#[cfg(test)]
mod tests {
    use crate::server::hub::execution::strategies::react_designer::{
        build_roster_status_sync, format_roster_for_prompt,
    };

    fn make_roster_agent(name: &str, role: &str) -> crate::db::TaskAgentRosterRow {
        crate::db::TaskAgentRosterRow {
            id: uuid::Uuid::new_v4(),
            mission_brief_id: uuid::Uuid::new_v4(),
            name: name.to_string(),
            role_description: role.to_string(),
            capabilities: vec![],
            execution_order: 0,
            created_at: chrono::Utc::now(),
            child_step_id: None,
        }
    }

    #[test]
    fn roster_status_all_pending() {
        let roster = vec![
            make_roster_agent("Scanner", "scans code"),
            make_roster_agent("Analyzer", "analyzes findings"),
        ];

        let status = build_roster_status_sync(&roster);

        assert!(status.contains("· Scanner — pending"));
        assert!(status.contains("· Analyzer — pending"));
        assert!(status.contains("Designed: 0/2"));
    }

    #[test]
    fn format_roster_includes_name_and_role() {
        let roster = vec![make_roster_agent("Scanner", "Security scanner")];

        let text = format_roster_for_prompt(&roster);

        assert!(text.contains("Scanner"));
        assert!(text.contains("Security scanner"));
    }
}
