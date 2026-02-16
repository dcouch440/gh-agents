//! Repository traits for database operations.
//!
//! Each trait abstracts the DB operations for a specific domain module.
//! Production code uses `PgRepo` (see `pg_repo.rs`). Tests use `MockXxxRepo` from mockall.

use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::db::{
    AgentDesignerOutputRow, AgentDesignerRunRow, AgentExecutionRow, AgentGuidanceRow, AgentRow,
    BeliefExtractionPlanRow, BeliefRow, ChatMessageRow, CollectionRunRow,
    CollectionWorkflowEdgeRow, CollectionWorkflowRow, ContentVersionRow, ContextStoreRow,
    DocumentRow, DocumentSearchResult, EnvelopeSnapshotRow, ExecutionMessageRow, OutputSchemaRow,
    PromptTemplateRow, ProtocolDocumentDefRow, ProtocolExecutionRow, ProtocolPortRow, ProtocolRow,
    ResultRow, RoomExecutionOutputRow, RoomMemberRow, RoomRow, RoomSessionRow, RoomStepConfigRow,
    RoomStepMemberRow, RoomTranscriptEntry, RouterRequestRow, RunSnapshotRow, RunTemplateRow,
    SessionRow, StepDocumentRow, StepInputRow, StepOutputRow, StepRoutingRuleRow, SystemConfigRow,
    TaskAgentRosterRow, TaskMissionBriefRow, TokenLedgerRow, ToolCapabilityRow, ToolRouterModeRow,
    ToolRouterRow, ToolRow, WorkflowCollectionRow, WorkflowExecutionRow, WorkflowRow,
    WorkflowStepAgentRow, WorkflowStepEdgeRow, WorkflowStepProtocolRow, WorkflowStepRow,
};
use crate::github::{PrQueueEntry, QueueError as MergeQueueError};
use crate::types::{Task, User, UserId};

// ============================================================================
// Merge Queue Repository
// ============================================================================

/// Database operations for the PR merge queue.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait MergeQueueRepo: Send + Sync {
    /// Insert or update a queue entry (upsert).
    async fn insert_queue_entry(
        &self,
        id: Uuid,
        owner: String,
        repo: String,
        pr_number: u32,
        position: u32,
        now: DateTime<Utc>,
        user_id: Uuid,
    ) -> Result<(), MergeQueueError>;

    /// Get the next queue position for a repo.
    async fn get_next_position(&self, owner: String, repo: String) -> Result<u32, MergeQueueError>;

    /// Delete a queue entry. Returns true if a row was deleted.
    async fn delete_queue_entry(
        &self,
        owner: String,
        repo: String,
        pr_number: u32,
    ) -> Result<bool, MergeQueueError>;

    /// Get all queue entries for a repo, ordered by position.
    async fn get_queue_entries(
        &self,
        owner: String,
        repo: String,
    ) -> Result<Vec<PrQueueEntry>, MergeQueueError>;

    /// Update the status (and optional error message) of a queue entry.
    async fn update_entry_status(
        &self,
        owner: String,
        repo: String,
        pr_number: u32,
        status: String,
        error_message: Option<String>,
        now: DateTime<Utc>,
    ) -> Result<bool, MergeQueueError>;

    /// Set conflict info on a queue entry.
    async fn set_entry_conflict(
        &self,
        owner: String,
        repo: String,
        pr_number: u32,
        conflict_json: String,
        now: DateTime<Utc>,
    ) -> Result<bool, MergeQueueError>;

    /// Update the position of a queue entry by ID.
    async fn update_entry_position(
        &self,
        id: Uuid,
        position: u32,
        now: DateTime<Utc>,
    ) -> Result<(), MergeQueueError>;

    /// Reset in_progress entries back to pending.
    async fn reset_interrupted(
        &self,
        owner: String,
        repo: String,
        now: DateTime<Utc>,
    ) -> Result<u32, MergeQueueError>;

    /// Delete merged/skipped entries older than cutoff.
    async fn cleanup_old(
        &self,
        owner: String,
        repo: String,
        cutoff: DateTime<Utc>,
    ) -> Result<u32, MergeQueueError>;
}

// ============================================================================
// Server Repository (API handler DB operations)
// ============================================================================

