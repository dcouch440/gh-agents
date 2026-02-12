//! Unified Chat Hub — single execution engine for chat, DAG pipelines,
//! and pipeline-inside-chat flows.
//!
//! All LLM execution in the application goes through `ExecutionEngine::execute()`
//! parameterized by an `ExecutionStrategy`. Different strategies handle chat
//! sessions, DAG workflow steps, and tool routing.

pub mod capability_resolver;
pub mod dag;
pub mod engine;
pub mod error;
pub mod prompt_registry;
pub mod protocols;
pub mod recorder;
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
pub use strategies::{
    ChatConfig, ChatStrategy, DagStepStrategy, RoomSpeakerConfig, RoomSpeakerStrategy,
};
pub use strategy::ExecutionStrategy;
pub use streaming::{NullSink, StreamSink};

/// Run a chat turn for the given agent. Loads config from DB, builds strategy, executes.
///
/// This is the primary entry point for all agent chat interactions. Both the
/// orchestrator and API handlers call through here.
pub async fn run_chat(
    state: &AppState,
    provider: Arc<dyn LLMProvider + Send + Sync>,
    agent_id: Uuid,
    session_id: Option<Uuid>,
    message_id: Uuid,
    content: &str,
    user_id: UserId,
    cancel: Option<&CancellationToken>,
) -> Result<ExecutionResult, HubError> {
    // Load agent from DB
    let agent = state
        .repo()
        .get_persisted_agent(agent_id)
        .await
        .map_err(HubError::Internal)?
        .ok_or_else(|| HubError::Internal(anyhow::anyhow!("Agent {agent_id} not found")))?;

    // Load agent tools
    let tools = state
        .repo()
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
    let strategy = ChatStrategy::new(chat_config, state.clone(), user_id, session_id, message_id);
    let mut engine = ExecutionEngine::new(provider);
    if let Some((filter_ctx, filters)) = schema_filters {
        engine = engine.with_filter_context(filter_ctx).with_filters(filters);
    }
    let sink = streaming::SseSink::new(state.clone(), message_id);
    let ae_repo = state.agent_execution_repo();
    let tl_repo = state.token_ledger_repo();
    let recorder = ExecutionRecorder::new(
        state.repo().as_ref(),
        ae_repo.as_deref(),
        tl_repo.as_deref(),
    );

    engine
        .execute(&strategy, content, &sink, &recorder, cancel)
        .await
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
