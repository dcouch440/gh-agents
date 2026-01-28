//! Inter-agent communication protocol with serialization and validation

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tracing::warn;
use uuid::Uuid;

use super::agent::AgentId;
use super::roles::RoleId;
use crate::types::{AgentTier, TaskStatus};

/// Protocol version for compatibility checking
pub const PROTOCOL_VERSION: &str = "1.0";

// =============================================================================
// Slice 3.7.1: TaskAssignment Message Format
// =============================================================================

/// Task assignment message from orchestrator to worker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskAssignment {
    /// Protocol version
    pub version: String,
    /// Unique task identifier
    pub task_id: Uuid,
    /// Short task title
    pub title: String,
    /// Detailed task description
    pub description: String,
    /// Context for the task
    pub context: TaskContext,
    /// Constraints on execution
    pub constraints: TaskConstraints,
    /// Maximum time to complete (in seconds)
    pub timeout_secs: u64,
    /// When this assignment was created
    pub created_at: DateTime<Utc>,
    /// Target agent tier (informational)
    pub target_tier: AgentTier,
    /// Role the agent should assume for this task
    pub role_id: RoleId,
    /// Delegation context (tracks hierarchy depth and parent)
    pub delegation: DelegationContext,
}

impl TaskAssignment {
    /// Create a new task assignment
    pub fn new(
        task_id: Uuid,
        title: impl Into<String>,
        description: impl Into<String>,
        target_tier: AgentTier,
    ) -> Self {
        Self {
            version: PROTOCOL_VERSION.to_string(),
            task_id,
            title: title.into(),
            description: description.into(),
            context: TaskContext::default(),
            constraints: TaskConstraints::default(),
            timeout_secs: 300, // 5 minute default
            created_at: Utc::now(),
            target_tier,
            role_id: RoleId::new("worker"),
            delegation: DelegationContext::default(),
        }
    }

    /// Get timeout as Duration
    pub fn timeout(&self) -> Duration {
        Duration::from_secs(self.timeout_secs)
    }

    /// Builder-style: add context
    pub fn with_context(mut self, context: TaskContext) -> Self {
        self.context = context;
        self
    }

    /// Builder-style: add constraints
    pub fn with_constraints(mut self, constraints: TaskConstraints) -> Self {
        self.constraints = constraints;
        self
    }

    /// Builder-style: set timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout_secs = timeout.as_secs();
        self
    }

    /// Builder-style: set role
    pub fn with_role(mut self, role_id: RoleId) -> Self {
        self.role_id = role_id;
        self
    }

    /// Builder-style: set delegation context
    pub fn with_delegation(mut self, delegation: DelegationContext) -> Self {
        self.delegation = delegation;
        self
    }
}

/// Context provided with a task assignment
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskContext {
    /// Pre-loaded file contents
    pub files: Vec<FileContent>,
    /// Relevant prior work history
    pub history: Vec<HistoryEntry>,
    /// Project conventions (e.g., from CLAUDE.md)
    pub conventions: String,
    /// Additional metadata
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

/// File content for context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileContent {
    /// Relative path from project root
    pub path: String,
    /// File contents
    pub content: String,
    /// Optional: line range if partial file
    pub line_range: Option<(usize, usize)>,
}

impl FileContent {
    pub fn new(path: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            content: content.into(),
            line_range: None,
        }
    }

    pub fn with_range(mut self, start: usize, end: usize) -> Self {
        self.line_range = Some((start, end));
        self
    }
}

/// History entry for context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    /// Related task ID
    pub task_id: Uuid,
    /// Brief summary of what was done
    pub summary: String,
    /// When this occurred
    pub timestamp: DateTime<Utc>,
}

impl HistoryEntry {
    pub fn new(task_id: Uuid, summary: impl Into<String>) -> Self {
        Self {
            task_id,
            summary: summary.into(),
            timestamp: Utc::now(),
        }
    }
}

/// Constraints on task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskConstraints {
    /// Maximum number of files the agent may modify
    pub max_files_modified: Option<u32>,
    /// Glob patterns for allowed file paths
    pub allowed_paths: Vec<String>,
    /// Whether tests are required
    pub require_tests: bool,
    /// Whether review is required before completion
    pub require_review: bool,
    /// Additional constraints as key-value pairs
    #[serde(default)]
    pub extra: HashMap<String, String>,
}