/// Database operations used by the HTTP API handlers.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait ServerRepo: Send + Sync {
    /// Check database connectivity (returns true if alive).
    async fn health_check(&self) -> bool;

    /// List tasks with optional status filter and limit.
    async fn list_tasks(
        &self,
        user_id: UserId,
        status: Option<String>,
        limit: Option<u32>,
    ) -> Result<Vec<Task>>;

    /// Get a single task by UUID.
    async fn get_task_by_uuid(&self, user_id: UserId, id: Uuid) -> Result<Option<Task>>;

    /// Insert a new task.
    async fn insert_task(&self, user_id: UserId, task: Task) -> Result<()>;

    /// Insert a chat message.
    async fn insert_chat_message(
        &self,
        user_id: UserId,
        id: Uuid,
        role: String,
        content: String,
    ) -> Result<()>;

    /// Get chat history with pagination.
    async fn get_chat_history(
        &self,
        user_id: UserId,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<ChatMessageRow>>;

    /// Clear all chat history.
    async fn clear_chat_history(&self, user_id: UserId) -> Result<()>;

    /// Check if a password has been configured.
    async fn has_password(&self) -> Result<bool>;

    /// Store the password hash.
    async fn set_password(&self, password_hash: String) -> Result<()>;

    /// Get the stored password hash.
    async fn get_password(&self) -> Result<Option<String>>;

    // --- Agent persistence ---

    /// List all agents for a user.
    async fn list_persisted_agents(&self, user_id: UserId) -> Result<Vec<AgentRow>>;

    /// Insert or update an agent definition.
    /// For user-owned agents, pass agent.user_id = Some(user_id).
    /// For system agents, pass agent.user_id = None.
    async fn upsert_agent(&self, agent: AgentRow) -> Result<()>;

    /// Get a single agent by ID.
    async fn get_persisted_agent(&self, agent_id: Uuid) -> Result<Option<AgentRow>>;

    /// Delete an agent by ID.
    async fn delete_persisted_agent(&self, agent_id: Uuid) -> Result<()>;

    // --- Tool persistence ---

    /// List all tools (system-wide).
    async fn list_tools(&self) -> Result<Vec<ToolRow>>;

    /// Get a tool by ID.
    async fn get_tool(&self, tool_id: Uuid) -> Result<Option<ToolRow>>;

    /// Insert or update a tool (system-wide).
    async fn upsert_tool(&self, tool: ToolRow) -> Result<()>;

    /// Delete a tool by ID.
    async fn delete_tool(&self, tool_id: Uuid) -> Result<()>;

    /// Get all tools assigned to an agent.
    async fn get_agent_tools(&self, agent_id: Uuid) -> Result<Vec<ToolRow>>;

    /// Set the full tool list for an agent (replaces existing).
    async fn set_agent_tools(&self, agent_id: Uuid, tool_ids: Vec<Uuid>) -> Result<()>;

    /// Seed the built-in execution tools (system-wide). Idempotent.
    async fn seed_builtin_tools(&self) -> Result<()>;

    // --- Agent context (document linkage) ---

    /// Get all context documents assigned to an agent.
    async fn get_agent_context(&self, agent_id: Uuid) -> Result<Vec<DocumentRow>>;

    /// Set the full context document list for an agent (replaces existing).
    async fn set_agent_context(&self, agent_id: Uuid, document_ids: Vec<Uuid>) -> Result<()>;

    // --- Session management ---

    /// Create a new chat session.
    async fn create_session(
        &self,
        user_id: UserId,
        session_id: Uuid,
        mode_id: &str,
        title: &str,
        agent_id: Option<Uuid>,
        draft_config: Option<serde_json::Value>,
    ) -> Result<()>;

    /// List sessions for a user.
    async fn list_sessions(&self, user_id: UserId) -> Result<Vec<SessionRow>>;

    /// Get a session by ID.
    async fn get_session(&self, session_id: Uuid) -> Result<Option<SessionRow>>;

    /// Delete a session and its messages.
    async fn delete_session(&self, session_id: Uuid) -> Result<()>;

    /// Insert a chat message scoped to a session.
    async fn insert_session_message(
        &self,
        user_id: UserId,
        session_id: Uuid,
        id: Uuid,
        role: String,
        content: String,
    ) -> Result<()>;

    /// Get chat history for a session.
    async fn get_session_history(
        &self,
        session_id: Uuid,
        limit: u32,
    ) -> Result<Vec<ChatMessageRow>>;

    /// Update the title for a session.
    async fn update_session_title(&self, session_id: Uuid, title: &str) -> Result<()>;

    /// Update the summary for a session.
    async fn update_session_summary(&self, session_id: Uuid, summary: &str) -> Result<()>;

    /// Count messages in a session.
    async fn count_session_messages(&self, session_id: Uuid) -> Result<u32>;

    /// Update draft_config for a session.
    async fn update_session_draft_config(
        &self,
        session_id: Uuid,
        draft_config: Option<serde_json::Value>,
    ) -> Result<()>;

    /// Clear all messages for a session.
    async fn clear_session_messages(&self, session_id: Uuid) -> Result<()>;

    /// Find a chat session linked to a workflow step via draft_config.
    async fn find_session_by_step_id(&self, step_id: Uuid) -> Result<Option<SessionRow>>;

    /// Link an agent to a session (and clear draft_config).
    async fn link_session_agent(&self, session_id: Uuid, agent_id: Uuid) -> Result<()>;

    // --- Agent guidance ---

    /// Load active guidances for an agent, optionally filtered by step.
    /// Returns global (step_id=NULL) + step-specific guidances, stacked.
    async fn get_agent_guidances(
        &self,
        agent_id: Uuid,
        step_id: Option<Uuid>,
    ) -> Result<Vec<AgentGuidanceRow>>;
}

// ============================================================================
// User Repository
// ============================================================================

/// Database operations for user management.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait UserRepo: Send + Sync {
    /// Create a new user with email and password.
    async fn create_user(&self, email: &str, password_hash: &str) -> Result<User>;
    /// Get a user by email.
    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>>;
    /// Get a user by ID.
    async fn get_user_by_id(&self, id: UserId) -> Result<Option<User>>;
    /// Get a user by GitHub ID.
    async fn get_user_by_github_id(&self, github_id: i64) -> Result<Option<User>>;
    /// Link GitHub account to existing user.
    async fn link_github(
        &self,
        user_id: UserId,
        github_id: i64,
        github_login: &str,
        token_encrypted: &str,
    ) -> Result<()>;
    /// Create a new user from GitHub OAuth.
    async fn create_github_user(
        &self,
        email: &str,
        github_id: i64,
        github_login: &str,
        token_encrypted: &str,
    ) -> Result<User>;
}

// ============================================================================
// Document Repository
// ============================================================================

/// Database operations for document management.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait DocumentRepo: Send + Sync {
    /// Create a new document.
    async fn create_document(
        &self,
        user_id: Uuid,
        session_id: Option<Uuid>,
        title: String,
        content: String,
        doc_type: String,
        ref_tag: String,
        tags: Vec<String>,
    ) -> Result<DocumentRow>;

    /// Create a blank document linked to a workflow for protocol-generated content.
    ///
    /// Sets `workflow_id`, `target_length`, `source_protocol_step_id`, and `is_static = false`.
    /// Content starts empty and is populated by the DocumenterExecutor at runtime.
    async fn create_workflow_document(
        &self,
        user_id: Uuid,
        title: String,
        workflow_id: Uuid,
        target_length: Option<i32>,
        source_protocol_step_id: Option<Uuid>,
    ) -> Result<DocumentRow>;

    /// Update a document's content, title, and tags.
    async fn update_document(
        &self,
        doc_id: Uuid,
        content: Option<String>,
        title: Option<String>,
        tags: Option<Vec<String>>,
    ) -> Result<DocumentRow>;

    /// Update a document's summary.
    async fn update_document_summary(&self, doc_id: Uuid, summary: String) -> Result<()>;

    /// Get a document by ID.
    async fn get_document(&self, doc_id: Uuid) -> Result<Option<DocumentRow>>;

    /// Get a document by ref_tag.
    async fn get_document_by_ref_tag(&self, ref_tag: &str) -> Result<Option<DocumentRow>>;

    /// List all documents for a user.
    async fn list_documents(&self, user_id: Uuid) -> Result<Vec<DocumentRow>>;

    /// List all documents for a session.
    async fn list_session_documents(&self, session_id: Uuid) -> Result<Vec<DocumentRow>>;

    /// Full-text search documents for a user.
    async fn search_documents(
        &self,
        user_id: Uuid,
        query: &str,
    ) -> Result<Vec<DocumentSearchResult>>;

    /// Delete a document by ID.
    async fn delete_document(&self, doc_id: Uuid) -> Result<()>;
}

