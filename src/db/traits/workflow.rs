use anyhow::Result;
use async_trait::async_trait;
use uuid::Uuid;

use crate::db::{
    AgentDesignerOutputRow, AgentDesignerRunRow, BeliefExtractionPlanRow, BeliefRow,
    CanvasElementMapRow, CanvasSnapshotRow, ProtocolDocumentDefRow, RoomStepConfigRow,
    RoomStepMemberRow, RunTemplateRow, StepDocumentRow, StepInputRow, StepOutputRow,
    StepQuestionStateRow, StepRoutingRuleRow, TaskAgentRosterRow, TaskMissionBriefRow, WorkflowRow,
    WorkflowStepEdgeRow, WorkflowStepRow,
};

// ============================================================================
// Workflow Repository
// ============================================================================

/// Input for creating a workflow.
#[derive(Debug, Clone)]
pub struct CreateWorkflowInput {
    pub user_id: Uuid,
    pub name: String,
    pub description: String,
    pub container_enabled: bool,
    pub target_repo_url: Option<String>,
    pub target_branch: Option<String>,
    pub vpn_enabled: bool,
}

/// Input for updating a workflow.
#[derive(Debug, Clone)]
pub struct UpdateWorkflowInput {
    pub id: Uuid,
    pub name: Option<String>,
    pub description: Option<String>,
    pub container_enabled: Option<bool>,
    pub target_repo_url: Option<Option<String>>,
    pub target_branch: Option<Option<String>>,
    pub vpn_enabled: Option<bool>,
}

/// Input for creating a step input port.
#[derive(Debug, Clone)]
pub struct CreateStepInputPort {
    pub workflow_step_id: Uuid,
    pub port_name: String,
    pub port_type: String,
    pub required: bool,
    pub default_value: Option<serde_json::Value>,
    pub description: Option<String>,
    pub json_schema: Option<serde_json::Value>,
}

/// Input for creating a designer output record.
#[derive(Debug, Clone)]
pub struct CreateDesignerOutputInput {
    pub designer_run_id: Uuid,
    pub agent_roster_entry_id: Uuid,
    pub agent_name: String,
    pub assigned_tools: Vec<String>,
    pub generated_system_prompt: String,
    pub generated_task_prompt: String,
    pub design_reasoning: String,
    pub execution_order: i32,
}

/// Input for creating a generic designer output record.
#[derive(Debug, Clone)]
pub struct CreateDesignerOutputGenericInput {
    pub designer_run_id: Uuid,
    pub source_entity_id: String,
    pub source_archetype: String,
    pub agent_name: String,
    pub assigned_tools: Vec<String>,
    pub generated_system_prompt: String,
    pub generated_task_prompt: String,
    pub design_reasoning: String,
    pub execution_order: i32,
    pub protocol_execution_id: Option<Uuid>,
}