impl Default for TaskConstraints {
    fn default() -> Self {
        Self {
            max_files_modified: None,
            allowed_paths: vec!["**/*".to_string()], // Allow all by default
            require_tests: false,
            require_review: true, // Review by default
            extra: HashMap::new(),
        }
    }
}

/// Delegation context for tracking hierarchy and permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegationContext {
    /// Current delegation depth (0 = user-initiated, 1 = primary agent, 2 = sub-agent)
    pub depth: u8,
    /// Maximum allowed depth (from role's max_delegation_depth)
    pub max_depth: u8,
    /// Parent agent that delegated this task (None if from user)
    pub parent_agent: Option<AgentId>,
    /// Role of the parent agent (for permission checking)
    pub parent_role: Option<RoleId>,
    /// Chain of delegation for traceability
    pub delegation_chain: Vec<DelegationHop>,
}

impl Default for DelegationContext {
    fn default() -> Self {
        Self {
            depth: 0,
            max_depth: 2, // Default: user -> agent -> sub-agent
            parent_agent: None,
            parent_role: None,
            delegation_chain: vec![],
        }
    }
}

impl DelegationContext {
    /// Create context for user-initiated task
    pub fn from_user() -> Self {
        Self::default()
    }

    /// Create context for delegated task
    pub fn delegated_from(
        parent_agent: AgentId,
        parent_role: RoleId,
        current_context: &DelegationContext,
    ) -> Self {
        let mut chain = current_context.delegation_chain.clone();
        chain.push(DelegationHop {
            agent_id: parent_agent.clone(),
            role_id: parent_role.clone(),
            timestamp: Utc::now(),
        });

        Self {
            depth: current_context.depth + 1,
            max_depth: current_context.max_depth,
            parent_agent: Some(parent_agent),
            parent_role: Some(parent_role),
            delegation_chain: chain,
        }
    }

    /// Check if further delegation is allowed
    pub fn can_delegate(&self) -> bool {
        self.depth < self.max_depth
    }
}

/// Record of a delegation hop for traceability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegationHop {
    pub agent_id: AgentId,
    pub role_id: RoleId,
    pub timestamp: DateTime<Utc>,
}

// =============================================================================
// Slice 3.7.2: TaskResult Message Format
// =============================================================================

/// Result of task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    /// Protocol version
    pub version: String,
    /// Task ID this result is for
    pub task_id: Uuid,
    /// Final task status
    pub status: TaskStatus,
    /// Human-readable output/summary
    pub output: String,
    /// Files that were modified
    pub files_modified: Vec<FileModification>,
    /// Errors encountered (empty for success)
    pub errors: Vec<TaskError>,
    /// When execution completed
    pub completed_at: DateTime<Utc>,
    /// Execution duration in milliseconds
    pub duration_ms: u64,
    /// Token usage for cost tracking
    pub token_usage: Option<TokenUsage>,
    /// Structured output (for programmatic use)
    pub structured_output: Option<serde_json::Value>,
}

impl TaskResult {
    /// Create a successful result
    pub fn success(task_id: Uuid, output: impl Into<String>) -> Self {
        Self {
            version: PROTOCOL_VERSION.to_string(),
            task_id,
            status: TaskStatus::Completed,
            output: output.into(),
            files_modified: vec![],
            errors: vec![],
            completed_at: Utc::now(),
            duration_ms: 0,
            token_usage: None,
            structured_output: None,
        }
    }

    /// Create a failed result
    pub fn failure(task_id: Uuid, error: impl Into<String>) -> Self {
        Self {
            version: PROTOCOL_VERSION.to_string(),
            task_id,
            status: TaskStatus::Failed,
            output: String::new(),
            files_modified: vec![],
            errors: vec![TaskError::new("execution_failed", error)],
            completed_at: Utc::now(),
            duration_ms: 0,
            token_usage: None,
            structured_output: None,
        }
    }

    /// Check if the result indicates success
    pub fn is_success(&self) -> bool {
        matches!(self.status, TaskStatus::Completed)
    }

