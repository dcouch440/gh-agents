//! Configuration types

use super::message::VerbosityLevel;
use crate::constants::*;
use serde::{Deserialize, Serialize};

/// Agent pool size configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
pub struct AgentPoolConfig {
    /// Maximum total agents in the pool
    pub max_agents: u8,
}

impl Default for AgentPoolConfig {
    fn default() -> Self {
        Self { max_agents: 12 }
    }
}

/// Global configuration (from ~/.config/nexor/config.toml)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct GlobalConfig {
    #[serde(default)]
    pub verbosity: VerbosityLevel,
    #[serde(default)]
    pub pool: AgentPoolConfig,
}

/// Autonomy level for agent operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AutonomyLevel {
    /// No human approval needed
    FullAuto,
    /// Approval at configured points
    #[default]
    ApprovalGates,
    /// Human reviews each step
    Supervised,
}

/// Approval gates configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApprovalGates {
    pub before_commit: bool,
    pub before_pr: bool,
    pub before_merge: bool,
}

impl Default for ApprovalGates {
    fn default() -> Self {
        Self {
            before_commit: false,
            before_pr: true,
            before_merge: true,
        }
    }
}

/// Git branching strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum GitStrategy {
    #[default]
    BranchPerSlice,
    BranchPerTicket,
}

/// Sandbox execution mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SandboxMode {
    #[default]
    Docker,
    LocalRestricted,
    None,
}

/// Merge strategy for pull requests
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MergeStrategy {
    /// Create a merge commit
    #[default]
    Merge,
    /// Squash all commits into one
    Squash,
    /// Rebase commits onto base branch
    Rebase,
}

/// Configuration for automatic PR merging
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrMergeConfig {
    /// Enable automatic PR merging when toggled on
    #[serde(default)]
    pub auto_merge_enabled: bool,

    /// Preferred merge strategy
    #[serde(default)]
    pub merge_strategy: MergeStrategy,

    /// Require human approval before merging conflict resolutions
    #[serde(default = "default_require_approval")]
    pub require_approval_for_conflicts: bool,

    /// Maximum number of concurrent merge operations
    #[serde(default = "default_max_concurrent")]
    pub max_concurrent_merges: u8,

    /// Automatically delete branches after merge
    #[serde(default)]
    pub delete_branch_after_merge: bool,
}

fn default_require_approval() -> bool {
    true
}

fn default_max_concurrent() -> u8 {
    1
}

impl Default for PrMergeConfig {
    fn default() -> Self {
        Self {
            auto_merge_enabled: false,
            merge_strategy: MergeStrategy::default(),
            require_approval_for_conflicts: true,
            max_concurrent_merges: 1,
            delete_branch_after_merge: false,
        }
    }
}

/// Project-specific configuration (from .nexor/config.toml)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ProjectConfig {
    #[serde(default)]
    pub autonomy: AutonomyLevel,
    #[serde(default)]
    pub approval_gates: ApprovalGates,
    #[serde(default)]
    pub git_strategy: GitStrategy,
    #[serde(default)]
    pub sandbox_mode: SandboxMode,
    #[serde(default)]
    pub pool: Option<AgentPoolConfig>,
    #[serde(default)]
    pub pr_merge: PrMergeConfig,
}

/// Default Postgres connection URL
pub const DEFAULT_DATABASE_URL: &str = "postgres://nexor:nexor@localhost:5432/nexor";

/// Merged configuration (global + project)
#[derive(Debug, Clone, PartialEq)]
pub struct AppConfig {
    pub verbosity: VerbosityLevel,
    pub autonomy: AutonomyLevel,
    pub approval_gates: ApprovalGates,
    pub git_strategy: GitStrategy,
    pub sandbox_mode: SandboxMode,
    pub pool: AgentPoolConfig,
    pub pr_merge: PrMergeConfig,
    pub database_url: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            verbosity: VerbosityLevel::default(),
            autonomy: AutonomyLevel::default(),
            approval_gates: ApprovalGates::default(),
            git_strategy: GitStrategy::default(),
            sandbox_mode: SandboxMode::default(),
            pool: AgentPoolConfig::default(),
            pr_merge: PrMergeConfig::default(),
            database_url: DEFAULT_DATABASE_URL.to_string(),
        }
    }
}

