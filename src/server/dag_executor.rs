//! Workflow DAG executor for pipeline stage execution.
//!
//! Given a workflow (steps + edges), this module:
//! 1. Topologically sorts the steps
//! 2. Runs entry nodes (no incoming edges) first
//! 3. Resolves `{variable}` references in prompts from prior step outputs
//! 4. Expands `for_each` steps into N parallel executions
//! 5. Handles interactive agents (two agent_execution rows per step)
//! 6. Writes all results to agent_executions, execution_messages, and token_ledger

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use anyhow::{anyhow, Result};
use chrono::Utc;
use serde_json::Value as JsonValue;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::db::traits::{AgentExecutionRepo, DocumentRepo, TokenLedgerRepo, WorkflowRepo};
use crate::db::{AgentRow, WorkflowStepEdgeRow, WorkflowStepRow};
use crate::llm::{ContentBlock, LLMProvider, LLMRequest, Message, StopReason, Tool};

use super::state::AppState;
use super::ws::PipelineUpdate;

/// Maximum tool use rounds per step execution.
const DAG_MAX_TOOL_ROUNDS: u32 = 15;

/// Completed step output, keyed by output_variable_name.
#[derive(Debug, Clone)]
pub struct StepOutput {
    pub variable_name: String,
    pub structured_output: Option<JsonValue>,
    pub raw_output: String,
}

/// Context passed into the DAG executor for one workflow run.
pub struct WorkflowExecutionContext {
    pub stage_execution_id: Uuid,
    pub run_id: Uuid,
    pub user_id: Uuid,
    pub initial_input: String,
    /// Outputs from prior pipeline stages, keyed by variable name.
    pub prior_outputs: HashMap<String, JsonValue>,
    /// Execution context for tool calls (file ops, git, etc.). None if tools are not available.
    pub execution_context: Option<crate::execution::ExecutionContext>,
}

/// Result of executing one workflow.
pub struct WorkflowExecutionResult {
    pub outputs: HashMap<String, StepOutput>,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub total_cost_usd: f32,
}

// ============================================================================
// Topological Sort
// ============================================================================

/// Returns step IDs in topological order. Errors if cycles are detected.
pub fn topological_sort(steps: &[WorkflowStepRow], edges: &[WorkflowStepEdgeRow]) -> Result<Vec<Uuid>> {
    let step_ids: HashSet<Uuid> = steps.iter().map(|s| s.id).collect();
    let mut in_degree: HashMap<Uuid, usize> = step_ids.iter().map(|id| (*id, 0)).collect();
    let mut adjacency: HashMap<Uuid, Vec<Uuid>> = step_ids.iter().map(|id| (*id, vec![])).collect();

    for edge in edges {
        if step_ids.contains(&edge.from_step_id) && step_ids.contains(&edge.to_step_id) {
            adjacency.entry(edge.from_step_id).or_default().push(edge.to_step_id);
            *in_degree.entry(edge.to_step_id).or_default() += 1;
        }
    }

    let mut queue: Vec<Uuid> = in_degree.iter().filter(|(_, &deg)| deg == 0).map(|(id, _)| *id).collect();
    // Sort entry nodes by display_order for deterministic ordering
    let step_order: HashMap<Uuid, i32> = steps.iter().map(|s| (s.id, s.display_order)).collect();
    queue.sort_by_key(|id| step_order.get(id).copied().unwrap_or(0));

    let mut sorted = Vec::new();
    while let Some(node) = queue.pop() {
        sorted.push(node);
        if let Some(children) = adjacency.get(&node) {
            for child in children {
                if let Some(deg) = in_degree.get_mut(child) {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push(*child);
                        queue.sort_by_key(|id| step_order.get(id).copied().unwrap_or(0));
                    }
                }
            }
        }
    }

    if sorted.len() != step_ids.len() {
        return Err(anyhow!("Cycle detected in workflow DAG"));
    }

    Ok(sorted)
}

/// Returns step IDs that have no incoming edges (entry points).
pub fn find_entry_steps(steps: &[WorkflowStepRow], edges: &[WorkflowStepEdgeRow]) -> Vec<Uuid> {
    let has_incoming: HashSet<Uuid> = edges.iter().map(|e| e.to_step_id).collect();
    steps.iter().filter(|s| !has_incoming.contains(&s.id)).map(|s| s.id).collect()
}

/// Returns step IDs that a given step depends on (parents).
pub fn get_parent_steps(step_id: Uuid, edges: &[WorkflowStepEdgeRow]) -> Vec<Uuid> {
    edges.iter().filter(|e| e.to_step_id == step_id).map(|e| e.from_step_id).collect()
}

/// Returns step IDs that depend on a given step (children).
pub fn get_child_steps(step_id: Uuid, edges: &[WorkflowStepEdgeRow]) -> Vec<Uuid> {
    edges.iter().filter(|e| e.from_step_id == step_id).map(|e| e.to_step_id).collect()
}

// ============================================================================
// Variable Resolution
// ============================================================================

