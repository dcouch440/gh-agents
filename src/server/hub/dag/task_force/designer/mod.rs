//! Agent Designer pre-lifecycle for task force execution.
//!
//! Before crew agents run, a single LLM call reads the mission brief, agent
//! roster, and upstream context, then generates an optimized (system prompt,
//! task prompt, tool assignment) tuple for each agent. This invests one LLM
//! call in prompt quality so crew agents produce better results.

mod tests;

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use anyhow::anyhow;
use serde::Deserialize;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};
use uuid::Uuid;

use crate::config::protocols::{roles, vars, AGENT_DESIGNER};
use crate::db::{TaskAgentRosterRow, TaskMissionBriefRow, WorkflowStepRow};
use crate::server::hub::engine::filters::{FilterContext, ReasoningTraceFilter};
use crate::server::hub::engine::ExecutionEngine;
use crate::server::hub::error::HubError;
use crate::server::hub::protocols::json_utils::parse_structured_output;
use crate::server::hub::recorder::ExecutionRecorder;
use crate::server::hub::strategies::agent_designer::{AgentDesignerConfig, AgentDesignerStrategy};
use crate::server::hub::strategies::compute_cost;
use crate::server::hub::streaming::NullSink;
use crate::server::state::AppState;
use crate::types::{StepExecutionEnvelope, UserId};

use super::super::WorkflowExecutionContext;

// ── Output types ────────────────────────────────────────────────────────────

/// Output from the Agent Designer — one prompt pair + tool assignment per agent.
#[derive(Debug, Clone)]
pub struct DesignedAgentPrompt {
    pub agent_roster_entry_id: Uuid,
    pub agent_name: String,
    pub tools: Vec<String>,
    pub system_prompt: String,
    pub task_prompt: String,
    pub reasoning: String,
    pub execution_order: i32,
}

/// Token usage from the designer call, for accumulating into step totals.
pub struct DesignerTokenUsage {
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cost_usd: f32,
}

// ── JSON deserialization types ──────────────────────────────────────────────

/// Parsed output from the Agent Designer LLM call.
#[derive(Debug, Deserialize)]
struct DesignerOutputSchema {
    agents: Vec<DesignerAgentEntry>,
}

#[derive(Debug, Deserialize)]
struct DesignerAgentEntry {
    agent_id: String,
    agent_name: String,
    tools: Vec<String>,
    system_prompt: String,
    task_prompt: String,
    reasoning: String,
}

// ── Main execution function ─────────────────────────────────────────────────

