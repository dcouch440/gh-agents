//! Workforce step execution within the DAG.
//!
//! When the DAG encounters a step with `execution_mode = "workforce"`, this
//! module loads the mission brief and agent roster, runs the Agent Designer
//! pre-lifecycle to generate optimized prompts, then executes each roster
//! agent sequentially with designed prompts.

mod tests;

use anyhow::anyhow;
use serde_json::Value as JsonValue;
use tracing::{info, warn};
use uuid::Uuid;

use crate::config::protocols::{roles, vars, WORKFORCE};
use crate::db::traits::CreateAgentExecutionInput;
use crate::db::WorkflowStepRow;
use crate::db::{TaskAgentRosterRow, TaskMissionBriefRow};
use crate::server::hub::capability_resolver::resolve_capabilities_to_tools;
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
use crate::types::{ExecutionMetadata, ExecutionStatus, StepExecutionEnvelope, UserId};

use crate::server::hub::protocols::context::{build_context_block, ContextDocument};

use super::agent_designer::{self, normalize_agent_name};
use super::container::{create_optional_container, destroy_optional_container};
use super::dag_state::DagExecutionState;
use super::designer_input::workforce::build_workforce_designer_input;
use super::utils::collect_upstream_context_data;
use super::{
    broadcast_workflow_event, compose_prompt, resolve_output_key, resolve_step_port_inputs,
    step_display_name, DagContext, PromptRepos, StepOutput,
};

// ── Workforce-owned types ─────────────────────────────────────────────────

/// Output from the Agent Designer — one prompt pair + tool assignment per agent.
#[derive(Debug, Clone)]
pub(crate) struct DesignedAgentPrompt {
    pub agent_roster_entry_id: Uuid,
    pub agent_name: String,
    pub tools: Vec<String>,
    pub system_prompt: String,
    pub task_prompt: String,
    pub execution_order: i32,
    pub receives_from: Vec<String>,
}

/// Token usage from the designer call, for accumulating into step totals.
pub(crate) struct DesignerTokenUsage {
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cost_usd: f32,
    /// The designer run ID for linking agent phases back to their designer.
    pub run_id: Uuid,
}

