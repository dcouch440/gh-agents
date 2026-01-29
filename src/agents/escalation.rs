//! Escalation flow for routing task failures up the tier hierarchy

use std::collections::HashMap;
use tracing::{info, warn};
use uuid::Uuid;

use crate::types::AgentTier;

/// Configuration for escalation behavior
#[derive(Debug, Clone)]
pub struct EscalationPolicy {
    /// Number of retries at Utility tier before escalating
    pub utility_retries: u32,
    /// Number of retries at Worker tier before escalating
    pub worker_retries: u32,
    /// Number of retries at Orchestrator tier before escalating to human
    pub orchestrator_retries: u32,
}

impl Default for EscalationPolicy {
    fn default() -> Self {
        Self {
            utility_retries: 1,      // Utilities fail fast
            worker_retries: 2,       // Workers get a couple tries
            orchestrator_retries: 1, // Orchestrators escalate to human quickly
        }
    }
}

/// Decision from escalation evaluation
#[derive(Debug, Clone, PartialEq)]
pub enum EscalationDecision {
    /// Retry at the same tier
    Retry,
    /// Escalate to a higher tier
    Escalate(AgentTier),
    /// Task needs human intervention
    NeedsHuman,
}

impl EscalationPolicy {
    /// Create a new policy with custom thresholds
    pub fn new(utility_retries: u32, worker_retries: u32, orchestrator_retries: u32) -> Self {
        Self {
            utility_retries,
            worker_retries,
            orchestrator_retries,
        }
    }

    /// Determine what to do with a failed task
    ///
    /// # Arguments
    /// * `current_tier` - The tier of the agent that failed
    /// * `failure_count` - Number of times this task has failed at this tier
    ///
    /// # Returns
    /// Decision on whether to retry, escalate, or request human help
    pub fn evaluate(&self, current_tier: AgentTier, failure_count: u32) -> EscalationDecision {
        let max_retries = self.max_retries_for_tier(current_tier);

        if failure_count <= max_retries {
            return EscalationDecision::Retry;
        }

        // Exceeded retries, escalate
        match self.next_tier(current_tier) {
            Some(tier) => EscalationDecision::Escalate(tier),
            None => EscalationDecision::NeedsHuman,
        }
    }

    /// Get the maximum retries for a tier
    pub fn max_retries_for_tier(&self, tier: AgentTier) -> u32 {
        match tier {
            AgentTier::Utility => self.utility_retries,
            AgentTier::Worker => self.worker_retries,
            AgentTier::Orchestrator => self.orchestrator_retries,
        }
    }

    /// Get the next tier in the escalation chain
    pub fn next_tier(&self, current: AgentTier) -> Option<AgentTier> {
        match current {
            AgentTier::Utility => Some(AgentTier::Worker),
            AgentTier::Worker => Some(AgentTier::Orchestrator),
            AgentTier::Orchestrator => None, // Escalates to human
        }
    }

    /// Get the full escalation path from a tier
    pub fn escalation_path(&self, from: AgentTier) -> Vec<AgentTier> {
        let mut path = vec![from];
        let mut current = from;

        while let Some(next) = self.next_tier(current) {
            path.push(next);
            current = next;
        }

        path
    }
}

/// Tracks failure state for a task across escalations
#[derive(Debug, Clone)]
pub struct TaskEscalationState {
    /// Task ID
    pub task_id: Uuid,
    /// Current tier attempting the task
    pub current_tier: AgentTier,
    /// Number of failures at current tier
    pub failure_count_at_tier: u32,
    /// Total failure count across all tiers
    pub total_failure_count: u32,
    /// History of tiers that have attempted this task
    pub tier_history: Vec<TierAttempt>,
}

/// Record of an attempt at a tier
#[derive(Debug, Clone)]
pub struct TierAttempt {
    pub tier: AgentTier,
    pub attempts: u32,
    pub final_error: String,
}

impl TaskEscalationState {
    /// Create new escalation state for a task
    pub fn new(task_id: Uuid, initial_tier: AgentTier) -> Self {
        Self {
            task_id,
            current_tier: initial_tier,
            failure_count_at_tier: 0,
            total_failure_count: 0,
            tier_history: Vec::new(),
        }
    }

