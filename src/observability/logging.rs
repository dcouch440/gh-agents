//! LLM call and decision logging
//!
//! Captures all LLM interactions for debugging and replay.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
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
    pub fn new(task_id: Uuid, decision_type: DecisionType, reasoning: impl Into<String>, outcome: impl Into<String>) -> Self {
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
    pool: SqlitePool,
}

impl LlmCallLogger {
    /// Create a new logger
    pub fn new(pool: SqlitePool) -> Self {
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
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
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
            WHERE task_id = ?
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
            WHERE timestamp >= ? AND timestamp <= ?
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
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
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
            WHERE task_id = ?
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
            WHERE timestamp >= ? AND timestamp <= ?
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
            timestamp: DateTime::parse_from_rfc3339(&row.timestamp)?
                .with_timezone(&Utc),
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
            timestamp: DateTime::parse_from_rfc3339(&row.timestamp)?
                .with_timezone(&Utc),
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
        let prompt = LlmPrompt::new("System").with_messages(vec![
            PromptMessage::user("Hello"),
        ]);
        let call = LlmCall::new("model", prompt, "response");

        let json = serde_json::to_string(&call).unwrap();
        let parsed: LlmCall = serde_json::from_str(&json).unwrap();

        assert_eq!(call.model, parsed.model);
        assert_eq!(call.response, parsed.response);
    }
}