/// Execute a workforce step within the DAG.
///
/// Loads the mission brief, agent roster, and deliverables, runs the Agent
/// Designer pre-lifecycle, then executes each roster agent sequentially.
/// The combined output includes agent results and deliverable metadata.
pub(super) async fn execute_workforce_step(
    dag: &DagContext<'_>,
    step: &WorkflowStepRow,
    dag_state: &mut DagExecutionState,
) -> Result<(), HubError> {
    let step_start = std::time::Instant::now();

    // 1. Broadcast step started
    broadcast_workflow_event(
        dag.state,
        dag.ctx,
        step.workflow_id,
        WorkflowEventKind::StepStarted {
            step_id: step.id,
            step_name: step_display_name(step),
            agent_id: None,
            execution_id: None,
        },
    );

    // 2. Load mission brief
    let brief = dag
        .state
        .repos()
        .workflows
        .get_mission_brief(step.id)
        .await
        .map_err(|e| HubError::Internal(anyhow!("failed to load mission brief: {}", e)))?
        .ok_or_else(|| {
            HubError::Internal(anyhow!("workforce step {} has no mission brief", step.id))
        })?;

    // 3. Load agent roster (sorted by execution_order)
    let roster = dag
        .state
        .repos()
        .workflows
        .list_agent_roster(brief.id)
        .await
        .map_err(|e| HubError::Internal(anyhow!("failed to load agent roster: {}", e)))?;

    if roster.is_empty() {
        return Err(HubError::Internal(anyhow!(
            "workforce step {} has empty agent roster",
            step.id
        )));
    }

    info!(
        step_id = %step.id,
        task = %brief.task_description,
        agents = roster.len(),
        failure_mode = %brief.failure_mode,
        "Starting workforce step execution"
    );

    // 5. Resolve port inputs
    let port_inputs = resolve_step_port_inputs(
        step,
        dag.port_meta,
        &dag_state.completed_envelopes,
    );

    // 5b. Collect upstream context from context nodes
    let incoming = dag.port_meta
        .incoming_edges
        .get(&step.id)
        .map(|v| v.as_slice())
        .unwrap_or(&[]);
    let upstream_context = collect_upstream_context_data(
        incoming,
        dag.steps,
        &dag_state.completed_envelopes,
    );

    // 6. Compose base prompt
    let repos = PromptRepos {
        prompt_template_repo: Some(&*dag.state.repos().prompt_templates),
        doc_repo: Some(&*dag.state.repos().documents),
        workflow_repo: Some(&*dag.state.repos().workflows),
        agent_repo: &*dag.state.repos().agents,
    };
    let prompt = compose_prompt(
        step,
        &repos,
        &dag_state.var_outputs,
        &dag.ctx.prior_outputs,
        None,
        port_inputs.as_ref(),
    )
    .await;

    // 7. Create protocol execution recorder
    let recorder =
        ProtocolExecutionRecorder::new(&*dag.state.repos().protocols, step.id, dag.ctx.run_id);

    // 8. Create optional container
    let managed_container = create_optional_container(
        dag.ctx.container_config.as_ref(),
        dag.ctx.wg_client.as_deref(),
        "workforce",
    )
    .await?;

    // 9. Run Agent Designer pre-lifecycle with deliverable context
    let designer_phase = recorder
        .create_phase_with_context("designer", None, None, None, Some("workforce"), None)
        .await?;

    // Broadcast designer started
    broadcast_workflow_event(
        dag.state,
        dag.ctx,
        step.workflow_id,
        WorkflowEventKind::WorkforceDesignerProgress {
            step_id: step.id,
            status: "started".to_string(),
        },
    );

    let child_edges = if let Some(child_wf_id) = step.child_workflow_id {
        dag.state
            .repos()
            .workflows
            .list_edges(child_wf_id)
            .await
            .unwrap_or_default()
    } else {
        vec![]
    };

    let designer_input = build_workforce_designer_input(
        &brief,
        &roster,
        &dag_state.completed_envelopes,
        dag.steps,
        dag.state
            .repos()
            .workflows
            .get_assistant_notes(step.id)
            .await
            .unwrap_or_default()
            .as_deref(),
        &*dag.state.repos().tool_capabilities,
        &child_edges,
    )
    .await;

    let (designed_prompts, designer_usage) = match agent_designer::run_agent_designer(
        dag.engine,
        dag.state,
        dag.ctx,
        step,
        designer_input,
        "",
        dag.cancel,
        Some(designer_phase.id),
    )
    .await
    {
        Ok(result) => {
            recorder
                .update_phase(
                    designer_phase.id,
                    PhaseCompletion {
                        status: "complete",
                        output_content: None,
                        error_message: None,
                        tokens_in: result.input_tokens,
                        tokens_out: result.output_tokens,
                        cost_usd: result.cost_usd,
                        model: Some("claude-sonnet-4-5-20250929"),
                    },
                )
                .await;

            // Map generic results to task-force-compatible prompts
            let prompts = map_designer_results(&result, &roster)?;

            let usage = DesignerTokenUsage {
                input_tokens: result.input_tokens,
                output_tokens: result.output_tokens,
                cost_usd: result.cost_usd,
                run_id: result.run_id,
            };

            broadcast_workflow_event(
                dag.state,
                dag.ctx,
                step.workflow_id,
                WorkflowEventKind::WorkforceDesignerProgress {
                    step_id: step.id,
                    status: "completed".to_string(),
                },
            );

            (prompts, usage)
        }
        Err(e) => {
            recorder
                .update_phase(
                    designer_phase.id,
                    PhaseCompletion {
                        status: "failed",
                        output_content: None,
                        error_message: Some(&e.to_string()),
                        tokens_in: 0,
                        tokens_out: 0,
                        cost_usd: 0.0,
                        model: None,
                    },
                )
                .await;

            broadcast_workflow_event(
                dag.state,
                dag.ctx,
                step.workflow_id,
                WorkflowEventKind::WorkforceDesignerProgress {
                    step_id: step.id,
                    status: "failed".to_string(),
                },
            );

            warn!(
                step_id = %step.id,
                error = %e,
                "Workforce designer failed, using static prompts"
            );
            let fallback = build_static_fallback_prompts(&brief, &roster, &prompt);
            let usage = DesignerTokenUsage {
                input_tokens: 0,
                output_tokens: 0,
                cost_usd: 0.0,
                run_id: Uuid::nil(),
            };
            (fallback, usage)
        }
    };

    // 9b. Build user_notes block from context nodes
    let user_notes_block = if upstream_context.is_empty() {
        String::new()
    } else {
        let docs: Vec<ContextDocument> = upstream_context
            .iter()
            .map(|(title, content)| {
                use std::collections::hash_map::DefaultHasher;
                use std::hash::{Hash, Hasher};
                let mut hasher = DefaultHasher::new();
                title.hash(&mut hasher);
                let short_id = format!("{:08x}", hasher.finish() & 0xFFFF_FFFF);
                ContextDocument {
                    short_id,
                    title: title.clone(),
                    content: content.clone(),
                }
            })
            .collect();
        let inner = build_context_block(&[], &docs);
        format!("<user_notes>\n{inner}\n</user_notes>")
    };

    // 10. Sequential agent execution loop
    let mut agent_outputs: Vec<(String, String)> = Vec::new();
    let mut step_in_tokens: i64 = designer_usage.input_tokens;
    let mut step_out_tokens: i64 = designer_usage.output_tokens;
    let mut step_cost: f32 = designer_usage.cost_usd;
    let total_agents = designed_prompts.len();
    let agent_cfg = WORKFORCE.agent("agent");

    for (idx, designed) in designed_prompts.iter().enumerate() {
        // Check cancellation
        if dag.cancel.is_some_and(|t| t.is_cancelled()) {
            destroy_optional_container(&managed_container, dag.ctx.wg_client.as_deref()).await;
            return Err(HubError::Cancelled);
        }

        // Broadcast agent progress
        broadcast_workflow_event(
            dag.state,
            dag.ctx,
            step.workflow_id,
            WorkflowEventKind::WorkforceAgentProgress {
                step_id: step.id,
                agent_name: designed.agent_name.clone(),
                roster_agent_id: designed.agent_roster_entry_id,
                agent_index: idx,
                total_agents,
                status: "started".to_string(),
            },
        );

        // Create protocol execution row
        let designer_run_id = if designer_usage.run_id.is_nil() {
            None
        } else {
            Some(designer_usage.run_id)
        };
        let exec_row = recorder
            .create_phase_with_context(
                &format!("agent_{}", idx),
                None,
                Some(&prompt),
                Some(&designed.agent_name),
                Some("workforce"),
                designer_run_id,
            )
            .await?;

        // Resolve capabilities
        let (tools, tool_names) =
            resolve_capabilities_to_tools(&designed.tools, &*dag.state.repos().tool_capabilities)
                .await
                .unwrap_or_else(|e| {
                    warn!(agent = %designed.agent_name, "Capability resolution failed: {}", e);
                    (vec![], vec![])
                });

        // Inject previous outputs filtered by designer routing
        let filtered = filter_outputs_for_agent(&agent_outputs, &designed.receives_from);
        let mut task_prompt = if filtered.is_empty() {
            designed.task_prompt.clone()
        } else {
            let previous_outputs = build_filtered_outputs_block(&filtered);
            format!(
                "{}\n\n<previous_agent_outputs>\n{}\n</previous_agent_outputs>",
                designed.task_prompt, previous_outputs
            )
        };

        // Inject user notes
        if !user_notes_block.is_empty() {
            task_prompt = format!("{user_notes_block}\n\n{task_prompt}");
        }

        // Create agent_execution row for message persistence
        let ae_repo = &*dag.state.repos().agent_executions;
        let ae_id = match ae_repo
            .create_agent_execution(CreateAgentExecutionInput {
                agent_id: None,
                workflow_step_id: Some(step.id),
                is_interactive: false,
                parent_agent_execution_id: None,
                system_prompt_rendered: designed.system_prompt.clone(),
                input: task_prompt.clone(),
                room_session_id: None,
                speaker_order: None,
                workflow_execution_id: Some(dag.ctx.stage_execution_id),
            })
            .await
        {
            Ok(row) => {
                // Record initial messages
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
            execution_context: dag.ctx.execution_context.clone(),
            container_handle: managed_container.as_ref().map(|mc| mc.agent_handle.clone()),
            state: Some(dag.state.clone()),
            user_id: Some(UserId(dag.ctx.user_id)),
            agent_execution_id: ae_id,
        });

        // Execute with live streaming sink
        let inner_recorder = ExecutionRecorder::new(
            &*dag.state.repos().sessions,
            &*dag.state.repos().chat_messages,
            Some(&*dag.state.repos().agent_executions),
            Some(&*dag.state.repos().token_ledger),
        );
        let sink = DagStreamSink::new(
            dag.state.clone(),
            dag.ctx.clone(),
            step.workflow_id,
            step.id,
            designed.agent_roster_entry_id,
            designed.agent_name.clone(),
        );
        let result = dag
            .engine
            .clone_with_provider()
            .execute(&strategy, &task_prompt, &sink, &inner_recorder, dag.cancel)
            .await;

        match result {
            Ok(exec_result) => {
                let cost = compute_cost(
                    &agent_cfg.model_id,
                    exec_result.input_tokens as i64,
                    exec_result.output_tokens as i64,
                );
                step_in_tokens += exec_result.input_tokens as i64;
                step_out_tokens += exec_result.output_tokens as i64;
                step_cost += cost;

                agent_outputs.push((designed.agent_name.clone(), exec_result.content.clone()));

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
                    idx = idx,
                    tokens_in = exec_result.input_tokens,
                    tokens_out = exec_result.output_tokens,
                    "Workforce agent completed"
                );

                broadcast_workflow_event(
                    dag.state,
                    dag.ctx,
                    step.workflow_id,
                    WorkflowEventKind::WorkforceAgentProgress {
                        step_id: step.id,
                        agent_name: designed.agent_name.clone(),
                        roster_agent_id: designed.agent_roster_entry_id,
                        agent_index: idx,
                        total_agents,
                        status: "completed".to_string(),
                    },
                );
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
                    dag.state,
                    dag.ctx,
                    step.workflow_id,
                    WorkflowEventKind::WorkforceAgentProgress {
                        step_id: step.id,
                        agent_name: designed.agent_name.clone(),
                        roster_agent_id: designed.agent_roster_entry_id,
                        agent_index: idx,
                        total_agents,
                        status: "failed".to_string(),
                    },
                );

                match brief.failure_mode.as_str() {
                    "fail_fast" => {
                        warn!(
                            agent = %designed.agent_name,
                            error = %err_msg,
                            "Workforce agent failed (fail_fast)"
                        );
                        destroy_optional_container(
                            &managed_container,
                            dag.ctx.wg_client.as_deref(),
                        )
                        .await;
                        return Err(e);
                    }
                    _ => {
                        warn!(
                            agent = %designed.agent_name,
                            error = %err_msg,
                            "Workforce agent failed, skipping ({})", brief.failure_mode
                        );
                        agent_outputs.push((
                            designed.agent_name.clone(),
                            format!("[AGENT FAILED: {}]", err_msg),
                        ));
                    }
                }
            }
        }
    }

    // 11. Destroy optional container
    destroy_optional_container(&managed_container, dag.ctx.wg_client.as_deref()).await;

    // 12. Compose combined output
    let combined_data = compose_workforce_output(&agent_outputs, &roster);
    let output_key = resolve_output_key(step, &dag.port_meta.step_outputs);

    // 13. Store results
    dag_state.accumulate_tokens(step_in_tokens, step_out_tokens, step_cost);

    let output = StepOutput {
        variable_name: output_key,
        raw_output: serde_json::to_string(&combined_data).unwrap_or_default(),
        structured_output: Some(combined_data.clone()),
    };

    let envelope = StepExecutionEnvelope {
        status: ExecutionStatus::Success,
        data: Some(combined_data),
        metadata: ExecutionMetadata {
            execution_time_ms: step_start.elapsed().as_millis() as u64,
            tokens_in: Some(step_in_tokens as i32),
            tokens_out: Some(step_out_tokens as i32),
            cost_usd: Some(step_cost as f64),
            model: Some(agent_cfg.model_id.clone()),
            ..ExecutionMetadata::new(step.id)
        },
        error: None,
    };

    let envelope_json = serde_json::to_string(&envelope).unwrap_or_default();
    dag_state.record_step_output(step.id, output, envelope);
    let _ = super::versioning::snapshot_content(
        &*dag.state.repos().content_versions,
        dag.ctx.run_id,
        step.id,
        step.id,
        super::versioning::content_types::ENVELOPE,
        "output",
        &envelope_json,
    )
    .await;

    // 14. Broadcast step completed
    broadcast_workflow_event(
        dag.state,
        dag.ctx,
        step.workflow_id,
        WorkflowEventKind::StepCompleted {
            step_id: step.id,
            step_name: step_display_name(step),
            agent_id: None,
            output: None,
            input_tokens: Some(step_in_tokens as u64),
            output_tokens: Some(step_out_tokens as u64),
            duration_ms: Some(step_start.elapsed().as_millis() as u64),
        },
    );

    info!(
        step_id = %step.id,
        agents = total_agents,
        tokens_in = step_in_tokens,
        tokens_out = step_out_tokens,
        duration_ms = step_start.elapsed().as_millis(),
        "Workforce step execution completed"
    );

    Ok(())
}

