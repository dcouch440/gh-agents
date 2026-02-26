//! Execution envelope types for standardized output wrapping
//!
//! All step executions return StepExecutionEnvelope with consistent structure.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ============================================================================
// Execution Type Discriminator
// ============================================================================

/// Discriminator for `agent_executions` rows — replaces implicit NULL-pattern
/// detection with an explicit type tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionType {
    /// Step-level dispatch: background LLM configures a step.
    Dispatch,
    /// Workflow-level dispatch: background LLM configures workflow topology.
    ManagerDispatch,
    /// Normal workflow step with agent.
    #[default]
    DagStep,
    /// Designer pre-lifecycle: protocol expansion phase.
    AgentDesigner,
    /// Workforce pipeline agent execution.
    PipelineAgent,
    /// User approval gate (formerly `is_interactive = true`).
    InteractiveReview,
    /// Verification agent critiquing primary output.
    DebateVerification,
    /// Deprecated: board dispatch is now agentless. Retained for existing DB rows.
    BoardDispatch,
}

impl ExecutionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Dispatch => "dispatch",
            Self::ManagerDispatch => "manager_dispatch",
            Self::DagStep => "dag_step",
            Self::AgentDesigner => "agent_designer",
            Self::PipelineAgent => "pipeline_agent",
            Self::InteractiveReview => "interactive_review",
            Self::DebateVerification => "debate_verification",
            Self::BoardDispatch => "board_dispatch",
        }
    }
}

impl std::fmt::Display for ExecutionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for ExecutionType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "dispatch" => Ok(Self::Dispatch),
            "manager_dispatch" => Ok(Self::ManagerDispatch),
            "dag_step" => Ok(Self::DagStep),
            "agent_designer" => Ok(Self::AgentDesigner),
            "pipeline_agent" => Ok(Self::PipelineAgent),
            "interactive_review" => Ok(Self::InteractiveReview),
            "debate_verification" => Ok(Self::DebateVerification),
            "board_dispatch" => Ok(Self::BoardDispatch),
            other => Err(anyhow::anyhow!("unknown execution type: {other}")),
        }
    }
}

// ============================================================================
// Execution Status
// ============================================================================

/// Standard execution status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    #[default]
    Success,
    Error,
    Partial,
    Skipped, // Step skipped due to unmatched conditional edges
}

/// Execution metadata (timing, costs, routing info)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionMetadata {
    pub execution_id: Uuid,
    pub execution_time_ms: u64,
    pub tokens_in: Option<i32>,
    pub tokens_out: Option<i32>,
    pub cost_usd: Option<f64>,
    pub model: Option<String>,
    pub agent_id: Option<Uuid>,

    pub iteration_index: Option<usize>,
    pub iteration_label: Option<String>,
    pub routing_label: Option<String>,
    pub upstream_agent_id: Option<Uuid>,
    pub upstream_routing_label: Option<String>,

    // For room step execution
    pub room_session_id: Option<Uuid>,
    pub room_id: Option<Uuid>,
    pub total_rounds: Option<i32>,

    // For sub-workflow steps
    pub child_workflow_execution_id: Option<Uuid>,
}

impl ExecutionMetadata {
    /// Create metadata with all optional fields set to None.
    pub fn new(execution_id: Uuid) -> Self {
        Self {
            execution_id,
            execution_time_ms: 0,
            tokens_in: None,
            tokens_out: None,
            cost_usd: None,
            model: None,
            agent_id: None,
            iteration_index: None,
            iteration_label: None,
            routing_label: None,
            upstream_agent_id: None,
            upstream_routing_label: None,
            room_session_id: None,
            room_id: None,
            total_rounds: None,
            child_workflow_execution_id: None,
        }
    }
}

impl Default for ExecutionMetadata {
    fn default() -> Self {
        Self::new(Uuid::nil())
    }
}

/// Execution error details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionError {
    pub message: String,
    pub error_type: String,
    pub retryable: bool,
    pub details: Option<serde_json::Value>,
}

/// Standard execution envelope (single step)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StepExecutionEnvelope {
    pub status: ExecutionStatus,
    pub data: Option<serde_json::Value>,
    pub metadata: ExecutionMetadata,
    pub error: Option<ExecutionError>,
}

// ============================================================================
// Downstream Routing Context
// ============================================================================

/// Context about a downstream label-routing step, used to inject
/// routing instructions into the upstream planner's prompt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownstreamRoutingContext {
    pub downstream_step_id: Uuid,
    pub routing_field: String,
    pub routes: Vec<RouteDescription>,
}

/// Description of a single routing rule for prompt injection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteDescription {
    pub label_value: String,
    pub description: Option<String>,
    pub agent_name: String,
    pub agent_tools: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn execution_type_roundtrip() {
        let variants = [
            ExecutionType::Dispatch,
            ExecutionType::ManagerDispatch,
            ExecutionType::DagStep,
            ExecutionType::AgentDesigner,
            ExecutionType::PipelineAgent,
            ExecutionType::InteractiveReview,
            ExecutionType::DebateVerification,
            ExecutionType::BoardDispatch,
        ];
        for variant in variants {
            let s = variant.to_string();
            let parsed: ExecutionType = s.parse().unwrap();
            assert_eq!(parsed, variant, "roundtrip failed for {s}");
            assert_eq!(variant.as_str(), s.as_str());
        }
    }

    #[test]
    fn execution_type_default_is_dag_step() {
        assert_eq!(ExecutionType::default(), ExecutionType::DagStep);
    }

    #[test]
    fn execution_type_serde_roundtrip() {
        let variant = ExecutionType::ManagerDispatch;
        let json = serde_json::to_string(&variant).unwrap();
        assert_eq!(json, "\"manager_dispatch\"");
        let parsed: ExecutionType = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, variant);
    }

    #[test]
    fn execution_type_unknown_returns_error() {
        let result: Result<ExecutionType, _> = "nonexistent".parse();
        assert!(result.is_err());
    }

    #[test]
    fn envelope_serde_roundtrip() {
        let envelope = StepExecutionEnvelope {
            status: ExecutionStatus::Success,
            data: Some(serde_json::json!({"result": "test"})),
            metadata: ExecutionMetadata {
                execution_time_ms: 100,
                tokens_in: Some(10),
                tokens_out: Some(20),
                cost_usd: Some(0.001),
                model: Some("claude-opus-4".into()),
                agent_id: Some(Uuid::new_v4()),
                ..ExecutionMetadata::new(Uuid::new_v4())
            },
            error: None,
        };

        let json = serde_json::to_string(&envelope).unwrap();
        let parsed: StepExecutionEnvelope = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.status, ExecutionStatus::Success);
    }
}
