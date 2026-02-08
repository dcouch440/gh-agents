//! Tool definitions and handlers for the orchestrator LLM.
//!
//! Provides tool schemas for codebase exploration, document management,
//! and structured output validation.

use serde_json::{json, Value};

use std::sync::Arc;

use crate::db::traits::DocumentRepo;
use crate::llm::{
    AnthropicClient, AnthropicConfig, LLMProvider, LLMRequest, Message as LlmMessage, Tool,
};
use crate::types::UserId;

use super::state::AppState;

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
        // LEGACY tools removed: list_agents, list_roles, create_agent, create_agents, assign_task,
        // get_task_result, list_pending_approvals, respond_to_approval, remove_agent,
        // create_pipeline, add_pipeline_stage, start_pipeline, get_pipeline_status
        "read_file" => execute_read_file(input).await,
        "list_files" => execute_list_files(input).await,
        "search_files" => execute_search_files(input).await,
        "think" => execute_think(input),
        "create_doc" => execute_create_doc(input, state, user_id).await,
        "update_doc" => execute_update_doc(input, state).await,
        "search_docs" => execute_search_docs(input, state, user_id).await,
        "submit_prd" => execute_submit_prd(input, state, user_id).await,
        "submit_ticket" => execute_submit_ticket(input).await,
        _ => json!({ "error": format!("Unknown tool: {}", name) }),
    }
}

// --- Codebase exploration tool handlers ---
// LEGACY TOOLS REMOVED: The following agent management and pipeline tools have been
// removed as they relied on the old agent pool system:
// - execute_list_agents, execute_list_roles, execute_create_agent, execute_create_agents
// - execute_assign_task, execute_get_task_result, execute_list_pending_approvals
// - execute_respond_to_approval, execute_remove_agent
// - execute_create_pipeline, execute_add_pipeline_stage, execute_start_pipeline
// - execute_get_pipeline_status
// Use the chat/session API and workflow system instead.

async fn execute_read_file(input: &Value) -> Value {
    let Some(path_str) = input["path"].as_str() else {
        return json!({ "error": "Missing required parameter: path" });
    };
    let focus = input["focus"].as_str();

    // Resolve relative to current working directory (project root)
    let cwd = std::env::current_dir().unwrap_or_default();
    let file_path = cwd.join(path_str);

    // Basic safety: don't escape the project root
    match file_path.canonicalize() {
        Ok(canonical) => {
            if !canonical.starts_with(&cwd) {
                return json!({ "error": "Path is outside the project directory" });
            }
            match tokio::fs::read_to_string(&canonical).await {
                Ok(content) => {
                    let size_bytes = content.len();
                    let line_count = content.lines().count();

                    // Small files: return directly
                    if content.len() <= crate::constants::TRUNCATE_SMALL_FILE {
                        return json!({
                            "path": path_str,
                            "content": content,
                            "line_count": line_count,
                            "size_bytes": size_bytes,
                            "summarized": false
                        });
                    }

                    // Large files: summarize with Haiku
                    let truncated_for_haiku: String = content
                        .chars()
                        .take(crate::constants::TRUNCATE_SUMMARIZE_INPUT)
                        .collect();
                    let focus_instruction = match focus {
                        Some(f) => format!("Focus on: {}. Extract the most relevant code sections, function signatures, and logic related to this focus area.", f),
                        None => "Extract the key structures, function signatures, imports, and overall purpose of this file.".to_string(),
                    };

                    let prompt = format!(
                        "File: {} ({} lines, {} bytes)\n\n{}\n\n---\n{}",
                        path_str, line_count, size_bytes, focus_instruction, truncated_for_haiku
                    );

                    match haiku_read_file(&prompt).await {
                        Some(summary) => json!({
                            "path": path_str,
                            "summary": summary,
                            "line_count": line_count,
                            "size_bytes": size_bytes,
                            "summarized": true
                        }),
                        None => {
                            // Haiku failed — fall back to truncated content
                            let fallback: String = content
                                .chars()
                                .take(crate::constants::TRUNCATE_SMALL_FILE)
                                .collect();
                            json!({
                                "path": path_str,
                                "content": fallback,
                                "line_count": line_count,
                                "size_bytes": size_bytes,
                                "summarized": false,
                                "truncated": true
                            })
                        }
                    }
                }
                Err(e) => json!({ "error": format!("Could not read file: {}", e) }),
            }
        }
        Err(e) => json!({ "error": format!("File not found or inaccessible: {}", e) }),
    }
}

