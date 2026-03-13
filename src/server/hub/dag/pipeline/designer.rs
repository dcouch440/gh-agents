//! Agent Designer lifecycle phase for the workforce pipeline.
//!
//! Implements `PipelinePhase` to generate optimized prompts for each
//! roster agent via the Agent Designer LLM call. Falls back to static
//! prompts when the designer fails — never propagates a designer error.

use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use anyhow::anyhow;
use tracing::{info, warn};
use uuid::Uuid;

use crate::config::protocols::{roles, vars, DESIGNER};
use crate::db::traits::CreateAgentExecutionInput;
use crate::db::{TaskAgentRosterRow, TaskMissionBriefRow, WorkflowStepEdgeRow};
use crate::server::hub::engine::filters::FilterContext;
use crate::server::hub::error::HubError;
use crate::server::hub::protocols::context::{build_context_block, ContextDocument};
use crate::server::hub::protocols::execution_recorder::{
    PhaseCompletion, ProtocolExecutionRecorder,
};
use crate::server::hub::recorder::ExecutionRecorder;
use crate::server::hub::strategies::react_designer::{ReactDesignerConfig, ReactDesignerStrategy};
use crate::server::services::system_store::store as system_store;
use crate::server::ws::events::WorkflowEventKind;
use crate::types::{ExecutionType, UserId};

use crate::server::hub::streaming::NullSink;

use super::super::agent_designer;
use super::super::designer_input::workforce::build_workforce_designer_input;
use super::super::{broadcast_workflow_event, DagContext};
use super::lifecycle::{PhaseOutput, PhaseTokenUsage, PipelineExecutionContext, PipelinePhase};
use super::output::build_team_roster_string;
use super::types::DesignedAgentPrompt;

/// Designer lifecycle phase — generates optimized prompts for workforce agents.
///
/// On success, maps Agent Designer LLM output to `DesignedAgentPrompt`s.
/// On failure, degrades gracefully to static prompts (never errors out).
pub(crate) struct DesignerPhase;

#[async_trait::async_trait]
impl PipelinePhase for DesignerPhase {
    fn name(&self) -> &str {
        "designer"
    }

