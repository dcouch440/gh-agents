#[cfg(test)]
mod tests {
    use super::super::*;

    // ========================================================================
    // Registry Completeness Tests
    // ========================================================================

    /// `run_command`'s description was once ~3.4KB of shell tutorial sent to
    /// every agent on every round. Keep it from silently re-inflating.
    #[test]
    fn run_command_description_stays_lean() {
        let tool = get_tool_definition("run_command").expect("run_command is registered");
        assert!(
            tool.description.len() <= 800,
            "run_command description is {} bytes — budget is 800. Agents already \
             know how to use a shell; add environment facts, not shell lessons.",
            tool.description.len()
        );
    }

    /// The old description ended with "Do not verify file creation with ls".
    /// That trained agents away from the file discipline the runtime prompt
    /// now depends on.
    #[test]
    fn run_command_does_not_discourage_file_checks() {
        let tool = get_tool_definition("run_command").expect("run_command is registered");
        assert!(!tool.description.contains("Do not verify file creation"));
    }

    #[test]
    fn test_all_execution_tools_mapped() {
        let execution_tools = vec![
            "read_file",
            "write_file",
            "edit_file",
            "list_files",
            "git_status",
            "git_diff",
            "git_add",
            "git_commit",
            "git_branch",
            "run_tests",
            "run_command",
        ];

        for tool_name in execution_tools {
            let tool = get_tool_definition(tool_name);
            assert!(
                tool.is_some(),
                "Execution tool '{}' not found in registry",
                tool_name
            );
            assert_eq!(tool.unwrap().name, tool_name);
        }
    }

    #[test]
    fn test_unknown_tool_returns_none() {
        assert!(get_tool_definition("unknown_tool").is_none());
        assert!(get_tool_definition("").is_none());
        assert!(get_tool_definition("bash").is_none());
    }

    #[test]
    fn test_tool_count() {
        let all_names = vec![
            "read_file",
            "write_file",
            "edit_file",
            "list_files",
            "git_status",
            "git_diff",
            "git_add",
            "git_commit",
            "git_branch",
            "run_tests",
            "run_command",
            "read_document",
            "think",
            "create_doc",
            "update_doc",
            "search_docs",
            // Manager topology tools
            "create_pipeline",
            "create_parallel",
            "insert_node",
            "remove_node",
            "wire_edge",
            "remove_edge",
        ];

        assert_eq!(all_names.len(), 22);

        // Verify all map to tools
        for name in all_names {
            assert!(
                get_tool_definition(name).is_some(),
                "Tool {} not found",
                name
            );
        }
    }

    // ========================================================================
    // Schema Validation Tests
    // ========================================================================

    #[test]
    fn test_all_tool_schemas_valid() {
        let all_tools = vec![
            "read_file",
            "write_file",
            "edit_file",
            "list_files",
            "git_status",
            "git_diff",
            "git_add",
            "git_commit",
            "git_branch",
            "run_tests",
            "run_command",
            "read_document",
            "think",
            "create_doc",
            "update_doc",
            "search_docs",
            // Manager topology tools
            "create_pipeline",
            "create_parallel",
            "insert_node",
            "remove_node",
            "wire_edge",
            "remove_edge",
        ];

        for tool_name in all_tools {
            let tool = get_tool_definition(tool_name).unwrap();

            // Verify basic fields
            assert!(!tool.name.is_empty(), "{}: name is empty", tool_name);
            assert!(
                !tool.description.is_empty(),
                "{}: description is empty",
                tool_name
            );

            // Verify schema is an object
            assert!(
                tool.input_schema.is_object(),
                "{}: schema is not an object",
                tool_name
            );

            // Verify schema has type: object
            assert_eq!(
                tool.input_schema.get("type").and_then(|v| v.as_str()),
                Some("object"),
                "{}: schema type is not 'object'",
                tool_name
            );

            // Verify schema has properties
            assert!(
                tool.input_schema.get("properties").is_some(),
                "{}: schema missing 'properties' field",
                tool_name
            );
        }
    }

    // ========================================================================
    // Individual Tool Tests
    // ========================================================================

