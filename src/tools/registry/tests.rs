#[cfg(test)]
mod tests {
    use super::super::*;

    // ========================================================================
    // Registry Completeness Tests
    // ========================================================================

    /// The worked examples are deliberate few-shots — they are what stops an
    /// agent computing in its head instead of shelling out, so there is no
    /// size budget here. Two framings are excluded, though: anything selling
    /// stdout as a destination ("no file needed"), and anything teaching the
    /// agent to swallow errors, which blinds the diagnostics engine.
    #[test]
    fn run_command_examples_do_not_teach_against_the_deliverable_model() {
        let tool = get_tool_definition("run_command").expect("run_command is registered");
        assert!(!tool.description.contains("no file needed"));
        assert!(!tool.description.contains("2>/dev/null || true"));
        // Inline interpreters stay — they are the cure for mental arithmetic.
        assert!(tool.description.contains("python -c"));
    }

    /// The old description ended with "Do not verify file creation with ls".
    /// That trained agents away from the file discipline the runtime prompt
    /// now depends on.
    /// The `command` parameter description talks *about* escape sequences, so
    /// its literal must stay raw. De-escaping it once turned "use real newlines
    /// (\\n), NOT literal backslash-n (\\\\n)" into an embedded newline plus the
    /// opposite advice.
    #[test]
    fn run_command_param_description_keeps_its_escapes_literal() {
        let tool = get_tool_definition("run_command").expect("run_command is registered");
        let desc = tool.input_schema["properties"]["command"]["description"]
            .as_str()
            .expect("command parameter has a description");

        assert!(
            !desc.contains('\n'),
            "description must not embed a real newline"
        );
        assert!(desc.contains(r"(\n)"));
        assert!(desc.contains(r"(\\n)"));
    }

    /// The description's first worked example used to be a heredoc, and its
    /// "File operations" list led with `cat > file << 'EOF'`. That made the
    /// shell the strongest signal for file creation even after the file tools
    /// became default, which is how an 816-line spec ended up written three
    /// times through a channel bounded by max_tokens.
    #[test]
    fn run_command_points_at_the_file_tools_before_it_shows_a_heredoc() {
        let tool = get_tool_definition("run_command").expect("run_command is registered");
        let desc = &tool.description;

        let pointer = desc
            .find("write_file")
            .expect("run_command must name write_file");
        let heredoc = desc
            .find("<< 'EOF'")
            .expect("the heredoc fallback example is still wanted");
        assert!(
            pointer < heredoc,
            "the file tools must be named before the first heredoc"
        );
        assert!(
            !desc.contains("sed -i 's/old/new/g'"),
            "sed -i must not be presented as the way to edit a file"
        );
        assert!(
            desc.contains("always close it"),
            "the unterminated-heredoc guard needs a matching line in the description"
        );
    }

    /// These four carry usage doctrine now, not one-line labels.
    /// `run_command`'s ~67-line description was the only thing an agent had to
    /// reason from, so it won every time.
    #[test]
    fn the_file_tools_carry_usage_doctrine() {
        for name in ["read_file", "write_file", "edit_file", "list_files"] {
            let tool = get_tool_definition(name).expect("registered");
            assert!(
                tool.description.len() > 200,
                "{name} description is still a label"
            );
        }

        let write = get_tool_definition("write_file").unwrap();

        // Derived from the config rather than hardcoded. This assertion used to
        // pin the literal "8,000 tokens" and went stale the moment the runtime
        // agent's cap was raised to 32k — the description then understated the
        // real ceiling by 4x, which is the direction that makes an agent chunk a
        // file it could have written in one call.
        let cap = crate::config::protocols::WORKFORCE
            .agents
            .get("agent")
            .expect("runtime agent config")
            .max_tokens;
        let rounded = format!("{},000 tokens", cap / 1000 / 10 * 10);
        assert!(
            write.description.contains(&rounded),
            "write_file must surface the max_tokens ceiling ({cap}); expected {rounded:?}"
        );
        assert!(
            write.description.contains("edit_file"),
            "write_file must name the chunked-append escape hatch"
        );
        assert!(
            write.description.contains("several files"),
            "write_file must offer splitting, not only appending — appending is what \
             produces one enormous file"
        );

        let edit = get_tool_definition("edit_file").unwrap();
        assert!(edit.description.contains("empty old_string"));
    }

    /// `read_only` is only as real as this list. A denylist would silently
    /// admit every tool added after it was written.
    #[test]
    fn the_read_only_set_excludes_every_writing_tool() {
        use crate::tools::registry::is_read_only_tool;

        for writer in [
            "write_file",
            "edit_file",
            "run_command",
            "run_tests",
            "git_add",
            "git_commit",
            "create_doc",
            "update_doc",
        ] {
            assert!(!is_read_only_tool(writer), "{writer} must not be read-only");
        }
        for reader in ["read_file", "list_files", "git_status", "brave_search"] {
            assert!(is_read_only_tool(reader), "{reader} should be read-only");
        }
    }

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
            // Web tools
            "brave_search",
            "read_webpage",
        ];

        assert_eq!(all_names.len(), 24);

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
            // Web tools
            "brave_search",
            "read_webpage",
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
