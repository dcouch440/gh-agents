//! LLM cost calculation and tracking

use std::collections::HashMap;
use std::sync::Arc;

use chrono::Utc;
use once_cell::sync::Lazy;
use tokio::sync::RwLock;
use uuid::Uuid;

use super::types::TokenUsage;
use crate::db::pg_repo::PgRepo;
use crate::db::traits::CostRepo;
use crate::types::{AgentId, AgentTier, CostRecord, CostSummary, TaskId};

/// Pricing information for a model (per 1000 tokens)
#[derive(Debug, Clone, Copy)]
pub struct ModelPricing {
    /// Cost per 1000 input tokens in USD
    pub input_cost_per_1k: f64,
    /// Cost per 1000 output tokens in USD
    pub output_cost_per_1k: f64,
}

impl ModelPricing {
    pub const fn new(input: f64, output: f64) -> Self {
        Self {
            input_cost_per_1k: input,
            output_cost_per_1k: output,
        }
    }

    /// Calculate cost for given token counts
    pub fn calculate_cost(&self, input_tokens: u32, output_tokens: u32) -> f64 {
        let input_cost = (input_tokens as f64 / 1000.0) * self.input_cost_per_1k;
        let output_cost = (output_tokens as f64 / 1000.0) * self.output_cost_per_1k;
        input_cost + output_cost
    }
}

/// Default pricing for unknown models (conservative estimate)
pub const DEFAULT_PRICING: ModelPricing = ModelPricing::new(0.015, 0.075);

/// Known model pricing table
///
/// Pricing as of early 2025 - update as needed
/// https://www.anthropic.com/pricing
static MODEL_PRICING: Lazy<HashMap<&'static str, ModelPricing>> = Lazy::new(|| {
    let mut map = HashMap::new();

    // Claude 3.5 Sonnet
    map.insert("claude-sonnet-4-20250514", ModelPricing::new(0.003, 0.015));
    map.insert("claude-3-5-sonnet-20241022", ModelPricing::new(0.003, 0.015));
    map.insert("claude-3-5-sonnet-20240620", ModelPricing::new(0.003, 0.015));

    // Claude 3.5 Haiku
    map.insert("claude-3-5-haiku-20241022", ModelPricing::new(0.001, 0.005));

    // Claude 3 Opus
    map.insert("claude-3-opus-20240229", ModelPricing::new(0.015, 0.075));

    // Claude 3 Sonnet
    map.insert("claude-3-sonnet-20240229", ModelPricing::new(0.003, 0.015));

    // Claude 3 Haiku
    map.insert("claude-3-haiku-20240307", ModelPricing::new(0.00025, 0.00125));

    // xAI Grok models
    map.insert("grok-4-1-fast", ModelPricing::new(0.0002, 0.0005));
    map.insert("grok-4", ModelPricing::new(0.003, 0.015));
    map.insert("grok-3", ModelPricing::new(0.003, 0.015));

    // Shorthand aliases
    map.insert("claude-3-opus", ModelPricing::new(0.015, 0.075));
    map.insert("claude-3-sonnet", ModelPricing::new(0.003, 0.015));
    map.insert("claude-3-haiku", ModelPricing::new(0.00025, 0.00125));
    map.insert("claude-haiku", ModelPricing::new(0.00025, 0.00125));

    map
});

/// Get pricing for a model, returning default if unknown
pub fn get_pricing(model_id: &str) -> ModelPricing {
    MODEL_PRICING.get(model_id).copied().unwrap_or_else(|| {
        tracing::warn!("Unknown model '{}', using default pricing", model_id);
        DEFAULT_PRICING
    })
}

/// Check if a model has known pricing
pub fn has_known_pricing(model_id: &str) -> bool {
    MODEL_PRICING.contains_key(model_id)
}