/// Run the Agent Designer pre-lifecycle.
///
/// Makes a single LLM call that generates (system_prompt, task_prompt, tools)
/// for each agent in the roster. Stores results in DB with token tracking.
/// Returns designed prompts sorted by execution_order, plus token usage.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_agent_designer(
    engine: &ExecutionEngine,
    state: &AppState,
    ctx: &WorkflowExecutionContext,
    step: &WorkflowStepRow,
    brief: &TaskMissionBriefRow,
    roster: &[TaskAgentRosterRow],
    completed_envelopes: &HashMap<Uuid, StepExecutionEnvelope>,
    cancel: Option<&CancellationToken>,
) -> Result<(Vec<DesignedAgentPrompt>, DesignerTokenUsage), HubError> {
    let designer_cfg = AGENT_DESIGNER.agent("designer");

    // 1. Build template variables for the designer's user prompt
    let template_vars = HashMap::from([
        (
            vars::designer::TASK_DESCRIPTION.to_string(),
            brief.task_description.clone(),
        ),
        (
            vars::designer::FAILURE_MODE.to_string(),
            brief.failure_mode.clone(),
        ),
        (
            vars::designer::DOWNSTREAM_CONTEXT.to_string(),
            brief.downstream_context.clone().unwrap_or_default(),
        ),
        (
            vars::designer::AGENT_ROSTER.to_string(),
            format_roster_for_designer(roster),
        ),
        (
            vars::designer::UPSTREAM_CONTEXT.to_string(),
            format_upstream_for_designer(completed_envelopes),
        ),
        (
            vars::designer::CAPABILITY_DESCRIPTIONS.to_string(),
            format_capability_descriptions(&brief.available_capabilities),
        ),
    ]);

    // 2. Resolve the Agent Designer's own prompts
    let protocol_ctx = roles::AGENT_DESIGNER_DESIGNER.resolve(&template_vars);

    // 3. Create a designer run record for token tracking
    let run_row = state
        .repos()
        .workflows
        .create_designer_run(
            ctx.stage_execution_id,
            ctx.stage_execution_id,
            step.id,
            brief.id,
            &designer_cfg.model_id,
        )
        .await
        .map_err(|e| HubError::Internal(anyhow!("failed to create designer run: {}", e)))?;

    info!(
        step_id = %step.id,
        run_id = %run_row.id,
        model = %designer_cfg.model_id,
        agents = roster.len(),
        "Running Agent Designer pre-lifecycle"
    );

    // 4. Build strategy (no tools, single round)
    let strategy = AgentDesignerStrategy::new(AgentDesignerConfig {
        system_prompt: protocol_ctx.system_prompt,
        model_id: designer_cfg.model_id.clone(),
        temperature: designer_cfg.temperature,
        max_rounds: designer_cfg.max_rounds,
        context_budget: designer_cfg.context_budget,
        state: Some(state.clone()),
        user_id: Some(UserId(ctx.user_id)),
    });

    // 5. Execute the designer call with reasoning trace filter
    let mut filter_ctx = FilterContext::new(&designer_cfg.model_id, step.id);
    filter_ctx.has_output_schema = true;
    let filters: Vec<Arc<dyn crate::server::hub::engine::filters::ExecutionFilter>> =
        vec![Arc::new(ReasoningTraceFilter::new())];

    let recorder = ExecutionRecorder::new(&**state.repo(), None, None);
    let result = engine
        .clone_with_provider()
        .with_filters(filters)
        .with_filter_context(filter_ctx)
        .execute(
            &strategy,
            &protocol_ctx.user_prompt,
            &NullSink,
            &recorder,
            cancel,
        )
        .await?;

    // 6. Compute cost and update token tracking
    let cost = compute_cost(
        &designer_cfg.model_id,
        result.input_tokens as i64,
        result.output_tokens as i64,
    );
    let _ = state
        .repos()
        .workflows
        .update_designer_run_tokens(
            run_row.id,
            result.input_tokens as i64,
            result.output_tokens as i64,
            cost,
        )
        .await;

    info!(
        run_id = %run_row.id,
        tokens_in = result.input_tokens,
        tokens_out = result.output_tokens,
        cost_usd = cost,
        "Agent Designer call completed"
    );

    // 7. Parse the designer's output as JSON
    let parsed_json = parse_structured_output(&result.content).ok_or_else(|| {
        HubError::Internal(anyhow!(
            "Agent Designer produced no valid JSON. Raw output: {}",
            truncate_for_context(&result.content, 500)
        ))
    })?;

    let designer_output: DesignerOutputSchema =
        serde_json::from_value(parsed_json).map_err(|e| {
            HubError::Internal(anyhow!(
                "Agent Designer JSON does not match expected schema: {}",
                e
            ))
        })?;

    // 8. Validate and store each generated prompt pair + tool assignment
    let allowed: HashSet<&str> = brief
        .available_capabilities
        .iter()
        .map(|s| s.as_str())
        .collect();

    let mut designed_prompts = Vec::with_capacity(designer_output.agents.len());

    for (idx, entry) in designer_output.agents.iter().enumerate() {
        // Find matching roster entry by agent_id
        let roster_entry = roster
            .iter()
            .find(|r| r.id.to_string() == entry.agent_id)
            .ok_or_else(|| {
                HubError::Internal(anyhow!(
                    "Designer referenced unknown agent_id: {}",
                    entry.agent_id
                ))
            })?;

        // Validate assigned tools come from the allowed pool
        let valid_tools: Vec<String> = entry
            .tools
            .iter()
            .filter(|tool| {
                if allowed.contains(tool.as_str()) {
                    true
                } else {
                    warn!(
                        agent = %entry.agent_name,
                        tool = %tool,
                        "Designer assigned tool not in allowed_capabilities, stripping"
                    );
                    false
                }
            })
            .cloned()
            .collect();

        // Store in DB
        let _ = state
            .repos()
            .workflows
            .create_designer_output(
                run_row.id,
                roster_entry.id,
                &entry.agent_name,
                &valid_tools,
                &entry.system_prompt,
                &entry.task_prompt,
                &entry.reasoning,
                idx as i32,
            )
            .await;

        designed_prompts.push(DesignedAgentPrompt {
            agent_roster_entry_id: roster_entry.id,
            agent_name: entry.agent_name.clone(),
            tools: valid_tools,
            system_prompt: entry.system_prompt.clone(),
            task_prompt: entry.task_prompt.clone(),
            reasoning: entry.reasoning.clone(),
            execution_order: roster_entry.execution_order,
        });
    }

    // Sort by execution_order
    designed_prompts.sort_by_key(|p| p.execution_order);

    let token_usage = DesignerTokenUsage {
        input_tokens: result.input_tokens as i64,
        output_tokens: result.output_tokens as i64,
        cost_usd: cost,
    };

    Ok((designed_prompts, token_usage))
}

