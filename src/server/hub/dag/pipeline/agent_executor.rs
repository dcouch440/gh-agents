//! Agent execution for the workforce step.
//!
//! Handles both single-agent sequential execution and parallel level-based
//! dispatch via `tokio::task::JoinSet`.

use std::sync::Arc;

use anyhow::anyhow;
use tracing::{error, info, warn};

use crate::config::protocols::{roles, WORKFORCE};
use crate::db::traits::CreateAgentExecutionInput;
use crate::execution::diagnostics::workspace::digest::format_size;
use crate::execution::diagnostics::DiagnosticsEngine;
use crate::server::hub::error::HubError;
use crate::server::hub::protocols::execution_recorder::{
    PhaseCompletion, ProtocolExecutionRecorder,
};
use crate::server::hub::recorder::ExecutionRecorder;
use crate::server::hub::strategies::compute_cost;
use crate::server::hub::strategies::workforce_agent::{
    WorkforceAgentConfig, WorkforceAgentStrategy,
};
use crate::server::hub::streaming::DagStreamSink;
use crate::server::ws::events::WorkflowEventKind;
use crate::types::ExecutionType;
use crate::types::UserId;

use super::super::container::destroy_optional_container;
use super::super::{broadcast_workflow_event, DagContext};
use super::output::{
    build_filtered_outputs_block, compute_execution_levels, filter_outputs_for_agent,
};
use super::types::{
    AgentExecutionResult, AgentFailureAction, DesignedAgentPrompt, LevelExecutionResult,
    WorkforceStepEnv,
};

/// Maximum files named in an agent's passdown manifest.
const MAX_PASSDOWN_FILES: usize = 10;

/// Append a `files:` line naming what the agent actually wrote to disk.
///
/// Objective counterpart to the agent's prose receipt: the agent says which
/// file is the deliverable and what is in it, this says what landed. Returns
/// `content` unchanged when there is no container or nothing survived the
/// noise filter.
async fn append_files_line(
    content: String,
    diagnostics: Option<&tokio::sync::Mutex<DiagnosticsEngine>>,
) -> String {
    let Some(diag) = diagnostics else {
        return content;
    };

    let (files, dropped) = diag.lock().await.produced_files(MAX_PASSDOWN_FILES);
    if files.is_empty() {
        return content;
    }

    let mut line = files
        .iter()
        .map(|f| {
            format!(
                "{} ({}, {})",
                f.path.display(),
                f.change_type,
                format_size(f.size)
            )
        })
        .collect::<Vec<_>>()
        .join(", ");

    if dropped > 0 {
        line.push_str(&format!(" (+{dropped} more)"));
    }

    format!("{content}\nfiles: {line}")
}

/// Decide how to handle an agent failure based on the configured failure mode.
fn handle_agent_failure(
    error: HubError,
    agent_name: &str,
    failure_mode: &str,
) -> AgentFailureAction {
    if failure_mode == "fail_fast" {
        return AgentFailureAction::Abort(error);
    }
    let err_msg = format!("{}", error);
    warn!(
        agent = %agent_name,
        error = %err_msg,
        "Workforce agent failed, skipping ({})", failure_mode
    );
    AgentFailureAction::Skip {
        name: agent_name.to_string(),
        error_output: format!("[AGENT FAILED: {}]", err_msg),
    }
}

