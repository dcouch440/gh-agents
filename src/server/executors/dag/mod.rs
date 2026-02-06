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
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::db::traits::{DocumentRepo, WorkflowRepo};
use crate::db::{AgentRow, StepInputRow, StepOutputRow, WorkflowStepEdgeRow, WorkflowStepRow};
use crate::llm::{ContentBlock, LLMProvider, LLMRequest, Message, StopReason, Tool};
use crate::types::{
    ExecutionError, ExecutionMetadata, ExecutionStatus, ForEachAggregateEnvelope, ForEachMetadata,
    IterationError, StepExecutionEnvelope,
};

use crate::server::state::AppState;
use crate::server::ws::PipelineUpdate;

/// Maximum tool use rounds per step execution.
const DAG_MAX_TOOL_ROUNDS: u32 = 15;

/// Completed step output, keyed by output_variable_name.
#[derive(Debug, Clone)]
pub struct StepOutput {
    pub variable_name: String,
    pub structured_output: Option<JsonValue>,
    pub raw_output: String,
}

/// Sentinel error: the DAG paused because a step is awaiting interactive user input.
#[derive(Debug)]
pub struct DagPaused {
    pub step_id: Uuid,
    pub execution_id: Uuid,
}

impl std::fmt::Display for DagPaused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "interactive step {} (execution {}) awaiting user input",
            self.step_id, self.execution_id
        )
    }
}

impl std::error::Error for DagPaused {}

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
pub fn topological_sort(
    steps: &[WorkflowStepRow],
    edges: &[WorkflowStepEdgeRow],
) -> Result<Vec<Uuid>> {
    let step_ids: HashSet<Uuid> = steps.iter().map(|s| s.id).collect();
    let mut in_degree: HashMap<Uuid, usize> = step_ids.iter().map(|id| (*id, 0)).collect();
    let mut adjacency: HashMap<Uuid, Vec<Uuid>> = step_ids.iter().map(|id| (*id, vec![])).collect();

    for edge in edges {
        if step_ids.contains(&edge.from_step_id) && step_ids.contains(&edge.to_step_id) {
            adjacency
                .entry(edge.from_step_id)
                .or_default()
                .push(edge.to_step_id);
            *in_degree.entry(edge.to_step_id).or_default() += 1;
        }
    }

    let mut queue: Vec<Uuid> = in_degree
        .iter()
        .filter(|(_, &deg)| deg == 0)
        .map(|(id, _)| *id)
        .collect();
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
    steps
        .iter()
        .filter(|s| !has_incoming.contains(&s.id))
        .map(|s| s.id)
        .collect()
}

/// Returns step IDs that a given step depends on (parents).
pub fn get_parent_steps(step_id: Uuid, edges: &[WorkflowStepEdgeRow]) -> Vec<Uuid> {
    edges
        .iter()
        .filter(|e| e.to_step_id == step_id)
        .map(|e| e.from_step_id)
        .collect()
}

/// Returns step IDs that depend on a given step (children).
pub fn get_child_steps(step_id: Uuid, edges: &[WorkflowStepEdgeRow]) -> Vec<Uuid> {
    edges
        .iter()
        .filter(|e| e.from_step_id == step_id)
        .map(|e| e.to_step_id)
        .collect()
}

// ============================================================================
// Variable Resolution
// ============================================================================

