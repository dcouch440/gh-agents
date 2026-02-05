//! Unified Chat Hub — single execution engine for chat, DAG pipelines,
//! and pipeline-inside-chat flows.
//!
//! All LLM execution in the application goes through `ExecutionEngine::execute()`
//! parameterized by an `ExecutionStrategy`. Different strategies handle chat
//! sessions, DAG workflow steps, and tool routing.

pub mod dag;
pub mod engine;
pub mod error;
pub mod mode_resolver;
pub mod prompt_registry;
pub mod recorder;
pub mod strategies;
pub mod strategy;
pub mod streaming;

use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tracing::debug;
use uuid::Uuid;

use crate::db::AgentModeRow;
use crate::llm::LLMProvider;
use crate::types::UserId;

use super::state::AppState;
use strategies::router::RouterConfig;

pub use engine::{ExecutionEngine, ExecutionResult};
pub use error::HubError;
pub use mode_resolver::{ModeResolver, ResolvedModeConfig, RoutingError};
pub use prompt_registry::PromptRegistry;
pub use recorder::ExecutionRecorder;
pub use strategies::{
    ChatConfig, ChatStrategy, DagStepStrategy, RoomSpeakerConfig, RoomSpeakerStrategy,
    RouterStrategy,
};
pub use strategy::ExecutionStrategy;
pub use streaming::{NullSink, StreamSink};

use crate::db::traits::ServerRepo;
use crate::db::AgentRow;
use crate::llm::Tool;

/// Construct agent defaults when mode_resolver is unavailable.
/// Used for backward compatibility.
pub async fn construct_agent_defaults(
    agent: &AgentRow,
    repo: &Arc<dyn ServerRepo>,
) -> Result<ResolvedModeConfig, anyhow::Error> {
    let agent_tool_rows = repo.get_agent_tools(agent.id).await.unwrap_or_default();

    let tools: Vec<Tool> = agent_tool_rows
        .iter()
        .filter_map(|row| crate::tools::registry::get_tool_definition(&row.name))
        .collect();

    let tool_names = tools.iter().map(|t| t.name.clone()).collect();

    Ok(ResolvedModeConfig {
        system_prompt: agent.system_prompt.clone(),
        tools,
        tool_names,
        temperature: agent.model_temperature,
        max_tokens: agent.model_max_tokens,
        selected_mode_id: None,
        selected_mode_key: None,
    })
}

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
        .map_err(HubError::Internal)?
        .ok_or_else(|| HubError::Internal(anyhow::anyhow!("Agent {agent_id} not found")))?;

    // Build ChatConfig based on whether agent uses router_modes or legacy agent_modes
    let chat_config = if agent.router_id.is_some() && state.mode_resolver.is_some() {
        // NEW: Use router_modes system via ModeResolver
        debug!(agent_id = %agent_id, router_id = ?agent.router_id, "Using router_modes system");

        // Load conversation history for context
        let history = if let Some(sid) = session_id {
            state
                .repo
                .get_session_history(sid, 10)
                .await
                .map_err(HubError::Internal)?
        } else {
            vec![]
        };
        let history_text = format_history(&history);

        // Resolve mode
        let mode = state
            .mode_resolver
            .as_ref()
            .unwrap()
            .resolve(&agent, content, Some(&history_text))
            .await
            .map_err(|e| HubError::Internal(anyhow::anyhow!("Mode resolution failed: {}", e)))?;

        debug!(
            selected_mode_id = ?mode.selected_mode_id,
            selected_mode_key = ?mode.selected_mode_key,
            "Mode resolved"
        );

        ChatConfig {
            system_prompt: mode.system_prompt,
            tool_names: mode.tool_names,
            model_id: agent.model_id,
            temperature: mode.temperature,
            max_history: 50,
            max_rounds: 10,
            context_budget: 480_000,
        }
    } else {
        // OLD: Use legacy agent_modes system
        debug!(agent_id = %agent_id, "Using legacy agent_modes system");

        // Load agent tools
        let tools = state
            .repo
            .get_agent_tools(agent_id)
            .await
            .map_err(HubError::Internal)?;

        let tool_names: Vec<String> = tools.into_iter().map(|t| t.name).collect();

        // Load agent modes and classify if applicable
        let modes = state
            .repo
            .get_agent_modes(agent_id)
            .await
            .map_err(HubError::Internal)?;

        let active_mode = if modes.is_empty() {
            None
        } else {
            classify_mode(&modes, content, state, &provider, user_id).await?
        };

        // Build ChatConfig from agent row
        let mut chat_config = ChatConfig {
            system_prompt: agent.system_prompt.clone(),
            tool_names: tool_names.clone(),
            model_id: agent.model_id.clone(),
            temperature: agent.model_temperature,
            max_history: 50,
            ..Default::default()
        };

        if let Some(mode) = &active_mode {
            debug!(mode = %mode.name, agent_id = %agent_id, "Applying mode overlay");
            apply_mode_overlay(&mut chat_config, mode);
        }

        chat_config
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

    engine
        .execute(&strategy, content, &sink, &recorder, cancel)
        .await
}