/// Resolve `{variable}` references in a prompt template.
///
/// Supports dot-path access: `{features.content.0.name}`.
/// Scope: completed step outputs (from this workflow) + prior stage outputs.
pub fn resolve_variables(template: &str, outputs: &HashMap<String, JsonValue>, prior_outputs: &HashMap<String, JsonValue>) -> String {
    let mut result = String::new();
    let mut chars = template.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '{' {
            // Collect the variable path
            let mut path = String::new();
            let mut depth = 1;
            for inner in chars.by_ref() {
                if inner == '{' {
                    depth += 1;
                    path.push(inner);
                } else if inner == '}' {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                    path.push(inner);
                } else {
                    path.push(inner);
                }
            }
            // Resolve the path
            let resolved = resolve_path(&path, outputs, prior_outputs);
            result.push_str(&resolved);
        } else {
            result.push(ch);
        }
    }

    result
}

/// Navigate a dot-path into the combined output map.
fn resolve_path(path: &str, outputs: &HashMap<String, JsonValue>, prior_outputs: &HashMap<String, JsonValue>) -> String {
    let parts: Vec<&str> = path.split('.').collect();
    if parts.is_empty() {
        return format!("{{{}}}", path);
    }

    let var_name = parts[0];

    // Look in workflow outputs first, then prior stage outputs
    let root = outputs.get(var_name).or_else(|| prior_outputs.get(var_name));

    match root {
        Some(value) => {
            let mut current = value.clone();
            for &part in &parts[1..] {
                current = if let Ok(idx) = part.parse::<usize>() {
                    current.get(idx).cloned().unwrap_or(JsonValue::Null)
                } else {
                    current.get(part).cloned().unwrap_or(JsonValue::Null)
                };
            }
            match &current {
                JsonValue::String(s) => s.clone(),
                JsonValue::Null => format!("{{{}}}", path),
                other => other.to_string(),
            }
        }
        None => format!("{{{}}}", path), // Unresolved, leave as-is
    }
}

/// For a for_each step, resolve the array to iterate over.
pub fn resolve_for_each_array(for_each_ref: &str, outputs: &HashMap<String, JsonValue>, prior_outputs: &HashMap<String, JsonValue>) -> Option<Vec<JsonValue>> {
    let parts: Vec<&str> = for_each_ref.split('.').collect();
    if parts.is_empty() {
        return None;
    }

    let var_name = parts[0];
    let root = outputs.get(var_name).or_else(|| prior_outputs.get(var_name))?;

    let mut current = root.clone();
    for &part in &parts[1..] {
        current = if let Ok(idx) = part.parse::<usize>() {
            current.get(idx).cloned().unwrap_or(JsonValue::Null)
        } else {
            current.get(part).cloned().unwrap_or(JsonValue::Null)
        };
    }

    current.as_array().cloned()
}

/// Extract the for_each label from an element using the label field.
pub fn extract_for_each_label(element: &JsonValue, label_field: Option<&str>) -> Option<String> {
    let field = label_field?;
    element.get(field).and_then(|v| v.as_str()).map(|s| s.to_string())
}

// ============================================================================
// Prompt Composition
// ============================================================================

/// Build the full prompt for a step execution.
///
/// Resolves the prompt template, appends attached document content.
pub async fn compose_prompt(
    step: &WorkflowStepRow,
    prompt_template_repo: Option<&dyn crate::db::traits::PromptTemplateRepo>,
    doc_repo: Option<&dyn DocumentRepo>,
    workflow_repo: Option<&dyn WorkflowRepo>,
    outputs: &HashMap<String, JsonValue>,
    prior_outputs: &HashMap<String, JsonValue>,
    for_each_element: Option<&JsonValue>,
) -> String {
    // Get prompt text: prefer saved template, fall back to inline
    let raw_prompt = if let Some(pt_id) = step.prompt_template_id {
        if let Some(repo) = prompt_template_repo {
            repo.get_prompt_template(pt_id)
                .await
                .ok()
                .flatten()
                .map(|pt| pt.content)
                .unwrap_or_else(|| step.prompt_template.clone())
        } else {
            step.prompt_template.clone()
        }
    } else {
        step.prompt_template.clone()
    };

    // For for_each steps, replace `$` with the current element in variable refs
    let prompt = if let Some(element) = for_each_element {
        // In for_each mode, `{var.content.$.name}` means current element's .name
        // We resolve $ by injecting the element into a special "__for_each_element" key
        let mut augmented_outputs = outputs.clone();
        // Replace $ references by pre-resolving them
        resolve_for_each_prompt(&raw_prompt, element, &augmented_outputs, prior_outputs)
    } else {
        resolve_variables(&raw_prompt, outputs, prior_outputs)
    };

    // Append attached documents
    let mut full_prompt = prompt;
    if let Some(wf_repo) = workflow_repo {
        if let Ok(step_docs) = wf_repo.list_step_documents(step.id).await {
            if let Some(d_repo) = doc_repo {
                for sd in &step_docs {
                    if let Ok(Some(doc)) = d_repo.get_document(sd.document_id).await {
                        full_prompt.push_str(&format!("\n\n---\n## {}\n{}", doc.title, doc.content));
                    }
                }
            }
        }
    }

    full_prompt
}

