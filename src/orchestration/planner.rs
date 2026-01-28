//! Planner for decomposing tickets into vertical slices.
//!
//! The Planner is the core intelligence of the orchestration layer. It:
//! 1. Takes a ticket as input
//! 2. Builds a decomposition prompt using M4's templates
//! 3. Calls the orchestrator LLM
//! 4. Parses the structured JSON response
//! 5. Handles parse failures with retry/correction
//! 6. Returns vertical slices with tasks

use std::path::PathBuf;
use std::sync::Arc;

use chrono::Utc;
use sqlx::SqlitePool;
use thiserror::Error;

use crate::llm::{LLMProvider, LLMRequest, Message};
use crate::prompts::schemas::{DecompositionOutput, TierOutput, ValidationError};
use crate::prompts::templates::OrchestratorPrompts;
use crate::types::{AgentTier, Priority, SliceId, Task, TaskId, TaskStatus, Ticket, VerticalSlice};

// ============================================================================
// Slice 5.1.1: Module Structure
// ============================================================================

/// Errors that can occur during decomposition
#[derive(Error, Debug)]
pub enum DecompositionError {
    #[error("LLM call failed: {0}")]
    LlmError(String),

    #[error("failed to parse LLM response: {reason}")]
    ParseError { reason: String, raw_output: String },

    #[error("validation failed: {0}")]
    ValidationError(#[from] ValidationError),

    #[error("max retries exceeded after {attempts} attempts")]
    MaxRetriesExceeded { attempts: u32 },

    #[error("database error: {0}")]
    DatabaseError(String),
}

/// Configuration for the Planner
#[derive(Debug, Clone)]
pub struct PlannerConfig {
    /// Maximum retry attempts for parse failures
    pub max_retries: u32,
    /// Model ID to use for decomposition
    pub model_id: String,
    /// Maximum tokens for the response
    pub max_tokens: u32,
}

impl Default for PlannerConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            model_id: "claude-sonnet-4-20250514".to_string(),
            max_tokens: 8192,
        }
    }
}

/// Result type for decomposition operations
pub type DecompositionResult<T> = Result<T, DecompositionError>;

/// The Planner decomposes tickets into vertical slices.
pub struct Planner<P: LLMProvider> {
    provider: Arc<P>,
    pool: Option<SqlitePool>,
    config: PlannerConfig,
}

// ============================================================================
// Slice 5.1.2: Core Decomposition Method
// ============================================================================

impl<P: LLMProvider> Planner<P> {
    /// Create a new Planner without database persistence.
    pub fn new(provider: Arc<P>, config: PlannerConfig) -> Self {
        Self {
            provider,
            pool: None,
            config,
        }
    }

    /// Create a new Planner with database persistence.
    pub fn with_db(provider: Arc<P>, pool: SqlitePool, config: PlannerConfig) -> Self {
        Self {
            provider,
            pool: Some(pool),
            config,
        }
    }

    /// Decompose a ticket into vertical slices.
    ///
    /// This is the main entry point. It handles retries internally.
    pub async fn decompose(&self, ticket: &Ticket) -> DecompositionResult<PlannerOutput> {
        let mut last_error: Option<DecompositionError> = None;

        for attempt in 1..=self.config.max_retries {
            let request = if attempt == 1 {
                self.build_decomposition_request(ticket)
            } else {
                self.build_correction_request(ticket, last_error.as_ref().unwrap())
            };

            let response = match self.provider.send_message(request).await {
                Ok(r) => r,
                Err(e) => {
                    last_error = Some(DecompositionError::LlmError(e.to_string()));
                    tracing::warn!(
                        attempt = attempt,
                        error = %e,
                        "LLM call failed, retrying"
                    );
                    continue;
                }
            };

            match self.parse_and_validate(&response.content) {
                Ok(output) => {
                    let planner_output = self.convert_to_planner_output(ticket, output);
                    return Ok(planner_output);
                }
                Err(e) => {
                    tracing::warn!(
                        attempt = attempt,
                        error = %e,
                        "Decomposition parse/validation failed, retrying"
                    );
                    last_error = Some(e);
                }
            }
        }

        Err(DecompositionError::MaxRetriesExceeded {
            attempts: self.config.max_retries,
        })
    }

    /// Decompose and persist to database.
    pub async fn decompose_and_save(&self, ticket: &Ticket) -> DecompositionResult<PlannerOutput> {
        let output = self.decompose(ticket).await?;

        if self.pool.is_some() {
            self.save_output(&output).await?;
        }

        Ok(output)
    }

