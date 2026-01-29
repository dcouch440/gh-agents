//! LLM call and decision logging
//!
//! Captures all LLM interactions for debugging and replay.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

/// A logged LLM API call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmCall {
    pub id: Uuid,
    pub task_id: Option<Uuid>,
    pub agent_id: Option<String>,
    pub model: String,
    pub prompt: LlmPrompt,
    pub response: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub latency_ms: u64,
    pub timestamp: DateTime<Utc>,
    pub cost_usd: f64,
}

impl LlmCall {
    /// Create a new LLM call record
    pub fn new(model: impl Into<String>, prompt: LlmPrompt, response: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            task_id: None,
            agent_id: None,
            model: model.into(),
            prompt,
            response: response.into(),
            input_tokens: 0,
            output_tokens: 0,
            latency_ms: 0,
            timestamp: Utc::now(),
            cost_usd: 0.0,
        }
    }

    /// Set the task ID
    pub fn with_task_id(mut self, task_id: Uuid) -> Self {
        self.task_id = Some(task_id);
        self
    }

    /// Set the agent ID
    pub fn with_agent_id(mut self, agent_id: impl Into<String>) -> Self {
        self.agent_id = Some(agent_id.into());
        self
    }

    /// Set token counts
    pub fn with_tokens(mut self, input: u32, output: u32) -> Self {
        self.input_tokens = input;
        self.output_tokens = output;
        self
    }

    /// Set latency
    pub fn with_latency(mut self, latency_ms: u64) -> Self {
        self.latency_ms = latency_ms;
        self
    }

    /// Set cost
    pub fn with_cost(mut self, cost_usd: f64) -> Self {
        self.cost_usd = cost_usd;
        self
    }

    /// Total tokens used
    pub fn total_tokens(&self) -> u32 {
        self.input_tokens + self.output_tokens
    }
}

/// The prompt sent to an LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmPrompt {
    pub system: String,
    pub messages: Vec<PromptMessage>,
}

impl LlmPrompt {
    /// Create a new prompt with just a system message
    pub fn new(system: impl Into<String>) -> Self {
        Self {
            system: system.into(),
            messages: Vec::new(),
        }
    }

    /// Add a message to the conversation
    pub fn add_message(&mut self, role: impl Into<String>, content: impl Into<String>) {
        self.messages.push(PromptMessage {
            role: role.into(),
            content: content.into(),
        });
    }

    /// Create with messages
    pub fn with_messages(mut self, messages: Vec<PromptMessage>) -> Self {
        self.messages = messages;
        self
    }
}

/// A message in the prompt conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptMessage {
    pub role: String, // "user" or "assistant"
    pub content: String,
}

impl PromptMessage {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.into(),
        }
    }
}

/// A logged orchestrator decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    pub id: Uuid,
    pub task_id: Uuid,
    pub decision_type: DecisionType,
    pub reasoning: String,
    pub outcome: String,
    pub llm_call_id: Option<Uuid>,
    pub cost_usd: f64,
    pub timestamp: DateTime<Utc>,
}

impl Decision {
    /// Create a new decision
    pub fn new(
        task_id: Uuid,
        decision_type: DecisionType,
        reasoning: impl Into<String>,
        outcome: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            task_id,
            decision_type,
            reasoning: reasoning.into(),
            outcome: outcome.into(),
            llm_call_id: None,
            cost_usd: 0.0,
            timestamp: Utc::now(),
        }
    }

    /// Link to the LLM call that made this decision
    pub fn with_llm_call(mut self, call_id: Uuid) -> Self {
        self.llm_call_id = Some(call_id);
        self
    }

    /// Set the cost
    pub fn with_cost(mut self, cost_usd: f64) -> Self {
        self.cost_usd = cost_usd;
        self
    }
}

/// Types of orchestrator decisions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DecisionType {
    /// Breaking ticket into slices
    Decomposition,
    /// Assigning task to tier
    TierRouting,
    /// Approving or rejecting work
    ReviewOutcome,
    /// Escalating to higher tier
    Escalation,
    /// Recovering from failure
    Recovery,
}

impl std::fmt::Display for DecisionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecisionType::Decomposition => write!(f, "Decomposition"),
            DecisionType::TierRouting => write!(f, "TierRouting"),
            DecisionType::ReviewOutcome => write!(f, "ReviewOutcome"),
            DecisionType::Escalation => write!(f, "Escalation"),
            DecisionType::Recovery => write!(f, "Recovery"),
        }
    }
}

/// Logger for LLM calls and decisions
#[derive(Clone)]
pub struct LlmCallLogger {
    pool: PgPool,
}

