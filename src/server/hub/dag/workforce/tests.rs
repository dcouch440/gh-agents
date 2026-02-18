#[cfg(test)]
mod tests {
    use crate::db::TaskAgentRosterRow;
    use crate::server::hub::dag::workforce::{
        build_filtered_outputs_block, build_team_roster_string, compose_workforce_output,
        filter_outputs_for_agent,
    };
    use chrono::Utc;
    use uuid::Uuid;

    fn make_roster_agent(name: &str, order: i32) -> TaskAgentRosterRow {
        TaskAgentRosterRow {
            id: Uuid::new_v4(),
            mission_brief_id: Uuid::new_v4(),
            name: name.to_string(),
            role_description: format!("{} role", name),
            capabilities: vec!["file_read".to_string()],
            execution_order: order,
            created_at: Utc::now(),
            child_step_id: None,
        }
    }

    #[test]
    fn compose_workforce_output_includes_agents() {
        let agent1 = make_roster_agent("Scanner", 0);
        let agent2 = make_roster_agent("Writer", 1);
        let roster = vec![agent1.clone(), agent2.clone()];

        let agent_outputs = vec![
            ("Scanner".to_string(), "scan results".to_string()),
            ("Writer".to_string(), "written docs".to_string()),
        ];

        let result = compose_workforce_output(&agent_outputs, &roster);

        assert!(result["agents"]["scanner"].is_string());
        assert_eq!(result["agents"]["scanner"], "scan results");
        assert_eq!(result["agents"]["writer"], "written docs");
    }

    #[test]
    fn filter_outputs_empty_receives_from_returns_all() {
        let outputs = vec![
            ("A".to_string(), "a_out".to_string()),
            ("B".to_string(), "b_out".to_string()),
        ];
        let filtered = filter_outputs_for_agent(&outputs, &[]);
        assert_eq!(filtered.len(), 2);
    }

    #[test]
    fn filter_outputs_with_receives_from_filters() {
        let outputs = vec![
            ("Scanner".to_string(), "scan".to_string()),
            ("Writer".to_string(), "write".to_string()),
        ];
        let filtered = filter_outputs_for_agent(&outputs, &["Scanner".to_string()]);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].0, "Scanner");
    }

    #[test]
    fn team_roster_string_includes_agents() {
        let agent = make_roster_agent("Scanner", 0);
        let roster = vec![agent.clone()];

        let result = build_team_roster_string(&roster);
        assert!(result.contains("Scanner"));
        assert!(result.contains("file_read"));
    }

    #[test]
    fn build_filtered_outputs_block_empty() {
        let result = build_filtered_outputs_block(&[]);
        assert!(result.contains("No previous agent outputs"));
    }

    #[test]
    fn build_filtered_outputs_block_with_outputs() {
        let outputs = vec![
            ("Agent A".to_string(), "output a".to_string()),
            ("Agent B".to_string(), "output b".to_string()),
        ];
        let refs: Vec<&(String, String)> = outputs.iter().collect();
        let result = build_filtered_outputs_block(&refs);
        assert!(result.contains("### Agent A"));
        assert!(result.contains("output a"));
        assert!(result.contains("### Agent B"));
    }
}
