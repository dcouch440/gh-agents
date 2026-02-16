//! Generalized Agent Designer pre-lifecycle.
//!
//! Before agents run, a single LLM call reads the archetype-agnostic
//! `DesignerInput` and generates optimized (system_prompt, task_prompt,
//! tool assignment) tuples for each agent. This invests one LLM call in
//! prompt quality so agents produce better results.
//!
//! Any archetype (task_force, documenter, room) can call `run_agent_designer()`
//! by building a `DesignerInput` via the formatters in `designer_input`.

mod tests;

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use anyhow::anyhow;
use serde::Deserialize;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};
use uuid::Uuid;

use crate::config::protocols::{roles, vars, AGENT_DESIGNER};
use crate::db::WorkflowStepRow;
use crate::server::hub::engine::filters::{FilterContext, ReasoningTraceFilter};
use crate::server::hub::engine::ExecutionEngine;
use crate::server::hub::error::HubError;
use crate::server::hub::protocols::json_utils::parse_structured_output;
use crate::server::hub::recorder::ExecutionRecorder;
use crate::server::hub::strategies::agent_designer::{AgentDesignerConfig, AgentDesignerStrategy};
use crate::server::hub::strategies::compute_cost;
use crate::server::hub::streaming::NullSink;
use crate::server::state::AppState;
use crate::types::UserId;

use super::designer_input::{AgentDefinition, DesignerInput, ToolDescription, UpstreamContext};
use super::WorkflowExecutionContext;

// ── Output types ────────────────────────────────────────────────────────────

/// Result of a designer run — prompts for all agents plus token usage.
#[derive(Debug, Clone)]
pub struct DesignerResult {
    pub run_id: Uuid,
    pub prompts: Vec<DesignedAgentPrompt>,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cost_usd: f32,
}

/// One designed prompt pair for one agent.
#[derive(Debug, Clone)]
pub struct DesignedAgentPrompt {
    pub agent_id: String,
    pub agent_name: String,
    pub tools: Vec<String>,
    pub system_prompt: String,
    pub task_prompt: String,
    pub reasoning: String,
    pub execution_order: i32,
    pub receives_from: Vec<String>,
}

// ── Name normalization ─────────────────────────────────────────────────────

/// Normalize an agent name for case-insensitive matching across case styles.
/// Strips spaces, underscores, hyphens, and lowercases.
/// "SecurityAuditor", "security_auditor", "Security Auditor" all → "securityauditor"
pub(crate) fn normalize_agent_name(name: &str) -> String {
    name.chars()
        .filter(|c| *c != ' ' && *c != '_' && *c != '-')
        .flat_map(|c| c.to_lowercase())
        .collect()
}

/// Validate receives_from entries against actual agent names using normalized matching.
/// Returns a corrected vec with names mapped to their canonical (original) form.
/// Logs warnings for unresolvable entries.
fn validate_receives_from(
    receives_from: &[String],
    all_agent_names: &[String],
    current_agent: &str,
) -> Vec<String> {
    if receives_from.is_empty() {
        return Vec::new();
    }

    let name_lookup: HashMap<String, &String> = all_agent_names
        .iter()
        .map(|n| (normalize_agent_name(n), n))
        .collect();

    receives_from
        .iter()
        .filter_map(|entry| {
            let normalized = normalize_agent_name(entry);
            if let Some(canonical) = name_lookup.get(&normalized) {
                Some((*canonical).clone())
            } else {
                warn!(
                    agent = %current_agent,
                    receives_from = %entry,
                    "Designer referenced unknown agent in receives_from, stripping"
                );
                None
            }
        })
        .collect()
}

// ── JSON deserialization types ──────────────────────────────────────────────

