#[cfg(test)]
mod tests {
    use serde_json::json;
    use tempfile::TempDir;

    use crate::server::tools::execution::{
        builtin_tool_rows, execute_execution_tool, execution_tools, ExecutionContext,
    };

    #[test]
    fn tool_schemas_are_valid() {
        let tools = execution_tools();
        assert_eq!(tools.len(), 15);
        for tool in &tools {
            assert!(!tool.name.is_empty());
            assert!(!tool.description.is_empty());
            assert!(tool.input_schema.is_object());
        }
    }

    #[test]
    fn tool_names_are_unique() {
        let tools = execution_tools();
        let mut names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), tools.len());
    }

    #[test]
    fn builtin_tool_rows_returns_15() {
        let rows = builtin_tool_rows();
        assert_eq!(rows.len(), 15);
        for row in &rows {
            assert!(!row.name.is_empty());
            assert!(!row.display_name.is_empty());
            assert!(!row.description.is_empty());
        }
    }

    #[test]
    fn builtin_tool_rows_have_unique_ids() {
        let rows = builtin_tool_rows();
        let mut ids: Vec<_> = rows.iter().map(|r| r.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), rows.len());
    }

    #[test]
    fn builtin_tool_rows_are_deterministic() {
        let a = builtin_tool_rows();
        let b = builtin_tool_rows();
        for (ra, rb) in a.iter().zip(b.iter()) {
            assert_eq!(ra.id, rb.id);
            assert_eq!(ra.name, rb.name);
        }
    }

    #[tokio::test]
    async fn read_file_tool_works() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("test.txt"), "hello world").unwrap();
        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let result =
            execute_execution_tool("read_file", &json!({ "path": "test.txt" }), &ctx, None).await;
        assert_eq!(result["content"], "hello world");
    }

    #[tokio::test]
    async fn write_file_tool_works() {
        let tmp = TempDir::new().unwrap();
        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let result = execute_execution_tool(
            "write_file",
            &json!({ "path": "out.txt", "content": "written" }),
            &ctx,
            None,
        )
        .await;
        assert_eq!(result["success"], true);
        let content = std::fs::read_to_string(tmp.path().join("out.txt")).unwrap();
        assert_eq!(content, "written");
    }

    #[tokio::test]
    async fn list_files_tool_works() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("a.txt"), "").unwrap();
        std::fs::write(tmp.path().join("b.txt"), "").unwrap();
        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let result =
            execute_execution_tool("list_files", &json!({ "path": "." }), &ctx, None).await;
        let files = result["files"].as_array().unwrap();
        assert_eq!(files.len(), 2);
    }

    #[tokio::test]
    async fn tool_allowlist_blocks_disallowed() {
        let tmp = TempDir::new().unwrap();
        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let allowed = vec!["read_file".to_string()];
        let result = execute_execution_tool(
            "write_file",
            &json!({ "path": "x.txt", "content": "no" }),
            &ctx,
            Some(&allowed),
        )
        .await;
        assert!(result["error"].as_str().unwrap().contains("not allowed"));
    }

    #[tokio::test]
    async fn edit_file_replaces_unique_match() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(
            tmp.path().join("code.rs"),
            "fn main() {\n    println!(\"old\");\n}\n",
        )
        .unwrap();
        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let result = execute_execution_tool(
            "edit_file",
            &json!({ "path": "code.rs", "old_string": "println!(\"old\")", "new_string": "println!(\"new\")" }),
            &ctx,
            None,
        )
        .await;
        assert_eq!(result["success"], true);
        assert!(result["line_start"].as_u64().is_some());
        let content = std::fs::read_to_string(tmp.path().join("code.rs")).unwrap();
        assert!(content.contains("println!(\"new\")"));
        assert!(!content.contains("println!(\"old\")"));
    }

    #[tokio::test]
    async fn edit_file_rejects_no_match() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("code.rs"), "fn main() {}\n").unwrap();
        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let result = execute_execution_tool(
            "edit_file",
            &json!({ "path": "code.rs", "old_string": "nonexistent", "new_string": "replacement" }),
            &ctx,
            None,
        )
        .await;
        assert!(result["error"].as_str().unwrap().contains("not found"));
    }

    #[tokio::test]
    async fn edit_file_rejects_multiple_matches() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("code.rs"), "let x = 1;\nlet x = 1;\n").unwrap();
        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let result = execute_execution_tool(
            "edit_file",
            &json!({ "path": "code.rs", "old_string": "let x = 1;", "new_string": "let x = 2;" }),
            &ctx,
            None,
        )
        .await;
        assert!(result["error"]
            .as_str()
            .unwrap()
            .contains("matches 2 locations"));
    }

    #[tokio::test]
    async fn edit_file_appends_with_empty_old_string() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("code.rs"), "line1\n").unwrap();
        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let result = execute_execution_tool(
            "edit_file",
            &json!({ "path": "code.rs", "old_string": "", "new_string": "line2\n" }),
            &ctx,
            None,
        )
        .await;
        assert_eq!(result["success"], true);
        let content = std::fs::read_to_string(tmp.path().join("code.rs")).unwrap();
        assert_eq!(content, "line1\nline2\n");
    }

    #[tokio::test]
    async fn unknown_tool_returns_error() {
        let tmp = TempDir::new().unwrap();
        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let result = execute_execution_tool("nope", &json!({}), &ctx, None).await;
        assert!(result["error"].as_str().unwrap().contains("Unknown tool"));
    }
}