// ============================================================================
// Output Schema Repository
// ============================================================================

/// Database operations for output schema management.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait OutputSchemaRepo: Send + Sync {
    /// Create a new output schema.
    async fn create_output_schema(
        &self,
        user_id: Option<Uuid>,
        name: String,
        schema: serde_json::Value,
    ) -> Result<OutputSchemaRow>;

    /// Get an output schema by ID.
    async fn get_output_schema(&self, id: Uuid) -> Result<Option<OutputSchemaRow>>;

    /// List all output schemas for a user.
    async fn list_output_schemas(&self, user_id: Uuid) -> Result<Vec<OutputSchemaRow>>;

    /// Update an output schema's name and/or schema.
    async fn update_output_schema(
        &self,
        id: Uuid,
        name: Option<String>,
        schema: Option<serde_json::Value>,
    ) -> Result<OutputSchemaRow>;

    /// Delete an output schema by ID.
    async fn delete_output_schema(&self, id: Uuid) -> Result<()>;
}

// ============================================================================
// Prompt Template Repository
// ============================================================================

/// Database operations for prompt template management.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait PromptTemplateRepo: Send + Sync {
    /// Create a new prompt template.
    async fn create_prompt_template(
        &self,
        user_id: Option<Uuid>,
        name: String,
        content: String,
    ) -> Result<PromptTemplateRow>;

    /// Get a prompt template by ID.
    async fn get_prompt_template(&self, id: Uuid) -> Result<Option<PromptTemplateRow>>;

    /// List all prompt templates for a user.
    async fn list_prompt_templates(&self, user_id: Uuid) -> Result<Vec<PromptTemplateRow>>;

    /// Update a prompt template's name and/or content.
    async fn update_prompt_template(
        &self,
        id: Uuid,
        name: Option<String>,
        content: Option<String>,
    ) -> Result<PromptTemplateRow>;

    /// Delete a prompt template by ID.
    async fn delete_prompt_template(&self, id: Uuid) -> Result<()>;
}

// ============================================================================
// Workflow Repository
// ============================================================================

/// Database operations for workflows, steps, edges, and step documents.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait WorkflowRepo: Send + Sync {
    // --- Workflows ---
    async fn create_workflow(
        &self,
        user_id: Uuid,
        name: String,
        description: String,
        container_enabled: bool,
        target_repo_url: Option<String>,
        target_branch: Option<String>,
        vpn_enabled: bool,
    ) -> Result<WorkflowRow>;
    async fn get_workflow(&self, id: Uuid) -> Result<Option<WorkflowRow>>;
    async fn list_workflows(&self, user_id: Uuid) -> Result<Vec<WorkflowRow>>;
    async fn update_workflow(
        &self,
        id: Uuid,
        name: Option<String>,
        description: Option<String>,
        container_enabled: Option<bool>,
        target_repo_url: Option<Option<String>>,
        target_branch: Option<Option<String>>,
        vpn_enabled: Option<bool>,
    ) -> Result<WorkflowRow>;
    async fn delete_workflow(&self, id: Uuid) -> Result<()>;

    // --- Steps ---
    async fn create_step(&self, step: WorkflowStepRow) -> Result<WorkflowStepRow>;
    async fn get_step(&self, id: Uuid) -> Result<Option<WorkflowStepRow>>;
    async fn list_steps(&self, workflow_id: Uuid) -> Result<Vec<WorkflowStepRow>>;
    async fn update_step(&self, step: WorkflowStepRow) -> Result<WorkflowStepRow>;
    async fn delete_step(&self, id: Uuid) -> Result<()>;

    // --- Edges ---
    async fn set_edges(&self, workflow_id: Uuid, edges: Vec<WorkflowStepEdgeRow>) -> Result<()>;
    async fn list_edges(&self, workflow_id: Uuid) -> Result<Vec<WorkflowStepEdgeRow>>;
    async fn add_edge(
        &self,
        workflow_id: Uuid,
        from_step_id: Uuid,
        to_step_id: Uuid,
    ) -> Result<WorkflowStepEdgeRow>;
    async fn remove_edge(&self, from_step_id: Uuid, to_step_id: Uuid) -> Result<()>;
    async fn delete_edge_by_id(&self, edge_id: Uuid) -> Result<()>;

    // --- Step documents ---
    async fn list_step_documents(&self, step_id: Uuid) -> Result<Vec<StepDocumentRow>>;
    async fn add_step_document(&self, step_id: Uuid, document_id: Uuid) -> Result<()>;
    async fn remove_step_document(&self, step_id: Uuid, document_id: Uuid) -> Result<()>;

    // --- Protocol Document Definitions ---

    /// Get a single document definition by ID.
    async fn get_document_def(&self, id: Uuid) -> Result<Option<ProtocolDocumentDefRow>>;

    /// List all document definitions for a documenter step.
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
    async fn create_step_input(
        &self,
        workflow_step_id: Uuid,
        port_name: &str,
        port_type: &str,
        required: bool,
        default_value: Option<serde_json::Value>,
        description: Option<String>,
        json_schema: Option<serde_json::Value>,
    ) -> Result<StepInputRow>;

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

    // --- Task Force (Mission Briefs + Agent Roster) ---

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
        designer_run_id: Uuid,
        agent_roster_entry_id: Uuid,
        agent_name: &str,
        assigned_tools: &[String],
        generated_system_prompt: &str,
        generated_task_prompt: &str,
        design_reasoning: &str,
        execution_order: i32,
    ) -> Result<AgentDesignerOutputRow>;

    /// Store a designer-generated prompt pair for any archetype.
    async fn create_designer_output_generic(
        &self,
        designer_run_id: Uuid,
        source_entity_id: &str,
        source_archetype: &str,
        agent_name: &str,
        assigned_tools: &[String],
        generated_system_prompt: &str,
        generated_task_prompt: &str,
        design_reasoning: &str,
        execution_order: i32,
        protocol_execution_id: Option<Uuid>,
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

    // --- Assistant Notes ---

    /// Get a single step's assistant notes content. Returns None if no notes exist.
    async fn get_assistant_notes(&self, step_id: Uuid) -> Result<Option<String>>;

    /// Create or replace a step's assistant notes (full replacement).
    async fn upsert_assistant_notes(&self, step_id: Uuid, content: &str) -> Result<()>;

    /// Get all assistant notes across a workflow (for board overview summarizer).
    /// Returns Vec<(step_id, step_name, execution_mode, notes_content)>.
    async fn get_all_assistant_notes_for_workflow(
        &self,
        workflow_id: Uuid,
    ) -> Result<Vec<(Uuid, Option<String>, String, String)>>;

    // --- Board Overview Summary ---

    /// Get the board overview summary for a workflow.
    async fn get_board_overview_summary(&self, workflow_id: Uuid) -> Result<String>;

    /// Update the board overview summary for a workflow.
    async fn update_board_overview_summary(&self, workflow_id: Uuid, summary: &str) -> Result<()>;

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
}

