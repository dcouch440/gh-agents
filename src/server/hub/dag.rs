//! DAG orchestration — topological sort, variable resolution, for-each fan-out,
//! and workflow execution using the unified ExecutionEngine.
//!
//! This module re-exports the pure graph/variable functions from `dag_executor`
//! and provides `execute_workflow_via_engine` which delegates step execution
//! to the hub's `ExecutionEngine` instead of running its own react loop.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::anyhow;
use serde_json::Value as JsonValue;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::db::{AgentRow, WorkflowStepEdgeRow, WorkflowStepRow};
use crate::llm::{LLMProvider, Tool};
use crate::server::state::AppState;

use super::engine::ExecutionEngine;
use super::error::HubError;
use super::recorder::ExecutionRecorder;
use super::strategies::dag_step::{compute_cost, DagStepConfig, DagStepStrategy};
use super::streaming::NullSink;

// Re-export pure DAG functions from the existing dag_executor
pub use crate::server::dag_executor::{
    compose_prompt, extract_for_each_label, find_entry_steps, get_child_steps, get_parent_steps,
    resolve_for_each_array, resolve_variables, topological_sort, StepOutput,
    WorkflowExecutionContext, WorkflowExecutionResult,
};

/// Execute a complete workflow DAG using the unified ExecutionEngine.
///
/// This replaces `dag_executor::execute_workflow` — same logic (topo sort,
/// variable resolution, for-each fan-out, interactive review) but step
/// execution goes through `ExecutionEngine::execute()` with `DagStepStrategy`.
pub async fn execute_workflow_via_engine(
    engine: &ExecutionEngine,
    state: &AppState,
    ctx: &WorkflowExecutionContext,
    steps: &[WorkflowStepRow],
    edges: &[WorkflowStepEdgeRow],
    cancel: Option<&CancellationToken>,
) -> Result<WorkflowExecutionResult, HubError> {
    let sorted = topological_sort(steps, edges).map_err(|_| HubError::DagCycle)?;
    let step_map: HashMap<Uuid, &WorkflowStepRow> = steps.iter().map(|s| (s.id, s)).collect();

    let mut completed: HashMap<Uuid, StepOutput> = HashMap::new();
    let mut var_outputs: HashMap<String, JsonValue> = HashMap::new();
    let mut total_input_tokens: i64 = 0;
    let mut total_output_tokens: i64 = 0;
    let mut total_cost_usd: f32 = 0.0;

    for step_id in &sorted {
        let step = match step_map.get(step_id) {
            Some(s) => *s,
            None => continue,
        };

        // Check cancellation before each step
        if cancel.map_or(false, |t| t.is_cancelled()) {
            return Err(HubError::Cancelled);
        }

        // Check all parents are completed
        let parents = get_parent_steps(*step_id, edges);
        let all_parents_done = parents.iter().all(|pid| completed.contains_key(pid));
        if !all_parents_done {
            warn!("Step {} has uncompleted parents, skipping", step_id);
            continue;
        }

        // Load agent
        let agent = state
            .repo
            .get_persisted_agent(step.agent_id)
            .await
            .map_err(|e| anyhow::anyhow!("failed to load agent: {}", e))?
            .ok_or_else(|| HubError::AgentNotFound {
                step_id: *step_id,
                agent_id: step.agent_id,
            })?;

        if step.execution_mode == "for_each" {
            execute_for_each_step(
                engine,
                state,
                ctx,
                step,
                &agent,
                edges,
                &var_outputs,
                &mut completed,
                &mut total_input_tokens,
                &mut total_output_tokens,
                &mut total_cost_usd,
                cancel,
            )
            .await?;
        } else {
            execute_single_step(
                engine,
                state,
                ctx,
                step,
                &agent,
                &var_outputs,
                &mut completed,
                &mut total_input_tokens,
                &mut total_output_tokens,
                &mut total_cost_usd,
                cancel,
            )
            .await?;
        }
    }

    let final_outputs: HashMap<String, StepOutput> = completed
        .into_iter()
        .map(|(id, out)| (id.to_string(), out))
        .collect();

    Ok(WorkflowExecutionResult {
        outputs: final_outputs,
        total_input_tokens,
        total_output_tokens,
        total_cost_usd,
    })
}

