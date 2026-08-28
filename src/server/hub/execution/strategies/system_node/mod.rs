//! SystemNodeStrategy — ExecutionStrategy for the system node agent.
//!
//! Thin strategy impl that delegates to shared services for validation,
//! state building, and file reading. Uses `run_command` for shell access
//! and `complete_system` to signal completion.

use std::path::{Path, PathBuf};
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
    _workflow_id: Uuid,
    session_id: Option<Uuid>,
    agent_execution_id: Option<Uuid>,
    container_handle: Option<ContainerHandle>,
    summary: Mutex<Option<String>>,
    base_dir: PathBuf,
}

impl SystemNodeStrategy {
    fn config(&self) -> &crate::config::protocols::AgentConfig {
        SYSTEM_NODE_AGENT.agent("system")
    }

    pub fn new(
        state: AppState,
        step_id: Uuid,
        workflow_id: Uuid,
        instruction: String,
        session_id: Option<Uuid>,
        container_handle: Option<ContainerHandle>,
        base_dir: PathBuf,
    ) -> Self {
        // System prompt is static — <current_state> rides the instruction
        // instead (see build_messages) so the prompt stays cacheable.
        let system_prompt = roles::SYSTEM_NODE_AGENT_SYSTEM.to_string();

        Self {
            system_prompt,
            instruction,
            state,
            step_id,
            _workflow_id: workflow_id,
            session_id,
            agent_execution_id: None,
            container_handle,
            summary: Mutex::new(None),
            base_dir,
        }
    }

    pub fn set_agent_execution_id(&mut self, id: Option<Uuid>) {
        self.agent_execution_id = id;
    }

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

    fn max_tokens(&self) -> u32 {
        self.config().max_tokens
    }

    fn effort(&self) -> Option<crate::llm::ReasoningEffort> {
        self.config().effort
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

    fn requires_terminal_tool(&self) -> Option<&str> {
        Some("complete_system")
    }

    async fn build_messages(&self, _input: &str) -> Result<Vec<Message>, HubError> {
        let text_instruction = match self.session_id {
            Some(session_id) => {
                let history = self
                    .state
                    .repos()
                    .sessions
                    .get_session_history(session_id, 20)
                    .await
                    .unwrap_or_default();

                if history.is_empty() {
                    self.instruction.clone()
                } else {
                    super::build_pruned_instruction(&history, &self.instruction, 3)
                }
            }
            None => self.instruction.clone(),
        };

        // One dispatch is one execution, so this is exactly once per generate.
        // Snapshotting here rather than at construction keeps it marginally fresher.
        let current_state = state::build_current_state(&self.base_dir);

        Ok(vec![Message::user(format!(
            "{current_state}\n\n{text_instruction}"
        ))])
    }

    async fn execute_tool(&self, name: &str, input: &Value) -> Value {
        match name {
            "complete_system" => self.handle_complete_system(input).await,
            "run_command" => self.handle_run_command(input).await,
            "think" => json!({ "status": "ok" }),
            _ => json!({ "error": format!("Unknown tool: {}", name) }),
        }
    }
}

// ── Tool handlers ────────────────────────────────────────────────────────

impl SystemNodeStrategy {
    async fn handle_complete_system(&self, input: &Value) -> Value {
        let summary = input["summary"].as_str().unwrap_or("").to_string();
        let verify = &input["verify"];

        let user_text_words = validate::extract_user_text_words(&self.instruction);
        let mut success = match validate::validate_verify(&self.base_dir, verify, user_text_words) {
            Ok(v) => v,
            Err(error_response) => return error_response,
        };

        success["description_changed"] = json!(self.has_description_changed().await);

        if let Ok(mut guard) = self.summary.lock() {
            *guard = Some(summary);
        }

        success
    }

    async fn handle_run_command(&self, input: &Value) -> Value {
        let mut result = crate::server::tools::execution::dispatch_tool_cascade(
            "run_command",
            input,
            self.container_handle.as_ref(),
            None,
            None,
            Some(&self.state),
            None,
        )
        .await;

        let errors = validate_written_files(&self.base_dir);
        if !errors.is_empty() {
            // `Value`'s IndexMut panics on anything but an object, and the
            // cascade can now return a bare string. Attach the errors in
            // whichever form the result actually has.
            match &mut result {
                Value::Object(map) => {
                    map.insert("write_validation_errors".to_string(), json!(errors));
                }
                Value::String(text) => {
                    text.push_str("\n\nwrite_validation_errors:\n");
                    for e in &errors {
                        text.push_str("  ");
                        text.push_str(e);
                        text.push('\n');
                    }
                }
                _ => {}
            }
        }

        result
    }

    /// Compare config.json description against the stored designer_handoff.
    async fn has_description_changed(&self) -> bool {
        let description = match file_reader::read_config(&self.base_dir) {
            Ok((_name, desc)) => desc,
            Err(_) => return false,
        };

        match self.state.repos().workflows.get_step(self.step_id).await {
            Ok(Some(step)) => step.designer_handoff != description,
            _ => true,
        }
    }
}

// ── Write-time validation ────────────────────────────────────────────────

/// Validate JSON files in the system node agent's repository after a write.
///
/// Returns a vec of error strings (empty if all valid). Only validates
/// files that exist — missing files are fine (written in a later command).
fn validate_written_files(base_dir: &Path) -> Vec<String> {
    let mut errors = Vec::new();

    validate_file(
        &mut errors,
        base_dir,
        "config.json",
        validate::validate_config,
    );
    validate_file(
        &mut errors,
        base_dir,
        "topology.json",
        validate::validate_topology,
    );

    let agents_dir = base_dir.join("agents");
    for path in json_files_in(&agents_dir) {
        let filename = path.file_name().unwrap_or_default().to_string_lossy();
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Err(e) = validate::validate_agent(&content) {
                errors.push(format!("agents/{filename}: {e}"));
            }
        }
    }

    errors
}

/// Validate a single file if it exists, pushing any error to the vec.
fn validate_file(
    errors: &mut Vec<String>,
    base_dir: &Path,
    filename: &str,
    validator: fn(&str) -> Result<(), String>,
) {
    let path = base_dir.join(filename);
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return, // file doesn't exist yet — not an error
    };
    if let Err(e) = validator(&content) {
        errors.push(format!("{filename}: {e}"));
    }
}

/// List .json files in a directory (empty vec if dir doesn't exist).
fn json_files_in(dir: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "json"))
        .collect()
}
