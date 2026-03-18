//! ReactDesignerStrategy — multi-turn ReAct designer that writes agent configs to the store.
//!
//! Replaces the one-shot `AgentDesignerStrategy` with an iterative approach:
//! writes one agent config at a time, reads back prior configs for consistency,
//! and self-corrects across turns.

mod tests;

use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

use crate::config::protocols::{roles, vars, DESIGNER};
use crate::db::TaskAgentRosterRow;
use crate::llm::{Message, TokenUsage, Tool};
use crate::server::hub::error::HubError;
use crate::server::hub::protocols::template_resolve::resolve_template;
use crate::server::hub::strategy::ExecutionStrategy;
use crate::server::services::dispatch::PreviousStepHandoff;
use crate::server::services::system_store::{s3::S3Backend, store as system_store};
use crate::server::state::AppState;

use super::build_pruned_instruction;

/// Configuration for the ReAct designer strategy.
pub struct ReactDesignerConfig {
    pub state: AppState,
    pub step_id: Uuid,
    pub workflow_id: Uuid,
    pub roster: Vec<TaskAgentRosterRow>,
    pub session_id: Option<Uuid>,
    pub agent_execution_id: Option<Uuid>,
    /// Pre-rendered `<board_state>` XML enriched with design status.
    pub board_state_xml: String,
    /// Compact upstream/downstream topology description.
    pub upstream_topology: String,
    /// The original dispatch instruction (changeset message) the builder received.
    pub dispatch_instruction: String,
    /// Agent names changed by the builder (for `rebuild_system_prompt` enrichment).
    pub changed_agents: Vec<String>,
    /// Handoff from the previous step's designer (None for first step in workflow).
    pub previous_step_handoff: Option<PreviousStepHandoff>,
    /// Raw box text of the next step in the workflow (None for last step).
    pub next_step_text: Option<String>,
}

/// Multi-turn ReAct designer strategy.
///
/// Writes agent configs one at a time to the system store, reads back prior
/// configs for consistency, and signals completion via `complete_design`.
pub struct ReactDesignerStrategy {
    system_prompt: String,
    instruction: String,
    state: AppState,
    step_id: Uuid,
    workflow_id: Uuid,
    roster: Vec<TaskAgentRosterRow>,
    session_id: Option<Uuid>,
    agent_execution_id: Option<Uuid>,
    completed: Mutex<bool>,
    design_summary: Mutex<Option<String>>,
    designed_count: Mutex<usize>,
    /// Agent names changed by the builder (for rebuild_system_prompt enrichment).
    changed_agents: Vec<String>,
    /// Step-level handoff captured from `complete_design` tool call.
    step_handoff: Mutex<Option<String>>,
    /// Cached config values to avoid returning references to temporaries.
    cached_model_id: String,
    cached_max_rounds: u32,
    cached_context_budget: usize,
    cached_temperature: f32,
}

impl ReactDesignerStrategy {
    /// Build a new ReactDesignerStrategy.
    pub fn new(config: ReactDesignerConfig) -> Self {
        let agent_cfg = DESIGNER.agent("react_designer");

        // Build system prompt with enriched board_state
        let mut sys_vars = HashMap::new();
        sys_vars.insert(
            vars::react_designer::NODE_NAME.to_string(),
            format!("step:{}", config.step_id),
        );
        sys_vars.insert(
            vars::system::BOARD_STATE.to_string(),
            config.board_state_xml,
        );
        let system_prompt = resolve_template(roles::REACT_DESIGNER.system, &sys_vars);

        // Build instruction (user message)
        let mut inst_vars = HashMap::new();
        inst_vars.insert(
            vars::react_designer::PRIOR_DESIGN.to_string(),
            String::new(), // Filled in build_messages from session history
        );
        inst_vars.insert(
            vars::react_designer::UPSTREAM_TOPOLOGY.to_string(),
            config.upstream_topology,
        );
        inst_vars.insert(
            vars::react_designer::DISPATCH_INSTRUCTION.to_string(),
            config.dispatch_instruction,
        );
        inst_vars.insert(
            vars::react_designer::PREVIOUS_STEP.to_string(),
            match &config.previous_step_handoff {
                Some(h) => format!(
                    "<previous_step name=\"{}\">\n<handoff>\n{}\n</handoff>\n</previous_step>",
                    h.step_name, h.handoff_description
                ),
                None => String::new(),
            },
        );
        inst_vars.insert(
            vars::react_designer::NEXT_STEP.to_string(),
            match &config.next_step_text {
                Some(text) => format!("<next_step>\n{}\n</next_step>", text),
                None => String::new(),
            },
        );
        let instruction = resolve_template(roles::REACT_DESIGNER.prompt, &inst_vars);

        Self {
            system_prompt,
            instruction,
            state: config.state,
            step_id: config.step_id,
            workflow_id: config.workflow_id,
            roster: config.roster,
            session_id: config.session_id,
            agent_execution_id: config.agent_execution_id,
            completed: Mutex::new(false),
            design_summary: Mutex::new(None),
            designed_count: Mutex::new(0),
            changed_agents: config.changed_agents,
            step_handoff: Mutex::new(None),
            cached_model_id: agent_cfg.model_id.clone(),
            cached_max_rounds: agent_cfg.max_rounds,
            cached_context_budget: agent_cfg.context_budget,
            cached_temperature: agent_cfg.temperature,
        }
    }