/// Database operations for workflows, steps, edges, and step documents.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait WorkflowRepo: Send + Sync {
    // --- Workflows ---
    async fn create_workflow(&self, input: CreateWorkflowInput) -> Result<WorkflowRow>;
    async fn get_workflow(&self, id: Uuid) -> Result<Option<WorkflowRow>>;
    async fn list_workflows(&self, user_id: Uuid) -> Result<Vec<WorkflowRow>>;
    async fn update_workflow(&self, input: UpdateWorkflowInput) -> Result<WorkflowRow>;
    async fn delete_workflow(&self, id: Uuid) -> Result<()>;

    // --- Steps ---
    async fn create_step(&self, step: WorkflowStepRow) -> Result<WorkflowStepRow>;
    async fn get_step(&self, id: Uuid) -> Result<Option<WorkflowStepRow>>;
    async fn find_step_by_ref_id(
        &self,
        workflow_id: Uuid,
        ref_id: &str,
    ) -> Result<Option<WorkflowStepRow>>;
    async fn list_steps(&self, workflow_id: Uuid) -> Result<Vec<WorkflowStepRow>>;
    async fn update_step(&self, step: WorkflowStepRow) -> Result<WorkflowStepRow>;
    async fn delete_step(&self, id: Uuid) -> Result<()>;

    /// Toggle the pinned flag on a step.
    async fn set_step_pinned(&self, step_id: Uuid, pinned: bool) -> Result<()>;

    /// Update the run results summary for a step.
    async fn update_run_results_summary(&self, step_id: Uuid, summary: &str) -> Result<()>;

    /// Get run context for a step: the step's own summary plus summaries of
    /// directly connected steps (upstream + downstream). Returns
    /// `(step_name, run_results_summary, is_pinned)` for each.
    async fn get_run_context_for_step(
        &self,
        workflow_id: Uuid,
        step_id: Uuid,
    ) -> Result<Vec<(String, String, bool)>>;

    // --- Edges ---
    async fn set_edges(&self, workflow_id: Uuid, edges: Vec<WorkflowStepEdgeRow>) -> Result<()>;
    async fn list_edges(&self, workflow_id: Uuid) -> Result<Vec<WorkflowStepEdgeRow>>;
    async fn add_edge(
        &self,
        workflow_id: Uuid,
        from_step_id: Uuid,
        to_step_id: Uuid,
    ) -> Result<WorkflowStepEdgeRow>;
    async fn remove_edge(
        &self,
        from_step_id: Uuid,
        to_step_id: Uuid,
    ) -> Result<WorkflowStepEdgeRow>;
    async fn delete_edge_by_id(&self, edge_id: Uuid) -> Result<WorkflowStepEdgeRow>;

    // --- Step documents ---
    async fn list_step_documents(&self, step_id: Uuid) -> Result<Vec<StepDocumentRow>>;
    async fn add_step_document(&self, step_id: Uuid, document_id: Uuid) -> Result<()>;
    async fn remove_step_document(&self, step_id: Uuid, document_id: Uuid) -> Result<()>;

    // --- Protocol Document Definitions ---

    /// Get a single document definition by ID.
    async fn get_document_def(&self, id: Uuid) -> Result<Option<ProtocolDocumentDefRow>>;

    /// List all document definitions for a workforce step.
    async fn list_document_defs(&self, step_id: Uuid) -> Result<Vec<ProtocolDocumentDefRow>>;

    /// Create a new document definition on a step.
    async fn create_document_def(
        &self,
        def: ProtocolDocumentDefRow,
    ) -> Result<ProtocolDocumentDefRow>;

    /// Update a document definition's name, description, and target length.
    async fn update_document_def(
        &self,
        id: Uuid,
        name: String,
        description: String,
        target_length: i32,
    ) -> Result<ProtocolDocumentDefRow>;

    /// Link a document entity to a document definition.
    async fn link_document_to_def(&self, def_id: Uuid, document_id: Uuid) -> Result<()>;

    /// Delete a document definition.
    async fn delete_document_def(&self, id: Uuid) -> Result<()>;

    // --- Port Management (Phase 3) ---

    /// Get all input ports for a workflow step
    async fn get_step_inputs(&self, workflow_step_id: Uuid) -> Result<Vec<StepInputRow>>;

    /// Get all output ports for a workflow step
    async fn get_step_outputs(&self, workflow_step_id: Uuid) -> Result<Vec<StepOutputRow>>;

    /// Create an input port for a workflow step
    async fn create_step_input(&self, input: CreateStepInputPort) -> Result<StepInputRow>;

    /// Create an output port for a workflow step
    async fn create_step_output(
        &self,
        workflow_step_id: Uuid,
        port_name: &str,
        port_type: &str,
        json_path: &str,
        description: Option<String>,
        json_schema: Option<serde_json::Value>,
    ) -> Result<StepOutputRow>;

    /// Delete an input port
    async fn delete_step_input(&self, id: Uuid) -> Result<()>;

    /// Delete an output port
    async fn delete_step_output(&self, id: Uuid) -> Result<()>;

    // --- Routing Rules (Phase 3) ---

    /// Get all routing rules for a workflow step
    async fn get_step_routing_rules(
        &self,
        workflow_step_id: Uuid,
    ) -> Result<Vec<StepRoutingRuleRow>>;

    /// Create a routing rule for label-based agent assignment
    async fn create_routing_rule(
        &self,
        workflow_step_id: Uuid,
        label_value: &str,
        agent_id: Uuid,
        description: Option<String>,
        display_order: i32,
    ) -> Result<StepRoutingRuleRow>;

    /// Update a routing rule
    async fn update_routing_rule(
        &self,
        id: Uuid,
        agent_id: Option<Uuid>,
        description: Option<String>,
        display_order: Option<i32>,
    ) -> Result<StepRoutingRuleRow>;

    /// Delete a routing rule
    async fn delete_routing_rule(&self, id: Uuid) -> Result<()>;

    /// Find a workflow step by its room_id reference.
    async fn find_step_by_room_id(&self, room_id: Uuid) -> Result<Option<WorkflowStepRow>>;

    // --- Workforce (Mission Briefs + Agent Roster) ---

    /// Get the mission brief for a step, if any.
    async fn get_mission_brief(&self, step_id: Uuid) -> Result<Option<TaskMissionBriefRow>>;

    /// Create or update the mission brief for a step.
    async fn upsert_mission_brief(
        &self,
        step_id: Uuid,
        task_description: &str,
        available_capabilities: &[String],
        failure_mode: &str,
        downstream_context: Option<String>,
    ) -> Result<TaskMissionBriefRow>;

    /// List all agents in a mission brief's roster, ordered by execution_order.
    async fn list_agent_roster(&self, mission_brief_id: Uuid) -> Result<Vec<TaskAgentRosterRow>>;

    /// Add an agent to a mission brief's roster.
    async fn add_roster_agent(
        &self,
        mission_brief_id: Uuid,
        name: &str,
        role_description: &str,
        capabilities: &[String],
        execution_order: i32,
    ) -> Result<TaskAgentRosterRow>;

    /// Update a roster agent's fields. Only provided fields are changed.
    async fn update_roster_agent(
        &self,
        agent_id: Uuid,
        name: Option<String>,
        role_description: Option<String>,
        capabilities: Option<Vec<String>>,
    ) -> Result<TaskAgentRosterRow>;

    /// Remove a roster agent by ID.
    async fn remove_roster_agent(&self, agent_id: Uuid) -> Result<()>;

    /// Update execution_order on a roster agent (used by topology recomputation).
    async fn update_roster_agent_order(&self, agent_id: Uuid, execution_order: i32) -> Result<()>;

    /// Link a roster agent to its corresponding child workflow step.
    async fn link_roster_agent_to_child_step(
        &self,
        agent_id: Uuid,
        child_step_id: Option<Uuid>,
    ) -> Result<()>;

    // --- Belief Capture (Extraction Plans) ---

    /// Get the extraction plan for a step, if any.
    async fn get_extraction_plan(&self, step_id: Uuid) -> Result<Option<BeliefExtractionPlanRow>>;

    /// Create or update the extraction plan for a step.
    async fn upsert_extraction_plan(
        &self,
        step_id: Uuid,
        extraction_focus: &str,
        tag_vocabulary: &[String],
        contradiction_handling: &str,
        confidence_threshold: &str,
    ) -> Result<BeliefExtractionPlanRow>;

    // --- Belief Capture (Runtime Beliefs) ---

    /// Insert a single extracted belief.
    async fn insert_belief(&self, belief: &BeliefRow) -> Result<BeliefRow>;

    /// List all beliefs for a specific workflow execution run.
    async fn list_beliefs_for_execution(
        &self,
        workflow_execution_id: Uuid,
    ) -> Result<Vec<BeliefRow>>;

    // --- Chat Beliefs ---

    /// Delete all chat-phase beliefs for a step, then insert replacements.
    async fn replace_chat_beliefs(
        &self,
        step_id: Uuid,
        beliefs: &[BeliefRow],
    ) -> Result<Vec<BeliefRow>>;

    /// Load chat-phase beliefs for all steps connected to a given step via edges.
    async fn get_beliefs_for_connected_steps(
        &self,
        workflow_id: Uuid,
        step_id: Uuid,
    ) -> Result<Vec<BeliefRow>>;

    // --- Room Step Config (Design-Time) ---

    async fn get_room_step_config(&self, step_id: Uuid) -> Result<Option<RoomStepConfigRow>>;

    async fn upsert_room_step_config(
        &self,
        step_id: Uuid,
        meeting_purpose: &str,
        max_turns: i32,
        interaction_mode: &str,
        gatekeeper_enabled: bool,
    ) -> Result<RoomStepConfigRow>;

    async fn list_room_step_members(&self, step_id: Uuid) -> Result<Vec<RoomStepMemberRow>>;

    async fn add_room_step_member(
        &self,
        step_id: Uuid,
        name: &str,
        role: &str,
        perspective: &str,
        display_order: i32,
    ) -> Result<RoomStepMemberRow>;

    async fn update_room_step_member(
        &self,
        member_id: Uuid,
        name: Option<String>,
        role: Option<String>,
        perspective: Option<String>,
    ) -> Result<RoomStepMemberRow>;

    async fn remove_room_step_member(&self, member_id: Uuid) -> Result<()>;

    // --- Agent Designer ---

    /// Create a new agent designer run record for token tracking (task-force-specific).
    async fn create_designer_run(
        &self,
        workflow_execution_id: Uuid,
        stage_execution_id: Uuid,
        step_id: Uuid,
        mission_brief_id: Uuid,
        model_id: &str,
    ) -> Result<AgentDesignerRunRow>;

    /// Create a designer run record for any archetype.
    async fn create_designer_run_generic(
        &self,
        workflow_execution_id: Uuid,
        stage_execution_id: Uuid,
        step_id: Uuid,
        archetype: &str,
        phase: &str,
        model_id: &str,
    ) -> Result<AgentDesignerRunRow>;

    /// Update designer run with token usage after completion.
    async fn update_designer_run_tokens(
        &self,
        run_id: Uuid,
        input_tokens: i64,
        output_tokens: i64,
        cost_usd: f32,
    ) -> Result<()>;

    /// Store a designer-generated prompt pair and tool assignment for one agent (task-force-specific).
    async fn create_designer_output(
        &self,
        input: CreateDesignerOutputInput,
    ) -> Result<AgentDesignerOutputRow>;

    /// Store a designer-generated prompt pair for any archetype.
    async fn create_designer_output_generic(
        &self,
        input: CreateDesignerOutputGenericInput,
    ) -> Result<AgentDesignerOutputRow>;

    /// List all designer outputs for a run, ordered by execution_order.
    async fn list_designer_outputs(
        &self,
        designer_run_id: Uuid,
    ) -> Result<Vec<AgentDesignerOutputRow>>;

    /// List designer outputs linked to a specific protocol execution phase.
    async fn list_designer_outputs_by_protocol_execution(
        &self,
        protocol_execution_id: Uuid,
    ) -> Result<Vec<AgentDesignerOutputRow>>;

    /// List designer runs for a step within a specific workflow execution.
    async fn list_designer_runs_for_step(
        &self,
        step_id: Uuid,
        workflow_execution_id: Uuid,
    ) -> Result<Vec<AgentDesignerRunRow>>;

    // --- Step Plan ---

    /// Get a single step's plan content. Returns None if no plan exists.
    async fn get_plan(&self, step_id: Uuid) -> Result<Option<String>>;

    /// Create or replace a step's plan (full replacement).
    async fn upsert_plan(&self, step_id: Uuid, content: &str) -> Result<()>;

    /// Get all plans across a workflow (for board overview summarizer).
    /// Returns Vec<(step_id, step_name, execution_mode, plan_content)>.
    async fn get_all_plans_for_workflow(
        &self,
        workflow_id: Uuid,
    ) -> Result<Vec<(Uuid, Option<String>, String, String)>>;

    // --- Board Overview Summary ---

    /// Get the board overview summary for a workflow.
    async fn get_board_overview_summary(&self, workflow_id: Uuid) -> Result<String>;

    /// Update the board overview summary for a workflow.
    async fn update_board_overview_summary(&self, workflow_id: Uuid, summary: &str) -> Result<()>;

    // --- Designer Handoff ---

    /// Update the designer handoff description for a step.
    async fn update_designer_handoff(&self, step_id: Uuid, handoff: &str) -> Result<()>;

    // --- Step Question State ---

    /// Get compressed status + pending question for a step.
    async fn get_step_question_state(&self, step_id: Uuid) -> Result<Option<StepQuestionStateRow>>;

    /// Batch-load question state for multiple steps (board fetch path).
    async fn get_step_question_states(
        &self,
        step_ids: &[Uuid],
    ) -> Result<Vec<StepQuestionStateRow>>;

    /// Create or replace a step's compressed status + question.
    async fn upsert_step_question_state(
        &self,
        step_id: Uuid,
        status_text: &str,
        question_text: Option<String>,
    ) -> Result<()>;

    // --- Run Templates ---

    /// Create a run template (frozen workflow snapshot).
    async fn create_template(
        &self,
        workflow_id: Uuid,
        user_id: Uuid,
        name: &str,
        description: Option<String>,
        snapshot: serde_json::Value,
    ) -> Result<RunTemplateRow>;

    /// Get a run template by ID.
    async fn get_template(&self, template_id: Uuid) -> Result<Option<RunTemplateRow>>;

    /// List all run templates for a workflow (newest first, without snapshot blob).
    async fn list_templates(&self, workflow_id: Uuid) -> Result<Vec<RunTemplateRow>>;

    /// Delete a run template.
    async fn delete_template(&self, template_id: Uuid) -> Result<()>;

    // --- Canvas Snapshots ---

    /// Get the latest canvas snapshot for a workflow (for diffing on next submit).
    async fn get_canvas_snapshot(&self, workflow_id: Uuid) -> Result<Option<CanvasSnapshotRow>>;

    /// Create or replace the canvas snapshot for a workflow.
    async fn upsert_canvas_snapshot(&self, row: CanvasSnapshotRow) -> Result<CanvasSnapshotRow>;

    /// Update the last board submit response JSON for debug panel rehydration.
    async fn update_canvas_snapshot_response(
        &self,
        workflow_id: Uuid,
        response_json: String,
    ) -> Result<()>;

    // --- Canvas Element Maps ---

    /// Load all element→step/edge mappings for a workflow.
    async fn list_element_maps(&self, workflow_id: Uuid) -> Result<Vec<CanvasElementMapRow>>;

    /// Create or update an element mapping (element_id → step_id or edge_id).
    async fn upsert_element_map(&self, row: CanvasElementMapRow) -> Result<CanvasElementMapRow>;

    /// Remove an element mapping.
    async fn delete_element_map(&self, workflow_id: Uuid, element_id: &str) -> Result<()>;

    // --- Step Images ---

    /// Store or update the pre-rendered stroke PNG for a step.
    async fn upsert_step_image(&self, step_id: Uuid, stroke_image_base64: &str) -> Result<()>;

    /// Load the pre-rendered stroke PNG for a step. Returns None if no image exists.
    async fn get_step_stroke_image(&self, step_id: Uuid) -> Result<Option<String>>;
}
