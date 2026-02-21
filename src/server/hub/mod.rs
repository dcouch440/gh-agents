//! Unified Chat Hub — single execution engine for chat, DAG pipelines,
//! and pipeline-inside-chat flows.
//!
//! All LLM execution in the application goes through `ExecutionEngine::execute()`
//! parameterized by an `ExecutionStrategy`. Different strategies handle chat
//! sessions, DAG workflow steps, and tool routing.

pub mod board_overview;
pub mod board_state;
pub mod capability_resolver;
pub mod chat_beliefs;
pub mod consistency_scanner;
pub mod dag;
pub mod dispatch_status;
pub mod engine;
pub mod error;
pub mod graph_context;
pub mod prompt_registry;
pub mod protocols;
pub mod recorder;
pub mod run_results;
pub mod strategies;
pub mod strategy;
pub mod streaming;

use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::llm::LLMProvider;
use crate::types::UserId;

use super::state::AppState;
use engine::filters::{
    ExecutionFilter, FilterContext, PartialJsonRecoveryFilter, SchemaEnhancementFilter,
    SchemaValidationRetryFilter,
};

pub use engine::{ExecutionEngine, ExecutionResult};
pub use error::HubError;
pub use prompt_registry::PromptRegistry;
pub use recorder::ExecutionRecorder;
pub use strategies::{ChatConfig, ChatStrategy, DagStepStrategy, StepChatContext};
pub use strategy::ExecutionStrategy;
pub use streaming::{DagStreamSink, NullSink, StreamSink};

/// Common chat request parameters shared between `run_chat` and `run_step_chat`.
pub struct ChatRequest<'a> {
    pub provider: Arc<dyn LLMProvider + Send + Sync>,
    pub message_id: Uuid,
    pub content: &'a str,
    pub user_id: UserId,
}

/// Run a chat turn for the given agent. Loads config from DB, builds strategy, executes.
///
/// This is the primary entry point for all agent chat interactions. Both the
/// orchestrator and API handlers call through here.
pub async fn run_chat(
    state: &AppState,
    req: ChatRequest<'_>,
    agent_id: Uuid,
    session_id: Option<Uuid>,
    cancel: Option<&CancellationToken>,
) -> Result<ExecutionResult, HubError> {
    // Load agent from DB
    let agent = state
        .repos()
        .agents
        .get_persisted_agent(agent_id)
        .await
        .map_err(HubError::Internal)?
        .ok_or_else(|| HubError::Internal(anyhow::anyhow!("Agent {agent_id} not found")))?;

    // Load agent tools
    let tools = state
        .repos()
        .tools
        .get_agent_tools(agent_id)
        .await
        .map_err(HubError::Internal)?;
    let tool_names: Vec<String> = tools.into_iter().map(|t| t.name).collect();

    // Build ChatConfig directly from agent row
    let mut chat_config = ChatConfig {
        system_prompt: agent.system_prompt.clone(),
        tool_names,
        model_id: agent.model_id.clone(),
        temperature: agent.model_temperature,
        max_history: 50,
        ..Default::default()
    };

    // Load output schema and build filter pipeline if configured
    let schema_filters = if let Some(schema_id) = agent.output_schema_id {
        if let Some((schema_xml, filter_ctx, filters)) =
            load_schema_filters(state, schema_id, &chat_config.model_id, agent_id).await
        {
            chat_config.system_prompt.push_str(&schema_xml);
            Some((filter_ctx, filters))
        } else {
            None
        }
    } else {
        None
    };

    // Create strategy, engine, sink, recorder
    let strategy = ChatStrategy::new(
        chat_config,
        state.clone(),
        req.user_id,
        session_id,
        req.message_id,
    );
    let mut engine = ExecutionEngine::new(req.provider);
    if let Some((filter_ctx, filters)) = schema_filters {
        engine = engine.with_filter_context(filter_ctx).with_filters(filters);
    }
    let sink = streaming::SseSink::new(state.clone(), req.message_id);
    let recorder = ExecutionRecorder::new(
        &*state.repos().sessions,
        &*state.repos().chat_messages,
        Some(&*state.repos().agent_executions),
        Some(&*state.repos().token_ledger),
    );

    engine
        .execute(&strategy, req.content, &sink, &recorder, cancel)
        .await
}

