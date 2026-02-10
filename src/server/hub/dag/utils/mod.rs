//! Workflow DAG utility functions.
//!
//! Pure functions for DAG operations: topological sort, variable resolution,
//! prompt composition, port-based data flow, and label routing.
//!
//! These are re-exported by `hub::dag` and used throughout the execution system.

use std::collections::{HashMap, HashSet};

use anyhow::{anyhow, Result};
use serde_json::Value as JsonValue;
use uuid::Uuid;

use tracing::warn;

use crate::db::traits::{DocumentRepo, WorkflowRepo};
use crate::db::{StepInputRow, StepOutputRow, WorkflowStepEdgeRow, WorkflowStepRow};
use crate::types::{DownstreamRoutingContext, StepExecutionEnvelope};

/// Completed step output, keyed by output_variable_name.
#[derive(Debug, Clone)]
pub struct StepOutput {
    pub variable_name: String,
    pub structured_output: Option<JsonValue>,
    pub raw_output: String,
}

impl StepOutput {
    /// Create a sentinel output for a step that was skipped due to unmatched conditional edges.
    pub fn skipped(step_id: Uuid) -> Self {
        Self {
            variable_name: format!("__skipped_{}", step_id),
            structured_output: None,
            raw_output: String::new(),
        }
    }
}

/// The readiness state of a step in the DAG.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepReadiness {
    /// All dependencies satisfied, step should execute.
    Ready,
    /// Some unconditional parents not yet completed. Check again later.
    Waiting,
    /// All conditional edges evaluated and none matched. Step should be permanently skipped.
    Skipped,
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

/// Configuration for creating a persistent Docker container per workflow step.
///
/// Stored on the workflow and used at runtime to spin up containers.
#[derive(Debug, Clone)]
pub struct ContainerExecutionConfig {
    /// GitHub repo clone URL (e.g., "https://github.com/owner/repo.git").
    pub clone_url: String,
    /// Branch to checkout after clone. None = default branch.
    pub branch: Option<String>,
    /// GitHub token for authenticated clone/push.
    pub github_token: crate::execution::RedactedString,
    /// Override Docker image (default: nexor-agent:latest).
    pub image: Option<String>,
    /// Override memory limit (default: 2g).
    pub memory_limit: Option<String>,
    /// Override CPU limit (default: 2.0).
    pub cpu_limit: Option<String>,
    /// When true, each container is paired with a WireGuard VPN sidecar.
    pub vpn_enabled: bool,
}

/// Context passed into the DAG executor for one workflow run.
#[derive(Clone)]
pub struct WorkflowExecutionContext {
    pub stage_execution_id: Uuid,
    pub run_id: Uuid,
    pub user_id: Uuid,
    pub initial_input: String,
    /// Outputs from prior pipeline stages, keyed by variable name.
    pub prior_outputs: HashMap<String, JsonValue>,
    /// Execution context for tool calls (file ops, git, etc.). None if tools are not available.
    pub execution_context: Option<crate::execution::ExecutionContext>,
    /// Container config for running steps in isolated Docker containers. None = local execution.
    pub container_config: Option<ContainerExecutionConfig>,
    /// wg-easy API client for VPN peer management. None if VPN is not configured.
    pub wg_client: Option<std::sync::Arc<crate::execution::WgEasyClient>>,
}

