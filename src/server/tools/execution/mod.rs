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
use crate::server::tools::shared::{error_json, is_tool_allowed, tool_not_allowed_error};
use crate::types::UserId;

pub use container::execute_tool_in_container;

/// Namespace UUID for generating deterministic builtin tool IDs.
const TOOLS_NS: Uuid = Uuid::from_bytes([
    0x6b, 0xa7, 0xb8, 0x10, 0x9d, 0xad, 0x11, 0xd1, 0x80, 0xb4, 0x00, 0xc0, 0x4f, 0xd4, 0x30, 0xc8,
]);

/// Depth `list_files` walks when the caller does not ask for one. Shared by
/// both backends so a listing does not change shape with where it ran.
///
/// Three levels reaches `src/server/handlers/` from the workspace root — deep
/// enough to see the shape of a decomposed deliverable in one call, shallow
/// enough that a repository does not flood the result.
const DEFAULT_LIST_DEPTH: u32 = 3;

/// Build the `list_files` result both execution backends return.
///
/// Both list relative to the requested `path`; the prefix is put back here so
/// every entry is a workspace-relative path that `read_file` and `edit_file`
/// accept unchanged. Without it an agent that lists a subdirectory and then
/// reads a name out of that listing gets a not-found for a file that is
/// there.
///
/// `dropped` is reported rather than swallowed: a silently truncated listing
/// reads as a complete one, and an agent looking for a file that is not in it
/// concludes the file does not exist.
fn list_files_response(path: &str, files: Vec<String>, dropped: usize, depth: u32) -> Value {
    let prefix = path.trim_end_matches('/');
    let files: Vec<String> = if prefix.is_empty() || prefix == "." {
        files
    } else {
        files.into_iter().map(|f| format!("{prefix}/{f}")).collect()
    };

    let mut out = json!({ "files": files, "depth": depth });
    if dropped > 0 {
        out["truncated"] = json!(dropped);
    }
    out
}

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
        // Single source of truth: the registry owns these definitions, as it
        // already does for `run_command` below. They used to be duplicated here
        // with different text, so the description an agent saw depended on
        // which path assembled its tool set.
        crate::tools::registry::get_tool_definition("read_file").unwrap(),
        crate::tools::registry::get_tool_definition("write_file").unwrap(),
        crate::tools::registry::get_tool_definition("edit_file").unwrap(),
        crate::tools::registry::get_tool_definition("list_files").unwrap(),
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
        // Single source of truth: use the registry's definition
        crate::tools::registry::get_tool_definition("run_command").unwrap(),
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
    if !is_tool_allowed(name, allowed_tools) {
        return tool_not_allowed_error(name);
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
    if !is_tool_allowed(name, allowed_tools) {
        return tool_not_allowed_error(name);
    }

    json!({ "error": format!("Tool '{}' requires an execution context", name) })
}

/// Dispatch a tool call through the unified cascade.
///
/// Document tools (read_document, create_doc, update_doc, search_docs) are
/// handled first when `state` is provided, then web tools (network only).
/// Then tries container execution
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
    let route = route_for(
        name,
        container_handle.is_some(),
        execution_context.is_some(),
        state.is_some(),
        user_id.is_some(),
    );

    match route {
        Route::Document => {
            // The only cascade branch that used to skip the allow-list. Every
            // workforce agent has state and a user, so `route_for` sends all
            // four document tools here — they ran for agents that were never
            // granted them, including read-only ones.
            if !is_tool_allowed(name, allowed_tools) {
                return tool_not_allowed_error(name);
            }
            // Guarded by `route_for`, which only returns Document when the
            // dependencies each tool needs are present.
            let state = state.expect("route_for guarantees state for Document");
            match name {
                "read_document" => documents::execute_read_document(input, state).await,
                "update_doc" => documents::execute_update_doc(input, state).await,
                "create_doc" => {
                    let uid = user_id.expect("route_for guarantees user for create_doc");
                    documents::execute_create_doc(input, state, uid).await
                }
                "search_docs" => {
                    let uid = user_id.expect("route_for guarantees user for search_docs");
                    documents::execute_search_docs(input, state, uid).await
                }
                other => error_json(format!("Unknown document tool: {}", other)),
            }
        }
        Route::Web => {
            if !is_tool_allowed(name, allowed_tools) {
                return tool_not_allowed_error(name);
            }
            crate::server::tools::web::dispatch(name, input, state).await
        }
        Route::Container => {
            let handle = container_handle.expect("route_for guarantees a container");
            execute_tool_in_container(name, input, handle, allowed_tools).await
        }
        Route::Local => {
            let ctx = execution_context.expect("route_for guarantees a context");
            execute_execution_tool(name, input, ctx, allowed_tools).await
        }
        Route::ContextFree => execute_context_free_tool(name, input, allowed_tools).await,
    }
}

/// Which branch of the cascade handles a tool call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Route {
    /// Knowledge-base tools. Need `AppState`, and some need a user.
    Document,
    /// Network-only tools. Need neither a workspace nor a container.
    Web,
    /// Runs inside the agent's container.
    Container,
    /// Runs against the host execution context.
    Local,
    /// Nothing here can run it; the caller gets an explanatory error.
    ContextFree,
}

/// Decide where a tool call goes.
///
/// Order matters and is load-bearing. Document and web tools are matched
/// first because the container and local branches are catch-alls: a web tool
/// reaching `Route::Container` would be handed to a container that has no
/// handler for it, and the agent would see an opaque failure.
pub(crate) fn route_for(
    name: &str,
    has_container: bool,
    has_execution_context: bool,
    has_state: bool,
    has_user: bool,
) -> Route {
    let is_document = match name {
        "read_document" | "update_doc" => has_state,
        "create_doc" | "search_docs" => has_state && has_user,
        _ => false,
    };
    if is_document {
        return Route::Document;
    }
    if crate::server::tools::web::is_web_tool(name) {
        return Route::Web;
    }
    if has_container {
        return Route::Container;
    }
    if has_execution_context {
        return Route::Local;
    }
    Route::ContextFree
}

mod tests;
