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

        // Orchestrator tools (8 additional unique)
        "search_files" => Some(search_files_tool()),
        "think" => Some(think_tool()),
        "create_doc" => Some(create_doc_tool()),
        "update_doc" => Some(update_doc_tool()),
        "search_docs" => Some(search_docs_tool()),
        "read_document" => Some(read_document_tool()),
        "submit_prd" => Some(submit_prd_tool()),
        "submit_ticket" => Some(submit_ticket_tool()),

        "update_config" => Some(update_config_tool()),

        // Universal node assistant tools (5)
        "set_node_archetype" => Some(set_node_archetype_tool()),
        "set_node_name" => Some(set_node_name_tool()),
        "set_node_description" => Some(set_node_description_tool()),
        "render_panel" => Some(render_panel_tool()),
        "update_notes" => Some(update_notes_tool()),

        // Task force archetype tools (6)
        "set_task" => Some(set_task_tool()),
        "add_agent" => Some(add_agent_tool()),
        "update_agent" => Some(update_agent_tool()),
        "remove_agent" => Some(remove_agent_tool()),
        "set_capabilities" => Some(set_capabilities_tool()),
        "set_failure_mode" => Some(set_failure_mode_tool()),

        "set_dependency" => Some(set_dependency_tool()),
        "remove_dependency" => Some(remove_dependency_tool()),

        // Dispatch tools (background service layer)
        "dispatch" => Some(dispatch_tool()),
        "cancel_dispatch" => Some(cancel_dispatch_tool()),

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

fn read_document_tool() -> Tool {
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
                    "enum": ["workforce"],
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

fn render_panel_tool() -> Tool {
    Tool {
        name: "render_panel".into(),
        description: concat!(
            "Render an interactive panel overlay on the node. The panel appears as a visual ",
            "card over the chat area. Use markdown with heading levels to create nested visual cards:\n\n",
            "- # Heading creates the outer card (highest elevation, strong shadow)\n",
            "- ## Heading creates inner cards (medium elevation)\n",
            "- ### Heading creates sub-sections within inner cards\n\n",
            "Interactive elements:\n",
            "- `- [ ] Option` renders as a checkbox the user can toggle\n",
            "- `- [x] Option` renders as a pre-checked checkbox\n\n",
            "Regular markdown (paragraphs, bullets, tables, bold, code blocks, blockquotes) renders ",
            "normally inside cards. Use standard `- item` bullets for informational lists and ",
            "`- [ ] item` checkboxes for choices the user should make.\n\n",
            "Use this tool when:\n",
            "- Proposing a plan the user should approve before you execute\n",
            "- Presenting options where the user needs to choose\n",
            "- Showing structured information (rosters, configs, summaries)\n\n",
            "Do NOT use for simple yes/no questions or short responses where chat is ",
            "sufficient. The user sees the panel as an overlay, makes selections, and ",
            "submits. Their choices come back as a structured message.",
        ).into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "content": {
                    "type": "string",
                    "description": "Markdown content for the panel. Use headings for nested cards and `- [ ]` checkboxes for interactive choices."
                },
                "submit_label": {
                    "type": "string",
                    "description": "Label for the submit button (default: 'Submit')"
                }
            },
            "required": ["content"]
        }),
    }
}

fn update_notes_tool() -> Tool {
    Tool {
        name: "update_notes".into(),
        description: concat!(
            "Update your personal notes. These notes persist across conversations and are ",
            "injected into the workflow designer at execution time. Use this to record important ",
            "discoveries, direction changes, special requirements, API details, or infrastructure ",
            "notes. You can reorganize, prune, or rewrite your notes at any time — the full ",
            "content is replaced on each call.",
        )
        .into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "content": {
                    "type": "string",
                    "description": "The complete updated notes content (replaces all previous notes). Use markdown formatting. Keep notes concise and actionable."
                }
            },
            "required": ["content"]
        }),
    }
}

// ============================================================================
// Task Force Archetype Tool Definitions
// ============================================================================

fn set_task_tool() -> Tool {
    Tool {
        name: "set_task".into(),
        description: "Set the mission description for this task force node. Describes what the team of agents should accomplish.".into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "description": {
                    "type": "string",
                    "description": "What the task force should accomplish"
                }
            },
            "required": ["description"]
        }),
    }
}

