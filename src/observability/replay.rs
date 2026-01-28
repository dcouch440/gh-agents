//! Decision replay functionality
//!
//! Provides tools to replay and understand agent decisions.

use crate::observability::logging::{Decision, LlmCall, LlmCallLogger};
use anyhow::Result;
use uuid::Uuid;

/// Decision replay tool
pub struct DecisionReplay {
    logger: LlmCallLogger,
}

impl DecisionReplay {
    /// Create a new replay tool
    pub fn new(logger: LlmCallLogger) -> Self {
        Self { logger }
    }

    /// Get the full timeline for a task
    pub async fn get_task_timeline(&self, task_id: Uuid) -> Result<TaskTimeline> {
        let calls = self.logger.get_calls_for_task(task_id).await?;
        let decisions = self.logger.get_decisions_for_task(task_id).await?;

        let total_cost: f64 = calls.iter().map(|c| c.cost_usd).sum();
        let total_tokens: u32 = calls.iter().map(|c| c.total_tokens()).sum();
        let total_latency_ms: u64 = calls.iter().map(|c| c.latency_ms).sum();

        Ok(TaskTimeline {
            task_id,
            calls,
            decisions,
            total_cost,
            total_tokens,
            total_latency_ms,
        })
    }
}

/// A timeline of all activity for a task
#[derive(Debug)]
pub struct TaskTimeline {
    /// The task ID
    pub task_id: Uuid,
    /// All LLM calls in chronological order
    pub calls: Vec<LlmCall>,
    /// All decisions in chronological order
    pub decisions: Vec<Decision>,
    /// Total cost of all LLM calls
    pub total_cost: f64,
    /// Total tokens used
    pub total_tokens: u32,
    /// Total latency across all calls
    pub total_latency_ms: u64,
}

impl TaskTimeline {
    /// Check if the timeline is empty
    pub fn is_empty(&self) -> bool {
        self.calls.is_empty() && self.decisions.is_empty()
    }

    /// Get the number of LLM calls
    pub fn call_count(&self) -> usize {
        self.calls.len()
    }

    /// Get the number of decisions
    pub fn decision_count(&self) -> usize {
        self.decisions.len()
    }

    /// Get average latency per call
    pub fn avg_latency_ms(&self) -> u64 {
        if self.calls.is_empty() {
            0
        } else {
            self.total_latency_ms / self.calls.len() as u64
        }
    }

    /// Get average cost per call
    pub fn avg_cost(&self) -> f64 {
        if self.calls.is_empty() {
            0.0
        } else {
            self.total_cost / self.calls.len() as f64
        }
    }

    /// Get a specific LLM call by index
    pub fn get_call(&self, index: usize) -> Option<&LlmCall> {
        self.calls.get(index)
    }

    /// Get a specific decision by index
    pub fn get_decision(&self, index: usize) -> Option<&Decision> {
        self.decisions.get(index)
    }

    /// Find the decision associated with an LLM call
    pub fn decision_for_call(&self, call_id: Uuid) -> Option<&Decision> {
        self.decisions
            .iter()
            .find(|d| d.llm_call_id == Some(call_id))
    }

    /// Find the LLM call associated with a decision
    pub fn call_for_decision(&self, decision: &Decision) -> Option<&LlmCall> {
        decision
            .llm_call_id
            .and_then(|id| self.calls.iter().find(|c| c.id == id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observability::logging::{DecisionType, LlmPrompt};

    fn mock_call(cost: f64, tokens: u32, latency: u64) -> LlmCall {
        LlmCall::new("model", LlmPrompt::new("system"), "response")
            .with_cost(cost)
            .with_tokens(tokens / 2, tokens / 2)
            .with_latency(latency)
    }

    fn mock_decision(task_id: Uuid) -> Decision {
        Decision::new(task_id, DecisionType::Decomposition, "reasoning", "outcome")
    }

    #[test]
    fn task_timeline_empty() {
        let timeline = TaskTimeline {
            task_id: Uuid::new_v4(),
            calls: vec![],
            decisions: vec![],
            total_cost: 0.0,
            total_tokens: 0,
            total_latency_ms: 0,
        };

        assert!(timeline.is_empty());
        assert_eq!(timeline.call_count(), 0);
        assert_eq!(timeline.decision_count(), 0);
        assert_eq!(timeline.avg_latency_ms(), 0);
        assert!((timeline.avg_cost() - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn task_timeline_with_calls() {
        let timeline = TaskTimeline {
            task_id: Uuid::new_v4(),
            calls: vec![mock_call(0.01, 100, 500), mock_call(0.02, 200, 1000)],
            decisions: vec![],
            total_cost: 0.03,
            total_tokens: 300,
            total_latency_ms: 1500,
        };

        assert!(!timeline.is_empty());
        assert_eq!(timeline.call_count(), 2);
        assert_eq!(timeline.avg_latency_ms(), 750);
        assert!((timeline.avg_cost() - 0.015).abs() < f64::EPSILON);
    }

    #[test]
    fn task_timeline_get_call() {
        let call = mock_call(0.01, 100, 500);
        let timeline = TaskTimeline {
            task_id: Uuid::new_v4(),
            calls: vec![call.clone()],
            decisions: vec![],
            total_cost: 0.01,
            total_tokens: 100,
            total_latency_ms: 500,
        };

        assert!(timeline.get_call(0).is_some());
        assert!(timeline.get_call(1).is_none());
    }

    #[test]
    fn task_timeline_get_decision() {
        let task_id = Uuid::new_v4();
        let decision = mock_decision(task_id);
        let timeline = TaskTimeline {
            task_id,
            calls: vec![],
            decisions: vec![decision],
            total_cost: 0.0,
            total_tokens: 0,
            total_latency_ms: 0,
        };

        assert!(timeline.get_decision(0).is_some());
        assert!(timeline.get_decision(1).is_none());
    }

    #[test]
    fn task_timeline_decision_call_linking() {
        let task_id = Uuid::new_v4();
        let call = mock_call(0.01, 100, 500);
        let call_id = call.id;
        let decision = mock_decision(task_id).with_llm_call(call_id);

        let timeline = TaskTimeline {
            task_id,
            calls: vec![call],
            decisions: vec![decision.clone()],
            total_cost: 0.01,
            total_tokens: 100,
            total_latency_ms: 500,
        };

        // Find decision for call
        let found_decision = timeline.decision_for_call(call_id);
        assert!(found_decision.is_some());
        assert_eq!(found_decision.unwrap().id, decision.id);

        // Find call for decision
        let found_call = timeline.call_for_decision(&decision);
        assert!(found_call.is_some());
        assert_eq!(found_call.unwrap().id, call_id);
    }

    #[test]
    fn task_timeline_no_linked_decision() {
        let timeline = TaskTimeline {
            task_id: Uuid::new_v4(),
            calls: vec![mock_call(0.01, 100, 500)],
            decisions: vec![],
            total_cost: 0.01,
            total_tokens: 100,
            total_latency_ms: 500,
        };

        let random_id = Uuid::new_v4();
        assert!(timeline.decision_for_call(random_id).is_none());
    }
}
