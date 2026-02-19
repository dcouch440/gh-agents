#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use uuid::Uuid;

    use crate::db::{
        AgentRow, StepInputRow, StepOutputRow, StepRoutingRuleRow, WorkflowStepEdgeRow,
        WorkflowStepRow,
    };

    use super::super::{port_metadata_from_snapshot, WorkflowSnapshot};

    fn make_step(id: Uuid, workflow_id: Uuid) -> WorkflowStepRow {
        WorkflowStepRow {
            id,
            workflow_id,
            agent_id: Some(Uuid::new_v4()),
            prompt_template: "Test prompt".to_string(),
            output_variable_name: Some("output".to_string()),
            name: Some("Test Step".to_string()),
            description: "Test step description".to_string(),
            ..Default::default()
        }
    }

    fn make_edge(from_id: Uuid, to_id: Uuid, workflow_id: Uuid) -> WorkflowStepEdgeRow {
        WorkflowStepEdgeRow {
            id: Uuid::new_v4(),
            from_step_id: from_id,
            to_step_id: to_id,
            workflow_id,
            ..Default::default()
        }
    }

    fn make_snapshot() -> WorkflowSnapshot {
        let wf_id = Uuid::new_v4();
        let step1_id = Uuid::new_v4();
        let step2_id = Uuid::new_v4();
        let agent_id = Uuid::new_v4();

        let mut step1 = make_step(step1_id, wf_id);
        step1.agent_id = Some(agent_id);

        let step2 = make_step(step2_id, wf_id);
        let edge = make_edge(step1_id, step2_id, wf_id);

        let agent = AgentRow {
            id: agent_id,
            name: "Test Agent".to_string(),
            system_prompt: "You are a test agent.".to_string(),
            model_id: "claude-sonnet-4-5-20250929".to_string(),
            ..Default::default()
        };

        let mut agents = HashMap::new();
        agents.insert(agent_id, agent);

        let input = StepInputRow {
            id: Uuid::new_v4(),
            workflow_step_id: step2_id,
            port_name: "context".to_string(),
            port_type: "string".to_string(),
            required: true,
            description: Some("Input context".to_string()),
            ..Default::default()
        };

        let output = StepOutputRow {
            id: Uuid::new_v4(),
            workflow_step_id: step1_id,
            port_name: "result".to_string(),
            port_type: "string".to_string(),
            json_path: "$.output".to_string(),
            description: Some("Step output".to_string()),
            ..Default::default()
        };

        let mut step_inputs = HashMap::new();
        step_inputs.insert(step2_id, vec![input]);

        let mut step_outputs = HashMap::new();
        step_outputs.insert(step1_id, vec![output]);

        WorkflowSnapshot {
            steps: vec![step1, step2],
            edges: vec![edge],
            step_inputs,
            step_outputs,
            routing_rules: HashMap::new(),
            document_defs: HashMap::new(),
            protocols: HashMap::new(),
            room_configs: HashMap::new(),
            room_members: HashMap::new(),
            mission_briefs: HashMap::new(),
            agent_rosters: HashMap::new(),
            agents,
            agent_tools: HashMap::new(),
        }
    }

    #[test]
    fn test_snapshot_serialization_roundtrip() {
        let snapshot = make_snapshot();

        // Serialize
        let json = serde_json::to_value(&snapshot).expect("serialize");
        assert!(json.is_object());
        assert!(json["steps"].is_array());
        assert_eq!(json["steps"].as_array().unwrap().len(), 2);
        assert!(json["agents"].is_object());

        // Deserialize
        let recovered: WorkflowSnapshot = serde_json::from_value(json).expect("deserialize");
        assert_eq!(recovered.steps.len(), 2);
        assert_eq!(recovered.edges.len(), 1);
        assert_eq!(recovered.agents.len(), 1);
        assert_eq!(recovered.step_inputs.len(), 1);
        assert_eq!(recovered.step_outputs.len(), 1);

        // Verify field values survived the roundtrip
        assert_eq!(recovered.steps[0].prompt_template, "Test prompt");
        let agent = recovered.agents.values().next().unwrap();
        assert_eq!(agent.name, "Test Agent");
        assert_eq!(agent.model_id, "claude-sonnet-4-5-20250929");
    }

    #[test]
    fn test_port_metadata_from_snapshot() {
        let snapshot = make_snapshot();
        let port_meta = port_metadata_from_snapshot(&snapshot);

        // Verify port metadata was correctly built
        assert_eq!(port_meta.step_inputs.len(), 1);
        assert_eq!(port_meta.step_outputs.len(), 1);
        assert!(port_meta.routing_rules.is_empty());

        // Verify step input details
        let step2_id = snapshot.steps[1].id;
        let inputs = port_meta.step_inputs.get(&step2_id).unwrap();
        assert_eq!(inputs.len(), 1);
        assert_eq!(inputs[0].port_name, "context");
    }

    #[test]
    fn test_snapshot_with_routing_rules() {
        let mut snapshot = make_snapshot();
        let step_id = snapshot.steps[0].id;
        let agent_id = Uuid::new_v4();

        let rule = StepRoutingRuleRow {
            id: Uuid::new_v4(),
            workflow_step_id: step_id,
            label_value: "high_priority".to_string(),
            description: Some("Route high priority items".to_string()),
            agent_id,
            display_order: 0,
            created_at: chrono::Utc::now(),
        };

        snapshot.routing_rules.insert(step_id, vec![rule]);

        // Roundtrip
        let json = serde_json::to_value(&snapshot).expect("serialize");
        let recovered: WorkflowSnapshot = serde_json::from_value(json).expect("deserialize");

        assert_eq!(recovered.routing_rules.len(), 1);
        let rules = recovered.routing_rules.get(&step_id).unwrap();
        assert_eq!(rules[0].label_value, "high_priority");
        assert_eq!(rules[0].agent_id, agent_id);

        // Port metadata includes routing rules
        let port_meta = port_metadata_from_snapshot(&recovered);
        assert_eq!(port_meta.routing_rules.len(), 1);
    }

    #[test]
    fn test_empty_snapshot() {
        let snapshot = WorkflowSnapshot {
            steps: vec![],
            edges: vec![],
            step_inputs: HashMap::new(),
            step_outputs: HashMap::new(),
            routing_rules: HashMap::new(),
            document_defs: HashMap::new(),
            protocols: HashMap::new(),
            room_configs: HashMap::new(),
            room_members: HashMap::new(),
            mission_briefs: HashMap::new(),
            agent_rosters: HashMap::new(),
            agents: HashMap::new(),
            agent_tools: HashMap::new(),
        };

        let json = serde_json::to_value(&snapshot).expect("serialize");
        let recovered: WorkflowSnapshot = serde_json::from_value(json).expect("deserialize");
        assert!(recovered.steps.is_empty());
        assert!(recovered.agents.is_empty());
    }
}
