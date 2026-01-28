//! Agent-related types

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for an agent
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(pub Uuid);

impl AgentId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for AgentId {
    fn default() -> Self {
        Self::new()
    }
}

/// Agent tier hierarchy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AgentTier {
    /// Expensive AI for planning, review, decisions
    Orchestrator,
    /// Mid-tier AI for code implementation
    #[default]
    Worker,
    /// Cheap AI for formatting, linting, boilerplate
    Utility,
}

/// Agent operational status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    #[default]
    Idle,
    Working,
    WaitingForContext,
    WaitingForApproval,
}

/// Communication style for agent personas
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum CommunicationStyle {
    Formal,
    #[default]
    Casual,
    Technical,
    Friendly,
}

/// LLM provider options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum LLMProvider {
    #[default]
    Anthropic,
}

/// Model configuration for an agent
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelConfig {
    pub provider: LLMProvider,
    pub model_id: String,
    pub max_tokens: u32,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
}

fn default_temperature() -> f32 {
    0.7
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            provider: LLMProvider::Anthropic,
            model_id: "claude-sonnet-4-20250514".to_string(),
            max_tokens: 4096,
            temperature: default_temperature(),
        }
    }
}

/// Agent personality configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentPersona {
    pub name: String,
    pub system_prompt: String,
    pub style: CommunicationStyle,
}

impl Default for AgentPersona {
    fn default() -> Self {
        Self {
            name: "Agent".to_string(),
            system_prompt: String::new(),
            style: CommunicationStyle::default(),
        }
    }
}

/// An AI agent instance
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Agent {
    pub id: AgentId,
    pub tier: AgentTier,
    pub persona: AgentPersona,
    pub model_config: ModelConfig,
    pub current_task: Option<super::task::TaskId>,
    pub status: AgentStatus,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_tier_default_is_worker() {
        assert_eq!(AgentTier::default(), AgentTier::Worker);
    }

    #[test]
    fn agent_status_default_is_idle() {
        assert_eq!(AgentStatus::default(), AgentStatus::Idle);
    }

    #[test]
    fn model_config_has_sensible_defaults() {
        let config = ModelConfig::default();
        assert_eq!(config.provider, LLMProvider::Anthropic);
        assert_eq!(config.max_tokens, 4096);
    }
}
