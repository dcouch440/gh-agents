//! Mode resolution service for router-based mode selection.
//!
//! Data-only: resolves agent + input → ResolvedModeConfig.
//! Does NOT create strategies or call ExecutionEngine.

use std::collections::HashSet;
use std::sync::Arc;
use uuid::Uuid;

use crate::db::traits::{ServerRepo, ToolRouterRepo};
use crate::db::{AgentRow, ToolRouterModeRow};
use crate::llm::{LLMProvider, Tool};
use crate::server::hub::engine::ExecutionEngine;
use crate::server::hub::recorder::ExecutionRecorder;
use crate::server::hub::strategies::router::{RouterConfig, RouterStrategy};
use crate::server::hub::streaming::NullSink;
use crate::tools::registry;

#[cfg(test)]
mod tests;

/// Resolved configuration after mode classification.
#[derive(Debug, Clone)]
pub struct ResolvedModeConfig {
    pub system_prompt: String,
    pub tools: Vec<Tool>,
    pub tool_names: Vec<String>,
    pub temperature: f32,
    pub max_tokens: i32,
    pub selected_mode_id: Option<Uuid>,
    pub selected_mode_key: Option<String>,
}

/// Error types for mode resolution.
#[derive(Debug, thiserror::Error)]
pub enum RoutingError {
    #[error("router not found")]
    RouterNotFound,

    #[error("router has no modes configured")]
    NoModesConfigured,

    #[error("database error: {0}")]
    Database(#[from] anyhow::Error),

    #[error("LLM call failed: {0}")]
    LlmFailed(String),
}

/// Mode resolution service.
pub struct ModeResolver {
    repo: Arc<dyn ServerRepo>,
    tool_router_repo: Arc<dyn ToolRouterRepo>,
    provider: Arc<dyn LLMProvider>,
}

impl ModeResolver {
    pub fn new(
        repo: Arc<dyn ServerRepo>,
        tool_router_repo: Arc<dyn ToolRouterRepo>,
        provider: Arc<dyn LLMProvider>,
    ) -> Self {
        Self {
            repo,
            tool_router_repo,
            provider,
        }
    }