    /// Add a file modification
    pub fn with_file_modified(mut self, modification: FileModification) -> Self {
        self.files_modified.push(modification);
        self
    }

    /// Add an error
    pub fn with_error(mut self, error: TaskError) -> Self {
        self.errors.push(error);
        self
    }

    /// Set token usage
    pub fn with_token_usage(mut self, usage: TokenUsage) -> Self {
        self.token_usage = Some(usage);
        self
    }

    /// Set duration
    pub fn with_duration_ms(mut self, duration_ms: u64) -> Self {
        self.duration_ms = duration_ms;
        self
    }

    /// Set structured output
    pub fn with_structured_output(mut self, output: serde_json::Value) -> Self {
        self.structured_output = Some(output);
        self
    }
}

/// Record of a file modification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileModification {
    /// Path to the file
    pub path: String,
    /// Type of modification
    pub modification_type: ModificationType,
    /// Lines added
    pub lines_added: u32,
    /// Lines removed
    pub lines_removed: u32,
}

impl FileModification {
    pub fn created(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            modification_type: ModificationType::Created,
            lines_added: 0,
            lines_removed: 0,
        }
    }

    pub fn modified(path: impl Into<String>, added: u32, removed: u32) -> Self {
        Self {
            path: path.into(),
            modification_type: ModificationType::Modified,
            lines_added: added,
            lines_removed: removed,
        }
    }

    pub fn deleted(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            modification_type: ModificationType::Deleted,
            lines_added: 0,
            lines_removed: 0,
        }
    }
}

/// Type of file modification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ModificationType {
    Created,
    Modified,
    Deleted,
    Renamed { from: String },
}

/// Task error with structured information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskError {
    /// Error code/category
    pub code: String,
    /// Human-readable message
    pub message: String,
    /// Optional details
    pub details: Option<String>,
    /// Whether this error is recoverable
    pub recoverable: bool,
}

impl TaskError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: None,
            recoverable: true,
        }
    }

    pub fn unrecoverable(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: None,
            recoverable: false,
        }
    }

    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }
}

/// Token usage for cost tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub model_id: String,
}

impl TokenUsage {
    pub fn new(model_id: impl Into<String>, input: u32, output: u32) -> Self {
        Self {
            model_id: model_id.into(),
            input_tokens: input,
            output_tokens: output,
        }
    }

    pub fn total(&self) -> u32 {
        self.input_tokens + self.output_tokens
    }
}

// =============================================================================
// Slice 3.7.3: ContextRequest/ContextResponse
// =============================================================================

/// Request for additional context from an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextRequest {
    /// Protocol version
    pub version: String,
    /// Unique request ID for correlation
    pub request_id: Uuid,
    /// Task ID this request is for
    pub task_id: Uuid,
    /// Files the agent needs to see
    pub files_needed: Vec<FileRequest>,
    /// Questions the agent has
    pub questions: Vec<Question>,
    /// When the request was made
    pub requested_at: DateTime<Utc>,
    /// Priority (affects how quickly it should be handled)
    pub priority: RequestPriority,
}

impl ContextRequest {
    pub fn new(task_id: Uuid) -> Self {
        Self {
            version: PROTOCOL_VERSION.to_string(),
            request_id: Uuid::new_v4(),
            task_id,
            files_needed: vec![],
            questions: vec![],
            requested_at: Utc::now(),
            priority: RequestPriority::Normal,
        }
    }

    pub fn with_file(mut self, path: impl Into<String>) -> Self {
        self.files_needed.push(FileRequest::full(path));
        self
    }

    pub fn with_file_range(mut self, path: impl Into<String>, start: usize, end: usize) -> Self {
        self.files_needed.push(FileRequest::range(path, start, end));
        self
    }

    pub fn with_question(mut self, question: impl Into<String>) -> Self {
        self.questions.push(Question::new(question));
        self
    }

    pub fn with_priority(mut self, priority: RequestPriority) -> Self {
        self.priority = priority;
        self
    }
}

/// File request specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRequest {
    /// Path to the file
    pub path: String,
    /// Optional line range (start, end inclusive)
    pub line_range: Option<(usize, usize)>,
    /// Why this file is needed
    pub reason: Option<String>,
}

