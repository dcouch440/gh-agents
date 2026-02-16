#[cfg(test)]
mod tests {
    use crate::db::{ProtocolDocumentDefRow, TaskAgentRosterRow};
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

    fn make_doc_def(name: &str, agent_id: Option<Uuid>) -> ProtocolDocumentDefRow {
        ProtocolDocumentDefRow {
            id: Uuid::new_v4(),
            step_id: Some(Uuid::new_v4()),
            name: name.to_string(),
            description: format!("{} description", name),
            target_length: 2000,
            display_order: 0,
            created_at: Utc::now(),
            protocol_id: None,
            document_id: None,
            agent_roster_entry_id: agent_id,
        }
    }

    #[test]
    fn compose_workforce_output_includes_agents_and_deliverables() {
        let agent1 = make_roster_agent("Scanner", 0);
        let agent2 = make_roster_agent("Writer", 1);
        let roster = vec![agent1.clone(), agent2.clone()];

        let doc1 = make_doc_def("Codebase Analysis", Some(agent1.id));
        let doc2 = make_doc_def("API Specs", Some(agent2.id));
        let doc_defs = vec![doc1, doc2];

        let agent_outputs = vec![
            ("Scanner".to_string(), "scan results".to_string()),
            ("Writer".to_string(), "written docs".to_string()),
        ];

        let result = compose_workforce_output(&agent_outputs, &roster, &doc_defs);

        // Check agents section
        assert!(result["agents"]["scanner"].is_string());
        assert_eq!(result["agents"]["scanner"], "scan results");
        assert_eq!(result["agents"]["writer"], "written docs");

        // Check deliverables section
        let deliverables = result["deliverables"].as_array().unwrap();
        assert_eq!(deliverables.len(), 2);
        assert_eq!(deliverables[0]["name"], "Codebase Analysis");
        assert_eq!(deliverables[0]["assigned_to"], "Scanner");
        assert_eq!(deliverables[1]["name"], "API Specs");
        assert_eq!(deliverables[1]["assigned_to"], "Writer");
    }

    #[test]
    fn compose_workforce_output_handles_unassigned_deliverables() {
        let roster = vec![make_roster_agent("Scanner", 0)];
        let doc_defs = vec![make_doc_def("Orphan Doc", None)];
        let agent_outputs = vec![("Scanner".to_string(), "output".to_string())];

        let result = compose_workforce_output(&agent_outputs, &roster, &doc_defs);
        let deliverables = result["deliverables"].as_array().unwrap();
        assert_eq!(deliverables[0]["assigned_to"], "unassigned");
    }

    #[test]
    fn compose_workforce_output_no_deliverables() {
        let roster = vec![make_roster_agent("Scanner", 0)];
        let agent_outputs = vec![("Scanner".to_string(), "output".to_string())];

        let result = compose_workforce_output(&agent_outputs, &roster, &[]);
        assert!(result.get("deliverables").is_none());
        assert!(result["agents"]["scanner"].is_string());
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
    fn team_roster_string_includes_deliverables() {
        let agent = make_roster_agent("Scanner", 0);
        let roster = vec![agent.clone()];
        let doc = make_doc_def("Analysis", Some(agent.id));

        let result = build_team_roster_string(&roster, &[doc]);
        assert!(result.contains("Scanner"));
        assert!(result.contains("Analysis"));
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
