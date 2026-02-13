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

        // Task force archetype tools (6)
        "set_task" => Some(set_task_tool()),
        "add_agent" => Some(add_agent_tool()),
        "update_agent" => Some(update_agent_tool()),
        "remove_agent" => Some(remove_agent_tool()),
        "set_capabilities" => Some(set_capabilities_tool()),
        "set_failure_mode" => Some(set_failure_mode_tool()),

        // Belief capture archetype tools (4)
        "set_extraction_focus" => Some(set_extraction_focus_tool()),
        "set_tag_vocabulary" => Some(set_tag_vocabulary_tool()),
        "set_contradiction_handling" => Some(set_contradiction_handling_tool()),
        "set_confidence_threshold" => Some(set_confidence_threshold_tool()),

        // Room archetype tools (6)
        "set_meeting_purpose" => Some(set_meeting_purpose_tool()),
        "add_member" => Some(add_member_tool()),
        "update_member" => Some(update_member_tool()),
        "remove_member" => Some(remove_member_tool()),
        "set_max_turns" => Some(set_max_turns_tool()),
        "set_interaction_mode" => Some(set_interaction_mode_tool()),

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
// Belief Capture Archetype Tool Definitions
// ============================================================================

fn set_extraction_focus_tool() -> Tool {
    Tool {
        name: "set_extraction_focus".into(),
        description: "Set what the belief capture node should focus on when extracting beliefs from upstream results. Free-text guidance that shapes gatekeeper extraction.".into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "guidance": {
                    "type": "string",
                    "description": "What to focus on when extracting beliefs (e.g., 'Extract all vulnerability findings, severity assessments, and fix recommendations')"
                }
            },
            "required": ["guidance"]
        }),
    }
}

fn set_tag_vocabulary_tool() -> Tool {
    Tool {
        name: "set_tag_vocabulary".into(),
        description: "Set the allowed semantic tags for extracted beliefs. The gatekeeper can only use these tags, ensuring downstream queries match exactly.".into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "tags": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Allowed semantic tags (e.g., ['vulnerability', 'severity', 'fix', 'risk'])"
                }
            },
            "required": ["tags"]
        }),
    }
}

fn set_contradiction_handling_tool() -> Tool {
    Tool {
        name: "set_contradiction_handling".into(),
        description: "Set how contradictions between upstream sources are handled during belief extraction.".into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "mode": {
                    "type": "string",
                    "enum": ["flag", "resolve", "keep_both"],
                    "description": "flag: preserve both with tension note. resolve: pick the stronger claim. keep_both: store without annotation."
                }
            },
            "required": ["mode"]
        }),
    }
}

fn set_confidence_threshold_tool() -> Tool {
    Tool {
        name: "set_confidence_threshold".into(),
        description: "Set the minimum confidence level for extracted beliefs. Beliefs below this threshold are filtered out.".into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "threshold": {
                    "type": "string",
                    "enum": ["low", "medium", "high"],
                    "description": "low: keep everything. medium: filter speculative claims. high: only strong evidence."
                }
            },
            "required": ["threshold"]
        }),
    }
}

// ============================================================================
// Room Archetype Tool Definitions
// ============================================================================

fn set_meeting_purpose_tool() -> Tool {
    Tool {
        name: "set_meeting_purpose".into(),
        description: "Set the meeting purpose for this room node. Describes what the agents will discuss, debate, or review.".into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "description": {
                    "type": "string",
                    "description": "What the room meeting should accomplish (e.g., 'Review security audit findings, prioritize fixes, and agree on a remediation timeline')"
                }
            },
            "required": ["description"]
        }),
    }
}

fn add_member_tool() -> Tool {
    Tool {
        name: "add_member".into(),
        description: "Add a member to the room meeting. Each member has a name, role, and perspective that shapes their contributions.".into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Member name (e.g., 'Security Lead', 'Skeptic', 'Pragmatist')"
                },
                "role": {
                    "type": "string",
                    "description": "What this member does in the meeting (e.g., 'Presents findings and recommends priorities')"
                },
                "perspective": {
                    "type": "string",
                    "description": "Perspective or bias that shapes this member's contributions (e.g., 'Risk-averse, wants comprehensive fixes')"
                }
            },
            "required": ["name", "role"]
        }),
    }
}

fn update_member_tool() -> Tool {
    Tool {
        name: "update_member".into(),
        description: "Update an existing room member's role or perspective. Identifies the member by name.".into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Name of the member to update (case-insensitive match)"
                },
                "role": {
                    "type": "string",
                    "description": "New role description"
                },
                "perspective": {
                    "type": "string",
                    "description": "New perspective or bias"
                }
            },
            "required": ["name"]
        }),
    }
}

fn remove_member_tool() -> Tool {
    Tool {
        name: "remove_member".into(),
        description: "Remove a member from the room meeting.".into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Name of the member to remove (case-insensitive match)"
                }
            },
            "required": ["name"]
        }),
    }
}

fn set_max_turns_tool() -> Tool {
    Tool {
        name: "set_max_turns".into(),
        description: "Set the maximum number of discussion turns for the room meeting.".into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "count": {
                    "type": "integer",
                    "description": "Maximum turns (1-100). Typically 8-15 turns is sufficient."
                }
            },
            "required": ["count"]
        }),
    }
}

fn set_interaction_mode_tool() -> Tool {
    Tool {
        name: "set_interaction_mode".into(),
        description: "Set how agents take turns in the room meeting.".into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "mode": {
                    "type": "string",
                    "enum": ["round_robin", "moderated", "open_floor"],
                    "description": "round_robin: strict rotation by display order. moderated: gatekeeper selects speakers each turn. open_floor: agents respond to whoever they find most compelling."
                }
            },
            "required": ["mode"]
        }),
    }
}
