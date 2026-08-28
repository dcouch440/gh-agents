//! WorkflowAgentStrategy — ExecutionStrategy for the workflow designer agent.
//!
//! Conversational agent that helps users design workflow topology (nodes + edges)
//! by reading and writing files in a board repo (`topology.json` + `nodes/*.md`).
//! Streams responses to the frontend via SSE. Syncs file changes to DB on completion.
//!
//! Pattern: SystemNodeStrategy's file-based approach + ChatStrategy's streaming +
//! ManagerDispatchStrategy's session/rebuild patterns.

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use serde_json::{json, Value};
use tracing::warn;
use uuid::Uuid;

use crate::llm::{Message, TokenUsage, Tool};
use crate::server::hub::error::HubError;
use crate::server::hub::strategy::ExecutionStrategy;
use crate::server::services::workflow_agent::{state, validate};
use crate::server::state::AppState;
use crate::tools::registry::get_tool_definition;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

/// Strategy for the workflow designer agent.
///
/// Works in a file-based repo at `base_dir/` containing `topology.json` and
/// `nodes/*.md`. The agent reads and writes these files via `run_command` (host
/// shell execution). On completion, changes sync to DB via `sync_to_db()`.
pub struct WorkflowAgentStrategy {
    system_prompt: String,
    state: AppState,
    user_id: Uuid,
    workflow_id: Uuid,
    session_id: Uuid,
    base_dir: PathBuf,
    message_id: Uuid,
}

impl WorkflowAgentStrategy {
    fn config(&self) -> &crate::config::protocols::AgentConfig {
        crate::config::protocols::WORKFLOW_AGENT.agent("agent")
    }

    /// Build a new workflow agent strategy.
    ///
    /// `system_prompt` is static and cacheable — `<current_state>` is attached
    /// to the user message in `build_messages`, not here.
    pub fn new(
        system_prompt: String,
        state: AppState,
        user_id: Uuid,
        workflow_id: Uuid,
        session_id: Uuid,
        base_dir: PathBuf,
        message_id: Uuid,
    ) -> Self {
        Self {
            system_prompt,
            state,
            user_id,
            workflow_id,
            session_id,
            base_dir,
            message_id,
        }
    }

    /// Execute a shell command in the base_dir (host execution, no container).
    ///
    /// Snapshots the repo before and after the command. If files changed,
    /// syncs to DB immediately so the frontend sees changes in real-time.
    async fn host_run_command(&self, input: &Value) -> Value {
        use crate::server::services::workflow_agent::file_reader::snapshot_board_files;

        let command = match input["command"].as_str() {
            Some(c) => c,
            None => return json!({ "error": "Missing required parameter: command" }),
        };

        // Snapshot before command
        let before = snapshot_board_files(&self.base_dir);

        let output = match tokio::process::Command::new("sh")
            .arg("-c")
            .arg(command)
            .current_dir(&self.base_dir)
            .output()
            .await
        {
            Ok(o) => o,
            Err(e) => return json!({ "error": format!("Failed to execute command: {e}") }),
        };

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let exit_code = output.status.code().unwrap_or(-1);

        let mut result = json!({
            "exit_code": exit_code,
            "stdout": stdout.as_ref(),
            "stderr": stderr.as_ref(),
            "success": output.status.success(),
        });

        // Validate written files (same pattern as SystemNodeStrategy)
        let errors = validate_written_files(&self.base_dir);
        if !errors.is_empty() {
            result["write_validation_errors"] = json!(errors);
        }

        // Snapshot after command — if files changed, sync immediately
        let after = snapshot_board_files(&self.base_dir);
        if before != after {
            let wf_repo = &*self.state.repos().workflows;
            if let Err(e) = crate::server::services::workflow_agent::sync::sync_to_db(
                &self.base_dir,
                self.workflow_id,
                self.user_id,
                wf_repo,
                &self.state,
            )
            .await
            {
                warn!(
                    workflow_id = %self.workflow_id,
                    error = %e,
                    "Per-command sync failed — will retry on turn completion"
                );
            }
        }

        result
    }
}

#[async_trait]
impl ExecutionStrategy for WorkflowAgentStrategy {
    fn system_prompt(&self) -> &str {
        &self.system_prompt
    }

