//! Static tool registry for mapping tool names to definitions.

use crate::llm::Tool;
use serde_json::json;

#[cfg(test)]
mod tests;

/// Get a tool definition by name.
///
/// Returns `Some(Tool)` if the tool exists, `None` otherwise.
///
/// # Examples
///
/// ```
/// use nexor::tools::get_tool_definition;
///
/// let tool = get_tool_definition("read_file").unwrap();
/// assert_eq!(tool.name, "read_file");
///
/// assert!(get_tool_definition("unknown_tool").is_none());
/// ```
pub fn get_tool_definition(name: &str) -> Option<Tool> {
    match name {
        // Execution tools (12)
        "read_file" => Some(read_file_tool()),
        "write_file" => Some(write_file_tool()),
        "edit_file" => Some(edit_file_tool()),
        "list_files" => Some(list_files_tool()),
        "git_status" => Some(git_status_tool()),
        "git_diff" => Some(git_diff_tool()),
        "git_add" => Some(git_add_tool()),
        "git_commit" => Some(git_commit_tool()),
        "git_branch" => Some(git_branch_tool()),
        "run_tests" => Some(run_tests_tool()),
        "run_command" => Some(run_command_tool()),
        "web_research" => Some(web_research_tool()),

        // Orchestrator tools (7 additional unique)
        "search_files" => Some(search_files_tool()),
        "think" => Some(think_tool()),
        "create_doc" => Some(create_doc_tool()),
        "update_doc" => Some(update_doc_tool()),
        "search_docs" => Some(search_docs_tool()),
        "submit_prd" => Some(submit_prd_tool()),
        "submit_ticket" => Some(submit_ticket_tool()),

        // Documenter assistant tools (4)
        "create_doc_def" => Some(create_doc_def_tool()),
        "update_doc_def" => Some(update_doc_def_tool()),
        "delete_doc_def" => Some(delete_doc_def_tool()),
        "update_config" => Some(update_config_tool()),

        // Universal node assistant tools (3)
        "set_node_archetype" => Some(set_node_archetype_tool()),
        "set_node_name" => Some(set_node_name_tool()),
        "set_node_description" => Some(set_node_description_tool()),

        _ => None,
    }
}

// ============================================================================
// Execution Tool Definitions (from src/agents/execution_tools.rs)
// ============================================================================

fn read_file_tool() -> Tool {
    Tool {
        name: "read_file".into(),
        description: "Read the contents of a file.".into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Relative path from project root"
                }
            },
            "required": ["path"]
        }),
    }
}

fn write_file_tool() -> Tool {
    Tool {
        name: "write_file".into(),
        description: "Write content to a file. Creates parent directories if needed.".into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Relative path from project root"
                },
                "content": {
                    "type": "string",
                    "description": "File content to write"
                }
            },
            "required": ["path", "content"]
        }),
    }
}

fn edit_file_tool() -> Tool {
    Tool {
        name: "edit_file".into(),
        description: "Edit a file by replacing old_string with new_string.".into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Relative path from project root"
                },
                "old_string": {
                    "type": "string",
                    "description": "Exact text to find"
                },
                "new_string": {
                    "type": "string",
                    "description": "Replacement text"
                }
            },
            "required": ["path", "old_string", "new_string"]
        }),
    }
}

fn list_files_tool() -> Tool {
    Tool {
        name: "list_files".into(),
        description: "List files and directories at a path.".into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Relative path from project root (empty = root)"
                }
            },
            "required": []
        }),
    }
}

fn git_status_tool() -> Tool {
    Tool {
        name: "git_status".into(),
        description: "Show git working tree status (modified, staged, untracked files).".into(),
        input_schema: json!({
            "type": "object",
            "properties": {},
            "required": []
        }),
    }
}

fn git_diff_tool() -> Tool {
    Tool {
        name: "git_diff".into(),
        description: "Show git changes (unstaged or staged).".into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "staged": {
                    "type": "boolean",
                    "description": "Show staged changes (default: unstaged)"
                }
            },
            "required": []
        }),
    }
}

fn git_add_tool() -> Tool {
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
    }
}

fn git_commit_tool() -> Tool {
    Tool {
        name: "git_commit".into(),
        description: "Create a git commit with staged changes.".into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "message": {
                    "type": "string",
                    "description": "Commit message"
                }
            },
            "required": ["message"]
        }),
    }
}

fn git_branch_tool() -> Tool {
    Tool {
        name: "git_branch".into(),
        description: "Get current branch or create/switch to a branch.".into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Branch name (empty = get current)"
                },
                "create": {
                    "type": "boolean",
                    "description": "Create branch if it doesn't exist"
                }
            },
            "required": []
        }),
    }
}

fn run_tests_tool() -> Tool {
    Tool {
        name: "run_tests".into(),
        description: "Run the project test suite.".into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "filter": {
                    "type": "string",
                    "description": "Test name filter (optional)"
                }
            },
            "required": []
        }),
    }
}

fn run_command_tool() -> Tool {
    Tool {
        name: "run_command".into(),
        description: "Execute a shell command in the project directory.".into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "Shell command to execute"
                }
            },
            "required": ["command"]
        }),
    }
}

fn web_research_tool() -> Tool {
    Tool {
        name: "web_research".into(),
        description: "Research via xAI Grok with web and X/Twitter search.".into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query"
                },
                "sources": {
                    "type": "array",
                    "items": {
                        "type": "string",
                        "enum": ["web", "x"]
                    },
                    "description": "Sources to search (web, x)"
                },
                "allowed_domains": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Restrict to specific domains (optional)"
                }
            },
            "required": ["query"]
        }),
    }
}

