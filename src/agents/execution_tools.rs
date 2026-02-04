//! Execution tools that agents can call during their tool use loop.
//!
//! Wraps the execution layer (FileOps, GitOps, TestRunner, Sandbox)
//! as Anthropic-compatible tool definitions with a single dispatcher.

use serde_json::{json, Value};
use uuid::Uuid;

use crate::db::ToolRow;
use crate::execution::{ExecutionContext, FileOps, GitOps, Sandbox, TestRunner};
use crate::llm::{
    GrokResearchClient, ResearchRequest, ResearchSource, Tool, WebSearchFilters, XSearchFilters,
};

/// Namespace UUID for generating deterministic builtin tool IDs.
const TOOLS_NS: Uuid = Uuid::from_bytes([
    0x6b, 0xa7, 0xb8, 0x10, 0x9d, 0xad, 0x11, 0xd1, 0x80, 0xb4, 0x00, 0xc0, 0x4f, 0xd4, 0x30, 0xc8,
]);

/// Convert a snake_case tool name to a human-readable display name.
fn tool_display_name(name: &str) -> String {
    name.split('_')
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().to_string() + c.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Return the built-in execution tools as DB rows for seeding.
///
/// Each tool gets a deterministic UUID via `Uuid::new_v5(TOOLS_NS, name)` so
/// seeding is idempotent.
pub fn builtin_tool_rows() -> Vec<ToolRow> {
    execution_tools()
        .into_iter()
        .map(|t| {
            let display_name = tool_display_name(&t.name);
            ToolRow {
                id: Uuid::new_v5(&TOOLS_NS, t.name.as_bytes()),
                user_id: Uuid::nil(), // Placeholder — overwritten by seed_builtin_tools
                name: t.name,
                display_name,
                description: t.description,
                parameters: t.input_schema,
                created_at: chrono::Utc::now(),
                version: 1,
            }
        })
        .collect()
}

/// Return all execution tool definitions for the Anthropic API.
pub fn execution_tools() -> Vec<Tool> {
    vec![
        Tool {
            name: "read_file".into(),
            description: "Read the contents of a file.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Relative path from project root" }
                },
                "required": ["path"]
            }),
        },
        Tool {
            name: "write_file".into(),
            description: "Write content to a file. Creates parent directories if needed.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Relative path from project root" },
                    "content": { "type": "string", "description": "File content to write" }
                },
                "required": ["path", "content"]
            }),
        },
        Tool {
            name: "edit_file".into(),
            description: "Edit a file by replacing an exact string match. Provide old_string (the existing code) and new_string (the replacement). old_string must match exactly one location in the file. Prefer this over write_file for modifying existing files.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Relative path from project root" },
                    "old_string": { "type": "string", "description": "Exact existing text to find and replace. Must be unique in the file." },
                    "new_string": { "type": "string", "description": "Replacement text" }
                },
                "required": ["path", "old_string", "new_string"]
            }),
        },
        Tool {
            name: "list_files".into(),
            description: "List files and directories at a path.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Relative path from project root (default: root)" }
                },
                "required": []
            }),
        },
        Tool {
            name: "git_status".into(),
            description: "Show the working tree status (staged, modified, untracked files).".into(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
        },
        Tool {
            name: "git_diff".into(),
            description: "Show unstaged changes in the working tree.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "staged": { "type": "boolean", "description": "If true, show staged changes instead" }
                },
                "required": []
            }),
        },
        Tool {
            name: "git_add".into(),
            description: "Stage files for commit.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "paths": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "File paths to stage"
                    }
                },
                "required": ["paths"]
            }),
        },
        Tool {
            name: "git_commit".into(),
            description: "Create a commit with the staged changes.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "message": { "type": "string", "description": "Commit message" }
                },
                "required": ["message"]
            }),
        },
        Tool {
            name: "git_branch".into(),
            description: "Get current branch name or create a new branch.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "create": { "type": "string", "description": "If provided, create a branch with this name" }
                },
                "required": []
            }),
        },
        Tool {
            name: "run_tests".into(),
            description: "Run the project's test suite.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "pattern": { "type": "string", "description": "If provided, only run tests matching this pattern" }
                },
                "required": []
            }),
        },
        Tool {
            name: "run_command".into(),
            description: "Execute a shell command in a sandboxed environment.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "command": { "type": "string", "description": "Shell command to execute" }
                },
                "required": ["command"]
            }),
        },
        Tool {
            name: "web_research".into(),
            description: "Research a topic using real-time web and X/Twitter search via xAI Grok. Returns a synthesized answer with citations. Requires XAI_API_KEY environment variable.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The research query or question to investigate"
                    },
                    "sources": {
                        "type": "array",
                        "items": { "type": "string", "enum": ["web", "x"] },
                        "description": "Which sources to search. Default: both web and X."
                    },
                    "allowed_domains": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Only search these web domains (max 5). Optional."
                    },
                    "x_handles": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Only search posts from these X handles (max 10). Optional."
                    },
                    "x_from_date": {
                        "type": "string",
                        "description": "ISO8601 date to limit X search start (YYYY-MM-DD). Optional."
                    }
                },
                "required": ["query"]
            }),
        },
    ]
}