impl FileRequest {
    pub fn full(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            line_range: None,
            reason: None,
        }
    }

    pub fn range(path: impl Into<String>, start: usize, end: usize) -> Self {
        Self {
            path: path.into(),
            line_range: Some((start, end)),
            reason: None,
        }
    }

    pub fn with_reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }
}

/// Question from agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Question {
    /// The question text
    pub text: String,
    /// Category of question
    pub category: QuestionCategory,
    /// Whether an answer is required
    pub required: bool,
}

impl Question {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            category: QuestionCategory::Clarification,
            required: true,
        }
    }

    pub fn optional(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            category: QuestionCategory::Clarification,
            required: false,
        }
    }

    pub fn with_category(mut self, category: QuestionCategory) -> Self {
        self.category = category;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum QuestionCategory {
    /// Need clarification on requirements
    Clarification,
    /// Need decision on approach
    Decision,
    /// Need information about architecture
    Architecture,
    /// Other
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RequestPriority {
    Low,
    Normal,
    High,
    Urgent,
}

/// Response to a context request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextResponse {
    /// Protocol version
    pub version: String,
    /// Request ID this responds to
    pub request_id: Uuid,
    /// Task ID for reference
    pub task_id: Uuid,
    /// File contents provided
    pub files: Vec<FileContent>,
    /// Answers to questions
    pub answers: Vec<Answer>,
    /// When the response was created
    pub responded_at: DateTime<Utc>,
    /// Any files that couldn't be provided
    pub unavailable_files: Vec<UnavailableFile>,
}

impl ContextResponse {
    pub fn new(request_id: Uuid, task_id: Uuid) -> Self {
        Self {
            version: PROTOCOL_VERSION.to_string(),
            request_id,
            task_id,
            files: vec![],
            answers: vec![],
            responded_at: Utc::now(),
            unavailable_files: vec![],
        }
    }

    pub fn with_file(mut self, content: FileContent) -> Self {
        self.files.push(content);
        self
    }

    pub fn with_answer(mut self, answer: Answer) -> Self {
        self.answers.push(answer);
        self
    }

    pub fn with_unavailable_file(mut self, file: UnavailableFile) -> Self {
        self.unavailable_files.push(file);
        self
    }
}

/// Answer to a question
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Answer {
    /// Original question text
    pub question: String,
    /// The answer
    pub answer: String,
    /// Who provided the answer
    pub source: AnswerSource,
}

impl Answer {
    pub fn from_user(question: impl Into<String>, answer: impl Into<String>) -> Self {
        Self {
            question: question.into(),
            answer: answer.into(),
            source: AnswerSource::User,
        }
    }

    pub fn from_orchestrator(question: impl Into<String>, answer: impl Into<String>) -> Self {
        Self {
            question: question.into(),
            answer: answer.into(),
            source: AnswerSource::Orchestrator,
        }
    }

    pub fn from_system(question: impl Into<String>, answer: impl Into<String>) -> Self {
        Self {
            question: question.into(),
            answer: answer.into(),
            source: AnswerSource::System,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AnswerSource {
    User,
    Orchestrator,
    System,
}

/// File that couldn't be provided
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnavailableFile {
    pub path: String,
    pub reason: String,
}

impl UnavailableFile {
    pub fn not_found(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            reason: "File not found".to_string(),
        }
    }

    pub fn permission_denied(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            reason: "Permission denied".to_string(),
        }
    }

    pub fn too_large(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            reason: "File too large".to_string(),
        }
    }
}

// =============================================================================
// Slice 3.7.4: ProgressUpdate for Feed
// =============================================================================

/// Progress update for the activity feed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressUpdate {
    /// Protocol version
    pub version: String,
    /// Task ID this update is for
    pub task_id: Uuid,
    /// Human-readable message
    pub message: String,
    /// Progress percentage (0-100), if determinable
    pub progress_percent: Option<u8>,
    /// Type of activity
    pub activity: ActivityType,
    /// When this update occurred
    pub timestamp: DateTime<Utc>,
    /// Verbosity level (for filtering)
    pub verbosity: VerbosityLevel,
    /// Optional details for verbose mode
    pub details: Option<String>,
}