/// Run a chat turn scoped to a workflow step. Builds ChatConfig from
/// step context (execution mode, upstream state) instead of from an agent.
///
/// Called by the chat consumer when a session has step_id in its draft_config.
pub async fn run_step_chat(
    state: &AppState,
    req: ChatRequest<'_>,
    session_id: Uuid,
    workflow_id: Uuid,
    step_id: Uuid,
    cancel: Option<&CancellationToken>,
) -> Result<ExecutionResult, HubError> {
    // Load step to determine execution_mode
    let step = state
        .repos()
        .workflows
        .get_step(step_id)
        .await
        .map_err(HubError::Internal)?
        .ok_or_else(|| HubError::Internal(anyhow::anyhow!("Step {step_id} not found")))?;

    // Build system prompt with live step state
    let system_prompt =
        build_step_system_prompt(state, workflow_id, step_id, &step.execution_mode).await?;

    // Build ChatConfig — tool_names empty because tools are resolved by step_context
    let chat_config = ChatConfig {
        system_prompt,
        tool_names: vec![],
        model_id: crate::constants::DEFAULT_MODEL.to_string(),
        temperature: crate::constants::DEFAULT_TEMPERATURE,
        max_history: 50,
        ..Default::default()
    };

    // Create strategy with step context
    let step_context = StepChatContext {
        workflow_id,
        step_id,
        execution_mode: step.execution_mode.clone(),
        step_name: step.name.clone().unwrap_or_default(),
    };
    let strategy = ChatStrategy::with_step_context(
        chat_config,
        state.clone(),
        req.user_id,
        Some(session_id),
        req.message_id,
        step_context,
    );

    // Execute
    let engine = ExecutionEngine::new(req.provider);
    let sink = streaming::SseSink::new(state.clone(), req.message_id);
    let recorder = ExecutionRecorder::new(
        &*state.repos().sessions,
        &*state.repos().chat_messages,
        Some(&*state.repos().agent_executions),
        Some(&*state.repos().token_ledger),
    );

    engine
        .execute(&strategy, req.content, &sink, &recorder, cancel)
        .await
}

/// Build the system prompt for a step chat session.
///
/// Uses the base+archetype-block architecture: the `ASSISTANT_BASE` role
/// definition is resolved with graph context, an archetype-specific block
/// (e.g. `WORKFORCE_ARCHETYPE`), and the live config snapshot injected via
/// template variables.
pub async fn build_step_system_prompt(
    state: &AppState,
    workflow_id: Uuid,
    step_id: Uuid,
    execution_mode: &str,
) -> Result<String, HubError> {
    use crate::config::protocols::{roles, vars};

    // 1. Load beliefs from connected nodes
    let connected_beliefs = state
        .repos()
        .workflows
        .get_beliefs_for_connected_steps(workflow_id, step_id)
        .await
        .unwrap_or_default();

    let board_context = chat_beliefs::format_beliefs_as_board_context(&connected_beliefs);

    // 1b. Load board overview summary
    let board_overview = state
        .repos()
        .workflows
        .get_board_overview_summary(workflow_id)
        .await
        .unwrap_or_default();
    let board_overview_text = if board_overview.is_empty() {
        "No steps have been configured yet.".to_string()
    } else {
        board_overview
    };

    // 1c. Load assistant notes for this step
    let assistant_notes = state
        .repos()
        .workflows
        .get_assistant_notes(step_id)
        .await
        .unwrap_or_default()
        .unwrap_or_else(|| {
            "No notes yet. Use update_notes to record important discoveries.".to_string()
        });

    // 2. Build archetype block + board state based on execution mode
    let (archetype_block, board_state_xml) = match execution_mode {
        "workforce" => {
            let xml = board_state::build(
                state.repos().workflows.as_ref(),
                board_state::BoardStateVariant::NodeAssistant,
                workflow_id,
                step_id,
            )
            .await
            .map_err(|e| HubError::Internal(anyhow::anyhow!("{}", e)))?;

            (roles::WORKFORCE_ARCHETYPE.to_string(), xml)
        }
        _ => {
            return Err(HubError::Internal(anyhow::anyhow!(
                "Step {} has unsupported execution_mode '{}'. Expected: workforce.",
                step_id,
                execution_mode
            )));
        }
    };

    // 2b. Build dispatch task status for this step
    let dispatch_status = dispatch_status::build(state.task_registry(), step_id);

    // 2c. Build run context (run results summaries from self + connected steps)
    let run_context = build_run_context(state, workflow_id, step_id).await;

    // 3. Resolve base template with all variables
    let mut vars_map = std::collections::HashMap::new();
    vars_map.insert(vars::system::BOARD_CONTEXT.to_string(), board_context);
    vars_map.insert(vars::system::ARCHETYPE_BLOCK.to_string(), archetype_block);
    vars_map.insert(vars::system::BOARD_STATE.to_string(), board_state_xml);
    vars_map.insert(
        vars::system::BOARD_OVERVIEW.to_string(),
        board_overview_text,
    );
    vars_map.insert(vars::system::ASSISTANT_NOTES.to_string(), assistant_notes);
    vars_map.insert(vars::system::DISPATCH_STATUS.to_string(), dispatch_status);
    vars_map.insert(vars::system::RUN_CONTEXT.to_string(), run_context);

    let resolved = roles::ASSISTANT_BASE.resolve(&vars_map);

    Ok(resolved.system_prompt)
}