// ── Helper formatters ───────────────────────────────────────────────────────

/// Format the agent roster as a readable list for the designer's input.
pub(crate) fn format_roster_for_designer(roster: &[TaskAgentRosterRow]) -> String {
    let mut out = String::new();
    for (idx, agent) in roster.iter().enumerate() {
        out.push_str(&format!(
            "{}. {} (id: {})\n   Role: {}\n   Execution Order: {}\n\n",
            idx + 1,
            agent.name,
            agent.id,
            agent.role_description,
            agent.execution_order,
        ));
    }
    out
}

/// Format upstream step outputs for the designer's context.
pub(crate) fn format_upstream_for_designer(
    envelopes: &HashMap<Uuid, StepExecutionEnvelope>,
) -> String {
    if envelopes.is_empty() {
        return "No upstream outputs available. This is the first step in the workflow."
            .to_string();
    }
    let mut out = String::new();
    for (step_id, env) in envelopes {
        let data_str = env.data.as_ref().map(|d| d.to_string()).unwrap_or_default();
        out.push_str(&format!(
            "<upstream_step id=\"{}\">\n{}\n</upstream_step>\n\n",
            step_id,
            truncate_for_context(&data_str, 4000),
        ));
    }
    out
}

/// Format capability names into descriptions the designer can reference.
pub(crate) fn format_capability_descriptions(capabilities: &[String]) -> String {
    let mut out = String::new();
    for cap in capabilities {
        let desc = match cap.as_str() {
            "file_read" => "file_read: Read file contents from the repository",
            "file_write" => "file_write: Create or modify files in the repository",
            "grep" => "grep: Search file contents with regex patterns",
            "shell" => "shell: Execute shell commands in a sandboxed environment",
            "git" => "git: Run git operations (status, diff, log, commit, branch)",
            "github_api" => "github_api: Interact with GitHub API (issues, PRs, reviews)",
            "web_search" => "web_search: Search the web for information",
            "database_query" => "database_query: Execute read-only SQL queries",
            other => other,
        };
        out.push_str(&format!("- {desc}\n"));
    }
    out
}

/// Truncate long content for context injection.
pub(crate) fn truncate_for_context(content: &str, max_chars: usize) -> &str {
    if content.len() <= max_chars {
        content
    } else {
        // Find a valid char boundary at or before max_chars
        let mut end = max_chars;
        while end > 0 && !content.is_char_boundary(end) {
            end -= 1;
        }
        &content[..end]
    }
}