impl ProgressUpdate {
    pub fn new(task_id: Uuid, message: impl Into<String>) -> Self {
        Self {
            version: PROTOCOL_VERSION.to_string(),
            task_id,
            message: message.into(),
            progress_percent: None,
            activity: ActivityType::Working,
            timestamp: Utc::now(),
            verbosity: VerbosityLevel::Normal,
            details: None,
        }
    }

    pub fn thinking(task_id: Uuid, message: impl Into<String>) -> Self {
        Self::new(task_id, message).with_activity(ActivityType::Thinking)
    }

    pub fn coding(task_id: Uuid, message: impl Into<String>) -> Self {
        Self::new(task_id, message).with_activity(ActivityType::Coding)
    }

    pub fn reviewing(task_id: Uuid, message: impl Into<String>) -> Self {
        Self::new(task_id, message).with_activity(ActivityType::Reviewing)
    }

    pub fn testing(task_id: Uuid, message: impl Into<String>) -> Self {
        Self::new(task_id, message).with_activity(ActivityType::Testing)
    }

    pub fn milestone(task_id: Uuid, message: impl Into<String>) -> Self {
        Self::new(task_id, message)
            .with_activity(ActivityType::Milestone)
            .with_verbosity(VerbosityLevel::Quiet) // Always show milestones
    }

    pub fn error(task_id: Uuid, message: impl Into<String>) -> Self {
        Self::new(task_id, message)
            .with_activity(ActivityType::Error)
            .with_verbosity(VerbosityLevel::Quiet) // Always show errors
    }

    pub fn with_progress(mut self, percent: u8) -> Self {
        self.progress_percent = Some(percent.min(100));
        self
    }

    pub fn with_activity(mut self, activity: ActivityType) -> Self {
        self.activity = activity;
        self
    }

    pub fn with_verbosity(mut self, verbosity: VerbosityLevel) -> Self {
        self.verbosity = verbosity;
        self
    }

    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }
}

/// Type of activity being performed
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ActivityType {
    /// General work
    Working,
    /// Analyzing/thinking
    Thinking,
    /// Writing code
    Coding,
    /// Reviewing code
    Reviewing,
    /// Running tests
    Testing,
    /// Waiting for input
    Waiting,
    /// Significant milestone reached
    Milestone,
    /// Error occurred
    Error,
}

impl ActivityType {
    /// Get a display icon for TUI
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Working => "●",
            Self::Thinking => "◐",
            Self::Coding => "◆",
            Self::Reviewing => "◇",
            Self::Testing => "▶",
            Self::Waiting => "○",
            Self::Milestone => "★",
            Self::Error => "✗",
        }
    }

    /// Get a display name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Working => "Working",
            Self::Thinking => "Thinking",
            Self::Coding => "Coding",
            Self::Reviewing => "Reviewing",
            Self::Testing => "Testing",
            Self::Waiting => "Waiting",
            Self::Milestone => "Milestone",
            Self::Error => "Error",
        }
    }
}

/// Verbosity level for filtering
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum VerbosityLevel {
    /// Always shown
    Quiet,
    /// Shown at normal verbosity
    Normal,
    /// Only shown at verbose level
    Verbose,
}

// =============================================================================
// Slice 3.7.5: Message Validation
// =============================================================================

/// Validation error with structured information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    /// Field that failed validation
    pub field: String,
    /// Error code
    pub code: String,
    /// Human-readable message
    pub message: String,
}

impl ValidationError {
    pub fn required(field: &str) -> Self {
        Self {
            field: field.to_string(),
            code: "required".to_string(),
            message: format!("{} is required", field),
        }
    }

    pub fn invalid(field: &str, message: impl Into<String>) -> Self {
        Self {
            field: field.to_string(),
            code: "invalid".to_string(),
            message: message.into(),
        }
    }

    pub fn version_mismatch(expected: &str, got: &str) -> Self {
        Self {
            field: "version".to_string(),
            code: "version_mismatch".to_string(),
            message: format!("expected version {}, got {}", expected, got),
        }
    }
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {} ({})", self.field, self.message, self.code)
    }
}