    /// Take the design summary captured by `complete_design`.
    pub fn take_design_summary(&self) -> Option<String> {
        self.design_summary.lock().ok().and_then(|mut s| s.take())
    }

    /// Take the step-level handoff captured by `complete_design`.
    pub fn take_step_handoff(&self) -> Option<String> {
        self.step_handoff.lock().ok().and_then(|mut s| s.take())
    }

    /// Get the resolved instruction (user message) for debug output.
    pub fn instruction(&self) -> &str {
        &self.instruction
    }

    /// Auto-scope a designer path by prefixing with `design/{step_id}/`.
    /// Normalizes the filename to lowercase so version tracking works across
    /// dispatches regardless of how the LLM capitalizes the slug.
    fn scope_path(&self, path: &str) -> String {
        let stripped = path.strip_prefix("design/").unwrap_or(path);
        let normalized = stripped.to_lowercase();
        format!("design/{}/{}", self.step_id, normalized)
    }

    /// Get S3 backend reference.
    fn s3(&self) -> Result<&S3Backend, Value> {
        self.state
            .s3()
            .map(|arc| arc.as_ref())
            .ok_or_else(|| serde_json::json!({"error": "S3 not available"}))
    }
}

#[async_trait]
impl ExecutionStrategy for ReactDesignerStrategy {
    fn system_prompt(&self) -> &str {
        &self.system_prompt
    }