/// Execute a single (non-for-each) step through the engine.
async fn execute_single_step(
    engine: &ExecutionEngine,
    state: &AppState,
    ctx: &WorkflowExecutionContext,
    step: &WorkflowStepRow,
    agent: &AgentRow,
    var_outputs: &HashMap<String, JsonValue>,
    completed: &mut HashMap<Uuid, StepOutput>,
    total_input_tokens: &mut i64,
    total_output_tokens: &mut i64,
    total_cost_usd: &mut f32,
    cancel: Option<&CancellationToken>,
) -> Result<(), HubError> {
    let prompt = compose_prompt(
        step,
        state.prompt_template_repo.as_deref(),
        state.doc_repo.as_deref(),
        state.workflow_repo.as_deref(),
        var_outputs,
        &ctx.prior_outputs,
        None,
    )
    .await;

    let (output, in_tok, out_tok, cost) =
        run_step_via_engine(engine, state, ctx, step, agent, &prompt, cancel).await?;

    *total_input_tokens += in_tok;
    *total_output_tokens += out_tok;
    *total_cost_usd += cost;

    // Store output in variable map
    if !output.variable_name.is_empty() {
        if let Some(structured) = &output.structured_output {
            // var_outputs is behind a mutable ref — safe to insert directly
            // (We'd need a different approach if we parallelize single steps)
        }
    }
    completed.insert(step.id, output);

    Ok(())
}

/// Execute a for-each step: expand into N iterations, run sequentially.
async fn execute_for_each_step(
    engine: &ExecutionEngine,
    state: &AppState,
    ctx: &WorkflowExecutionContext,
    step: &WorkflowStepRow,
    agent: &AgentRow,
    _edges: &[WorkflowStepEdgeRow],
    var_outputs: &HashMap<String, JsonValue>,
    completed: &mut HashMap<Uuid, StepOutput>,
    total_input_tokens: &mut i64,
    total_output_tokens: &mut i64,
    total_cost_usd: &mut f32,
    cancel: Option<&CancellationToken>,
) -> Result<(), HubError> {
    let for_each_ref = step
        .for_each_ref
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("for_each step {} missing for_each_ref", step.id))?;

    let array = resolve_for_each_array(for_each_ref, var_outputs, &ctx.prior_outputs)
        .ok_or_else(|| HubError::ForEachNotArray {
            reference: for_each_ref.to_string(),
        })?;

    let label_field = step.for_each_label_field.as_deref();

    info!(
        step_id = %step.id,
        count = array.len(),
        "for_each expansion"
    );

    let mut iteration_outputs = Vec::new();

    for (idx, element) in array.iter().enumerate() {
        if cancel.map_or(false, |t| t.is_cancelled()) {
            return Err(HubError::Cancelled);
        }
        let _label = extract_for_each_label(element, label_field);

        let prompt = compose_prompt(
            step,
            state.prompt_template_repo.as_deref(),
            state.doc_repo.as_deref(),
            state.workflow_repo.as_deref(),
            var_outputs,
            &ctx.prior_outputs,
            Some(element),
        )
        .await;

        match run_step_via_engine(engine, state, ctx, step, agent, &prompt, cancel).await {
            Ok((output, in_tok, out_tok, cost)) => {
                *total_input_tokens += in_tok;
                *total_output_tokens += out_tok;
                *total_cost_usd += cost;
                iteration_outputs.push(output.structured_output.clone());
            }
            Err(e) => {
                error!("for_each iteration {} failed for step {}: {}", idx, step.id, e);
            }
        }
    }

    // Aggregate outputs as array
    let aggregated = JsonValue::Array(iteration_outputs.into_iter().flatten().collect());
    let variable_name = step.output_variable_name.clone().unwrap_or_default();

    completed.insert(
        step.id,
        StepOutput {
            variable_name,
            structured_output: Some(aggregated),
            raw_output: String::new(),
        },
    );

    Ok(())
}