/// Trait for validatable messages
pub trait Validatable {
    fn validate(&self) -> Result<(), Vec<ValidationError>>;
}

impl Validatable for TaskAssignment {
    fn validate(&self) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();

        // Version check
        if self.version != PROTOCOL_VERSION {
            errors.push(ValidationError::version_mismatch(
                PROTOCOL_VERSION,
                &self.version,
            ));
        }

        // Required fields
        if self.title.trim().is_empty() {
            errors.push(ValidationError::required("title"));
        }

        if self.description.trim().is_empty() {
            errors.push(ValidationError::required("description"));
        }

        // Timeout sanity check
        if self.timeout_secs == 0 {
            errors.push(ValidationError::invalid(
                "timeout_secs",
                "timeout must be greater than 0",
            ));
        }

        if self.timeout_secs > 3600 * 24 {
            errors.push(ValidationError::invalid(
                "timeout_secs",
                "timeout cannot exceed 24 hours",
            ));
        }

        // Delegation depth check
        if self.delegation.depth > self.delegation.max_depth {
            errors.push(ValidationError::invalid(
                "delegation.depth",
                format!(
                    "delegation depth {} exceeds max_depth {}",
                    self.delegation.depth, self.delegation.max_depth
                ),
            ));
        }

        // Context validation
        for (i, file) in self.context.files.iter().enumerate() {
            if file.path.trim().is_empty() {
                errors.push(ValidationError::invalid(
                    &format!("context.files[{}].path", i),
                    "file path cannot be empty",
                ));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            warn!(task_id = ?self.task_id, error_count = errors.len(), "Task assignment validation failed");
            Err(errors)
        }
    }
}

impl Validatable for TaskResult {
    fn validate(&self) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();

        // Version check
        if self.version != PROTOCOL_VERSION {
            errors.push(ValidationError::version_mismatch(
                PROTOCOL_VERSION,
                &self.version,
            ));
        }

        // Failed results should have errors
        if matches!(self.status, TaskStatus::Failed) && self.errors.is_empty() {
            errors.push(ValidationError::invalid(
                "errors",
                "failed status requires at least one error",
            ));
        }

        // File modifications should have valid paths
        for (i, mod_) in self.files_modified.iter().enumerate() {
            if mod_.path.trim().is_empty() {
                errors.push(ValidationError::invalid(
                    &format!("files_modified[{}].path", i),
                    "file path cannot be empty",
                ));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            warn!(task_id = ?self.task_id, error_count = errors.len(), "Task result validation failed");
            Err(errors)
        }
    }
}

impl Validatable for ContextRequest {
    fn validate(&self) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();

        // Version check
        if self.version != PROTOCOL_VERSION {
            errors.push(ValidationError::version_mismatch(
                PROTOCOL_VERSION,
                &self.version,
            ));
        }

        // Must have at least one file or question
        if self.files_needed.is_empty() && self.questions.is_empty() {
            errors.push(ValidationError::invalid(
                "files_needed/questions",
                "context request must have at least one file or question",
            ));
        }

        // Validate file paths
        for (i, file) in self.files_needed.iter().enumerate() {
            if file.path.trim().is_empty() {
                errors.push(ValidationError::invalid(
                    &format!("files_needed[{}].path", i),
                    "file path cannot be empty",
                ));
            }
        }

        // Validate questions
        for (i, question) in self.questions.iter().enumerate() {
            if question.text.trim().is_empty() {
                errors.push(ValidationError::invalid(
                    &format!("questions[{}].text", i),
                    "question text cannot be empty",
                ));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            warn!(request_id = ?self.request_id, error_count = errors.len(), "Context request validation failed");
            Err(errors)
        }
    }
}

impl Validatable for ContextResponse {
    fn validate(&self) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();

        // Version check
        if self.version != PROTOCOL_VERSION {
            errors.push(ValidationError::version_mismatch(
                PROTOCOL_VERSION,
                &self.version,
            ));
        }