async fn execute_list_files(input: &Value) -> Value {
    let path_str = input["path"].as_str().unwrap_or(".");

    let cwd = std::env::current_dir().unwrap_or_default();
    let dir_path = if path_str.is_empty() || path_str == "." {
        cwd.clone()
    } else {
        cwd.join(path_str)
    };

    match dir_path.canonicalize() {
        Ok(canonical) => {
            if !canonical.starts_with(&cwd) {
                return json!({ "error": "Path is outside the project directory" });
            }
            match tokio::fs::read_dir(&canonical).await {
                Ok(mut entries) => {
                    let mut files = Vec::new();
                    let mut dirs = Vec::new();
                    while let Ok(Some(entry)) = entries.next_entry().await {
                        let name = entry.file_name().to_string_lossy().to_string();
                        // Skip hidden files/dirs
                        if name.starts_with('.') {
                            continue;
                        }
                        if let Ok(ft) = entry.file_type().await {
                            if ft.is_dir() {
                                dirs.push(format!("{}/", name));
                            } else {
                                files.push(name);
                            }
                        }
                    }
                    dirs.sort();
                    files.sort();
                    json!({
                        "path": path_str,
                        "directories": dirs,
                        "files": files
                    })
                }
                Err(e) => json!({ "error": format!("Could not list directory: {}", e) }),
            }
        }
        Err(e) => json!({ "error": format!("Directory not found: {}", e) }),
    }
}

async fn execute_search_files(input: &Value) -> Value {
    let Some(pattern) = input["pattern"].as_str() else {
        return json!({ "error": "Missing required parameter: pattern" });
    };
    let path_str = input["path"].as_str().unwrap_or(".");
    let max_results = input["max_results"]
        .as_u64()
        .unwrap_or(crate::constants::DEFAULT_SEARCH_RESULTS as u64) as usize;

    let cwd = std::env::current_dir().unwrap_or_default();
    let search_dir = if path_str.is_empty() || path_str == "." {
        cwd.clone()
    } else {
        cwd.join(path_str)
    };

    // Validate path
    match search_dir.canonicalize() {
        Ok(canonical) => {
            if !canonical.starts_with(&cwd) {
                return json!({ "error": "Path is outside the project directory" });
            }

            // Use grep -rn for search
            let output = tokio::process::Command::new("grep")
                .args([
                    "-rn",
                    "--include=*.rs",
                    "--include=*.ts",
                    "--include=*.tsx",
                    "--include=*.js",
                    "--include=*.json",
                    "--include=*.toml",
                    "--include=*.sql",
                    "--include=*.md",
                    "--include=*.txt",
                    "--include=*.css",
                    "--include=*.html",
                    "-m",
                    &(max_results * 2).to_string(), // overfetch for filtering
                    pattern,
                ])
                .arg(&canonical)
                .output()
                .await;

            match output {
                Ok(out) => {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    let matches: Vec<Value> = stdout
                        .lines()
                        .take(max_results)
                        .filter_map(|line| {
                            // Format: /abs/path:line_num:content
                            let rest = line.strip_prefix(canonical.to_str()?)?;
                            let rest = rest.strip_prefix('/')?;
                            let mut parts = rest.splitn(3, ':');
                            let file = parts.next()?;
                            let line_num = parts.next()?;
                            let text = parts.next().unwrap_or("").trim();
                            Some(json!({
                                "file": file,
                                "line": line_num.parse::<u64>().unwrap_or(0),
                                "text": &text[..text.len().min(200)]
                            }))
                        })
                        .collect();

                    let total = stdout.lines().count();
                    json!({
                        "pattern": pattern,
                        "matches": matches,
                        "total_matches": total,
                        "truncated": total > max_results
                    })
                }
                Err(e) => json!({ "error": format!("Search failed: {}", e) }),
            }
        }
        Err(e) => json!({ "error": format!("Directory not found: {}", e) }),
    }
}

/// The think tool is a no-op — it returns the agent's reasoning back to it.
/// This gives the model a scratchpad to reason step-by-step before acting.
fn execute_think(input: &Value) -> Value {
    let thought = input["thought"].as_str().unwrap_or("");
    json!({ "thought_recorded": true, "length": thought.len() })
}

// --- Document tool handlers ---

/// Generate a kebab-case ref_tag from a title.
fn title_to_ref_tag(title: &str) -> String {
    title
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-')
        .collect()
}

