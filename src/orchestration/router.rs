//! Task routing to appropriate agent tiers.
//!
//! The Router determines which agent tier (Orchestrator, Worker, or Utility)
//! should handle a given task, based on configurable routing rules and task
//! metadata hints.

use crate::types::{AgentTier, Task};

/// A single routing rule that maps task characteristics to a tier
#[derive(Debug, Clone)]
pub struct RoutingRule {
    pub name: String,
    pub matcher: RuleMatcher,
    pub target_tier: AgentTier,
    /// Higher priority rules are checked first
    pub priority: u8,
}

/// Rule matching criteria
#[derive(Debug, Clone)]
pub enum RuleMatcher {
    /// Match if task title contains any of these keywords (case-insensitive)
    TitleContains(Vec<String>),

    /// Match if task has specific metadata key
    HasMetadata(String),

    /// Match if task complexity exceeds threshold ("low", "medium", "high")
    ComplexityAbove(String),

    /// Match if task has explicit tier override in metadata or assigned_tier
    ExplicitTierOverride,

    /// Always matches (fallback rule)
    Always,
}

/// Configuration for the Router
#[derive(Debug, Clone)]
pub struct RouterConfig {
    pub rules: Vec<RoutingRule>,
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self {
            rules: vec![
                // Highest priority: explicit overrides
                RoutingRule {
                    name: "explicit_override".to_string(),
                    matcher: RuleMatcher::ExplicitTierOverride,
                    target_tier: AgentTier::Worker, // Ignored, will use override value
                    priority: 100,
                },
                // Utility tier: simple tasks
                RoutingRule {
                    name: "formatting_tasks".to_string(),
                    matcher: RuleMatcher::TitleContains(vec![
                        "format".to_string(),
                        "lint".to_string(),
                        "style".to_string(),
                    ]),
                    target_tier: AgentTier::Utility,
                    priority: 80,
                },
                RoutingRule {
                    name: "documentation_tasks".to_string(),
                    matcher: RuleMatcher::TitleContains(vec![
                        "docs".to_string(),
                        "documentation".to_string(),
                        "readme".to_string(),
                        "comment".to_string(),
                    ]),
                    target_tier: AgentTier::Utility,
                    priority: 80,
                },
                RoutingRule {
                    name: "boilerplate_tasks".to_string(),
                    matcher: RuleMatcher::TitleContains(vec![
                        "boilerplate".to_string(),
                        "scaffold".to_string(),
                        "template".to_string(),
                    ]),
                    target_tier: AgentTier::Utility,
                    priority: 80,
                },
                // Orchestrator tier: complex/review tasks
                RoutingRule {
                    name: "review_tasks".to_string(),
                    matcher: RuleMatcher::TitleContains(vec![
                        "review".to_string(),
                        "approve".to_string(),
                        "architect".to_string(),
                        "design".to_string(),
                        "plan".to_string(),
                    ]),
                    target_tier: AgentTier::Orchestrator,
                    priority: 70,
                },
                RoutingRule {
                    name: "high_complexity".to_string(),
                    matcher: RuleMatcher::ComplexityAbove("high".to_string()),
                    target_tier: AgentTier::Orchestrator,
                    priority: 60,
                },
                // Default: Worker tier
                RoutingRule {
                    name: "default_to_worker".to_string(),
                    matcher: RuleMatcher::Always,
                    target_tier: AgentTier::Worker,
                    priority: 0,
                },
            ],
        }
    }
}

/// Routes tasks to the appropriate agent tier based on configurable rules
pub struct Router {
    config: RouterConfig,
}

impl Router {
    /// Create a new Router with the given configuration
    pub fn new(config: RouterConfig) -> Self {
        Self { config }
    }

    /// Create a new Router with default routing rules
    pub fn with_defaults() -> Self {
        Self::new(RouterConfig::default())
    }

