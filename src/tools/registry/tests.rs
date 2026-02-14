#[cfg(test)]
mod tests {
    use super::super::*;

    // ========================================================================
    // Registry Completeness Tests
    // ========================================================================

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
            "web_research",
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
    fn test_all_orchestrator_tools_mapped() {
        let orchestrator_tools = vec![
            "read_file",
            "list_files",
            "search_files",
            "think",
            "create_doc",
            "update_doc",
            "search_docs",
            "submit_prd",
            "submit_ticket",
        ];

        for tool_name in orchestrator_tools {
            let tool = get_tool_definition(tool_name);
            assert!(
                tool.is_some(),
                "Orchestrator tool '{}' not found in registry",
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
    fn test_all_documenter_assistant_tools_mapped() {
        let documenter_tools = vec![
            "create_doc_def",
            "update_doc_def",
            "delete_doc_def",
            "update_config",
        ];

        for tool_name in documenter_tools {
            let tool = get_tool_definition(tool_name);
            assert!(
                tool.is_some(),
                "Documenter assistant tool '{}' not found in registry",
                tool_name
            );
            assert_eq!(tool.unwrap().name, tool_name);
        }
    }

    #[test]
    fn test_tool_count() {
        // 23 unique tool names total (19 + 4 documenter assistant)
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
            "web_research",
            "search_files",
            "think",
            "create_doc",
            "update_doc",
            "search_docs",
            "submit_prd",
            "submit_ticket",
            "create_doc_def",
            "update_doc_def",
            "delete_doc_def",
            "update_config",
        ];

        assert_eq!(all_names.len(), 23);

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
            "web_research",
            "search_files",
            "think",
            "create_doc",
            "update_doc",
            "search_docs",
            "submit_prd",
            "submit_ticket",
            "create_doc_def",
            "update_doc_def",
            "delete_doc_def",
            "update_config",
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
    fn test_submit_prd_schema() {
        let tool = get_tool_definition("submit_prd").unwrap();
        assert_eq!(tool.name, "submit_prd");

        let props = tool.input_schema["properties"].as_object().unwrap();
        assert!(props.contains_key("title"));
        assert!(props.contains_key("problem_statement"));
        assert!(props.contains_key("goals"));
        assert!(props.contains_key("milestones"));
        assert!(props.contains_key("complexity"));

        // Verify complexity enum
        let complexity = &props["complexity"]["enum"];
        assert!(complexity.is_array());
        let enums: Vec<&str> = complexity
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v.as_str())
            .collect();
        assert_eq!(enums, vec!["S", "M", "L", "XL"]);
    }

    #[test]
    fn test_web_research_schema() {
        let tool = get_tool_definition("web_research").unwrap();
        assert_eq!(tool.name, "web_research");

        let props = tool.input_schema["properties"].as_object().unwrap();
        assert!(props.contains_key("query"));
        assert!(props.contains_key("sources"));
        assert!(props.contains_key("allowed_domains"));

        let required = tool.input_schema["required"].as_array().unwrap();
        assert_eq!(required.len(), 1);
        assert_eq!(required[0], "query");
    }

    #[test]
    fn test_think_tool_schema() {
        let tool = get_tool_definition("think").unwrap();
        assert_eq!(tool.name, "think");

        let props = tool.input_schema["properties"].as_object().unwrap();
        assert!(props.contains_key("thought"));
    }

    // ========================================================================
    // Documenter Assistant Tool Schema Tests
    // ========================================================================

    #[test]
    fn test_create_doc_def_schema() {
        let tool = get_tool_definition("create_doc_def").unwrap();
        assert_eq!(tool.name, "create_doc_def");

        let props = tool.input_schema["properties"].as_object().unwrap();
        assert!(props.contains_key("name"));
        assert!(props.contains_key("description"));
        assert!(props.contains_key("target_length"));

        let required = tool.input_schema["required"].as_array().unwrap();
        assert_eq!(required.len(), 1);
        assert_eq!(required[0], "name");
    }

    #[test]
    fn test_update_doc_def_schema() {
        let tool = get_tool_definition("update_doc_def").unwrap();
        assert_eq!(tool.name, "update_doc_def");

        let props = tool.input_schema["properties"].as_object().unwrap();
        assert!(props.contains_key("doc_def_id"));
        assert!(props.contains_key("name"));
        assert!(props.contains_key("description"));
        assert!(props.contains_key("target_length"));

        let required = tool.input_schema["required"].as_array().unwrap();
        assert_eq!(required.len(), 1);
        assert_eq!(required[0], "doc_def_id");
    }

    #[test]
    fn test_delete_doc_def_schema() {
        let tool = get_tool_definition("delete_doc_def").unwrap();
        assert_eq!(tool.name, "delete_doc_def");

        let props = tool.input_schema["properties"].as_object().unwrap();
        assert!(props.contains_key("doc_def_id"));

        let required = tool.input_schema["required"].as_array().unwrap();
        assert_eq!(required.len(), 1);
        assert_eq!(required[0], "doc_def_id");
    }

    #[test]
    fn test_update_config_schema() {
        let tool = get_tool_definition("update_config").unwrap();
        assert_eq!(tool.name, "update_config");

        let props = tool.input_schema["properties"].as_object().unwrap();
        assert!(props.contains_key("name"));
        assert!(props.contains_key("description"));
        assert!(props.contains_key("prompt_template"));

        let required = tool.input_schema["required"].as_array().unwrap();
        assert!(required.is_empty());
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
        assert!(tool.description.len() > 200, "render_panel description should be detailed");
    }
}
