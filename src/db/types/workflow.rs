use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Row type for persisted workflow definitions.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct WorkflowRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub description: String,
    pub execution_mode: String,
    pub created_at: DateTime<Utc>,
    pub version: i32,
    pub container_enabled: bool,
    pub target_repo_url: Option<String>,
    pub target_branch: Option<String>,
    pub vpn_enabled: bool,
    pub board_overview_summary: String,
}

/// Row type for a workflow step (DAG node).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct WorkflowStepRow {
    pub id: Uuid,
    pub workflow_id: Uuid,
    pub agent_id: Option<Uuid>,
    pub execution_mode: String, // "single", "workforce", "context", "input", "sub_workflow", "container"
    pub agent_execution_mode: Option<String>, // "sequential" or "parallel", NULL = inherit from workflow
    pub for_each_ref: Option<String>,
    pub prompt_template_id: Option<Uuid>,
    pub prompt_template: String,
    pub output_schema_id: Option<Uuid>,
    pub output_variable_name: Option<String>,
    pub interactive_agent_id: Option<Uuid>,
    pub for_each_label_field: Option<String>,
    pub room_id: Option<Uuid>,
    pub routing_mode: Option<String>,
    pub routing_field: Option<String>,
    pub display_order: i32,
    pub version: i32,
    pub reasoning_trace: bool,
    pub verification_agent_ids: Option<serde_json::Value>,
    pub position_x: Option<f64>,
    pub position_y: Option<f64>,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub name: Option<String>,
    pub system_prompt_suffix: Option<String>,
    pub visible: bool,
    pub description: String,
    // Board context: Haiku-distilled awareness of the workflow board
    pub board_context_cache: String,
    pub board_context_updated_at: Option<DateTime<Utc>>,
    pub goal_summary: String,
    pub goal_summary_updated_at: Option<DateTime<Utc>>,
    /// Template to execute as a child workflow (sub_workflow execution mode).
    pub sub_workflow_template_id: Option<Uuid>,
    /// Live child workflow for workforce steps (edited at design time, snapshotted at execution).
    pub child_workflow_id: Option<Uuid>,
    /// Stable readable identifier for LLM-facing references (e.g. "workforce-1").
    pub ref_id: Option<String>,
    /// Whether this step's output is frozen (replayed instead of re-executed).
    pub pinned: bool,
    /// Haiku-generated summary of this step's last execution output.
    pub run_results_summary: String,
}

/// Row type for a workflow step edge (DAG edge).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct WorkflowStepEdgeRow {
    pub id: Uuid,
    pub from_step_id: Uuid,
    pub to_step_id: Uuid,
    pub from_output_port: Option<String>,
    pub to_input_port: Option<String>,
    pub transform_jsonpath: Option<String>,
    pub condition_type: Option<String>,
    pub condition_value: Option<serde_json::Value>,
    pub edge_label: Option<String>,
    pub workflow_id: Uuid,
}

/// Row type for a step-document attachment.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct StepDocumentRow {
    pub step_id: Uuid,
    pub document_id: Uuid,
}

/// Input port definition for workflow steps
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct StepInputRow {
    pub id: Uuid,
    pub workflow_step_id: Uuid,
    pub port_name: String,
    pub port_type: String,
    pub required: bool,
    pub default_value: Option<serde_json::Value>,
    pub description: Option<String>,
    pub json_schema: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

/// Output port definition for workflow steps
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct StepOutputRow {
    pub id: Uuid,
    pub workflow_step_id: Uuid,
    pub port_name: String,
    pub port_type: String,
    pub json_path: String,
    pub description: Option<String>,
    pub json_schema: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

/// Routing rule for label-based agent assignment
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct StepRoutingRuleRow {
    pub id: Uuid,
    pub workflow_step_id: Uuid,
    pub label_value: String,
    pub description: Option<String>,
    pub agent_id: Uuid,
    pub display_order: i32,
    pub created_at: DateTime<Utc>,
}

/// Row type for step question state (compressed status + pending question).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct StepQuestionStateRow {
    pub step_id: Uuid,
    pub status_text: String,
    pub question_text: Option<String>,
    pub updated_at: DateTime<Utc>,
}

/// Row type for workflow step agents (multi-agent step support).
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct WorkflowStepAgentRow {
    pub step_id: Uuid,
    pub agent_id: Uuid,
    pub execution_strategy: String,
    pub agent_order: i32,
}

impl Default for WorkflowRow {
    fn default() -> Self {
        Self {
            id: Uuid::nil(),
            user_id: Uuid::nil(),
            name: String::new(),
            description: String::new(),
            execution_mode: "dag".to_string(),
            created_at: Utc::now(),
            version: 1,
            container_enabled: false,
            target_repo_url: None,
            target_branch: None,
            vpn_enabled: false,
            board_overview_summary: String::new(),
        }
    }
}

impl Default for WorkflowStepRow {
    fn default() -> Self {
        Self {
            id: Uuid::nil(),
            workflow_id: Uuid::nil(),
            agent_id: None,
            execution_mode: "single".to_string(),
            agent_execution_mode: None,
            for_each_ref: None,
            prompt_template_id: None,
            prompt_template: String::new(),
            output_schema_id: None,
            output_variable_name: None,
            interactive_agent_id: None,
            for_each_label_field: None,
            room_id: None,
            routing_mode: None,
            routing_field: None,
            display_order: 0,
            version: 1,
            reasoning_trace: false,
            verification_agent_ids: None,
            position_x: None,
            position_y: None,
            width: None,
            height: None,
            name: None,
            system_prompt_suffix: None,
            visible: true,
            description: String::new(),
            board_context_cache: String::new(),
            board_context_updated_at: None,
            goal_summary: String::new(),
            goal_summary_updated_at: None,
            sub_workflow_template_id: None,
            child_workflow_id: None,
            ref_id: None,
            pinned: false,
            run_results_summary: String::new(),
        }
    }
}

impl Default for WorkflowStepEdgeRow {
    fn default() -> Self {
        Self {
            id: Uuid::nil(),
            from_step_id: Uuid::nil(),
            to_step_id: Uuid::nil(),
            from_output_port: None,
            to_input_port: None,
            transform_jsonpath: None,
            condition_type: None,
            condition_value: None,
            edge_label: None,
            workflow_id: Uuid::nil(),
        }
    }
}

impl Default for StepInputRow {
    fn default() -> Self {
        Self {
            id: Uuid::nil(),
            workflow_step_id: Uuid::nil(),
            port_name: String::new(),
            port_type: "any".to_string(),
            required: false,
            default_value: None,
            description: None,
            json_schema: None,
            created_at: Utc::now(),
        }
    }
}

impl Default for StepOutputRow {
    fn default() -> Self {
        Self {
            id: Uuid::nil(),
            workflow_step_id: Uuid::nil(),
            port_name: String::new(),
            port_type: "any".to_string(),
            json_path: "$".to_string(),
            description: None,
            json_schema: None,
            created_at: Utc::now(),
        }
    }
}