    /// Route a task to the appropriate agent tier
    pub fn route(&self, task: &Task) -> AgentTier {
        // Sort rules by priority (highest first)
        let mut rules: Vec<_> = self.config.rules.iter().collect();
        rules.sort_by(|a, b| b.priority.cmp(&a.priority));

        for rule in rules {
            if let Some(tier) = self.evaluate_rule(rule, task) {
                tracing::debug!(
                    task_id = %task.id.0,
                    rule = %rule.name,
                    tier = ?tier,
                    "Task routed"
                );
                return tier;
            }
        }

        // Should never reach here due to Always matcher, but fallback just in case
        tracing::warn!(
            task_id = %task.id.0,
            "No routing rule matched, defaulting to Worker"
        );
        AgentTier::Worker
    }

    /// Add a custom routing rule
    pub fn add_rule(&mut self, rule: RoutingRule) {
        self.config.rules.push(rule);
    }

    /// Get the current routing configuration
    pub fn config(&self) -> &RouterConfig {
        &self.config
    }

    fn evaluate_rule(&self, rule: &RoutingRule, task: &Task) -> Option<AgentTier> {
        match &rule.matcher {
            RuleMatcher::ExplicitTierOverride => self.get_tier_override(task),

            RuleMatcher::TitleContains(keywords) => {
                let title_lower = task.title.to_lowercase();
                if keywords
                    .iter()
                    .any(|k| title_lower.contains(&k.to_lowercase()))
                {
                    Some(rule.target_tier)
                } else {
                    None
                }
            }

            RuleMatcher::HasMetadata(key) => {
                if self.has_metadata(task, key) {
                    Some(rule.target_tier)
                } else {
                    None
                }
            }

            RuleMatcher::ComplexityAbove(threshold) => {
                if self.complexity_exceeds(task, threshold) {
                    Some(rule.target_tier)
                } else {
                    None
                }
            }

            RuleMatcher::Always => Some(rule.target_tier),
        }
    }

    fn get_tier_override(&self, task: &Task) -> Option<AgentTier> {
        // First check metadata for explicit override
        if let Some(tier_str) = self.get_metadata(task, "tier_override") {
            return match tier_str.to_lowercase().as_str() {
                "orchestrator" => Some(AgentTier::Orchestrator),
                "worker" => Some(AgentTier::Worker),
                "utility" => Some(AgentTier::Utility),
                _ => None,
            };
        }

        // Then check if assigned_tier was explicitly set to non-Worker
        // Worker is the default, so non-Worker indicates explicit override
        match task.assigned_tier {
            AgentTier::Worker => None,
            tier => Some(tier),
        }
    }

    fn has_metadata(&self, task: &Task, key: &str) -> bool {
        task.metadata
            .as_ref()
            .map(|m| m.contains_key(key))
            .unwrap_or(false)
    }

    fn get_metadata(&self, task: &Task, key: &str) -> Option<String> {
        task.metadata.as_ref().and_then(|m| m.get(key).cloned())
    }

    fn complexity_exceeds(&self, task: &Task, threshold: &str) -> bool {
        // Check metadata for explicit complexity first
        if let Some(complexity) = self.get_metadata(task, "complexity") {
            let threshold_value = Self::complexity_value(threshold);
            let task_value = Self::complexity_value(&complexity);
            return task_value >= threshold_value;
        }

        // Fall back to inference based on task characteristics
        self.infer_complexity(task, threshold)
    }

    fn complexity_value(level: &str) -> u8 {
        match level.to_lowercase().as_str() {
            "high" => 3,
            "medium" => 2,
            "low" => 1,
            _ => 2, // Default to medium
        }
    }

    fn infer_complexity(&self, task: &Task, threshold: &str) -> bool {
        // Infer complexity from task characteristics
        // - More context files = higher complexity
        // - Longer description = higher complexity
        let file_count = task.context_files.len();
        let desc_len = task.description.len();

        let inferred = if file_count > 5 || desc_len > 500 {
            3 // high
        } else if file_count > 2 || desc_len > 200 {
            2 // medium
        } else {
            1 // low
        };

        inferred >= Self::complexity_value(threshold)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Priority, TaskId, TaskStatus};
    use chrono::Utc;
    use std::collections::HashMap;