/// Resolve a for_each prompt where `$` represents the current element.
fn resolve_for_each_prompt(template: &str, element: &JsonValue, outputs: &HashMap<String, JsonValue>, prior_outputs: &HashMap<String, JsonValue>) -> String {
    let mut result = String::new();
    let mut chars = template.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '{' {
            let mut path = String::new();
            let mut depth = 1;
            for inner in chars.by_ref() {
                if inner == '{' {
                    depth += 1;
                    path.push(inner);
                } else if inner == '}' {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                    path.push(inner);
                } else {
                    path.push(inner);
                }
            }

            // Check if path contains `$` for for_each element access
            if path.contains(".$") {
                let resolved = resolve_for_each_path(&path, element, outputs, prior_outputs);
                result.push_str(&resolved);
            } else {
                let resolved = resolve_path(&path, outputs, prior_outputs);
                result.push_str(&resolved);
            }
        } else {
            result.push(ch);
        }
    }

    result
}

/// Resolve a path containing `$` — e.g. `features.content.$.name`
/// The `$` is replaced with the current for_each element.
fn resolve_for_each_path(path: &str, element: &JsonValue, outputs: &HashMap<String, JsonValue>, prior_outputs: &HashMap<String, JsonValue>) -> String {
    let parts: Vec<&str> = path.split('.').collect();

    // Find the position of `$`
    let dollar_pos = parts.iter().position(|&p| p == "$");
    let Some(dollar_pos) = dollar_pos else {
        return resolve_path(path, outputs, prior_outputs);
    };

    // Navigate from element using parts after `$`
    let mut current = element.clone();
    for &part in &parts[dollar_pos + 1..] {
        current = if let Ok(idx) = part.parse::<usize>() {
            current.get(idx).cloned().unwrap_or(JsonValue::Null)
        } else {
            current.get(part).cloned().unwrap_or(JsonValue::Null)
        };
    }

    match &current {
        JsonValue::String(s) => s.clone(),
        JsonValue::Null => format!("{{{}}}", path),
        other => other.to_string(),
    }
}

// ============================================================================
// Step Execution
// ============================================================================

/// Resolve the LLM tool definitions for an agent from the database.
///
/// Returns an empty vec if the agent has no tools assigned — in that case
/// the LLM will never return `StopReason::ToolUse` and the loop executes once.
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

/// Execute a single workflow step: make LLM call(s), record results.
///
/// If the agent has tools assigned, runs a react loop (up to `DAG_MAX_TOOL_ROUNDS`).
/// Otherwise behaves as a single LLM call.
///
/// Returns (agent_execution_id, StepOutput, input_tokens, output_tokens, cost_usd).
async fn execute_step(
    state: &AppState,
    provider: &dyn LLMProvider,
    ctx: &WorkflowExecutionContext,
    step: &WorkflowStepRow,
    agent: &AgentRow,
    prompt: &str,
    _for_each_index: Option<i32>,
    _for_each_label: Option<String>,
) -> Result<(Uuid, StepOutput, i64, i64, f32)> {
    let ae_repo = state.agent_execution_repo.as_ref().ok_or_else(|| anyhow!("agent_execution_repo not configured"))?;

    // Build system prompt with output schema instructions
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

    // Resolve tools for this agent (empty vec = no tools = single call)
    let tools = resolve_agent_tools(state, agent.id).await;
    let tool_names: Vec<String> = tools.iter().map(|t| t.name.clone()).collect();

    // Create agent_execution row
    let ae_row = ae_repo
        .create_agent_execution(ctx.stage_execution_id, agent.id, Some(step.id), false, None, &system_prompt, prompt, None, None, None)
        .await?;

    // Record system + user messages
    let _ = ae_repo.create_execution_message(ae_row.id, "system", &system_prompt, None, 0, 0).await;
    let _ = ae_repo.create_execution_message(ae_row.id, "user", prompt, None, 0, 0).await;

    // Broadcast running status
    broadcast_agent_execution_update(state, ctx.run_id, &ae_row.id, step.id, &agent.name, false, "running", None, 0, 0, 0.0);

    // React loop
    let mut messages = vec![Message::user(prompt)];
    let mut total_input_tokens: i64 = 0;
    let mut total_output_tokens: i64 = 0;
    let mut total_cost_usd: f32 = 0.0;
    let mut final_content = String::new();

    for round in 0..DAG_MAX_TOOL_ROUNDS {
        let request = LLMRequest {
            model: agent.model_id.clone(),
            system: Some(system_prompt.clone()),
            messages: messages.clone(),
            max_tokens: agent.model_max_tokens as u32,
            temperature: agent.model_temperature,
            stream: false,
            tools: tools.clone(),
        };

        let response = provider.send_message(request).await.map_err(|e| anyhow!("LLM call failed (round {}): {}", round, e))?;

        let in_tok = response.usage.input_tokens as i64;
        let out_tok = response.usage.output_tokens as i64;
        let cost = compute_cost(&agent.model_id, in_tok, out_tok);
        total_input_tokens += in_tok;
        total_output_tokens += out_tok;
        total_cost_usd += cost;

        // Write token_ledger for every LLM call
        if let Some(tl_repo) = &state.token_ledger_repo {
            let _ = tl_repo.insert_ledger_entry(ctx.user_id, Some(ae_row.id), &agent.model_id, in_tok, out_tok, cost).await;
        }

        if response.stop_reason == StopReason::ToolUse {
            // Record the assistant message with tool calls
            let _ = ae_repo.create_execution_message(ae_row.id, "assistant", &response.content, None, in_tok, out_tok).await;

            // Add assistant response (with content blocks) to conversation
            messages.push(Message::assistant_with_blocks(response.content_blocks.clone()));

            // Execute each tool call and collect results
            let mut tool_results = Vec::new();
            for block in &response.content_blocks {
                if let ContentBlock::ToolUse { id, name, input } = block {
                    info!(agent = %agent.name, round = round, tool = %name, "DAG step tool call");

                    let result = match &ctx.execution_context {
                        Some(exec_ctx) => crate::agents::execution_tools::execute_execution_tool(name, input, exec_ctx, Some(&tool_names)).await,
                        None => {
                            serde_json::json!({ "error": "No execution context available for tool calls" })
                        }
                    };

                    let result_str = serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string());

                    // Record tool call and result as execution_messages
                    let call_content = serde_json::json!({ "tool": name, "input": input }).to_string();
                    let _ = ae_repo.create_execution_message(ae_row.id, "assistant", &call_content, Some(id.clone()), 0, 0).await;
                    let _ = ae_repo.create_execution_message(ae_row.id, "tool", &result_str, Some(id.clone()), 0, 0).await;

                    tool_results.push(ContentBlock::ToolResult {
                        tool_use_id: id.clone(),
                        content: result_str,
                    });
                }
            }

            messages.push(Message::tool_results(tool_results));
            continue;
        }

        // EndTurn or MaxTokens — final answer
        final_content = response.content;

        // Record final assistant message
        let _ = ae_repo.create_execution_message(ae_row.id, "assistant", &final_content, None, in_tok, out_tok).await;
        break;
    }

    // Parse structured output from final response
    let structured_output = parse_structured_output(&final_content);

    // Update agent_execution with results
    let _ = ae_repo
        .update_agent_execution_status(
            ae_row.id,
            "completed",
            Some(final_content.clone()),
            structured_output.clone(),
        )
        .await;

    // Broadcast completed status
    broadcast_agent_execution_update(
        state,
        ctx.run_id,
        &ae_row.id,
        step.id,
        &agent.name,
        false,
        "completed",
        structured_output.as_ref(),
        total_input_tokens,
        total_output_tokens,
        total_cost_usd,
    );

    let variable_name = step.output_variable_name.clone().unwrap_or_default();
    let step_output = StepOutput {
        variable_name: variable_name.clone(),
        structured_output: structured_output.clone(),
        raw_output: final_content,
    };

    Ok((ae_row.id, step_output, total_input_tokens, total_output_tokens, total_cost_usd))
}