    fn tools(&self) -> Vec<Tool> {
        let mut tools = Vec::with_capacity(3);
        if let Some(t) = get_tool_definition("run_command") {
            tools.push(t);
        }
        if let Some(t) = get_tool_definition("think") {
            tools.push(t);
        }
        if let Some(t) = get_tool_definition("render_panel") {
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
        true
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

    fn user_id(&self) -> Option<Uuid> {
        Some(self.user_id)
    }

    async fn build_messages(&self, input: &str) -> Result<Vec<Message>, HubError> {
        let history = self
            .state
            .repos()
            .sessions
            .get_session_history(self.session_id, 30)
            .await
            .unwrap_or_default();

        // Convert history to messages (skip tool calls — the LLM doesn't need them)
        let mut messages = Vec::new();
        for row in &history {
            if row.source_type.as_deref() == Some("tool") {
                continue;
            }
            match row.role.as_str() {
                "user" => messages.push(Message::user(&row.content)),
                "assistant" => messages.push(Message::assistant(&row.content)),
                _ => {}
            }
        }

        // Ensure the current user message is included (handles first turn
        // and race conditions where the insert hasn't flushed to history yet)
        if !messages
            .iter()
            .any(|m| m.role == crate::llm::Role::User && m.text() == input)
        {
            messages.push(Message::user(input));
        }

        // Attach <current_state> to this turn's user message. The system prompt
        // stays static (and cacheable); the block is never persisted, so the
        // history re-read above cannot accumulate stale state.
        //
        // Sent on every turn, not just when the board changed: the block only
        // ever rides the *last* user message, so an earlier turn's copy is gone
        // from the transcript by the time the next turn is built. Skipping it
        // would leave the agent with no board state at all.
        if let Some(last_user) = messages
            .iter_mut()
            .rev()
            .find(|m| m.role == crate::llm::Role::User)
        {
            match state::build_current_state(self.workflow_id, &self.state).await {
                Ok(current_state) => {
                    *last_user = Message::user(format!("{current_state}\n\n{}", last_user.text()));
                }
                Err(e) => {
                    warn!(
                        workflow_id = %self.workflow_id,
                        error = %e,
                        "Failed to build <current_state> — proceeding without it"
                    );
                }
            }
        }

        Ok(messages)
    }

    async fn execute_tool(&self, name: &str, input: &Value) -> Value {
        let result = match name {
            "run_command" => self.host_run_command(input).await,
            "think" => json!({ "status": "ok" }),
            "render_panel" => {
                let content = input["content"].as_str().unwrap_or("");
                let submit_label = input["submit_label"].as_str().unwrap_or("Submit");
                self.state.send_stream_chunk(
                    self.message_id,
                    crate::server::state::StreamChunk::PanelRender {
                        content: content.to_string(),
                        submit_label: submit_label.to_string(),
                    },
                );
                json!({ "rendered": true, "content": content, "submit_label": submit_label })
            }
            _ => json!({ "error": format!("Unknown tool: {name}") }),
        };

        // Persist tool call + result to session (source_type = "tool")
        let tool_input = match name {
            "run_command" => input
                .get("command")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            "think" => input
                .get("thought")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            _ => serde_json::to_string(input).unwrap_or_default(),
        };
        let result_str = serde_json::to_string(&result).unwrap_or_default();
        let truncated = if result_str.len() > 4096 {
            format!("{}…", &result_str[..4096])
        } else {
            result_str
        };
        let combined = format!("{name}: {tool_input}\n---\n{truncated}");

        let _ = self
            .state
            .repos()
            .sessions
            .insert_agent_message(
                crate::types::UserId(self.user_id),
                self.session_id,
                Uuid::new_v4(),
                "assistant".to_string(),
                combined,
                "tool".to_string(),
            )
            .await;

        result
    }

    async fn on_complete(&self, response: &str, usage: &TokenUsage) -> Result<(), HubError> {
        // Log token usage
        super::log_token_usage(&self.state, self.user_id, None, self.model_id(), usage).await;

        // Sync file changes to DB
        let wf_repo = &*self.state.repos().workflows;
        if let Err(e) = crate::server::services::workflow_agent::sync::sync_to_db(
            &self.base_dir,
            self.workflow_id,
            self.user_id,
            wf_repo,
            &self.state,
        )
        .await
        {
            warn!(
                workflow_id = %self.workflow_id,
                error = %e,
                "Failed to sync workflow agent changes to DB"
            );
        }

        // Persist assistant response as session message
        if let Err(e) = self
            .state
            .repos()
            .sessions
            .insert_session_message(
                crate::types::UserId(self.user_id),
                self.session_id,
                Uuid::new_v4(),
                "assistant".to_string(),
                response.to_string(),
            )
            .await
        {
            warn!(
                session_id = %self.session_id,
                error = %e,
                "Failed to persist assistant message"
            );
        }

        Ok(())
    }
}

// ── Write validation ───────────────────────────────────────────────────────

/// Validate files in the board repo after a run_command.
///
/// Returns error messages for any invalid files. Silently skips missing files
/// (the agent may not have written them this command).
fn validate_written_files(base_dir: &Path) -> Vec<String> {
    let mut errors = Vec::new();

    // Validate topology.json if it exists
    let topology_path = base_dir.join("topology.json");
    if topology_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&topology_path) {
            if let Err(msg) = validate::validate_topology(&content) {
                errors.push(format!("topology.json: {msg}"));
            }
        }
    }

    // Validate node files if nodes/ directory exists
    let nodes_dir = base_dir.join("nodes");
    if nodes_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&nodes_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("md") {
                    if let Some(slug) = path.file_stem().and_then(|s| s.to_str()) {
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            if let Err(msg) = validate::validate_node(&content, slug) {
                                errors.push(format!("nodes/{slug}.md: {msg}"));
                            }
                        }
                    }
                }
            }
        }
    }

    // Cross-reference validation (topology ↔ node files, cycles, dangling refs).
    // Only runs once topology.json exists — early commands that haven't created it skip this.
    if base_dir.join("topology.json").exists() {
        for err in validate::cross_reference(base_dir) {
            errors.push(format!("{}: {}", err.file, err.error));
        }
    }

    errors
}