// ============================================================================
// Agent Execution Repository
// ============================================================================

/// Database operations for agent executions and execution messages.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait AgentExecutionRepo: Send + Sync {
    // --- Agent Executions ---
    async fn create_agent_execution(
        &self,
        agent_id: Uuid,
        workflow_step_id: Option<Uuid>,
        is_interactive: bool,
        parent_agent_execution_id: Option<Uuid>,
        system_prompt_rendered: &str,
        input: &str,
        room_session_id: Option<Uuid>,
        speaker_order: Option<i32>,
        workflow_execution_id: Option<Uuid>,
    ) -> Result<AgentExecutionRow>;
    async fn get_agent_execution(&self, id: Uuid) -> Result<Option<AgentExecutionRow>>;
    async fn update_agent_execution_status(
        &self,
        id: Uuid,
        status: &str,
        output: Option<String>,
        structured_output: Option<serde_json::Value>,
    ) -> Result<AgentExecutionRow>;

    // --- Execution Messages ---
    async fn create_execution_message(
        &self,
        agent_execution_id: Uuid,
        role: &str,
        content: &str,
        tool_call_id: Option<String>,
        input_tokens: i64,
        output_tokens: i64,
    ) -> Result<ExecutionMessageRow>;
    async fn list_execution_messages(
        &self,
        agent_execution_id: Uuid,
    ) -> Result<Vec<ExecutionMessageRow>>;

    /// List completed non-interactive agent executions for a set of workflow step IDs.
    /// Used to reconstruct DAG state on resume.
    async fn list_completed_executions_for_step_ids(
        &self,
        workflow_step_ids: &[Uuid],
    ) -> Result<Vec<AgentExecutionRow>>;

    /// List interactive agent executions for a specific workflow step.
    /// Used to check if all interactive reviews are approved before resuming.
    async fn list_interactive_executions_for_step(
        &self,
        workflow_step_id: Uuid,
    ) -> Result<Vec<AgentExecutionRow>>;

    /// List interactive agent executions for a user, optionally filtered by status.
    /// Joins through workflow_executions to filter by user_id.
    async fn list_agent_executions(
        &self,
        user_id: Uuid,
        status: Option<String>,
    ) -> Result<Vec<AgentExecutionRow>>;

    /// List agent executions for a specific step within a specific workflow run.
    async fn list_agent_executions_for_step_and_run(
        &self,
        workflow_step_id: Uuid,
        workflow_execution_id: Uuid,
    ) -> Result<Vec<AgentExecutionRow>>;

    /// List completed executions marked as exemplary for few-shot injection.
    /// Returns rows ordered by most recent, limited to `limit`.
    async fn list_exemplary_executions(
        &self,
        agent_id: Uuid,
        workflow_step_id: Option<Uuid>,
        limit: u32,
    ) -> Result<Vec<AgentExecutionRow>>;

    /// Toggle the exemplary flag on an execution.
    async fn set_execution_exemplary(
        &self,
        id: Uuid,
        is_exemplary: bool,
    ) -> Result<AgentExecutionRow>;
}

// ============================================================================
// Token Ledger Repository
// ============================================================================

/// Aggregated spend by model.
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow, utoipa::ToSchema)]
pub struct ModelSpendRow {
    pub model_id: String,
    pub total_input_tokens: i64,
    pub total_output_tokens: i64,
    pub total_cost_usd: f64,
    pub call_count: i64,
}

/// Database operations for token cost tracking.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait TokenLedgerRepo: Send + Sync {
    async fn insert_ledger_entry(
        &self,
        user_id: Uuid,
        agent_execution_id: Option<Uuid>,
        model_id: &str,
        input_tokens: i64,
        output_tokens: i64,
        cost_usd: f32,
    ) -> Result<TokenLedgerRow>;
    async fn get_user_spend(&self, user_id: Uuid, since: Option<DateTime<Utc>>) -> Result<f64>;
    async fn get_model_breakdown(
        &self,
        user_id: Uuid,
        since: Option<DateTime<Utc>>,
    ) -> Result<Vec<ModelSpendRow>>;
}

// ============================================================================
// Result Repository
// ============================================================================

/// Database operations for saved structured results.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait ResultRepo: Send + Sync {
    async fn save_result(
        &self,
        user_id: Uuid,
        agent_execution_id: Uuid,
        output_schema_id: Option<Uuid>,
        name: &str,
        data: serde_json::Value,
    ) -> Result<ResultRow>;
    async fn get_result(&self, id: Uuid) -> Result<Option<ResultRow>>;
    async fn list_results(&self, user_id: Uuid) -> Result<Vec<ResultRow>>;
    async fn list_results_by_schema(
        &self,
        user_id: Uuid,
        output_schema_id: Uuid,
    ) -> Result<Vec<ResultRow>>;
    async fn delete_result(&self, id: Uuid) -> Result<()>;
}

// ============================================================================
// Tool Router Repository
// ============================================================================