/// Execute the interactive review agent for a step.
async fn execute_interactive_review(
    state: &AppState,
    provider: &dyn LLMProvider,
    ctx: &WorkflowExecutionContext,
    step: &WorkflowStepRow,
    interactive_agent: &AgentRow,
    parent_ae_id: Uuid,
    main_output: &str,
) -> Result<Option<JsonValue>> {
    let ae_repo = state.agent_execution_repo.as_ref().ok_or_else(|| anyhow!("agent_execution_repo not configured"))?;

    let system_prompt = interactive_agent.system_prompt.clone();
    let review_prompt = format!("Review the following output and provide feedback:\n\n{}", main_output);

    // Create interactive agent_execution
    let iae_row = ae_repo
        .create_agent_execution(ctx.stage_execution_id, interactive_agent.id, Some(step.id), true, Some(parent_ae_id), &system_prompt, &review_prompt, None, None, None)
        .await?;

    // Record messages
    let _ = ae_repo.create_execution_message(iae_row.id, "system", &system_prompt, None, 0, 0).await;
    let _ = ae_repo.create_execution_message(iae_row.id, "user", &review_prompt, None, 0, 0).await;

    // Make LLM call for initial review
    let request = LLMRequest {
        model: interactive_agent.model_id.clone(),
        system: Some(system_prompt),
        messages: vec![Message::user(&review_prompt)],
        max_tokens: interactive_agent.model_max_tokens as u32,
        temperature: interactive_agent.model_temperature,
        stream: false,
        ..Default::default()
    };

    let response = provider.send_message(request).await.map_err(|e| anyhow!("Interactive LLM call failed: {}", e))?;

    let input_tokens = response.usage.input_tokens as i64;
    let output_tokens = response.usage.output_tokens as i64;
    let cost_usd = compute_cost(&interactive_agent.model_id, input_tokens, output_tokens);

    // Record assistant response
    let _ = ae_repo.create_execution_message(iae_row.id, "assistant", &response.content, None, input_tokens, output_tokens).await;

    // Write token_ledger for the review call
    if let Some(tl_repo) = &state.token_ledger_repo {
        let _ = tl_repo
            .insert_ledger_entry(ctx.user_id, Some(iae_row.id), &interactive_agent.model_id, input_tokens, output_tokens, cost_usd)
            .await;
    }

    // Set status to awaiting_user — the user will chat and approve via the API
    let _ = ae_repo
        .update_agent_execution_status(iae_row.id, "awaiting_user", Some(response.content.clone()), None)
        .await;

    // Broadcast awaiting_user
    broadcast_agent_execution_update(
        state,
        ctx.run_id,
        &iae_row.id,
        step.id,
        &interactive_agent.name,
        true,
        "awaiting_user",
        None,
        input_tokens,
        output_tokens,
        cost_usd,
    );

    // Return None — the interactive output will be resolved when the user approves
    // The caller should wait for approval via the API (POST /agent-executions/:id/approve)
    Ok(None)
}

