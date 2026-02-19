//! Agent Designer pre-lifecycle for the workforce step.
//!
//! Runs the designer to generate optimized prompts for each roster agent,
//! with a static fallback when the designer fails.

use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use anyhow::anyhow;
use tracing::warn;
use uuid::Uuid;

use crate::config::protocols::{roles, vars, AGENT_DESIGNER};
use crate::db::{TaskAgentRosterRow, TaskMissionBriefRow};
use crate::server::hub::error::HubError;
use crate::server::hub::protocols::context::{build_context_block, ContextDocument};
use crate::server::hub::protocols::execution_recorder::{
    PhaseCompletion, ProtocolExecutionRecorder,
};
use crate::server::ws::events::WorkflowEventKind;
use crate::types::StepExecutionEnvelope;

use super::super::agent_designer;
use super::super::designer_input::workforce::build_workforce_designer_input;
use super::super::{broadcast_workflow_event, DagContext};
use super::output::build_team_roster_string;
use super::types::{DesignedAgentPrompt, DesignerTokenUsage};

/// Run the Agent Designer pre-lifecycle and build the user notes block.
///
/// On designer success, maps results to `DesignedAgentPrompt`s. On failure,
/// falls back to static prompts (never propagates a designer error).
/// Returns `(designed_prompts, designer_usage, user_notes_block)`.
pub(super) async fn run_designer_phase(
    dag: &DagContext<'_>,
    step: &crate::db::WorkflowStepRow,
    brief: &TaskMissionBriefRow,
    roster: &[TaskAgentRosterRow],
    completed_envelopes: &HashMap<Uuid, StepExecutionEnvelope>,
    base_prompt: &str,
    upstream_context: &[(String, String)],
) -> Result<(Vec<DesignedAgentPrompt>, DesignerTokenUsage, String), HubError> {
    let recorder =
        ProtocolExecutionRecorder::new(&*dag.state.repos().protocols, step.id, dag.ctx.run_id);
    let designer_phase = recorder
        .create_phase_with_context("designer", None, None, None, Some("workforce"), None)
        .await?;

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
        brief,
        roster,
        completed_envelopes,
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
                        model: Some(&AGENT_DESIGNER.agent("designer").model_id),
                    },
                )
                .await;

            let prompts = map_designer_results(&result, roster)?;

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
            let fallback = build_static_fallback_prompts(brief, roster, base_prompt);
            let usage = DesignerTokenUsage {
                input_tokens: 0,
                output_tokens: 0,
                cost_usd: 0.0,
                run_id: Uuid::nil(),
            };
            (fallback, usage)
        }
    };

    // Build user_notes block from upstream context nodes
    let user_notes_block = if upstream_context.is_empty() {
        String::new()
    } else {
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
    };

    Ok((designed_prompts, designer_usage, user_notes_block))
}

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

/// Build static fallback prompts when the Agent Designer fails.
fn build_static_fallback_prompts(
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