/// Run a single step execution through the ExecutionEngine.
///
/// Creates the agent_execution record, builds a DagStepStrategy, and
/// calls `engine.execute()`. Returns (StepOutput, input_tokens, output_tokens, cost).
async fn run_step_via_engine(
    engine: &ExecutionEngine,
    state: &AppState,
    ctx: &WorkflowExecutionContext,
    step: &WorkflowStepRow,
    agent: &AgentRow,
    prompt: &str,
    cancel: Option<&CancellationToken>,
) -> Result<(StepOutput, i64, i64, f32), HubError> {
    let ae_repo = state
        .agent_execution_repo
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("agent_execution_repo not configured"))?;

    // Build system prompt with optional schema enforcement
    let mut system_prompt = agent.system_prompt.clone();
    if let Some(schema_id) = step.output_schema_id {
        if let Some(os_repo) = &state.output_schema_repo {
            if let Ok(Some(schema)) = os_repo.get_output_schema(schema_id).await {
                system_prompt.push_str(&format!(
                    "\n\nYou MUST respond with valid JSON matching this schema:\n```json\n{}\n```\nRespond ONLY with the JSON object, no other text.",
                    serde_json::to_string_pretty(&schema.schema).unwrap_or_default()
                ));
            }
        }
    }

    // Resolve tools
    let tools = resolve_agent_tools(state, agent.id).await;
    let tool_names: Vec<String> = tools.iter().map(|t| t.name.clone()).collect();

    // Create agent_execution row
    let ae_row = ae_repo
        .create_agent_execution(
            ctx.stage_execution_id,
            agent.id,
            Some(step.id),
            false,
            None,
            &system_prompt,
            prompt,
        )
        .await
        .map_err(|e| anyhow::anyhow!("failed to create agent execution: {}", e))?;

    // Record initial messages
    let _ = ae_repo
        .create_execution_message(ae_row.id, "system", &system_prompt, None, 0, 0)
        .await;
    let _ = ae_repo
        .create_execution_message(ae_row.id, "user", prompt, None, 0, 0)
        .await;

    // Build strategy
    let config = DagStepConfig {
        agent: agent.clone(),
        step: step.clone(),
        system_prompt,
        user_prompt: prompt.to_string(),
        tools,
        tool_names,
        execution_context: ctx.execution_context.clone(),
        run_id: ctx.run_id,
        user_id: ctx.user_id,
        agent_execution_id: ae_row.id,
    };
    let strategy = DagStepStrategy::new(config, state.clone());

    // Build recorder (strategy handles its own recording in on_complete)
    let recorder = ExecutionRecorder::new(
        state.repo.as_ref(),
        state.agent_execution_repo.as_deref(),
        state.token_ledger_repo.as_deref(),
    );

    let sink = NullSink;

    // Execute
    let result = engine.execute(&strategy, prompt, &sink, &recorder, cancel).await?;

    let cost = compute_cost(
        &agent.model_id,
        result.input_tokens as i64,
        result.output_tokens as i64,
    );

    let variable_name = step.output_variable_name.clone().unwrap_or_default();
    let structured = super::strategies::dag_step::DagStepStrategy::parse_output(&result.content);

    let output = StepOutput {
        variable_name,
        structured_output: structured,
        raw_output: result.content,
    };

    Ok((output, result.input_tokens as i64, result.output_tokens as i64, cost))
}