// ── Helper functions ─────────────────────────────────────────────────────

/// Map generic designer results to workforce `DesignedAgentPrompt`s.
fn map_designer_results(
    result: &agent_designer::DesignerResult,
    roster: &[TaskAgentRosterRow],
) -> Result<Vec<DesignedAgentPrompt>, HubError> {
    let mut prompts = Vec::with_capacity(result.prompts.len());

    for entry in &result.prompts {
        let roster_entry = roster
            .iter()
            .find(|r| r.id.to_string() == entry.agent_id)
            .ok_or_else(|| {
                HubError::Internal(anyhow!(
                    "Designer referenced unknown agent_id: {}",
                    entry.agent_id
                ))
            })?;

        prompts.push(DesignedAgentPrompt {
            agent_roster_entry_id: roster_entry.id,
            agent_name: entry.agent_name.clone(),
            tools: entry.tools.clone(),
            system_prompt: entry.system_prompt.clone(),
            task_prompt: entry.task_prompt.clone(),
            execution_order: roster_entry.execution_order,
            receives_from: entry.receives_from.clone(),
        });
    }

    prompts.sort_by_key(|p| p.execution_order);
    Ok(prompts)
}

/// Build static fallback prompts when the Agent Designer fails.
fn build_static_fallback_prompts(
    brief: &TaskMissionBriefRow,
    roster: &[TaskAgentRosterRow],
    base_prompt: &str,
) -> Vec<DesignedAgentPrompt> {
    let team_roster = build_team_roster_string(roster);

    roster
        .iter()
        .map(|entry| {
            let mut v = std::collections::HashMap::new();
            v.insert(vars::workforce::AGENT_NAME.into(), entry.name.clone());
            v.insert(
                vars::workforce::ROLE_DESCRIPTION.into(),
                entry.role_description.clone(),
            );
            v.insert(
                vars::workforce::TASK_DESCRIPTION.into(),
                brief.task_description.clone(),
            );
            v.insert(vars::workforce::TEAM_ROSTER.into(), team_roster.clone());
            v.insert(vars::workforce::PREVIOUS_OUTPUTS.into(), String::new());
            v.insert(vars::user::PROMPT.into(), base_prompt.to_string());

            let role_ctx = roles::WORKFORCE_AGENT.resolve(&v);

            DesignedAgentPrompt {
                agent_roster_entry_id: entry.id,
                agent_name: entry.name.clone(),
                tools: entry.capabilities.clone(),
                system_prompt: role_ctx.system_prompt,
                task_prompt: role_ctx.user_prompt,
                execution_order: entry.execution_order,
                receives_from: vec![],
            }
        })
        .collect()
}

