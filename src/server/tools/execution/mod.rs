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
            description: "Execute a shell command. Chain with && to do multiple things in one call.\n\n\
                Examples:\n\
                \x20 mkdir -p my-app && cat > my-app/main.py << 'EOF'\n\
                \x20 import sys\n\
                \x20 print(f\"Hello {sys.argv[1]}\")\n\
                \x20 EOF\n\n\
                \x20 pip install requests && python scraper.py | jq '.results[]'\n\n\
                \x20 grep -rn 'TODO' src/ | head -20\n\n\
                \x20 find . -name '*.py' | xargs wc -l | sort -n | tail -5\n\n\
                \x20 curl -s https://api.example.com/data | jq '.items[] | {name, count}' > results.json\n\n\
                File operations:\n\
                - Write: cat > file << 'EOF' ... EOF (always single-quote EOF)\n\
                - Read: cat file | head -50\n\
                - Edit: sed -i 's/old/new/g' file.py\n\
                - Search: grep -rn 'pattern' . | head -20\n\
                - Browse: find . -type f -name '*.py' | head -20\n\n\
                Available tools: python, pip, git, curl, wget, jq, grep, sed, awk, find, xargs, sort, uniq, wc, head, tail, tee, tr, cut.\n\
                Installed packages persist to the next step."
                .into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "command": { "type": "string", "description": "Shell command to execute. Chain with && for efficiency." }
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

mod tests;