    fn build_decomposition_request(&self, ticket: &Ticket) -> LLMRequest {
        // Use the existing OrchestratorPrompts from M4
        let prompt_builder = OrchestratorPrompts::decomposition(
            &ticket.title,
            &ticket.description,
            None, // Could add codebase context here
            None, // Could add conventions here
        );

        let built_prompt = prompt_builder.build();

        LLMRequest::new(
            &self.config.model_id,
            vec![Message::user(&built_prompt.text)],
        )
        .with_max_tokens(self.config.max_tokens)
    }

    // ============================================================================
    // Slice 5.1.4: Retry with Correction Prompt
    // ============================================================================

    fn build_correction_request(&self, ticket: &Ticket, error: &DecompositionError) -> LLMRequest {
        let error_context = match error {
            DecompositionError::ParseError { reason, raw_output } => {
                format!(
                    "Your previous output couldn't be parsed.\n\
                     Error: {}\n\
                     Your output (truncated): {}\n\n\
                     Please regenerate with valid JSON matching the schema.",
                    reason,
                    &raw_output[..raw_output.len().min(500)]
                )
            }
            DecompositionError::ValidationError(v) => {
                format!(
                    "Your previous output failed validation.\n\
                     Error: {}\n\n\
                     Please fix the issue and regenerate.",
                    v
                )
            }
            _ => format!("Previous attempt failed: {}", error),
        };

        // Build correction prompt
        let prompt_builder =
            OrchestratorPrompts::decomposition(&ticket.title, &ticket.description, None, None);

        let built_prompt = prompt_builder.build();

        // Prepend correction context
        let full_prompt = format!(
            "**CORRECTION NEEDED**\n\n{}\n\n---\n\n{}",
            error_context, built_prompt.text
        );

        LLMRequest::new(&self.config.model_id, vec![Message::user(&full_prompt)])
            .with_max_tokens(self.config.max_tokens)
    }

    // ============================================================================
    // Slice 5.1.3: Parse LLM Response
    // ============================================================================

    fn parse_and_validate(&self, content: &str) -> DecompositionResult<DecompositionOutput> {
        // Extract JSON from response
        let json_str = self.extract_json(content)?;

        // Parse JSON into DecompositionOutput
        let output: DecompositionOutput =
            serde_json::from_str(&json_str).map_err(|e| DecompositionError::ParseError {
                reason: e.to_string(),
                raw_output: content.to_string(),
            })?;

        // Validate using M4's validation logic
        output.validate()?;

        Ok(output)
    }

    fn extract_json(&self, content: &str) -> DecompositionResult<String> {
        // Try to find JSON in markdown code blocks first
        if let Some(start) = content.find("```json") {
            let json_start = start + 7; // len("```json")
            if let Some(end_offset) = content[json_start..].find("```") {
                return Ok(content[json_start..json_start + end_offset]
                    .trim()
                    .to_string());
            }
        }

        // Try generic code block
        if let Some(start) = content.find("```") {
            let code_start = start + 3;
            // Skip optional language identifier on same line
            let line_end = content[code_start..]
                .find('\n')
                .unwrap_or(content.len() - code_start);
            let actual_start = code_start + line_end + 1;

            if let Some(end_offset) = content[actual_start..].find("```") {
                let json_content = content[actual_start..actual_start + end_offset].trim();
                if json_content.starts_with('{') {
                    return Ok(json_content.to_string());
                }
            }
        }

        // Try to find raw JSON object
        if let Some(start) = content.find('{') {
            if let Some(end) = content.rfind('}') {
                return Ok(content[start..=end].to_string());
            }
        }

        Err(DecompositionError::ParseError {
            reason: "No JSON found in response".to_string(),
            raw_output: content.to_string(),
        })
    }