impl LlmCallLogger {
    /// Create a new logger
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Log an LLM call
    pub async fn log_call(&self, call: &LlmCall) -> Result<()> {
        let prompt_json = serde_json::to_string(&call.prompt)?;
        let id = call.id.to_string();
        let task_id = call.task_id.map(|u| u.to_string());
        let timestamp = call.timestamp.to_rfc3339();
        let input_tokens = call.input_tokens as i64;
        let output_tokens = call.output_tokens as i64;
        let latency_ms = call.latency_ms as i64;

        sqlx::query(
            r#"
            INSERT INTO llm_calls (
                id, task_id, agent_id, model, prompt, response,
                input_tokens, output_tokens, latency_ms, timestamp, cost_usd
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            "#,
        )
        .bind(&id)
        .bind(&task_id)
        .bind(&call.agent_id)
        .bind(&call.model)
        .bind(&prompt_json)
        .bind(&call.response)
        .bind(input_tokens)
        .bind(output_tokens)
        .bind(latency_ms)
        .bind(&timestamp)
        .bind(call.cost_usd)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get all LLM calls for a task
    pub async fn get_calls_for_task(&self, task_id: Uuid) -> Result<Vec<LlmCall>> {
        let task_id_str = task_id.to_string();
        let rows: Vec<LlmCallRow> = sqlx::query_as(
            r#"
            SELECT id, task_id, agent_id, model, prompt, response,
                   input_tokens, output_tokens, latency_ms, timestamp, cost_usd
            FROM llm_calls
            WHERE task_id = $1
            ORDER BY timestamp ASC
            "#,
        )
        .bind(&task_id_str)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|row| row.try_into()).collect()
    }

    /// Get LLM calls within a time range
    pub async fn get_calls_in_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<LlmCall>> {
        let start_str = start.to_rfc3339();
        let end_str = end.to_rfc3339();
        let rows: Vec<LlmCallRow> = sqlx::query_as(
            r#"
            SELECT id, task_id, agent_id, model, prompt, response,
                   input_tokens, output_tokens, latency_ms, timestamp, cost_usd
            FROM llm_calls
            WHERE timestamp >= $1 AND timestamp <= $2
            ORDER BY timestamp ASC
            "#,
        )
        .bind(&start_str)
        .bind(&end_str)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|row| row.try_into()).collect()
    }

    /// Log a decision
    pub async fn log_decision(&self, decision: &Decision) -> Result<()> {
        let id = decision.id.to_string();
        let task_id = decision.task_id.to_string();
        let decision_type = serde_json::to_string(&decision.decision_type)?;
        let llm_call_id = decision.llm_call_id.map(|u| u.to_string());
        let timestamp = decision.timestamp.to_rfc3339();

        sqlx::query(
            r#"
            INSERT INTO decisions (
                id, task_id, decision_type, reasoning, outcome, llm_call_id, cost_usd, timestamp
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
        )
        .bind(&id)
        .bind(&task_id)
        .bind(&decision_type)
        .bind(&decision.reasoning)
        .bind(&decision.outcome)
        .bind(&llm_call_id)
        .bind(decision.cost_usd)
        .bind(&timestamp)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get all decisions for a task
    pub async fn get_decisions_for_task(&self, task_id: Uuid) -> Result<Vec<Decision>> {
        let task_id_str = task_id.to_string();
        let rows: Vec<DecisionRow> = sqlx::query_as(
            r#"
            SELECT id, task_id, decision_type, reasoning, outcome, llm_call_id, cost_usd, timestamp
            FROM decisions
            WHERE task_id = $1
            ORDER BY timestamp ASC
            "#,
        )
        .bind(&task_id_str)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|row| row.try_into()).collect()
    }

    /// Get decisions within a time range
    pub async fn get_decisions_in_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Decision>> {
        let start_str = start.to_rfc3339();
        let end_str = end.to_rfc3339();
        let rows: Vec<DecisionRow> = sqlx::query_as(
            r#"
            SELECT id, task_id, decision_type, reasoning, outcome, llm_call_id, cost_usd, timestamp
            FROM decisions
            WHERE timestamp >= $1 AND timestamp <= $2
            ORDER BY timestamp ASC
            "#,
        )
        .bind(&start_str)
        .bind(&end_str)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|row| row.try_into()).collect()
    }
}

// Internal row types for SQLx

#[derive(sqlx::FromRow)]
struct LlmCallRow {
    id: String,
    task_id: Option<String>,
    agent_id: Option<String>,
    model: String,
    prompt: String,
    response: String,
    input_tokens: i64,
    output_tokens: i64,
    latency_ms: i64,
    timestamp: String,
    cost_usd: f64,
}