// ============================================================================
// Orchestrator Tool Definitions (from src/server/tools/mod.rs)
// ============================================================================

fn search_files_tool() -> Tool {
    Tool {
        name: "search_files".into(),
        description: "Search files in codebase with regex pattern.".into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Regex pattern to search for"
                },
                "path": {
                    "type": "string",
                    "description": "Directory to search (empty = all)"
                }
            },
            "required": ["pattern"]
        }),
    }
}

fn think_tool() -> Tool {
    Tool {
        name: "think".into(),
        description: "Internal reasoning scratchpad (no-op, for agent thinking).".into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "thought": {
                    "type": "string",
                    "description": "Internal thought or reasoning"
                }
            },
            "required": ["thought"]
        }),
    }
}

fn create_doc_tool() -> Tool {
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
                    "description": "Document content (markdown)"
                }
            },
            "required": ["title", "content"]
        }),
    }
}

fn update_doc_tool() -> Tool {
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
                    "description": "New content (markdown)"
                }
            },
            "required": ["document_id", "content"]
        }),
    }
}

fn search_docs_tool() -> Tool {
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
    }
}

fn submit_prd_tool() -> Tool {
    Tool {
        name: "submit_prd".into(),
        description: "Submit a Product Requirements Document with validation.".into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "title": {
                    "type": "string",
                    "description": "PRD title"
                },
                "problem_statement": {
                    "type": "string",
                    "description": "What problem does this solve?"
                },
                "goals": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "List of goals"
                },
                "milestones": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "name": { "type": "string" },
                            "deliverables": {
                                "type": "array",
                                "items": { "type": "string" }
                            }
                        },
                        "required": ["name", "deliverables"]
                    },
                    "description": "Milestones with deliverables"
                },
                "complexity": {
                    "type": "string",
                    "enum": ["S", "M", "L", "XL"],
                    "description": "Complexity estimate"
                }
            },
            "required": ["title", "problem_statement", "goals", "milestones", "complexity"]
        }),
    }
}

fn submit_ticket_tool() -> Tool {
    Tool {
        name: "submit_ticket".into(),
        description: "Submit a decomposition ticket with validation.".into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "title": {
                    "type": "string",
                    "description": "Ticket title"
                },
                "description": {
                    "type": "string",
                    "description": "Detailed description"
                },
                "acceptance_criteria": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Acceptance criteria checklist"
                }
            },
            "required": ["title", "description", "acceptance_criteria"]
        }),
    }
}

// ============================================================================
// Documenter Assistant Tool Definitions
// ============================================================================

fn create_doc_def_tool() -> Tool {
    Tool {
        name: "create_doc_def".into(),
        description: "Create a new document definition on the documenter step. The document will appear as a node on the workflow canvas.".into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Document name (e.g., 'API Reference', 'Migration Guide')"
                },
                "description": {
                    "type": "string",
                    "description": "What this document should contain and its purpose"
                },
                "target_length": {
                    "type": "integer",
                    "description": "Target word count. Short: 500-1000, Medium: 1500-3000, Long: 3000-6000"
                }
            },
            "required": ["name"]
        }),
    }
}

fn update_doc_def_tool() -> Tool {
    Tool {
        name: "update_doc_def".into(),
        description: "Update an existing document definition. Use read_context first to see current definitions.".into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "doc_def_id": {
                    "type": "string",
                    "description": "ID of the document definition to update"
                },
                "name": {
                    "type": "string",
                    "description": "New document name"
                },
                "description": {
                    "type": "string",
                    "description": "New document description"
                },
                "target_length": {
                    "type": "integer",
                    "description": "New target word count"
                }
            },
            "required": ["doc_def_id"]
        }),
    }
}

fn delete_doc_def_tool() -> Tool {
    Tool {
        name: "delete_doc_def".into(),
        description:
            "Delete a document definition. The corresponding node will be removed from the canvas."
                .into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "doc_def_id": {
                    "type": "string",
                    "description": "ID of the document definition to delete"
                }
            },
            "required": ["doc_def_id"]
        }),
    }
}

fn update_config_tool() -> Tool {
    Tool {
        name: "update_config".into(),
        description: "Update the documenter step's configuration. All fields are optional — only provided fields are changed.".into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Step name displayed on the canvas"
                },
                "description": {
                    "type": "string",
                    "description": "What this step does and what it provides"
                },
                "prompt_template": {
                    "type": "string",
                    "description": "Instruction prompt that controls how the documenter generates documents"
                }
            },
            "required": []
        }),
    }
}

// ============================================================================
// Universal Node Assistant Tool Definitions
// ============================================================================

fn set_node_archetype_tool() -> Tool {
    Tool {
        name: "set_node_archetype".into(),
        description: "Set the archetype (execution mode) for this node. This determines what the node does and which tools are available.".into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "archetype": {
                    "type": "string",
                    "enum": ["documenter", "task_force", "belief_capture", "room"],
                    "description": "The archetype to apply to this node"
                }
            },
            "required": ["archetype"]
        }),
    }
}

fn set_node_name_tool() -> Tool {
    Tool {
        name: "set_node_name".into(),
        description: "Set the display name for this node on the workflow canvas.".into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Display name for the node"
                }
            },
            "required": ["name"]
        }),
    }
}

fn set_node_description_tool() -> Tool {
    Tool {
        name: "set_node_description".into(),
        description: "Set the description for this node. Helps other assistants and users understand what the node does.".into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "description": {
                    "type": "string",
                    "description": "What this node does in the workflow"
                }
            },
            "required": ["description"]
        }),
    }
}