        // Validate file paths
        for (i, file) in self.files.iter().enumerate() {
            if file.path.trim().is_empty() {
                errors.push(ValidationError::invalid(
                    &format!("files[{}].path", i),
                    "file path cannot be empty",
                ));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

impl Validatable for ProgressUpdate {
    fn validate(&self) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();

        // Version check
        if self.version != PROTOCOL_VERSION {
            errors.push(ValidationError::version_mismatch(
                PROTOCOL_VERSION,
                &self.version,
            ));
        }

        // Message required
        if self.message.trim().is_empty() {
            errors.push(ValidationError::required("message"));
        }

        // Progress percentage range
        if let Some(percent) = self.progress_percent {
            if percent > 100 {
                errors.push(ValidationError::invalid(
                    "progress_percent",
                    "progress must be 0-100",
                ));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Validate any protocol message
pub fn validate_message<T: Validatable>(message: &T) -> Result<(), Vec<ValidationError>> {
    message.validate()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Slice 3.7.1 tests
    #[test]
    fn task_assignment_serialization() {
        let assignment = TaskAssignment::new(
            Uuid::new_v4(),
            "Implement feature",
            "Add a new button to the UI",
            AgentTier::Worker,
        );

        // Serialize to JSON
        let json = serde_json::to_string(&assignment).unwrap();

        // Deserialize back
        let parsed: TaskAssignment = serde_json::from_str(&json).unwrap();

        assert_eq!(assignment.task_id, parsed.task_id);
        assert_eq!(assignment.title, parsed.title);
        assert_eq!(assignment.version, PROTOCOL_VERSION);
    }

    #[test]
    fn delegation_context_tracks_chain() {
        let user_ctx = DelegationContext::from_user();
        assert_eq!(user_ctx.depth, 0);
        assert!(user_ctx.can_delegate());

        let agent1 = AgentId::new();
        let role1 = RoleId::new("orchestrator");
        let delegated1 =
            DelegationContext::delegated_from(agent1.clone(), role1.clone(), &user_ctx);

        assert_eq!(delegated1.depth, 1);
        assert!(delegated1.can_delegate());
        assert_eq!(delegated1.delegation_chain.len(), 1);

        let agent2 = AgentId::new();
        let role2 = RoleId::new("worker");
        let delegated2 = DelegationContext::delegated_from(agent2, role2, &delegated1);

        assert_eq!(delegated2.depth, 2);
        assert!(!delegated2.can_delegate()); // max_depth = 2, depth = 2
        assert_eq!(delegated2.delegation_chain.len(), 2);
    }

    // Slice 3.7.2 tests
    #[test]
    fn task_result_success_serialization() {
        let result = TaskResult::success(Uuid::new_v4(), "Task completed successfully")
            .with_file_modified(FileModification::modified("src/main.rs", 10, 2));

        let json = serde_json::to_string(&result).unwrap();
        let parsed: TaskResult = serde_json::from_str(&json).unwrap();

        assert!(parsed.is_success());
        assert_eq!(parsed.files_modified.len(), 1);
    }

    #[test]
    fn task_result_failure_serialization() {
        let result = TaskResult::failure(Uuid::new_v4(), "Compilation error");

        let json = serde_json::to_string(&result).unwrap();
        let parsed: TaskResult = serde_json::from_str(&json).unwrap();

        assert!(!parsed.is_success());
        assert_eq!(parsed.errors.len(), 1);
    }

    #[test]
    fn token_usage_total() {
        let usage = TokenUsage::new("claude-3-opus", 1000, 500);
        assert_eq!(usage.total(), 1500);
    }

    // Slice 3.7.3 tests
    #[test]
    fn context_request_response_cycle() {
        let task_id = Uuid::new_v4();

        // Create request
        let request = ContextRequest::new(task_id)
            .with_file("src/main.rs")
            .with_question("What testing framework should I use?");

        let request_json = serde_json::to_string(&request).unwrap();
        let parsed_request: ContextRequest = serde_json::from_str(&request_json).unwrap();

        // Create response
        let response = ContextResponse::new(parsed_request.request_id, task_id)
            .with_file(FileContent::new("src/main.rs", "fn main() {}"))
            .with_answer(Answer::from_orchestrator(
                "What testing framework should I use?",
                "Use the built-in Rust test framework",
            ));

        let response_json = serde_json::to_string(&response).unwrap();
        let parsed_response: ContextResponse = serde_json::from_str(&response_json).unwrap();

        assert_eq!(parsed_request.request_id, parsed_response.request_id);
    }

    #[test]
    fn file_request_with_range() {
        let request =
            FileRequest::range("src/lib.rs", 10, 50).with_reason("Need function definition");

        assert_eq!(request.line_range, Some((10, 50)));
        assert!(request.reason.is_some());
    }

    // Slice 3.7.4 tests
    #[test]
    fn progress_update_serialization() {
        let update = ProgressUpdate::coding(Uuid::new_v4(), "Writing authentication module")
            .with_progress(45)
            .with_details("Adding JWT validation");

        let json = serde_json::to_string(&update).unwrap();
        let parsed: ProgressUpdate = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.activity, ActivityType::Coding);
        assert_eq!(parsed.progress_percent, Some(45));
    }

    #[test]
    fn milestone_always_shown() {
        let update = ProgressUpdate::milestone(Uuid::new_v4(), "Feature complete");

        assert_eq!(update.verbosity, VerbosityLevel::Quiet);
        assert_eq!(update.activity, ActivityType::Milestone);
    }

    #[test]
    fn activity_type_icons() {
        assert_eq!(ActivityType::Coding.icon(), "◆");
        assert_eq!(ActivityType::Error.icon(), "✗");
        assert_eq!(ActivityType::Milestone.icon(), "★");
    }

    // Slice 3.7.5 tests
    #[test]
    fn valid_task_assignment_passes() {
        let assignment = TaskAssignment::new(
            Uuid::new_v4(),
            "Valid title",
            "Valid description",
            AgentTier::Worker,
        );

        assert!(assignment.validate().is_ok());
    }

    #[test]
    fn empty_title_fails_validation() {
        let assignment =
            TaskAssignment::new(Uuid::new_v4(), "", "Valid description", AgentTier::Worker);

        let errors = assignment.validate().unwrap_err();
        assert!(errors.iter().any(|e| e.field == "title"));
    }

    #[test]
    fn zero_timeout_fails_validation() {
        let mut assignment =
            TaskAssignment::new(Uuid::new_v4(), "Title", "Description", AgentTier::Worker);
        assignment.timeout_secs = 0;

        let errors = assignment.validate().unwrap_err();
        assert!(errors.iter().any(|e| e.field == "timeout_secs"));
    }

    #[test]
    fn exceeded_delegation_depth_fails_validation() {
        let mut assignment =
            TaskAssignment::new(Uuid::new_v4(), "Title", "Description", AgentTier::Worker);
        assignment.delegation.depth = 5;
        assignment.delegation.max_depth = 2;

        let errors = assignment.validate().unwrap_err();
        assert!(errors.iter().any(|e| e.field == "delegation.depth"));
    }

    #[test]
    fn failed_result_without_errors_fails_validation() {
        let result = TaskResult {
            version: PROTOCOL_VERSION.to_string(),
            task_id: Uuid::new_v4(),
            status: TaskStatus::Failed,
            output: String::new(),
            files_modified: vec![],
            errors: vec![], // Missing required errors
            completed_at: Utc::now(),
            duration_ms: 0,
            token_usage: None,
            structured_output: None,
        };

        let errors = result.validate().unwrap_err();
        assert!(errors.iter().any(|e| e.field == "errors"));
    }

    #[test]
    fn empty_context_request_fails_validation() {
        let request = ContextRequest::new(Uuid::new_v4());
        // No files or questions

        let errors = request.validate().unwrap_err();
        assert!(errors.iter().any(|e| e.field == "files_needed/questions"));
    }

    #[test]
    fn valid_context_request_passes() {
        let request = ContextRequest::new(Uuid::new_v4()).with_file("src/main.rs");

        assert!(request.validate().is_ok());
    }

    #[test]
    fn empty_progress_message_fails_validation() {
        let update = ProgressUpdate::new(Uuid::new_v4(), "");

        let errors = update.validate().unwrap_err();
        assert!(errors.iter().any(|e| e.field == "message"));
    }

    #[test]
    fn progress_clamped_to_100() {
        let update = ProgressUpdate::new(Uuid::new_v4(), "Working").with_progress(150);

        assert_eq!(update.progress_percent, Some(100));
    }
}