/// Database operations for tool router management.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait ToolRouterRepo: Send + Sync {
    /// List all tool routers for a user.
    async fn list_tool_routers(&self, user_id: Uuid) -> Result<Vec<ToolRouterRow>>;
    /// Get a tool router by ID.
    async fn get_tool_router(&self, id: Uuid) -> Result<Option<ToolRouterRow>>;
    /// Create a new tool router.
    async fn create_tool_router(
        &self,
        user_id: Uuid,
        name: &str,
        description: Option<String>,
        system_prompt: &str,
        model_id: &str,
    ) -> Result<ToolRouterRow>;
    /// Update a tool router.
    async fn update_tool_router(
        &self,
        id: Uuid,
        name: Option<String>,
        description: Option<String>,
        system_prompt: Option<String>,
        model_id: Option<String>,
        is_active: Option<bool>,
    ) -> Result<ToolRouterRow>;
    /// Delete a tool router.
    async fn delete_tool_router(&self, id: Uuid) -> Result<()>;
    /// Get all tools assigned to a router.
    async fn get_router_tools(&self, router_id: Uuid) -> Result<Vec<ToolRow>>;
    /// Set the full tool list for a router (replaces existing).
    async fn set_router_tools(&self, router_id: Uuid, tool_ids: &[Uuid]) -> Result<()>;

    // --- Router Modes ---

    /// List all modes for a router, ordered by display_order.
    async fn list_router_modes(&self, router_id: Uuid) -> Result<Vec<ToolRouterModeRow>>;
    /// Get a router mode by ID.
    async fn get_router_mode(&self, id: Uuid) -> Result<Option<ToolRouterModeRow>>;
    /// Get a router mode by its key within a router.
    async fn get_router_mode_by_key(
        &self,
        router_id: Uuid,
        mode_key: &str,
    ) -> Result<Option<ToolRouterModeRow>>;
    /// Create a new router mode.
    async fn create_router_mode(
        &self,
        router_id: Uuid,
        mode_key: &str,
        display_name: &str,
        description: &str,
        system_prompt: &str,
        temperature: f32,
        max_tokens: i32,
        append_to_agent_system_prompt: bool,
        append_to_agent_tools: bool,
        display_order: i32,
    ) -> Result<ToolRouterModeRow>;
    /// Update a router mode (all fields optional for partial updates).
    async fn update_router_mode(
        &self,
        id: Uuid,
        mode_key: Option<String>,
        display_name: Option<String>,
        description: Option<String>,
        system_prompt: Option<String>,
        temperature: Option<f32>,
        max_tokens: Option<i32>,
        append_to_agent_system_prompt: Option<bool>,
        append_to_agent_tools: Option<bool>,
        display_order: Option<i32>,
    ) -> Result<ToolRouterModeRow>;
    /// Delete a router mode.
    async fn delete_router_mode(&self, id: Uuid) -> Result<()>;
    /// Get all tools assigned to a mode.
    async fn get_mode_tools(&self, mode_id: Uuid) -> Result<Vec<ToolRow>>;
    /// Set the full tool list for a mode (replaces existing).
    async fn set_mode_tools(&self, mode_id: Uuid, tool_ids: &[Uuid]) -> Result<()>;
}

// ============================================================================
// Context Store Repository
// ============================================================================

/// Database operations for the per-session context store.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait ContextStoreRepo: Send + Sync {
    /// Add a context entry to a session.
    async fn add_context(
        &self,
        session_id: Uuid,
        source: &str,
        priority: f32,
        content: &str,
        metadata: Option<serde_json::Value>,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<ContextStoreRow>;
    /// Get active context for a session, ordered by priority descending.
    async fn get_active_context(
        &self,
        session_id: Uuid,
        limit: u32,
    ) -> Result<Vec<ContextStoreRow>>;
    /// Update the status of a context entry.
    async fn update_context_status(&self, id: Uuid, status: &str) -> Result<()>;
    /// Expire stale context entries (past expires_at). Returns count expired.
    async fn expire_stale_context(&self, session_id: Uuid) -> Result<u32>;
}

// ============================================================================
// Router Request Repository
// ============================================================================

/// Database operations for router request logging.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait RouterRequestRepo: Send + Sync {
    /// Create a new router request log entry.
    async fn create_router_request(
        &self,
        session_id: Uuid,
        agent_execution_id: Option<Uuid>,
        intent: &str,
        priority: &str,
        callback_hint: Option<String>,
    ) -> Result<RouterRequestRow>;
    /// Update a router request with routing decision and result.
    async fn update_router_request(
        &self,
        id: Uuid,
        routed_tool: Option<String>,
        routed_args: Option<serde_json::Value>,
        is_async: bool,
        passdown: Option<String>,
        chain: Option<serde_json::Value>,
        status: &str,
        result: Option<String>,
    ) -> Result<RouterRequestRow>;
    /// Get a router request by ID.
    async fn get_router_request(&self, id: Uuid) -> Result<Option<RouterRequestRow>>;
    /// List all router requests for a session.
    async fn list_session_requests(&self, session_id: Uuid) -> Result<Vec<RouterRequestRow>>;
}

// ============================================================================
// Room Repository
// ============================================================================

/// Input type for setting room members in bulk.
#[derive(Debug, Clone)]
pub struct RoomMemberInput {
    pub agent_id: Uuid,
    pub display_name: Option<String>,
    pub role_description: String,
    pub display_order: i32,
}

