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
        // Execution tools (11)
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

        // Shared tools (used by chat, dispatch, and execution agents)
        "think" => Some(think_tool()),
        "create_doc" => Some(create_doc_tool()),
        "update_doc" => Some(update_doc_tool()),
        "search_docs" => Some(search_docs_tool()),
        "read_document" => Some(read_document_tool()),

        // Universal node assistant tools (5)
        "set_node_archetype" => Some(set_node_archetype_tool()),
        "set_node_name" => Some(set_node_name_tool()),
        "set_node_description" => Some(set_node_description_tool()),
        "render_panel" => Some(render_panel_tool()),
        "update_plan" => Some(update_plan_tool()),

        // Workforce archetype tools (6)
        "set_task" => Some(set_task_tool()),
        "add_agent" => Some(add_agent_tool()),
        "update_agent" => Some(update_agent_tool()),
        "remove_agent" => Some(remove_agent_tool()),
        "set_capabilities" => Some(set_capabilities_tool()),
        "set_failure_mode" => Some(set_failure_mode_tool()),

        "configure_team" => Some(configure_team_tool()),
        "set_dependency" => Some(set_dependency_tool()),
        "remove_dependency" => Some(remove_dependency_tool()),

        // Dispatch tools (background service layer)
        "dispatch" => Some(dispatch_tool()),
        "cancel_dispatch" => Some(cancel_dispatch_tool()),

        // Agent messaging tools
        "send_message" => Some(send_message_tool()),
        "dispatch_to_nodes" => Some(dispatch_to_nodes_tool()),
        "dispatch_to_builders" => Some(dispatch_to_builders_tool()),

        // Builder completion tool
        "complete_task" => Some(complete_task_tool()),

        // Manager topology tools (6)
        "create_pipeline" => Some(create_pipeline_tool()),
        "create_parallel" => Some(create_parallel_tool()),
        "insert_node" => Some(insert_node_tool()),
        "remove_node" => Some(remove_node_tool()),
        "wire_edge" => Some(wire_edge_tool()),
        "remove_edge" => Some(remove_edge_tool()),

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

// ============================================================================
// Shared Tool Definitions
// ============================================================================

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
            "Render an interactive panel inline in chat. The panel appears as a structured ",
            "message with interactive elements the user can fill out and submit.\n\n",
            "Use markdown headings to organize sections:\n",
            "- `# Heading` for top-level sections\n",
            "- `## Heading` for sub-sections\n",
            "- `### Heading` for nested sub-sections\n\n",
            "Interactive elements:\n",
            "- `- [ ] Option` renders as a checkbox the user can toggle\n",
            "- `- [x] Option` renders as a pre-checked checkbox\n",
            "- `- [> Label]` renders as a text input field where the user types a value\n\n",
            "A free-form \"Notes\" text field is always appended to the bottom of every panel ",
            "automatically — do NOT add one yourself.\n\n",
            "Regular markdown (paragraphs, bullets, tables, bold, code blocks, blockquotes) renders ",
            "normally. Use standard `- item` bullets for informational lists, ",
            "`- [ ] item` checkboxes for choices, and `- [> label]` for text the user should provide.\n\n",
            "Use this tool when:\n",
            "- Proposing a plan the user should approve before you execute\n",
            "- Presenting options where the user needs to choose\n",
            "- Collecting configuration values (names, paths, parameters)\n",
            "- Showing structured information (rosters, configs, summaries)\n\n",
            "This tool is for human interaction only. Do NOT use it to respond to agent ",
            "messages or other system events — only render a panel when the human user needs ",
            "to see, configure, or approve something.\n\n",
            "Do NOT use for simple yes/no questions or short responses where chat is ",
            "sufficient. The user fills out the panel and submits. Their choices come back ",
            "as a structured message.",
        ).into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "content": {
                    "type": "string",
                    "description": "Markdown content for the panel. Use headings for sections, `- [ ]` for checkboxes, and `- [> Label]` for text inputs."
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

fn update_plan_tool() -> Tool {
    Tool {
        name: "update_plan".into(),
        description: concat!(
            "Update the execution plan for this node. The plan persists across conversations and is ",
            "injected into the workflow designer at execution time. Use this to record the execution ",
            "blueprint, key decisions, special requirements, API details, or infrastructure ",
            "constraints. You can reorganize, prune, or rewrite the plan at any time — the full ",
            "content is replaced on each call.",
        )
        .into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "content": {
                    "type": "string",
                    "description": "The complete updated plan content (replaces all previous plan). Use markdown formatting. Keep the plan concise and actionable."
                }
            },
            "required": ["content"]
        }),
    }
}

