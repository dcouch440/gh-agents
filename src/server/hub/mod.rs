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
use tracing::debug;
use uuid::Uuid;

use crate::db::AgentModeRow;
use crate::llm::LLMProvider;
use crate::types::UserId;

use super::state::AppState;
use strategies::router::RouterConfig;

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

    // Load agent modes and classify if applicable
    let modes = state
        .repo
        .get_agent_modes(agent_id)
        .await
        .map_err(|e| HubError::Internal(e))?;

    let active_mode = if modes.is_empty() {
        None
    } else {
        classify_mode(&modes, content, state, &provider, user_id).await?
    };

    // Build ChatConfig from agent row
    let mut chat_config = ChatConfig {
        system_prompt: agent.system_prompt,
        tool_names: tool_names.clone(),
        model_id: agent.model_id,
        temperature: agent.model_temperature,
        max_history: 50,
        ..Default::default()
    };

    if let Some(mode) = &active_mode {
        debug!(mode = %mode.name, agent_id = %agent_id, "Applying mode overlay");
        apply_mode_overlay(&mut chat_config, mode);
    }

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

    let result = engine.execute(&strategy, user_message, &sink, &recorder, None).await?;

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
fn apply_mode_overlay(config: &mut ChatConfig, mode: &AgentModeRow) {
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

/// Schedule removal of a response stream after a delay (for late-connecting SSE clients).
pub fn schedule_stream_cleanup(state: &AppState, message_id: Uuid) {
    let cleanup_state = state.clone();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(120)).await;
        cleanup_state.remove_response_stream(message_id).await;
    });
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_mode(name: &str, hint: &str) -> AgentModeRow {
        AgentModeRow {
            id: Uuid::new_v4(),
            agent_id: Uuid::new_v4(),
            name: name.to_string(),
            system_prompt_suffix: None,
            temperature_override: None,
            model_override: None,
            tool_overrides: None,
            classifier_hint: hint.to_string(),
            created_at: Utc::now(),
        }
    }

    #[test]
    fn apply_mode_overlay_prompt_suffix() {
        let mut config = ChatConfig {
            system_prompt: "Base prompt.".into(),
            ..Default::default()
        };
        let mut mode = make_mode("technical", "For technical questions");
        mode.system_prompt_suffix = Some("Be precise and technical.".into());

        apply_mode_overlay(&mut config, &mode);
        assert!(config.system_prompt.contains("Base prompt."));
        assert!(config.system_prompt.contains("Be precise and technical."));
    }

    #[test]
    fn apply_mode_overlay_temperature() {
        let mut config = ChatConfig {
            temperature: 0.7,
            ..Default::default()
        };
        let mut mode = make_mode("creative", "For creative writing");
        mode.temperature_override = Some(0.95);

        apply_mode_overlay(&mut config, &mode);
        assert!((config.temperature - 0.95).abs() < f32::EPSILON);
    }

    #[test]
    fn apply_mode_overlay_model() {
        let mut config = ChatConfig {
            model_id: "claude-3-haiku".into(),
            ..Default::default()
        };
        let mut mode = make_mode("deep", "For deep analysis");
        mode.model_override = Some("claude-sonnet-4-20250514".into());

        apply_mode_overlay(&mut config, &mode);
        assert_eq!(config.model_id, "claude-sonnet-4-20250514");
    }

    #[test]
    fn apply_mode_overlay_tools() {
        let mut config = ChatConfig {
            tool_names: vec!["think".into(), "search".into()],
            ..Default::default()
        };
        let mut mode = make_mode("code", "For coding tasks");
        mode.tool_overrides = Some(vec!["think".into(), "write_file".into(), "run_test".into()]);

        apply_mode_overlay(&mut config, &mode);
        assert_eq!(config.tool_names, vec!["think", "write_file", "run_test"]);
    }

    #[test]
    fn apply_mode_overlay_no_overrides() {
        let mut config = ChatConfig {
            system_prompt: "Original.".into(),
            model_id: "haiku".into(),
            temperature: 0.5,
            tool_names: vec!["think".into()],
            ..Default::default()
        };
        let mode = make_mode("plain", "No overrides");

        apply_mode_overlay(&mut config, &mode);
        assert_eq!(config.system_prompt, "Original.");
        assert_eq!(config.model_id, "haiku");
        assert!((config.temperature - 0.5).abs() < f32::EPSILON);
        assert_eq!(config.tool_names, vec!["think"]);
    }
}
