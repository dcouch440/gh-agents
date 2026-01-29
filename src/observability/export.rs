//! Session export functionality
//!
//! Export observability data for external analysis.

use crate::observability::logging::{Decision, LlmCall, LlmCallLogger};
use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;
use uuid::Uuid;

/// Exported session data
#[derive(Debug, Serialize)]
pub struct SessionExport {
    /// Export format version
    pub version: String,
    /// When the export was created
    pub exported_at: DateTime<Utc>,
    /// Time range covered
    pub time_range: TimeRange,
    /// Summary statistics
    pub summary: ExportSummary,
    /// All LLM calls
    pub llm_calls: Vec<LlmCall>,
    /// All decisions
    pub decisions: Vec<Decision>,
}

/// Time range for export
#[derive(Debug, Clone, Serialize)]
pub struct TimeRange {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

impl TimeRange {
    /// Create a new time range
    pub fn new(start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        Self { start, end }
    }

    /// Last N hours
    pub fn last_hours(hours: i64) -> Self {
        let end = Utc::now();
        let start = end - Duration::hours(hours);
        Self { start, end }
    }

    /// Last 24 hours
    pub fn last_24_hours() -> Self {
        Self::last_hours(24)
    }

    /// Duration in seconds
    pub fn duration_secs(&self) -> i64 {
        (self.end - self.start).num_seconds()
    }
}

/// Summary statistics for export
#[derive(Debug, Serialize)]
pub struct ExportSummary {
    pub total_llm_calls: usize,
    pub total_decisions: usize,
    pub total_tokens: u32,
    pub total_cost_usd: f64,
    pub total_latency_ms: u64,
    pub cost_by_model: HashMap<String, f64>,
    pub cost_by_decision_type: HashMap<String, f64>,
    pub calls_by_model: HashMap<String, usize>,
}

impl ExportSummary {
    /// Create summary from calls and decisions
    pub fn from_data(calls: &[LlmCall], decisions: &[Decision]) -> Self {
        let total_tokens: u32 = calls.iter().map(|c| c.total_tokens()).sum();
        let total_cost: f64 = calls.iter().map(|c| c.cost_usd).sum();
        let total_latency: u64 = calls.iter().map(|c| c.latency_ms).sum();

        let mut cost_by_model: HashMap<String, f64> = HashMap::new();
        let mut calls_by_model: HashMap<String, usize> = HashMap::new();

        for call in calls {
            *cost_by_model.entry(call.model.clone()).or_default() += call.cost_usd;
            *calls_by_model.entry(call.model.clone()).or_default() += 1;
        }

        let mut cost_by_decision_type: HashMap<String, f64> = HashMap::new();
        for decision in decisions {
            *cost_by_decision_type
                .entry(decision.decision_type.to_string())
                .or_default() += decision.cost_usd;
        }

        Self {
            total_llm_calls: calls.len(),
            total_decisions: decisions.len(),
            total_tokens,
            total_cost_usd: total_cost,
            total_latency_ms: total_latency,
            cost_by_model,
            cost_by_decision_type,
            calls_by_model,
        }
    }
}

/// Session exporter
pub struct SessionExporter {
    logger: LlmCallLogger,
}

impl SessionExporter {
    /// Create a new exporter
    pub fn new(logger: LlmCallLogger) -> Self {
        Self { logger }
    }

    /// Export data for a time range
    pub async fn export(&self, time_range: TimeRange) -> Result<SessionExport> {
        let calls = self
            .logger
            .get_calls_in_range(time_range.start, time_range.end)
            .await?;
        let decisions = self
            .logger
            .get_decisions_in_range(time_range.start, time_range.end)
            .await?;

        let summary = ExportSummary::from_data(&calls, &decisions);

        Ok(SessionExport {
            version: env!("CARGO_PKG_VERSION").to_string(),
            exported_at: Utc::now(),
            time_range,
            summary,
            llm_calls: calls,
            decisions,
        })
    }

    /// Export data for a specific task
    pub async fn export_task(&self, task_id: Uuid) -> Result<SessionExport> {
        let calls = self.logger.get_calls_for_task(task_id).await?;
        let decisions = self.logger.get_decisions_for_task(task_id).await?;

        // Determine time range from data
        let time_range = if calls.is_empty() && decisions.is_empty() {
            TimeRange::last_24_hours()
        } else {
            let mut timestamps: Vec<DateTime<Utc>> = Vec::new();
            timestamps.extend(calls.iter().map(|c| c.timestamp));
            timestamps.extend(decisions.iter().map(|d| d.timestamp));
            timestamps.sort();

            TimeRange::new(
                *timestamps.first().unwrap_or(&Utc::now()),
                *timestamps.last().unwrap_or(&Utc::now()),
            )
        };

        let summary = ExportSummary::from_data(&calls, &decisions);

        Ok(SessionExport {
            version: env!("CARGO_PKG_VERSION").to_string(),
            exported_at: Utc::now(),
            time_range,
            summary,
            llm_calls: calls,
            decisions,
        })
    }

