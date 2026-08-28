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

    // ── read_file windowing ───────────────────────────────────────────────

    use crate::server::tools::execution::container::read_file_window;

    /// An empty file skipped the past-the-end guard, so `lines[5..0]` panicked.
    /// The panic surfaces as `Agent task panicked` and fails the whole step, and
    /// a model retrying a read with an offset is all it takes to reach it.
    #[test]
    fn read_file_window_rejects_an_offset_into_an_empty_file() {
        let out = read_file_window("empty.txt", "", 5, 100);
        assert!(out["error"].as_str().unwrap().contains("past the end"));
    }

    /// Offset 0 into an empty file is a legitimate empty read, not an error.
    #[test]
    fn read_file_window_reads_an_empty_file_from_the_start() {
        let out = read_file_window("empty.txt", "", 0, 100);
        assert!(out.get("error").is_none());
        assert_eq!(out["content"], "");
        assert_eq!(out["total_lines"], 0);
    }

    /// A zero limit returned nothing while advertising `next_offset` equal to
    /// the offset just asked for — a read loop with no termination.
    #[test]
    fn read_file_window_treats_a_zero_limit_as_the_default() {
        let out = read_file_window("a.txt", "one\ntwo\nthree\n", 0, 0);
        assert_eq!(out["content"], "one\ntwo\nthree\n");
        assert!(out.get("next_offset").is_none());
    }

    /// The window must concatenate back to the exact bytes. `lines()` stripped
    /// the `\r` of a CRLF file and the trailing newline, so the read/modify/
    /// write_file round trip the tool descriptions teach silently rewrote every
    /// line ending in the file.
    #[test]
    fn read_file_window_preserves_crlf_and_the_trailing_newline() {
        let crlf = "alpha\r\nbeta\r\n";
        assert_eq!(read_file_window("w.txt", crlf, 0, 100)["content"], crlf);

        let trailing = "alpha\nbeta\n";
        assert_eq!(
            read_file_window("u.txt", trailing, 0, 100)["content"],
            trailing
        );

        let no_trailing = "alpha\nbeta";
        assert_eq!(
            read_file_window("n.txt", no_trailing, 0, 100)["content"],
            no_trailing
        );
    }

    /// A bounded window still reports the full line count and where to resume.
    #[test]
    fn read_file_window_paginates_and_reports_the_next_offset() {
        let out = read_file_window("a.txt", "one\ntwo\nthree\nfour\n", 1, 2);
        assert_eq!(out["content"], "two\nthree\n");
        assert_eq!(out["total_lines"], 4);
        assert_eq!(out["next_offset"], 3);
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
    async fn list_files_paths_round_trip_into_read_file() {
        // The walk lists relative to `path`; the response puts the prefix
        // back. Without that, a name taken from a subdirectory listing is not
        // a path read_file can resolve.
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir(tmp.path().join("reports")).unwrap();
        std::fs::write(tmp.path().join("reports/summary.md"), "body").unwrap();
        let ctx = ExecutionContext::new(tmp.path().to_path_buf());

        let listing =
            execute_execution_tool("list_files", &json!({ "path": "reports" }), &ctx, None).await;
        let files = listing["files"].as_array().unwrap();
        assert_eq!(files, &vec![json!("reports/summary.md")]);

        let read = execute_execution_tool(
            "read_file",
            &json!({ "path": files[0].as_str().unwrap() }),
            &ctx,
            None,
        )
        .await;
        assert_eq!(read["content"], json!("body"));
    }

    #[tokio::test]
    async fn list_files_reports_a_missing_path_as_an_error() {
        let tmp = TempDir::new().unwrap();
        let ctx = ExecutionContext::new(tmp.path().to_path_buf());

        let result =
            execute_execution_tool("list_files", &json!({ "path": "nope" }), &ctx, None).await;
        assert!(result["files"].is_null(), "{result}");
        assert!(result["error"].is_string(), "{result}");
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

    /// The diagnostics recorder reads `bytes` off the tool result and
    /// *overwrites* the manifest entry with it. `edit_file` never reported the
    /// field, so the chunked-append pattern the prompts prescribe — write_file
    /// the first chunk, edit_file the rest — filed the finished deliverable at
    /// 0 bytes, which sorts last in `produced_files` and is the first thing
    /// dropped from the downstream `files:` line.
    #[tokio::test]
    async fn edit_file_reports_the_resulting_file_size() {
        let tmp = TempDir::new().unwrap();
        let ctx = ExecutionContext::new(tmp.path().to_path_buf());

        let written = execute_execution_tool(
            "write_file",
            &json!({ "path": "doc.md", "content": "# Spec\n" }),
            &ctx,
            None,
        )
        .await;
        assert_eq!(written["bytes"], 7);

        // Append mode: empty old_string.
        let appended = execute_execution_tool(
            "edit_file",
            &json!({ "path": "doc.md", "old_string": "", "new_string": "body\n" }),
            &ctx,
            None,
        )
        .await;
        assert_eq!(appended["success"], true);
        let on_disk = std::fs::read_to_string(tmp.path().join("doc.md")).unwrap();
        assert_eq!(appended["bytes"], on_disk.len());
        assert!(appended["bytes"].as_u64().unwrap() > 0);

        // Replacement mode reports the resulting size too, not the edit's size.
        let replaced = execute_execution_tool(
            "edit_file",
            &json!({ "path": "doc.md", "old_string": "body", "new_string": "much longer body" }),
            &ctx,
            None,
        )
        .await;
        let on_disk = std::fs::read_to_string(tmp.path().join("doc.md")).unwrap();
        assert_eq!(replaced["bytes"], on_disk.len());
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

#[cfg(test)]
mod routing_tests {
    use super::super::{route_for, Route};

    // The bug this guards: the container and local branches are catch-alls, so
    // a web tool matched after them gets shelled into a container that has no
    // handler for it. Every combination must still route Web.
    #[test]
    fn web_tools_never_reach_the_container() {
        for name in ["brave_search", "read_webpage"] {
            for &has_ctx in &[true, false] {
                for &has_state in &[true, false] {
                    for &has_user in &[true, false] {
                        assert_eq!(
                            route_for(name, true, has_ctx, has_state, has_user),
                            Route::Web,
                            "{name} misrouted with container present"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn web_tools_route_web_with_nothing_available() {
        // Notably NOT ContextFree: a web tool needs no context, so it must not
        // be told it requires one.
        assert_eq!(
            route_for("brave_search", false, false, false, false),
            Route::Web
        );
    }

    #[test]
    fn document_tools_take_precedence_over_the_container() {
        assert_eq!(
            route_for("read_document", true, true, true, true),
            Route::Document
        );
        assert_eq!(
            route_for("search_docs", true, true, true, true),
            Route::Document
        );
    }

    #[test]
    fn document_tools_fall_through_without_their_dependencies() {
        // read_document needs state; without it the call falls to the
        // container, preserving the pre-existing behaviour.
        assert_eq!(
            route_for("read_document", true, false, false, false),
            Route::Container
        );
        // create_doc needs a user as well as state.
        assert_eq!(
            route_for("create_doc", false, true, true, false),
            Route::Local
        );
        assert_eq!(
            route_for("create_doc", false, false, true, false),
            Route::ContextFree
        );
    }

    #[test]
    fn ordinary_tools_prefer_container_then_local_then_context_free() {
        assert_eq!(
            route_for("run_command", true, true, false, false),
            Route::Container
        );
        assert_eq!(
            route_for("run_command", false, true, false, false),
            Route::Local
        );
        assert_eq!(
            route_for("run_command", false, false, false, false),
            Route::ContextFree
        );
    }

    #[test]
    fn an_unknown_tool_still_reaches_a_branch_that_can_report_it() {
        assert_eq!(
            route_for("nonexistent_tool", false, false, true, true),
            Route::ContextFree
        );
    }
}