impl AppConfig {
    /// Merge global and project configs (project overrides global)
    pub fn merge(global: GlobalConfig, project: Option<ProjectConfig>) -> Self {
        let database_url =
            std::env::var(ENV_DATABASE_URL).unwrap_or_else(|_| DEFAULT_DATABASE_URL.to_string());

        match project {
            Some(proj) => Self {
                verbosity: global.verbosity,
                autonomy: proj.autonomy,
                approval_gates: proj.approval_gates,
                git_strategy: proj.git_strategy,
                sandbox_mode: proj.sandbox_mode,
                pool: proj.pool.unwrap_or(global.pool),
                pr_merge: proj.pr_merge,
                database_url,
            },
            None => Self {
                verbosity: global.verbosity,
                autonomy: AutonomyLevel::default(),
                approval_gates: ApprovalGates::default(),
                git_strategy: GitStrategy::default(),
                sandbox_mode: SandboxMode::default(),
                pool: global.pool,
                pr_merge: PrMergeConfig::default(),
                database_url,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn autonomy_default_is_approval_gates() {
        assert_eq!(AutonomyLevel::default(), AutonomyLevel::ApprovalGates);
    }

    #[test]
    fn sandbox_default_is_docker() {
        assert_eq!(SandboxMode::default(), SandboxMode::Docker);
    }

    #[test]
    fn config_merge_uses_project_overrides() {
        let global = GlobalConfig::default();
        let project = ProjectConfig {
            autonomy: AutonomyLevel::FullAuto,
            ..Default::default()
        };
        let merged = AppConfig::merge(global, Some(project));
        assert_eq!(merged.autonomy, AutonomyLevel::FullAuto);
    }

    #[test]
    fn config_merge_uses_global_when_no_project() {
        let global = GlobalConfig::default();
        let merged = AppConfig::merge(global.clone(), None);
        assert_eq!(merged.pool, global.pool);
    }

    #[test]
    fn merge_strategy_default_is_merge() {
        assert_eq!(MergeStrategy::default(), MergeStrategy::Merge);
    }

    #[test]
    fn pr_merge_config_default() {
        let config = PrMergeConfig::default();
        assert!(!config.auto_merge_enabled);
        assert!(config.require_approval_for_conflicts);
        assert_eq!(config.merge_strategy, MergeStrategy::Merge);
        assert_eq!(config.max_concurrent_merges, 1);
        assert!(!config.delete_branch_after_merge);
    }

    #[test]
    fn config_merge_includes_pr_merge() {
        let global = GlobalConfig::default();
        let project = ProjectConfig {
            pr_merge: PrMergeConfig {
                auto_merge_enabled: true,
                merge_strategy: MergeStrategy::Squash,
                ..Default::default()
            },
            ..Default::default()
        };
        let merged = AppConfig::merge(global, Some(project));
        assert!(merged.pr_merge.auto_merge_enabled);
        assert_eq!(merged.pr_merge.merge_strategy, MergeStrategy::Squash);
    }

    #[test]
    fn config_default_database_url() {
        let config = AppConfig::default();
        assert_eq!(config.database_url, DEFAULT_DATABASE_URL);
    }

    #[test]
    fn config_merge_reads_database_url_env() {
        std::env::set_var(ENV_DATABASE_URL, "postgres://test:test@db:5432/testdb");
        let global = GlobalConfig::default();
        let merged = AppConfig::merge(global, None);
        assert_eq!(merged.database_url, "postgres://test:test@db:5432/testdb");
        std::env::remove_var(ENV_DATABASE_URL);
    }

    #[test]
    fn config_merge_falls_back_to_default_database_url() {
        std::env::remove_var(ENV_DATABASE_URL);
        let global = GlobalConfig::default();
        let merged = AppConfig::merge(global, None);
        assert_eq!(merged.database_url, DEFAULT_DATABASE_URL);
    }

    #[test]
    fn config_merge_database_url_with_project_config() {
        std::env::remove_var(ENV_DATABASE_URL);
        let global = GlobalConfig::default();
        let project = ProjectConfig {
            autonomy: AutonomyLevel::FullAuto,
            ..Default::default()
        };
        let merged = AppConfig::merge(global, Some(project));
        assert_eq!(merged.database_url, DEFAULT_DATABASE_URL);
    }

    #[test]
    fn config_merge_database_url_env_overrides_with_project() {
        std::env::set_var(ENV_DATABASE_URL, "postgres://custom:pw@host:5433/mydb");
        let global = GlobalConfig::default();
        let project = ProjectConfig::default();
        let merged = AppConfig::merge(global, Some(project));
        assert_eq!(merged.database_url, "postgres://custom:pw@host:5433/mydb");
        std::env::remove_var(ENV_DATABASE_URL);
    }

    #[test]
    fn default_database_url_is_valid_postgres_url() {
        assert!(DEFAULT_DATABASE_URL.starts_with("postgres://"));
        assert!(DEFAULT_DATABASE_URL.contains("nexor"));
        assert!(DEFAULT_DATABASE_URL.contains("5432"));
    }

    #[test]
    fn config_merge_preserves_global_verbosity() {
        let global = GlobalConfig {
            verbosity: VerbosityLevel::Verbose,
            ..Default::default()
        };
        let project = ProjectConfig::default();
        let merged = AppConfig::merge(global, Some(project));
        assert_eq!(merged.verbosity, VerbosityLevel::Verbose);
    }

    #[test]
    fn config_merge_project_pool_overrides_global() {
        let global = GlobalConfig::default();
        let project = ProjectConfig {
            pool: Some(AgentPoolConfig { max_agents: 5 }),
            ..Default::default()
        };
        let merged = AppConfig::merge(global, Some(project));
        assert_eq!(merged.pool.max_agents, 5);
    }

    #[test]
    fn config_merge_no_project_pool_uses_global() {
        let global = GlobalConfig {
            pool: AgentPoolConfig { max_agents: 20 },
            ..Default::default()
        };
        let merged = AppConfig::merge(global, None);
        assert_eq!(merged.pool.max_agents, 20);
    }

    #[test]
    fn git_strategy_default_is_branch_per_slice() {
        assert_eq!(GitStrategy::default(), GitStrategy::BranchPerSlice);
    }

    #[test]
    fn approval_gates_default_values() {
        let gates = ApprovalGates::default();
        assert!(!gates.before_commit);
        assert!(gates.before_pr);
        assert!(gates.before_merge);
    }

    #[test]
    fn agent_pool_config_default_values() {
        let pool = AgentPoolConfig::default();
        assert_eq!(pool.max_agents, 12);
    }
}