    /// Export to a JSON file
    pub async fn export_to_file(&self, path: &Path, time_range: TimeRange) -> Result<()> {
        let export = self.export(time_range).await?;
        let json = serde_json::to_string_pretty(&export)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Export a task to a JSON file
    pub async fn export_task_to_file(&self, path: &Path, task_id: Uuid) -> Result<()> {
        let export = self.export_task(task_id).await?;
        let json = serde_json::to_string_pretty(&export)?;
        std::fs::write(path, json)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observability::{DecisionType, LlmPrompt};
    use sqlx::SqlitePool;
    use tempfile::TempDir;

    fn mock_call(model: &str, cost: f64, tokens: u32) -> LlmCall {
        LlmCall::new(model, LlmPrompt::new("system"), "response")
            .with_cost(cost)
            .with_tokens(tokens / 2, tokens / 2)
            .with_latency(500)
    }

    fn mock_decision(decision_type: DecisionType, cost: f64) -> Decision {
        Decision::new(Uuid::new_v4(), decision_type, "reasoning", "outcome").with_cost(cost)
    }

    async fn setup_test_db() -> (SqlitePool, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let pool = crate::db::init_db_at(db_path.to_str().unwrap())
            .await
            .unwrap();
        (pool, temp_dir)
    }

    #[test]
    fn time_range_last_hours() {
        let range = TimeRange::last_hours(2);
        let duration = range.duration_secs();
        assert!(duration >= 7200 - 10 && duration <= 7200 + 10); // Allow small variance
    }

    #[test]
    fn time_range_last_24_hours() {
        let range = TimeRange::last_24_hours();
        let duration = range.duration_secs();
        assert!(duration >= 86400 - 10 && duration <= 86400 + 10);
    }

    #[test]
    fn time_range_new() {
        let start = Utc::now() - Duration::hours(5);
        let end = Utc::now();
        let range = TimeRange::new(start, end);
        assert_eq!(range.start, start);
        assert_eq!(range.end, end);
        let dur = range.duration_secs();
        assert!(dur >= 18000 - 10 && dur <= 18000 + 10);
    }

    #[test]
    fn time_range_duration_secs_exact() {
        let start = DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let end = DateTime::parse_from_rfc3339("2024-01-01T01:30:00Z")
            .unwrap()
            .with_timezone(&Utc);
        let range = TimeRange::new(start, end);
        assert_eq!(range.duration_secs(), 5400);
    }

    #[test]
    fn time_range_last_hours_one() {
        let range = TimeRange::last_hours(1);
        let dur = range.duration_secs();
        assert!(dur >= 3600 - 10 && dur <= 3600 + 10);
    }

    #[test]
    fn export_summary_empty() {
        let summary = ExportSummary::from_data(&[], &[]);

        assert_eq!(summary.total_llm_calls, 0);
        assert_eq!(summary.total_decisions, 0);
        assert_eq!(summary.total_tokens, 0);
        assert!((summary.total_cost_usd - 0.0).abs() < f64::EPSILON);
        assert_eq!(summary.total_latency_ms, 0);
        assert!(summary.cost_by_model.is_empty());
        assert!(summary.cost_by_decision_type.is_empty());
        assert!(summary.calls_by_model.is_empty());
    }

    #[test]
    fn export_summary_with_calls() {
        let calls = vec![
            mock_call("claude-sonnet-4-20250514", 0.01, 100),
            mock_call("claude-sonnet-4-20250514", 0.02, 200),
            mock_call("claude-haiku", 0.005, 50),
        ];

        let summary = ExportSummary::from_data(&calls, &[]);

        assert_eq!(summary.total_llm_calls, 3);
        assert_eq!(summary.total_tokens, 350);
        assert!((summary.total_cost_usd - 0.035).abs() < f64::EPSILON);
        assert_eq!(
            summary.calls_by_model.get("claude-sonnet-4-20250514"),
            Some(&2)
        );
        assert_eq!(summary.calls_by_model.get("claude-haiku"), Some(&1));
    }

    #[test]
    fn export_summary_latency_totals() {
        let calls = vec![mock_call("m1", 0.01, 100), mock_call("m2", 0.02, 200)];
        let summary = ExportSummary::from_data(&calls, &[]);
        assert_eq!(summary.total_latency_ms, 1000); // 500 + 500
    }

    #[test]
    fn export_summary_with_calls_and_decisions() {
        let calls = vec![
            mock_call("model-a", 0.10, 100),
            mock_call("model-b", 0.05, 50),
        ];
        let decisions = vec![
            mock_decision(DecisionType::Decomposition, 0.10),
            mock_decision(DecisionType::ReviewOutcome, 0.03),
        ];

        let summary = ExportSummary::from_data(&calls, &decisions);

        assert_eq!(summary.total_llm_calls, 2);
        assert_eq!(summary.total_decisions, 2);
        assert_eq!(summary.total_tokens, 150);
        assert!((summary.total_cost_usd - 0.15).abs() < f64::EPSILON);
        assert_eq!(summary.cost_by_model.len(), 2);
        assert_eq!(summary.cost_by_decision_type.len(), 2);
        assert!(
            (summary.cost_by_decision_type.get("ReviewOutcome").unwrap() - 0.03).abs()
                < f64::EPSILON
        );
    }

    #[test]
    fn export_summary_cost_by_model() {
        let calls = vec![
            mock_call("model-a", 0.10, 100),
            mock_call("model-a", 0.15, 100),
            mock_call("model-b", 0.05, 100),
        ];

        let summary = ExportSummary::from_data(&calls, &[]);

        assert!((summary.cost_by_model.get("model-a").unwrap() - 0.25).abs() < f64::EPSILON);
        assert!((summary.cost_by_model.get("model-b").unwrap() - 0.05).abs() < f64::EPSILON);
    }

    #[test]
    fn export_summary_cost_by_decision_type() {
        let decisions = vec![
            mock_decision(DecisionType::Decomposition, 0.10),
            mock_decision(DecisionType::Decomposition, 0.05),
            mock_decision(DecisionType::TierRouting, 0.02),
        ];

        let summary = ExportSummary::from_data(&[], &decisions);

        assert_eq!(summary.total_decisions, 3);
        assert!(
            (summary.cost_by_decision_type.get("Decomposition").unwrap() - 0.15).abs()
                < f64::EPSILON
        );
        assert!(
            (summary.cost_by_decision_type.get("TierRouting").unwrap() - 0.02).abs() < f64::EPSILON
        );
    }

    #[test]
    fn session_export_serialization() {
        let export = SessionExport {
            version: "0.1.0".to_string(),
            exported_at: Utc::now(),
            time_range: TimeRange::last_24_hours(),
            summary: ExportSummary::from_data(&[], &[]),
            llm_calls: vec![],
            decisions: vec![],
        };

        let json = serde_json::to_string(&export).unwrap();
        assert!(json.contains("\"version\":\"0.1.0\""));
        assert!(json.contains("\"total_llm_calls\":0"));
    }

    #[test]
    fn session_export_serialization_with_data() {
        let calls = vec![mock_call("model-x", 0.05, 200)];
        let decisions = vec![mock_decision(DecisionType::Escalation, 0.01)];
        let summary = ExportSummary::from_data(&calls, &decisions);

        let export = SessionExport {
            version: "1.0.0".to_string(),
            exported_at: Utc::now(),
            time_range: TimeRange::last_hours(1),
            summary,
            llm_calls: calls,
            decisions,
        };

        let json = serde_json::to_string_pretty(&export).unwrap();
        assert!(json.contains("\"total_llm_calls\": 1"));
        assert!(json.contains("\"total_decisions\": 1"));
        assert!(json.contains("model-x"));
        assert!(json.contains("Escalation"));
        assert!(json.contains("\"version\": \"1.0.0\""));
    }

    #[test]
    fn time_range_clone() {
        let range = TimeRange::last_hours(3);
        let cloned = range.clone();
        assert_eq!(range.start, cloned.start);
        assert_eq!(range.end, cloned.end);
    }

    #[test]
    fn export_summary_single_model_single_call() {
        let calls = vec![mock_call("only-model", 0.42, 1000)];
        let summary = ExportSummary::from_data(&calls, &[]);
        assert_eq!(summary.total_llm_calls, 1);
        assert_eq!(summary.total_tokens, 1000);
        assert!((summary.total_cost_usd - 0.42).abs() < f64::EPSILON);
        assert_eq!(summary.calls_by_model.len(), 1);
        assert_eq!(summary.calls_by_model.get("only-model"), Some(&1));
    }

    #[test]
    fn export_summary_decisions_only() {
        let decisions = vec![
            mock_decision(DecisionType::Recovery, 0.07),
            mock_decision(DecisionType::Escalation, 0.03),
            mock_decision(DecisionType::Recovery, 0.08),
        ];
        let summary = ExportSummary::from_data(&[], &decisions);
        assert_eq!(summary.total_llm_calls, 0);
        assert_eq!(summary.total_decisions, 3);
        assert_eq!(summary.total_tokens, 0);
        assert!((summary.total_cost_usd - 0.0).abs() < f64::EPSILON);
        assert!(
            (summary.cost_by_decision_type.get("Recovery").unwrap() - 0.15).abs() < f64::EPSILON
        );
        assert!(
            (summary.cost_by_decision_type.get("Escalation").unwrap() - 0.03).abs() < f64::EPSILON
        );
    }

    // --- Async DB tests for SessionExporter ---

    #[tokio::test]
    async fn exporter_export_empty_range() {
        let (pool, _dir) = setup_test_db().await;
        let logger = LlmCallLogger::new(pool);
        let exporter = SessionExporter::new(logger);

        let range = TimeRange::last_hours(1);
        let export = exporter.export(range).await.unwrap();

        assert_eq!(export.summary.total_llm_calls, 0);
        assert_eq!(export.summary.total_decisions, 0);
        assert!(export.llm_calls.is_empty());
        assert!(export.decisions.is_empty());
        assert!(!export.version.is_empty());
    }

    #[tokio::test]
    async fn exporter_export_with_data() {
        let (pool, _dir) = setup_test_db().await;
        let logger = LlmCallLogger::new(pool);

        let task_id = Uuid::new_v4();
        let call = LlmCall::new("test-model", LlmPrompt::new("sys"), "resp")
            .with_task_id(task_id)
            .with_tokens(10, 20)
            .with_cost(0.01)
            .with_latency(100);
        logger.log_call(&call).await.unwrap();

        let decision =
            Decision::new(task_id, DecisionType::TierRouting, "reason", "outcome").with_cost(0.02);
        logger.log_decision(&decision).await.unwrap();

        let exporter = SessionExporter::new(logger);
        let range = TimeRange::last_hours(1);
        let export = exporter.export(range).await.unwrap();

        assert_eq!(export.summary.total_llm_calls, 1);
        assert_eq!(export.summary.total_decisions, 1);
        assert_eq!(export.llm_calls.len(), 1);
        assert_eq!(export.decisions.len(), 1);
        assert_eq!(export.llm_calls[0].model, "test-model");
    }

    #[tokio::test]
    async fn exporter_export_task_empty() {
        let (pool, _dir) = setup_test_db().await;
        let logger = LlmCallLogger::new(pool);
        let exporter = SessionExporter::new(logger);

        let task_id = Uuid::new_v4();
        let export = exporter.export_task(task_id).await.unwrap();

        // When no data, should fall back to last_24_hours time range
        assert_eq!(export.summary.total_llm_calls, 0);
        assert_eq!(export.summary.total_decisions, 0);
        let dur = export.time_range.duration_secs();
        assert!(dur >= 86400 - 10 && dur <= 86400 + 10);
    }

    #[tokio::test]
    async fn exporter_export_task_with_calls_only() {
        let (pool, _dir) = setup_test_db().await;
        let logger = LlmCallLogger::new(pool);

        let task_id = Uuid::new_v4();
        let call = LlmCall::new("m", LlmPrompt::new("s"), "r")
            .with_task_id(task_id)
            .with_tokens(5, 5)
            .with_cost(0.001);
        logger.log_call(&call).await.unwrap();

        let exporter = SessionExporter::new(logger);
        let export = exporter.export_task(task_id).await.unwrap();

        assert_eq!(export.summary.total_llm_calls, 1);
        assert_eq!(export.summary.total_decisions, 0);
        // Time range derived from the single call's timestamp
        assert!(export.time_range.duration_secs() >= 0);
    }

    #[tokio::test]
    async fn exporter_export_task_with_decisions_only() {
        let (pool, _dir) = setup_test_db().await;
        let logger = LlmCallLogger::new(pool);

        let task_id = Uuid::new_v4();
        let decision =
            Decision::new(task_id, DecisionType::Decomposition, "r", "o").with_cost(0.05);
        logger.log_decision(&decision).await.unwrap();

        let exporter = SessionExporter::new(logger);
        let export = exporter.export_task(task_id).await.unwrap();

        assert_eq!(export.summary.total_llm_calls, 0);
        assert_eq!(export.summary.total_decisions, 1);
    }

    #[tokio::test]
    async fn exporter_export_task_with_both() {
        let (pool, _dir) = setup_test_db().await;
        let logger = LlmCallLogger::new(pool);

        let task_id = Uuid::new_v4();

        let call = LlmCall::new("model", LlmPrompt::new("s"), "r")
            .with_task_id(task_id)
            .with_tokens(10, 10)
            .with_cost(0.01);
        logger.log_call(&call).await.unwrap();

        let decision =
            Decision::new(task_id, DecisionType::ReviewOutcome, "r", "o").with_cost(0.02);
        logger.log_decision(&decision).await.unwrap();

        let exporter = SessionExporter::new(logger);
        let export = exporter.export_task(task_id).await.unwrap();

        assert_eq!(export.summary.total_llm_calls, 1);
        assert_eq!(export.summary.total_decisions, 1);
        assert_eq!(export.llm_calls.len(), 1);
        assert_eq!(export.decisions.len(), 1);
        // Time range should span from earliest to latest timestamp
        assert!(export.time_range.duration_secs() >= 0);
    }

    #[tokio::test]
    async fn exporter_export_to_file() {
        let (pool, _dir) = setup_test_db().await;
        let logger = LlmCallLogger::new(pool);

        let task_id = Uuid::new_v4();
        let call = LlmCall::new("file-model", LlmPrompt::new("s"), "r")
            .with_task_id(task_id)
            .with_tokens(5, 5)
            .with_cost(0.001);
        logger.log_call(&call).await.unwrap();

        let exporter = SessionExporter::new(logger);
        let out_dir = TempDir::new().unwrap();
        let out_path = out_dir.path().join("export.json");

        exporter
            .export_to_file(&out_path, TimeRange::last_hours(1))
            .await
            .unwrap();

        let contents = std::fs::read_to_string(&out_path).unwrap();
        assert!(contents.contains("file-model"));
        assert!(contents.contains("total_llm_calls"));

        // Verify it's valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&contents).unwrap();
        assert_eq!(parsed["summary"]["total_llm_calls"], 1);
    }

    #[tokio::test]
    async fn exporter_export_task_to_file() {
        let (pool, _dir) = setup_test_db().await;
        let logger = LlmCallLogger::new(pool);

        let task_id = Uuid::new_v4();
        let decision = Decision::new(task_id, DecisionType::Recovery, "r", "o").with_cost(0.03);
        logger.log_decision(&decision).await.unwrap();

        let exporter = SessionExporter::new(logger);
        let out_dir = TempDir::new().unwrap();
        let out_path = out_dir.path().join("task_export.json");

        exporter
            .export_task_to_file(&out_path, task_id)
            .await
            .unwrap();

        let contents = std::fs::read_to_string(&out_path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&contents).unwrap();
        assert_eq!(parsed["summary"]["total_decisions"], 1);
        assert_eq!(parsed["summary"]["total_llm_calls"], 0);
    }

    #[tokio::test]
    async fn exporter_export_to_file_empty() {
        let (pool, _dir) = setup_test_db().await;
        let logger = LlmCallLogger::new(pool);
        let exporter = SessionExporter::new(logger);

        let out_dir = TempDir::new().unwrap();
        let out_path = out_dir.path().join("empty_export.json");

        exporter
            .export_to_file(&out_path, TimeRange::last_hours(1))
            .await
            .unwrap();

        let contents = std::fs::read_to_string(&out_path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&contents).unwrap();
        assert_eq!(parsed["summary"]["total_llm_calls"], 0);
        assert_eq!(parsed["llm_calls"].as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn exporter_export_task_to_file_empty() {
        let (pool, _dir) = setup_test_db().await;
        let logger = LlmCallLogger::new(pool);
        let exporter = SessionExporter::new(logger);

        let out_dir = TempDir::new().unwrap();
        let out_path = out_dir.path().join("empty_task.json");

        exporter
            .export_task_to_file(&out_path, Uuid::new_v4())
            .await
            .unwrap();

        let contents = std::fs::read_to_string(&out_path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&contents).unwrap();
        assert_eq!(parsed["summary"]["total_llm_calls"], 0);
        assert_eq!(parsed["summary"]["total_decisions"], 0);
    }
}