/// Parsed output from the Agent Designer LLM call.
#[derive(Debug, Deserialize)]
pub(crate) struct DesignerOutputSchema {
    pub agents: Vec<DesignerAgentEntry>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DesignerAgentEntry {
    agent_id: String,
    agent_name: String,
    tools: Vec<String>,
    system_prompt: String,
    task_prompt: String,
    reasoning: String,
    #[serde(default)]
    receives_from: Vec<String>,
}

// ── Main execution function ─────────────────────────────────────────────────

/// Run the Agent Designer pre-lifecycle for any archetype.
///
/// Accepts archetype-agnostic `DesignerInput`, makes a single LLM call,
/// returns generated (system_prompt, task_prompt, tools) for each agent.
/// Stores results in DB with full token tracking.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_agent_designer(
    engine: &ExecutionEngine,
    state: &AppState,
    ctx: &WorkflowExecutionContext,
    step: &WorkflowStepRow,
    input: DesignerInput,
    phase: &str,
    cancel: Option<&CancellationToken>,
    protocol_execution_id: Option<Uuid>,
) -> Result<DesignerResult, HubError> {
    let designer_cfg = AGENT_DESIGNER.agent("designer");

    // 1. Build template variables from the generic input
    let template_vars = HashMap::from([
        (
            vars::designer::ARCHETYPE.to_string(),
            input.archetype.clone(),
        ),
        (
            vars::designer::CONTEXT_DESCRIPTION.to_string(),
            input.context_description.clone(),
        ),
        (
            vars::designer::AGENT_DEFINITIONS.to_string(),
            format_agent_definitions(&input.agents),
        ),
        (
            vars::designer::UPSTREAM_CONTEXT.to_string(),
            format_upstream_context(&input.upstream),
        ),
        (
            vars::designer::AVAILABLE_TOOLS.to_string(),
            format_tool_descriptions(&input.available_tools),
        ),
        (
            vars::designer::ARCHETYPE_GUIDANCE.to_string(),
            input.archetype_guidance.clone(),
        ),
    ]);

    // 2. Resolve the Agent Designer's own prompts
    let protocol_ctx = roles::AGENT_DESIGNER_DESIGNER.resolve(&template_vars);

    // 3. Create designer run record for token tracking
    let run_row = state
        .repos()
        .workflows
        .create_designer_run_generic(
            ctx.stage_execution_id,
            ctx.stage_execution_id,
            step.id,
            &input.archetype,
            phase,
            &designer_cfg.model_id,
        )
        .await
        .map_err(|e| HubError::Internal(anyhow!("failed to create designer run: {}", e)))?;

    info!(
        step_id = %step.id,
        run_id = %run_row.id,
        archetype = %input.archetype,
        phase = %phase,
        model = %designer_cfg.model_id,
        agents = input.agents.len(),
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
            truncate_for_log(&result.content, 500)
        ))
    })?;

    let designer_output: DesignerOutputSchema =
        serde_json::from_value(parsed_json).map_err(|e| {
            HubError::Internal(anyhow!(
                "Agent Designer JSON does not match expected schema: {}",
                e
            ))
        })?;

    // 8. Build allowed capabilities from all agents in the input
    let allowed: HashSet<&str> = input
        .agents
        .iter()
        .flat_map(|a| a.capabilities.iter().map(|s| s.as_str()))
        .collect();

    // 8b. Collect all agent names for receives_from validation
    let all_agent_names: Vec<String> = designer_output
        .agents
        .iter()
        .map(|a| a.agent_name.clone())
        .collect();

    let mut designed_prompts = Vec::with_capacity(designer_output.agents.len());

    for (idx, entry) in designer_output.agents.iter().enumerate() {
        // Validate assigned tools against the allowed capabilities
        let valid_tools: Vec<String> = entry
            .tools
            .iter()
            .filter(|tool| {
                if allowed.is_empty() || allowed.contains(tool.as_str()) {
                    true
                } else {
                    warn!(
                        agent = %entry.agent_name,
                        tool = %tool,
                        "Designer assigned tool not in allowed capabilities, stripping"
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
            .create_designer_output_generic(
                run_row.id,
                &entry.agent_id,
                &input.archetype,
                &entry.agent_name,
                &valid_tools,
                &entry.system_prompt,
                &entry.task_prompt,
                &entry.reasoning,
                idx as i32,
                protocol_execution_id,
            )
            .await;

        // Validate receives_from against actual agent names
        let valid_receives_from =
            validate_receives_from(&entry.receives_from, &all_agent_names, &entry.agent_name);

        designed_prompts.push(DesignedAgentPrompt {
            agent_id: entry.agent_id.clone(),
            agent_name: entry.agent_name.clone(),
            tools: valid_tools,
            system_prompt: entry.system_prompt.clone(),
            task_prompt: entry.task_prompt.clone(),
            reasoning: entry.reasoning.clone(),
            execution_order: idx as i32,
            receives_from: valid_receives_from,
        });
    }

    Ok(DesignerResult {
        run_id: run_row.id,
        prompts: designed_prompts,
        input_tokens: result.input_tokens as i64,
        output_tokens: result.output_tokens as i64,
        cost_usd: cost,
    })
}

// ── Formatting helpers ──────────────────────────────────────────────────────

/// Format agent definitions as a numbered list for the designer's input.
pub(crate) fn format_agent_definitions(agents: &[AgentDefinition]) -> String {
    let mut out = String::new();
    for (idx, agent) in agents.iter().enumerate() {
        out.push_str(&format!(
            "{}. {} (id: {})\n   Role: {}\n   Execution Order: {}\n",
            idx + 1,
            agent.name,
            agent.id,
            agent.role,
            agent.execution_order,
        ));

        if !agent.capabilities.is_empty() {
            out.push_str(&format!(
                "   Capabilities: {}\n",
                agent.capabilities.join(", ")
            ));
        }

        if !agent.additional_context.is_empty() {
            out.push_str(&format!(
                "   Additional context:\n   {}\n",
                agent.additional_context.replace('\n', "\n   ")
            ));
        }

        out.push('\n');
    }
    out
}

/// Format upstream context as XML blocks for the designer.
pub(crate) fn format_upstream_context(upstream: &[UpstreamContext]) -> String {
    let mut out = String::new();
    for ctx in upstream {
        out.push_str(&format!(
            "<upstream source=\"{}\" type=\"{}\">\n{}\n</upstream>\n\n",
            ctx.source_name, ctx.source_type, ctx.content,
        ));
    }
    out
}

/// Format tool descriptions as a bulleted list for the designer.
pub(crate) fn format_tool_descriptions(tools: &[ToolDescription]) -> String {
    if tools.is_empty() {
        return "No tools available for this execution.".to_string();
    }
    let mut out = String::new();
    for tool in tools {
        out.push_str(&format!("- {}: {}\n", tool.name, tool.description));
    }
    out
}

/// Truncate long content for log messages.
fn truncate_for_log(content: &str, max_chars: usize) -> &str {
    if content.len() <= max_chars {
        content
    } else {
        let mut end = max_chars;
        while end > 0 && !content.is_char_boundary(end) {
            end -= 1;
        }
        &content[..end]
    }
}