// ============================================================================
// DAG Executor
// ============================================================================

/// Execute a complete workflow DAG.
///
/// This is the main entry point. Given a workflow's steps and edges, it:
/// 1. Topologically sorts the steps
/// 2. Executes entry nodes
/// 3. Propagates outputs to dependent steps
/// 4. Expands for_each steps into parallel executions
/// 5. Handles interactive agents
pub async fn execute_workflow(
    state: &AppState,
    provider: Arc<dyn LLMProvider>,
    ctx: WorkflowExecutionContext,
    steps: Vec<WorkflowStepRow>,
    edges: Vec<WorkflowStepEdgeRow>,
) -> Result<WorkflowExecutionResult> {
    let sorted = topological_sort(&steps, &edges)?;
    let step_map: HashMap<Uuid, &WorkflowStepRow> = steps.iter().map(|s| (s.id, s)).collect();

    // Track completed step outputs: step_id → StepOutput
    let completed: Arc<RwLock<HashMap<Uuid, StepOutput>>> = Arc::new(RwLock::new(HashMap::new()));
    // Track outputs by variable name for resolution
    let var_outputs: Arc<RwLock<HashMap<String, JsonValue>>> = Arc::new(RwLock::new(HashMap::new()));

    let mut total_input_tokens: i64 = 0;
    let mut total_output_tokens: i64 = 0;
    let mut total_cost_usd: f32 = 0.0;

    for step_id in &sorted {
        let step = match step_map.get(step_id) {
            Some(s) => *s,
            None => continue,
        };

        // Check all parents are completed
        let parents = get_parent_steps(*step_id, &edges);
        {
            let completed_guard = completed.read().await;
            let all_parents_done = parents.iter().all(|pid| completed_guard.contains_key(pid));
            if !all_parents_done {
                warn!("Step {} has uncompleted parents, skipping", step_id);
                continue;
            }
        }

        // Load agent
        let agent = state.repo.get_persisted_agent(step.agent_id).await?.ok_or_else(|| anyhow!("Agent {} not found", step.agent_id))?;

        // Build current outputs snapshot for variable resolution
        let current_outputs = {
            let guard = var_outputs.read().await;
            guard.clone()
        };

        if step.execution_mode == "for_each" {
            // Expand for_each step into N parallel executions
            let for_each_ref = step.for_each_ref.as_deref().ok_or_else(|| anyhow!("for_each step {} missing for_each_ref", step.id))?;
            let array = resolve_for_each_array(for_each_ref, &current_outputs, &ctx.prior_outputs).ok_or_else(|| anyhow!("for_each_ref '{}' did not resolve to an array", for_each_ref))?;

            let label_field = step.for_each_label_field.as_deref();

            // Broadcast for_each_spawned event
            broadcast_for_each_spawned(state, ctx.run_id, ctx.stage_execution_id, step.id, &agent.name, array.len());

            // Execute all iterations (could parallelize, but sequential for now to avoid overwhelming the LLM)
            let mut iteration_outputs = Vec::new();
            for (idx, element) in array.iter().enumerate() {
                let label = extract_for_each_label(element, label_field);

                let prompt = compose_prompt(
                    step,
                    state.prompt_template_repo.as_deref(),
                    state.doc_repo.as_deref(),
                    state.workflow_repo.as_deref(),
                    &current_outputs,
                    &ctx.prior_outputs,
                    Some(element),
                )
                .await;

                match execute_step(state, provider.as_ref(), &ctx, step, &agent, &prompt, Some(idx as i32), label).await {
                    Ok((ae_id, output, in_tok, out_tok, cost)) => {
                        total_input_tokens += in_tok;
                        total_output_tokens += out_tok;
                        total_cost_usd += cost;

                        // Handle interactive review for each iteration
                        if let Some(interactive_agent_id) = step.interactive_agent_id {
                            if let Ok(Some(ia)) = state.repo.get_persisted_agent(interactive_agent_id).await {
                                let _ = execute_interactive_review(state, provider.as_ref(), &ctx, step, &ia, ae_id, &output.raw_output).await;
                                // Note: interactive steps pause here. In a full implementation,
                                // we'd wait for user approval before continuing.
                                // For now, we continue with the main output.
                            }
                        }

                        iteration_outputs.push(output.structured_output.clone());
                    }
                    Err(e) => {
                        error!("for_each iteration {} failed for step {}: {}", idx, step.id, e);
                    }
                }
            }

            // Aggregate for_each outputs as an array under the variable name
            let aggregated = JsonValue::Array(iteration_outputs.into_iter().flatten().collect());
            let variable_name = step.output_variable_name.clone().unwrap_or_default();

            {
                let mut guard = var_outputs.write().await;
                guard.insert(variable_name.clone(), aggregated.clone());
            }
            {
                let mut guard = completed.write().await;
                guard.insert(
                    *step_id,
                    StepOutput {
                        variable_name,
                        structured_output: Some(aggregated),
                        raw_output: String::new(),
                    },
                );
            }
        } else {
            // Single execution
            let prompt = compose_prompt(
                step,
                state.prompt_template_repo.as_deref(),
                state.doc_repo.as_deref(),
                state.workflow_repo.as_deref(),
                &current_outputs,
                &ctx.prior_outputs,
                None,
            )
            .await;

            let (ae_id, output, in_tok, out_tok, cost) = execute_step(state, provider.as_ref(), &ctx, step, &agent, &prompt, None, None).await?;

            total_input_tokens += in_tok;
            total_output_tokens += out_tok;
            total_cost_usd += cost;

            // Handle interactive review
            if let Some(interactive_agent_id) = step.interactive_agent_id {
                if let Ok(Some(ia)) = state.repo.get_persisted_agent(interactive_agent_id).await {
                    let _ = execute_interactive_review(state, provider.as_ref(), &ctx, step, &ia, ae_id, &output.raw_output).await;
                    // Note: interactive steps pause here. Full implementation would
                    // wait for POST /agent-executions/:id/approve before continuing.
                }
            }

            // Store output
            if !output.variable_name.is_empty() {
                if let Some(structured) = &output.structured_output {
                    let mut guard = var_outputs.write().await;
                    guard.insert(output.variable_name.clone(), structured.clone());
                }
            }
            {
                let mut guard = completed.write().await;
                guard.insert(*step_id, output);
            }
        }
    }

    let completed_guard = completed.read().await;
    let final_outputs: HashMap<String, StepOutput> = completed_guard.iter().map(|(id, out)| (id.to_string(), out.clone())).collect();
    Ok(WorkflowExecutionResult {
        outputs: final_outputs,
        total_input_tokens,
        total_output_tokens,
        total_cost_usd,
    })
}