    /// Record a failure at the current tier
    pub fn record_failure(&mut self, error: &str) {
        self.failure_count_at_tier += 1;
        self.total_failure_count += 1;

        // Update or add tier attempt record
        if let Some(attempt) = self
            .tier_history
            .iter_mut()
            .find(|a| a.tier == self.current_tier)
        {
            attempt.attempts += 1;
            attempt.final_error = error.to_string();
        } else {
            self.tier_history.push(TierAttempt {
                tier: self.current_tier,
                attempts: 1,
                final_error: error.to_string(),
            });
        }
    }

    /// Escalate to a new tier
    pub fn escalate_to(&mut self, new_tier: AgentTier) {
        self.current_tier = new_tier;
        self.failure_count_at_tier = 0;
    }
}

/// Manages escalation decisions across tasks
pub struct EscalationManager {
    /// Escalation policy
    policy: EscalationPolicy,
    /// Escalation state per task
    task_states: HashMap<Uuid, TaskEscalationState>,
}

impl EscalationManager {
    /// Create a new escalation manager
    pub fn new(policy: EscalationPolicy) -> Self {
        Self {
            policy,
            task_states: HashMap::new(),
        }
    }

    /// Create with default policy
    pub fn with_default_policy() -> Self {
        Self::new(EscalationPolicy::default())
    }

    /// Start tracking a new task
    pub fn track_task(&mut self, task_id: Uuid, initial_tier: AgentTier) {
        self.task_states
            .insert(task_id, TaskEscalationState::new(task_id, initial_tier));
    }

    /// Handle a task failure and return the decision
    pub fn on_task_failed(
        &mut self,
        task_id: Uuid,
        current_tier: AgentTier,
        error: &str,
    ) -> EscalationDecision {
        // Get or create task state
        let state = self
            .task_states
            .entry(task_id)
            .or_insert_with(|| TaskEscalationState::new(task_id, current_tier));

        // Ensure we're tracking the right tier
        if state.current_tier != current_tier {
            state.escalate_to(current_tier);
        }

        // Record the failure
        state.record_failure(error);

        // Evaluate the policy
        let decision = self
            .policy
            .evaluate(state.current_tier, state.failure_count_at_tier);

        // Log the decision
        match &decision {
            EscalationDecision::Retry => {
                info!(
                    task_id = ?task_id,
                    tier = ?current_tier,
                    failure_count = state.failure_count_at_tier,
                    "Task will be retried at same tier"
                );
            }
            EscalationDecision::Escalate(next_tier) => {
                warn!(
                    task_id = ?task_id,
                    from_tier = ?current_tier,
                    to_tier = ?next_tier,
                    total_failures = state.total_failure_count,
                    "Task escalating to higher tier"
                );
                // Update state for next attempt
                state.escalate_to(*next_tier);
            }
            EscalationDecision::NeedsHuman => {
                warn!(
                    task_id = ?task_id,
                    tier = ?current_tier,
                    total_failures = state.total_failure_count,
                    "Task requires human intervention"
                );
            }
        }

        decision
    }

    /// Mark a task as successfully completed
    pub fn on_task_completed(&mut self, task_id: Uuid) {
        if let Some(state) = self.task_states.remove(&task_id) {
            info!(
                task_id = ?task_id,
                final_tier = ?state.current_tier,
                total_failures = state.total_failure_count,
                "Task completed successfully"
            );
        }
    }

    /// Get the current state for a task
    pub fn get_state(&self, task_id: &Uuid) -> Option<&TaskEscalationState> {
        self.task_states.get(task_id)
    }

    /// Get all tasks that need human review
    pub fn tasks_needing_human(&self) -> Vec<Uuid> {
        self.task_states
            .iter()
            .filter(|(_, state)| {
                let decision = self
                    .policy
                    .evaluate(state.current_tier, state.failure_count_at_tier);
                matches!(decision, EscalationDecision::NeedsHuman)
            })
            .map(|(id, _)| *id)
            .collect()
    }

    /// Clear state for a task
    pub fn clear_task(&mut self, task_id: &Uuid) {
        self.task_states.remove(task_id);
    }