fn add_agent_tool() -> Tool {
    Tool {
        name: "add_agent".into(),
        description: "Add an agent to the task force roster. Each agent has a name, role, and capabilities that determine what tools they can use at runtime.".into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Agent name (e.g., 'Scanner', 'Developer', 'Tester')"
                },
                "role": {
                    "type": "string",
                    "description": "What this agent does in the mission"
                },
                "capabilities": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Tool capabilities: file_read, file_write, grep, shell, git, github_api, web_search, database_query"
                }
            },
            "required": ["name"]
        }),
    }
}

fn update_agent_tool() -> Tool {
    Tool {
        name: "update_agent".into(),
        description:
            "Update an existing agent in the task force roster. Only provided fields are changed."
                .into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "agent_id": {
                    "type": "string",
                    "description": "ID of the agent to update"
                },
                "name": {
                    "type": "string",
                    "description": "New agent name"
                },
                "role": {
                    "type": "string",
                    "description": "New role description"
                },
                "capabilities": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "New capabilities list (replaces existing)"
                }
            },
            "required": ["agent_id"]
        }),
    }
}

fn remove_agent_tool() -> Tool {
    Tool {
        name: "remove_agent".into(),
        description: "Remove an agent from the task force roster.".into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "agent_id": {
                    "type": "string",
                    "description": "ID of the agent to remove"
                }
            },
            "required": ["agent_id"]
        }),
    }
}

fn set_capabilities_tool() -> Tool {
    Tool {
        name: "set_capabilities".into(),
        description: "Set the available capabilities for the task force. These determine what tools agents can be assigned.".into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "capabilities": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Available capabilities: file_read, file_write, grep, shell, git, github_api, web_search, database_query"
                }
            },
            "required": ["capabilities"]
        }),
    }
}

fn set_failure_mode_tool() -> Tool {
    Tool {
        name: "set_failure_mode".into(),
        description: "Set how the task force handles agent failures during execution.".into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "mode": {
                    "type": "string",
                    "enum": ["fail_fast", "skip_and_continue", "retry"],
                    "description": "fail_fast: stop on first failure. skip_and_continue: skip failed agent, continue with rest. retry: retry the failed agent."
                }
            },
            "required": ["mode"]
        }),
    }
}

// ============================================================================
// Workforce Archetype Tool Definitions (deliverable-specific; agent tools shared)
// ============================================================================

fn set_dependency_tool() -> Tool {
    Tool {
        name: "set_dependency".into(),
        description: "Create a dependency between two agents. The to_agent will receive the output of from_agent before executing.".into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "from_agent": {
                    "type": "string",
                    "description": "Name of the upstream agent (produces output)"
                },
                "to_agent": {
                    "type": "string",
                    "description": "Name of the downstream agent (depends on from_agent's output)"
                }
            },
            "required": ["from_agent", "to_agent"]
        }),
    }
}

fn remove_dependency_tool() -> Tool {
    Tool {
        name: "remove_dependency".into(),
        description: "Remove a dependency between two agents.".into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "from_agent": {
                    "type": "string",
                    "description": "Name of the upstream agent"
                },
                "to_agent": {
                    "type": "string",
                    "description": "Name of the downstream agent"
                }
            },
            "required": ["from_agent", "to_agent"]
        }),
    }
}

// ============================================================================
// Dispatch Tool Definitions (background service layer)
// ============================================================================

fn dispatch_tool() -> Tool {
    Tool {
        name: "dispatch".into(),
        description: concat!(
            "Send a plain English instruction to a background agent that will configure this ",
            "step. The background agent loads the current step state and calls mutation tools ",
            "(add_agent, set_task, add_deliverable, etc.) on your behalf. You stay responsive ",
            "while the work happens in the background. Use this instead of calling mutation ",
            "tools directly.",
        )
        .into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "instruction": {
                    "type": "string",
                    "description": "Plain English instruction describing what to configure (e.g., 'Add a researcher agent focused on market trends and a writer agent for the final report')"
                }
            },
            "required": ["instruction"]
        }),
    }
}

fn cancel_dispatch_tool() -> Tool {
    Tool {
        name: "cancel_dispatch".into(),
        description: "Cancel a running background dispatch task.".into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "execution_id": {
                    "type": "string",
                    "description": "UUID of the dispatch task to cancel"
                }
            },
            "required": ["execution_id"]
        }),
    }
}