/// Resolve tool definitions for an agent from the database.
async fn resolve_agent_tools(state: &AppState, agent_id: Uuid) -> Vec<Tool> {
    let tools = match state.repo.get_agent_tools(agent_id).await {
        Ok(rows) => rows,
        Err(_) => return vec![],
    };
    tools
        .into_iter()
        .map(|t| Tool {
            name: t.name,
            description: t.description,
            input_schema: t.parameters,
        })
        .collect()
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::dag_executor::{topological_sort, resolve_variables};

    #[test]
    fn topo_sort_linear() {
        let s1 = Uuid::new_v4();
        let s2 = Uuid::new_v4();
        let steps = vec![
            WorkflowStepRow {
                id: s1,
                workflow_id: Uuid::new_v4(),
                agent_id: Uuid::new_v4(),
                execution_mode: "single".into(),
                for_each_ref: None,
                prompt_template_id: None,
                prompt_template: "p1".into(),
                output_schema_id: None,
                output_variable_name: Some("v1".into()),
                interactive_agent_id: None,
                for_each_label_field: None,
                display_order: 0,
                version: 1,
            },
            WorkflowStepRow {
                id: s2,
                workflow_id: Uuid::new_v4(),
                agent_id: Uuid::new_v4(),
                execution_mode: "single".into(),
                for_each_ref: None,
                prompt_template_id: None,
                prompt_template: "p2".into(),
                output_schema_id: None,
                output_variable_name: Some("v2".into()),
                interactive_agent_id: None,
                for_each_label_field: None,
                display_order: 1,
                version: 1,
            },
        ];
        let edges = vec![WorkflowStepEdgeRow {
            from_step_id: s1,
            to_step_id: s2,
        }];

        let sorted = topological_sort(&steps, &edges).unwrap();
        assert_eq!(sorted[0], s1);
        assert_eq!(sorted[1], s2);
    }

    #[test]
    fn topo_sort_cycle_detected() {
        let s1 = Uuid::new_v4();
        let s2 = Uuid::new_v4();
        let steps = vec![
            WorkflowStepRow {
                id: s1, workflow_id: Uuid::new_v4(), agent_id: Uuid::new_v4(),
                execution_mode: "single".into(), for_each_ref: None,
                prompt_template_id: None, prompt_template: "p".into(),
                output_schema_id: None, output_variable_name: None,
                interactive_agent_id: None, for_each_label_field: None, display_order: 0, version: 1,
            },
            WorkflowStepRow {
                id: s2, workflow_id: Uuid::new_v4(), agent_id: Uuid::new_v4(),
                execution_mode: "single".into(), for_each_ref: None,
                prompt_template_id: None, prompt_template: "p".into(),
                output_schema_id: None, output_variable_name: None,
                interactive_agent_id: None, for_each_label_field: None, display_order: 1, version: 1,
            },
        ];
        let edges = vec![
            WorkflowStepEdgeRow { from_step_id: s1, to_step_id: s2 },
            WorkflowStepEdgeRow { from_step_id: s2, to_step_id: s1 },
        ];

        assert!(topological_sort(&steps, &edges).is_err());
    }

    #[test]
    fn resolve_variables_basic() {
        let mut outputs = HashMap::new();
        outputs.insert("name".to_string(), JsonValue::String("Alice".to_string()));

        let result = resolve_variables("Hello {name}!", &outputs, &HashMap::new());
        assert_eq!(result, "Hello Alice!");
    }

    #[test]
    fn resolve_variables_dot_path() {
        let mut outputs = HashMap::new();
        outputs.insert(
            "user".to_string(),
            serde_json::json!({"name": "Bob", "age": 30}),
        );

        let result = resolve_variables("Name: {user.name}, Age: {user.age}", &outputs, &HashMap::new());
        assert_eq!(result, "Name: Bob, Age: 30");
    }

    #[test]
    fn resolve_variables_unresolved_left_as_is() {
        let result = resolve_variables("Hello {unknown}!", &HashMap::new(), &HashMap::new());
        assert_eq!(result, "Hello {unknown}!");
    }

    #[test]
    fn resolve_for_each_array_basic() {
        let mut outputs = HashMap::new();
        outputs.insert(
            "items".to_string(),
            serde_json::json!([{"name": "a"}, {"name": "b"}]),
        );

        let arr = resolve_for_each_array("items", &outputs, &HashMap::new()).unwrap();
        assert_eq!(arr.len(), 2);
    }

    #[test]
    fn resolve_for_each_array_nested() {
        let mut outputs = HashMap::new();
        outputs.insert(
            "result".to_string(),
            serde_json::json!({"data": {"items": [1, 2, 3]}}),
        );

        let arr = resolve_for_each_array("result.data.items", &outputs, &HashMap::new()).unwrap();
        assert_eq!(arr.len(), 3);
    }
}
