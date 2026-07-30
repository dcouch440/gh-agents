#[cfg(test)]
mod tests {
    use crate::server::services::canvas_sync::filesystem;
    use tempfile::TempDir;

    #[test]
    fn write_and_remove_node_file() {
        let tmp = TempDir::new().unwrap();
        let base = tmp.path();

        // Write
        filesystem::write_node_file(base, "research", "# Research\n\nDo research.").unwrap();
        let content = std::fs::read_to_string(base.join("nodes/research.md")).unwrap();
        assert_eq!(content, "# Research\n\nDo research.");

        // Remove
        filesystem::remove_node_file(base, "research").unwrap();
        assert!(!base.join("nodes/research.md").exists());
    }

    #[test]
    fn rewrite_topology_from_steps_and_edges() {
        let tmp = TempDir::new().unwrap();
        let base = tmp.path();

        let step_a = crate::db::WorkflowStepRow {
            id: uuid::Uuid::new_v4(),
            workflow_id: uuid::Uuid::new_v4(),
            execution_mode: "workforce".to_string(),
            ref_id: Some("research".to_string()),
            ..make_empty_step()
        };
        let step_b = crate::db::WorkflowStepRow {
            id: uuid::Uuid::new_v4(),
            workflow_id: step_a.workflow_id,
            execution_mode: "workforce".to_string(),
            ref_id: Some("report".to_string()),
            ..make_empty_step()
        };

        let edge = crate::db::WorkflowStepEdgeRow {
            id: uuid::Uuid::new_v4(),
            workflow_id: step_a.workflow_id,
            from_step_id: step_a.id,
            to_step_id: step_b.id,
            from_output_port: None,
            to_input_port: None,
            transform_jsonpath: None,
            condition_type: None,
            condition_value: None,
            edge_label: None,
        };

        filesystem::rewrite_topology(base, &[step_a, step_b], &[edge]).unwrap();

        let content = std::fs::read_to_string(base.join("topology.json")).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        let nodes = parsed["nodes"].as_object().unwrap();
        assert!(nodes.contains_key("research"));
        assert!(nodes.contains_key("report"));

        let report_deps = nodes["report"]["depends_on"].as_array().unwrap();
        assert_eq!(report_deps.len(), 1);
        assert_eq!(report_deps[0].as_str().unwrap(), "research");
    }

    fn make_empty_step() -> crate::db::WorkflowStepRow {
        crate::db::WorkflowStepRow {
            id: uuid::Uuid::new_v4(),
            workflow_id: uuid::Uuid::new_v4(),
            agent_id: None,
            execution_mode: String::new(),
            agent_execution_mode: None,
            for_each_ref: None,
            prompt_template_id: None,
            prompt_template: String::new(),
            output_schema_id: None,
            output_variable_name: None,
            interactive_agent_id: None,
            for_each_label_field: None,
            room_id: None,
            routing_mode: None,
            routing_field: None,
            display_order: 0,
            version: 1,
            reasoning_trace: false,
            verification_agent_ids: None,
            position_x: None,
            position_y: None,
            width: None,
            height: None,
            name: None,
            system_prompt_suffix: None,
            visible: true,
            description: String::new(),
            board_context_cache: String::new(),
            board_context_updated_at: None,
            goal_summary: String::new(),
            goal_summary_updated_at: None,
            child_workflow_id: None,
            ref_id: None,
            pinned: false,
            run_results_summary: String::new(),
            designer_handoff: String::new(),
        }
    }
}