/// Build the `<run_context>` block from run results summaries of the step
/// itself and directly connected steps (upstream + downstream).
///
/// Returns empty string if no summaries exist (blank-line collapsed by
/// the template resolver).
async fn build_run_context(state: &AppState, workflow_id: Uuid, step_id: Uuid) -> String {
    let entries = state
        .repos()
        .workflows
        .get_run_context_for_step(workflow_id, step_id)
        .await
        .unwrap_or_default();

    if entries.is_empty() {
        return String::new();
    }

    let mut lines = Vec::new();
    for (step_name, summary, pinned) in &entries {
        let pin_marker = if *pinned { " [pinned]" } else { "" };
        lines.push(format!("- {}{}: {}", step_name, pin_marker, summary));
    }

    format!("<run_context>\n{}\n</run_context>", lines.join("\n"))
}

/// Truncate a string to at most `max` bytes at a char boundary.
pub(crate) fn truncate_str(s: &str, max: usize) -> &str {
    if s.len() <= max {
        s
    } else {
        let end = s.floor_char_boundary(max);
        &s[..end]
    }
}

/// Load an output schema and build filter pipeline for schema enforcement.
///
/// Returns the schema XML to append to the system prompt, plus the filter context
/// and filter vec to attach to the engine. Returns `None` if no schema is configured.
async fn load_schema_filters(
    state: &AppState,
    schema_id: Uuid,
    model_id: &str,
    agent_id: Uuid,
) -> Option<(String, FilterContext, Vec<Arc<dyn ExecutionFilter>>)> {
    let os_repo = &state.repos().output_schemas;
    let schema = os_repo.get_output_schema(schema_id).await.ok()??;

    let schema_xml = format!(
        "\n\n<schema>\nYour response is parsed directly by a JSON parser. Respond with a valid JSON object matching this schema:\n```json\n{}\n```\n</schema>",
        serde_json::to_string_pretty(&schema.schema).unwrap_or_default()
    );

    let filter_ctx = FilterContext::new(model_id, agent_id).with_schema(schema.schema);
    let filters: Vec<Arc<dyn ExecutionFilter>> = vec![
        Arc::new(SchemaEnhancementFilter::new()),
        Arc::new(SchemaValidationRetryFilter::new()),
        Arc::new(PartialJsonRecoveryFilter::new()),
    ];

    Some((schema_xml, filter_ctx, filters))
}

/// Schedule removal of a response stream after a delay (for late-connecting SSE clients).
pub fn schedule_stream_cleanup(state: &AppState, message_id: Uuid) {
    let cleanup_state = state.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(120)).await;
        cleanup_state.remove_response_stream(message_id);
    });
}

#[cfg(test)]
mod tests;