    fn make_task(title: &str, tier: AgentTier) -> Task {
        Task {
            id: TaskId::new(),
            slice_id: None,
            title: title.to_string(),
            description: String::new(),
            assigned_tier: tier,
            assigned_agent: None,
            status: TaskStatus::Pending,
            priority: Priority::Normal,
            context_files: vec![],
            metadata: None,
            depends_on: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn make_task_with_metadata(title: &str, metadata: HashMap<String, String>) -> Task {
        Task {
            id: TaskId::new(),
            slice_id: None,
            title: title.to_string(),
            description: String::new(),
            assigned_tier: AgentTier::Worker,
            assigned_agent: None,
            status: TaskStatus::Pending,
            priority: Priority::Normal,
            context_files: vec![],
            metadata: Some(metadata),
            depends_on: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn make_complex_task(title: &str, description: &str, file_count: usize) -> Task {
        Task {
            id: TaskId::new(),
            slice_id: None,
            title: title.to_string(),
            description: description.to_string(),
            assigned_tier: AgentTier::Worker,
            assigned_agent: None,
            status: TaskStatus::Pending,
            priority: Priority::Normal,
            context_files: (0..file_count)
                .map(|i| format!("file{}.rs", i).into())
                .collect(),
            metadata: None,
            depends_on: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    // === Basic Routing Tests ===

    #[test]
    fn routes_formatting_to_utility() {
        let router = Router::with_defaults();
        let task = make_task("Format the code files", AgentTier::Worker);

        assert_eq!(router.route(&task), AgentTier::Utility);
    }

    #[test]
    fn routes_lint_to_utility() {
        let router = Router::with_defaults();
        let task = make_task("Run linting on source files", AgentTier::Worker);

        assert_eq!(router.route(&task), AgentTier::Utility);
    }

    #[test]
    fn routes_docs_to_utility() {
        let router = Router::with_defaults();
        let task = make_task("Update documentation for API", AgentTier::Worker);

        assert_eq!(router.route(&task), AgentTier::Utility);
    }

    #[test]
    fn routes_review_to_orchestrator() {
        let router = Router::with_defaults();
        let task = make_task("Review the authentication changes", AgentTier::Worker);

        assert_eq!(router.route(&task), AgentTier::Orchestrator);
    }

    #[test]
    fn routes_design_to_orchestrator() {
        let router = Router::with_defaults();
        let task = make_task("Design the new architecture", AgentTier::Worker);

        assert_eq!(router.route(&task), AgentTier::Orchestrator);
    }

    #[test]
    fn routes_feature_to_worker() {
        let router = Router::with_defaults();
        let task = make_task("Implement user login endpoint", AgentTier::Worker);

        assert_eq!(router.route(&task), AgentTier::Worker);
    }

    #[test]
    fn routes_bug_fix_to_worker() {
        let router = Router::with_defaults();
        let task = make_task("Fix null pointer exception", AgentTier::Worker);

        assert_eq!(router.route(&task), AgentTier::Worker);
    }

    // === Explicit Override Tests ===

    #[test]
    fn respects_assigned_tier_override() {
        let router = Router::with_defaults();
        // Task explicitly assigned to Utility by Planner
        let task = make_task("Implement something complex", AgentTier::Utility);

        // Should respect the override even though title doesn't match utility keywords
        assert_eq!(router.route(&task), AgentTier::Utility);
    }

    #[test]
    fn metadata_tier_override_takes_precedence() {
        let router = Router::with_defaults();

        let mut metadata = HashMap::new();
        metadata.insert("tier_override".to_string(), "utility".to_string());

        let task = make_task_with_metadata("Complex implementation task", metadata);

        // Should respect metadata override
        assert_eq!(router.route(&task), AgentTier::Utility);
    }

    #[test]
    fn metadata_override_orchestrator() {
        let router = Router::with_defaults();

        let mut metadata = HashMap::new();
        metadata.insert("tier_override".to_string(), "orchestrator".to_string());

        let task = make_task_with_metadata("Simple formatting task", metadata);

        // Should respect metadata override over keyword match
        assert_eq!(router.route(&task), AgentTier::Orchestrator);
    }

    // === Complexity-Based Routing Tests ===

    #[test]
    fn high_complexity_metadata_routes_to_orchestrator() {
        let router = Router::with_defaults();

        let mut metadata = HashMap::new();
        metadata.insert("complexity".to_string(), "high".to_string());

        let task = make_task_with_metadata("Simple sounding task", metadata);

        // High complexity should route to orchestrator
        assert_eq!(router.route(&task), AgentTier::Orchestrator);
    }

    #[test]
    fn inferred_high_complexity_routes_to_orchestrator() {
        let router = Router::with_defaults();
        // Task with 6+ context files should infer high complexity
        let task = make_complex_task("Implement new feature", "", 7);

        assert_eq!(router.route(&task), AgentTier::Orchestrator);
    }

    #[test]
    fn inferred_high_complexity_by_description() {
        let router = Router::with_defaults();
        // Task with long description should infer high complexity
        let long_desc = "x".repeat(600);
        let task = make_complex_task("Implement new feature", &long_desc, 0);

        assert_eq!(router.route(&task), AgentTier::Orchestrator);
    }

    #[test]
    fn low_complexity_routes_to_worker() {
        let router = Router::with_defaults();
        // Simple task with few files and short description
        let task = make_complex_task("Add a button", "Add submit button", 1);

        assert_eq!(router.route(&task), AgentTier::Worker);
    }

    // === Custom Rules Tests ===

    #[test]
    fn custom_rule_can_be_added() {
        let mut router = Router::with_defaults();

        // Add custom rule for "security" tasks
        router.add_rule(RoutingRule {
            name: "security_tasks".to_string(),
            matcher: RuleMatcher::TitleContains(vec!["security".to_string()]),
            target_tier: AgentTier::Orchestrator,
            priority: 90, // High priority
        });

        let task = make_task("Security audit of auth module", AgentTier::Worker);
        assert_eq!(router.route(&task), AgentTier::Orchestrator);
    }

    #[test]
    fn has_metadata_rule_works() {
        let router = Router::new(RouterConfig {
            rules: vec![
                RoutingRule {
                    name: "urgent_flag".to_string(),
                    matcher: RuleMatcher::HasMetadata("urgent".to_string()),
                    target_tier: AgentTier::Orchestrator,
                    priority: 90,
                },
                RoutingRule {
                    name: "default".to_string(),
                    matcher: RuleMatcher::Always,
                    target_tier: AgentTier::Worker,
                    priority: 0,
                },
            ],
        });

        let mut metadata = HashMap::new();
        metadata.insert("urgent".to_string(), "true".to_string());

        let task = make_task_with_metadata("Regular task", metadata);
        assert_eq!(router.route(&task), AgentTier::Orchestrator);
    }

    // === Edge Cases ===

    #[test]
    fn case_insensitive_matching() {
        let router = Router::with_defaults();
        let task = make_task("FORMAT THE CODE", AgentTier::Worker);

        assert_eq!(router.route(&task), AgentTier::Utility);
    }

    #[test]
    fn empty_title_routes_to_worker() {
        let router = Router::with_defaults();
        let task = make_task("", AgentTier::Worker);

        assert_eq!(router.route(&task), AgentTier::Worker);
    }

    #[test]
    fn no_metadata_still_works() {
        let router = Router::with_defaults();
        let task = make_task("Implement feature", AgentTier::Worker);
        assert!(task.metadata.is_none());

        // Should still route correctly
        assert_eq!(router.route(&task), AgentTier::Worker);
    }

    #[test]
    fn default_config_has_all_tiers() {
        let config = RouterConfig::default();

        let has_orchestrator = config
            .rules
            .iter()
            .any(|r| r.target_tier == AgentTier::Orchestrator);
        let has_worker = config
            .rules
            .iter()
            .any(|r| r.target_tier == AgentTier::Worker);
        let has_utility = config
            .rules
            .iter()
            .any(|r| r.target_tier == AgentTier::Utility);

        assert!(has_orchestrator, "Should have orchestrator rules");
        assert!(has_worker, "Should have worker rules");
        assert!(has_utility, "Should have utility rules");
    }

    #[test]
    fn rules_ordered_by_priority() {
        let config = RouterConfig::default();

        // The explicit_override rule should have highest priority
        let explicit_rule = config
            .rules
            .iter()
            .find(|r| r.name == "explicit_override")
            .unwrap();
        let default_rule = config
            .rules
            .iter()
            .find(|r| r.name == "default_to_worker")
            .unwrap();

        assert!(explicit_rule.priority > default_rule.priority);
    }
}