/// Build a team roster string for fallback prompts.
pub(crate) fn build_team_roster_string(roster: &[TaskAgentRosterRow]) -> String {
    roster
        .iter()
        .map(|a| {
            let caps = if a.capabilities.is_empty() {
                String::new()
            } else {
                format!(" [{}]", a.capabilities.join(", "))
            };

            format!(
                "- **{}** (order {}): {}{}",
                a.name, a.execution_order, a.role_description, caps
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Filter agent outputs based on receives_from routing.
pub(crate) fn filter_outputs_for_agent<'a>(
    agent_outputs: &'a [(String, String)],
    receives_from: &[String],
) -> Vec<&'a (String, String)> {
    if receives_from.is_empty() {
        agent_outputs.iter().collect()
    } else {
        let normalized_receives: std::collections::HashSet<String> = receives_from
            .iter()
            .map(|n| normalize_agent_name(n))
            .collect();
        agent_outputs
            .iter()
            .filter(|(name, _)| normalized_receives.contains(&normalize_agent_name(name)))
            .collect()
    }
}

/// Build filtered outputs block for injection.
pub(crate) fn build_filtered_outputs_block(outputs: &[&(String, String)]) -> String {
    if outputs.is_empty() {
        "No previous agent outputs yet. You are the first agent to execute.".to_string()
    } else {
        outputs
            .iter()
            .map(|(name, output)| format!("### {}\n{}", name, output))
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}

/// Compose workforce output: agent results keyed by normalized name.
pub(crate) fn compose_workforce_output(
    agent_outputs: &[(String, String)],
    _roster: &[TaskAgentRosterRow],
) -> JsonValue {
    let mut composite = serde_json::Map::new();

    // Agent outputs keyed by normalized name
    let mut agents = serde_json::Map::new();
    for (name, output) in agent_outputs {
        let key = name.to_lowercase().replace(' ', "_");
        let value: JsonValue =
            serde_json::from_str(output).unwrap_or_else(|_| JsonValue::String(output.clone()));
        agents.insert(key, value);
    }
    composite.insert("agents".to_string(), JsonValue::Object(agents));

    JsonValue::Object(composite)
}