/// Execute all agent levels (sequential within levels, parallel across agents
/// in the same level). Handles cancellation and failure modes.
pub(super) async fn execute_agent_levels(
    env: &WorkforceStepEnv,
    dag: &DagContext<'_>,
    designed_prompts: &[DesignedAgentPrompt],
    failure_mode: &str,
    managed_container: &Option<super::super::container::ManagedContainer>,
) -> Result<LevelExecutionResult, HubError> {
    let levels = compute_execution_levels(designed_prompts);
    let mut agent_outputs: Vec<(String, String)> = Vec::with_capacity(designed_prompts.len());
    let mut input_tokens: i64 = 0;
    let mut output_tokens: i64 = 0;
    let mut cost_usd: f32 = 0.0;

    for level_indices in &levels {
        if dag.cancel.is_some_and(|t| t.is_cancelled()) {
            destroy_optional_container(managed_container, dag.ctx.wg_client.as_deref()).await;
            return Err(HubError::Cancelled);
        }

        if level_indices.len() == 1 {
            // Single agent — run directly (no spawn overhead)
            let idx = level_indices[0];
            let designed = &designed_prompts[idx];

            match execute_single_agent(
                env,
                &dag.engine.clone_with_provider(),
                designed,
                &agent_outputs,
                idx,
            )
            .await
            {
                Ok(result) => {
                    input_tokens += result.input_tokens;
                    output_tokens += result.output_tokens;
                    cost_usd += result.cost;
                    agent_outputs.push((result.name, result.content));
                }
                Err(e) => match handle_agent_failure(e, &designed.agent_name, failure_mode) {
                    AgentFailureAction::Abort(err) => {
                        destroy_optional_container(managed_container, dag.ctx.wg_client.as_deref())
                            .await;
                        return Err(err);
                    }
                    AgentFailureAction::Skip { name, error_output } => {
                        agent_outputs.push((name, error_output));
                    }
                },
            }
        } else {
            // Multiple agents — run in parallel via JoinSet
            let mut join_set = tokio::task::JoinSet::new();
            let outputs_snapshot = agent_outputs.clone();

            for &idx in level_indices {
                let designed = designed_prompts[idx].clone();
                let env_clone = env.clone();
                let provider = dag.engine.provider();
                let debug_stream = dag.state.env().debug_stream;
                let outputs = outputs_snapshot.clone();

                join_set.spawn(async move {
                    let engine =
                        crate::server::hub::engine::ExecutionEngine::new(provider, debug_stream);
                    let result =
                        execute_single_agent(&env_clone, &engine, &designed, &outputs, idx).await;
                    (idx, result)
                });
            }

            let mut level_failed = false;
            while let Some(join_result) = join_set.join_next().await {
                match join_result {
                    Ok((_idx, Ok(agent_result))) => {
                        input_tokens += agent_result.input_tokens;
                        output_tokens += agent_result.output_tokens;
                        cost_usd += agent_result.cost;
                        agent_outputs.push((agent_result.name, agent_result.content));
                    }
                    Ok((idx, Err(e))) => {
                        match handle_agent_failure(
                            e,
                            &designed_prompts[idx].agent_name,
                            failure_mode,
                        ) {
                            AgentFailureAction::Abort(err) => {
                                join_set.abort_all();
                                destroy_optional_container(
                                    managed_container,
                                    dag.ctx.wg_client.as_deref(),
                                )
                                .await;
                                return Err(err);
                            }
                            AgentFailureAction::Skip { name, error_output } => {
                                agent_outputs.push((name, error_output));
                                level_failed = true;
                            }
                        }
                    }
                    Err(join_err) => {
                        error!("Workforce agent task panicked: {}", join_err);
                        level_failed = true;
                    }
                }
            }

            if level_failed && failure_mode == "fail_fast" {
                destroy_optional_container(managed_container, dag.ctx.wg_client.as_deref()).await;
                return Err(HubError::Internal(anyhow!("Agent task panicked")));
            }
        }
    }

    Ok(LevelExecutionResult {
        agent_outputs,
        input_tokens,
        output_tokens,
        cost_usd,
    })
}