/// Database operations for rooms, room members, and room sessions.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait RoomRepo: Send + Sync {
    // --- Room CRUD ---

    /// Create a new room within a collection.
    async fn create_room(
        &self,
        user_id: Uuid,
        collection_id: Option<Uuid>,
        name: &str,
        gatekeeper_enabled: bool,
        gatekeeper_model_id: &str,
        max_speakers_per_turn: i32,
        max_turns: i32,
        tools_enabled: bool,
    ) -> Result<RoomRow>;

    /// Get a room by ID.
    async fn get_room(&self, id: Uuid) -> Result<Option<RoomRow>>;

    /// Update a room's configuration.
    async fn update_room(
        &self,
        id: Uuid,
        name: Option<String>,
        gatekeeper_enabled: Option<bool>,
        gatekeeper_model_id: Option<String>,
        max_speakers_per_turn: Option<i32>,
        max_turns: Option<i32>,
        tools_enabled: Option<bool>,
    ) -> Result<RoomRow>;

    /// Delete a room by ID.
    async fn delete_room(&self, id: Uuid) -> Result<()>;

    // --- Room members (join table) ---

    /// List all members of a room, ordered by display_order.
    async fn list_room_members(&self, room_id: Uuid) -> Result<Vec<RoomMemberRow>>;

    /// Add a single member to a room.
    async fn add_room_member(
        &self,
        room_id: Uuid,
        agent_id: Uuid,
        display_name: Option<String>,
        role_description: String,
        display_order: i32,
    ) -> Result<()>;

    /// Remove a single member from a room.
    async fn remove_room_member(&self, room_id: Uuid, agent_id: Uuid) -> Result<()>;

    /// Replace all members of a room atomically.
    async fn set_room_members(&self, room_id: Uuid, members: &[RoomMemberInput]) -> Result<()>;

    // --- Room sessions (runtime) ---

    /// Start a new room session.
    async fn create_room_session(&self, room_id: Uuid) -> Result<RoomSessionRow>;

    /// Get a room session by ID.
    async fn get_room_session(&self, id: Uuid) -> Result<Option<RoomSessionRow>>;

    /// Update room session status.
    async fn update_room_session_status(&self, id: Uuid, status: &str) -> Result<()>;

    /// Increment turn counter and return new value.
    async fn increment_room_session_turn(&self, id: Uuid) -> Result<i32>;

    /// Set the compressed transcript summary for older turns.
    async fn set_transcript_summary(&self, id: Uuid, summary: &str) -> Result<()>;

    // --- Room transcript ---

    /// Load the full room transcript (cross-execution message join).
    async fn get_room_transcript(&self, room_session_id: Uuid) -> Result<Vec<RoomTranscriptEntry>>;

    // --- Room Execution Outputs (Phase 3) ---

    /// Save a structured output from a room speaker
    async fn save_room_execution_output(
        &self,
        room_session_id: Uuid,
        agent_execution_id: Uuid,
        agent_id: Uuid,
        speaker_order: i32,
        turn_number: i32,
        output_name: &str,
        structured_output: &serde_json::Value,
        raw_output: &str,
        schema_id: Option<Uuid>,
    ) -> Result<RoomExecutionOutputRow>;

    /// Get room execution outputs, optionally filtered by turn number
    async fn get_room_execution_outputs(
        &self,
        room_session_id: Uuid,
        turn_number: Option<i32>,
    ) -> Result<Vec<RoomExecutionOutputRow>>;

    /// Get room execution outputs by schema ID
    async fn get_room_outputs_by_schema(
        &self,
        room_session_id: Uuid,
        schema_id: Uuid,
    ) -> Result<Vec<RoomExecutionOutputRow>>;
}

// ============================================================================
// Workflow Collection Repository
// ============================================================================

/// Database operations for workflow collections (multi-tier DAG architecture).
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait WorkflowCollectionRepo: Send + Sync {
    // --- Collections ---
    async fn create_collection(
        &self,
        user_id: Uuid,
        name: String,
        description: Option<String>,
        execution_mode: String,
    ) -> Result<WorkflowCollectionRow>;
    async fn get_collection(&self, id: Uuid) -> Result<Option<WorkflowCollectionRow>>;
    async fn list_collections(&self, user_id: Uuid) -> Result<Vec<WorkflowCollectionRow>>;
    async fn update_collection(
        &self,
        id: Uuid,
        name: Option<String>,
        description: Option<String>,
        execution_mode: Option<String>,
    ) -> Result<WorkflowCollectionRow>;
    async fn delete_collection(&self, id: Uuid) -> Result<()>;

    // --- Collection Workflows (Membership) ---
    async fn add_collection_workflow(
        &self,
        collection_id: Uuid,
        workflow_id: Uuid,
        display_order: i32,
        execution_mode: Option<String>,
    ) -> Result<CollectionWorkflowRow>;
    async fn list_collection_workflows(
        &self,
        collection_id: Uuid,
    ) -> Result<Vec<CollectionWorkflowRow>>;
    async fn remove_collection_workflow(
        &self,
        collection_id: Uuid,
        workflow_id: Uuid,
    ) -> Result<()>;
    async fn update_collection_workflow(
        &self,
        collection_id: Uuid,
        workflow_id: Uuid,
        display_order: Option<i32>,
        execution_mode: Option<String>,
    ) -> Result<CollectionWorkflowRow>;

    // --- Collection Workflow Edges (DAG edges between workflows) ---
    async fn set_collection_edges(
        &self,
        collection_id: Uuid,
        edges: Vec<CollectionWorkflowEdgeRow>,
    ) -> Result<()>;
    async fn list_collection_edges(
        &self,
        collection_id: Uuid,
    ) -> Result<Vec<CollectionWorkflowEdgeRow>>;
    async fn add_collection_edge(
        &self,
        collection_id: Uuid,
        from_workflow_id: Uuid,
        to_workflow_id: Uuid,
    ) -> Result<()>;
    async fn remove_collection_edge(
        &self,
        collection_id: Uuid,
        from_workflow_id: Uuid,
        to_workflow_id: Uuid,
    ) -> Result<()>;

    // --- Collection Runs (Execution Tracking) ---
    async fn create_collection_run(
        &self,
        collection_id: Uuid,
        user_id: Uuid,
    ) -> Result<CollectionRunRow>;
    async fn get_collection_run(&self, id: Uuid) -> Result<Option<CollectionRunRow>>;
    async fn list_collection_runs(&self, collection_id: Uuid) -> Result<Vec<CollectionRunRow>>;
    async fn update_collection_run_status(
        &self,
        id: Uuid,
        status: &str,
        error: Option<String>,
    ) -> Result<CollectionRunRow>;

    // --- Workflow Executions (Workflow-level execution within a collection run) ---
    async fn create_workflow_execution(
        &self,
        collection_run_id: Uuid,
        workflow_id: Uuid,
        user_id: Uuid,
    ) -> Result<WorkflowExecutionRow>;
    async fn get_workflow_execution(&self, id: Uuid) -> Result<Option<WorkflowExecutionRow>>;
    async fn list_workflow_executions(
        &self,
        collection_run_id: Uuid,
    ) -> Result<Vec<WorkflowExecutionRow>>;
    async fn update_workflow_execution_status(
        &self,
        id: Uuid,
        status: &str,
        outputs: Option<serde_json::Value>,
        error: Option<String>,
    ) -> Result<WorkflowExecutionRow>;

    async fn list_workflow_executions_by_workflow(
        &self,
        workflow_id: Uuid,
        user_id: Uuid,
    ) -> Result<Vec<WorkflowExecutionRow>>;

    // --- Standalone Workflow Execution (no collection) ---
    async fn create_standalone_workflow_execution(
        &self,
        workflow_id: Uuid,
        user_id: Uuid,
    ) -> Result<WorkflowExecutionRow>;

    // --- Child Workflow Execution (sub-workflow nesting) ---
    async fn create_child_workflow_execution(
        &self,
        parent_execution_id: Uuid,
        workflow_id: Uuid,
        user_id: Uuid,
        template_id: Uuid,
    ) -> Result<WorkflowExecutionRow>;

    async fn list_child_executions(
        &self,
        parent_execution_id: Uuid,
    ) -> Result<Vec<WorkflowExecutionRow>>;

    /// List the full execution tree rooted at `root_id` (O(1) via root_execution_id index).
    async fn list_execution_tree(
        &self,
        root_id: Uuid,
    ) -> Result<Vec<WorkflowExecutionRow>>;

    // --- Workshop (persistent per-workflow execution context) ---
    async fn get_or_create_workshop(
        &self,
        workflow_id: Uuid,
        user_id: Uuid,
    ) -> Result<WorkflowExecutionRow>;
}

