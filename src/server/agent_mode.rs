//! Agent mode definitions and registry.
//!
//! Each agent mode configures how the orchestrator handles a chat session:
//! system prompt, available tools, mounted files, and history policy.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Identifies an agent mode (e.g., "home", "planning", "agent_builder", "decomp").
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentModeId(pub String);

impl AgentModeId {
    pub fn new(id: &str) -> Self {
        Self(id.to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for AgentModeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// How chat history is loaded for a mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HistoryPolicy {
    /// No history loaded — context comes from tools and mounted files.
    None,
    /// Load history scoped to the current session.
    SessionScoped { max_messages: u32 },
}

/// Configuration for a chat mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMode {
    pub id: AgentModeId,
    pub name: String,
    pub description: String,
    pub system_prompt: String,
    /// Tool names to include from `agent_tools()`. Empty = all tools.
    pub tools: Vec<String>,
    /// File paths loaded as additional system prompt context.
    pub mounted_files: Vec<String>,
    pub history_policy: HistoryPolicy,
}

/// Registry of available agent modes, built at startup.
pub struct ModeRegistry {
    modes: HashMap<AgentModeId, AgentMode>,
}

impl ModeRegistry {
    /// Create the registry with the default built-in modes.
    pub fn new() -> Self {
        let mut modes = HashMap::new();

        let home = AgentMode {
            id: AgentModeId::new("home"),
            name: "Home".to_string(),
            description: "Project-level assistant. Check agent status, browse PRDs, manage files."
                .to_string(),
            system_prompt: "You are nexor, the central AI command center for software engineering teams. \
                You coordinate a multi-tier agent system: Orchestrators (Tier 2, planning/architecture), \
                Workers (Tier 1, implementation), and Utilities (Tier 0, quick tasks). \
                You can check agent pool status, browse project artifacts (PRDs, tickets, roadmaps), \
                manage files, and spin up workflows. Use ASCII diagrams to explain system state. \
                Be direct and technical. When the user asks about capabilities, show them — \
                don't just tell them."
                .to_string(),
            tools: vec![], // all tools available
            mounted_files: vec![],
            history_policy: HistoryPolicy::None,
        };

        let planning = AgentMode {
            id: AgentModeId::new("planning"),
            name: "Planning".to_string(),
            description: "Build and refine a PRD collaboratively.".to_string(),
            system_prompt: include_str!("prompts/planning.txt").to_string(),
            tools: vec![],
            mounted_files: vec![],
            history_policy: HistoryPolicy::SessionScoped { max_messages: 30 },
        };

        let agent_builder = AgentMode {
            id: AgentModeId::new("agent_builder"),
            name: "Agent Builder".to_string(),
            description: "Create and configure agents, assign tasks, define roles.".to_string(),
            system_prompt: include_str!("prompts/agent_builder.txt").to_string(),
            tools: vec![
                "list_agents".to_string(),
                "list_roles".to_string(),
                "create_agent".to_string(),
                "create_agents".to_string(),
                "remove_agent".to_string(),
                "assign_task".to_string(),
                "get_task_result".to_string(),
                "list_pending_approvals".to_string(),
                "respond_to_approval".to_string(),
                "create_cluster".to_string(),
                "add_to_cluster".to_string(),
                "remove_from_cluster".to_string(),
                "list_clusters".to_string(),
                "create_pipeline".to_string(),
                "add_pipeline_stage".to_string(),
                "start_pipeline".to_string(),
                "get_pipeline_status".to_string(),
                "create_schedule".to_string(),
                "list_schedules".to_string(),
                "toggle_schedule".to_string(),
                "create_trigger".to_string(),
                "list_triggers".to_string(),
                "read_file".to_string(),
                "list_files".to_string(),
            ],
            mounted_files: vec![],
            history_policy: HistoryPolicy::SessionScoped { max_messages: 20 },
        };

        let decomp = AgentMode {
            id: AgentModeId::new("decomp"),
            name: "Decomposition".to_string(),
            description: "Break an approved PRD into implementation tickets.".to_string(),
            system_prompt: "You are nexor's Decomposition agent. Given an approved PRD, \
                break it into implementation tickets as a multi-stage pipeline.\n\n\
                For each ticket:\n\
                - Title and one-paragraph description\n\
                - Acceptance criteria (testable, specific)\n\
                - Files expected to be created or modified\n\
                - Dependencies on other tickets\n\
                - Complexity: S/M/L/XL\n\
                - Suggested role: worker, reviewer, or utility\n\n\
                Then build the execution pipeline: create agents, add pipeline stages in dependency order, \
                and set approval gates before risky stages. Present the pipeline as a diagram before starting it."
                .to_string(),
            tools: vec![
                "create_pipeline".to_string(),
                "add_pipeline_stage".to_string(),
                "start_pipeline".to_string(),
                "get_pipeline_status".to_string(),
                "list_agents".to_string(),
                "assign_task".to_string(),
            ],
            mounted_files: vec![],
            history_policy: HistoryPolicy::None,
        };

        modes.insert(home.id.clone(), home);
        modes.insert(planning.id.clone(), planning);
        modes.insert(agent_builder.id.clone(), agent_builder);
        modes.insert(decomp.id.clone(), decomp);

        Self { modes }
    }

    /// Get a mode by ID.
    pub fn get(&self, id: &AgentModeId) -> Option<&AgentMode> {
        self.modes.get(id)
    }

    /// List all available modes.
    pub fn list(&self) -> Vec<&AgentMode> {
        self.modes.values().collect()
    }

    /// Get the default mode ID ("home").
    pub fn default_mode_id() -> AgentModeId {
        AgentModeId::new("home")
    }
}

impl Default for ModeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_has_default_modes() {
        let registry = ModeRegistry::new();
        assert!(registry.get(&AgentModeId::new("home")).is_some());
        assert!(registry.get(&AgentModeId::new("planning")).is_some());
        assert!(registry.get(&AgentModeId::new("agent_builder")).is_some());
        assert!(registry.get(&AgentModeId::new("decomp")).is_some());
    }

    #[test]
    fn list_returns_all_modes() {
        let registry = ModeRegistry::new();
        assert_eq!(registry.list().len(), 4);
    }

    #[test]
    fn home_mode_has_no_history() {
        let registry = ModeRegistry::new();
        let home = registry.get(&AgentModeId::new("home")).unwrap();
        assert!(matches!(home.history_policy, HistoryPolicy::None));
    }

    #[test]
    fn planning_mode_has_session_history() {
        let registry = ModeRegistry::new();
        let planning = registry.get(&AgentModeId::new("planning")).unwrap();
        assert!(matches!(
            planning.history_policy,
            HistoryPolicy::SessionScoped { max_messages: 30 }
        ));
    }

    #[test]
    fn agent_builder_has_filtered_tools() {
        let registry = ModeRegistry::new();
        let ab = registry.get(&AgentModeId::new("agent_builder")).unwrap();
        assert!(!ab.tools.is_empty());
        assert!(ab.tools.contains(&"create_agent".to_string()));
    }

    #[test]
    fn unknown_mode_returns_none() {
        let registry = ModeRegistry::new();
        assert!(registry.get(&AgentModeId::new("nonexistent")).is_none());
    }
}
