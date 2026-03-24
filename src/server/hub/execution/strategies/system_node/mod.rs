//! SystemNodeStrategy — ExecutionStrategy for the system node agent.
//!
//! Thin strategy impl that delegates to shared services for validation,
//! state building, and file reading. Uses `run_command` for shell access
//! and `complete_system` to signal completion.

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
use crate::server::services::system_node::{file_reader, state, validate};
use crate::server::state::AppState;
use crate::server::tools::system_node::complete_system_tool;
use crate::tools::registry::get_tool_definition;

mod tests;

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
        let current_state = state::build_current_state(&base_dir);
        let system_prompt = format!("{}\n\n{}", roles::SYSTEM_NODE_AGENT_SYSTEM, current_state);

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
        self.summary.lock().ok().and_then(|mut s| s.take())
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
        self.summary.lock().map(|s| s.is_some()).unwrap_or(false)
    }

    async fn rebuild_system_prompt(&self) -> Result<Option<String>, HubError> {
        let current_state = state::build_current_state(&self.base_dir);
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
                let summary = input["summary"].as_str().unwrap_or("").to_string();

                let verify = &input["verify"];
                match validate::validate_verify(&self.base_dir, verify) {
                    Ok(mut success) => {
                        // Compare config.json description against previous designer_handoff
                        let description_changed = match file_reader::read_config(&self.base_dir) {
                            Ok((_name, description)) => {
                                match self.state.repos().workflows.get_step(self.step_id).await {
                                    Ok(Some(step)) => step.designer_handoff != description,
                                    _ => true,
                                }
                            }
                            Err(_) => false,
                        };
                        success["description_changed"] = json!(description_changed);

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
                let result = crate::server::tools::execution::dispatch_tool_cascade(
                    name,
                    input,
                    self.container_handle.as_ref(),
                    None, // no local execution context
                    None, // no tool allow-list filter
                    Some(&self.state),
                    None, // no user_id for doc tools
                )
                .await;

                // Post-execution write validation: check any system node JSON files
                // that exist on disk. If a heredoc write produced truncated/invalid JSON,
                // append errors to the tool response so the agent can fix immediately.
                let validation_errors = validate_written_files(&self.base_dir);
                if validation_errors.is_empty() {
                    result
                } else {
                    // Append validation errors to the run_command output
                    let mut patched = result.clone();
                    let warnings = validation_errors.join("\n");
                    if let Some(output) = patched.get_mut("output") {
                        let existing = output.as_str().unwrap_or("");
                        *output = json!(format!(
                            "{}\n\n⚠ Write validation errors:\n{}",
                            existing, warnings
                        ));
                    } else {
                        patched["write_validation_errors"] = json!(validation_errors);
                    }
                    patched
                }
            }
            "think" => json!({ "status": "ok" }),
            _ => json!({ "error": format!("Unknown tool: {}", name) }),
        }
    }
}

/// Validate JSON files in the system node agent's repository after a write.
///
/// Checks config.json, topology.json, and agents/*.json for structural validity.
/// Returns a vec of human-readable error strings (empty if all valid).
/// Only validates files that exist — missing files are not errors here
/// (the agent may write them in a subsequent command).
fn validate_written_files(base_dir: &std::path::Path) -> Vec<String> {
    let mut errors = Vec::new();

    // Validate config.json
    let config_path = base_dir.join("config.json");
    if config_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&config_path) {
            if let Err(e) = validate::validate_config(&content) {
                errors.push(format!("config.json: {e}"));
            }
        }
    }

    // Validate topology.json
    let topology_path = base_dir.join("topology.json");
    if topology_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&topology_path) {
            if let Err(e) = validate::validate_topology(&content) {
                errors.push(format!("topology.json: {e}"));
            }
        }
    }

    // Validate agents/*.json
    let agents_dir = base_dir.join("agents");
    if agents_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&agents_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|ext| ext == "json") {
                    let filename = path.file_name().unwrap_or_default().to_string_lossy();
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        if let Err(e) = validate::validate_agent(&content) {
                            errors.push(format!("agents/{filename}: {e}"));
                        }
                    }
                }
            }
        }
    }

    errors
}