// ============================================================================
// Workforce Archetype Tool Definitions
// ============================================================================

fn set_task_tool() -> Tool {
    Tool {
        name: "set_task".into(),
        description: "Set the mission description for this workforce node. Describes what the team of agents should accomplish.".into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "description": {
                    "type": "string",
                    "description": "What the workforce should accomplish"
                }
            },
            "required": ["description"]
        }),
    }
}

fn add_agent_tool() -> Tool {
    Tool {
        name: "add_agent".into(),
        description: "Add an agent to the workforce roster. Each agent has a name, role, and capabilities that determine what tools they can use at runtime.".into(),
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
                    "description": "Tool capabilities: file_read, file_write, content_search, shell, document_read, database_query"
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
            "Update an existing agent in the workforce roster. Only provided fields are changed."
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
        description: "Remove an agent from the workforce roster.".into(),
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
        description: "Set the available capabilities for the workforce. These determine what tools agents can be assigned.".into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "capabilities": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Available capabilities: file_read, file_write, content_search, shell, document_read, database_query"
                }
            },
            "required": ["capabilities"]
        }),
    }
}

fn set_failure_mode_tool() -> Tool {
    Tool {
        name: "set_failure_mode".into(),
        description: "Set how the workforce handles agent failures during execution.".into(),
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

fn configure_team_tool() -> Tool {
    Tool {
        name: "configure_team".into(),
        description: concat!(
            "Declaratively configure the full team for this node in a single call. ",
            "Provide the complete desired state: task description, agent roster, and ",
            "dependencies. The tool diffs against the current state and applies only ",
            "the changes needed — creating new agents, removing agents not in the spec, ",
            "updating agents whose role or capabilities changed, and reconciling ",
            "dependencies. Use this for initial team setup or full rebuilds. For ",
            "single-agent tweaks after setup, use add_agent/update_agent/remove_agent. ",
            "Plans are managed separately via update_plan.",
        )
        .into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "task": {
                    "type": "string",
                    "description": "Mission description: what the team produces, what inputs they work from, what success looks like"
                },
                "agents": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "name": {
                                "type": "string",
                                "description": "Agent name (e.g. 'Scanner', 'Analyzer', 'Reporter')"
                            },
                            "role_description": {
                                "type": "string",
                                "description": "What this agent does — domain expertise, approach, scope boundaries, output expectations"
                            },
                            "capabilities": {
                                "type": "array",
                                "items": { "type": "string" },
                                "description": "Tool capabilities: file_read, file_write, content_search, shell, document_read, database_query"
                            }
                        },
                        "required": ["name", "role_description"]
                    },
                    "description": "Complete agent roster in data-flow order (producers before consumers)"
                },
                "dependencies": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "from": {
                                "type": "string",
                                "description": "Upstream agent name (produces output)"
                            },
                            "to": {
                                "type": "string",
                                "description": "Downstream agent name (receives output)"
                            }
                        },
                        "required": ["from", "to"]
                    },
                    "description": "Data routing between agents. Each dependency means to_agent receives from_agent's output"
                }
            },
            "required": ["task", "agents"]
        }),
    }
}

// ============================================================================
// Workforce Dependency Tool Definitions
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
            "(add_agent, set_task, configure_team, etc.) on your behalf. You stay responsive ",
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

// ============================================================================
// Agent Messaging Tool Definitions
// ============================================================================