/// Execute an execution tool by name. Returns JSON result.
///
/// If `allowed_tools` is Some, only tools in the list are permitted.
pub async fn execute_execution_tool(
    name: &str,
    input: &Value,
    ctx: &ExecutionContext,
    allowed_tools: Option<&[String]>,
) -> Value {
    if let Some(allowed) = allowed_tools {
        if !allowed.iter().any(|t| t == name) {
            return json!({ "error": format!("Tool '{}' is not allowed for this agent", name) });
        }
    }

    match name {
        "read_file" => exec_read_file(input, ctx).await,
        "write_file" => exec_write_file(input, ctx).await,
        "edit_file" => exec_edit_file(input, ctx).await,
        "list_files" => exec_list_files(input, ctx).await,
        "git_status" => exec_git_status(ctx),
        "git_diff" => exec_git_diff(input, ctx),
        "git_add" => exec_git_add(input, ctx),
        "git_commit" => exec_git_commit(input, ctx),
        "git_branch" => exec_git_branch(input, ctx),
        "run_tests" => exec_run_tests(input, ctx).await,
        "run_command" => exec_run_command(input, ctx).await,
        "web_research" => exec_web_research(input).await,
        _ => json!({ "error": format!("Unknown tool: {}", name) }),
    }
}

// --- File operations ---

async fn exec_read_file(input: &Value, ctx: &ExecutionContext) -> Value {
    let path = match input["path"].as_str() {
        Some(p) => p,
        None => return json!({ "error": "Missing required parameter: path" }),
    };
    let file_ops = FileOps::new(ctx.clone());
    let full_path = ctx.project_root.join(path);
    match file_ops.read_file(&full_path).await {
        Ok(content) => json!({ "content": content }),
        Err(e) => json!({ "error": e.to_string() }),
    }
}

async fn exec_write_file(input: &Value, ctx: &ExecutionContext) -> Value {
    let path = match input["path"].as_str() {
        Some(p) => p,
        None => return json!({ "error": "Missing required parameter: path" }),
    };
    let content = match input["content"].as_str() {
        Some(c) => c,
        None => return json!({ "error": "Missing required parameter: content" }),
    };
    let file_ops = FileOps::new(ctx.clone());
    let full_path = ctx.project_root.join(path);
    match file_ops.write_file(&full_path, content).await {
        Ok(()) => json!({ "success": true, "path": path }),
        Err(e) => json!({ "error": e.to_string() }),
    }
}