// ============================================================================
// Workflow Step Agent Repository
// ============================================================================

/// Database operations for workflow step agents (multi-agent support).
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait WorkflowStepAgentRepo: Send + Sync {
    /// Add an agent to a workflow step.
    async fn add_step_agent(
        &self,
        step_id: Uuid,
        agent_id: Uuid,
        execution_strategy: String,
        agent_order: i32,
    ) -> Result<WorkflowStepAgentRow>;

    /// List all agents for a workflow step.
    async fn list_step_agents(&self, step_id: Uuid) -> Result<Vec<WorkflowStepAgentRow>>;

    /// Remove an agent from a workflow step.
    async fn remove_step_agent(&self, step_id: Uuid, agent_id: Uuid) -> Result<()>;

    /// Update agent configuration for a step.
    async fn update_step_agent(
        &self,
        step_id: Uuid,
        agent_id: Uuid,
        execution_strategy: Option<String>,
        agent_order: Option<i32>,
    ) -> Result<WorkflowStepAgentRow>;

    /// Replace all agents for a step (for bulk updates).
    async fn set_step_agents(&self, step_id: Uuid, agents: Vec<WorkflowStepAgentRow>)
        -> Result<()>;
}

// ============================================================================
// Tool Capability Repository (Phase 3)
// ============================================================================

/// Repository for tool capability taxonomy and assignments
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait ToolCapabilityRepo: Send + Sync {
    // Capability taxonomy queries

    /// Get all tool capabilities
    async fn get_tool_capabilities(&self) -> Result<Vec<ToolCapabilityRow>>;

    /// Get a capability by ID
    async fn get_tool_capability(&self, id: Uuid) -> Result<Option<ToolCapabilityRow>>;

    /// Get a capability by key
    async fn get_tool_capability_by_key(&self, key: &str) -> Result<Option<ToolCapabilityRow>>;

    // Tool-to-capability assignments

    /// Get all capabilities assigned to a tool
    async fn get_capabilities_by_tool(&self, tool_id: Uuid) -> Result<Vec<ToolCapabilityRow>>;

    /// Get all tools that provide a capability
    async fn get_tools_by_capability(&self, capability_key: &str) -> Result<Vec<ToolRow>>;

    /// Get all tools that provide ANY of the given capabilities (union, deduplicated)
    async fn get_tools_by_capabilities(&self, capability_keys: &[String]) -> Result<Vec<ToolRow>>;

    /// Assign a capability to a tool
    async fn assign_capability_to_tool(&self, tool_id: Uuid, capability_id: Uuid) -> Result<()>;

    /// Remove a capability from a tool
    async fn remove_capability_from_tool(&self, tool_id: Uuid, capability_id: Uuid) -> Result<()>;

    /// Set all capabilities for a tool (replaces existing)
    async fn set_tool_capabilities(&self, tool_id: Uuid, capability_ids: &[Uuid]) -> Result<()>;

    // Mode-to-capability requirements

    /// Get all capabilities required by a mode
    async fn get_mode_capabilities(&self, mode_id: Uuid) -> Result<Vec<ToolCapabilityRow>>;

    /// Set capabilities required by a mode (replaces existing)
    async fn set_mode_capabilities(
        &self,
        mode_id: Uuid,
        capability_ids: &[Uuid],
        is_required: bool,
    ) -> Result<()>;
}

// ============================================================================
// System Configuration Repository (Phase 3)
// ============================================================================

/// Repository for system-wide configuration (admin-controlled)
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait SystemConfigRepo: Send + Sync {
    // Basic config operations

    /// Get a system config by key
    async fn get_system_config(&self, config_key: &str) -> Result<Option<SystemConfigRow>>;

    /// List system configs, optionally filtered by type
    async fn list_system_configs(
        &self,
        config_type: Option<String>,
    ) -> Result<Vec<SystemConfigRow>>;

    /// Upsert a system config (insert or update)
    async fn upsert_system_config(
        &self,
        config_type: &str,
        config_key: &str,
        config_value: &serde_json::Value,
        description: Option<String>,
        created_by: Option<Uuid>,
    ) -> Result<SystemConfigRow>;

    /// Delete a system config
    async fn delete_system_config(&self, config_key: &str) -> Result<()>;

    // Specialized config queries

    /// Get all execution constraints as a map
    async fn get_execution_constraints(
        &self,
    ) -> Result<std::collections::HashMap<String, serde_json::Value>>;

    /// Check if unsafe operations are enabled
    async fn get_unsafe_operations_enabled(&self) -> Result<bool>;
}

// ============================================================================
// Protocol Repository
// ============================================================================