    /// Resolve mode config for an agent.
    ///
    /// # Parameters
    /// - `agent`: Agent row (already loaded by caller)
    /// - `user_input`: User's message for classification
    /// - `context_hint`: Optional context for router LLM
    ///   - Chat: formatted conversation history
    ///   - Rooms: formatted transcript
    ///   - DAG: step description
    ///
    /// # Returns
    /// Agent defaults if agent has no router_id.
    /// Routed config if agent has router_id.
    pub async fn resolve(
        &self,
        agent: &AgentRow,
        user_input: &str,
        context_hint: Option<&str>,
    ) -> Result<ResolvedModeConfig, RoutingError> {
        // 1. If no router_id → return agent defaults
        let router_id = match agent.router_id {
            Some(id) => id,
            None => return self.agent_defaults(agent).await,
        };

        // 2. Load router + modes
        let router = self
            .tool_router_repo
            .get_tool_router(router_id)
            .await
            .map_err(|e| RoutingError::Database(e.into()))?
            .ok_or(RoutingError::RouterNotFound)?;

        let modes = self
            .tool_router_repo
            .list_router_modes(router_id)
            .await
            .map_err(|e| RoutingError::Database(e.into()))?;

        if modes.is_empty() {
            return Err(RoutingError::NoModesConfigured);
        }

        // 3. Build classification prompt
        let prompt = build_classification_prompt(user_input, context_hint, &modes);

        // 4. Call router LLM
        let router_config = RouterConfig {
            system_prompt: router.system_prompt.clone(),
            model_id: router.model_id.clone(),
            state: None,
            user_id: None,
        };

        let strategy = RouterStrategy::new(router_config);
        let engine = ExecutionEngine::new(self.provider.clone());
        let sink = NullSink;
        let recorder = ExecutionRecorder::new(self.repo.as_ref(), None, None);

        let result = engine
            .execute(&strategy, &prompt, &sink, &recorder, None)
            .await
            .map_err(|e| RoutingError::LlmFailed(e.to_string()))?;

        // 5. Parse mode key (with fallback)
        let mode_key = parse_mode_key(&result.content).unwrap_or_else(|| {
            tracing::warn!("Router returned invalid JSON, falling back to first mode");
            modes[0].mode_key.clone()
        });

        // 6. Find mode (with fallback)
        let mode = modes
            .iter()
            .find(|m| m.mode_key == mode_key)
            .cloned()
            .unwrap_or_else(|| {
                tracing::warn!(
                    "Router selected unknown mode '{}', using first mode",
                    mode_key
                );
                modes[0].clone()
            });

        // 7. Load mode tools
        let mode_tool_rows = self
            .tool_router_repo
            .get_mode_tools(mode.id)
            .await
            .map_err(|e| RoutingError::Database(e.into()))?;

        let mode_tools: Vec<Tool> = mode_tool_rows
            .iter()
            .filter_map(|row| registry::get_tool_definition(&row.name))
            .collect();

        // 8. Merge system prompt (append or replace)
        let system_prompt = if mode.append_to_agent_system_prompt {
            format!("{}\n\n{}", agent.system_prompt, mode.system_prompt)
        } else {
            mode.system_prompt.clone()
        };

        // 9. Merge tools (union or replace)
        let tools = if mode.append_to_agent_tools {
            let agent_tool_rows = self
                .repo
                .get_agent_tools(agent.id)
                .await
                .map_err(|e| RoutingError::Database(e.into()))?;

            let agent_tools: Vec<Tool> = agent_tool_rows
                .iter()
                .filter_map(|row| registry::get_tool_definition(&row.name))
                .collect();

            union_by_name(agent_tools, mode_tools)
        } else {
            mode_tools
        };

        let tool_names = tools.iter().map(|t| t.name.clone()).collect();

        // 10. Return resolved config
        Ok(ResolvedModeConfig {
            system_prompt,
            tools,
            tool_names,
            temperature: mode.temperature,
            max_tokens: mode.max_tokens,
            selected_mode_id: Some(mode.id),
            selected_mode_key: Some(mode.mode_key.clone()),
        })
    }

    /// Return agent's default config (no router).
    async fn agent_defaults(&self, agent: &AgentRow) -> Result<ResolvedModeConfig, RoutingError> {
        let agent_tool_rows = self
            .repo
            .get_agent_tools(agent.id)
            .await
            .map_err(|e| RoutingError::Database(e.into()))?;

        let tools: Vec<Tool> = agent_tool_rows
            .iter()
            .filter_map(|row| registry::get_tool_definition(&row.name))
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
}

/// Build classification prompt with context + input + modes.
fn build_classification_prompt(
    user_input: &str,
    context_hint: Option<&str>,
    modes: &[ToolRouterModeRow],
) -> String {
    let context_block = context_hint
        .map(|c| format!("## Context:\n{}\n\n", c))
        .unwrap_or_default();

    let mode_list = modes
        .iter()
        .map(|m| format!("- {}: {}", m.mode_key, m.description))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "{}## Current User Input:\n{}\n\n## Available Modes:\n{}\n\n\
         Based on the context and current input, output ONLY the mode key.",
        context_block, user_input, mode_list
    )
}

/// Parse mode key from router LLM response (expects JSON).
fn parse_mode_key(response: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(response.trim())
        .ok()
        .and_then(|v| v.get("mode").and_then(|m| m.as_str().map(String::from)))
}

/// Union two tool vectors by name (deduplicates).
fn union_by_name(mut agent_tools: Vec<Tool>, mode_tools: Vec<Tool>) -> Vec<Tool> {
    let mut seen: HashSet<String> = agent_tools.iter().map(|t| t.name.clone()).collect();

    for tool in mode_tools {
        if seen.insert(tool.name.clone()) {
            agent_tools.push(tool);
        }
    }

    agent_tools
}