/// Resolve `{variable}` references in a prompt template.
///
/// Supports dot-path access: `{features.content.0.name}`.
/// Scope: completed step outputs (from this workflow) + prior stage outputs.
pub fn resolve_variables(
    template: &str,
    outputs: &HashMap<String, JsonValue>,
    prior_outputs: &HashMap<String, JsonValue>,
) -> String {
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
fn resolve_path(
    path: &str,
    outputs: &HashMap<String, JsonValue>,
    prior_outputs: &HashMap<String, JsonValue>,
) -> String {
    let parts: Vec<&str> = path.split('.').collect();
    if parts.is_empty() {
        return format!("{{{}}}", path);
    }

    let var_name = parts[0];

    // Look in workflow outputs first, then prior stage outputs
    let root = outputs
        .get(var_name)
        .or_else(|| prior_outputs.get(var_name));

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
pub fn resolve_for_each_array(
    for_each_ref: &str,
    outputs: &HashMap<String, JsonValue>,
    prior_outputs: &HashMap<String, JsonValue>,
) -> Option<Vec<JsonValue>> {
    let parts: Vec<&str> = for_each_ref.split('.').collect();
    if parts.is_empty() {
        return None;
    }

    let var_name = parts[0];
    let root = outputs
        .get(var_name)
        .or_else(|| prior_outputs.get(var_name))?;

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
    element
        .get(field)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

// ============================================================================
// Prompt Composition
// ============================================================================

/// Build the full prompt for a step execution.
///
/// Resolves the prompt template, appends attached document content.
/// If `port_inputs` is provided, port values are injected as structured context
/// and made available for `{port_name}` variable resolution.
pub async fn compose_prompt(
    step: &WorkflowStepRow,
    prompt_template_repo: Option<&dyn crate::db::traits::PromptTemplateRepo>,
    doc_repo: Option<&dyn DocumentRepo>,
    workflow_repo: Option<&dyn WorkflowRepo>,
    server_repo: &dyn crate::db::traits::ServerRepo,
    outputs: &HashMap<String, JsonValue>,
    prior_outputs: &HashMap<String, JsonValue>,
    for_each_element: Option<&JsonValue>,
    port_inputs: Option<&HashMap<String, JsonValue>>,
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

    // Merge port inputs into the variable resolution scope so {port_name} works
    let effective_outputs = if let Some(ports) = port_inputs {
        if !ports.is_empty() {
            let mut merged = outputs.clone();
            for (k, v) in ports {
                merged.insert(k.clone(), v.clone());
            }
            merged
        } else {
            outputs.clone()
        }
    } else {
        outputs.clone()
    };

    // For for_each steps, replace `$` with the current element in variable refs
    let prompt = if let Some(element) = for_each_element {
        // In for_each mode, `{var.content.$.name}` means current element's .name
        // We resolve $ by injecting the element into a special "__for_each_element" key
        let augmented_outputs = effective_outputs.clone();
        // Replace $ references by pre-resolving them
        resolve_for_each_prompt(&raw_prompt, element, &augmented_outputs, prior_outputs)
    } else {
        resolve_variables(&raw_prompt, &effective_outputs, prior_outputs)
    };

    let mut full_prompt = prompt;

    // Append structured port input data block
    if let Some(ports) = port_inputs {
        if !ports.is_empty() {
            full_prompt.push_str("\n\n## Input Data\n");
            for (port_name, value) in ports {
                let formatted =
                    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string());
                full_prompt.push_str(&format!(
                    "\n<{}>\n{}\n</{}>\n",
                    port_name, formatted, port_name
                ));
            }
        }
    }

    // Append agent context documents (global to agent)
    if let Some(_d_repo) = doc_repo {
        if let Ok(agent_docs) = server_repo.get_agent_context(step.agent_id).await {
            for doc in &agent_docs {
                full_prompt.push_str(&format!(
                    "\n\n---\n## {} (Agent Context)\n{}",
                    doc.title, doc.content
                ));
            }
        }
    }

    // Append step documents (specific to this workflow step)
    if let Some(wf_repo) = workflow_repo {
        if let Ok(step_docs) = wf_repo.list_step_documents(step.id).await {
            if let Some(d_repo) = doc_repo {
                for sd in &step_docs {
                    if let Ok(Some(doc)) = d_repo.get_document(sd.document_id).await {
                        full_prompt.push_str(&format!(
                            "\n\n---\n## {} (Step Context)\n{}",
                            doc.title, doc.content
                        ));
                    }
                }
            }
        }
    }

    full_prompt
}

/// Resolve a for_each prompt where `$` represents the current element.
fn resolve_for_each_prompt(
    template: &str,
    element: &JsonValue,
    outputs: &HashMap<String, JsonValue>,
    prior_outputs: &HashMap<String, JsonValue>,
) -> String {
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
fn resolve_for_each_path(
    path: &str,
    element: &JsonValue,
    outputs: &HashMap<String, JsonValue>,
    prior_outputs: &HashMap<String, JsonValue>,
) -> String {
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
// Port-Based Data Flow
// ============================================================================

/// Error types for port resolution failures.
#[derive(Debug)]
pub enum PortResolutionError {
    /// A required input port has no incoming edge and no default value.
    MissingRequiredInput { port_name: String, step_id: Uuid },
    /// The source step hasn't completed yet (shouldn't happen in topo order).
    SourceStepNotCompleted {
        from_step_id: Uuid,
        port_name: String,
    },
    /// The output port definition was not found on the source step.
    OutputPortNotFound { step_id: Uuid, port_name: String },
    /// Data extraction from the envelope failed (json_path didn't match).
    DataExtractionFailed {
        step_id: Uuid,
        port_name: String,
        json_path: String,
    },
}

impl std::fmt::Display for PortResolutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingRequiredInput { port_name, step_id } => {
                write!(
                    f,
                    "required input '{}' missing for step {}",
                    port_name, step_id
                )
            }
            Self::SourceStepNotCompleted {
                from_step_id,
                port_name,
            } => {
                write!(
                    f,
                    "source step {} not completed for port '{}'",
                    from_step_id, port_name
                )
            }
            Self::OutputPortNotFound { step_id, port_name } => {
                write!(
                    f,
                    "output port '{}' not found on step {}",
                    port_name, step_id
                )
            }
            Self::DataExtractionFailed {
                step_id,
                port_name,
                json_path,
            } => {
                write!(
                    f,
                    "data extraction failed for port '{}' on step {} (json_path: {})",
                    port_name, step_id, json_path
                )
            }
        }
    }
}

impl std::error::Error for PortResolutionError {}

/// Navigate a JSON value using dot-path notation.
///
/// Supports field access (`"name"`), array index (`"0"`), and nesting (`"data.items.0.name"`).
/// Returns `None` if any segment fails to resolve or the final value is null.
pub fn resolve_dot_path(value: &JsonValue, path: &str) -> Option<JsonValue> {
    if path.is_empty() {
        return Some(value.clone());
    }
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = value.clone();
    for part in &parts {
        current = if let Ok(idx) = part.parse::<usize>() {
            current.get(idx).cloned()?
        } else {
            current.get(*part).cloned()?
        };
    }
    if current.is_null() {
        None
    } else {
        Some(current)
    }
}

/// Resolve all input port values for a step from upstream envelopes.
///
/// For each incoming edge with `from_output_port` and `to_input_port` set:
/// 1. Gets the source step's completed envelope
/// 2. Finds the output port definition to get its `json_path`
/// 3. Extracts data from `envelope.data` via the json_path
/// 4. Optionally applies `transform_jsonpath` from the edge
/// 5. Maps to the `to_input_port` key in the result
///
/// Missing optional inputs are filled from `StepInputRow.default_value`.
/// Missing required inputs with no default produce an error.
pub fn resolve_port_inputs(
    step_id: Uuid,
    edges: &[WorkflowStepEdgeRow],
    step_inputs: &[StepInputRow],
    source_outputs_map: &HashMap<Uuid, Vec<StepOutputRow>>,
    completed_envelopes: &HashMap<Uuid, StepExecutionEnvelope>,
) -> std::result::Result<HashMap<String, JsonValue>, PortResolutionError> {
    let mut resolved: HashMap<String, JsonValue> = HashMap::new();

    // Find all incoming edges with port wiring
    let incoming_edges: Vec<&WorkflowStepEdgeRow> = edges
        .iter()
        .filter(|e| {
            e.to_step_id == step_id && e.from_output_port.is_some() && e.to_input_port.is_some()
        })
        .collect();

    for edge in &incoming_edges {
        let from_port = edge.from_output_port.as_ref().unwrap();
        let to_port = edge.to_input_port.as_ref().unwrap();

        // Get source envelope
        let envelope = completed_envelopes.get(&edge.from_step_id).ok_or_else(|| {
            PortResolutionError::SourceStepNotCompleted {
                from_step_id: edge.from_step_id,
                port_name: from_port.clone(),
            }
        })?;

        // Find output port definition for json_path
        let source_outputs = source_outputs_map.get(&edge.from_step_id);
        let output_port =
            source_outputs.and_then(|outputs| outputs.iter().find(|o| o.port_name == *from_port));

        // Extract data from envelope
        let extracted = if let Some(port_def) = output_port {
            // Use the output port's json_path to extract from envelope.data
            if let Some(ref data) = envelope.data {
                resolve_dot_path(data, &port_def.json_path)
            } else {
                None
            }
        } else {
            // No output port definition — try using the port name as json_path
            if let Some(ref data) = envelope.data {
                resolve_dot_path(data, from_port)
            } else {
                None
            }
        };

        let mut value = extracted.ok_or_else(|| {
            let json_path = output_port
                .map(|p| p.json_path.clone())
                .unwrap_or_else(|| from_port.clone());
            PortResolutionError::DataExtractionFailed {
                step_id: edge.from_step_id,
                port_name: from_port.clone(),
                json_path,
            }
        })?;

        // Apply edge transform if present
        if let Some(ref transform) = edge.transform_jsonpath {
            if let Some(transformed) = resolve_dot_path(&value, transform) {
                value = transformed;
            }
        }

        resolved.insert(to_port.clone(), value);
    }

    // Fill defaults for missing optional inputs, error on missing required
    for input in step_inputs {
        if resolved.contains_key(&input.port_name) {
            continue;
        }
        if let Some(ref default_val) = input.default_value {
            resolved.insert(input.port_name.clone(), default_val.clone());
        } else if input.required {
            return Err(PortResolutionError::MissingRequiredInput {
                port_name: input.port_name.clone(),
                step_id,
            });
        }
    }

    Ok(resolved)
}

// ============================================================================
// Step Execution
// ============================================================================

/// Resolve the LLM tool definitions for an agent from the database.
///
/// Returns an empty vec if the agent has no tools assigned — in that case
/// the LLM will never return `StopReason::ToolUse` and the loop executes once.
async fn resolve_agent_tools(state: &AppState, agent_id: Uuid) -> Vec<Tool> {
    let tools = match state.repo().get_agent_tools(agent_id).await {
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
) -> Result<(Uuid, StepExecutionEnvelope, i64, i64, f32)> {
    let ae_repo = &state.repos().agent_executions;

    // Build system prompt with output schema instructions
    let mut system_prompt = agent.system_prompt.clone();
    if let Some(schema_id) = step.output_schema_id {
        let os_repo = &state.repos().output_schemas;
        if let Ok(Some(schema)) = os_repo.get_output_schema(schema_id).await {
            system_prompt.push_str(&format!(
                "\n\nYou MUST respond with valid JSON matching this schema:\n```json\n{}\n```\nRespond ONLY with the JSON object, no other text.",
                serde_json::to_string_pretty(&schema.schema).unwrap_or_default()
            ));
        }
    }

    // Resolve tools for this agent (empty vec = no tools = single call)
    let tools = resolve_agent_tools(state, agent.id).await;
    let tool_names: Vec<String> = tools.iter().map(|t| t.name.clone()).collect();

    // Create agent_execution row
    let ae_row = ae_repo
        .create_agent_execution(
            agent.id,
            Some(step.id),
            false,
            None,
            &system_prompt,
            prompt,
            None,
            None,
            None,
        )
        .await?;

    // Record system + user messages
    let _ = ae_repo
        .create_execution_message(ae_row.id, "system", &system_prompt, None, 0, 0)
        .await;
    let _ = ae_repo
        .create_execution_message(ae_row.id, "user", prompt, None, 0, 0)
        .await;

    // Broadcast running status
    broadcast_agent_execution_update(
        state,
        ctx.run_id,
        &ae_row.id,
        step.id,
        &agent.name,
        false,
        "running",
        None,
        0,
        0,
        0.0,
    );

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

        let response = provider
            .send_message(request)
            .await
            .map_err(|e| anyhow!("LLM call failed (round {}): {}", round, e))?;

        let in_tok = response.usage.input_tokens as i64;
        let out_tok = response.usage.output_tokens as i64;
        let cost = compute_cost(&agent.model_id, in_tok, out_tok);
        total_input_tokens += in_tok;
        total_output_tokens += out_tok;
        total_cost_usd += cost;

        // Write token_ledger for every LLM call
        let tl_repo = &state.repos().token_ledger;
        let _ = tl_repo
            .insert_ledger_entry(
                ctx.user_id,
                Some(ae_row.id),
                &agent.model_id,
                in_tok,
                out_tok,
                cost,
            )
            .await;

        if response.stop_reason == StopReason::ToolUse {
            // Record the assistant message with tool calls
            let _ = ae_repo
                .create_execution_message(
                    ae_row.id,
                    "assistant",
                    &response.content,
                    None,
                    in_tok,
                    out_tok,
                )
                .await;

            // Add assistant response (with content blocks) to conversation
            messages.push(Message::assistant_with_blocks(
                response.content_blocks.clone(),
            ));

            // Execute each tool call and collect results
            let mut tool_results = Vec::new();
            for block in &response.content_blocks {
                if let ContentBlock::ToolUse { id, name, input } = block {
                    info!(agent = %agent.name, round = round, tool = %name, "DAG step tool call");

                    let result = match &ctx.execution_context {
                        Some(exec_ctx) => {
                            crate::agents::execution_tools::execute_execution_tool(
                                name,
                                input,
                                exec_ctx,
                                Some(&tool_names),
                            )
                            .await
                        }
                        None => {
                            serde_json::json!({ "error": "No execution context available for tool calls" })
                        }
                    };

                    let result_str = serde_json::to_string_pretty(&result)
                        .unwrap_or_else(|_| result.to_string());

                    // Record tool call and result as execution_messages
                    let call_content =
                        serde_json::json!({ "tool": name, "input": input }).to_string();
                    let _ = ae_repo
                        .create_execution_message(
                            ae_row.id,
                            "assistant",
                            &call_content,
                            Some(id.clone()),
                            0,
                            0,
                        )
                        .await;
                    let _ = ae_repo
                        .create_execution_message(
                            ae_row.id,
                            "tool",
                            &result_str,
                            Some(id.clone()),
                            0,
                            0,
                        )
                        .await;

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
        let _ = ae_repo
            .create_execution_message(
                ae_row.id,
                "assistant",
                &final_content,
                None,
                in_tok,
                out_tok,
            )
            .await;
        break;
    }

    // Parse structured output from final response
    let structured_output = parse_structured_output(&final_content);

    // Get execution timing
    let execution_time_ms = (Utc::now() - ae_row.started_at).num_milliseconds() as u64;

    // Wrap output in StepExecutionEnvelope
    let envelope = StepExecutionEnvelope {
        status: if final_content.is_empty() {
            ExecutionStatus::Error
        } else {
            ExecutionStatus::Success
        },
        data: structured_output.clone(),
        metadata: ExecutionMetadata {
            execution_id: ae_row.id,
            execution_time_ms,
            tokens_in: Some(total_input_tokens as i32),
            tokens_out: Some(total_output_tokens as i32),
            cost_usd: Some(total_cost_usd as f64),
            model: Some(agent.model_id.clone()),
            agent_id: Some(agent.id),
            iteration_index: _for_each_index.map(|i| i as usize),
            iteration_label: _for_each_label.clone(),
            routing_label: None,
            selected_routing_document_id: None,
        },
        error: None,
    };

    // Store envelope as structured_output in database
    let envelope_json = serde_json::to_value(&envelope)
        .map_err(|e| anyhow!("Failed to serialize envelope: {}", e))?;

    // Update agent_execution with results (storing envelope instead of raw output)
    let _ = ae_repo
        .update_agent_execution_status(
            ae_row.id,
            "completed",
            Some(final_content.clone()),
            Some(envelope_json.clone()),
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
        Some(&envelope_json),
        total_input_tokens,
        total_output_tokens,
        total_cost_usd,
    );

    Ok((
        ae_row.id,
        envelope,
        total_input_tokens,
        total_output_tokens,
        total_cost_usd,
    ))
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
    let ae_repo = state
        .agent_execution_repo()
        .ok_or_else(|| anyhow!("agent_execution_repo not configured"))?;

    let system_prompt = interactive_agent.system_prompt.clone();
    let review_prompt = format!(
        "Review the following output and provide feedback:\n\n{}",
        main_output
    );

    // Create interactive agent_execution
    let iae_row = ae_repo
        .create_agent_execution(
            interactive_agent.id,
            Some(step.id),
            true,
            Some(parent_ae_id),
            &system_prompt,
            &review_prompt,
            None,
            None,
            None,
        )
        .await?;

    // Record messages
    let _ = ae_repo
        .create_execution_message(iae_row.id, "system", &system_prompt, None, 0, 0)
        .await;
    let _ = ae_repo
        .create_execution_message(iae_row.id, "user", &review_prompt, None, 0, 0)
        .await;

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

    let response = provider
        .send_message(request)
        .await
        .map_err(|e| anyhow!("Interactive LLM call failed: {}", e))?;

    let input_tokens = response.usage.input_tokens as i64;
    let output_tokens = response.usage.output_tokens as i64;
    let cost_usd = compute_cost(&interactive_agent.model_id, input_tokens, output_tokens);

    // Record assistant response
    let _ = ae_repo
        .create_execution_message(
            iae_row.id,
            "assistant",
            &response.content,
            None,
            input_tokens,
            output_tokens,
        )
        .await;

    // Write token_ledger for the review call
    let tl_repo = &state.repos().token_ledger;
    let _ = tl_repo
        .insert_ledger_entry(
            ctx.user_id,
            Some(iae_row.id),
            &interactive_agent.model_id,
            input_tokens,
            output_tokens,
            cost_usd,
        )
        .await;

    // Set status to awaiting_user — the user will chat and approve via the API
    let _ = ae_repo
        .update_agent_execution_status(
            iae_row.id,
            "awaiting_user",
            Some(response.content.clone()),
            None,
        )
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

    // Halt the DAG — the caller should propagate this error so the pipeline pauses.
    // The user interacts via POST /agent-executions/:id/messages, then approves
    // via POST /agent-executions/:id/approve to resume.
    Err(DagPaused {
        step_id: step.id,
        execution_id: iae_row.id,
    }
    .into())
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
    let var_outputs: Arc<RwLock<HashMap<String, JsonValue>>> =
        Arc::new(RwLock::new(HashMap::new()));

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
        let agent = state
            .repo()
            .get_persisted_agent(step.agent_id)
            .await?
            .ok_or_else(|| anyhow!("Agent {} not found", step.agent_id))?;

        // Build current outputs snapshot for variable resolution
        let current_outputs = {
            let guard = var_outputs.read().await;
            guard.clone()
        };

        if step.execution_mode == "for_each" {
            // Expand for_each step into N parallel executions
            let for_each_ref = step
                .for_each_ref
                .as_deref()
                .ok_or_else(|| anyhow!("for_each step {} missing for_each_ref", step.id))?;
            let array = resolve_for_each_array(for_each_ref, &current_outputs, &ctx.prior_outputs)
                .ok_or_else(|| {
                    anyhow!(
                        "for_each_ref '{}' did not resolve to an array",
                        for_each_ref
                    )
                })?;

            let label_field = step.for_each_label_field.as_deref();

            // Broadcast for_each_spawned event
            broadcast_for_each_spawned(
                state,
                ctx.run_id,
                ctx.stage_execution_id,
                step.id,
                &agent.name,
                array.len(),
            );

            // Execute all iterations (could parallelize, but sequential for now to avoid overwhelming the LLM)
            let mut iteration_envelopes = Vec::new();
            let mut iteration_errors = Vec::new();
            let mut successful_count = 0;
            let mut failed_count = 0;

            for (idx, element) in array.iter().enumerate() {
                let label = extract_for_each_label(element, label_field);

                let prompt = compose_prompt(
                    step,
                    state.prompt_template_repo().as_deref(),
                    state.doc_repo().as_deref(),
                    state.workflow_repo().as_deref(),
                    state.repo().as_ref(),
                    &current_outputs,
                    &ctx.prior_outputs,
                    Some(element),
                    None,
                )
                .await;

                match execute_step(
                    state,
                    provider.as_ref(),
                    &ctx,
                    step,
                    &agent,
                    &prompt,
                    Some(idx as i32),
                    label.clone(),
                )
                .await
                {
                    Ok((_ae_id, envelope, in_tok, out_tok, cost)) => {
                        total_input_tokens += in_tok;
                        total_output_tokens += out_tok;
                        total_cost_usd += cost;

                        // Track success/failure
                        if envelope.status == ExecutionStatus::Success {
                            successful_count += 1;
                        } else {
                            failed_count += 1;
                        }

                        // Handle interactive review for each iteration
                        if let Some(interactive_agent_id) = step.interactive_agent_id {
                            if let Ok(Some(_ia)) =
                                state.repo().get_persisted_agent(interactive_agent_id).await
                            {
                                // For interactive review, we need the raw LLM output
                                // The envelope doesn't store this, so we'll skip review for now
                                // TODO: Add raw_output field to envelope or retrieve from DB
                                warn!("Interactive review not yet supported with envelopes for step {}", step.id);
                            }
                        }

                        iteration_envelopes.push(envelope);
                    }
                    Err(e) => {
                        failed_count += 1;
                        error!(
                            "for_each iteration {} failed for step {}: {}",
                            idx, step.id, e
                        );

                        // Create error envelope for failed iteration
                        let error_message = format!("{}", e);
                        iteration_errors.push(IterationError {
                            iteration_index: idx,
                            iteration_label: label.clone(),
                            message: error_message.clone(),
                            error_type: "ExecutionError".to_string(),
                        });

                        // Add error envelope to collection
                        iteration_envelopes.push(StepExecutionEnvelope {
                            status: ExecutionStatus::Error,
                            data: None,
                            metadata: ExecutionMetadata {
                                execution_id: Uuid::new_v4(),
                                execution_time_ms: 0,
                                tokens_in: None,
                                tokens_out: None,
                                cost_usd: None,
                                model: Some(agent.model_id.clone()),
                                agent_id: Some(agent.id),
                                iteration_index: Some(idx),
                                iteration_label: label,
                                routing_label: None,
                                selected_routing_document_id: None,
                            },
                            error: Some(ExecutionError {
                                message: error_message,
                                error_type: "ExecutionError".to_string(),
                                retryable: false,
                                details: None,
                            }),
                        });
                    }
                }
            }

            // Build ForEachAggregateEnvelope
            let aggregate_envelope = ForEachAggregateEnvelope {
                status: if failed_count == array.len() {
                    ExecutionStatus::Error
                } else if failed_count > 0 {
                    ExecutionStatus::Partial
                } else {
                    ExecutionStatus::Success
                },
                data: iteration_envelopes,
                metadata: ForEachMetadata {
                    total_iterations: array.len(),
                    successful_iterations: successful_count,
                    failed_iterations: failed_count,
                    execution_time_ms: 0, // TODO: Track timing
                    total_tokens_in: total_input_tokens as i32,
                    total_tokens_out: total_output_tokens as i32,
                    total_cost_usd: total_cost_usd as f64,
                    routing_mode: step.routing_mode.clone(),
                    routing_distribution: None, // TODO: Track label distribution
                },
                errors: iteration_errors,
            };

            // Store aggregate envelope
            let variable_name = step.output_variable_name.clone().unwrap_or_default();
            let aggregate_json = serde_json::to_value(&aggregate_envelope)?;

            {
                let mut guard = var_outputs.write().await;
                guard.insert(variable_name.clone(), aggregate_json.clone());
            }
            {
                let mut guard = completed.write().await;
                guard.insert(
                    *step_id,
                    StepOutput {
                        variable_name,
                        structured_output: Some(aggregate_json),
                        raw_output: String::new(),
                    },
                );
            }
        } else {
            // Single execution
            let prompt = compose_prompt(
                step,
                state.prompt_template_repo().as_deref(),
                state.doc_repo().as_deref(),
                state.workflow_repo().as_deref(),
                state.repo().as_ref(),
                &current_outputs,
                &ctx.prior_outputs,
                None,
                None,
            )
            .await;

            let (_ae_id, envelope, in_tok, out_tok, cost) = execute_step(
                state,
                provider.as_ref(),
                &ctx,
                step,
                &agent,
                &prompt,
                None,
                None,
            )
            .await?;

            total_input_tokens += in_tok;
            total_output_tokens += out_tok;
            total_cost_usd += cost;

            // Handle interactive review
            if let Some(interactive_agent_id) = step.interactive_agent_id {
                if let Ok(Some(_ia)) = state.repo().get_persisted_agent(interactive_agent_id).await
                {
                    // TODO: Interactive review not yet supported with envelopes
                    // Need to retrieve raw output from database or add it to envelope
                    warn!(
                        "Interactive review not yet supported with envelopes for step {}",
                        step.id
                    );
                }
            }

            // Store envelope (wrapped in StepOutput for backward compatibility)
            let variable_name = step.output_variable_name.clone().unwrap_or_default();
            let envelope_json = serde_json::to_value(&envelope)?;

            if !variable_name.is_empty() {
                let mut guard = var_outputs.write().await;
                guard.insert(variable_name.clone(), envelope_json.clone());
            }
            {
                let mut guard = completed.write().await;
                guard.insert(
                    *step_id,
                    StepOutput {
                        variable_name,
                        structured_output: Some(envelope_json),
                        raw_output: String::new(),
                    },
                );
            }
        }
    }

    let completed_guard = completed.read().await;
    let final_outputs: HashMap<String, StepOutput> = completed_guard
        .iter()
        .map(|(id, out)| (id.to_string(), out.clone()))
        .collect();
    Ok(WorkflowExecutionResult {
        outputs: final_outputs,
        total_input_tokens,
        total_output_tokens,
        total_cost_usd,
    })
}

// ============================================================================
// Helpers
// ============================================================================

/// Parse structured JSON output from raw LLM response text.
pub(crate) fn parse_structured_output(content: &str) -> Option<JsonValue> {
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
            let json_text = if let Some(nl) = block.find('\n') {
                block[nl + 1..].trim()
            } else {
                block
            };
            if let Ok(v) = serde_json::from_str::<JsonValue>(json_text) {
                return Some(v);
            }
        }
    }
    None
}

/// Compute approximate cost in USD based on model and tokens.
pub(crate) fn compute_cost(model_id: &str, input_tokens: i64, output_tokens: i64) -> f32 {
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

    (input_tokens as f32 * input_rate / 1_000_000.0)
        + (output_tokens as f32 * output_rate / 1_000_000.0)
}

// ============================================================================
// WebSocket Broadcasting
// ============================================================================

fn broadcast_agent_execution_update(
    state: &AppState,
    run_id: Uuid,
    ae_id: &Uuid,
    _step_id: Uuid,
    agent_name: &str,
    _is_interactive: bool,
    _status: &str,
    structured_output: Option<&JsonValue>,
    input_tokens: i64,
    output_tokens: i64,
    _cost_usd: f32,
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

fn broadcast_for_each_spawned(
    state: &AppState,
    run_id: Uuid,
    _stage_execution_id: Uuid,
    step_id: Uuid,
    agent_name: &str,
    count: usize,
) {
    state.broadcast_pipeline(PipelineUpdate {
        run_id,
        pipeline_id: Uuid::nil(),
        event: "for_each_spawned".into(),
        stage_number: None,
        stage_name: Some(format!("{} ({}x)", agent_name, count)),
        agent_id: None,
        output: Some(
            serde_json::json!({ "workflow_step_id": step_id, "count": count }).to_string(),
        ),
        input_tokens: None,
        output_tokens: None,
        duration_ms: None,
        user_input: None,
        timestamp: Utc::now(),
        user_id: None,
    });
}

#[cfg(test)]
mod tests;
