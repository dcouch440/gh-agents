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
    pub plan: String,
    pub builder_action: String,
    pub agent_execution_id: Option<Uuid>,
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
        let roster_status = build_roster_status_sync(&config.roster);
        // Build initial system prompt with roster status
        let mut vars = HashMap::new();
        vars.insert(
            vars::react_designer::NODE_NAME.to_string(),
            format!("step:{}", config.step_id),
        );
        vars.insert(
            vars::react_designer::ROSTER_STATUS.to_string(),
            roster_status,
        );
        let system_prompt = resolve_template(roles::REACT_DESIGNER.system, &vars);

        // Build instruction from plan + roster + builder_action
        let roster_text = format_roster_for_prompt(&config.roster);
        let mut inst_vars = HashMap::new();
        inst_vars.insert(
            vars::react_designer::PRIOR_DESIGN.to_string(),
            String::new(), // Filled in build_messages from session history
        );
        inst_vars.insert(vars::react_designer::PLAN.to_string(), config.plan);
        inst_vars.insert(vars::react_designer::ROSTER.to_string(), roster_text);
        inst_vars.insert(
            vars::react_designer::BUILDER_ACTION.to_string(),
            config.builder_action,
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

    /// Auto-scope a designer path by prefixing with `design/{step_id}/`.
    fn scope_path(&self, path: &str) -> String {
        let stripped = path.strip_prefix("design/").unwrap_or(path);
        format!("design/{}/{}", self.step_id, stripped)
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
        let roster_status = self.build_roster_status().await;
        let mut vars = HashMap::new();
        vars.insert(
            vars::react_designer::NODE_NAME.to_string(),
            format!("step:{}", self.step_id),
        );
        vars.insert(
            vars::react_designer::ROSTER_STATUS.to_string(),
            roster_status,
        );
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
                    },
                )
                .await
                {
                    Ok(row) => serde_json::json!({
                        "status": "written",
                        "path": row.path,
                        "version": row.version,
                    }),
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
                if let Ok(mut guard) = self.design_summary.lock() {
                    *guard = Some(summary.clone());
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

impl ReactDesignerStrategy {
    /// Build roster status by checking which agents have configs in the store.
    async fn build_roster_status(&self) -> String {
        if self.state.s3().is_none() {
            return build_roster_status_sync(&self.roster);
        }
        let repo = self.state.repos().system_files.as_ref();
        let prefix = format!("design/{}/agents/", self.step_id);

        let files = system_store::list_files(repo, self.workflow_id, &prefix)
            .await
            .unwrap_or_default();

        let file_map: HashMap<String, i32> = files
            .iter()
            .filter_map(|f| {
                let filename = f.path.rsplit('/').next()?;
                let slug = filename.strip_suffix(".json")?;
                Some((slug.to_string(), f.version))
            })
            .collect();

        let mut status = String::from("<roster_status>\n");
        let mut designed = 0;
        let total = self.roster.len();

        for agent in &self.roster {
            let slug = crate::server::hub::dag::agent_designer::agent_name_to_slug(&agent.name);
            if let Some(version) = file_map.get(&slug) {
                status.push_str(&format!("  ✓ {} — designed (v{})\n", agent.name, version));
                designed += 1;
            } else {
                status.push_str(&format!("  · {} — pending\n", agent.name));
            }
        }

        status.push_str(&format!("\n  Designed: {}/{}\n", designed, total));
        status.push_str("</roster_status>");
        status
    }
}

/// Build a synchronous roster status (all pending — used before first round).
fn build_roster_status_sync(roster: &[TaskAgentRosterRow]) -> String {
    let mut status = String::from("<roster_status>\n");
    for agent in roster {
        status.push_str(&format!("  · {} — pending\n", agent.name));
    }
    status.push_str(&format!("\n  Designed: 0/{}\n", roster.len()));
    status.push_str("</roster_status>");
    status
}

/// Format roster entries for the designer's instruction prompt.
fn format_roster_for_prompt(roster: &[TaskAgentRosterRow]) -> String {
    let mut out = String::new();
    for agent in roster {
        out.push_str(&agent.name);
        out.push('\n');
        out.push_str(&format!("  role: \"{}\"\n", agent.role_description));
        if !agent.capabilities.is_empty() {
            out.push_str(&format!(
                "  capabilities: [{}]\n",
                agent.capabilities.join(", ")
            ));
        }
        out.push('\n');
    }
    out
}
