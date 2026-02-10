#[cfg(test)]
mod tests {
    //! Tests for tool definitions

    use super::super::*;

    #[test]
    fn agent_tools_returns_all_tools() {
        let tools = agent_tools();
        // LEGACY tools removed: list_agents, list_roles, create_agent, create_agents, assign_task,
        // get_task_result, list_pending_approvals, respond_to_approval, remove_agent,
        // create_pipeline, add_pipeline_stage, start_pipeline, get_pipeline_status
        // Use chat/session API and workflow system instead.
        assert_eq!(tools.len(), 9);
        assert_eq!(tools[0].name, "read_file");
        assert_eq!(tools[1].name, "list_files");
        assert_eq!(tools[2].name, "search_files");
        assert_eq!(tools[3].name, "think");
        assert_eq!(tools[4].name, "create_doc");
        assert_eq!(tools[5].name, "update_doc");
        assert_eq!(tools[6].name, "search_docs");
        assert_eq!(tools[7].name, "submit_prd");
        assert_eq!(tools[8].name, "submit_ticket");
    }

    #[test]
    fn tool_schemas_are_valid_json() {
        for tool in agent_tools() {
            assert!(tool.input_schema.is_object());
            assert!(tool.input_schema["type"].as_str() == Some("object"));
        }
    }
}
