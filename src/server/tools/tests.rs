//! Tests for tool definitions

use super::*;

#[test]
fn agent_tools_returns_all_tools() {
    let tools = agent_tools();
    assert_eq!(tools.len(), 22);
    assert_eq!(tools[0].name, "list_agents");
    assert_eq!(tools[1].name, "list_roles");
    assert_eq!(tools[2].name, "create_agent");
    assert_eq!(tools[3].name, "create_agents");
    assert_eq!(tools[4].name, "assign_task");
    assert_eq!(tools[5].name, "get_task_result");
    assert_eq!(tools[6].name, "list_pending_approvals");
    assert_eq!(tools[7].name, "respond_to_approval");
    assert_eq!(tools[8].name, "remove_agent");
    assert_eq!(tools[9].name, "create_pipeline");
    assert_eq!(tools[10].name, "add_pipeline_stage");
    assert_eq!(tools[11].name, "start_pipeline");
    assert_eq!(tools[12].name, "get_pipeline_status");
    assert_eq!(tools[13].name, "read_file");
    assert_eq!(tools[14].name, "list_files");
    assert_eq!(tools[15].name, "search_files");
    assert_eq!(tools[17].name, "create_doc");
    assert_eq!(tools[18].name, "update_doc");
    assert_eq!(tools[19].name, "search_docs");
    assert_eq!(tools[20].name, "submit_prd");
    assert_eq!(tools[21].name, "submit_ticket");
}

#[test]
fn tool_schemas_are_valid_json() {
    for tool in agent_tools() {
        assert!(tool.input_schema.is_object());
        assert!(tool.input_schema["type"].as_str() == Some("object"));
    }
}
