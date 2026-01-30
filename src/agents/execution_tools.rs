//! Execution tools that agents can call during their tool use loop.
//!
//! Wraps the execution layer (FileOps, GitOps, TestRunner, Sandbox)
//! as Anthropic-compatible tool definitions with a single dispatcher.

use serde_json::{json, Value};

use crate::execution::{ExecutionContext, FileOps, GitOps, Sandbox, TestRunner};
use crate::llm::Tool;

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
        "list_files" => exec_list_files(input, ctx).await,
        "git_status" => exec_git_status(ctx),
        "git_diff" => exec_git_diff(input, ctx),
        "git_add" => exec_git_add(input, ctx),
        "git_commit" => exec_git_commit(input, ctx),
        "git_branch" => exec_git_branch(input, ctx),
        "run_tests" => exec_run_tests(input, ctx).await,
        "run_command" => exec_run_command(input, ctx).await,
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
    let result = if staged { git.diff_staged() } else { git.diff() };
    match result {
        Ok(diff) => json!({ "diff": diff }),
        Err(e) => json!({ "error": e.to_string() }),
    }
}

fn exec_git_add(input: &Value, ctx: &ExecutionContext) -> Value {
    let paths = match input["paths"].as_array() {
        Some(arr) => arr
            .iter()
            .filter_map(|v| v.as_str())
            .collect::<Vec<_>>(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn tool_schemas_are_valid() {
        let tools = execution_tools();
        assert_eq!(tools.len(), 10);
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

    #[tokio::test]
    async fn read_file_tool_works() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("test.txt"), "hello world").unwrap();
        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let result = execute_execution_tool(
            "read_file",
            &json!({ "path": "test.txt" }),
            &ctx,
            None,
        )
        .await;
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
        let result = execute_execution_tool(
            "list_files",
            &json!({ "path": "." }),
            &ctx,
            None,
        )
        .await;
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
    async fn unknown_tool_returns_error() {
        let tmp = TempDir::new().unwrap();
        let ctx = ExecutionContext::new(tmp.path().to_path_buf());
        let result = execute_execution_tool("nope", &json!({}), &ctx, None).await;
        assert!(result["error"].as_str().unwrap().contains("Unknown tool"));
    }
}
