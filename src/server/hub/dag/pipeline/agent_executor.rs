//! Agent execution for the workforce step.
//!
//! Handles both single-agent sequential execution and parallel level-based
//! dispatch via `tokio::task::JoinSet`.

use anyhow::anyhow;
use tracing::{error, info, warn};

use crate::config::protocols::WORKFORCE;
use crate::db::traits::CreateAgentExecutionInput;
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

    // Resolve capabilities
    let (tools, tool_names) = env
        .state
        .capability_registry()
        .resolve_tools(&designed.tools);

    // Build task prompt: shared mission context + designer's assignment
    let filtered = filter_outputs_for_agent(prior_outputs, &designed.receives_from);
    let mut task_prompt = format!(
        "<context>\n{}\n</context>\n\n<assignment>\n{}\n</assignment>",
        env.task_description, designed.assignment,
    );

    // Append upstream agent outputs
    if !filtered.is_empty() {
        let previous_outputs = build_filtered_outputs_block(&filtered);
        task_prompt = format!(
            "{}\n\n<previous_agent_outputs>\n{}\n</previous_agent_outputs>",
            task_prompt, previous_outputs
        );
    }

    // Inject user notes
    if !env.user_notes_block.is_empty() {
        task_prompt = format!("{}\n\n{task_prompt}", env.user_notes_block);
    }

    // Create agent_execution row for message persistence
    let ae_repo = &*env.state.repos().agent_executions;
    let ae_id = match ae_repo
        .create_agent_execution(CreateAgentExecutionInput {
            execution_type: ExecutionType::PipelineAgent,
            agent_id: None,
            workflow_step_id: Some(env.step_id),
            parent_agent_execution_id: None,
            system_prompt_rendered: designed.system_prompt.clone(),
            input: task_prompt.clone(),
            room_session_id: None,
            speaker_order: None,
            workflow_execution_id: Some(env.ctx.stage_execution_id),
        })
        .await
    {
        Ok(row) => {
            let _ = ae_repo
                .create_execution_message(row.id, "system", &designed.system_prompt, None, 0, 0)
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

    // Build strategy
    let strategy = WorkforceAgentStrategy::new(WorkforceAgentConfig {
        system_prompt: designed.system_prompt.clone(),
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
    );
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

            Ok(AgentExecutionResult {
                name: designed.agent_name.clone(),
                content: exec_result.content,
                input_tokens: exec_result.input_tokens as i64,
                output_tokens: exec_result.output_tokens as i64,
                cost,
                roster_agent_id: designed.agent_roster_entry_id,
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
