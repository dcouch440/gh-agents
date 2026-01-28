//! Configuration types

use serde::{Deserialize, Serialize};

use super::agent::ModelConfig;
use super::message::VerbosityLevel;

/// Model configuration for each agent tier
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TierModels {
    pub orchestrator: ModelConfig,
    pub worker: ModelConfig,
    pub utility: ModelConfig,
}

impl Default for TierModels {
    fn default() -> Self {
        Self {
            orchestrator: ModelConfig {
                model_id: "claude-sonnet-4-20250514".to_string(),
                max_tokens: 8192,
                ..Default::default()
            },
            worker: ModelConfig {
                model_id: "claude-sonnet-4-20250514".to_string(),
                max_tokens: 4096,
                ..Default::default()
            },
            utility: ModelConfig {
                model_id: "claude-haiku".to_string(),
                max_tokens: 2048,
                ..Default::default()
            },
        }
    }
}

/// Agent pool size configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentPoolConfig {
    pub max_orchestrators: u8,
    pub max_workers: u8,
    pub max_utilities: u8,
}

impl Default for AgentPoolConfig {
    fn default() -> Self {
        Self {
            max_orchestrators: 2,
            max_workers: 6,
            max_utilities: 4,
        }
    }
}

/// Global configuration (from ~/.config/nexor/config.toml)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct GlobalConfig {
    #[serde(default)]
    pub default_models: TierModels,
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
    pub models: Option<TierModels>,
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

/// Merged configuration (global + project)
#[derive(Debug, Clone, PartialEq)]
pub struct AppConfig {
    pub models: TierModels,
    pub verbosity: VerbosityLevel,
    pub autonomy: AutonomyLevel,
    pub approval_gates: ApprovalGates,
    pub git_strategy: GitStrategy,
    pub sandbox_mode: SandboxMode,
    pub pool: AgentPoolConfig,
    pub pr_merge: PrMergeConfig,
}

impl AppConfig {
    /// Merge global and project configs (project overrides global)
    pub fn merge(global: GlobalConfig, project: Option<ProjectConfig>) -> Self {
        match project {
            Some(proj) => Self {
                models: proj.models.unwrap_or(global.default_models),
                verbosity: global.verbosity,
                autonomy: proj.autonomy,
                approval_gates: proj.approval_gates,
                git_strategy: proj.git_strategy,
                sandbox_mode: proj.sandbox_mode,
                pool: proj.pool.unwrap_or(global.pool),
                pr_merge: proj.pr_merge,
            },
            None => Self {
                models: global.default_models,
                verbosity: global.verbosity,
                autonomy: AutonomyLevel::default(),
                approval_gates: ApprovalGates::default(),
                git_strategy: GitStrategy::default(),
                sandbox_mode: SandboxMode::default(),
                pool: global.pool,
                pr_merge: PrMergeConfig::default(),
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
        assert_eq!(merged.models, global.default_models);
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
}