    fn convert_to_planner_output(
        &self,
        ticket: &Ticket,
        output: DecompositionOutput,
    ) -> PlannerOutput {
        let now = Utc::now();
        let mut slices = Vec::new();
        let mut all_tasks = Vec::new();

        for slice_output in output.slices {
            let slice_id = SliceId::new();
            let mut task_ids = Vec::new();

            // Create tasks for this slice
            for task_output in slice_output.tasks {
                let task_id = TaskId::new();
                task_ids.push(task_id.clone());

                let task = Task {
                    id: task_id,
                    slice_id: Some(slice_id.clone()),
                    title: task_output.title,
                    description: String::new(), // Could be enhanced
                    assigned_tier: match task_output.tier {
                        TierOutput::Worker => AgentTier::Worker,
                        TierOutput::Utility => AgentTier::Utility,
                    },
                    assigned_agent: None,
                    status: TaskStatus::Pending,
                    priority: Priority::Normal,
                    context_files: task_output
                        .context_files
                        .into_iter()
                        .map(PathBuf::from)
                        .collect(),
                    created_at: now,
                    updated_at: now,
                };
                all_tasks.push(task);
            }

            let slice = VerticalSlice {
                id: slice_id,
                ticket_id: ticket.id.0,
                title: slice_output.title,
                description: slice_output.description,
                tasks: task_ids,
                status: TaskStatus::Pending,
                created_at: now,
            };
            slices.push(slice);
        }

        PlannerOutput {
            ticket_id: ticket.id.clone(),
            slices,
            tasks: all_tasks,
            questions: output.questions,
            risks: output.risks,
            thinking: output.thinking,
        }
    }

    // ============================================================================
    // Slice 5.1.5: Persist to Database
    // ============================================================================