    async fn execute(
        &self,
        dag: &DagContext<'_>,
        ctx: &PipelineExecutionContext,
    ) -> Result<PhaseOutput, HubError> {
        let recorder = ProtocolExecutionRecorder::new(
            &*dag.state.repos().protocols,
            ctx.step.id,
            dag.ctx.run_id,
        );
        let designer_phase = match recorder
            .create_phase_with_context("designer", None, None, None, Some("workforce"), None)
            .await
        {
            Ok(phase) => phase,
            Err(e) => {
                warn!(step_id = %ctx.step.id, error = %e, "Failed to create designer phase record, continuing without tracking");
                // Create a dummy row so we can continue — phase tracking is non-critical
                crate::db::ProtocolExecutionRow {
                    id: Uuid::new_v4(),
                    protocol_step_id: ctx.step.id,
                    workflow_run_id: Some(dag.ctx.run_id),
                    phase: "designer".to_string(),
                    document_def_id: None,
                    agent_id: None,
                    input_prompt: None,
                    output_content: None,
                    status: "running".to_string(),
                    error_message: None,
                    tokens_in: None,
                    tokens_out: None,
                    cost_usd: None,
                    model: None,
                    capabilities_used: None,
                    created_at: chrono::Utc::now(),
                    completed_at: None,
                    agent_name: None,
                    archetype: Some("workforce".to_string()),
                    designer_run_id: None,
                }
            }
        };

        broadcast_workflow_event(
            dag.state,
            dag.ctx,
            ctx.step.workflow_id,
            WorkflowEventKind::WorkforceDesignerProgress {
                step_id: ctx.step.id,
                status: "started".to_string(),
            },
        );

        let child_edges = if let Some(child_wf_id) = ctx.step.child_workflow_id {
            dag.state
                .repos()
                .workflows
                .list_edges(child_wf_id)
                .await
                .unwrap_or_default()
        } else {
            vec![]
        };

        // Check if configs already exist in the store (from designer_handoff at design time).
        // If so, reuse them — no need to re-run the designer during execution.
        if let Some(s3) = dag.state.s3() {
            let repo = dag.state.repos().system_files.as_ref();
            if let Ok(mut prompts) =
                parse_store_configs(s3, repo, ctx.step.workflow_id, ctx.step.id, &ctx.roster).await
            {
                info!(
                    step_id = %ctx.step.id,
                    agents = prompts.len(),
                    "Reusing existing store configs — skipping designer"
                );
                enforce_edge_routing(&mut prompts, &ctx.roster, &child_edges);

                recorder
                    .update_phase(
                        designer_phase.id,
                        PhaseCompletion {
                            status: "complete",
                            output_content: None,
                            error_message: None,
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
                    ctx.step.workflow_id,
                    WorkflowEventKind::WorkforceDesignerProgress {
                        step_id: ctx.step.id,
                        status: "completed".to_string(),
                    },
                );

                let user_notes_block = build_user_notes_block(&ctx.upstream_context);
                return Ok(PhaseOutput {
                    designed_prompts: prompts,
                    user_notes_block,
                    token_usage: PhaseTokenUsage {
                        input_tokens: 0,
                        output_tokens: 0,
                        cost_usd: 0.0,
                        run_id: None,
                    },
                });
            }
        }

        let designer_input = build_workforce_designer_input(
            &ctx.brief,
            &ctx.roster,
            &ctx.completed_envelopes,
            dag.steps,
            dag.state
                .repos()
                .workflows
                .get_plan(ctx.step.id)
                .await
                .unwrap_or_default()
                .as_deref(),
            dag.state.capability_registry(),
            &child_edges,
        );

        // Try the ReAct designer if S3 is available, fall back to one-shot
        let use_react = dag.state.s3().is_some();

        let (designed_prompts, token_usage) = if use_react {
            match run_react_designer(dag, ctx, &child_edges, &designer_phase.id).await {
                Ok((prompts, usage)) => {
                    recorder
                        .update_phase(
                            designer_phase.id,
                            PhaseCompletion {
                                status: "complete",
                                output_content: None,
                                error_message: None,
                                tokens_in: usage.input_tokens,
                                tokens_out: usage.output_tokens,
                                cost_usd: usage.cost_usd,
                                model: Some(&DESIGNER.agent("react_designer").model_id),
                            },
                        )
                        .await;

                    broadcast_workflow_event(
                        dag.state,
                        dag.ctx,
                        ctx.step.workflow_id,
                        WorkflowEventKind::WorkforceDesignerProgress {
                            step_id: ctx.step.id,
                            status: "completed".to_string(),
                        },
                    );

                    (prompts, usage)
                }
                Err(e) => {
                    warn!(
                        step_id = %ctx.step.id,
                        error = %e,
                        "ReAct designer failed, falling back to one-shot"
                    );
                    // Fall through to the one-shot path below
                    run_oneshot_designer(dag, ctx, designer_input, &recorder, &designer_phase.id)
                        .await
                }
            }
        } else {
            run_oneshot_designer(dag, ctx, designer_input, &recorder, &designer_phase.id).await
        };

        let user_notes_block = build_user_notes_block(&ctx.upstream_context);

        Ok(PhaseOutput {
            designed_prompts,
            user_notes_block,
            token_usage,
        })
    }
}

/// Run the one-shot designer (existing path).
async fn run_oneshot_designer(
    dag: &DagContext<'_>,
    ctx: &PipelineExecutionContext,
    designer_input: crate::server::hub::dag::designer_input::DesignerInput,
    recorder: &ProtocolExecutionRecorder<'_>,
    phase_id: &Uuid,
) -> (Vec<DesignedAgentPrompt>, PhaseTokenUsage) {
    let child_edges = if let Some(child_wf_id) = ctx.step.child_workflow_id {
        dag.state
            .repos()
            .workflows
            .list_edges(child_wf_id)
            .await
            .unwrap_or_default()
    } else {
        vec![]
    };

    match agent_designer::run_agent_designer(
        dag.engine,
        dag.state,
        dag.ctx,
        &ctx.step,
        designer_input,
        "",
        dag.cancel,
        Some(*phase_id),
        &NullSink,
    )
    .await
    {
        Ok(result) => {
            recorder
                .update_phase(
                    *phase_id,
                    PhaseCompletion {
                        status: "complete",
                        output_content: None,
                        error_message: None,
                        tokens_in: result.input_tokens,
                        tokens_out: result.output_tokens,
                        cost_usd: result.cost_usd,
                        model: Some(&DESIGNER.agent("designer").model_id),
                    },
                )
                .await;

            let mut prompts = match map_designer_results(&result, &ctx.roster) {
                Ok(p) => p,
                Err(e) => {
                    warn!(error = %e, "Failed to map designer results, using fallback");
                    return (
                        build_static_fallback_prompts(&ctx.brief, &ctx.roster, &ctx.base_prompt),
                        PhaseTokenUsage {
                            input_tokens: 0,
                            output_tokens: 0,
                            cost_usd: 0.0,
                            run_id: None,
                        },
                    );
                }
            };
            enforce_edge_routing(&mut prompts, &ctx.roster, &child_edges);

            let usage = PhaseTokenUsage {
                input_tokens: result.input_tokens,
                output_tokens: result.output_tokens,
                cost_usd: result.cost_usd,
                run_id: Some(result.run_id),
            };

            broadcast_workflow_event(
                dag.state,
                dag.ctx,
                ctx.step.workflow_id,
                WorkflowEventKind::WorkforceDesignerProgress {
                    step_id: ctx.step.id,
                    status: "completed".to_string(),
                },
            );

            (prompts, usage)
        }
        Err(e) => {
            recorder
                .update_phase(
                    *phase_id,
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
                ctx.step.workflow_id,
                WorkflowEventKind::WorkforceDesignerProgress {
                    step_id: ctx.step.id,
                    status: "failed".to_string(),
                },
            );

            warn!(
                step_id = %ctx.step.id,
                error = %e,
                "Workforce designer failed, using static prompts"
            );
            let fallback = build_static_fallback_prompts(&ctx.brief, &ctx.roster, &ctx.base_prompt);
            let usage = PhaseTokenUsage {
                input_tokens: 0,
                output_tokens: 0,
                cost_usd: 0.0,
                run_id: None,
            };
            (fallback, usage)
        }
    }
}

/// Run the ReAct designer — writes configs to the store one at a time.
async fn run_react_designer(
    dag: &DagContext<'_>,
    ctx: &PipelineExecutionContext,
    child_edges: &[WorkflowStepEdgeRow],
    phase_id: &Uuid,
) -> Result<(Vec<DesignedAgentPrompt>, PhaseTokenUsage), HubError> {
    let plan = dag
        .state
        .repos()
        .workflows
        .get_plan(ctx.step.id)
        .await
        .unwrap_or_default()
        .unwrap_or_default();

    let designer_cfg = DESIGNER.agent("react_designer");

    // Create designer run record (for FK linkage to protocol_executions)
    let run_row = dag
        .state
        .repos()
        .workflows
        .create_designer_run_generic(
            dag.ctx.stage_execution_id,
            dag.ctx.stage_execution_id,
            ctx.step.id,
            "workforce",
            &phase_id.to_string(),
            &designer_cfg.model_id,
        )
        .await
        .map_err(|e| HubError::Internal(anyhow!("failed to create designer run: {e}")))?;

    // Create agent execution record
    let ae_repo = &*dag.state.repos().agent_executions;
    let designer_ae_id = ae_repo
        .create_agent_execution(CreateAgentExecutionInput {
            execution_type: ExecutionType::AgentDesigner,
            agent_id: None,
            workflow_step_id: Some(ctx.step.id),
            parent_agent_execution_id: None,
            system_prompt_rendered: String::new(),
            input: String::new(),
            room_session_id: None,
            speaker_order: None,
            workflow_execution_id: Some(dag.ctx.stage_execution_id),
        })
        .await
        .ok()
        .map(|row| row.id);

    // Deterministic session ID for designer persistence
    let designer_session_id = Uuid::new_v5(
        &Uuid::NAMESPACE_OID,
        format!("designer:{}", ctx.step.id).as_bytes(),
    );

    let strategy = ReactDesignerStrategy::new(ReactDesignerConfig {
        state: dag.state.clone(),
        step_id: ctx.step.id,
        workflow_id: ctx.step.workflow_id,
        roster: ctx.roster.clone(),
        session_id: Some(designer_session_id),
        plan,
        builder_action: format!("Configured {}-agent roster", ctx.roster.len()),
        agent_execution_id: designer_ae_id,
    });

    let filter_ctx = FilterContext::new(&designer_cfg.model_id, ctx.step.id);
    let recorder = ExecutionRecorder::new(
        &*dag.state.repos().sessions,
        &*dag.state.repos().chat_messages,
        Some(&*dag.state.repos().agent_executions),
        Some(&*dag.state.repos().token_ledger),
    );

    info!(
        step_id = %ctx.step.id,
        agents = ctx.roster.len(),
        "Running ReAct designer"
    );

    let result = dag
        .engine
        .clone_with_provider()
        .with_filter_context(filter_ctx)
        .execute(&strategy, "", &NullSink, &recorder, dag.cancel)
        .await?;

    let cost = crate::server::hub::pricing::compute_cost(
        &designer_cfg.model_id,
        result.input_tokens as i64,
        result.output_tokens as i64,
    );

    // Parse designed configs from the store
    let s3 = dag
        .state
        .s3()
        .ok_or_else(|| HubError::Internal(anyhow!("S3 not available")))?;
    let repo = dag.state.repos().system_files.as_ref();
    let mut prompts =
        parse_store_configs(s3, repo, ctx.step.workflow_id, ctx.step.id, &ctx.roster).await?;

    enforce_edge_routing(&mut prompts, &ctx.roster, child_edges);

    let usage = PhaseTokenUsage {
        input_tokens: result.input_tokens as i64,
        output_tokens: result.output_tokens as i64,
        cost_usd: cost,
        run_id: Some(run_row.id),
    };

    // Persist design summary to session for next re-trigger
    if let Some(summary) = strategy.take_design_summary() {
        let _ = dag
            .state
            .repos()
            .sessions
            .insert_session_message(
                UserId(dag.ctx.user_id),
                designer_session_id,
                Uuid::new_v4(),
                "assistant".to_string(),
                summary,
            )
            .await;
    }

    Ok((prompts, usage))
}

/// Stored agent config JSON shape (what the designer writes to the store).
#[derive(serde::Deserialize)]
struct StoredAgentConfig {
    #[serde(default)]
    tools: Vec<String>,
    #[serde(default)]
    system_prompt: String,
    #[serde(default)]
    assignment: String,
    #[serde(default)]
    expected_output: Option<String>,
}

/// Read designed agent configs from the store and map to DesignedAgentPrompts.
async fn parse_store_configs(
    s3: &crate::server::services::system_store::s3::S3Backend,
    repo: &dyn crate::db::traits::SystemFileRepo,
    workflow_id: Uuid,
    step_id: Uuid,
    roster: &[TaskAgentRosterRow],
) -> Result<Vec<DesignedAgentPrompt>, HubError> {
    let prefix = format!("design/{}/agents/", step_id);
    let files = system_store::list_files(repo, workflow_id, &prefix)
        .await
        .map_err(|e| HubError::Internal(anyhow!("Failed to list store configs: {e}")))?;

    // Build slug → roster entry lookup
    let roster_by_slug: HashMap<String, &TaskAgentRosterRow> = roster
        .iter()
        .map(|r| (agent_designer::agent_name_to_slug(&r.name), r))
        .collect();

    let mut prompts = Vec::with_capacity(files.len());

    for file in &files {
        let slug = file
            .path
            .rsplit('/')
            .next()
            .and_then(|f| f.strip_suffix(".json"))
            .unwrap_or("");

        let roster_entry = match roster_by_slug.get(slug) {
            Some(entry) => *entry,
            None => {
                warn!(
                    slug = slug,
                    "Store config has no matching roster agent, skipping"
                );
                continue;
            }
        };

        let (bytes, _meta) = system_store::read_file(s3, repo, workflow_id, &file.path)
            .await
            .map_err(|e| {
                HubError::Internal(anyhow!("Failed to read store config {}: {e}", file.path))
            })?;

        let config: StoredAgentConfig = serde_json::from_slice(&bytes).map_err(|e| {
            HubError::Internal(anyhow!("Failed to parse store config {}: {e}", file.path))
        })?;

        prompts.push(DesignedAgentPrompt {
            agent_roster_entry_id: roster_entry.id,
            agent_name: roster_entry.name.clone(),
            tools: config.tools,
            system_prompt: config.system_prompt,
            assignment: config.assignment,
            expected_output: config.expected_output,
            execution_order: roster_entry.execution_order,
            receives_from: vec![],
        });
    }

    if prompts.is_empty() {
        return Err(HubError::Internal(anyhow!(
            "ReAct designer produced no configs in the store"
        )));
    }

    prompts.sort_by_key(|p| p.execution_order);
    Ok(prompts)
}

// ── Shared Helpers ──────────────────────────────────────────────────────────

/// Build user notes block from upstream context node data.
///
/// Used by lifecycle phases to inject upstream context into agent task prompts.
pub(super) fn build_user_notes_block(upstream_context: &[(String, String)]) -> String {
    if upstream_context.is_empty() {
        return String::new();
    }

    let docs: Vec<ContextDocument> = upstream_context
        .iter()
        .map(|(title, content)| {
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
}

/// Build static fallback prompts when no designer phase runs or when the
/// designer fails. Maps roster agents directly to prompts using role templates.
pub(crate) fn build_static_fallback_prompts(
    brief: &TaskMissionBriefRow,
    roster: &[TaskAgentRosterRow],
    base_prompt: &str,
) -> Vec<DesignedAgentPrompt> {
    let team_roster = build_team_roster_string(roster);

    let mut base_vars = HashMap::with_capacity(6);
    base_vars.insert(
        vars::workforce::TASK_DESCRIPTION.into(),
        brief.task_description.clone(),
    );
    base_vars.insert(vars::workforce::TEAM_ROSTER.into(), team_roster);
    base_vars.insert(vars::workforce::PREVIOUS_OUTPUTS.into(), String::new());
    base_vars.insert(vars::user::PROMPT.into(), base_prompt.to_string());

    roster
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let mut v = base_vars.clone();
            v.insert(vars::workforce::AGENT_NAME.into(), entry.name.clone());
            v.insert(
                vars::workforce::ROLE_DESCRIPTION.into(),
                entry.role_description.clone(),
            );

            let role_ctx = roles::WORKFORCE_AGENT.resolve(&v);

            // Sequential chain: each agent receives from the previous one
            let receives_from = if i > 0 {
                vec![roster[i - 1].name.clone()]
            } else {
                vec![]
            };

            DesignedAgentPrompt {
                agent_roster_entry_id: entry.id,
                agent_name: entry.name.clone(),
                tools: entry.capabilities.clone(),
                system_prompt: role_ctx.system_prompt,
                assignment: entry.role_description.clone(),
                expected_output: None,
                execution_order: entry.execution_order,
                receives_from,
            }
        })
        .collect()
}

// ── Private Helpers ─────────────────────────────────────────────────────────

/// Map generic designer results to workforce `DesignedAgentPrompt`s.
fn map_designer_results(
    result: &agent_designer::DesignerResult,
    roster: &[TaskAgentRosterRow],
) -> Result<Vec<DesignedAgentPrompt>, HubError> {
    let roster_by_id: HashMap<String, &TaskAgentRosterRow> =
        roster.iter().map(|r| (r.id.to_string(), r)).collect();

    let mut prompts = Vec::with_capacity(result.prompts.len());

    for entry in &result.prompts {
        let roster_entry = roster_by_id.get(&entry.agent_id).ok_or_else(|| {
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
            assignment: entry.assignment.clone(),
            expected_output: entry.expected_output.clone(),
            execution_order: roster_entry.execution_order,
            receives_from: vec![],
        });
    }

    prompts.sort_by_key(|p| p.execution_order);
    Ok(prompts)
}

/// Override Designer-generated `receives_from` with the actual DB edge graph.
///
/// The Designer LLM may ignore or reorder dependencies. This function
/// ensures execution routing matches the edges that were explicitly
/// created (by `add_agent`, `set_dependency`, or `configure_team`).
///
/// When no edges exist, prompts are left unchanged (Designer routing stands).
pub(crate) fn enforce_edge_routing(
    prompts: &mut [DesignedAgentPrompt],
    roster: &[TaskAgentRosterRow],
    child_edges: &[WorkflowStepEdgeRow],
) {
    if child_edges.is_empty() {
        return;
    }

    // Build child_step_id → agent_name lookup
    let step_to_name: HashMap<uuid::Uuid, &str> = roster
        .iter()
        .filter_map(|r| r.child_step_id.map(|sid| (sid, r.name.as_str())))
        .collect();

    // Build agent_name → Vec<from_agent_name> from edges
    // (only agent-to-agent edges; Designer→agent edges are filtered out)
    let agent_names: std::collections::HashSet<&str> = step_to_name.values().copied().collect();
    let mut edge_receives: HashMap<&str, Vec<String>> = HashMap::new();

    for edge in child_edges {
        if let (Some(from_name), Some(to_name)) = (
            step_to_name.get(&edge.from_step_id).copied(),
            step_to_name.get(&edge.to_step_id).copied(),
        ) {
            if agent_names.contains(from_name) && agent_names.contains(to_name) {
                edge_receives
                    .entry(to_name)
                    .or_default()
                    .push(from_name.to_string());
            }
        }
    }

    // Override receives_from on each prompt
    for prompt in prompts.iter_mut() {
        if let Some(from_agents) = edge_receives.get(prompt.agent_name.as_str()) {
            prompt.receives_from = from_agents.clone();
        } else {
            // No incoming edges → root agent
            prompt.receives_from = vec![];
        }
    }
}