    #[test]
    fn test_read_file_schema() {
        let tool = get_tool_definition("read_file").unwrap();
        assert_eq!(tool.name, "read_file");

        let props = tool.input_schema["properties"].as_object().unwrap();
        assert!(props.contains_key("path"));

        let required = tool.input_schema["required"].as_array().unwrap();
        assert_eq!(required.len(), 1);
        assert_eq!(required[0], "path");
    }

    #[test]
    fn test_edit_file_schema() {
        let tool = get_tool_definition("edit_file").unwrap();
        assert_eq!(tool.name, "edit_file");

        let props = tool.input_schema["properties"].as_object().unwrap();
        assert!(props.contains_key("path"));
        assert!(props.contains_key("old_string"));
        assert!(props.contains_key("new_string"));

        let required = tool.input_schema["required"].as_array().unwrap();
        assert_eq!(required.len(), 3);
    }

    #[test]
    fn test_git_add_schema() {
        let tool = get_tool_definition("git_add").unwrap();
        assert_eq!(tool.name, "git_add");

        let props = tool.input_schema["properties"].as_object().unwrap();
        assert!(props.contains_key("paths"));

        let paths_schema = &props["paths"];
        assert_eq!(paths_schema["type"], "array");
    }

    #[test]
    fn test_think_tool_schema() {
        let tool = get_tool_definition("think").unwrap();
        assert_eq!(tool.name, "think");

        let props = tool.input_schema["properties"].as_object().unwrap();
        assert!(props.contains_key("thought"));
    }

    #[test]
    fn test_read_document_schema() {
        let tool = get_tool_definition("read_document").unwrap();
        assert_eq!(tool.name, "read_document");

        let props = tool.input_schema["properties"].as_object().unwrap();
        assert!(props.contains_key("document_id"));

        let required = tool.input_schema["required"].as_array().unwrap();
        assert_eq!(required.len(), 1);
        assert_eq!(required[0], "document_id");
    }

    #[test]
    fn test_render_panel_schema() {
        let tool = get_tool_definition("render_panel").unwrap();
        assert_eq!(tool.name, "render_panel");

        let props = tool.input_schema["properties"].as_object().unwrap();
        assert!(props.contains_key("content"));
        assert!(props.contains_key("submit_label"));

        let required = tool.input_schema["required"].as_array().unwrap();
        assert_eq!(required.len(), 1);
        assert_eq!(required[0], "content");

        // Verify description is detailed (Anthropic best practice)
        assert!(
            tool.description.len() > 200,
            "render_panel description should be detailed"
        );
    }

    #[test]
    fn test_update_plan_schema() {
        let tool = get_tool_definition("update_plan").unwrap();
        assert_eq!(tool.name, "update_plan");

        let props = tool.input_schema["properties"].as_object().unwrap();
        assert!(props.contains_key("content"));

        let required = tool.input_schema["required"].as_array().unwrap();
        assert_eq!(required.len(), 1);
        assert_eq!(required[0], "content");
    }

    #[test]
    fn test_dispatch_to_nodes_schema() {
        let tool = get_tool_definition("dispatch_to_nodes").unwrap();
        assert_eq!(tool.name, "dispatch_to_nodes");

        let props = tool.input_schema["properties"].as_object().unwrap();
        assert!(props.contains_key("messages"));

        let messages_schema = &props["messages"];
        assert_eq!(messages_schema["type"], "array");

        let item_props = messages_schema["items"]["properties"].as_object().unwrap();
        assert!(item_props.contains_key("node"));
        assert!(item_props.contains_key("message_type"));
        assert!(item_props.contains_key("content"));

        let item_required = messages_schema["items"]["required"].as_array().unwrap();
        assert_eq!(item_required.len(), 3);

        let required = tool.input_schema["required"].as_array().unwrap();
        assert_eq!(required.len(), 1);
        assert_eq!(required[0], "messages");
    }

    #[test]
    fn test_send_message_schema() {
        let tool = get_tool_definition("send_message").unwrap();
        assert_eq!(tool.name, "send_message");

        let props = tool.input_schema["properties"].as_object().unwrap();
        assert!(props.contains_key("step_id"));
        assert!(props.contains_key("message_type"));
        assert!(props.contains_key("content"));

        let required = tool.input_schema["required"].as_array().unwrap();
        assert_eq!(required.len(), 3);
    }

