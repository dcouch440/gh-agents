//! Agent execution for the workforce step.
//!
//! Handles both single-agent sequential execution and parallel level-based
//! dispatch via `tokio::task::JoinSet`.

use std::sync::Arc;

use anyhow::anyhow;
use tracing::{error, info, warn};

use crate::config::protocols::{roles, WORKFORCE};
use crate::db::traits::CreateAgentExecutionInput;
use std::collections::HashMap;

use crate::execution::diagnostics::types::FileChange;
use crate::execution::diagnostics::workspace::digest::format_size;
use crate::execution::diagnostics::DiagnosticsEngine;
use crate::server::hub::error::HubError;
use crate::server::hub::protocols::execution_recorder::{
    PhaseCompletion, ProtocolExecutionRecorder,
};
use crate::server::hub::recorder::ExecutionRecorder;
use crate::server::hub::strategies::workforce_agent::{
    WorkforceAgentConfig, WorkforceAgentStrategy,
};
use crate::server::hub::streaming::DagStreamSink;
use crate::server::ws::events::WorkflowEventKind;
use crate::types::ExecutionType;
use crate::types::UserId;

use super::super::{broadcast_workflow_event, DagContext};
use super::output::{
    build_filtered_outputs_block, compute_execution_levels, filter_outputs_for_agent,
};
use super::types::{
    AgentExecutionResult, AgentFailureAction, DesignedAgentPrompt, LevelExecutionResult,
    WorkforceStepEnv,
};

/// Maximum entries named in an agent's passdown manifest, after directories
/// have been rolled up.
const MAX_PASSDOWN_FILES: usize = 10;

/// Collapse a list of changed files into manifest entries, rolling any
/// directory holding more than one file into a single line.
///
/// A deliverable can be a directory, and a flat list of its contents is the
/// wrong shape for the passdown twice over: it spends the whole cap on one
/// deliverable, and it says nothing about the deliverable being one thing.
/// `tally/ (12 files, 38.4 KB)` is both shorter and more true than twelve
/// paths, ten of which fit.
///
/// Grouping is on the first path component only. Deeper nesting stays inside
/// its top-level group, so a source tree is one entry however deep it goes.
/// Files at the root are never grouped — each one is its own entry, which is
/// what keeps the single-file case reading exactly as it did before.
pub(crate) fn passdown_entries(files: &[FileChange]) -> (Vec<String>, usize) {
    /// One top-level path component's worth of changes.
    struct Group<'a> {
        size: u64,
        count: usize,
        /// Kept so a group that turns out to hold one file can name that file
        /// rather than reporting `dir/ (1 file, …)`, which is strictly less
        /// information than the path it replaced.
        first: &'a FileChange,
    }

    let mut order: Vec<String> = Vec::new();
    let mut groups: HashMap<String, Group> = HashMap::new();

    for f in files {
        let mut components = f.path.components();
        let Some(first) = components.next() else {
            continue;
        };
        // Files at the workspace root are their own group of one and stay
        // individually named — the single-file case reads exactly as before.
        let key = if components.next().is_some() {
            format!("{}/", first.as_os_str().to_string_lossy())
        } else {
            f.path.display().to_string()
        };

        groups
            .entry(key.clone())
            .and_modify(|g| {
                g.size += f.size;
                g.count += 1;
            })
            .or_insert_with(|| {
                order.push(key);
                Group {
                    size: f.size,
                    count: 1,
                    first: f,
                }
            });
    }

    let mut entries: Vec<(String, u64)> = order
        .into_iter()
        .map(|key| {
            let g = &groups[&key];
            let label = if g.count == 1 {
                format!(
                    "{} ({}, {})",
                    g.first.path.display(),
                    g.first.change_type,
                    format_size(g.first.size)
                )
            } else {
                format!("{key} ({} files, {})", g.count, format_size(g.size))
            };
            (label, g.size)
        })
        .collect();

    // Largest first so the deliverable leads, label as a tiebreak so the line
    // is deterministic.
    entries.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    let dropped = entries.len().saturating_sub(MAX_PASSDOWN_FILES);
    entries.truncate(MAX_PASSDOWN_FILES);
    (entries.into_iter().map(|(l, _)| l).collect(), dropped)
}

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

    let files = diag.lock().await.produced_files();
    if files.is_empty() {
        return content;
    }

    let (entries, dropped) = passdown_entries(&files);
    let mut line = entries.join(", ");

    if dropped > 0 {
        line.push_str(&format!(" (+{dropped} more)"));
    }

    format!("{content}\nfiles: {line}")
}