    fn tools(&self) -> Vec<Tool> {
        vec![
            Tool {
                name: "write_file".to_string(),
                description: "Write an agent config file to the store. Path should be design/agents/{slug}.json".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "File path (e.g. design/agents/scanner.json)"
                        },
                        "content": {
                            "type": "string",
                            "description": "File content — valid JSON with tools, system_prompt, assignment, expected_output"
                        }
                    },
                    "required": ["path", "content"]
                }),
            },
            Tool {
                name: "read_file".to_string(),
                description: "Read a config file from the store.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "File path to read"
                        }
                    },
                    "required": ["path"]
                }),
            },
            Tool {
                name: "complete_design".to_string(),
                description: "Signal that all agent configs are written. No tools after this.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "summary": {
                            "type": "string",
                            "description": "Summary: topology shape, format chain, key decisions (1-5 sentences)"
                        },
                        "step_handoff": {
                            "type": "string",
                            "description": "What this step produces for the next step's designer. 1-3 sentences: key outputs, their location, and how the next step should use them."
                        }
                    },
                    "required": ["summary"]
                }),
            },
        ]
    }

    fn model_id(&self) -> &str {
        &self.cached_model_id
    }

    fn max_rounds(&self) -> u32 {
        self.cached_max_rounds
    }

    fn context_budget(&self) -> usize {
        self.cached_context_budget
    }

    fn streaming(&self) -> bool {
        false
    }

    fn temperature(&self) -> f32 {
        self.cached_temperature
    }

    fn state(&self) -> Option<&AppState> {
        Some(&self.state)
    }

    fn agent_execution_id(&self) -> Option<Uuid> {
        self.agent_execution_id
    }

    fn should_stop(&self) -> bool {
        self.completed.lock().map(|c| *c).unwrap_or(false)
    }

    async fn rebuild_system_prompt(&self) -> Result<Option<String>, HubError> {
        // Rebuild enriched board_state from current store state
        let repos = self.state.repos();
        let board_state_xml = match crate::server::hub::board_state::build_snapshot(
            repos.workflows.as_ref(),
            None,
            crate::server::hub::board_state::BoardStateVariant::Dispatch,
            self.workflow_id,
            self.step_id,
        )
        .await
        {
            Ok(mut snapshot) => {
                crate::server::hub::board_state::enrich_design_status(
                    &mut snapshot,
                    repos.system_files.as_ref(),
                    self.step_id,
                    self.workflow_id,
                    &self.changed_agents,
                )
                .await;
                crate::server::hub::board_state::render(
                    &snapshot,
                    crate::server::hub::board_state::BoardStateVariant::Dispatch,
                )
            }
            Err(_) => String::new(),
        };

        let mut vars = HashMap::new();
        vars.insert(
            vars::react_designer::NODE_NAME.to_string(),
            format!("step:{}", self.step_id),
        );
        vars.insert(vars::system::BOARD_STATE.to_string(), board_state_xml);
        Ok(Some(resolve_template(roles::REACT_DESIGNER.system, &vars)))
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
                // Reuse build_pruned_instruction but rename <prior_work> to <prior_design>
                let pruned = build_pruned_instruction(&history, &self.instruction, 3);
                pruned
                    .replace("<prior_work>", "<prior_design>")
                    .replace("</prior_work>", "</prior_design>")
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
            "write_file" => {
                let path = input["path"].as_str().unwrap_or("");
                let content = input["content"].as_str().unwrap_or("");
                let scoped = self.scope_path(path);

                let s3 = match self.s3() {
                    Ok(s3) => s3,
                    Err(e) => return e,
                };
                let repo = self.state.repos().system_files.as_ref();

                match system_store::write_file(
                    s3,
                    repo,
                    system_store::WriteFileInput {
                        workflow_id: self.workflow_id,
                        path: scoped,
                        content: content.as_bytes().to_vec(),
                        media_type: "application/json".to_string(),
                        description: "Agent config for designer phase".to_string(),
                        tags: vec!["designer".to_string()],
                        produced_by: None,
                        produced_by_agent: Some("designer".to_string()),
                        workflow_run_id: None, // design configs persist across runs
                    },
                )
                .await
                {
                    Ok(row) => {
                        // Broadcast per-agent design progress
                        let count = {
                            let mut guard = self
                                .designed_count
                                .lock()
                                .unwrap_or_else(|e| e.into_inner());
                            *guard += 1;
                            *guard
                        };
                        let agent_name = row
                            .path
                            .rsplit('/')
                            .next()
                            .and_then(|f| f.strip_suffix(".json"))
                            .unwrap_or("unknown")
                            .to_string();
                        self.state.broadcast_workflow(
                            crate::server::ws::events::WorkflowEvent {
                                workflow_id: self.workflow_id,
                                run_id: None,
                                user_id: None,
                                kind: crate::server::ws::events::WorkflowEventKind::DesignerAgentDesigned {
                                    step_id: self.step_id,
                                    agent_name,
                                    designed_count: count,
                                    total_count: self.roster.len(),
                                },
                            },
                        );

                        serde_json::json!({
                            "status": "written",
                            "path": row.path,
                            "version": row.version,
                        })
                    }
                    Err(e) => serde_json::json!({"error": e.to_string()}),
                }
            }
            "read_file" => {
                let path = input["path"].as_str().unwrap_or("");
                let scoped = self.scope_path(path);

                let s3 = match self.s3() {
                    Ok(s3) => s3,
                    Err(e) => return e,
                };
                let repo = self.state.repos().system_files.as_ref();

                match system_store::read_file(s3, repo, self.workflow_id, &scoped).await {
                    Ok((bytes, _meta)) => {
                        let content = String::from_utf8_lossy(&bytes).to_string();
                        serde_json::json!({
                            "content": content,
                        })
                    }
                    Err(e) => serde_json::json!({"error": e.to_string()}),
                }
            }
            "complete_design" => {
                let summary = input["summary"].as_str().unwrap_or("").to_string();
                let step_handoff = input["step_handoff"].as_str().map(String::from);

                if let Ok(mut guard) = self.design_summary.lock() {
                    *guard = Some(summary.clone());
                }
                if let Ok(mut guard) = self.step_handoff.lock() {
                    *guard = step_handoff.clone();
                }

                // Persist step handoff to DB for the next step's builder/designer
                if let Some(ref handoff) = step_handoff {
                    let repos = self.state.repos();
                    if let Err(e) = repos
                        .workflows
                        .update_designer_handoff(self.step_id, handoff)
                        .await
                    {
                        tracing::warn!(
                            step_id = %self.step_id,
                            error = %e,
                            "Failed to persist designer handoff"
                        );
                    }
                }

                if let Ok(mut guard) = self.completed.lock() {
                    *guard = true;
                }
                serde_json::json!({
                    "status": "design_complete",
                    "summary": summary,
                })
            }
            _ => serde_json::json!({"error": format!("unknown tool: {name}")}),
        }
    }

    async fn on_complete(&self, response: &str, usage: &TokenUsage) -> Result<(), HubError> {
        // Persist completion to agent execution record
        super::complete_agent_execution(
            Some(&self.state),
            self.user_id(),
            self.agent_execution_id,
            self.model_id(),
            response,
            usage,
            false,
        )
        .await;
        Ok(())
    }
}
