//! Execution tools that agents can call during their tool use loop.
//!
//! Wraps the execution layer (FileOps, GitOps, TestRunner, Sandbox)
//! as Anthropic-compatible tool definitions with a single dispatcher.

mod container;
mod file_io;
mod local;

use serde_json::{json, Value};
use uuid::Uuid;

use crate::db::ToolRow;
use crate::execution::{ContainerHandle, ExecutionContext};
use crate::llm::Tool;
use crate::server::state::AppState;
use crate::server::tools::documents;
use crate::types::UserId;

pub use container::execute_tool_in_container;

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
            name: "create_doc".into(),
            description: "Create a new document with auto-generated summary.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "title": {
                        "type": "string",
                        "description": "Document title"
                    },
                    "content": {
                        "type": "string",
                        "description": "Document content (markdown supported)"
                    }
                },
                "required": ["title", "content"]
            }),
        },
        Tool {
            name: "search_docs".into(),
            description: "Full-text search across all documents.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query"
                    }
                },
                "required": ["query"]
            }),
        },
        Tool {
            name: "update_doc".into(),
            description: "Update existing document content.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "document_id": {
                        "type": "string",
                        "description": "Document UUID"
                    },
                    "content": {
                        "type": "string",
                        "description": "New document content"
                    }
                },
                "required": ["document_id", "content"]
            }),
        },
        Tool {
            name: "read_document".into(),
            description: "Read a document from the knowledge base by ID. Returns the document title, content, and metadata.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "document_id": {
                        "type": "string",
                        "description": "UUID of the document to read"
                    }
                },
                "required": ["document_id"]
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
        "read_file" => local::exec_read_file(input, ctx).await,
        "write_file" => local::exec_write_file(input, ctx).await,
        "edit_file" => local::exec_edit_file(input, ctx).await,
        "list_files" => local::exec_list_files(input, ctx).await,
        "git_status" => local::exec_git_status(ctx),
        "git_diff" => local::exec_git_diff(input, ctx),
        "git_add" => local::exec_git_add(input, ctx),
        "git_commit" => local::exec_git_commit(input, ctx),
        "git_branch" => local::exec_git_branch(input, ctx),
        "run_tests" => local::exec_run_tests(input, ctx).await,
        "run_command" => local::exec_run_command(input, ctx).await,
        _ => json!({ "error": format!("Unknown tool: {}", name) }),
    }
}

/// Execute a tool that does not require an `ExecutionContext` (e.g. external API calls).
/// Returns an error for tools that need filesystem/git access.
pub async fn execute_context_free_tool(
    name: &str,
    _input: &Value,
    allowed_tools: Option<&[String]>,
) -> Value {
    if let Some(allowed) = allowed_tools {
        if !allowed.iter().any(|t| t == name) {
            return json!({ "error": format!("Tool '{}' is not allowed for this agent", name) });
        }
    }

    json!({ "error": format!("Tool '{}' requires an execution context", name) })
}

/// Dispatch a tool call through the unified cascade.
///
/// Document tools (read_document, create_doc, update_doc, search_docs) are
/// handled first when `state` is provided. Then tries container execution
/// (if a handle is provided), then local execution via the host execution
/// context, then context-free tools (external APIs only).
pub async fn dispatch_tool_cascade(
    name: &str,
    input: &Value,
    container_handle: Option<&ContainerHandle>,
    execution_context: Option<&ExecutionContext>,
    allowed_tools: Option<&[String]>,
    state: Option<&AppState>,
    user_id: Option<UserId>,
) -> Value {
    // Document tools need DB access (AppState), not filesystem.
    match name {
        "read_document" => {
            if let Some(state) = state {
                return documents::execute_read_document(input, state).await;
            }
        }
        "create_doc" => {
            if let (Some(state), Some(uid)) = (state, user_id) {
                return documents::execute_create_doc(input, state, uid).await;
            }
        }
        "update_doc" => {
            if let Some(state) = state {
                return documents::execute_update_doc(input, state).await;
            }
        }
        "search_docs" => {
            if let (Some(state), Some(uid)) = (state, user_id) {
                return documents::execute_search_docs(input, state, uid).await;
            }
        }
        _ => {}
    }

    if let Some(handle) = container_handle {
        return execute_tool_in_container(name, input, handle, allowed_tools).await;
    }
    if let Some(ctx) = execution_context {
        return execute_execution_tool(name, input, ctx, allowed_tools).await;
    }
    execute_context_free_tool(name, input, allowed_tools).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

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