/// Call Haiku to summarize a file for the orchestrator context.
pub async fn haiku_read_file(prompt: &str) -> Option<String> {
    let config = AnthropicConfig::from_env().ok()?;
    let client = AnthropicClient::new(config).ok()?;

    let request = LLMRequest::new(crate::constants::MODEL_HAIKU, vec![LlmMessage::user(prompt.to_string())])
        .with_system(
            "You are a code reader. Given a source file, extract and return the most relevant content. \
         Include function signatures, struct/type definitions, key logic, and imports. \
         Use the original code when possible — quote exact lines for precision. \
         If a focus area is specified, prioritize content related to it. \
         Be concise but preserve technical accuracy. Do not add commentary.",
        )
        .with_max_tokens(crate::constants::MAX_TOKENS_FILE_READ);

    match client.send_message(request).await {
        Ok(resp) => Some(resp.content),
        Err(e) => {
            tracing::warn!("Haiku file read failed: {}", e);
            None
        }
    }
}

/// Call Haiku to generate a short summary for search indexing.
pub async fn haiku_summarize(content: &str) -> Option<String> {
    let config = AnthropicConfig::from_env().ok()?;
    let client = AnthropicClient::new(config).ok()?;

    let truncated: String = content
        .chars()
        .take(crate::constants::TRUNCATE_SUMMARY_INPUT)
        .collect();
    let request = LLMRequest::new(
        crate::constants::MODEL_HAIKU,
        vec![LlmMessage::user(truncated)],
    )
    .with_system("Summarize this document in 2-3 sentences. Include key entities, topics, and actions. This summary is used for search indexing.")
    .with_max_tokens(crate::constants::MAX_TOKENS_SUMMARIZE);

    match client.send_message(request).await {
        Ok(resp) => Some(resp.content),
        Err(e) => {
            tracing::warn!("Haiku summarization failed: {}", e);
            None
        }
    }
}

/// Call Haiku to generate a short title for a conversation.
pub async fn haiku_summarize_title(content: &str) -> Option<String> {
    let config = AnthropicConfig::from_env().ok()?;
    let client = AnthropicClient::new(config).ok()?;

    let truncated: String = content
        .chars()
        .take(crate::constants::TRUNCATE_TITLE_INPUT)
        .collect();
    let request = LLMRequest::new(crate::constants::MODEL_HAIKU, vec![LlmMessage::user(truncated)])
        .with_system("Generate a short title (3-6 words) for this conversation. This title appears in sidebar navigation. Return the title as plain text, without quotes or trailing punctuation.")
        .with_max_tokens(crate::constants::MAX_TOKENS_TITLE);

    match client.send_message(request).await {
        Ok(resp) => {
            let title = resp.content.trim().to_string();
            if title.is_empty() {
                None
            } else {
                Some(title)
            }
        }
        Err(e) => {
            tracing::warn!("Haiku title generation failed: {}", e);
            None
        }
    }
}

/// Call Haiku to extract relevant context from a conversation summary
/// based on the user's current message.
pub async fn haiku_extract_context(summary: &str, current_message: &str) -> Option<String> {
    let config = AnthropicConfig::from_env().ok()?;
    let client = AnthropicClient::new(config).ok()?;

    let user_text = format!(
        "Summary:\n{}\n\nCurrent message:\n{}",
        summary, current_message
    );
    let request = LLMRequest::new(
        crate::constants::MODEL_HAIKU,
        vec![LlmMessage::user(user_text)],
    )
    .with_system("Extract relevant context from a conversation summary based on the user's current message. The extracted context will be prepended to a new conversation turn. Return 2-4 sentences that are directly relevant to the current request. If nothing is relevant, return 'No prior context needed.'")
    .with_max_tokens(crate::constants::MAX_TOKENS_CONTEXT);

    match client.send_message(request).await {
        Ok(resp) => Some(resp.content),
        Err(e) => {
            tracing::warn!("Haiku context extraction failed: {}", e);
            None
        }
    }
}

/// Spawn a background task to generate and store a document summary.
fn spawn_summary_task(doc_repo: Arc<dyn DocumentRepo>, doc_id: uuid::Uuid, content: String) {
    tokio::spawn(async move {
        if let Some(summary) = haiku_summarize(&content).await {
            if let Err(e) = doc_repo.update_document_summary(doc_id, summary).await {
                tracing::error!("Failed to update document summary: {}", e);
            }
        }
    });
}