/// Tools every containerized workforce agent gets whether or not the designer
/// asked for them.
///
/// The inversion this fixes: `run_command` maps to `shell_execution`, which
/// `capabilities.yaml` marks `safety_level: unsafe`, `requires_approval: true`,
/// `default_enabled: false` — and it was the *only* tool injected. `file_read`
/// (safe) and `file_write` (caution) required an opt-in the designer prompt
/// actively discouraged, so every agent in every run wrote its deliverable
/// through shell heredocs, bounded invisibly by `max_tokens`.
pub(super) const CONTAINER_BASELINE_TOOLS: &[&str] = &[
    "read_file",
    "write_file",
    "edit_file",
    "list_files",
    "run_command",
];

/// Baseline for an agent the designer marked `read_only`.
///
/// A verifier that can write stops verifying — in run dd27d008 the QA agent
/// finished its checks and then spent its remaining rounds editing the code it
/// had just signed off.
pub(super) const READ_ONLY_BASELINE_TOOLS: &[&str] = &["read_file", "list_files"];

/// Append the baseline tools the designer did not already assign.
pub(super) fn inject_baseline_tools(
    tools: &mut Vec<crate::llm::Tool>,
    tool_names: &mut Vec<String>,
    baseline: &[&str],
) {
    for name in baseline {
        if tool_names.iter().any(|t| t == name) {
            continue;
        }
        if let Some(tool) = crate::tools::registry::get_tool_definition(name) {
            tools.push(tool);
            tool_names.push((*name).to_string());
        }
    }
}

/// Drop every writing tool from a resolved set.
///
/// The designer can still assign a writing capability by mistake, and the
/// baseline injection would otherwise add the full set back. Restriction runs
/// before injection so both routes are covered by one rule.
pub(super) fn restrict_to_read_only(
    tools: &mut Vec<crate::llm::Tool>,
    tool_names: &mut Vec<String>,
) {
    tools.retain(|t| crate::tools::registry::is_read_only_tool(&t.name));
    tool_names.retain(|n| crate::tools::registry::is_read_only_tool(n));
}

/// Terminate the `agent_executions` row for an agent that failed.
///
/// The success path runs `on_complete` → `complete_agent_execution` →
/// `update_agent_execution_status`. There was no error counterpart: the `Err`
/// arm of `execute_single_agent` only called `recorder.update_phase`, which
/// writes `protocol_executions` — a different table from the one the UI reads
/// through `get_running_step_ids_for_run`. Every failed workforce agent left a
/// permanent `running` row and a permanently spinning node.
///
/// `agent_executions` has no error column, so the message goes in `output`;
/// `completed_at` is stamped by the repo's `CASE` for `'failed'`.
pub(super) async fn fail_agent_execution(
    repo: &dyn crate::db::traits::AgentExecutionRepo,
    ae_id: Option<uuid::Uuid>,
    error: &str,
) {
    let Some(id) = ae_id else { return };
    if let Err(e) = repo
        .update_agent_execution_status(id, "failed", Some(format!("[FAILED] {}", error)), None)
        .await
    {
        warn!(
            agent_execution_id = %id,
            error = %e,
            "Failed to terminate agent execution row"
        );
    }
}

