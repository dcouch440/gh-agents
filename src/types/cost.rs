//! Cost tracking types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use super::agent::{AgentId, AgentTier};
use super::task::TaskId;

/// Record of a single LLM API call cost
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CostRecord {
    pub id: Uuid,
    pub task_id: Option<TaskId>,
    pub agent_id: AgentId,
    pub agent_tier: AgentTier,
    pub model_id: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cost_usd: f64,
    pub timestamp: DateTime<Utc>,
}

/// Aggregated cost summary
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct CostSummary {
    pub session_total: f64,
    pub by_tier: HashMap<String, f64>,
    pub by_task: HashMap<String, f64>,
    pub by_model: HashMap<String, f64>,
}

impl CostSummary {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a cost record to the summary
    pub fn add_record(&mut self, record: &CostRecord) {
        self.session_total += record.cost_usd;

        let tier_key = format!("{:?}", record.agent_tier);
        *self.by_tier.entry(tier_key).or_insert(0.0) += record.cost_usd;

        if let Some(task_id) = &record.task_id {
            let task_key = task_id.0.to_string();
            *self.by_task.entry(task_key).or_insert(0.0) += record.cost_usd;
        }

        *self.by_model.entry(record.model_id.clone()).or_insert(0.0) += record.cost_usd;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn cost_summary_starts_empty() {
        let summary = CostSummary::new();
        assert_eq!(summary.session_total, 0.0);
        assert!(summary.by_tier.is_empty());
    }

    #[test]
    fn cost_summary_accumulates() {
        let mut summary = CostSummary::new();
        let record = CostRecord {
            id: Uuid::new_v4(),
            task_id: None,
            agent_id: AgentId::new(),
            agent_tier: AgentTier::Worker,
            model_id: "test-model".to_string(),
            input_tokens: 100,
            output_tokens: 50,
            cost_usd: 0.01,
            timestamp: Utc::now(),
        };
        summary.add_record(&record);
        assert_eq!(summary.session_total, 0.01);
    }
}