/// Execute a single workforce agent. Used by both sequential (single agent at
/// a level) and parallel (spawned task) paths.
async fn execute_single_agent(
    env: &WorkforceStepEnv,
    engine: &crate::server::hub::engine::ExecutionEngine,
    designed: &DesignedAgentPrompt,
    prior_outputs: &[(String, String)],
    agent_index: usize,
) -> Result<AgentExecutionResult, HubError> {
    let agent_cfg = WORKFORCE.agent("agent");

    // Broadcast started
    broadcast_workflow_event(
        &env.state,
        &env.ctx,
        env.workflow_id,
        WorkflowEventKind::WorkforceAgentProgress {
            step_id: env.step_id,
            agent_name: designed.agent_name.clone(),
            roster_agent_id: designed.agent_roster_entry_id,
            agent_index,
            total_agents: env.total_agents,
            status: "started".to_string(),
        },
    );

    // Create protocol execution recorder (per-agent, owns its own repo refs)
    let recorder =
        ProtocolExecutionRecorder::new(&*env.state.repos().protocols, env.step_id, env.ctx.run_id);
    let exec_row = recorder
        .create_phase_with_context(
            &format!("agent_{}", agent_index),
            None,
            Some(&env.original_prompt),
            Some(&designed.agent_name),
            Some("workforce"),
            env.designer_run_id,
        )
        .await?;

    // Resolve capabilities from designer's tools list
    let (mut tools, mut tool_names) = env
        .state
        .capability_registry()
        .resolve_tools(&designed.tools);

    // C1: Inject baseline workspace tools when running in a container.
    // Shell access is implicit — the designer never assigns it.
    if env.container_handle.is_some() {
        let baseline = ["run_command"];
        for name in baseline {
            if !tool_names.contains(&name.to_string()) {
                if let Some(tool) = crate::tools::registry::get_tool_definition(name) {
                    tools.push(tool);
                    tool_names.push(name.to_string());
                }
            }
        }
    }

    // C2: Workspace grounding + file discipline for containerized agents.
    // Text lives in config/runtime_agent/system.md alongside the other agents'
    // prompts. Gated on the container: without one there is no run_command and
    // the guidance would be false.
    let system_prompt = if env.container_handle.is_some() {
        format!(
            "{}\n\n{}",
            designed.system_prompt,
            roles::WORKFORCE_AGENT.system
        )
    } else {
        designed.system_prompt.clone()
    };

    // Build task prompt: <previous_step> + <assignment> + <deliverable>
    let filtered = filter_outputs_for_agent(prior_outputs, &designed.receives_from);
    let previous_step = if filtered.is_empty() {
        // First agent (or no receives_from) — use upstream DAG step output
        env.upstream_step_output.clone()
    } else {
        // Has prior agent outputs — use those as previous_step
        build_filtered_outputs_block(&filtered)
    };

    let task_prompt = TaskPromptBuilder {
        previous_step,
        assignment: designed.assignment.clone(),
        expected_output: designed.expected_output.clone(),
    }
    .build();

    // Create agent_execution row for message persistence
    let ae_repo = &*env.state.repos().agent_executions;
    let ae_id = match ae_repo
        .create_agent_execution(CreateAgentExecutionInput {
            execution_type: ExecutionType::PipelineAgent,
            agent_id: None,
            workflow_step_id: Some(env.step_id),
            parent_agent_execution_id: None,
            system_prompt_rendered: system_prompt.clone(),
            input: task_prompt.clone(),
            room_session_id: None,
            speaker_order: None,
            workflow_execution_id: Some(env.ctx.stage_execution_id),
        })
        .await
    {
        Ok(row) => {
            let _ = ae_repo
                .create_execution_message(row.id, "system", &system_prompt, None, 0, 0)
                .await;
            let _ = ae_repo
                .create_execution_message(row.id, "user", &task_prompt, None, 0, 0)
                .await;
            Some(row.id)
        }
        Err(e) => {
            warn!(agent = %designed.agent_name, error = %e, "Failed to create agent execution");
            None
        }
    };

    // Build diagnostics engine (per-agent, stateful across run_command calls)
    let diagnostics = if env.container_handle.is_some() {
        Some(Arc::new(tokio::sync::Mutex::new(DiagnosticsEngine::new())))
    } else {
        None
    };
    // Retained so the produced-file list can be read back after execution.
    let diagnostics_ref = diagnostics.clone();

    // Build strategy
    let strategy = WorkforceAgentStrategy::new(WorkforceAgentConfig {
        system_prompt,
        model_id: agent_cfg.model_id.clone(),
        temperature: agent_cfg.temperature,
        max_rounds: agent_cfg.max_rounds,
        context_budget: agent_cfg.context_budget,
        tools,
        tool_names,
        execution_context: env.ctx.execution_context.clone(),
        container_handle: env.container_handle.clone(),
        state: Some(env.state.clone()),
        user_id: Some(UserId(env.ctx.user_id)),
        agent_execution_id: ae_id,
        stroke_image: env.stroke_image.clone(),
        workflow_id: Some(env.workflow_id),
        step_id: Some(env.step_id),
        agent_name: Some(designed.agent_name.clone()),
        workflow_run_id: Some(env.ctx.run_id),
        diagnostics,
    });

    // Execute with live streaming sink
    let inner_recorder = ExecutionRecorder::new(
        &*env.state.repos().sessions,
        &*env.state.repos().chat_messages,
        Some(&*env.state.repos().agent_executions),
        Some(&*env.state.repos().token_ledger),
    );
    let sink = DagStreamSink::new(
        env.state.clone(),
        env.ctx.clone(),
        env.workflow_id,
        env.step_id,
        designed.agent_roster_entry_id,
        designed.agent_name.clone(),
    )
    .with_agent_name(Some(designed.agent_name.clone()));
    let result = engine
        .execute(
            &strategy,
            &task_prompt,
            &sink,
            &inner_recorder,
            env.cancel.as_ref(),
        )
        .await;

    match result {
        Ok(exec_result) => {
            let cost = compute_cost(
                &agent_cfg.model_id,
                exec_result.input_tokens as i64,
                exec_result.output_tokens as i64,
            );

            recorder
                .update_phase(
                    exec_row.id,
                    PhaseCompletion {
                        status: "complete",
                        output_content: Some(&exec_result.content),
                        error_message: None,
                        tokens_in: exec_result.input_tokens as i64,
                        tokens_out: exec_result.output_tokens as i64,
                        cost_usd: cost,
                        model: Some(&agent_cfg.model_id),
                    },
                )
                .await;

            info!(
                agent = %designed.agent_name,
                idx = agent_index,
                tokens_in = exec_result.input_tokens,
                tokens_out = exec_result.output_tokens,
                "Workforce agent completed"
            );

            broadcast_workflow_event(
                &env.state,
                &env.ctx,
                env.workflow_id,
                WorkflowEventKind::WorkforceAgentProgress {
                    step_id: env.step_id,
                    agent_name: designed.agent_name.clone(),
                    roster_agent_id: designed.agent_roster_entry_id,
                    agent_index,
                    total_agents: env.total_agents,
                    status: "completed".to_string(),
                },
            );

            // When the LLM returns EndTurn with no text after tool use,
            // synthesize a summary from recorded tool results so downstream
            // agents have context about what this agent did.
            let content = if exec_result.content.trim().is_empty() && exec_result.rounds_used > 1 {
                if let Some(id) = ae_id {
                    synthesize_tool_summary(id, &env.state).await
                } else {
                    format!("[Completed {} rounds of tool use]", exec_result.rounds_used)
                }
            } else {
                exec_result.content
            };

            // Attach the objective file manifest. The agent's prose says which
            // file matters and why; this says what actually landed on disk.
            let content = append_files_line(content, diagnostics_ref.as_deref()).await;

            Ok(AgentExecutionResult {
                name: designed.agent_name.clone(),
                content,
                input_tokens: exec_result.input_tokens as i64,
                output_tokens: exec_result.output_tokens as i64,
                cost,
            })
        }
        Err(e) => {
            let err_msg = format!("{}", e);
            recorder
                .update_phase(
                    exec_row.id,
                    PhaseCompletion {
                        status: "failed",
                        output_content: None,
                        error_message: Some(&err_msg),
                        tokens_in: 0,
                        tokens_out: 0,
                        cost_usd: 0.0,
                        model: Some(&agent_cfg.model_id),
                    },
                )
                .await;

            broadcast_workflow_event(
                &env.state,
                &env.ctx,
                env.workflow_id,
                WorkflowEventKind::WorkforceAgentProgress {
                    step_id: env.step_id,
                    agent_name: designed.agent_name.clone(),
                    roster_agent_id: designed.agent_roster_entry_id,
                    agent_index,
                    total_agents: env.total_agents,
                    status: "failed".to_string(),
                },
            );

            warn!(
                agent = %designed.agent_name,
                error = %err_msg,
                "Workforce agent failed"
            );
            Err(e)
        }
    }
}

