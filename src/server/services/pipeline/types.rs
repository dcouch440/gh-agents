//! Types for the pipeline service layer.
//!
//! These types define the interface between callers (workforce tools,
//! protocol apply, future pipeline creators) and the pipeline service.
//! They are deliberately decoupled from DB row types — the service
//! handles the mapping internally.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Identifies the parent step that owns a pipeline.
///
/// All pipeline service functions take this as context to locate the
/// child workflow and validate ownership.
#[derive(Debug, Clone)]
pub struct PipelineContext {
    pub parent_step_id: Uuid,
    pub parent_workflow_id: Uuid,
}

/// Input for adding a step to a pipeline.
#[derive(Debug, Clone)]
pub struct AddStepInput {
    pub name: String,
    pub description: String,
    pub execution_mode: String,
    pub agent_id: Option<Uuid>,
    pub prompt_template: Option<String>,
    pub output_variable_name: Option<String>,
    pub display_order: Option<i32>,
}

/// Input for updating an existing pipeline step.
#[derive(Debug, Clone, Default)]
pub struct UpdateStepInput {
    pub name: Option<String>,
    pub description: Option<String>,
    pub execution_mode: Option<String>,
    pub prompt_template: Option<String>,
}

/// Result of creating a pipeline.
#[derive(Debug, Clone)]
pub struct PipelineCreated {
    /// The child workflow ID (the pipeline itself).
    pub pipeline_id: Uuid,
    /// The auto-managed Designer step, if requested.
    pub designer_step_id: Option<Uuid>,
}

/// Result of adding a step to the pipeline.
#[derive(Debug, Clone)]
pub struct StepAdded {
    pub step_id: Uuid,
    pub name: String,
    pub display_order: i32,
}

/// Entry in the computed execution sequence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionOrderEntry {
    pub step_id: Uuid,
    pub name: String,
    pub order: i32,
}