fn send_message_tool() -> Tool {
    Tool {
        name: "send_message".into(),
        description: concat!(
            "Send a message to a node assistant's chat session. The message is delivered in ",
            "real-time and appears in the node's conversation history. The node assistant ",
            "processes it on its next turn and can respond, dispatch configuration changes, ",
            "or raise questions. Use this to provide instructions, context, or updates to ",
            "individual nodes.",
        )
        .into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "step_id": {
                    "type": "string",
                    "description": "UUID of the target workflow step (node) to send the message to"
                },
                "message_type": {
                    "type": "string",
                    "enum": ["initial_instruction", "update", "upstream_change", "coordination", "feedback"],
                    "description": "Type of message: initial_instruction (first contact), update (new info or answers), upstream_change (a connected node changed), coordination (cross-node), feedback (post-run results)"
                },
                "content": {
                    "type": "string",
                    "description": "The message content to deliver to the node assistant"
                }
            },
            "required": ["step_id", "message_type", "content"]
        }),
    }
}

fn dispatch_to_nodes_tool() -> Tool {
    Tool {
        name: "dispatch_to_nodes".into(),
        description: concat!(
            "Send instructions to multiple node assistants in one action. Each message is ",
            "delivered to the node's chat session and triggers an automatic response. ",
            "Reference nodes by name (e.g. \"Collector\") or ref ID (e.g. \"workforce-1\"). ",
            "Messages are sent concurrently and each node processes them independently.",
        )
        .into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "messages": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "node": {
                                "type": "string",
                                "description": "Node name (e.g. \"Collector\") or ref ID (e.g. \"workforce-1\")"
                            },
                            "message_type": {
                                "type": "string",
                                "enum": ["initial_instruction", "update", "upstream_change", "coordination", "feedback"],
                                "description": "Type of message: initial_instruction (first contact), update (new info), upstream_change (topology change), coordination (cross-node sync), feedback (post-run)"
                            },
                            "content": {
                                "type": "string",
                                "description": "The instruction or message content for the node assistant"
                            }
                        },
                        "required": ["node", "message_type", "content"]
                    },
                    "description": "Array of messages, one per target node"
                }
            },
            "required": ["messages"]
        }),
    }
}

fn dispatch_to_builders_tool() -> Tool {
    Tool {
        name: "dispatch_to_builders".into(),
        description: concat!(
            "Send configuration instructions directly to node builders (L4), bypassing the ",
            "node assistants (L3). Each instruction spawns a background builder agent that ",
            "configures the node's workforce team. Use this for configuration tasks. Use ",
            "dispatch_to_nodes for conversational messages to node assistants.",
        )
        .into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "messages": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "node": {
                                "type": "string",
                                "description": "Node name (e.g. \"Collector\") or ref ID (e.g. \"workforce-1\")"
                            },
                            "instruction": {
                                "type": "string",
                                "description": "Configuration instruction for the node builder"
                            }
                        },
                        "required": ["node", "instruction"]
                    },
                    "description": "Array of instructions, one per target node builder"
                }
            },
            "required": ["messages"]
        }),
    }
}

// ============================================================================
// Builder Completion Tool Definition
// ============================================================================

fn complete_task_tool() -> Tool {
    Tool {
        name: "complete_task".into(),
        description: concat!(
            "Signal that you have finished configuring this node. Call this once when done. ",
            "Provide a plan for the designer (what the team should accomplish and how), a ",
            "summary of what you configured, and optionally a question if you need user input ",
            "to proceed. Do not call any tools after calling complete_task.",
        )
        .into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "plan": {
                    "type": "string",
                    "description": "Plan for the designer — what the team should accomplish, key decisions, execution direction. This feeds into the agent designer at execution time."
                },
                "summary": {
                    "type": "string",
                    "description": "What you configured and key decisions made (1-3 sentences). Displayed to the manager/user."
                },
                "question": {
                    "type": "string",
                    "description": "Question for the manager/user if you need input to proceed. Leave null if no question.",
                    "nullable": true
                }
            },
            "required": ["plan", "summary"]
        }),
    }
}