/// When an agent completes with empty text (tool-only execution),
/// synthesize a summary from recorded tool results so downstream agents
/// have context about what was done (e.g., which files were created/modified).
async fn synthesize_tool_summary(
    ae_id: uuid::Uuid,
    state: &crate::server::state::AppState,
) -> String {
    let messages = match state
        .repos()
        .agent_executions
        .list_execution_messages(ae_id)
        .await
    {
        Ok(m) => m,
        Err(_) => return "[Agent completed via tool use]".to_string(),
    };

    let tool_messages: Vec<_> = messages.iter().filter(|m| m.role == "tool").collect();

    if tool_messages.is_empty() {
        return "[Agent completed via tool use]".to_string();
    }

    // Extract file changes from diagnostics output
    let mut files_created = Vec::new();
    let mut files_modified = Vec::new();

    for msg in &tool_messages {
        // Tool content may be JSON ({"output": "..."}) or plain text
        let text = serde_json::from_str::<serde_json::Value>(&msg.content)
            .ok()
            .and_then(|v| v.get("output").and_then(|o| o.as_str()).map(String::from))
            .unwrap_or_else(|| msg.content.clone());

        for line in text.lines() {
            let trimmed = line.trim();
            if let Some(name) = trimmed.strip_prefix("created: ") {
                files_created.push(name.to_string());
            } else if let Some(name) = trimmed.strip_prefix("modified: ") {
                files_modified.push(name.to_string());
            }
        }
    }

    let mut parts = Vec::new();
    if !files_created.is_empty() {
        parts.push(format!("Created {}", files_created.join(", ")));
    }
    if !files_modified.is_empty() {
        parts.push(format!("Modified {}", files_modified.join(", ")));
    }

    if parts.is_empty() {
        format!("[Completed {} tool calls]", tool_messages.len())
    } else {
        format!("Task complete: {}.", parts.join(". "))
    }
}

/// Assembles the task prompt for a workforce agent from 3 blocks.
///
/// Block order:
/// 1. `<previous_step>` — orientation from whoever ran before (omitted if empty)
/// 2. `<assignment>` — what to do (always present)
/// 3. `<deliverable>` — the file contract: what the saved file must contain
///    (optional, from designer)
pub(super) struct TaskPromptBuilder {
    pub(super) previous_step: String,
    pub(super) assignment: String,
    pub(super) expected_output: Option<String>,
}

impl TaskPromptBuilder {
    pub(super) fn build(self) -> String {
        let mut prompt = String::new();

        if !self.previous_step.is_empty() {
            prompt.push_str(&format!(
                "<previous_step>\n{}\n</previous_step>\n\n",
                self.previous_step
            ));
        }

        prompt.push_str(&format!("<assignment>\n{}\n</assignment>", self.assignment,));

        if let Some(expected) = &self.expected_output {
            if !expected.is_empty() {
                prompt.push_str(&format!(
                    "\n\n<deliverable>\n{}\n</deliverable>\n\n\
                     Save this to a file with run_command before you reply. Your response \
                     should be a short receipt naming the file, not the deliverable itself.",
                    expected
                ));
            }
        }

        prompt
    }
}