impl TryFrom<LlmCallRow> for LlmCall {
    type Error = anyhow::Error;

    fn try_from(row: LlmCallRow) -> Result<Self> {
        Ok(LlmCall {
            id: Uuid::parse_str(&row.id)?,
            task_id: row.task_id.map(|s| Uuid::parse_str(&s)).transpose()?,
            agent_id: row.agent_id,
            model: row.model,
            prompt: serde_json::from_str(&row.prompt)?,
            response: row.response,
            input_tokens: row.input_tokens as u32,
            output_tokens: row.output_tokens as u32,
            latency_ms: row.latency_ms as u64,
            timestamp: DateTime::parse_from_rfc3339(&row.timestamp)?.with_timezone(&Utc),
            cost_usd: row.cost_usd,
        })
    }
}

#[derive(sqlx::FromRow)]
struct DecisionRow {
    id: String,
    task_id: String,
    decision_type: String,
    reasoning: String,
    outcome: String,
    llm_call_id: Option<String>,
    cost_usd: f64,
    timestamp: String,
}

impl TryFrom<DecisionRow> for Decision {
    type Error = anyhow::Error;

    fn try_from(row: DecisionRow) -> Result<Self> {
        Ok(Decision {
            id: Uuid::parse_str(&row.id)?,
            task_id: Uuid::parse_str(&row.task_id)?,
            decision_type: serde_json::from_str(&row.decision_type)?,
            reasoning: row.reasoning,
            outcome: row.outcome,
            llm_call_id: row.llm_call_id.map(|s| Uuid::parse_str(&s)).transpose()?,
            cost_usd: row.cost_usd,
            timestamp: DateTime::parse_from_rfc3339(&row.timestamp)?.with_timezone(&Utc),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn llm_call_builder() {
        let prompt = LlmPrompt::new("You are a helpful assistant");
        let call = LlmCall::new("claude-sonnet-4-20250514", prompt, "Hello!")
            .with_tokens(100, 50)
            .with_latency(1500)
            .with_cost(0.0015);

        assert_eq!(call.model, "claude-sonnet-4-20250514");
        assert_eq!(call.input_tokens, 100);
        assert_eq!(call.output_tokens, 50);
        assert_eq!(call.total_tokens(), 150);
        assert_eq!(call.latency_ms, 1500);
        assert!((call.cost_usd - 0.0015).abs() < f64::EPSILON);
    }

    #[test]
    fn llm_call_with_context() {
        let task_id = Uuid::new_v4();
        let prompt = LlmPrompt::new("System");
        let call = LlmCall::new("model", prompt, "response")
            .with_task_id(task_id)
            .with_agent_id("agent-1");

        assert_eq!(call.task_id, Some(task_id));
        assert_eq!(call.agent_id, Some("agent-1".to_string()));
    }

    #[test]
    fn llm_prompt_with_messages() {
        let mut prompt = LlmPrompt::new("System prompt");
        prompt.add_message("user", "Hello");
        prompt.add_message("assistant", "Hi there!");

        assert_eq!(prompt.system, "System prompt");
        assert_eq!(prompt.messages.len(), 2);
        assert_eq!(prompt.messages[0].role, "user");
        assert_eq!(prompt.messages[1].role, "assistant");
    }

    #[test]
    fn prompt_message_helpers() {
        let user_msg = PromptMessage::user("Hello");
        let assistant_msg = PromptMessage::assistant("Hi");

        assert_eq!(user_msg.role, "user");
        assert_eq!(assistant_msg.role, "assistant");
    }

    #[test]
    fn decision_builder() {
        let task_id = Uuid::new_v4();
        let call_id = Uuid::new_v4();
        let decision = Decision::new(
            task_id,
            DecisionType::Decomposition,
            "Need to split into smaller tasks",
            "Created 3 slices",
        )
        .with_llm_call(call_id)
        .with_cost(0.05);

        assert_eq!(decision.task_id, task_id);
        assert_eq!(decision.decision_type, DecisionType::Decomposition);
        assert_eq!(decision.llm_call_id, Some(call_id));
        assert!((decision.cost_usd - 0.05).abs() < f64::EPSILON);
    }

    #[test]
    fn decision_type_display() {
        assert_eq!(DecisionType::Decomposition.to_string(), "Decomposition");
        assert_eq!(DecisionType::TierRouting.to_string(), "TierRouting");
        assert_eq!(DecisionType::ReviewOutcome.to_string(), "ReviewOutcome");
        assert_eq!(DecisionType::Escalation.to_string(), "Escalation");
        assert_eq!(DecisionType::Recovery.to_string(), "Recovery");
    }

    #[test]
    fn decision_type_serialization() {
        let dt = DecisionType::Decomposition;
        let json = serde_json::to_string(&dt).unwrap();
        let parsed: DecisionType = serde_json::from_str(&json).unwrap();
        assert_eq!(dt, parsed);
    }

    #[test]
    fn llm_call_serialization() {
        let prompt = LlmPrompt::new("System").with_messages(vec![PromptMessage::user("Hello")]);
        let call = LlmCall::new("model", prompt, "response");

        let json = serde_json::to_string(&call).unwrap();
        let parsed: LlmCall = serde_json::from_str(&json).unwrap();

        assert_eq!(call.model, parsed.model);
        assert_eq!(call.response, parsed.response);
    }

    #[test]
    fn llm_call_defaults() {
        let prompt = LlmPrompt::new("sys");
        let call = LlmCall::new("model", prompt, "resp");
        assert_eq!(call.input_tokens, 0);
        assert_eq!(call.output_tokens, 0);
        assert_eq!(call.latency_ms, 0);
        assert_eq!(call.cost_usd, 0.0);
        assert!(call.task_id.is_none());
        assert!(call.agent_id.is_none());
    }

    #[test]
    fn llm_call_total_tokens() {
        let prompt = LlmPrompt::new("sys");
        let call = LlmCall::new("model", prompt, "resp").with_tokens(100, 200);
        assert_eq!(call.total_tokens(), 300);
    }

    #[test]
    fn llm_prompt_new_empty_messages() {
        let prompt = LlmPrompt::new("system");
        assert_eq!(prompt.system, "system");
        assert!(prompt.messages.is_empty());
    }

    #[test]
    fn decision_new_defaults() {
        let task_id = Uuid::new_v4();
        let d = Decision::new(task_id, DecisionType::Escalation, "reason", "outcome");
        assert_eq!(d.task_id, task_id);
        assert!(d.llm_call_id.is_none());
        assert_eq!(d.cost_usd, 0.0);
    }

    #[test]
    fn decision_type_serde_all_variants() {
        let variants = [
            DecisionType::Decomposition,
            DecisionType::TierRouting,
            DecisionType::ReviewOutcome,
            DecisionType::Escalation,
            DecisionType::Recovery,
        ];
        for v in &variants {
            let json = serde_json::to_string(v).unwrap();
            let parsed: DecisionType = serde_json::from_str(&json).unwrap();
            assert_eq!(*v, parsed);
        }
    }

    // --- Async DB tests ---

    use tempfile::TempDir;

    async fn setup_test_db() -> PgPool {
        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set for tests");
        crate::db::init_db_with_url(&url).await.unwrap()
    }

    fn make_call(model: &str, task_id: Option<Uuid>) -> LlmCall {
        let prompt = LlmPrompt::new("system").with_messages(vec![PromptMessage::user("hi")]);
        let mut call = LlmCall::new(model, prompt, "response")
            .with_tokens(10, 20)
            .with_latency(100)
            .with_cost(0.001);
        if let Some(tid) = task_id {
            call = call.with_task_id(tid);
        }
        call
    }

    #[tokio::test]
    async fn log_call_and_retrieve() {
        let pool = setup_test_db().await;
        let logger = LlmCallLogger::new(pool);

        let task_id = Uuid::new_v4();
        let call = make_call("test-model", Some(task_id)).with_agent_id("agent-1");
        logger.log_call(&call).await.unwrap();

        let calls = logger.get_calls_for_task(task_id).await.unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].id, call.id);
        assert_eq!(calls[0].model, "test-model");
        assert_eq!(calls[0].agent_id, Some("agent-1".to_string()));
        assert_eq!(calls[0].input_tokens, 10);
        assert_eq!(calls[0].output_tokens, 20);
        assert_eq!(calls[0].latency_ms, 100);
        assert!((calls[0].cost_usd - 0.001).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn get_calls_for_task_filters_correctly() {
        let pool = setup_test_db().await;
        let logger = LlmCallLogger::new(pool);

        let tid1 = Uuid::new_v4();
        let tid2 = Uuid::new_v4();
        logger.log_call(&make_call("m1", Some(tid1))).await.unwrap();
        logger.log_call(&make_call("m2", Some(tid1))).await.unwrap();
        logger.log_call(&make_call("m3", Some(tid2))).await.unwrap();

        let calls = logger.get_calls_for_task(tid1).await.unwrap();
        assert_eq!(calls.len(), 2);
        let calls2 = logger.get_calls_for_task(tid2).await.unwrap();
        assert_eq!(calls2.len(), 1);
        assert_eq!(calls2[0].model, "m3");
    }

    #[tokio::test]
    async fn get_calls_in_range_filters_correctly() {
        let pool = setup_test_db().await;
        let logger = LlmCallLogger::new(pool);

        let t1 = Utc::now() - chrono::Duration::hours(2);
        let t2 = Utc::now() - chrono::Duration::hours(1);
        let t3 = Utc::now();

        let mut c1 = make_call("old", None);
        c1.timestamp = t1;
        let mut c2 = make_call("mid", None);
        c2.timestamp = t2;
        let mut c3 = make_call("new", None);
        c3.timestamp = t3;

        logger.log_call(&c1).await.unwrap();
        logger.log_call(&c2).await.unwrap();
        logger.log_call(&c3).await.unwrap();

        let range_start = t1 - chrono::Duration::seconds(1);
        let range_end = t2 + chrono::Duration::seconds(1);
        let calls = logger
            .get_calls_in_range(range_start, range_end)
            .await
            .unwrap();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].model, "old");
        assert_eq!(calls[1].model, "mid");
    }

    #[tokio::test]
    async fn log_decision_and_retrieve() {
        let pool = setup_test_db().await;
        let logger = LlmCallLogger::new(pool);

        let task_id = Uuid::new_v4();
        let call_id = Uuid::new_v4();
        let decision = Decision::new(task_id, DecisionType::TierRouting, "reason", "outcome")
            .with_llm_call(call_id)
            .with_cost(0.05);
        logger.log_decision(&decision).await.unwrap();

        let decisions = logger.get_decisions_for_task(task_id).await.unwrap();
        assert_eq!(decisions.len(), 1);
        assert_eq!(decisions[0].id, decision.id);
        assert_eq!(decisions[0].decision_type, DecisionType::TierRouting);
        assert_eq!(decisions[0].reasoning, "reason");
        assert_eq!(decisions[0].outcome, "outcome");
        assert_eq!(decisions[0].llm_call_id, Some(call_id));
        assert!((decisions[0].cost_usd - 0.05).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn get_decisions_for_task_filters_correctly() {
        let pool = setup_test_db().await;
        let logger = LlmCallLogger::new(pool);

        let tid1 = Uuid::new_v4();
        let tid2 = Uuid::new_v4();
        logger
            .log_decision(&Decision::new(
                tid1,
                DecisionType::Decomposition,
                "r1",
                "o1",
            ))
            .await
            .unwrap();
        logger
            .log_decision(&Decision::new(tid1, DecisionType::Escalation, "r2", "o2"))
            .await
            .unwrap();
        logger
            .log_decision(&Decision::new(tid2, DecisionType::Recovery, "r3", "o3"))
            .await
            .unwrap();

        let d1 = logger.get_decisions_for_task(tid1).await.unwrap();
        assert_eq!(d1.len(), 2);
        let d2 = logger.get_decisions_for_task(tid2).await.unwrap();
        assert_eq!(d2.len(), 1);
        assert_eq!(d2[0].decision_type, DecisionType::Recovery);
    }

    #[tokio::test]
    async fn get_decisions_in_range_filters_correctly() {
        let pool = setup_test_db().await;
        let logger = LlmCallLogger::new(pool);

        let tid = Uuid::new_v4();
        let t1 = Utc::now() - chrono::Duration::hours(3);
        let t2 = Utc::now();

        let mut d1 = Decision::new(tid, DecisionType::Decomposition, "old", "o1");
        d1.timestamp = t1;
        let mut d2 = Decision::new(tid, DecisionType::Escalation, "new", "o2");
        d2.timestamp = t2;

        logger.log_decision(&d1).await.unwrap();
        logger.log_decision(&d2).await.unwrap();

        let range_start = t1 - chrono::Duration::seconds(1);
        let range_end = t1 + chrono::Duration::seconds(1);
        let results = logger
            .get_decisions_in_range(range_start, range_end)
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].reasoning, "old");
    }

    #[test]
    fn llm_call_row_try_from_valid() {
        let id = Uuid::new_v4();
        let task_id = Uuid::new_v4();
        let prompt = LlmPrompt::new("sys");
        let prompt_json = serde_json::to_string(&prompt).unwrap();
        let ts = Utc::now().to_rfc3339();

        let row = LlmCallRow {
            id: id.to_string(),
            task_id: Some(task_id.to_string()),
            agent_id: Some("a1".to_string()),
            model: "m".to_string(),
            prompt: prompt_json,
            response: "r".to_string(),
            input_tokens: 5,
            output_tokens: 10,
            latency_ms: 50,
            timestamp: ts,
            cost_usd: 0.01,
        };

        let call: LlmCall = row.try_into().unwrap();
        assert_eq!(call.id, id);
        assert_eq!(call.task_id, Some(task_id));
        assert_eq!(call.input_tokens, 5);
    }

    #[test]
    fn llm_call_row_try_from_invalid_uuid() {
        let row = LlmCallRow {
            id: "not-a-uuid".to_string(),
            task_id: None,
            agent_id: None,
            model: "m".to_string(),
            prompt: r#"{"system":"s","messages":[]}"#.to_string(),
            response: "r".to_string(),
            input_tokens: 0,
            output_tokens: 0,
            latency_ms: 0,
            timestamp: Utc::now().to_rfc3339(),
            cost_usd: 0.0,
        };
        let result: Result<LlmCall> = row.try_into();
        assert!(result.is_err());
    }

    #[test]
    fn decision_row_try_from_valid() {
        let id = Uuid::new_v4();
        let task_id = Uuid::new_v4();
        let call_id = Uuid::new_v4();
        let ts = Utc::now().to_rfc3339();
        let dt_json = serde_json::to_string(&DecisionType::ReviewOutcome).unwrap();

        let row = DecisionRow {
            id: id.to_string(),
            task_id: task_id.to_string(),
            decision_type: dt_json,
            reasoning: "r".to_string(),
            outcome: "o".to_string(),
            llm_call_id: Some(call_id.to_string()),
            cost_usd: 0.02,
            timestamp: ts,
        };

        let d: Decision = row.try_into().unwrap();
        assert_eq!(d.id, id);
        assert_eq!(d.decision_type, DecisionType::ReviewOutcome);
        assert_eq!(d.llm_call_id, Some(call_id));
    }

    #[test]
    fn decision_row_try_from_invalid_uuid() {
        let row = DecisionRow {
            id: "bad".to_string(),
            task_id: Uuid::new_v4().to_string(),
            decision_type: r#""Decomposition""#.to_string(),
            reasoning: "r".to_string(),
            outcome: "o".to_string(),
            llm_call_id: None,
            cost_usd: 0.0,
            timestamp: Utc::now().to_rfc3339(),
        };
        let result: Result<Decision> = row.try_into();
        assert!(result.is_err());
    }

    #[test]
    fn llm_call_row_try_from_invalid_prompt_json() {
        let row = LlmCallRow {
            id: Uuid::new_v4().to_string(),
            task_id: None,
            agent_id: None,
            model: "m".to_string(),
            prompt: "not json".to_string(),
            response: "r".to_string(),
            input_tokens: 0,
            output_tokens: 0,
            latency_ms: 0,
            timestamp: Utc::now().to_rfc3339(),
            cost_usd: 0.0,
        };
        let result: Result<LlmCall> = row.try_into();
        assert!(result.is_err());
    }

    #[test]
    fn llm_call_row_try_from_invalid_timestamp() {
        let prompt = LlmPrompt::new("sys");
        let prompt_json = serde_json::to_string(&prompt).unwrap();
        let row = LlmCallRow {
            id: Uuid::new_v4().to_string(),
            task_id: None,
            agent_id: None,
            model: "m".to_string(),
            prompt: prompt_json,
            response: "r".to_string(),
            input_tokens: 0,
            output_tokens: 0,
            latency_ms: 0,
            timestamp: "not-a-timestamp".to_string(),
            cost_usd: 0.0,
        };
        let result: Result<LlmCall> = row.try_into();
        assert!(result.is_err());
    }

    #[test]
    fn llm_call_row_try_from_invalid_task_id_uuid() {
        let prompt = LlmPrompt::new("sys");
        let prompt_json = serde_json::to_string(&prompt).unwrap();
        let row = LlmCallRow {
            id: Uuid::new_v4().to_string(),
            task_id: Some("not-a-uuid".to_string()),
            agent_id: None,
            model: "m".to_string(),
            prompt: prompt_json,
            response: "r".to_string(),
            input_tokens: 0,
            output_tokens: 0,
            latency_ms: 0,
            timestamp: Utc::now().to_rfc3339(),
            cost_usd: 0.0,
        };
        let result: Result<LlmCall> = row.try_into();
        assert!(result.is_err());
    }

    #[test]
    fn llm_call_row_try_from_no_task_id() {
        let prompt = LlmPrompt::new("sys");
        let prompt_json = serde_json::to_string(&prompt).unwrap();
        let row = LlmCallRow {
            id: Uuid::new_v4().to_string(),
            task_id: None,
            agent_id: None,
            model: "m".to_string(),
            prompt: prompt_json,
            response: "r".to_string(),
            input_tokens: 0,
            output_tokens: 0,
            latency_ms: 0,
            timestamp: Utc::now().to_rfc3339(),
            cost_usd: 0.0,
        };
        let call: LlmCall = row.try_into().unwrap();
        assert!(call.task_id.is_none());
        assert!(call.agent_id.is_none());
    }

    #[test]
    fn decision_row_try_from_invalid_task_id() {
        let row = DecisionRow {
            id: Uuid::new_v4().to_string(),
            task_id: "bad-uuid".to_string(),
            decision_type: r#""Decomposition""#.to_string(),
            reasoning: "r".to_string(),
            outcome: "o".to_string(),
            llm_call_id: None,
            cost_usd: 0.0,
            timestamp: Utc::now().to_rfc3339(),
        };
        let result: Result<Decision> = row.try_into();
        assert!(result.is_err());
    }

    #[test]
    fn decision_row_try_from_invalid_decision_type() {
        let row = DecisionRow {
            id: Uuid::new_v4().to_string(),
            task_id: Uuid::new_v4().to_string(),
            decision_type: "not valid json".to_string(),
            reasoning: "r".to_string(),
            outcome: "o".to_string(),
            llm_call_id: None,
            cost_usd: 0.0,
            timestamp: Utc::now().to_rfc3339(),
        };
        let result: Result<Decision> = row.try_into();
        assert!(result.is_err());
    }

    #[test]
    fn decision_row_try_from_invalid_timestamp() {
        let row = DecisionRow {
            id: Uuid::new_v4().to_string(),
            task_id: Uuid::new_v4().to_string(),
            decision_type: r#""Escalation""#.to_string(),
            reasoning: "r".to_string(),
            outcome: "o".to_string(),
            llm_call_id: None,
            cost_usd: 0.0,
            timestamp: "bad-ts".to_string(),
        };
        let result: Result<Decision> = row.try_into();
        assert!(result.is_err());
    }

    #[test]
    fn decision_row_try_from_invalid_llm_call_id() {
        let row = DecisionRow {
            id: Uuid::new_v4().to_string(),
            task_id: Uuid::new_v4().to_string(),
            decision_type: r#""Recovery""#.to_string(),
            reasoning: "r".to_string(),
            outcome: "o".to_string(),
            llm_call_id: Some("not-a-uuid".to_string()),
            cost_usd: 0.0,
            timestamp: Utc::now().to_rfc3339(),
        };
        let result: Result<Decision> = row.try_into();
        assert!(result.is_err());
    }

    #[test]
    fn decision_row_try_from_no_llm_call_id() {
        let row = DecisionRow {
            id: Uuid::new_v4().to_string(),
            task_id: Uuid::new_v4().to_string(),
            decision_type: r#""TierRouting""#.to_string(),
            reasoning: "r".to_string(),
            outcome: "o".to_string(),
            llm_call_id: None,
            cost_usd: 0.0,
            timestamp: Utc::now().to_rfc3339(),
        };
        let d: Decision = row.try_into().unwrap();
        assert!(d.llm_call_id.is_none());
    }

    #[test]
    fn decision_serialization_roundtrip() {
        let task_id = Uuid::new_v4();
        let call_id = Uuid::new_v4();
        let decision = Decision::new(task_id, DecisionType::Recovery, "reason", "outcome")
            .with_llm_call(call_id)
            .with_cost(1.23);

        let json = serde_json::to_string(&decision).unwrap();
        let parsed: Decision = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.task_id, task_id);
        assert_eq!(parsed.decision_type, DecisionType::Recovery);
        assert_eq!(parsed.llm_call_id, Some(call_id));
        assert_eq!(parsed.reasoning, "reason");
        assert_eq!(parsed.outcome, "outcome");
    }

