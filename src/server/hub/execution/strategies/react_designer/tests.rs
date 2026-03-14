#[cfg(test)]
mod tests {
    use crate::server::hub::board_state::types::AgentDesignStatus;
    use crate::server::hub::board_state::{AgentSnapshot, BoardSnapshot, NodeSnapshot};

    #[test]
    fn enrichment_marks_changed_agents_pending() {
        let mut snapshot = make_snapshot(vec!["Scanner", "Analyzer"]);

        // Simulate enrichment: Scanner was changed, Analyzer was not
        for node in &mut snapshot.nodes {
            for agent in &mut node.agents {
                if agent.name == "Scanner" {
                    agent.design_status = AgentDesignStatus::Pending;
                } else {
                    agent.design_status = AgentDesignStatus::Designed {
                        version: 1,
                        config_path: "design/abc/agents/analyzer.json".to_string(),
                    };
                }
            }
        }

        let scanner = &snapshot.nodes[0].agents[0];
        let analyzer = &snapshot.nodes[0].agents[1];

        assert!(matches!(scanner.design_status, AgentDesignStatus::Pending));
        assert!(matches!(
            analyzer.design_status,
            AgentDesignStatus::Designed { version: 1, .. }
        ));
    }

    fn make_snapshot(agent_names: Vec<&str>) -> BoardSnapshot {
        let agents: Vec<AgentSnapshot> = agent_names
            .into_iter()
            .map(|name| AgentSnapshot {
                id: uuid::Uuid::new_v4(),
                name: name.to_string(),
                role_description: format!("{} agent", name),
                capabilities: vec![],
                receives_from: vec![],
                design_status: AgentDesignStatus::default(),
            })
            .collect();

        BoardSnapshot {
            workflow_name: String::new(),
            workflow_id: uuid::Uuid::new_v4(),
            nodes: vec![NodeSnapshot {
                id: uuid::Uuid::new_v4(),
                ref_id: None,
                name: "Test Node".to_string(),
                protocol: "workforce".to_string(),
                status: "configured".to_string(),
                task: "Test task".to_string(),
                capabilities: vec![],
                failure_mode: String::new(),
                summary: format!("{} agents", agents.len()),
                compressed_status: None,
                agents,
                input_ports: vec![],
                output_ports: vec![],
                incoming_context: vec![],
                plan: String::new(),
                asking: None,
                receives: None,
                initial_instructions_sent: false,
                node_text: String::new(),
            }],
            available_capabilities: vec![],
        }
    }
}
