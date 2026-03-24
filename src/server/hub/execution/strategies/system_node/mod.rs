//! SystemNodeStrategy — ExecutionStrategy for the system node agent.
//!
//! Replaces the builder + designer pipeline with a single ReAct agent
//! that writes JSON config files to a repository. The agent uses
//! `run_command` to write files and `complete_system` to signal done.

use std::path::PathBuf;
use std::sync::Mutex;

use async_trait::async_trait;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::config::protocols::{roles, SYSTEM_NODE_AGENT};
use crate::execution::ContainerHandle;
use crate::llm::{Message, Tool};
use crate::server::hub::error::HubError;
use crate::server::hub::strategy::ExecutionStrategy;
use crate::server::state::AppState;
use crate::tools::registry::get_tool_definition;

mod tests;
pub(crate) mod validate;

/// Strategy for the system node agent.
///
/// Runs in a container with `run_command` and `complete_system` tools.
/// Writes config.json, topology.json, and agents/*.json to a repository.
/// The execution engine reads these files after completion.
pub struct SystemNodeStrategy {
    system_prompt: String,
    instruction: String,
    state: AppState,
    step_id: Uuid,
    workflow_id: Uuid,
    session_id: Option<Uuid>,
    agent_execution_id: Option<Uuid>,
    /// Container handle for run_command execution.
    container_handle: Option<ContainerHandle>,
    /// Captured summary from complete_system tool call.
    summary: Mutex<Option<String>>,
    /// Base directory for the system node agent's repository.
    base_dir: PathBuf,
}

impl SystemNodeStrategy {
    /// Protocol config for the system node agent role.
    fn config(&self) -> &crate::config::protocols::AgentConfig {
        SYSTEM_NODE_AGENT.agent("system")
    }

    /// Build a new system node strategy.
    pub fn new(
        state: AppState,
        step_id: Uuid,
        workflow_id: Uuid,
        instruction: String,
        session_id: Option<Uuid>,
        container_handle: Option<ContainerHandle>,
        base_dir: PathBuf,
    ) -> Self {
        let current_state = build_current_state(&base_dir);
        let system_prompt = format!(
            "{}\n\n{}",
            roles::SYSTEM_NODE_AGENT_SYSTEM,
            current_state
        );

        Self {
            system_prompt,
            instruction,
            state,
            step_id,
            workflow_id,
            session_id,
            agent_execution_id: None,
            container_handle,
            summary: Mutex::new(None),
            base_dir,
        }
    }

    /// Set the agent execution ID (created after strategy construction).
    pub fn set_agent_execution_id(&mut self, id: Option<Uuid>) {
        self.agent_execution_id = id;
    }

    /// Take the captured summary from `complete_system`, if any.
    pub fn take_summary(&self) -> Option<String> {
        self.summary
            .lock()
            .ok()
            .and_then(|mut s| s.take())
    }
}

#[async_trait]
impl ExecutionStrategy for SystemNodeStrategy {
    fn system_prompt(&self) -> &str {
        &self.system_prompt
    }

    fn tools(&self) -> Vec<Tool> {
        let mut tools = Vec::with_capacity(3);
        if let Some(t) = get_tool_definition("run_command") {
            tools.push(t);
        }
        tools.push(complete_system_tool());
        if let Some(t) = get_tool_definition("think") {
            tools.push(t);
        }
        tools
    }

    fn model_id(&self) -> &str {
        &self.config().model_id
    }

    fn max_rounds(&self) -> u32 {
        self.config().max_rounds
    }

    fn context_budget(&self) -> usize {
        self.config().context_budget
    }

    fn streaming(&self) -> bool {
        false
    }

    fn temperature(&self) -> f32 {
        self.config().temperature
    }

    fn state(&self) -> Option<&AppState> {
        Some(&self.state)
    }

    fn agent_execution_id(&self) -> Option<Uuid> {
        self.agent_execution_id
    }

    fn should_stop(&self) -> bool {
        self.summary
            .lock()
            .map(|s| s.is_some())
            .unwrap_or(false)
    }

    async fn rebuild_system_prompt(&self) -> Result<Option<String>, HubError> {
        let current_state = build_current_state(&self.base_dir);
        Ok(Some(format!(
            "{}\n\n{}",
            roles::SYSTEM_NODE_AGENT_SYSTEM,
            current_state
        )))
    }

    async fn build_messages(&self, _input: &str) -> Result<Vec<Message>, HubError> {
        let text_instruction = if let Some(session_id) = self.session_id {
            let history = self
                .state
                .repos()
                .sessions
                .get_session_history(session_id, 20)
                .await
                .unwrap_or_default();

            if !history.is_empty() {
                super::build_pruned_instruction(&history, &self.instruction, 3)
            } else {
                self.instruction.clone()
            }
        } else {
            self.instruction.clone()
        };

        Ok(vec![Message::user(&text_instruction)])
    }