// ============================================================================
// Manager Topology Tool Definitions
// ============================================================================

fn create_pipeline_tool() -> Tool {
    Tool {
        name: "create_pipeline".into(),
        description: concat!(
            "Create workforce nodes wired in sequence. Returns the created nodes with their ",
            "ref IDs. Node names must be unique within the workflow — duplicate names are ",
            "rejected. Optionally connect to an existing source node that feeds into the ",
            "first new node.",
        )
        .into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "source": {
                    "type": "string",
                    "description": "Existing node name or ref ID to wire before the first new node (optional)"
                },
                "nodes": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "name": {
                                "type": "string",
                                "description": "Unique display name for the node"
                            },
                            "description": {
                                "type": "string",
                                "description": "What this node does in the workflow"
                            }
                        },
                        "required": ["name"]
                    },
                    "minItems": 1,
                    "description": "Nodes to create, wired in order"
                }
            },
            "required": ["nodes"]
        }),
    }
}

fn create_parallel_tool() -> Tool {
    Tool {
        name: "create_parallel".into(),
        description: concat!(
            "Create workforce nodes in parallel — fan-out from an optional source, fan-in to ",
            "an optional target. Source and target must be existing nodes (name or ref ID). ",
            "Node names must be unique within the workflow.",
        )
        .into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "source": {
                    "type": "string",
                    "description": "Existing node to fan out from (optional)"
                },
                "parallel": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "name": {
                                "type": "string",
                                "description": "Unique display name for the node"
                            },
                            "description": {
                                "type": "string",
                                "description": "What this node does in the workflow"
                            }
                        },
                        "required": ["name"]
                    },
                    "minItems": 2,
                    "description": "Nodes to create in parallel"
                },
                "target": {
                    "type": "string",
                    "description": "Existing node to fan in to (optional)"
                }
            },
            "required": ["parallel"]
        }),
    }
}

fn insert_node_tool() -> Tool {
    Tool {
        name: "insert_node".into(),
        description: concat!(
            "Insert a new workforce node between two existing connected nodes. Removes the ",
            "direct edge and wires through the new node. The node name must be unique within ",
            "the workflow.",
        )
        .into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "from": {
                    "type": "string",
                    "description": "Upstream node name or ref ID"
                },
                "to": {
                    "type": "string",
                    "description": "Downstream node name or ref ID"
                },
                "node": {
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "Unique display name for the new node"
                        },
                        "description": {
                            "type": "string",
                            "description": "What this node does in the workflow"
                        }
                    },
                    "required": ["name"]
                }
            },
            "required": ["from", "to", "node"]
        }),
    }
}

fn remove_node_tool() -> Tool {
    Tool {
        name: "remove_node".into(),
        description: concat!(
            "Remove a node from the workflow. If reconnect is true (default), wires its ",
            "predecessors directly to its successors to maintain flow.",
        )
        .into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "node": {
                    "type": "string",
                    "description": "Node name or ref ID to remove"
                },
                "reconnect": {
                    "type": "boolean",
                    "description": "Wire predecessors to successors after removal (default: true)"
                }
            },
            "required": ["node"]
        }),
    }
}

fn wire_edge_tool() -> Tool {
    Tool {
        name: "wire_edge".into(),
        description: "Add a connection between two existing nodes. Accepts node names or ref IDs."
            .into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "from": {
                    "type": "string",
                    "description": "Upstream node name or ref ID"
                },
                "to": {
                    "type": "string",
                    "description": "Downstream node name or ref ID"
                }
            },
            "required": ["from", "to"]
        }),
    }
}

fn remove_edge_tool() -> Tool {
    Tool {
        name: "remove_edge".into(),
        description: "Remove a connection between two nodes. Accepts node names or ref IDs.".into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "from": {
                    "type": "string",
                    "description": "Upstream node name or ref ID"
                },
                "to": {
                    "type": "string",
                    "description": "Downstream node name or ref ID"
                }
            },
            "required": ["from", "to"]
        }),
    }
}
