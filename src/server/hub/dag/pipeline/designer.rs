//! Agent Designer lifecycle phase for the workforce pipeline.
//!
//! Implements `PipelinePhase` to generate optimized prompts for each
//! roster agent via the Agent Designer LLM call. Falls back to static
//! prompts when the designer fails — never propagates a designer error.

use std::collections::HashMap;

use anyhow::anyhow;
use tracing::{info, warn};
use uuid::Uuid;

use crate::config::protocols::{roles, vars, DESIGNER};
use crate::db::traits::CreateAgentExecutionInput;
use crate::db::{TaskAgentRosterRow, TaskMissionBriefRow, WorkflowStepEdgeRow};
use crate::server::hub::engine::filters::FilterContext;
use crate::server::hub::error::HubError;
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
use super::super::{broadcast_workflow_event, DagContext};
use super::lifecycle::{PhaseOutput, PhaseTokenUsage, PipelineExecutionContext, PipelinePhase};
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

                return Ok(PhaseOutput {
                    designed_prompts: prompts,
                    token_usage: PhaseTokenUsage {
                        input_tokens: 0,
                        output_tokens: 0,
                        cost_usd: 0.0,
                        run_id: None,
                    },
                });
            }
        }

        // Run ReAct designer (writes configs to store). Falls back to static prompts on failure.
        let (designed_prompts, token_usage) = if dag.state.s3().is_some() {
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
                        "ReAct designer failed, using static fallback prompts"
                    );
                    broadcast_workflow_event(
                        dag.state,
                        dag.ctx,
                        ctx.step.workflow_id,
                        WorkflowEventKind::WorkforceDesignerProgress {
                            step_id: ctx.step.id,
                            status: "failed".to_string(),
                        },
                    );
                    let fallback =
                        build_static_fallback_prompts(&ctx.brief, &ctx.roster, &ctx.base_prompt);
                    (
                        fallback,
                        PhaseTokenUsage {
                            input_tokens: 0,
                            output_tokens: 0,
                            cost_usd: 0.0,
                            run_id: None,
                        },
                    )
                }
            }
        } else {
            // No S3 — static fallback only
            warn!(
                step_id = %ctx.step.id,
                "S3 not available, using static fallback prompts"
            );
            let fallback = build_static_fallback_prompts(&ctx.brief, &ctx.roster, &ctx.base_prompt);
            (
                fallback,
                PhaseTokenUsage {
                    input_tokens: 0,
                    output_tokens: 0,
                    cost_usd: 0.0,
                    run_id: None,
                },
            )
        };

        Ok(PhaseOutput {
            designed_prompts,
            token_usage,
        })
    }
}

/// Run the one-shot designer (existing path).
/// Run the ReAct designer — writes configs to the store one at a time.
async fn run_react_designer(
    dag: &DagContext<'_>,
    ctx: &PipelineExecutionContext,
    child_edges: &[WorkflowStepEdgeRow],
    phase_id: &Uuid,
) -> Result<(Vec<DesignedAgentPrompt>, PhaseTokenUsage), HubError> {
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

    let upstream_topology = crate::server::services::dispatch::build_upstream_topology(
        dag.state,
        ctx.step.id,
        ctx.step.workflow_id,
    )
    .await;

    // Build enriched board_state with design status (no changed_agents — pipeline path)
    let board_state_xml = match crate::server::hub::board_state::build_snapshot(
        dag.state.repos().workflows.as_ref(),
        None,
        crate::server::hub::board_state::BoardStateVariant::Dispatch,
        ctx.step.workflow_id,
        ctx.step.id,
    )
    .await
    {
        Ok(mut snapshot) => {
            crate::server::hub::board_state::enrich_design_status(
                &mut snapshot,
                dag.state.repos().system_files.as_ref(),
                ctx.step.id,
                ctx.step.workflow_id,
                &[], // no builder changeset in pipeline path
            )
            .await;
            crate::server::hub::board_state::render(
                &snapshot,
                crate::server::hub::board_state::BoardStateVariant::Dispatch,
            )
        }
        Err(e) => {
            warn!(step_id = %ctx.step.id, error = %e, "Failed to build board state for designer");
            String::new()
        }
    };

    // Read previous step's handoff and next step's box text for design context (step 15)
    let (previous_step_handoff, next_step_text) = {
        let edges = dag
            .state
            .repos()
            .workflows
            .list_edges(ctx.step.workflow_id)
            .await
            .unwrap_or_default();
        let parent_ids = crate::server::hub::dag::get_parent_steps(ctx.step.id, &edges);
        let child_ids = crate::server::hub::dag::get_child_steps(ctx.step.id, &edges);

        let mut prev = Vec::new();
        for parent_id in &parent_ids {
            if let Ok(Some(s)) = dag.state.repos().workflows.get_step(*parent_id).await {
                if !s.designer_handoff.is_empty() {
                    prev.push(crate::server::services::dispatch::PreviousStepHandoff {
                        step_name: s.name.unwrap_or_default(),
                        handoff_description: s.designer_handoff,
                    });
                }
            }
        }

        let mut next = Vec::new();
        for child_id in &child_ids {
            if let Ok(Some(s)) = dag.state.repos().workflows.get_step(*child_id).await {
                if !s.description.is_empty() {
                    next.push(crate::server::services::dispatch::NextStepText {
                        step_name: s.name.unwrap_or_default(),
                        description: s.description.clone(),
                    });
                }
            }
        }

        (prev, next)
    };

    let strategy = ReactDesignerStrategy::new(ReactDesignerConfig {
        state: dag.state.clone(),
        step_id: ctx.step.id,
        workflow_id: ctx.step.workflow_id,
        roster: ctx.roster.clone(),
        session_id: Some(designer_session_id),
        agent_execution_id: designer_ae_id,
        board_state_xml,
        step_order: upstream_topology,
        task: ctx.brief.task_description.clone(),
        changed_agents: vec![], // no builder changeset in pipeline path
        previous_step_handoff,
        next_step_text,
        current_design_handoff: ctx.step.designer_handoff.clone(),
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
        .execute(
            &strategy,
            strategy.instruction(),
            &NullSink,
            &recorder,
            dag.cancel,
        )
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

    // Persist step handoff to session for prior_design context on re-runs
    if let Some(handoff) = strategy.take_step_handoff() {
        let _ = dag
            .state
            .repos()
            .sessions
            .insert_session_message(
                UserId(dag.ctx.user_id),
                designer_session_id,
                Uuid::new_v4(),
                "assistant".to_string(),
                handoff,
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
        let raw_slug = file
            .path
            .rsplit('/')
            .next()
            .and_then(|f| f.strip_suffix(".json"))
            .unwrap_or("");
        let slug = agent_designer::normalize_agent_name(raw_slug);

        let roster_entry = match roster_by_slug.get(&slug) {
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

/// Build static fallback prompts when no designer phase runs or when the
/// designer fails. Maps roster agents directly to prompts using role templates.
pub(crate) fn build_static_fallback_prompts(
    brief: &TaskMissionBriefRow,
    roster: &[TaskAgentRosterRow],
    base_prompt: &str,
) -> Vec<DesignedAgentPrompt> {
    let mut base_vars = HashMap::with_capacity(3);
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
                assignment: format!("{}\n\n{}", brief.task_description, entry.role_description),
                expected_output: None,
                execution_order: entry.execution_order,
                receives_from,
            }
        })
        .collect()
}

// ── Private Helpers ─────────────────────────────────────────────────────────

/// Map generic designer results to workforce `DesignedAgentPrompt`s.
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