    async fn execute_tool(&self, name: &str, input: &Value) -> Value {
        match name {
            "complete_system" => {
                let summary = input["summary"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();

                let verify = &input["verify"];
                match validate::validate_verify(&self.base_dir, verify) {
                    Ok(mut success) => {
                        // Check if config.json description changed vs previous
                        // TODO: Compare against previous description (slice 3)
                        success["description_changed"] = json!(false);

                        if let Ok(mut guard) = self.summary.lock() {
                            *guard = Some(summary);
                        }
                        success
                    }
                    Err(error_response) => {
                        // Don't capture summary — agent needs to fix and retry
                        error_response
                    }
                }
            }
            "run_command" => {
                // TODO: Add write-time JSON validation (slice 1.4)
                // For now, pass through to container/local execution.
                crate::server::tools::execution::dispatch_tool_cascade(
                    name,
                    input,
                    self.container_handle.as_ref(),
                    None, // no local execution context
                    None, // no tool allow-list filter
                    Some(&self.state),
                    None, // no user_id for doc tools
                )
                .await
            }
            "think" => json!({ "status": "ok" }),
            _ => json!({ "error": format!("Unknown tool: {}", name) }),
        }
    }
}

// ---------------------------------------------------------------------------
// complete_system tool definition
// ---------------------------------------------------------------------------

/// Build the `complete_system` tool definition for the LLM.
pub(crate) fn complete_system_tool() -> Tool {
    Tool {
        name: "complete_system".into(),
        description: "Signal that you are done configuring the system. \
            Validates your repository — if something is wrong, you'll get \
            an error and can fix it."
            .into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "summary": {
                    "type": "string",
                    "description": "What you configured and key decisions (1-3 sentences)."
                },
                "verify": {
                    "type": "object",
                    "description": "Verify your work. Each boolean is you signing off that it's correct.",
                    "properties": {
                        "topology_complete": {
                            "type": "boolean",
                            "description": "The topology defines all agents and their dependencies are correct."
                        },
                        "agents_complete": {
                            "type": "boolean",
                            "description": "Every agent has a valid config with system_prompt, assignment, and expected_output."
                        },
                        "config_accurate": {
                            "type": "boolean",
                            "description": "config.json name and description accurately reflect this system."
                        }
                    },
                    "required": ["topology_complete", "agents_complete", "config_accurate"]
                }
            },
            "required": ["summary", "verify"]
        }),
    }
}

// ---------------------------------------------------------------------------
// current_state builder (stub — full implementation in slice 1.5)
// ---------------------------------------------------------------------------

/// Build the `<current_state>` XML from the repository filesystem.
///
/// Reads topology.json, agents/*.json, and config.json to produce a summary
/// of what exists, what's valid, and what's missing.
pub(crate) fn build_current_state(base_dir: &std::path::Path) -> String {
    let topology_path = base_dir.join("topology.json");
    let config_path = base_dir.join("config.json");
    let agents_dir = base_dir.join("agents");

    // Empty state — nothing exists yet
    if !topology_path.exists() && !config_path.exists() {
        return "<current_state refresh=\"every turn — always reflects the current filesystem\">\n  \
                <topology status=\"empty\" />\n  \
                <config status=\"missing\" />\n\
                </current_state>"
            .to_string();
    }

    let mut lines = Vec::new();
    lines.push(
        "<current_state refresh=\"every turn — always reflects the current filesystem\">".into(),
    );

    // Parse topology and render agent statuses
    if let Ok(content) = std::fs::read_to_string(&topology_path) {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(agents) = val.get("agents").and_then(|v| v.as_object()) {
                lines.push("  <topology>".into());

                for (slug, entry) in agents {
                    let deps: Vec<&str> = entry
                        .get("depends_on")
                        .and_then(|v| v.as_array())
                        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
                        .unwrap_or_default();
                    let depends_on = deps.join(", ");

                    let agent_path = agents_dir.join(format!("{slug}.json"));
                    let status = if agent_path.exists() {
                        "configured"
                    } else {
                        "missing"
                    };

                    lines.push(format!(
                        "    <agent slug=\"{slug}\" depends_on=\"{depends_on}\" status=\"{status}\" />"
                    ));
                }

                lines.push("  </topology>".into());
            } else {
                lines.push("  <topology status=\"invalid\" />".into());
            }
        } else {
            lines.push("  <topology status=\"invalid\" />".into());
        }
    } else {
        lines.push("  <topology status=\"empty\" />".into());
    }

    // Config status
    if let Ok(content) = std::fs::read_to_string(&config_path) {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
            let name = val
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if name.is_empty() {
                lines.push("  <config status=\"configured\" />".into());
            } else {
                lines.push(format!("  <config name=\"{name}\" status=\"configured\" />"));
            }
        } else {
            lines.push("  <config status=\"invalid\" />".into());
        }
    } else {
        lines.push("  <config status=\"missing\" />".into());
    }

    lines.push("</current_state>".into());
    lines.join("\n")
}