    async fn save_output(&self, output: &PlannerOutput) -> DecompositionResult<()> {
        let pool = self
            .pool
            .as_ref()
            .ok_or_else(|| DecompositionError::DatabaseError("No database pool".to_string()))?;

        let mut tx = pool
            .begin()
            .await
            .map_err(|e| DecompositionError::DatabaseError(e.to_string()))?;

        // Insert slices
        for slice in &output.slices {
            sqlx::query(
                r#"
                INSERT INTO vertical_slices (id, ticket_id, title, description, status, created_at)
                VALUES (?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(slice.id.0.to_string())
            .bind(slice.ticket_id.to_string())
            .bind(&slice.title)
            .bind(&slice.description)
            .bind("pending")
            .bind(slice.created_at.to_rfc3339())
            .execute(&mut *tx)
            .await
            .map_err(|e| DecompositionError::DatabaseError(e.to_string()))?;
        }

        // Insert tasks
        for task in &output.tasks {
            let slice_id = task.slice_id.as_ref().map(|s| s.0.to_string());

            sqlx::query(
                r#"
                INSERT INTO tasks (id, slice_id, title, description, assigned_tier, status, priority, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(task.id.0.to_string())
            .bind(slice_id)
            .bind(&task.title)
            .bind(&task.description)
            .bind(format!("{:?}", task.assigned_tier))
            .bind("pending")
            .bind(format!("{:?}", task.priority))
            .bind(task.created_at.to_rfc3339())
            .bind(task.updated_at.to_rfc3339())
            .execute(&mut *tx)
            .await
            .map_err(|e| DecompositionError::DatabaseError(e.to_string()))?;
        }

        tx.commit()
            .await
            .map_err(|e| DecompositionError::DatabaseError(e.to_string()))?;

        tracing::info!(
            ticket_id = %output.ticket_id.0,
            slices = output.slices.len(),
            tasks = output.tasks.len(),
            "Decomposition saved to database"
        );

        Ok(())
    }
}

/// Output from the Planner
#[derive(Debug, Clone)]
pub struct PlannerOutput {
    /// The ticket that was decomposed
    pub ticket_id: crate::types::TicketId,
    /// Vertical slices produced
    pub slices: Vec<VerticalSlice>,
    /// Tasks within all slices
    pub tasks: Vec<Task>,
    /// Clarifying questions (if any)
    pub questions: Vec<String>,
    /// Identified risks
    pub risks: Vec<String>,
    /// LLM's reasoning
    pub thinking: String,
}

impl PlannerOutput {
    /// Check if the LLM had questions instead of producing slices.
    pub fn has_questions(&self) -> bool {
        !self.questions.is_empty()
    }

    /// Check if any risks were identified.
    pub fn has_risks(&self) -> bool {
        !self.risks.is_empty()
    }

    /// Get total task count.
    pub fn task_count(&self) -> usize {
        self.tasks.len()
    }

    /// Get slice count.
    pub fn slice_count(&self) -> usize {
        self.slices.len()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::{LLMResponse, LLMResult, StopReason, TokenUsage};
    use async_trait::async_trait;
    use futures::Stream;
    use std::pin::Pin;

    /// Mock LLM provider for testing
    struct MockProvider {
        response: String,
        should_fail: bool,
    }

    impl MockProvider {
        fn with_response(response: &str) -> Self {
            Self {
                response: response.to_string(),
                should_fail: false,
            }
        }

        fn failing() -> Self {
            Self {
                response: String::new(),
                should_fail: true,
            }
        }
    }

    #[async_trait]
    impl LLMProvider for MockProvider {
        async fn send_message(&self, _request: LLMRequest) -> LLMResult<LLMResponse> {
            if self.should_fail {
                return Err(crate::llm::LLMError::ApiError {
                    status: 500,
                    message: "Mock failure".to_string(),
                });
            }

            Ok(LLMResponse {
                content: self.response.clone(),
                model: "mock".to_string(),
                stop_reason: StopReason::EndTurn,
                usage: TokenUsage {
                    input_tokens: 100,
                    output_tokens: 200,
                },
            })
        }

        async fn send_message_stream(
            &self,
            _request: LLMRequest,
        ) -> LLMResult<Pin<Box<dyn Stream<Item = LLMResult<crate::llm::StreamChunk>> + Send>>>
        {
            unimplemented!("Not needed for planner tests")
        }

        fn provider_name(&self) -> &'static str {
            "mock"
        }

        fn model_id(&self) -> &str {
            "mock-model"
        }
    }

    fn create_test_ticket() -> Ticket {
        Ticket {
            id: crate::types::TicketId::new(),
            source: crate::types::TicketSource::Manual,
            title: "Add user authentication".to_string(),
            description: "Implement login and registration for users".to_string(),
            labels: vec![],
            slices: vec![],
            status: crate::types::TicketStatus::New,
            created_at: Utc::now(),
        }
    }

    fn valid_decomposition_json() -> &'static str {
        r#"{
            "thinking": "This requires login and registration functionality...",
            "slices": [
                {
                    "title": "User model and database",
                    "description": "Create user table and model",
                    "tasks": [
                        {
                            "title": "Create user migration",
                            "tier": "worker",
                            "estimated_complexity": "low",
                            "context_files": ["migrations/"]
                        }
                    ],
                    "dependencies": [],
                    "acceptance_criteria": ["User table exists", "Can create users"]
                }
            ],
            "questions": [],
            "risks": ["Password hashing complexity"]
        }"#
    }

    #[test]
    fn test_planner_config_default() {
        let config = PlannerConfig::default();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.max_tokens, 8192);
    }

    #[test]
    fn test_extract_json_from_code_block() {
        let provider = Arc::new(MockProvider::with_response(""));
        let planner = Planner::new(provider, PlannerConfig::default());

        let content = r#"Here's the decomposition:

```json
{"thinking": "test", "slices": []}
```"#;

        let result = planner.extract_json(content);
        assert!(result.is_ok());
        assert!(result.unwrap().contains("thinking"));
    }

    #[test]
    fn test_extract_json_raw() {
        let provider = Arc::new(MockProvider::with_response(""));
        let planner = Planner::new(provider, PlannerConfig::default());

        let content = r#"{"thinking": "test", "slices": []}"#;

        let result = planner.extract_json(content);
        assert!(result.is_ok());
    }

    #[test]
    fn test_extract_json_no_json() {
        let provider = Arc::new(MockProvider::with_response(""));
        let planner = Planner::new(provider, PlannerConfig::default());

        let content = "No JSON here, just text.";

        let result = planner.extract_json(content);
        assert!(matches!(result, Err(DecompositionError::ParseError { .. })));
    }

    #[test]
    fn test_parse_and_validate_valid() {
        let provider = Arc::new(MockProvider::with_response(""));
        let planner = Planner::new(provider, PlannerConfig::default());

        let result = planner.parse_and_validate(valid_decomposition_json());
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(!output.thinking.is_empty());
        assert_eq!(output.slices.len(), 1);
        assert_eq!(output.slices[0].tasks.len(), 1);
    }

    #[test]
    fn test_parse_and_validate_invalid_json() {
        let provider = Arc::new(MockProvider::with_response(""));
        let planner = Planner::new(provider, PlannerConfig::default());

        let result = planner.parse_and_validate("not json");
        assert!(matches!(result, Err(DecompositionError::ParseError { .. })));
    }

    #[test]
    fn test_parse_and_validate_missing_thinking() {
        let provider = Arc::new(MockProvider::with_response(""));
        let planner = Planner::new(provider, PlannerConfig::default());

        let json = r#"{"thinking": "", "slices": [], "questions": ["What?"]}"#;
        let result = planner.parse_and_validate(json);
        assert!(matches!(
            result,
            Err(DecompositionError::ValidationError(_))
        ));
    }

    #[test]
    fn test_convert_to_planner_output() {
        let provider = Arc::new(MockProvider::with_response(""));
        let planner = Planner::new(provider, PlannerConfig::default());

        let ticket = create_test_ticket();
        let decomp: DecompositionOutput = serde_json::from_str(valid_decomposition_json()).unwrap();

        let output = planner.convert_to_planner_output(&ticket, decomp);

        assert_eq!(output.slice_count(), 1);
        assert_eq!(output.task_count(), 1);
        assert!(output.has_risks());
        assert!(!output.has_questions());
    }

    #[tokio::test]
    async fn test_decompose_success() {
        let provider = Arc::new(MockProvider::with_response(valid_decomposition_json()));
        let planner = Planner::new(provider, PlannerConfig::default());

        let ticket = create_test_ticket();
        let result = planner.decompose(&ticket).await;

        assert!(result.is_ok());
        let output = result.unwrap();
        assert_eq!(output.slice_count(), 1);
    }

    #[tokio::test]
    async fn test_decompose_retries_on_parse_error() {
        // First call returns invalid JSON, second call returns valid
        struct RetryMockProvider {
            call_count: std::sync::atomic::AtomicU32,
        }

        #[async_trait]
        impl LLMProvider for RetryMockProvider {
            async fn send_message(&self, _request: LLMRequest) -> LLMResult<LLMResponse> {
                let count = self
                    .call_count
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

                let content = if count == 0 {
                    "invalid json".to_string()
                } else {
                    valid_decomposition_json().to_string()
                };

                Ok(LLMResponse {
                    content,
                    model: "mock".to_string(),
                    stop_reason: StopReason::EndTurn,
                    usage: TokenUsage {
                        input_tokens: 100,
                        output_tokens: 200,
                    },
                })
            }

            async fn send_message_stream(
                &self,
                _request: LLMRequest,
            ) -> LLMResult<Pin<Box<dyn Stream<Item = LLMResult<crate::llm::StreamChunk>> + Send>>>
            {
                unimplemented!()
            }

            fn provider_name(&self) -> &'static str {
                "retry-mock"
            }

            fn model_id(&self) -> &str {
                "mock"
            }
        }

        let provider = Arc::new(RetryMockProvider {
            call_count: std::sync::atomic::AtomicU32::new(0),
        });
        let planner = Planner::new(provider.clone(), PlannerConfig::default());

        let ticket = create_test_ticket();
        let result = planner.decompose(&ticket).await;

        assert!(result.is_ok());
        assert_eq!(
            provider
                .call_count
                .load(std::sync::atomic::Ordering::SeqCst),
            2
        );
    }