/// Classify the user's message into one of the agent's modes using a RouterStrategy call.
///
/// Returns `None` if no mode is a clear fit (router returns "default" or unrecognized name).
async fn classify_mode(
    modes: &[AgentModeRow],
    user_message: &str,
    state: &AppState,
    provider: &Arc<dyn LLMProvider + Send + Sync>,
    user_id: UserId,
) -> Result<Option<AgentModeRow>, HubError> {
    // Build the mode listing for the routing prompt
    let mode_list: String = modes
        .iter()
        .map(|m| format!("- \"{}\": {}", m.name, m.classifier_hint))
        .collect::<Vec<_>>()
        .join("\n");

    let system_prompt = format!(
        "You are a conversation classifier. Given the user's message, pick the most appropriate mode.\n\n\
         Available modes:\n{mode_list}\n\n\
         Respond with JSON only: {{\"mode\": \"<name>\"}}\n\
         If no mode is a clear fit, respond: {{\"mode\": \"default\"}}"
    );

    let router_config = RouterConfig {
        system_prompt,
        model_id: "claude-3-haiku-20240307".to_string(),
        state: Some(state.clone()),
        user_id: Some(user_id),
    };

    let strategy = RouterStrategy::new(router_config);
    let engine = ExecutionEngine::new(provider.clone());
    let sink = NullSink;
    let recorder = ExecutionRecorder::new(
        state.repo.as_ref(),
        state.agent_execution_repo.as_deref(),
        state.token_ledger_repo.as_deref(),
    );

    let result = engine
        .execute(&strategy, user_message, &sink, &recorder, None)
        .await?;

    // Parse the mode name from the router's JSON response
    let mode_name = result
        .content
        .trim()
        .strip_prefix('{')
        .and_then(|s| s.strip_suffix('}'))
        .and_then(|_| serde_json::from_str::<serde_json::Value>(result.content.trim()).ok())
        .and_then(|v| v.get("mode").and_then(|m| m.as_str().map(String::from)));

    match mode_name {
        Some(name) if name != "default" => {
            let found = modes.iter().find(|m| m.name == name).cloned();
            if found.is_some() {
                debug!(mode = %name, "Router classified message");
            } else {
                debug!(mode = %name, "Router returned unknown mode, using base config");
            }
            Ok(found)
        }
        _ => {
            debug!("Router selected default mode, using base config");
            Ok(None)
        }
    }
}

/// Merge an agent mode's overrides onto a ChatConfig.
pub(crate) fn apply_mode_overlay(config: &mut ChatConfig, mode: &AgentModeRow) {
    if let Some(suffix) = &mode.system_prompt_suffix {
        config.system_prompt.push_str("\n\n");
        config.system_prompt.push_str(suffix);
    }
    if let Some(temp) = mode.temperature_override {
        config.temperature = temp as f32;
    }
    if let Some(model) = &mode.model_override {
        config.model_id = model.clone();
    }
    if let Some(tools) = &mode.tool_overrides {
        config.tool_names = tools.clone();
    }
}

/// Format conversation history into text for router context.
fn format_history(history: &[crate::db::ChatMessageRow]) -> String {
    if history.is_empty() {
        return String::new();
    }

    history
        .iter()
        .map(|msg| {
            let role = if msg.role == "user" {
                "User"
            } else {
                "Assistant"
            };
            format!("{}: {}", role, msg.content)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Schedule removal of a response stream after a delay (for late-connecting SSE clients).
pub fn schedule_stream_cleanup(state: &AppState, message_id: Uuid) {
    let cleanup_state = state.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(120)).await;
        cleanup_state.remove_response_stream(message_id).await;
    });
}

#[cfg(test)]
mod tests;