// ============================================================================
// Pipeline Stage Executor (uses stage members + workflows)
// ============================================================================

/// Execute a pipeline stage using the new pipeline_stage_members → workflows model.
///
/// For each member in the stage, loads the workflow and executes its DAG.
/// All members in a stage run in parallel (via tokio::spawn).
pub async fn execute_stage_via_members(
    state: &AppState,
    provider: Arc<dyn LLMProvider>,
    run_id: Uuid,
    user_id: Uuid,
    pipeline_id: Uuid,
    stage_number: i32,
    initial_input: &str,
    prior_outputs: &HashMap<String, JsonValue>,
) -> Result<Vec<WorkflowExecutionResult>> {
    let member_repo = state.stage_member_repo.as_ref().ok_or_else(|| anyhow!("stage_member_repo not configured"))?;
    let workflow_repo = state.workflow_repo.as_ref().ok_or_else(|| anyhow!("workflow_repo not configured"))?;

    let members = member_repo.list_stage_members(pipeline_id, stage_number).await?;
    if members.is_empty() {
        return Ok(vec![]);
    }

    let mut handles = Vec::new();

    for member in &members {
        let workflow = workflow_repo
            .get_workflow(member.workflow_id)
            .await?
            .ok_or_else(|| anyhow!("Workflow {} not found", member.workflow_id))?;
        let steps = workflow_repo.list_steps(member.workflow_id).await?;
        let edges = workflow_repo.list_edges(member.workflow_id).await?;

        // Create stage_execution row for this member
        let se_id = Uuid::new_v4();
        let se = crate::db::StageExecutionRow {
            id: se_id,
            run_id,
            stage_number,
            stage_name: workflow.name.clone(),
            agent_id: None,
            status: "running".to_string(),
            rendered_prompt: None,
            output: None,
            structured_output: None,
            user_input: None,
            input_tokens: 0,
            output_tokens: 0,
            started_at: Utc::now(),
            completed_at: None,
            duration_ms: 0,
            stage_member_id: Some(member.id),
            pipeline_id: Some(pipeline_id),
        };
        let _ = state.repo.create_stage_execution(&se).await;

        let ctx = WorkflowExecutionContext {
            stage_execution_id: se_id,
            run_id,
            user_id,
            initial_input: initial_input.to_string(),
            prior_outputs: prior_outputs.clone(),
            execution_context: None, // TODO: pass execution context for tool-enabled agents
        };

        let state_clone = state.clone();
        let provider_clone = Arc::clone(&provider);

        handles.push(tokio::spawn(async move {
            let result = execute_workflow(&state_clone, provider_clone, ctx, steps, edges).await;

            // Update stage_execution status
            match &result {
                Ok(wf_result) => {
                    if let Ok(execs) = state_clone.repo.list_stage_executions(run_id).await {
                        if let Some(exec) = execs.into_iter().find(|e| e.id == se_id) {
                            let mut updated = exec;
                            updated.status = "completed".to_string();
                            updated.input_tokens = wf_result.total_input_tokens;
                            updated.output_tokens = wf_result.total_output_tokens;
                            updated.completed_at = Some(Utc::now());
                            let _ = state_clone.repo.update_stage_execution(&updated).await;
                        }
                    }
                }
                Err(e) => {
                    error!("Workflow execution failed: {}", e);
                    if let Ok(execs) = state_clone.repo.list_stage_executions(run_id).await {
                        if let Some(exec) = execs.into_iter().find(|e| e.id == se_id) {
                            let mut updated = exec;
                            updated.status = "failed".to_string();
                            updated.completed_at = Some(Utc::now());
                            let _ = state_clone.repo.update_stage_execution(&updated).await;
                        }
                    }
                }
            }

            result
        }));
    }

    // Await all parallel workflow executions
    let mut results = Vec::new();
    for handle in handles {
        match handle.await {
            Ok(Ok(result)) => results.push(result),
            Ok(Err(e)) => error!("Workflow execution error: {}", e),
            Err(e) => error!("Workflow task panicked: {}", e),
        }
    }

    Ok(results)
}