/// Decide how to handle an agent failure based on the error kind and the
/// configured failure mode.
///
/// Round exhaustion is special-cased ahead of `failure_mode`. An agent that ran
/// out of budget is not a broken agent — in run dd27d008 the QA agent had
/// already finished its verification pass (67/67 green) when the ceiling hit,
/// and aborting the step for that discarded seven successful agents' work.
pub(super) fn handle_agent_failure(
    error: HubError,
    agent_name: &str,
    failure_mode: &str,
) -> AgentFailureAction {
    if let HubError::MaxRoundsExhausted { max } = &error {
        let max = *max;
        warn!(
            agent = %agent_name,
            max_rounds = max,
            "Workforce agent exhausted its round budget, skipping"
        );
        return AgentFailureAction::Skip {
            name: agent_name.to_string(),
            error_output: format!(
                "[AGENT INCOMPLETE: ran out of tool rounds after {max}. Whatever it wrote is \
                 on disk, but it never wrote a receipt — read the files before relying on them.]"
            ),
        };
    }

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
///
/// The caller owns the container lifecycle — this function never destroys it,
/// so the step overlay can still be extracted after a failure. It used to tear
/// the container down on four of its own error paths, which is why run
/// dd27d008 lost its homepage: the overlay was gone before extraction ran.
pub(super) async fn execute_agent_levels(
    env: &WorkforceStepEnv,
    dag: &DagContext<'_>,
    designed_prompts: &[DesignedAgentPrompt],
    failure_mode: &str,
) -> Result<LevelExecutionResult, HubError> {
    let levels = compute_execution_levels(designed_prompts);
    let mut agent_outputs: Vec<(String, String)> = Vec::with_capacity(designed_prompts.len());
    let mut input_tokens: i64 = 0;
    let mut output_tokens: i64 = 0;
    let mut cost_usd: f32 = 0.0;

    for level_indices in &levels {
        if dag.cancel.is_some_and(|t| t.is_cancelled()) {
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
                    AgentFailureAction::Abort(err) => return Err(err),
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

    // A read-only agent loses every writing tool, including any the designer
    // assigned by mistake. Restriction runs before injection so both routes are
    // covered by one rule.
    if designed.read_only {
        restrict_to_read_only(&mut tools, &mut tool_names);
    }

    // C1: Inject the baseline workspace tools when running in a container.
    // Workspace access is implicit — the designer never assigns it.
    if env.container_handle.is_some() {
        let baseline = if designed.read_only {
            READ_ONLY_BASELINE_TOOLS
        } else {
            CONTAINER_BASELINE_TOOLS
        };
        inject_baseline_tools(&mut tools, &mut tool_names, baseline);
    }

    // C2: Workspace grounding + file discipline for containerized agents.
    // Text lives in config/runtime_agent/system.md alongside the other agents'
    // prompts. Gated on the container: without one there is no run_command and
    // the guidance would be false.
    //
    // The designed prompt is wrapped rather than merely concatenated. Two
    // prompts joined by a blank line read as one document, and the agent
    // treats the operational half as part of its persona — which is how a
    // pricing analyst ends up describing itself as a workspace. The tag is
    // also what lets the two halves be reordered later: static-first is the
    // cacheable order (this file is byte-identical for every agent in every
    // step; the designed prompt is unique per agent), and with a named seam
    // that flip is a one-line change instead of a prompt rewrite.
    //
    // Both branches wrap, so the agent sees one shape whether or not the run
    // has a container.
    let expertise = format!("<expertise>\n{}\n</expertise>", designed.system_prompt);
    let system_prompt = if env.container_handle.is_some() {
        format!("{}\n\n{}", expertise, roles::WORKFORCE_AGENT.system_text())
    } else {
        expertise
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
        has_container: env.container_handle.is_some(),
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
        max_tokens: agent_cfg.max_tokens,
        effort: agent_cfg.effort,
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
            let cost = crate::server::hub::pricing::compute_cost_cached(
                &agent_cfg.model_id,
                exec_result.input_tokens as i64,
                exec_result.cached_input_tokens as i64,
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

            // Close out the row the UI actually reads. `update_phase` below
            // only touches `protocol_executions`.
            fail_agent_execution(&*env.state.repos().agent_executions, ae_id, &err_msg).await;

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
/// 3. `<deliverable>` — the output contract, written by the designer: what to
///    produce and where it goes (optional)
///
/// The builder does not append a directive about where output belongs. That is
/// `expected_output`'s job, and the designer is the only layer that knows the
/// answer: whether the deliverable is one file, several, or — for a `read_only`
/// agent, a flag the designer itself sets — a report returned in the reply. A
/// hardcoded "save this to a file" contradicted the designed contract every
/// time an agent legitimately produced more than one file, or none.
pub(super) struct TaskPromptBuilder {
    pub(super) previous_step: String,
    pub(super) assignment: String,
    pub(super) expected_output: Option<String>,
    /// Whether the agent has a workspace container.
    ///
    /// The one fact here the designer cannot know — it depends on the run's
    /// container config, not the design. Without a container there are no
    /// workspace tools at all, so a `<deliverable>` describing a saved file is
    /// unachievable and the agent is told so.
    pub(super) has_container: bool,
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
                prompt.push_str(&format!("\n\n<deliverable>\n{}\n</deliverable>", expected));

                // Only the container fact is stated here, and only when it
                // makes the designed contract impossible. Everything else about
                // where output goes belongs to `expected_output`.
                if !self.has_container {
                    prompt.push_str(
                        "\n\nThere is no workspace in this step and no tools to write one, \
                         so whatever the deliverable describes has to be in your response.",
                    );
                }
            }
        }

        prompt
    }
}
