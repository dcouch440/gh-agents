//! Agent Designer lifecycle phase for the workforce pipeline.
//!
//! Implements `PipelinePhase` to generate optimized prompts for each
//! roster agent via the Agent Designer LLM call. Falls back to static
//! prompts when the designer fails — never propagates a designer error.

use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use anyhow::anyhow;
use tracing::warn;

use crate::config::protocols::{roles, vars, DESIGNER};
use crate::db::{TaskAgentRosterRow, TaskMissionBriefRow};
use crate::server::hub::error::HubError;
use crate::server::hub::protocols::context::{build_context_block, ContextDocument};
use crate::server::hub::protocols::execution_recorder::{
    PhaseCompletion, ProtocolExecutionRecorder,
};
use crate::server::ws::events::WorkflowEventKind;

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
        let designer_phase = recorder
            .create_phase_with_context("designer", None, None, None, Some("workforce"), None)
            .await?;

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

        let (designed_prompts, token_usage) = match agent_designer::run_agent_designer(
            dag.engine,
            dag.state,
            dag.ctx,
            &ctx.step,
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
                            model: Some(&DESIGNER.agent("designer").model_id),
                        },
                    )
                    .await;

                let prompts = map_designer_results(&result, &ctx.roster)?;

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
                let fallback =
                    build_static_fallback_prompts(&ctx.brief, &ctx.roster, &ctx.base_prompt);
                let usage = PhaseTokenUsage {
                    input_tokens: 0,
                    output_tokens: 0,
                    cost_usd: 0.0,
                    run_id: None,
                };
                (fallback, usage)
            }
        };

        let user_notes_block = build_user_notes_block(&ctx.upstream_context);

        Ok(PhaseOutput {
            designed_prompts,
            user_notes_block,
            token_usage,
        })
    }
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
pub(super) fn build_static_fallback_prompts(
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
        .map(|entry| {
            let mut v = base_vars.clone();
            v.insert(vars::workforce::AGENT_NAME.into(), entry.name.clone());
            v.insert(
                vars::workforce::ROLE_DESCRIPTION.into(),
                entry.role_description.clone(),
            );

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
            task_prompt: entry.task_prompt.clone(),
            execution_order: roster_entry.execution_order,
            receives_from: entry.receives_from.clone(),
        });
    }

    prompts.sort_by_key(|p| p.execution_order);
    Ok(prompts)
}