// ============================================================================
// Helpers
// ============================================================================

/// Parse structured JSON output from raw LLM response text.
fn parse_structured_output(content: &str) -> Option<JsonValue> {
    // Try parsing the whole response as JSON
    if let Ok(v) = serde_json::from_str::<JsonValue>(content) {
        return Some(v);
    }
    // Try extracting JSON from markdown code blocks
    if let Some(start) = content.find("```json") {
        let rest = &content[start + 7..];
        if let Some(end) = rest.find("```") {
            if let Ok(v) = serde_json::from_str::<JsonValue>(rest[..end].trim()) {
                return Some(v);
            }
        }
    }
    // Try extracting from generic code blocks
    if let Some(start) = content.find("```") {
        let rest = &content[start + 3..];
        if let Some(end) = rest.find("```") {
            let block = rest[..end].trim();
            // Skip the language identifier line if present
            let json_text = if let Some(nl) = block.find('\n') { block[nl + 1..].trim() } else { block };
            if let Ok(v) = serde_json::from_str::<JsonValue>(json_text) {
                return Some(v);
            }
        }
    }
    None
}

/// Compute approximate cost in USD based on model and tokens.
fn compute_cost(model_id: &str, input_tokens: i64, output_tokens: i64) -> f32 {
    // Approximate pricing per 1M tokens (input/output)
    let (input_rate, output_rate) = if model_id.contains("opus") {
        (15.0_f32, 75.0_f32)
    } else if model_id.contains("sonnet") {
        (3.0, 15.0)
    } else if model_id.contains("haiku") {
        (0.25, 1.25)
    } else if model_id.contains("gpt-4o") {
        (2.5, 10.0)
    } else if model_id.contains("gpt-4") {
        (30.0, 60.0)
    } else {
        (1.0, 3.0) // Default fallback
    };

    (input_tokens as f32 * input_rate / 1_000_000.0) + (output_tokens as f32 * output_rate / 1_000_000.0)
}

// ============================================================================
// WebSocket Broadcasting
// ============================================================================

fn broadcast_agent_execution_update(
    state: &AppState,
    run_id: Uuid,
    ae_id: &Uuid,
    step_id: Uuid,
    agent_name: &str,
    is_interactive: bool,
    status: &str,
    structured_output: Option<&JsonValue>,
    input_tokens: i64,
    output_tokens: i64,
    cost_usd: f32,
) {
    // Use the pipeline broadcast channel for now
    let pipeline_id = Uuid::nil(); // Will be resolved by the frontend via run_id
    state.broadcast_pipeline(PipelineUpdate {
        run_id,
        pipeline_id,
        event: "agent_execution_update".into(),
        stage_number: None,
        stage_name: Some(agent_name.to_string()),
        agent_id: Some(ae_id.to_string()),
        output: structured_output.map(|v| v.to_string()),
        input_tokens: Some(input_tokens as u64),
        output_tokens: Some(output_tokens as u64),
        duration_ms: None,
        user_input: None,
        timestamp: Utc::now(),
        user_id: None,
    });
}