    #[test]
    fn llm_prompt_serialization_roundtrip() {
        let mut prompt = LlmPrompt::new("system msg");
        prompt.add_message("user", "hello");
        prompt.add_message("assistant", "hi");

        let json = serde_json::to_string(&prompt).unwrap();
        let parsed: LlmPrompt = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.system, "system msg");
        assert_eq!(parsed.messages.len(), 2);
        assert_eq!(parsed.messages[0].role, "user");
        assert_eq!(parsed.messages[0].content, "hello");
        assert_eq!(parsed.messages[1].role, "assistant");
        assert_eq!(parsed.messages[1].content, "hi");
    }

    #[test]
    fn prompt_message_serialization_roundtrip() {
        let msg = PromptMessage::user("test content");
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: PromptMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.role, "user");
        assert_eq!(parsed.content, "test content");
    }

    #[test]
    fn llm_call_with_empty_strings() {
        let prompt = LlmPrompt::new("");
        let call = LlmCall::new("", prompt, "");
        assert_eq!(call.model, "");
        assert_eq!(call.response, "");
        assert_eq!(call.prompt.system, "");
    }

    #[test]
    fn decision_with_empty_strings() {
        let task_id = Uuid::new_v4();
        let d = Decision::new(task_id, DecisionType::Decomposition, "", "");
        assert_eq!(d.reasoning, "");
        assert_eq!(d.outcome, "");
    }

    #[test]
    fn llm_call_total_tokens_zero() {
        let prompt = LlmPrompt::new("sys");
        let call = LlmCall::new("m", prompt, "r");
        assert_eq!(call.total_tokens(), 0);
    }

    #[test]
    fn llm_prompt_with_messages_replaces() {
        let prompt = LlmPrompt::new("sys").with_messages(vec![
            PromptMessage::user("a"),
            PromptMessage::assistant("b"),
        ]);
        assert_eq!(prompt.messages.len(), 2);

        // with_messages replaces, not appends
        let prompt2 = prompt.with_messages(vec![PromptMessage::user("c")]);
        assert_eq!(prompt2.messages.len(), 1);
        assert_eq!(prompt2.messages[0].content, "c");
    }

    #[test]
    fn decision_type_equality() {
        assert_eq!(DecisionType::Decomposition, DecisionType::Decomposition);
        assert_ne!(DecisionType::Decomposition, DecisionType::Escalation);
    }

    #[test]
    fn llm_call_clone() {
        let prompt = LlmPrompt::new("sys");
        let call = LlmCall::new("model", prompt, "resp")
            .with_task_id(Uuid::new_v4())
            .with_agent_id("a")
            .with_tokens(1, 2)
            .with_latency(3)
            .with_cost(0.5);
        let cloned = call.clone();
        assert_eq!(call.id, cloned.id);
        assert_eq!(call.model, cloned.model);
        assert_eq!(call.task_id, cloned.task_id);
        assert_eq!(call.agent_id, cloned.agent_id);
    }

    #[test]
    fn decision_clone() {
        let d = Decision::new(Uuid::new_v4(), DecisionType::Escalation, "r", "o")
            .with_llm_call(Uuid::new_v4())
            .with_cost(0.1);
        let cloned = d.clone();
        assert_eq!(d.id, cloned.id);
        assert_eq!(d.decision_type, cloned.decision_type);
    }

    #[tokio::test]
    async fn log_decision_without_llm_call() {
        let pool = setup_test_db().await;
        let logger = LlmCallLogger::new(pool);

        let task_id = Uuid::new_v4();
        let decision = Decision::new(task_id, DecisionType::Decomposition, "r", "o");
        logger.log_decision(&decision).await.unwrap();

        let decisions = logger.get_decisions_for_task(task_id).await.unwrap();
        assert_eq!(decisions.len(), 1);
        assert!(decisions[0].llm_call_id.is_none());
    }

    #[tokio::test]
    async fn get_calls_for_nonexistent_task() {
        let pool = setup_test_db().await;
        let logger = LlmCallLogger::new(pool);

        let calls = logger.get_calls_for_task(Uuid::new_v4()).await.unwrap();
        assert!(calls.is_empty());
    }

    #[tokio::test]
    async fn get_decisions_for_nonexistent_task() {
        let pool = setup_test_db().await;
        let logger = LlmCallLogger::new(pool);

        let decisions = logger.get_decisions_for_task(Uuid::new_v4()).await.unwrap();
        assert!(decisions.is_empty());
    }

    #[tokio::test]
    async fn get_calls_in_range_empty() {
        let pool = setup_test_db().await;
        let logger = LlmCallLogger::new(pool);

        let start = Utc::now() - chrono::Duration::hours(10);
        let end = Utc::now() - chrono::Duration::hours(9);
        let calls = logger.get_calls_in_range(start, end).await.unwrap();
        assert!(calls.is_empty());
    }

    #[tokio::test]
    async fn get_decisions_in_range_empty() {
        let pool = setup_test_db().await;
        let logger = LlmCallLogger::new(pool);

        let start = Utc::now() - chrono::Duration::hours(10);
        let end = Utc::now() - chrono::Duration::hours(9);
        let decisions = logger.get_decisions_in_range(start, end).await.unwrap();
        assert!(decisions.is_empty());
    }

    #[tokio::test]
    async fn log_call_without_task_id() {
        let pool = setup_test_db().await;
        let logger = LlmCallLogger::new(pool);

        let call = make_call("model", None);
        logger.log_call(&call).await.unwrap();

        // Should not appear when filtering by a random task_id
        let calls = logger.get_calls_for_task(Uuid::new_v4()).await.unwrap();
        assert!(calls.is_empty());
    }
}