/// List all models with known pricing
pub fn known_models() -> Vec<&'static str> {
    MODEL_PRICING.keys().copied().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claude_3_haiku_pricing() {
        let pricing = get_pricing("claude-3-haiku-20240307");
        assert!((pricing.input_cost_per_1k - 0.00025).abs() < 0.0001);
        assert!((pricing.output_cost_per_1k - 0.00125).abs() < 0.0001);
    }

    #[test]
    fn test_cost_calculation() {
        let pricing = get_pricing("claude-3-haiku-20240307");
        // 1000 input tokens @ $0.00025/1k = $0.00025
        // 1000 output tokens @ $0.00125/1k = $0.00125
        // Total = $0.0015
        let cost = pricing.calculate_cost(1000, 1000);
        assert!((cost - 0.0015).abs() < 0.0001);
    }

    #[test]
    fn test_unknown_model_uses_default() {
        let pricing = get_pricing("unknown-model-xyz");
        assert_eq!(pricing.input_cost_per_1k, DEFAULT_PRICING.input_cost_per_1k);
    }

    #[test]
    fn test_sonnet_alias() {
        let pricing = get_pricing("claude-sonnet-4-20250514");
        assert!(pricing.input_cost_per_1k > 0.0);
    }

    #[test]
    fn calculate_cost_zero_tokens() {
        let pricing = ModelPricing::new(0.003, 0.015);
        assert_eq!(pricing.calculate_cost(0, 0), 0.0);
    }

    #[test]
    fn calculate_cost_large_tokens() {
        let pricing = ModelPricing::new(0.003, 0.015);
        let cost = pricing.calculate_cost(1_000_000, 500_000);
        // 1M input: 1000 * 0.003 = 3.0
        // 500K output: 500 * 0.015 = 7.5
        assert!((cost - 10.5).abs() < 0.001);
    }

    #[test]
    fn get_pricing_known_models() {
        let models = [
            "claude-sonnet-4-20250514",
            "claude-3-5-sonnet-20241022",
            "claude-3-5-haiku-20241022",
            "claude-3-opus-20240229",
            "claude-3-haiku-20240307",
        ];
        for model in &models {
            let pricing = get_pricing(model);
            assert!(pricing.input_cost_per_1k > 0.0, "Failed for {}", model);
        }
    }

    #[test]
    fn get_pricing_unknown_returns_default() {
        let pricing = get_pricing("totally-unknown-model");
        assert_eq!(pricing.input_cost_per_1k, DEFAULT_PRICING.input_cost_per_1k);
        assert_eq!(pricing.output_cost_per_1k, DEFAULT_PRICING.output_cost_per_1k);
    }

    #[test]
    fn has_known_pricing_true_false() {
        assert!(has_known_pricing("claude-3-opus-20240229"));
        assert!(!has_known_pricing("nonexistent-model"));
    }

    #[test]
    fn known_models_nonempty() {
        let models = known_models();
        assert!(!models.is_empty());
        assert!(models.len() >= 5);
    }

    #[test]
    fn cost_tracker_in_memory() {
        let tracker = CostTracker::<PgRepo>::in_memory();
        assert!(tracker.repo.is_none());
    }
}

/// Tracks LLM API costs
pub struct CostTracker<R: CostRepo = PgRepo> {
    /// Repository for persistence (optional for testing without DB)
    repo: Option<R>,

    /// In-memory records for current session
    records: Arc<RwLock<Vec<CostRecord>>>,
}

impl CostTracker<PgRepo> {
    /// Create a new cost tracker with database persistence
    pub fn new(db_pool: sqlx::PgPool) -> Self {
        Self {
            repo: Some(PgRepo::new(db_pool)),
            records: Arc::new(RwLock::new(Vec::new())),
        }
    }
}