async fn exec_edit_file(input: &Value, ctx: &ExecutionContext) -> Value {
    let path = match input["path"].as_str() {
        Some(p) => p,
        None => return json!({ "error": "Missing required parameter: path" }),
    };
    let old_string = match input["old_string"].as_str() {
        Some(s) => s,
        None => return json!({ "error": "Missing required parameter: old_string" }),
    };
    let new_string = match input["new_string"].as_str() {
        Some(s) => s,
        None => return json!({ "error": "Missing required parameter: new_string" }),
    };

    let file_ops = FileOps::new(ctx.clone());
    let full_path = ctx.project_root.join(path);

    // Handle append mode: empty old_string means append to end
    if old_string.is_empty() {
        let existing = file_ops.read_file(&full_path).await.unwrap_or_default();
        let new_content = if existing.is_empty() {
            new_string.to_string()
        } else if existing.ends_with('\n') {
            format!("{}{}", existing, new_string)
        } else {
            format!("{}\n{}", existing, new_string)
        };
        return match file_ops.write_file(&full_path, &new_content).await {
            Ok(()) => json!({ "success": true, "path": path, "action": "appended" }),
            Err(e) => json!({ "error": e.to_string() }),
        };
    }

    // Read the existing file
    let content = match file_ops.read_file(&full_path).await {
        Ok(c) => c,
        Err(e) => return json!({ "error": e.to_string() }),
    };

    // Count occurrences
    let matches: Vec<_> = content.match_indices(old_string).collect();

    if matches.is_empty() {
        return json!({
            "error": format!("old_string not found in {}", path),
            "hint": "Check for exact whitespace and newline matches. Use read_file to see the current content."
        });
    }

    if matches.len() > 1 {
        return json!({
            "error": format!("old_string matches {} locations in {}. Add surrounding context to make it unique.", matches.len(), path),
            "match_count": matches.len()
        });
    }

    // Exactly one match — perform the replacement
    let byte_offset = matches[0].0;
    let new_content = format!(
        "{}{}{}",
        &content[..byte_offset],
        new_string,
        &content[byte_offset + old_string.len()..]
    );

    match file_ops.write_file(&full_path, &new_content).await {
        Ok(()) => {
            // Calculate line numbers for the edited region
            let line_start = content[..byte_offset].matches('\n').count() + 1;
            let line_end = line_start + new_string.matches('\n').count();

            // Build a preview: show the replaced lines with a few lines of context
            let new_lines: Vec<&str> = new_content.lines().collect();
            let preview_start = line_start.saturating_sub(2); // 1 line before (0-indexed)
            let preview_end = (line_end + 2).min(new_lines.len()); // 1 line after
            let preview: Vec<String> = new_lines[preview_start..preview_end]
                .iter()
                .enumerate()
                .map(|(i, line)| format!("{:>4} | {}", preview_start + i + 1, line))
                .collect();

            json!({
                "success": true,
                "path": path,
                "line_start": line_start,
                "line_end": line_end,
                "preview": preview.join("\n")
            })
        }
        Err(e) => json!({ "error": e.to_string() }),
    }
}

async fn exec_list_files(input: &Value, ctx: &ExecutionContext) -> Value {
    let path = input["path"].as_str().unwrap_or(".");
    let file_ops = FileOps::new(ctx.clone());
    let full_path = ctx.project_root.join(path);
    match file_ops.list_dir(&full_path).await {
        Ok(entries) => {
            let names: Vec<String> = entries
                .iter()
                .filter_map(|p| {
                    p.strip_prefix(&ctx.project_root)
                        .ok()
                        .map(|rel| rel.to_string_lossy().to_string())
                })
                .collect();
            json!({ "files": names })
        }
        Err(e) => json!({ "error": e.to_string() }),
    }
}

// --- Git operations ---

fn exec_git_status(ctx: &ExecutionContext) -> Value {
    let git = GitOps::new(ctx.clone());
    match git.status() {
        Ok(status) => json!({
            "branch": status.branch,
            "staged": status.staged.iter().map(|f| json!({
                "path": f.path,
                "change_type": format!("{:?}", f.change_type)
            })).collect::<Vec<_>>(),
            "unstaged": status.unstaged.iter().map(|f| json!({
                "path": f.path,
                "change_type": format!("{:?}", f.change_type)
            })).collect::<Vec<_>>(),
            "untracked": status.untracked,
        }),
        Err(e) => json!({ "error": e.to_string() }),
    }
}

fn exec_git_diff(input: &Value, ctx: &ExecutionContext) -> Value {
    let git = GitOps::new(ctx.clone());
    let staged = input["staged"].as_bool().unwrap_or(false);
    let result = if staged {
        git.diff_staged()
    } else {
        git.diff()
    };
    match result {
        Ok(diff) => json!({ "diff": diff }),
        Err(e) => json!({ "error": e.to_string() }),
    }
}

fn exec_git_add(input: &Value, ctx: &ExecutionContext) -> Value {
    let paths = match input["paths"].as_array() {
        Some(arr) => arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>(),
        None => return json!({ "error": "Missing required parameter: paths" }),
    };
    let git = GitOps::new(ctx.clone());
    match git.add_files(&paths) {
        Ok(()) => json!({ "success": true, "staged": paths }),
        Err(e) => json!({ "error": e.to_string() }),
    }
}

fn exec_git_commit(input: &Value, ctx: &ExecutionContext) -> Value {
    let message = match input["message"].as_str() {
        Some(m) => m,
        None => return json!({ "error": "Missing required parameter: message" }),
    };
    let git = GitOps::new(ctx.clone());
    match git.commit(message) {
        Ok(info) => json!({
            "success": true,
            "sha": info.hash,
            "message": info.message,
        }),
        Err(e) => json!({ "error": e.to_string() }),
    }
}