fn broadcast_for_each_spawned(state: &AppState, run_id: Uuid, stage_execution_id: Uuid, step_id: Uuid, agent_name: &str, count: usize) {
    state.broadcast_pipeline(PipelineUpdate {
        run_id,
        pipeline_id: Uuid::nil(),
        event: "for_each_spawned".into(),
        stage_number: None,
        stage_name: Some(format!("{} ({}x)", agent_name, count)),
        agent_id: None,
        output: Some(serde_json::json!({ "workflow_step_id": step_id, "count": count }).to_string()),
        input_tokens: None,
        output_tokens: None,
        duration_ms: None,
        user_input: None,
        timestamp: Utc::now(),
        user_id: None,
    });
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{WorkflowStepEdgeRow, WorkflowStepRow};

    fn make_step(id: Uuid, workflow_id: Uuid, order: i32) -> WorkflowStepRow {
        WorkflowStepRow {
            id,
            workflow_id,
            agent_id: Uuid::new_v4(),
            execution_mode: "single".to_string(),
            for_each_ref: None,
            prompt_template_id: None,
            prompt_template: String::new(),
            output_schema_id: None,
            output_variable_name: None,
            interactive_agent_id: None,
            for_each_label_field: None,
            display_order: order,
            version: 1,
            room_id: None,
        }
    }

    #[test]
    fn test_topological_sort_linear() {
        let wid = Uuid::new_v4();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();
        let steps = vec![make_step(a, wid, 0), make_step(b, wid, 1), make_step(c, wid, 2)];
        let edges = vec![WorkflowStepEdgeRow { from_step_id: a, to_step_id: b }, WorkflowStepEdgeRow { from_step_id: b, to_step_id: c }];
        let sorted = topological_sort(&steps, &edges).unwrap();
        assert_eq!(sorted, vec![a, b, c]);
    }

    #[test]
    fn test_topological_sort_parallel() {
        let wid = Uuid::new_v4();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();
        let d = Uuid::new_v4();
        let steps = vec![make_step(a, wid, 0), make_step(b, wid, 1), make_step(c, wid, 2), make_step(d, wid, 3)];
        let edges = vec![
            WorkflowStepEdgeRow { from_step_id: a, to_step_id: b },
            WorkflowStepEdgeRow { from_step_id: a, to_step_id: c },
            WorkflowStepEdgeRow { from_step_id: b, to_step_id: d },
            WorkflowStepEdgeRow { from_step_id: c, to_step_id: d },
        ];
        let sorted = topological_sort(&steps, &edges).unwrap();
        assert_eq!(sorted[0], a); // a is first
        assert_eq!(sorted[3], d); // d is last
        assert!(sorted[1..3].contains(&b));
        assert!(sorted[1..3].contains(&c));
    }

    #[test]
    fn test_topological_sort_cycle() {
        let wid = Uuid::new_v4();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let steps = vec![make_step(a, wid, 0), make_step(b, wid, 1)];
        let edges = vec![WorkflowStepEdgeRow { from_step_id: a, to_step_id: b }, WorkflowStepEdgeRow { from_step_id: b, to_step_id: a }];
        assert!(topological_sort(&steps, &edges).is_err());
    }

    #[test]
    fn test_find_entry_steps() {
        let wid = Uuid::new_v4();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();
        let steps = vec![make_step(a, wid, 0), make_step(b, wid, 1), make_step(c, wid, 2)];
        let edges = vec![WorkflowStepEdgeRow { from_step_id: a, to_step_id: b }, WorkflowStepEdgeRow { from_step_id: b, to_step_id: c }];
        let entries = find_entry_steps(&steps, &edges);
        assert_eq!(entries, vec![a]);
    }

    #[test]
    fn test_resolve_variables_simple() {
        let mut outputs = HashMap::new();
        outputs.insert("name".to_string(), serde_json::json!("Dave"));
        let result = resolve_variables("Hello {name}!", &outputs, &HashMap::new());
        assert_eq!(result, "Hello Dave!");
    }

    #[test]
    fn test_resolve_variables_dot_path() {
        let mut outputs = HashMap::new();
        outputs.insert(
            "features".to_string(),
            serde_json::json!({
                "content": [
                    {"name": "Button"},
                    {"name": "Table"}
                ],
                "passdown": "Found 2 features"
            }),
        );
        let result = resolve_variables("Build {features.content.0.name}", &outputs, &HashMap::new());
        assert_eq!(result, "Build Button");
    }

    #[test]
    fn test_resolve_variables_unresolved() {
        let result = resolve_variables("Hello {unknown}!", &HashMap::new(), &HashMap::new());
        assert_eq!(result, "Hello {unknown}!");
    }

    #[test]
    fn test_resolve_for_each_array() {
        let mut outputs = HashMap::new();
        outputs.insert(
            "features".to_string(),
            serde_json::json!({
                "content": [
                    {"name": "Button"},
                    {"name": "Table"},
                    {"name": "Form"}
                ]
            }),
        );
        let array = resolve_for_each_array("features.content", &outputs, &HashMap::new());
        assert_eq!(array.unwrap().len(), 3);
    }

    #[test]
    fn test_extract_for_each_label() {
        let element = serde_json::json!({"name": "Button", "type": "component"});
        assert_eq!(extract_for_each_label(&element, Some("name")), Some("Button".to_string()));
        assert_eq!(extract_for_each_label(&element, None), None);
    }

    #[test]
    fn test_resolve_for_each_path() {
        let element = serde_json::json!({"name": "Button", "type": "component"});
        let result = resolve_for_each_path("features.content.$.name", &element, &HashMap::new(), &HashMap::new());
        assert_eq!(result, "Button");
    }

    #[test]
    fn test_parse_structured_output_raw_json() {
        let json = r#"{"name": "test", "value": 42}"#;
        let result = parse_structured_output(json);
        assert!(result.is_some());
        assert_eq!(result.unwrap()["name"], "test");
    }

    #[test]
    fn test_parse_structured_output_markdown_json() {
        let content = "Here is the result:\n```json\n{\"name\": \"test\"}\n```\nDone.";
        let result = parse_structured_output(content);
        assert!(result.is_some());
        assert_eq!(result.unwrap()["name"], "test");
    }

    #[test]
    fn test_compute_cost_sonnet() {
        let cost = compute_cost("claude-sonnet-4-20250514", 1000, 500);
        // 1000 * 3.0 / 1M + 500 * 15.0 / 1M = 0.003 + 0.0075 = 0.0105
        assert!((cost - 0.0105).abs() < 0.001);
    }
}