    // ========================================================================
    // Manager Topology Tool Tests
    // ========================================================================

    #[test]
    fn test_create_pipeline_schema() {
        let tool = get_tool_definition("create_pipeline").unwrap();
        assert_eq!(tool.name, "create_pipeline");

        let props = tool.input_schema["properties"].as_object().unwrap();
        assert!(props.contains_key("source"));
        assert!(props.contains_key("nodes"));

        let nodes_schema = &props["nodes"];
        assert_eq!(nodes_schema["type"], "array");
        assert_eq!(nodes_schema["minItems"], 1);

        let item_props = nodes_schema["items"]["properties"].as_object().unwrap();
        assert!(item_props.contains_key("name"));
        assert!(item_props.contains_key("description"));

        let required = tool.input_schema["required"].as_array().unwrap();
        assert_eq!(required.len(), 1);
        assert_eq!(required[0], "nodes");
    }

    #[test]
    fn test_create_parallel_schema() {
        let tool = get_tool_definition("create_parallel").unwrap();
        assert_eq!(tool.name, "create_parallel");

        let props = tool.input_schema["properties"].as_object().unwrap();
        assert!(props.contains_key("source"));
        assert!(props.contains_key("parallel"));
        assert!(props.contains_key("target"));

        let parallel_schema = &props["parallel"];
        assert_eq!(parallel_schema["type"], "array");
        assert_eq!(parallel_schema["minItems"], 2);

        let required = tool.input_schema["required"].as_array().unwrap();
        assert_eq!(required.len(), 1);
        assert_eq!(required[0], "parallel");
    }

    #[test]
    fn test_insert_node_schema() {
        let tool = get_tool_definition("insert_node").unwrap();
        assert_eq!(tool.name, "insert_node");

        let props = tool.input_schema["properties"].as_object().unwrap();
        assert!(props.contains_key("from"));
        assert!(props.contains_key("to"));
        assert!(props.contains_key("node"));

        let node_props = props["node"]["properties"].as_object().unwrap();
        assert!(node_props.contains_key("name"));
        assert!(node_props.contains_key("description"));

        let required = tool.input_schema["required"].as_array().unwrap();
        assert_eq!(required.len(), 3);
    }

    #[test]
    fn test_remove_node_schema() {
        let tool = get_tool_definition("remove_node").unwrap();
        assert_eq!(tool.name, "remove_node");

        let props = tool.input_schema["properties"].as_object().unwrap();
        assert!(props.contains_key("node"));
        assert!(props.contains_key("reconnect"));
        assert_eq!(props["reconnect"]["type"], "boolean");

        let required = tool.input_schema["required"].as_array().unwrap();
        assert_eq!(required.len(), 1);
        assert_eq!(required[0], "node");
    }

    #[test]
    fn test_wire_edge_schema() {
        let tool = get_tool_definition("wire_edge").unwrap();
        assert_eq!(tool.name, "wire_edge");

        let props = tool.input_schema["properties"].as_object().unwrap();
        assert!(props.contains_key("from"));
        assert!(props.contains_key("to"));

        let required = tool.input_schema["required"].as_array().unwrap();
        assert_eq!(required.len(), 2);
    }

    #[test]
    fn test_remove_edge_schema() {
        let tool = get_tool_definition("remove_edge").unwrap();
        assert_eq!(tool.name, "remove_edge");

        let props = tool.input_schema["properties"].as_object().unwrap();
        assert!(props.contains_key("from"));
        assert!(props.contains_key("to"));

        let required = tool.input_schema["required"].as_array().unwrap();
        assert_eq!(required.len(), 2);
    }

    #[test]
    fn test_all_manager_topology_tools_mapped() {
        let topology_tools = vec![
            "create_pipeline",
            "create_parallel",
            "insert_node",
            "remove_node",
            "wire_edge",
            "remove_edge",
        ];

        for tool_name in topology_tools {
            let tool = get_tool_definition(tool_name);
            assert!(
                tool.is_some(),
                "Manager topology tool '{}' not found in registry",
                tool_name
            );
            assert_eq!(tool.unwrap().name, tool_name);
        }
    }
}
