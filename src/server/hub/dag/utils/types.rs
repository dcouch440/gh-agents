//! Core types used across the DAG execution system.

use std::collections::HashMap;
use std::sync::Arc;

use serde_json::Value as JsonValue;
use uuid::Uuid;

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

    /// Build a pass-through output for context/input steps (no LLM call).
    ///
    /// Returns the output and its JSON value (needed for envelope construction).
    pub fn passthrough(output_key: String, content: String) -> (Self, JsonValue) {
        let value = JsonValue::String(content.clone());
        let output = Self {
            variable_name: output_key,
            structured_output: Some(value.clone()),
            raw_output: content,
        };
        (output, value)
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
pub(crate) struct DagPaused {
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

/// Parent workflow context for sub-workflow event relay.
///
/// When a child workflow executes inside a sub-workflow step, this struct
/// carries the parent's identifiers so child step events can be relayed
/// to the parent's WebSocket channel.
#[derive(Debug, Clone)]
pub struct SubWorkflowParentContext {
    pub parent_step_id: Uuid,
    pub parent_run_id: Uuid,
    pub parent_workflow_id: Uuid,
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
    pub wg_client: Option<Arc<crate::execution::WgEasyClient>>,
    /// Frozen workflow snapshot for template-based execution. When present, agent/tool data
    /// is loaded from the snapshot instead of live DB.
    pub snapshot: Option<Arc<super::super::templates::WorkflowSnapshot>>,
    /// Parent context for sub-workflow execution. When set, child step events are relayed
    /// to the parent's WebSocket channel with this context.
    pub parent_context: Option<SubWorkflowParentContext>,
}

/// Result of executing one workflow.
pub struct WorkflowExecutionResult {
    pub outputs: HashMap<String, StepOutput>,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub total_cost_usd: f32,
    pub duration_ms: u64,
}