/// Result of executing one workflow.
pub struct WorkflowExecutionResult {
    pub outputs: HashMap<String, StepOutput>,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub total_cost_usd: f32,
    pub duration_ms: u64,
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
// Conditional Edge Evaluation
// ============================================================================

/// Evaluate a single edge condition against the parent step's output envelope.
///
/// Supports:
/// - `"port_match"`: Checks `envelope.data[field] == value` (route protocol)
/// - `"equals"`: Checks `envelope.data[field] == value` (review protocol)
///
/// Returns `false` for unknown condition types (fail-closed), missing fields,
/// or null envelope data.
pub fn evaluate_edge_condition(
    edge: &WorkflowStepEdgeRow,
    parent_envelope: &StepExecutionEnvelope,
) -> bool {
    let condition_type = match &edge.condition_type {
        Some(ct) => ct.as_str(),
        None => return true, // No condition = unconditional edge
    };

    let condition_value = match &edge.condition_value {
        Some(cv) => cv,
        None => return false,
    };

    let data = match &parent_envelope.data {
        Some(d) => d,
        None => return false,
    };

    match condition_type {
        "equals" | "port_match" => {
            let field = match condition_value.get("field").and_then(|f| f.as_str()) {
                Some(f) => f,
                None => return false,
            };
            let expected = match condition_value.get("value") {
                Some(v) => v,
                None => return false,
            };
            let actual = match data.get(field) {
                Some(v) => v,
                None => return false,
            };
            actual == expected
        }
        _ => {
            warn!("Unknown edge condition type: {}", condition_type);
            false
        }
    }
}

/// Determine whether a step should execute based on its incoming edges.
///
/// Rules:
/// 1. Entry steps (no incoming edges) always execute.
/// 2. All unconditional parent steps must be completed.
/// 3. If conditional edges exist, at least one must have a completed parent
///    whose output matches the condition.
/// 4. If ONLY conditional edges exist and NONE match, the step is skipped.
pub fn check_step_readiness(
    step_id: Uuid,
    edges: &[WorkflowStepEdgeRow],
    completed: &HashMap<Uuid, StepOutput>,
    completed_envelopes: &HashMap<Uuid, StepExecutionEnvelope>,
) -> StepReadiness {
    let incoming: Vec<&WorkflowStepEdgeRow> =
        edges.iter().filter(|e| e.to_step_id == step_id).collect();

    if incoming.is_empty() {
        return StepReadiness::Ready;
    }

    // Partition into unconditional and conditional edges
    let unconditional: Vec<&&WorkflowStepEdgeRow> = incoming
        .iter()
        .filter(|e| e.condition_type.is_none())
        .collect();
    let conditional: Vec<&&WorkflowStepEdgeRow> = incoming
        .iter()
        .filter(|e| e.condition_type.is_some())
        .collect();

    // All unconditional parents must be completed
    for edge in &unconditional {
        if !completed.contains_key(&edge.from_step_id) {
            return StepReadiness::Waiting;
        }
    }

    // If there are conditional edges, at least one must match
    if !conditional.is_empty() {
        let mut any_parent_pending = false;
        let mut any_matched = false;

        for edge in &conditional {
            if let Some(envelope) = completed_envelopes.get(&edge.from_step_id) {
                if evaluate_edge_condition(edge, envelope) {
                    any_matched = true;
                    // Ensure the matched parent is actually in completed
                    if !completed.contains_key(&edge.from_step_id) {
                        return StepReadiness::Waiting;
                    }
                }
            } else {
                // Parent hasn't produced an envelope yet
                any_parent_pending = true;
            }
        }

        if !any_matched {
            // If some parents haven't run yet, we might match later
            if any_parent_pending {
                return StepReadiness::Waiting;
            }
            // All conditional parents ran, none matched => skip
            return StepReadiness::Skipped;
        }
    }

    StepReadiness::Ready
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

    // If no prompt template exists, construct one from available inputs so data flows
    // through without requiring explicit templates on every step.
    let raw_prompt = if raw_prompt.is_empty() {
        let mut parts: Vec<String> = Vec::new();
        // Port inputs from upstream steps (wired via edges)
        if let Some(ports) = port_inputs {
            for (_, value) in ports {
                match value {
                    JsonValue::String(s) => parts.push(s.clone()),
                    other => parts.push(
                        serde_json::to_string_pretty(other).unwrap_or_else(|_| other.to_string()),
                    ),
                }
            }
        }
        // Prior outputs (port of entry / collection pipeline)
        if parts.is_empty() {
            for (_, value) in prior_outputs {
                match value {
                    JsonValue::String(s) => parts.push(s.clone()),
                    other => parts.push(
                        serde_json::to_string_pretty(other).unwrap_or_else(|_| other.to_string()),
                    ),
                }
            }
        }
        parts.join("\n\n")
    } else {
        raw_prompt
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

    let mut full_prompt = format!("<task>\n{}\n</task>", prompt);

    // Track whether we've opened a <context> block (for port inputs + documents).
    let mut context_opened = false;

    // Append structured port input data block — only for ports NOT referenced in the template.
    // If the user wrote {port_name} or {port_name.field}, the data is already inlined.
    if let Some(ports) = port_inputs {
        let unreferenced: Vec<_> = ports
            .iter()
            .filter(|(name, _)| !raw_prompt.contains(&format!("{{{}", name)))
            .collect();
        if !unreferenced.is_empty() {
            full_prompt.push_str("\n\n<context>");
            context_opened = true;
            for (port_name, value) in unreferenced {
                let formatted =
                    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string());
                full_prompt.push_str(&format!(
                    "\n<input name=\"{}\">\n{}\n</input>",
                    port_name, formatted
                ));
            }
        }
    }

    // Append agent context documents (global to agent)
    if let Some(_d_repo) = doc_repo {
        if let Some(agent_id) = step.agent_id {
            if let Ok(agent_docs) = server_repo.get_agent_context(agent_id).await {
                if !agent_docs.is_empty() && !context_opened {
                    full_prompt.push_str("\n\n<context>");
                    context_opened = true;
                }
                for doc in &agent_docs {
                    full_prompt.push_str(&format!(
                        "\n<document title=\"{}\" source=\"agent\">\n{}\n</document>",
                        doc.title, doc.content
                    ));
                }
            }
        }
    }

    // Append step documents (specific to this workflow step)
    if let Some(wf_repo) = workflow_repo {
        if let Ok(step_docs) = wf_repo.list_step_documents(step.id).await {
            if let Some(d_repo) = doc_repo {
                let mut step_doc_contents = Vec::new();
                for sd in &step_docs {
                    if let Ok(Some(doc)) = d_repo.get_document(sd.document_id).await {
                        step_doc_contents.push(doc);
                    }
                }
                if !step_doc_contents.is_empty() && !context_opened {
                    full_prompt.push_str("\n\n<context>");
                    context_opened = true;
                }
                for doc in &step_doc_contents {
                    full_prompt.push_str(&format!(
                        "\n<document title=\"{}\" source=\"step\">\n{}\n</document>",
                        doc.title, doc.content
                    ));
                }
            }
        }
    }

    // Close context block if it was opened
    if context_opened {
        full_prompt.push_str("\n</context>");
    }

    full_prompt
}

// ============================================================================
// Downstream Routing Context Injection (Phase 6)
// ============================================================================

/// Build a routing instruction text block from downstream routing context.
///
/// Appended to a planner step's prompt to inform the LLM about valid label
/// values, their meanings, and which agents handle each route.
pub fn build_routing_instruction_block(ctx: &DownstreamRoutingContext) -> String {
    let mut block = String::new();

    block.push_str("\n\n<routing>\n");
    block.push_str(&format!(
        "Each item MUST include a \"{}\" field set to exactly one of the following values.\n",
        ctx.routing_field
    ));
    block.push_str("Pick the single best match for each item. Do not use any other values.\n\n");

    for route in &ctx.routes {
        block.push_str(&format!("- {}", route.label_value));

        if let Some(ref desc) = route.description {
            block.push_str(&format!(": {}", desc));
        }

        block.push('\n');

        if route.agent_tools.is_empty() {
            block.push_str(&format!("  Routed to: {} (no tools)\n\n", route.agent_name));
        } else {
            let tools_str = route.agent_tools.join(", ");
            block.push_str(&format!(
                "  Routed to: {} (tools: {})\n\n",
                route.agent_name, tools_str
            ));
        }
    }

    block.push_str("</routing>");

    block
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

    // Find all incoming edges with port wiring, filtering out non-matching conditional edges
    let incoming_edges: Vec<&WorkflowStepEdgeRow> = edges
        .iter()
        .filter(|e| {
            e.to_step_id == step_id && e.from_output_port.is_some() && e.to_input_port.is_some()
        })
        .filter(|e| {
            // Skip conditional edges whose condition didn't match
            if e.condition_type.is_some() {
                completed_envelopes
                    .get(&e.from_step_id)
                    .map(|env| evaluate_edge_condition(e, env))
                    .unwrap_or(false)
            } else {
                true
            }
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

#[cfg(test)]
mod tests;