    /// Create a human review request for a task
    pub fn create_human_review_request(
        &self,
        task_id: Uuid,
        title: &str,
        description: &str,
    ) -> Option<HumanReviewRequest> {
        let state = self.task_states.get(&task_id)?;

        // Build reason from last error
        let reason = state
            .tier_history
            .last()
            .map(|a| {
                format!(
                    "Failed at {:?} tier after {} attempts: {}",
                    a.tier, a.attempts, a.final_error
                )
            })
            .unwrap_or_else(|| "Unknown failure".to_string());

        Some(HumanReviewRequest {
            task_id,
            title: title.to_string(),
            description: description.to_string(),
            escalation_history: state.tier_history.clone(),
            total_attempts: state.total_failure_count,
            reason,
            requested_at: chrono::Utc::now(),
        })
    }

    /// Get summary of all human review requests
    pub fn human_review_summary(&self) -> Vec<HumanReviewSummary> {
        self.task_states
            .iter()
            .filter_map(|(task_id, state)| {
                let decision = self
                    .policy
                    .evaluate(state.current_tier, state.failure_count_at_tier);

                if matches!(decision, EscalationDecision::NeedsHuman) {
                    Some(HumanReviewSummary {
                        task_id: *task_id,
                        total_failures: state.total_failure_count,
                        tiers_attempted: state.tier_history.iter().map(|a| a.tier).collect(),
                        last_error: state
                            .tier_history
                            .last()
                            .map(|a| a.final_error.clone())
                            .unwrap_or_default(),
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    /// Apply a human action to a task
    pub fn apply_human_action(
        &mut self,
        task_id: Uuid,
        action: HumanAction,
    ) -> Result<(), EscalationError> {
        let state = self
            .task_states
            .get_mut(&task_id)
            .ok_or(EscalationError::TaskNotFound(task_id))?;

        match action {
            HumanAction::RetryAt(tier) => {
                info!(task_id = ?task_id, tier = ?tier, "Human requested retry at tier");
                state.escalate_to(tier);
                state.failure_count_at_tier = 0; // Reset retry count
            }
            HumanAction::RetryWithGuidance { tier, guidance } => {
                info!(
                    task_id = ?task_id,
                    tier = ?tier,
                    guidance = %guidance,
                    "Human provided guidance for retry"
                );
                state.escalate_to(tier);
                state.failure_count_at_tier = 0;
                // Guidance would be passed to the task context
            }
            HumanAction::Cancel => {
                info!(task_id = ?task_id, "Human cancelled task");
                self.task_states.remove(&task_id);
            }
            HumanAction::MarkComplete => {
                info!(task_id = ?task_id, "Human marked task as complete");
                self.task_states.remove(&task_id);
            }
        }

        Ok(())
    }
}

/// Request for human review of a task
#[derive(Debug, Clone)]
pub struct HumanReviewRequest {
    /// Task ID
    pub task_id: Uuid,
    /// Original task title
    pub title: String,
    /// Original task description
    pub description: String,
    /// Escalation history showing what was tried
    pub escalation_history: Vec<TierAttempt>,
    /// Total attempts across all tiers
    pub total_attempts: u32,
    /// Reason human review is needed
    pub reason: String,
    /// Timestamp when human review was requested
    pub requested_at: chrono::DateTime<chrono::Utc>,
}

/// Summary of a task awaiting human review
#[derive(Debug, Clone)]
pub struct HumanReviewSummary {
    pub task_id: Uuid,
    pub total_failures: u32,
    pub tiers_attempted: Vec<AgentTier>,
    pub last_error: String,
}

/// Response type for tasks needing human action
#[derive(Debug, Clone)]
pub enum HumanAction {
    /// Retry at a specific tier
    RetryAt(AgentTier),
    /// Provide guidance and retry
    RetryWithGuidance { tier: AgentTier, guidance: String },
    /// Cancel the task
    Cancel,
    /// Mark as complete (human did it manually)
    MarkComplete,
}

#[derive(thiserror::Error, Debug)]
pub enum EscalationError {
    #[error("task not found: {0}")]
    TaskNotFound(Uuid),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escalation_policy_retries_first() {
        let policy = EscalationPolicy::default();

        // First failure at utility - should retry
        let decision = policy.evaluate(AgentTier::Utility, 0);
        assert_eq!(decision, EscalationDecision::Retry);
    }

    #[test]
    fn escalation_policy_escalates_after_retries() {
        let policy = EscalationPolicy::default();

        // With utility_retries = 1, we get 1 retry, so need 2 failures to escalate
        let decision = policy.evaluate(AgentTier::Utility, 2);
        assert_eq!(decision, EscalationDecision::Escalate(AgentTier::Worker));
    }

    #[test]
    fn escalation_policy_human_at_end() {
        let policy = EscalationPolicy::default();

        // After orchestrator exhausted (orchestrator_retries = 1, so need 2 failures)
        let decision = policy.evaluate(AgentTier::Orchestrator, 2);
        assert_eq!(decision, EscalationDecision::NeedsHuman);
    }

    #[test]
    fn escalation_path_correct() {
        let policy = EscalationPolicy::default();

        let path = policy.escalation_path(AgentTier::Utility);
        assert_eq!(
            path,
            vec![
                AgentTier::Utility,
                AgentTier::Worker,
                AgentTier::Orchestrator,
            ]
        );
    }

    #[test]
    fn manager_tracks_failures() {
        let mut manager = EscalationManager::with_default_policy();
        let task_id = Uuid::new_v4();

        manager.track_task(task_id, AgentTier::Utility);

        // First failure - should retry
        let decision = manager.on_task_failed(task_id, AgentTier::Utility, "error 1");
        assert_eq!(decision, EscalationDecision::Retry);

        // Second failure - should escalate (default utility_retries = 1)
        let decision = manager.on_task_failed(task_id, AgentTier::Utility, "error 2");
        assert_eq!(decision, EscalationDecision::Escalate(AgentTier::Worker));
    }

    #[test]
    fn manager_escalates_through_chain() {
        let mut manager = EscalationManager::new(EscalationPolicy::new(0, 0, 0));
        let task_id = Uuid::new_v4();

        // Utility -> Worker
        let decision = manager.on_task_failed(task_id, AgentTier::Utility, "error");
        assert_eq!(decision, EscalationDecision::Escalate(AgentTier::Worker));

        // Worker -> Orchestrator
        let decision = manager.on_task_failed(task_id, AgentTier::Worker, "error");
        assert_eq!(
            decision,
            EscalationDecision::Escalate(AgentTier::Orchestrator)
        );

        // Orchestrator -> Human
        let decision = manager.on_task_failed(task_id, AgentTier::Orchestrator, "error");
        assert_eq!(decision, EscalationDecision::NeedsHuman);
    }

    #[test]
    fn human_review_request_includes_history() {
        let mut manager = EscalationManager::new(EscalationPolicy::new(0, 0, 0));
        let task_id = Uuid::new_v4();

        // Run through escalation chain
        manager.on_task_failed(task_id, AgentTier::Utility, "utility error");
        manager.on_task_failed(task_id, AgentTier::Worker, "worker error");
        manager.on_task_failed(task_id, AgentTier::Orchestrator, "orchestrator error");

        let request = manager
            .create_human_review_request(task_id, "Test Task", "Test description")
            .unwrap();

        assert_eq!(request.total_attempts, 3);
        assert_eq!(request.escalation_history.len(), 3);
    }

    #[test]
    fn human_action_retry_at_resets_count() {
        let mut manager = EscalationManager::new(EscalationPolicy::new(0, 0, 0));
        let task_id = Uuid::new_v4();

        // Escalate to needs human
        manager.on_task_failed(task_id, AgentTier::Utility, "error");
        manager.on_task_failed(task_id, AgentTier::Worker, "error");
        manager.on_task_failed(task_id, AgentTier::Orchestrator, "error");

        // Human says retry at worker
        manager
            .apply_human_action(task_id, HumanAction::RetryAt(AgentTier::Worker))
            .unwrap();

        let state = manager.get_state(&task_id).unwrap();
        assert_eq!(state.current_tier, AgentTier::Worker);
        assert_eq!(state.failure_count_at_tier, 0);
    }

    #[test]
    fn human_action_cancel_removes_task() {
        let mut manager = EscalationManager::with_default_policy();
        let task_id = Uuid::new_v4();

        manager.track_task(task_id, AgentTier::Utility);
        assert!(manager.get_state(&task_id).is_some());

        manager
            .apply_human_action(task_id, HumanAction::Cancel)
            .unwrap();

        assert!(manager.get_state(&task_id).is_none());
    }

    #[test]
    fn tasks_needing_human_filters_correctly() {
        let mut manager = EscalationManager::new(EscalationPolicy::new(0, 0, 0));
        let task1 = Uuid::new_v4();
        let task2 = Uuid::new_v4();

        // Task 1 needs human
        manager.on_task_failed(task1, AgentTier::Utility, "error");
        manager.on_task_failed(task1, AgentTier::Worker, "error");
        manager.on_task_failed(task1, AgentTier::Orchestrator, "error");

        // Task 2 only failed at utility, escalated to worker
        manager.on_task_failed(task2, AgentTier::Utility, "error");

        let needing_human = manager.tasks_needing_human();
        assert_eq!(needing_human.len(), 1);
        assert!(needing_human.contains(&task1));
    }

    #[test]
    fn custom_policy_new() {
        let policy = EscalationPolicy::new(5, 3, 2);
        assert_eq!(policy.utility_retries, 5);
        assert_eq!(policy.worker_retries, 3);
        assert_eq!(policy.orchestrator_retries, 2);
    }

    #[test]
    fn max_retries_for_each_tier() {
        let policy = EscalationPolicy::new(4, 7, 2);
        assert_eq!(policy.max_retries_for_tier(AgentTier::Utility), 4);
        assert_eq!(policy.max_retries_for_tier(AgentTier::Worker), 7);
        assert_eq!(policy.max_retries_for_tier(AgentTier::Orchestrator), 2);
    }

    #[test]
    fn next_tier_each_level() {
        let policy = EscalationPolicy::default();
        assert_eq!(
            policy.next_tier(AgentTier::Utility),
            Some(AgentTier::Worker)
        );
        assert_eq!(
            policy.next_tier(AgentTier::Worker),
            Some(AgentTier::Orchestrator)
        );
        assert_eq!(policy.next_tier(AgentTier::Orchestrator), None);
    }

    #[test]
    fn escalation_path_from_worker() {
        let policy = EscalationPolicy::default();
        let path = policy.escalation_path(AgentTier::Worker);
        assert_eq!(path, vec![AgentTier::Worker, AgentTier::Orchestrator]);
    }

    #[test]
    fn escalation_path_from_orchestrator() {
        let policy = EscalationPolicy::default();
        let path = policy.escalation_path(AgentTier::Orchestrator);
        assert_eq!(path, vec![AgentTier::Orchestrator]);
    }

    #[test]
    fn evaluate_worker_retry_then_escalate() {
        let policy = EscalationPolicy::default(); // worker_retries = 2
        assert_eq!(
            policy.evaluate(AgentTier::Worker, 1),
            EscalationDecision::Retry
        );
        assert_eq!(
            policy.evaluate(AgentTier::Worker, 2),
            EscalationDecision::Retry
        );
        assert_eq!(
            policy.evaluate(AgentTier::Worker, 3),
            EscalationDecision::Escalate(AgentTier::Orchestrator)
        );
    }

    #[test]
    fn evaluate_at_exact_retry_boundary() {
        let policy = EscalationPolicy::new(1, 1, 1);
        // failure_count == max_retries => Retry
        assert_eq!(
            policy.evaluate(AgentTier::Utility, 1),
            EscalationDecision::Retry
        );
        // failure_count > max_retries => Escalate
        assert_eq!(
            policy.evaluate(AgentTier::Utility, 2),
            EscalationDecision::Escalate(AgentTier::Worker)
        );
    }

    #[test]
    fn task_escalation_state_new() {
        let id = Uuid::new_v4();
        let state = TaskEscalationState::new(id, AgentTier::Worker);
        assert_eq!(state.task_id, id);
        assert_eq!(state.current_tier, AgentTier::Worker);
        assert_eq!(state.failure_count_at_tier, 0);
        assert_eq!(state.total_failure_count, 0);
        assert!(state.tier_history.is_empty());
    }

    #[test]
    fn record_failure_creates_and_updates_tier_history() {
        let mut state = TaskEscalationState::new(Uuid::new_v4(), AgentTier::Utility);

        // First failure creates a new TierAttempt
        state.record_failure("first error");
        assert_eq!(state.failure_count_at_tier, 1);
        assert_eq!(state.total_failure_count, 1);
        assert_eq!(state.tier_history.len(), 1);
        assert_eq!(state.tier_history[0].attempts, 1);
        assert_eq!(state.tier_history[0].final_error, "first error");

        // Second failure at same tier updates existing TierAttempt
        state.record_failure("second error");
        assert_eq!(state.failure_count_at_tier, 2);
        assert_eq!(state.total_failure_count, 2);
        assert_eq!(state.tier_history.len(), 1); // still one entry
        assert_eq!(state.tier_history[0].attempts, 2);
        assert_eq!(state.tier_history[0].final_error, "second error");
    }

    #[test]
    fn escalate_to_resets_failure_count() {
        let mut state = TaskEscalationState::new(Uuid::new_v4(), AgentTier::Utility);
        state.record_failure("err");
        state.record_failure("err");
        assert_eq!(state.failure_count_at_tier, 2);

        state.escalate_to(AgentTier::Worker);
        assert_eq!(state.current_tier, AgentTier::Worker);
        assert_eq!(state.failure_count_at_tier, 0);
        // total is preserved
        assert_eq!(state.total_failure_count, 2);
    }

    #[test]
    fn on_task_failed_auto_creates_state() {
        let mut manager = EscalationManager::with_default_policy();
        let task_id = Uuid::new_v4();
        // Don't call track_task - on_task_failed should create state
        let decision = manager.on_task_failed(task_id, AgentTier::Worker, "err");
        assert_eq!(decision, EscalationDecision::Retry);
        assert!(manager.get_state(&task_id).is_some());
    }

    #[test]
    fn on_task_failed_tier_mismatch_resets() {
        let mut manager = EscalationManager::with_default_policy();
        let task_id = Uuid::new_v4();
        manager.track_task(task_id, AgentTier::Utility);

        // Call with a different tier than tracked
        let decision = manager.on_task_failed(task_id, AgentTier::Worker, "err");
        assert_eq!(decision, EscalationDecision::Retry);
        let state = manager.get_state(&task_id).unwrap();
        assert_eq!(state.current_tier, AgentTier::Worker);
    }

    #[test]
    fn on_task_completed_removes_state() {
        let mut manager = EscalationManager::with_default_policy();
        let task_id = Uuid::new_v4();
        manager.track_task(task_id, AgentTier::Utility);
        assert!(manager.get_state(&task_id).is_some());

        manager.on_task_completed(task_id);
        assert!(manager.get_state(&task_id).is_none());
    }

    #[test]
    fn on_task_completed_nonexistent_is_noop() {
        let mut manager = EscalationManager::with_default_policy();
        // Should not panic
        manager.on_task_completed(Uuid::new_v4());
    }

    #[test]
    fn clear_task_removes_state() {
        let mut manager = EscalationManager::with_default_policy();
        let task_id = Uuid::new_v4();
        manager.track_task(task_id, AgentTier::Utility);
        manager.clear_task(&task_id);
        assert!(manager.get_state(&task_id).is_none());
    }

    #[test]
    fn clear_task_nonexistent_is_noop() {
        let mut manager = EscalationManager::with_default_policy();
        manager.clear_task(&Uuid::new_v4());
    }

    #[test]
    fn create_human_review_request_nonexistent_returns_none() {
        let manager = EscalationManager::with_default_policy();
        assert!(manager
            .create_human_review_request(Uuid::new_v4(), "t", "d")
            .is_none());
    }

    #[test]
    fn create_human_review_request_no_history_uses_unknown() {
        let mut manager = EscalationManager::with_default_policy();
        let task_id = Uuid::new_v4();
        manager.track_task(task_id, AgentTier::Utility);
        // State exists but no failures recorded, so tier_history is empty
        let req = manager
            .create_human_review_request(task_id, "Title", "Desc")
            .unwrap();
        assert_eq!(req.reason, "Unknown failure");
        assert_eq!(req.title, "Title");
        assert_eq!(req.description, "Desc");
        assert_eq!(req.total_attempts, 0);
    }

    #[test]
    fn create_human_review_request_with_history() {
        let mut manager = EscalationManager::new(EscalationPolicy::new(0, 0, 0));
        let task_id = Uuid::new_v4();
        manager.on_task_failed(task_id, AgentTier::Utility, "util err");
        manager.on_task_failed(task_id, AgentTier::Worker, "work err");

        let req = manager
            .create_human_review_request(task_id, "T", "D")
            .unwrap();
        assert!(req.reason.contains("work err"));
        assert_eq!(req.escalation_history.len(), 2);
        assert_eq!(req.total_attempts, 2);
    }

    #[test]
    fn human_action_mark_complete_removes_task() {
        let mut manager = EscalationManager::with_default_policy();
        let task_id = Uuid::new_v4();
        manager.track_task(task_id, AgentTier::Utility);

        manager
            .apply_human_action(task_id, HumanAction::MarkComplete)
            .unwrap();
        assert!(manager.get_state(&task_id).is_none());
    }

    #[test]
    fn human_action_retry_with_guidance() {
        let mut manager = EscalationManager::with_default_policy();
        let task_id = Uuid::new_v4();
        manager.track_task(task_id, AgentTier::Orchestrator);
        manager.on_task_failed(task_id, AgentTier::Orchestrator, "err");

        manager
            .apply_human_action(
                task_id,
                HumanAction::RetryWithGuidance {
                    tier: AgentTier::Utility,
                    guidance: "try a different approach".to_string(),
                },
            )
            .unwrap();

        let state = manager.get_state(&task_id).unwrap();
        assert_eq!(state.current_tier, AgentTier::Utility);
        assert_eq!(state.failure_count_at_tier, 0);
    }

    #[test]
    fn apply_human_action_task_not_found() {
        let mut manager = EscalationManager::with_default_policy();
        let task_id = Uuid::new_v4();
        let result = manager.apply_human_action(task_id, HumanAction::Cancel);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, EscalationError::TaskNotFound(_)));
        assert!(err.to_string().contains("task not found"));
    }

    #[test]
    fn human_review_summary_empty_when_no_tasks() {
        let manager = EscalationManager::with_default_policy();
        assert!(manager.human_review_summary().is_empty());
    }

    #[test]
    fn human_review_summary_includes_correct_fields() {
        let mut manager = EscalationManager::new(EscalationPolicy::new(0, 0, 0));
        let task_id = Uuid::new_v4();

        manager.on_task_failed(task_id, AgentTier::Utility, "u err");
        manager.on_task_failed(task_id, AgentTier::Worker, "w err");
        manager.on_task_failed(task_id, AgentTier::Orchestrator, "o err");

        let summaries = manager.human_review_summary();
        assert_eq!(summaries.len(), 1);
        let s = &summaries[0];
        assert_eq!(s.task_id, task_id);
        assert_eq!(s.total_failures, 3);
        assert_eq!(s.last_error, "o err");
        assert_eq!(s.tiers_attempted.len(), 3);
    }

    #[test]
    fn human_review_summary_excludes_non_human_tasks() {
        let mut manager = EscalationManager::with_default_policy();
        let task_id = Uuid::new_v4();
        manager.track_task(task_id, AgentTier::Utility);
        // No failures, so not needing human
        assert!(manager.human_review_summary().is_empty());
    }

    #[test]
    fn tasks_needing_human_empty() {
        let manager = EscalationManager::with_default_policy();
        assert!(manager.tasks_needing_human().is_empty());
    }

    #[test]
    fn get_state_nonexistent_returns_none() {
        let manager = EscalationManager::with_default_policy();
        assert!(manager.get_state(&Uuid::new_v4()).is_none());
    }
}