    #[tokio::test]
    async fn test_decompose_max_retries_exceeded() {
        let provider = Arc::new(MockProvider::failing());
        let config = PlannerConfig {
            max_retries: 2,
            ..Default::default()
        };
        let planner = Planner::new(provider, config);

        let ticket = create_test_ticket();
        let result = planner.decompose(&ticket).await;

        assert!(matches!(
            result,
            Err(DecompositionError::MaxRetriesExceeded { attempts: 2 })
        ));
    }

    #[test]
    fn test_planner_output_helpers() {
        let output = PlannerOutput {
            ticket_id: crate::types::TicketId::new(),
            slices: vec![],
            tasks: vec![],
            questions: vec!["What is X?".to_string()],
            risks: vec![],
            thinking: "Thought about it".to_string(),
        };

        assert!(output.has_questions());
        assert!(!output.has_risks());
        assert_eq!(output.slice_count(), 0);
        assert_eq!(output.task_count(), 0);
    }

    #[test]
    fn test_decomposition_error_display() {
        let err = DecompositionError::LlmError("Connection failed".to_string());
        assert!(err.to_string().contains("Connection failed"));

        let err = DecompositionError::ParseError {
            reason: "Invalid JSON".to_string(),
            raw_output: "bad".to_string(),
        };
        assert!(err.to_string().contains("Invalid JSON"));

        let err = DecompositionError::MaxRetriesExceeded { attempts: 3 };
        assert!(err.to_string().contains("3 attempts"));
    }
}
