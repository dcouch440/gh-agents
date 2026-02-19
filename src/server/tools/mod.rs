//! Tool definitions and handlers for the orchestrator LLM.
//!
//! Provides tool schemas for codebase exploration, document management,
//! and structured output validation. Handler implementations live in
//! focused sub-modules: `exploration`, `documents`, and `haiku`.

pub mod documents;
mod exploration;
pub mod haiku;
pub mod node_assistant;
pub mod shared;
pub mod workforce;

use serde_json::{json, Value};

use crate::llm::Tool;
use crate::types::UserId;

use super::state::AppState;

// Re-exports for backward compatibility.
pub use haiku::{haiku_extract_context, haiku_read_file, haiku_summarize, haiku_summarize_title};

/// Return tool definitions filtered by allowed names.
/// If `allowed` is empty, returns all tools.
pub fn filtered_tools(allowed: &[String]) -> Vec<Tool> {
    let all = agent_tools();
    if allowed.is_empty() {
        return all;
    }
    all.into_iter()
        .filter(|t| allowed.iter().any(|a| a == &t.name))
        .collect()
}

/// Return tool definitions for the Anthropic API.
/// LEGACY: Agent pool and pipeline tools removed. Use chat/session API instead.
pub fn agent_tools() -> Vec<Tool> {
    vec![
        // --- Codebase exploration tools (read-only) ---
        Tool {
            name: "read_file".to_string(),
            description: "Read a file in the project. Small files are returned directly. Large files are summarized by a fast model — use the 'focus' parameter to get relevant sections.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "File path relative to the project root (e.g. 'src/main.rs', 'ui/src/App.tsx')"
                    },
                    "focus": {
                        "type": "string",
                        "description": "Optional: what you're looking for in the file (e.g. 'error handling', 'the User struct', 'imports'). Helps extract relevant sections from large files."
                    }
                },
                "required": ["path"]
            }),
        },
        Tool {
            name: "list_files".to_string(),
            description: "List files and directories at a given path in the project. Use this to explore the codebase structure.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Directory path relative to the project root. Use '.' or '' for the root."
                    }
                },
                "required": ["path"]
            }),
        },
        Tool {
            name: "search_files".to_string(),
            description: "Search for a pattern in project files. Returns matching lines with file paths and line numbers. Use this to find code references instead of reading entire files."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Search pattern (text or regex) to find in files"
                    },
                    "path": {
                        "type": "string",
                        "description": "Optional: subdirectory to search in (e.g. 'src/', 'ui/src/'). Defaults to project root."
                    },
                    "max_results": {
                        "type": "integer",
                        "description": "Maximum number of matches to return. Defaults to 20."
                    }
                },
                "required": ["pattern"]
            }),
        },
        Tool {
            name: "think".to_string(),
            description: "Use this tool to think step-by-step before taking action. Write out your reasoning, \
                plan your approach, and consider edge cases. This tool has no side effects — it simply \
                returns your thoughts back to you. Use it before complex decisions, when choosing between \
                multiple approaches, or when you need to analyze information before responding."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "thought": {
                        "type": "string",
                        "description": "Your step-by-step reasoning, analysis, or plan."
                    }
                },
                "required": ["thought"]
            }),
        },
        // --- Document tools ---
        Tool {
            name: "create_doc".to_string(),
            description: "Create a new document (architecture note, design doc, etc.). Returns the document ID and ref_tag. A summary is generated automatically in the background.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "title": {
                        "type": "string",
                        "description": "Title of the document"
                    },
                    "content": {
                        "type": "string",
                        "description": "Full content/body of the document"
                    },
                    "tags": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Optional tags for categorization"
                    }
                },
                "required": ["title", "content"]
            }),
        },
        Tool {
            name: "update_doc".to_string(),
            description: "Update an existing document's content, title, or tags. A new summary is regenerated automatically in the background.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "doc_id": {
                        "type": "string",
                        "description": "The UUID of the document to update"
                    },
                    "content": {
                        "type": "string",
                        "description": "New content for the document"
                    },
                    "title": {
                        "type": "string",
                        "description": "New title for the document"
                    },
                    "tags": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "New tags for the document"
                    }
                },
                "required": ["doc_id"]
            }),
        },
        Tool {
            name: "search_docs".to_string(),
            description: "Search documents by full-text query. Returns summaries and snippets, not full content.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query string"
                    }
                },
                "required": ["query"]
            }),
        },
        // --- Structured output validation tools ---
        Tool {
            name: "submit_prd".to_string(),
            description: "Submit a finalized PRD as structured JSON. Validates all fields and stores the PRD as a document. Returns validation errors if any fields are missing or invalid."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "title": { "type": "string", "description": "PRD title" },
                    "problem_statement": { "type": "string", "description": "What problem this solves" },
                    "goals": { "type": "array", "items": { "type": "string" }, "description": "Measurable goals (min 1)" },
                    "non_goals": { "type": "array", "items": { "type": "string" }, "description": "Explicit scope boundaries (min 1)" },
                    "user_stories": { "type": "array", "items": { "type": "string" }, "description": "User stories (min 1)" },
                    "technical_approach": { "type": "string", "description": "Technical approach and architecture" },
                    "milestones": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "name": { "type": "string" },
                                "deliverables": { "type": "array", "items": { "type": "string" } }
                            },
                            "required": ["name", "deliverables"]
                        },
                        "description": "Implementation milestones (min 1)"
                    },
                    "complexity": { "type": "string", "enum": ["S", "M", "L", "XL"], "description": "Complexity estimate" },
                    "success_metrics": { "type": "array", "items": { "type": "string" }, "description": "Optional success metrics" },
                    "risks": { "type": "array", "items": { "type": "string" }, "description": "Optional risks" }
                },
                "required": ["title", "problem_statement", "goals", "non_goals", "user_stories", "technical_approach", "milestones", "complexity"]
            }),
        },
        Tool {
            name: "submit_ticket".to_string(),
            description: "Submit a decomposition ticket as structured JSON. Validates all fields and returns the validated ticket. Does not store the ticket — it flows through the pipeline system."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "title": { "type": "string", "description": "Ticket title" },
                    "description": { "type": "string", "description": "Detailed description" },
                    "acceptance_criteria": { "type": "array", "items": { "type": "string" }, "description": "Acceptance criteria (min 1)" },
                    "files_to_modify": { "type": "array", "items": { "type": "string" }, "description": "Files to create or modify (min 1)" },
                    "complexity": { "type": "string", "enum": ["S", "M", "L", "XL"], "description": "Complexity estimate" },
                    "role": { "type": "string", "enum": ["worker", "reviewer", "utility"], "description": "Suggested agent role" },
                    "dependencies": { "type": "array", "items": { "type": "string" }, "description": "Optional ticket title dependencies" }
                },
                "required": ["title", "description", "acceptance_criteria", "files_to_modify", "complexity", "role"]
            }),
        },
    ]
}

/// Execute a tool by name with the given JSON input.
///
/// Returns a JSON value describing the result.
pub async fn execute_tool(
    name: &str,
    input: &Value,
    state: &AppState,
    user_id: UserId,
    _session_id: Option<uuid::Uuid>,
) -> Value {
    match name {
        "read_file" => exploration::execute_read_file(input).await,
        "list_files" => exploration::execute_list_files(input).await,
        "search_files" => exploration::execute_search_files(input).await,
        "think" => exploration::execute_think(input),
        "create_doc" => documents::execute_create_doc(input, state, user_id).await,
        "update_doc" => documents::execute_update_doc(input, state).await,
        "search_docs" => documents::execute_search_docs(input, state, user_id).await,
        "submit_prd" => documents::execute_submit_prd(input, state, user_id).await,
        "submit_ticket" => documents::execute_submit_ticket(input).await,
        _ => json!({ "error": format!("Unknown tool: {}", name) }),
    }
}

#[cfg(test)]
mod tests;
