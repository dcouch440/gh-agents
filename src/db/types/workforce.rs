use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Row type for workforce mission briefs (one per step).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct TaskMissionBriefRow {
    pub id: Uuid,
    pub step_id: Uuid,
    pub task_description: String,
    pub available_capabilities: Vec<String>,
    pub failure_mode: String,
    pub downstream_context: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Row type for task force agent roster entries.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct TaskAgentRosterRow {
    pub id: Uuid,
    pub mission_brief_id: Uuid,
    pub name: String,
    pub role_description: String,
    pub capabilities: Vec<String>,
    pub execution_order: i32,
    pub created_at: DateTime<Utc>,
    /// Corresponding visual step in the workforce child workflow.
    pub child_step_id: Option<Uuid>,
}

/// Row type for agent designer execution runs.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct AgentDesignerRunRow {
    pub id: Uuid,
    pub workflow_execution_id: Uuid,
    pub stage_execution_id: Uuid,
    pub step_id: Uuid,
    pub mission_brief_id: Option<Uuid>,
    pub archetype: String,
    pub phase: String,
    pub model_id: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cost_usd: f32,
    pub created_at: DateTime<Utc>,
}

/// Row type for designer-generated agent prompt outputs.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct AgentDesignerOutputRow {
    pub id: Uuid,
    pub designer_run_id: Uuid,
    pub agent_roster_entry_id: Option<Uuid>,
    pub agent_name: String,
    pub assigned_tools: Vec<String>,
    pub generated_system_prompt: String,
    pub generated_task_prompt: String,
    pub design_reasoning: String,
    pub execution_order: i32,
    pub source_entity_id: String,
    pub source_archetype: String,
    /// Direct link to the protocol execution phase that triggered this designer output.
    pub protocol_execution_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

/// Row type for belief extraction plans (one per step, design-time config).
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct BeliefExtractionPlanRow {
    pub id: Uuid,
    pub step_id: Uuid,
    pub extraction_focus: String,
    pub tag_vocabulary: Vec<String>,
    pub contradiction_handling: String,
    pub confidence_threshold: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Row type for extracted beliefs (populated at runtime by the gatekeeper).
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct BeliefRow {
    pub id: Uuid,
    pub workflow_id: Uuid,
    pub workflow_execution_id: Option<Uuid>,
    pub source_step_id: Uuid,
    pub source_document_title: Option<String>,
    pub source_document_def_id: Option<Uuid>,
    pub source_phase: String,
    pub content: String,
    pub reasoning: String,
    pub belief_type: String,
    pub confidence: String,
    pub confidence_justification: Option<String>,
    pub semantic_tags: Vec<String>,
    pub emotional_tone: Option<String>,
    pub cross_source_tension: Option<String>,
    pub source_step_name: String,
    pub extraction_model: String,
    pub extraction_tokens_in: i32,
    pub extraction_tokens_out: i32,
    pub created_at: DateTime<Utc>,
}

impl Default for TaskMissionBriefRow {
    fn default() -> Self {
        Self {
            id: Uuid::nil(),
            step_id: Uuid::nil(),
            task_description: String::new(),
            available_capabilities: vec![],
            failure_mode: "fail_fast".to_string(),
            downstream_context: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

impl Default for TaskAgentRosterRow {
    fn default() -> Self {
        Self {
            id: Uuid::nil(),
            mission_brief_id: Uuid::nil(),
            name: String::new(),
            role_description: String::new(),
            capabilities: vec![],
            execution_order: 0,
            created_at: Utc::now(),
            child_step_id: None,
        }
    }
}

impl Default for BeliefRow {
    fn default() -> Self {
        Self {
            id: Uuid::nil(),
            workflow_id: Uuid::nil(),
            workflow_execution_id: None,
            source_step_id: Uuid::nil(),
            source_document_title: None,
            source_document_def_id: None,
            source_phase: String::new(),
            content: String::new(),
            reasoning: String::new(),
            belief_type: String::new(),
            confidence: String::new(),
            confidence_justification: None,
            semantic_tags: vec![],
            emotional_tone: None,
            cross_source_tension: None,
            source_step_name: String::new(),
            extraction_model: String::new(),
            extraction_tokens_in: 0,
            extraction_tokens_out: 0,
            created_at: Utc::now(),
        }
    }
}