async fn execute_create_doc(input: &Value, state: &AppState, user_id: UserId) -> Value {
    let Some(doc_repo) = state.doc_repo() else {
        return json!({ "error": "Document repository not initialized" });
    };

    let Some(title) = input["title"].as_str() else {
        return json!({ "error": "title is required" });
    };
    let Some(content) = input["content"].as_str() else {
        return json!({ "error": "content is required" });
    };

    let tags: Vec<String> = input["tags"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let ref_tag = title_to_ref_tag(title);

    match doc_repo
        .create_document(
            user_id.0,
            None, // session_id
            title.to_string(),
            content.to_string(),
            "architecture".to_string(),
            ref_tag.clone(),
            tags,
        )
        .await
    {
        Ok(row) => {
            // Spawn background summary generation
            spawn_summary_task(Arc::clone(&doc_repo), row.id, content.to_string());

            json!({
                "doc_id": row.id.to_string(),
                "ref_tag": ref_tag,
                "title": title
            })
        }
        Err(e) => json!({ "error": e.to_string() }),
    }
}

async fn execute_update_doc(input: &Value, state: &AppState) -> Value {
    let Some(doc_repo) = state.doc_repo() else {
        return json!({ "error": "Document repository not initialized" });
    };

    let Some(id_str) = input["doc_id"].as_str() else {
        return json!({ "error": "doc_id is required" });
    };
    let Ok(doc_id) = uuid::Uuid::parse_str(id_str) else {
        return json!({ "error": format!("Invalid UUID: {}", id_str) });
    };

    let content = input["content"].as_str().map(String::from);
    let title = input["title"].as_str().map(String::from);
    let tags: Option<Vec<String>> = input["tags"].as_array().map(|arr| {
        arr.iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect()
    });

    match doc_repo
        .update_document(doc_id, content.clone(), title.clone(), tags)
        .await
    {
        Ok(row) => {
            // Spawn background summary regeneration using updated content
            let summary_content = content.unwrap_or(row.content.clone());
            spawn_summary_task(Arc::clone(&doc_repo), doc_id, summary_content);

            json!({
                "updated": true,
                "doc_id": doc_id.to_string(),
                "title": row.title
            })
        }
        Err(e) => json!({ "error": e.to_string() }),
    }
}

async fn execute_search_docs(input: &Value, state: &AppState, user_id: UserId) -> Value {
    let Some(doc_repo) = state.doc_repo() else {
        return json!({ "error": "Document repository not initialized" });
    };

    let Some(query) = input["query"].as_str() else {
        return json!({ "error": "query is required" });
    };

    match doc_repo.search_documents(user_id.0, query).await {
        Ok(results) => {
            let items: Vec<Value> = results
                .into_iter()
                .map(|r| {
                    json!({
                        "id": r.id.to_string(),
                        "title": r.title,
                        "ref_tag": r.ref_tag,
                        "summary": r.summary,
                        "snippet": r.snippet
                    })
                })
                .collect();
            json!({ "results": items, "count": items.len() })
        }
        Err(e) => json!({ "error": e.to_string() }),
    }
}

// --- Structured output validation tool handlers ---

async fn execute_submit_prd(input: &Value, state: &AppState, user_id: UserId) -> Value {
    let mut errors = Vec::new();

    // Validate required string fields
    let title = input["title"].as_str().unwrap_or("");
    if title.is_empty() {
        errors.push("Missing field: title".to_string());
    }
    let problem_statement = input["problem_statement"].as_str().unwrap_or("");
    if problem_statement.is_empty() {
        errors.push("Missing field: problem_statement".to_string());
    }
    let technical_approach = input["technical_approach"].as_str().unwrap_or("");
    if technical_approach.is_empty() {
        errors.push("Missing field: technical_approach".to_string());
    }

    // Validate required array fields
    let goals = input["goals"].as_array();
    if goals.is_none_or(|a| a.is_empty()) {
        errors.push("goals must have at least 1 entry".to_string());
    }
    let non_goals = input["non_goals"].as_array();
    if non_goals.is_none_or(|a| a.is_empty()) {
        errors.push("non_goals must have at least 1 entry".to_string());
    }
    let user_stories = input["user_stories"].as_array();
    if user_stories.is_none_or(|a| a.is_empty()) {
        errors.push("user_stories must have at least 1 entry".to_string());
    }

    // Validate milestones
    let milestones = input["milestones"].as_array();
    if milestones.is_none_or(|a| a.is_empty()) {
        errors.push("milestones must have at least 1 entry".to_string());
    } else if let Some(ms) = milestones {
        for (i, m) in ms.iter().enumerate() {
            if m["name"].as_str().unwrap_or("").is_empty() {
                errors.push(format!("milestones[{}] missing name", i));
            }
            if m["deliverables"].as_array().is_none_or(|a| a.is_empty()) {
                errors.push(format!(
                    "milestones[{}] must have at least 1 deliverable",
                    i
                ));
            }
        }
    }

    // Validate complexity
    let complexity = input["complexity"].as_str().unwrap_or("");
    if !matches!(complexity, "S" | "M" | "L" | "XL") {
        errors.push("complexity must be one of: S, M, L, XL".to_string());
    }

    if !errors.is_empty() {
        return json!({ "valid": false, "errors": errors });
    }

    // Format PRD as markdown
    let goals_arr = goals.unwrap();
    let non_goals_arr = non_goals.unwrap();
    let user_stories_arr = user_stories.unwrap();
    let milestones_arr = milestones.unwrap();

    let mut md = format!("# PRD: {}\n\n## Status: APPROVED\n\n", title);
    md.push_str(&format!(
        "## Problem Statement\n\n{}\n\n",
        problem_statement
    ));

    md.push_str("## Goals\n\n");
    for g in goals_arr {
        md.push_str(&format!("- {}\n", g.as_str().unwrap_or("")));
    }

    md.push_str("\n## Non-Goals\n\n");
    for ng in non_goals_arr {
        md.push_str(&format!("- {}\n", ng.as_str().unwrap_or("")));
    }

    md.push_str("\n## User Stories\n\n");
    for us in user_stories_arr {
        md.push_str(&format!("- {}\n", us.as_str().unwrap_or("")));
    }

    md.push_str(&format!(
        "\n## Technical Approach\n\n{}\n\n",
        technical_approach
    ));

    md.push_str("## Milestones\n\n");
    for m in milestones_arr {
        md.push_str(&format!("### {}\n\n", m["name"].as_str().unwrap_or("")));
        if let Some(deliverables) = m["deliverables"].as_array() {
            for d in deliverables {
                md.push_str(&format!("- {}\n", d.as_str().unwrap_or("")));
            }
        }
        md.push('\n');
    }

    md.push_str(&format!("## Complexity: {}\n\n", complexity));

    if let Some(metrics) = input["success_metrics"].as_array() {
        if !metrics.is_empty() {
            md.push_str("## Success Metrics\n\n");
            for m in metrics {
                md.push_str(&format!("- {}\n", m.as_str().unwrap_or("")));
            }
            md.push('\n');
        }
    }

    if let Some(risks) = input["risks"].as_array() {
        if !risks.is_empty() {
            md.push_str("## Risks\n\n");
            for r in risks {
                md.push_str(&format!("- {}\n", r.as_str().unwrap_or("")));
            }
            md.push('\n');
        }
    }

    // Store as document
    let Some(doc_repo) = state.doc_repo() else {
        return json!({ "error": "Document repository not initialized" });
    };

    let ref_tag = title_to_ref_tag(title);

    match doc_repo
        .create_document(
            user_id.0,
            None,
            title.to_string(),
            md.clone(),
            "prd".to_string(),
            ref_tag.clone(),
            vec!["prd".to_string()],
        )
        .await
    {
        Ok(row) => {
            spawn_summary_task(Arc::clone(&doc_repo), row.id, md);
            json!({
                "valid": true,
                "doc_id": row.id.to_string(),
                "ref_tag": ref_tag
            })
        }
        Err(e) => json!({ "error": e.to_string() }),
    }
}

async fn execute_submit_ticket(input: &Value) -> Value {
    let mut errors = Vec::new();

    let title = input["title"].as_str().unwrap_or("");
    if title.is_empty() {
        errors.push("Missing field: title".to_string());
    }
    let description = input["description"].as_str().unwrap_or("");
    if description.is_empty() {
        errors.push("Missing field: description".to_string());
    }

    let acceptance_criteria = input["acceptance_criteria"].as_array();
    if acceptance_criteria.is_none_or(|a| a.is_empty()) {
        errors.push("acceptance_criteria must have at least 1 entry".to_string());
    }
    let files_to_modify = input["files_to_modify"].as_array();
    if files_to_modify.is_none_or(|a| a.is_empty()) {
        errors.push("files_to_modify must have at least 1 entry".to_string());
    }

    let complexity = input["complexity"].as_str().unwrap_or("");
    if !matches!(complexity, "S" | "M" | "L" | "XL") {
        errors.push("complexity must be one of: S, M, L, XL".to_string());
    }

    let role = input["role"].as_str().unwrap_or("");
    if !matches!(role, "worker" | "reviewer" | "utility") {
        errors.push("role must be one of: worker, reviewer, utility".to_string());
    }

    if !errors.is_empty() {
        return json!({ "valid": false, "errors": errors });
    }

    let dependencies: Vec<String> = input["dependencies"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    json!({
        "valid": true,
        "ticket": {
            "title": title,
            "description": description,
            "acceptance_criteria": acceptance_criteria.unwrap(),
            "files_to_modify": files_to_modify.unwrap(),
            "complexity": complexity,
            "role": role,
            "dependencies": dependencies
        }
    })
}

#[cfg(test)]
mod tests;