/// Database operations for protocol management (reusable execution recipes).
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait ProtocolRepo: Send + Sync {
    // --- Protocols ---

    /// Create a new protocol.
    async fn create_protocol(
        &self,
        name: String,
        description: String,
        protocol_type: String,
        config: serde_json::Value,
        agent_id: Option<Uuid>,
        output_schema_id: Option<Uuid>,
        prompt_template_id: Option<Uuid>,
    ) -> Result<ProtocolRow>;

    /// Get a protocol by ID.
    async fn get_protocol(&self, id: Uuid) -> Result<Option<ProtocolRow>>;

    /// Get a protocol by its protocol_type (e.g., "documenter").
    async fn get_protocol_by_type(&self, protocol_type: &str) -> Result<Option<ProtocolRow>>;

    /// List all protocols.
    async fn list_protocols(&self) -> Result<Vec<ProtocolRow>>;

    /// Seed the built-in system protocols and their associated agents,
    /// output schemas, and prompt templates. Idempotent via ON CONFLICT DO UPDATE.
    async fn seed_builtin_protocols(&self) -> Result<()>;

    /// Update a protocol.
    async fn update_protocol(
        &self,
        id: Uuid,
        name: Option<String>,
        description: Option<String>,
        config: Option<serde_json::Value>,
        agent_id: Option<Uuid>,
        output_schema_id: Option<Uuid>,
        prompt_template_id: Option<Uuid>,
    ) -> Result<ProtocolRow>;

    /// Delete a protocol by ID.
    async fn delete_protocol(&self, id: Uuid) -> Result<()>;

    // --- Protocol Ports ---

    /// List all ports for a protocol, ordered by display_order.
    async fn list_protocol_ports(&self, protocol_id: Uuid) -> Result<Vec<ProtocolPortRow>>;

    /// Add a port to a protocol.
    async fn create_protocol_port(
        &self,
        protocol_id: Uuid,
        port_name: String,
        description: String,
        agent_id: Uuid,
        display_order: i32,
    ) -> Result<ProtocolPortRow>;

    /// Update a protocol port.
    async fn update_protocol_port(
        &self,
        id: Uuid,
        port_name: Option<String>,
        description: Option<String>,
        agent_id: Option<Uuid>,
        display_order: Option<i32>,
    ) -> Result<ProtocolPortRow>;

    /// Delete a protocol port.
    async fn delete_protocol_port(&self, id: Uuid) -> Result<()>;

    // --- Workflow Step Protocol Linkage ---

    /// Get the protocol linkage for a workflow step.
    async fn get_step_protocol(
        &self,
        workflow_step_id: Uuid,
    ) -> Result<Option<WorkflowStepProtocolRow>>;

    /// Link a protocol to a workflow step (stores expansion snapshot).
    async fn create_step_protocol(
        &self,
        workflow_step_id: Uuid,
        protocol_id: Uuid,
        applied_expansion: serde_json::Value,
    ) -> Result<WorkflowStepProtocolRow>;

    /// Remove a protocol linkage from a workflow step.
    async fn delete_step_protocol(&self, workflow_step_id: Uuid) -> Result<()>;

    // --- Protocol-scoped Document Definitions ---

    /// List document definitions scoped to a protocol (template defs).
    async fn list_protocol_document_defs(
        &self,
        protocol_id: Uuid,
    ) -> Result<Vec<ProtocolDocumentDefRow>>;

    /// Create a protocol-scoped document definition.
    async fn create_protocol_document_def(
        &self,
        def: ProtocolDocumentDefRow,
    ) -> Result<ProtocolDocumentDefRow>;

    /// Update a protocol-scoped document definition.
    async fn update_protocol_document_def(
        &self,
        id: Uuid,
        name: String,
        description: String,
        target_length: i32,
    ) -> Result<ProtocolDocumentDefRow>;

    /// Delete a protocol-scoped document definition.
    async fn delete_protocol_document_def(&self, id: Uuid) -> Result<()>;

    // --- Protocol Executions ---

    /// Create a new protocol execution record.
    async fn create_protocol_execution(
        &self,
        row: ProtocolExecutionRow,
    ) -> Result<ProtocolExecutionRow>;

    /// Update a protocol execution's status and output fields.
    async fn update_protocol_execution_status(
        &self,
        id: Uuid,
        status: String,
        output_content: Option<String>,
        error_message: Option<String>,
        tokens_in: Option<i32>,
        tokens_out: Option<i32>,
        cost_usd: Option<f64>,
        model: Option<String>,
    ) -> Result<ProtocolExecutionRow>;

    /// List all protocol executions for a given step.
    async fn list_protocol_executions_by_step(
        &self,
        step_id: Uuid,
    ) -> Result<Vec<ProtocolExecutionRow>>;

    /// List all protocol executions for a given workflow run.
    async fn list_protocol_executions_by_run(
        &self,
        run_id: Uuid,
    ) -> Result<Vec<ProtocolExecutionRow>>;
}

// ============================================================================
// Content Version Repository
// ============================================================================

/// Database operations for content versioning and run snapshots.
#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait ContentVersionRepo: Send + Sync {
    /// Find an existing version by (source_id, content_type, content_hash)
    /// or create a new one. Returns the version row (existing or newly created).
    async fn find_or_create_version(
        &self,
        source_id: Uuid,
        content_type: &str,
        content_hash: &str,
        content: &str,
    ) -> Result<ContentVersionRow>;

    /// Create a run snapshot linking (run_id, step_id, content_type, role) to a version.
    async fn create_run_snapshot(
        &self,
        run_id: Uuid,
        step_id: Uuid,
        content_type: &str,
        role: &str,
        content_version_id: Uuid,
        source_id: Uuid,
    ) -> Result<RunSnapshotRow>;

    /// Get the content version for a specific (run_id, step_id, content_type, role).
    async fn get_run_snapshot(
        &self,
        run_id: Uuid,
        step_id: Uuid,
        content_type: &str,
        role: &str,
    ) -> Result<Option<RunSnapshotRow>>;

    /// List all snapshots for a given run.
    async fn list_run_snapshots(&self, run_id: Uuid) -> Result<Vec<RunSnapshotRow>>;

    /// Resolve a document def_id to its versioned content for a specific run.
    async fn resolve_document_version_by_def(
        &self,
        def_id: Uuid,
        run_id: Uuid,
    ) -> Result<Option<ContentVersionRow>>;

    /// List all envelope output snapshots for a run (JOIN with content_versions).
    /// Returns (step_id, envelope_json, source_id) for DagState reconstruction.
    async fn list_envelope_snapshots_for_run(
        &self,
        run_id: Uuid,
    ) -> Result<Vec<EnvelopeSnapshotRow>>;
}