impl<R: CostRepo> CostTracker<R> {
    /// Create a cost tracker with an explicit repo
    pub fn with_repo(repo: R) -> Self {
        Self {
            repo: Some(repo),
            records: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Create a cost tracker without database (for testing)
    pub fn in_memory() -> Self {
        Self {
            repo: None,
            records: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Record an API call
    pub async fn record_call(&self, agent_id: AgentId, agent_tier: AgentTier, task_id: Option<TaskId>, model_id: &str, usage: TokenUsage) -> Result<CostRecord, CostTrackerError> {
        let pricing = get_pricing(model_id);
        let cost_usd = pricing.calculate_cost(usage.input_tokens, usage.output_tokens);

        let record = CostRecord {
            id: Uuid::new_v4(),
            task_id,
            agent_id,
            agent_tier,
            model_id: model_id.to_string(),
            input_tokens: usage.input_tokens,
            output_tokens: usage.output_tokens,
            cost_usd,
            timestamp: Utc::now(),
        };

        // Add to in-memory list
        {
            let mut records = self.records.write().await;
            records.push(record.clone());
        }

        // Persist to database if available
        if let Some(repo) = &self.repo {
            repo.persist_cost_record(record.clone()).await.map_err(CostTrackerError::DatabaseError)?;
        }

        tracing::debug!("Recorded API call: model={}, tokens={}+{}, cost=${:.6}", model_id, usage.input_tokens, usage.output_tokens, cost_usd);

        Ok(record)
    }

    /// Get all records for current session
    pub async fn session_records(&self) -> Vec<CostRecord> {
        self.records.read().await.clone()
    }

    /// Get total cost for current session
    pub async fn session_total(&self) -> f64 {
        self.records.read().await.iter().map(|r| r.cost_usd).sum()
    }

    /// Get a summary of costs for the current session
    pub async fn get_summary(&self) -> CostSummary {
        let records = self.records.read().await;
        Self::summarize_records(&records)
    }

    /// Get summary from database (all time or filtered)
    pub async fn get_historical_summary(&self, since: Option<chrono::DateTime<Utc>>) -> Result<CostSummary, CostTrackerError> {
        let Some(repo) = &self.repo else {
            // No database, return session summary
            return Ok(self.get_summary().await);
        };

        let cost_records = repo.get_cost_records(since).await.map_err(CostTrackerError::DatabaseError)?;

        Ok(Self::summarize_records(&cost_records))
    }

    /// Summarize a list of records
    fn summarize_records(records: &[CostRecord]) -> CostSummary {
        let mut summary = CostSummary::new();

        for record in records {
            summary.add_record(record);
        }

        summary
    }

    /// Get costs grouped by tier
    pub async fn costs_by_tier(&self) -> HashMap<AgentTier, f64> {
        let records = self.records.read().await;
        let mut by_tier: HashMap<AgentTier, f64> = HashMap::new();

        for record in records.iter() {
            *by_tier.entry(record.agent_tier).or_insert(0.0) += record.cost_usd;
        }

        by_tier
    }

    /// Get costs grouped by model
    pub async fn costs_by_model(&self) -> HashMap<String, f64> {
        let records = self.records.read().await;
        let mut by_model: HashMap<String, f64> = HashMap::new();

        for record in records.iter() {
            *by_model.entry(record.model_id.clone()).or_insert(0.0) += record.cost_usd;
        }

        by_model
    }

    /// Get costs for a specific task
    pub async fn cost_for_task(&self, task_id: &TaskId) -> f64 {
        self.records.read().await.iter().filter(|r| r.task_id.as_ref() == Some(task_id)).map(|r| r.cost_usd).sum()
    }

    /// Format cost as a human-readable string
    pub fn format_cost(cost: f64) -> String {
        if cost < 0.01 {
            format!("${:.4}", cost)
        } else if cost < 1.0 {
            format!("${:.3}", cost)
        } else {
            format!("${:.2}", cost)
        }
    }
}

/// Errors from cost tracking operations
#[derive(Debug, thiserror::Error)]
pub enum CostTrackerError {
    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Invalid data: {0}")]
    InvalidData(String),
}

#[cfg(test)]
mod tracker_tests {
    use super::*;

    #[tokio::test]
    async fn test_record_call_in_memory() {
        let tracker = CostTracker::<PgRepo>::in_memory();

        let result = tracker
            .record_call(
                AgentId::new(),
                AgentTier::Worker,
                None,
                "claude-3-haiku-20240307",
                TokenUsage {
                    input_tokens: 1000,
                    output_tokens: 500,
                },
            )
            .await;

        assert!(result.is_ok());
        let record = result.unwrap();
        assert!(record.cost_usd > 0.0);
    }

    #[tokio::test]
    async fn test_session_total() {
        let tracker = CostTracker::<PgRepo>::in_memory();

        // Record two calls
        tracker
            .record_call(
                AgentId::new(),
                AgentTier::Worker,
                None,
                "claude-3-haiku-20240307",
                TokenUsage {
                    input_tokens: 1000,
                    output_tokens: 500,
                },
            )
            .await
            .unwrap();

        tracker
            .record_call(
                AgentId::new(),
                AgentTier::Orchestrator,
                None,
                "claude-3-haiku-20240307",
                TokenUsage {
                    input_tokens: 2000,
                    output_tokens: 1000,
                },
            )
            .await
            .unwrap();

        let total = tracker.session_total().await;
        assert!(total > 0.0);
    }
}

#[cfg(test)]
mod summary_tests {
    use super::*;

    #[tokio::test]
    async fn test_costs_by_tier() {
        let tracker = CostTracker::<PgRepo>::in_memory();

        // Worker call
        tracker
            .record_call(
                AgentId::new(),
                AgentTier::Worker,
                None,
                "claude-3-haiku-20240307",
                TokenUsage {
                    input_tokens: 1000,
                    output_tokens: 500,
                },
            )
            .await
            .unwrap();

        // Orchestrator call
        tracker
            .record_call(
                AgentId::new(),
                AgentTier::Orchestrator,
                None,
                "claude-3-opus-20240229",
                TokenUsage {
                    input_tokens: 1000,
                    output_tokens: 500,
                },
            )
            .await
            .unwrap();

        let by_tier = tracker.costs_by_tier().await;
        assert!(by_tier.contains_key(&AgentTier::Worker));
        assert!(by_tier.contains_key(&AgentTier::Orchestrator));
        // Opus should be more expensive
        assert!(by_tier[&AgentTier::Orchestrator] > by_tier[&AgentTier::Worker]);
    }

    #[tokio::test]
    async fn test_costs_by_model() {
        let tracker = CostTracker::<PgRepo>::in_memory();

        tracker
            .record_call(
                AgentId::new(),
                AgentTier::Worker,
                None,
                "claude-3-haiku-20240307",
                TokenUsage {
                    input_tokens: 1000,
                    output_tokens: 500,
                },
            )
            .await
            .unwrap();

        tracker
            .record_call(
                AgentId::new(),
                AgentTier::Worker,
                None,
                "claude-3-opus-20240229",
                TokenUsage {
                    input_tokens: 1000,
                    output_tokens: 500,
                },
            )
            .await
            .unwrap();

        let by_model = tracker.costs_by_model().await;
        assert!(by_model.contains_key("claude-3-haiku-20240307"));
        assert!(by_model.contains_key("claude-3-opus-20240229"));
    }

    #[tokio::test]
    async fn test_get_summary() {
        let tracker = CostTracker::<PgRepo>::in_memory();

        tracker
            .record_call(
                AgentId::new(),
                AgentTier::Worker,
                None,
                "claude-3-haiku-20240307",
                TokenUsage {
                    input_tokens: 1000,
                    output_tokens: 500,
                },
            )
            .await
            .unwrap();

        let summary = tracker.get_summary().await;
        assert!(summary.session_total > 0.0);
        assert!(summary.by_tier.contains_key("Worker"));
        assert!(summary.by_model.contains_key("claude-3-haiku-20240307"));
    }

    #[test]
    fn test_format_cost() {
        assert_eq!(CostTracker::<PgRepo>::format_cost(0.0001), "$0.0001");
        assert_eq!(CostTracker::<PgRepo>::format_cost(0.123), "$0.123");
        assert_eq!(CostTracker::<PgRepo>::format_cost(1.50), "$1.50");
    }
}

#[cfg(test)]
mod db_tests {
    use super::*;
    use crate::db::traits::MockCostRepo;

    #[tokio::test]
    async fn cost_tracker_new_with_repo() {
        let mock = MockCostRepo::new();
        let tracker = CostTracker::with_repo(mock);
        assert!(tracker.repo.is_some());
    }

    #[tokio::test]
    async fn record_call_persists_to_repo() {
        let mut mock = MockCostRepo::new();
        mock.expect_persist_cost_record().times(1).returning(|_| Ok(()));

        let task_id = TaskId::new();
        let agent_id = AgentId::new();
        let tracker = CostTracker::with_repo(mock);

        let record = tracker
            .record_call(
                agent_id,
                AgentTier::Worker,
                Some(task_id.clone()),
                "claude-3-haiku-20240307",
                TokenUsage {
                    input_tokens: 1000,
                    output_tokens: 500,
                },
            )
            .await
            .unwrap();

        assert!(record.cost_usd > 0.0);
        assert_eq!(record.task_id.as_ref(), Some(&task_id));

        // Verify via session_records
        let records = tracker.session_records().await;
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].id, record.id);

        // Verify via cost_for_task
        let task_cost = tracker.cost_for_task(&task_id).await;
        assert!(task_cost > 0.0);
        assert!((task_cost - record.cost_usd).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn get_historical_summary_without_since() {
        let mut mock = MockCostRepo::new();
        // First call: persist_cost_record
        mock.expect_persist_cost_record().times(1).returning(|_| Ok(()));
        // Second call: get_cost_records for historical summary
        mock.expect_get_cost_records().times(1).returning(|_| {
            Ok(vec![CostRecord {
                id: Uuid::new_v4(),
                task_id: None,
                agent_id: AgentId::new(),
                agent_tier: AgentTier::Worker,
                model_id: "claude-3-haiku-20240307".to_string(),
                input_tokens: 1000,
                output_tokens: 500,
                cost_usd: 0.0009,
                timestamp: Utc::now(),
            }])
        });

        let agent_id = AgentId::new();
        let tracker = CostTracker::with_repo(mock);

        tracker
            .record_call(
                agent_id,
                AgentTier::Worker,
                None,
                "claude-3-haiku-20240307",
                TokenUsage {
                    input_tokens: 1000,
                    output_tokens: 500,
                },
            )
            .await
            .unwrap();

        let summary = tracker.get_historical_summary(None).await.unwrap();
        assert!(summary.session_total > 0.0);
        assert!(summary.by_model.contains_key("claude-3-haiku-20240307"));
    }

    #[tokio::test]
    async fn get_historical_summary_with_since() {
        let mut mock = MockCostRepo::new();
        mock.expect_persist_cost_record().times(1).returning(|_| Ok(()));
        // First get_cost_records call: returns records (since = before)
        mock.expect_get_cost_records().times(1).returning(|_| {
            Ok(vec![CostRecord {
                id: Uuid::new_v4(),
                task_id: None,
                agent_id: AgentId::new(),
                agent_tier: AgentTier::Orchestrator,
                model_id: "claude-3-opus-20240229".to_string(),
                input_tokens: 500,
                output_tokens: 200,
                cost_usd: 0.01,
                timestamp: Utc::now(),
            }])
        });
        // Second get_cost_records call: returns empty (since = future)
        mock.expect_get_cost_records().times(1).returning(|_| Ok(vec![]));

        let agent_id = AgentId::new();
        let tracker = CostTracker::with_repo(mock);

        let before = Utc::now();

        tracker
            .record_call(
                agent_id,
                AgentTier::Orchestrator,
                None,
                "claude-3-opus-20240229",
                TokenUsage {
                    input_tokens: 500,
                    output_tokens: 200,
                },
            )
            .await
            .unwrap();

        // Query with since = before the record was created
        let summary = tracker.get_historical_summary(Some(before)).await.unwrap();
        assert!(summary.session_total > 0.0);

        // Query with since = future should return empty
        let future = Utc::now() + chrono::Duration::hours(1);
        let summary_empty = tracker.get_historical_summary(Some(future)).await.unwrap();
        assert_eq!(summary_empty.session_total, 0.0);
    }

    #[test]
    fn cost_tracker_in_memory_no_repo() {
        let tracker = CostTracker::<MockCostRepo>::in_memory();
        assert!(tracker.repo.is_none());
    }
}