fn exec_git_branch(input: &Value, ctx: &ExecutionContext) -> Value {
    let git = GitOps::new(ctx.clone());
    if let Some(name) = input["create"].as_str() {
        match git.create_branch(name) {
            Ok(info) => json!({ "created": info.name }),
            Err(e) => json!({ "error": e.to_string() }),
        }
    } else {
        match git.current_branch() {
            Ok(Some(branch)) => json!({ "branch": branch }),
            Ok(None) => json!({ "branch": null, "detached": true }),
            Err(e) => json!({ "error": e.to_string() }),
        }
    }
}

// --- Test runner ---

async fn exec_run_tests(input: &Value, ctx: &ExecutionContext) -> Value {
    let mut runner = TestRunner::new(ctx.clone());
    runner.detect_framework();

    if let Some(pattern) = input["pattern"].as_str() {
        match runner.run_specific(pattern).await {
            Ok(result) => test_result_to_json(&result),
            Err(e) => json!({ "error": e.to_string() }),
        }
    } else {
        match runner.run_tests().await {
            Ok(result) => test_result_to_json(&result),
            Err(e) => json!({ "error": e.to_string() }),
        }
    }
}

fn test_result_to_json(result: &crate::execution::TestResult) -> Value {
    json!({
        "passed": result.passed,
        "failed": result.failed,
        "skipped": result.skipped,
        "success": result.success,
        "duration_ms": result.duration_ms,
        "stdout": result.stdout,
        "stderr": result.stderr,
    })
}

// --- Sandbox ---

async fn exec_run_command(input: &Value, ctx: &ExecutionContext) -> Value {
    let command = match input["command"].as_str() {
        Some(c) => c,
        None => return json!({ "error": "Missing required parameter: command" }),
    };
    let sandbox = Sandbox::with_defaults(ctx.clone());
    match sandbox.exec_shell(command).await {
        Ok(result) => json!({
            "exit_code": result.exit_code,
            "stdout": result.stdout,
            "stderr": result.stderr,
            "success": result.exit_code == 0,
        }),
        Err(e) => json!({ "error": e.to_string() }),
    }
}

// --- Web research (xAI Grok) ---

async fn exec_web_research(input: &Value) -> Value {
    let query = match input["query"].as_str() {
        Some(q) => q,
        None => return json!({ "error": "Missing required parameter: query" }),
    };

    // Parse sources (default: both)
    let sources = match input.get("sources").and_then(|v| v.as_array()) {
        Some(arr) => arr
            .iter()
            .filter_map(|v| match v.as_str() {
                Some("web") => Some(ResearchSource::Web),
                Some("x") => Some(ResearchSource::X),
                _ => None,
            })
            .collect(),
        None => vec![ResearchSource::Web, ResearchSource::X],
    };

    // Parse optional filters
    let web_filters = input
        .get("allowed_domains")
        .and_then(|v| v.as_array())
        .map(|domains| WebSearchFilters {
            allowed_domains: Some(
                domains
                    .iter()
                    .filter_map(|d| d.as_str().map(String::from))
                    .collect(),
            ),
            excluded_domains: None,
        });

    let x_handles = input.get("x_handles").and_then(|v| v.as_array());
    let x_from_date = input.get("x_from_date").and_then(|v| v.as_str());
    let x_filters = if x_handles.is_some() || x_from_date.is_some() {
        Some(XSearchFilters {
            allowed_x_handles: x_handles.map(|arr| {
                arr.iter()
                    .filter_map(|h| h.as_str().map(String::from))
                    .collect()
            }),
            excluded_x_handles: None,
            from_date: x_from_date.map(String::from),
            to_date: None,
        })
    } else {
        None
    };

    let client = match GrokResearchClient::from_env() {
        Ok(c) => c,
        Err(e) => {
            return json!({
                "error": format!("Grok client initialization failed: {}. Ensure XAI_API_KEY is set.", e)
            })
        }
    };

    let request = ResearchRequest {
        query: query.to_string(),
        sources,
        web_filters,
        x_filters,
        system_prompt: None,
    };

    match client.research(&request).await {
        Ok(response) => json!({
            "answer": response.answer,
            "citations": response.citations,
            "usage": {
                "input_tokens": response.usage.input_tokens,
                "output_tokens": response.usage.output_tokens
            }
        }),
        Err(e) => json!({ "error": format!("Research failed: {}", e) }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn tool_schemas_are_valid() {
        let tools = execution_tools();
        assert_eq!(tools.len(), 12);
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
    fn builtin_tool_rows_returns_12() {
        let rows = builtin_tool_rows();
        assert_eq!(rows.len(), 12);
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
