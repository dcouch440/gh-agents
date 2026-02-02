//! Unified Chat Hub — single execution engine for chat, DAG pipelines,
//! and pipeline-inside-chat flows.
//!
//! All LLM execution in the application goes through `ExecutionEngine::execute()`
//! parameterized by an `ExecutionStrategy`. Different strategies handle chat
//! sessions, DAG workflow steps, and tool routing.

pub mod dag;
pub mod engine;
pub mod error;
pub mod pipeline_advance;
pub mod prompt_registry;
pub mod recorder;
pub mod strategies;
pub mod streaming;
pub mod strategy;

use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::llm::LLMProvider;
use crate::types::UserId;

use super::state::AppState;

pub use engine::{ExecutionEngine, ExecutionResult};
pub use error::HubError;
pub use pipeline_advance::{advance_pipeline, PipelineAdvanceAction};
pub use prompt_registry::PromptRegistry;
pub use recorder::ExecutionRecorder;
pub use strategies::{ChatConfig, ChatStrategy, DagStepStrategy, RouterStrategy};
pub use streaming::{NullSink, StreamSink};
pub use strategy::ExecutionStrategy;

/// Run a chat turn for the given agent. Loads config from DB, builds strategy, executes.
///
/// This is the primary entry point for all chat interactions. Both the
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
        .repo
        .get_persisted_agent(agent_id)
        .await
        .map_err(|e| HubError::Internal(e))?
        .ok_or_else(|| HubError::Internal(anyhow::anyhow!("Agent {agent_id} not found")))?;

    // Load agent tools
    let tools = state
        .repo
        .get_agent_tools(agent_id)
        .await
        .map_err(|e| HubError::Internal(e))?;

    let tool_names: Vec<String> = tools.into_iter().map(|t| t.name).collect();

    // Build ChatConfig from agent row
    let chat_config = ChatConfig {
        system_prompt: agent.system_prompt,
        tool_names,
        model_id: agent.model_id,
        temperature: agent.model_temperature,
        max_history: 50,
        ..Default::default()
    };

    // Create strategy, engine, sink, recorder
    let strategy = ChatStrategy::new(chat_config, state.clone(), user_id, session_id, message_id);
    let engine = ExecutionEngine::new(provider);
    let sink = streaming::SseSink::new(state.clone(), message_id);
    let recorder = ExecutionRecorder::new(
        state.repo.as_ref(),
        state.agent_execution_repo.as_deref(),
        state.token_ledger_repo.as_deref(),
    );

    engine.execute(&strategy, content, &sink, &recorder, cancel).await
}

/// Schedule removal of a response stream after a delay (for late-connecting SSE clients).
pub fn schedule_stream_cleanup(state: &AppState, message_id: Uuid) {
    let cleanup_state = state.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(120)).await;
        cleanup_state.remove_response_stream(message_id).await;
    });
}
