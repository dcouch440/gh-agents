//! Context window validation for LLM prompts.
//!
//! This module provides:
//! - `TokenCounter` - Token counting with multiple tokenizer types
//! - `ModelLimits` - Context limits for different models
//! - `ContextValidator` - Pre-flight validation before LLM calls
//! - `ContextTruncator` - Automatic truncation to fit limits
//! - `ContextPressureWarning` - Warning system for approaching limits

use std::collections::HashMap;

use super::manager::ContextCategory;
use super::summarizer::FileSummarizer;

// ============================================================================
// Slice 4.11.1: Token Counter
// ============================================================================

/// Token counter for context validation.
///
/// Uses simple heuristics by default. Can be replaced with
/// model-specific tokenizers for accuracy.
pub struct TokenCounter {
    /// Tokenizer type to use
    tokenizer_type: TokenizerType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenizerType {
    /// Simple character-based estimation (~4 chars per token)
    Simple,
    /// Claude-specific tokenizer (more accurate)
    Claude,
    /// GPT-specific tokenizer (BPE-based)
    Gpt,
}

impl Default for TokenCounter {
    fn default() -> Self {
        Self {
            tokenizer_type: TokenizerType::Simple,
        }
    }
}

impl TokenCounter {
    pub fn new(tokenizer_type: TokenizerType) -> Self {
        Self { tokenizer_type }
    }

    /// Count tokens in text.
    pub fn count(&self, text: &str) -> usize {
        match self.tokenizer_type {
            TokenizerType::Simple => self.count_simple(text),
            TokenizerType::Claude => self.count_claude(text),
            TokenizerType::Gpt => self.count_gpt(text),
        }
    }

    /// Simple estimation: ~4 characters per token
    fn count_simple(&self, text: &str) -> usize {
        // Rough estimation that works reasonably well
        // Most tokenizers average 3-5 chars per token
        let char_count = text.chars().count();
        char_count.div_ceil(4) // Round up
    }

    /// Claude-specific counting (placeholder - would use actual tokenizer)
    fn count_claude(&self, text: &str) -> usize {
        // Claude's tokenizer is similar to GPT but with some differences
        // For now, use a slightly adjusted simple count
        // In production, would use anthropic's tokenizer
        let base = self.count_simple(text);

        // Claude tends to be slightly more efficient with common patterns
        (base as f64 * 0.95) as usize
    }

    /// GPT-specific counting (placeholder - would use tiktoken)
    fn count_gpt(&self, text: &str) -> usize {
        // GPT uses BPE tokenizer
        // Would use tiktoken crate in production
        self.count_simple(text)
    }

    /// Count tokens in a prompt with message structure.
    pub fn count_messages(&self, messages: &[MessageForCounting]) -> usize {
        let mut total = 0;

        for msg in messages {
            // Each message has overhead for role and structure
            total += 4; // Approximate overhead per message

            // Role token
            total += 1;

            // Content tokens
            total += self.count(&msg.content);
        }

        // Final assistant response priming
        total += 3;

        total
    }

    /// Estimate output tokens needed for response.
    pub fn estimate_response_tokens(&self, max_response_length: ResponseLength) -> usize {
        match max_response_length {
            ResponseLength::Short => 256,
            ResponseLength::Medium => 1024,
            ResponseLength::Long => 4096,
            ResponseLength::VeryLong => 8192,
            ResponseLength::Custom(n) => n,
        }
    }
}

/// A message for token counting purposes
#[derive(Debug, Clone)]
pub struct MessageForCounting {
    pub role: String,
    pub content: String,
}

/// Expected response length categories
#[derive(Debug, Clone, Copy)]
pub enum ResponseLength {
    Short,    // Brief answers, yes/no
    Medium,   // Typical responses
    Long,     // Detailed explanations
    VeryLong, // Full implementations
    Custom(usize),
}

// ============================================================================
// Slice 4.11.2: Model Context Budgets
// ============================================================================

/// Context limits for different models.
#[derive(Debug, Clone)]
pub struct ModelLimits {
    /// Model identifier
    pub model_id: String,

    /// Maximum context window (input + output)
    pub max_context_tokens: usize,

    /// Maximum output tokens
    pub max_output_tokens: usize,

    /// Recommended safe limit (leave buffer for response)
    pub safe_input_tokens: usize,
}

impl ModelLimits {
    /// Create limits for a known model.
    pub fn for_model(model_id: &str) -> Self {
        match model_id {
            // Anthropic Claude models - exact matches and short names
            "claude-3-opus-20240229" | "claude-opus-4-20250514" | "claude-opus-4-5-20251101" | "claude-opus" => Self::claude_opus(),
            "claude-3-sonnet-20240229" | "claude-sonnet-4-20250514" | "claude-sonnet" | "claude-3-sonnet" => Self::claude_sonnet(),
            "claude-3-haiku-20240307" | "claude-3-5-haiku-20241022" | "claude-haiku" => Self::claude_haiku(),

            // Default/unknown - conservative limits
            _ => Self::default_limits(model_id),
        }
    }

    fn claude_opus() -> Self {
        Self {
            model_id: "claude-3-opus-20240229".to_string(),
            max_context_tokens: 200_000,
            max_output_tokens: 32_768,
            safe_input_tokens: 166_000,
        }
    }

    fn claude_sonnet() -> Self {
        Self {
            model_id: "claude-sonnet".to_string(),
            max_context_tokens: 200_000,
            max_output_tokens: 16_384,
            safe_input_tokens: 182_000,
        }
    }

    fn claude_haiku() -> Self {
        Self {
            model_id: "claude-haiku".to_string(),
            max_context_tokens: 200_000,
            max_output_tokens: 8_192,
            safe_input_tokens: 190_000,
        }
    }

    fn default_limits(model_id: &str) -> Self {
        // Conservative default for unknown models
        Self {
            model_id: model_id.to_string(),
            max_context_tokens: 200_000,
            max_output_tokens: 8_192,
            safe_input_tokens: 190_000,
        }
    }

    /// Create custom limits.
    pub fn custom(model_id: impl Into<String>, max_context: usize, max_output: usize) -> Self {
        let safe_input = max_context.saturating_sub(max_output).saturating_sub(1000);

        Self {
            model_id: model_id.into(),
            max_context_tokens: max_context,
            max_output_tokens: max_output,
            safe_input_tokens: safe_input,
        }
    }

    /// Check if a token count is within safe limits.
    pub fn is_safe(&self, input_tokens: usize, expected_output: usize) -> bool {
        input_tokens + expected_output <= self.max_context_tokens && input_tokens <= self.safe_input_tokens
    }

    /// Get remaining safe capacity.
    pub fn remaining_safe(&self, current_input: usize) -> usize {
        self.safe_input_tokens.saturating_sub(current_input)
    }

    /// Get a warning threshold (e.g., 80% of safe limit).
    pub fn warning_threshold(&self) -> usize {
        (self.safe_input_tokens as f64 * 0.80) as usize
    }
}

/// Registry of model limits
#[derive(Debug, Default)]
pub struct ModelLimitsRegistry {
    limits: HashMap<String, ModelLimits>,
}

impl ModelLimitsRegistry {
    pub fn new() -> Self {
        let mut registry = Self::default();

        // Register known models
        registry.register(ModelLimits::claude_opus());
        registry.register(ModelLimits::claude_sonnet());
        registry.register(ModelLimits::claude_haiku());

        registry
    }

    pub fn register(&mut self, limits: ModelLimits) {
        self.limits.insert(limits.model_id.clone(), limits);
    }

    pub fn get(&self, model_id: &str) -> ModelLimits {
        self.limits.get(model_id).cloned().unwrap_or_else(|| ModelLimits::for_model(model_id))
    }
}

// ============================================================================
// Slice 4.11.3: Pre-flight Check
// ============================================================================

/// Validates context before sending to LLM.
pub struct ContextValidator {
    counter: TokenCounter,
    limits_registry: ModelLimitsRegistry,
}

impl ContextValidator {
    pub fn new() -> Self {
        Self {
            counter: TokenCounter::default(),
            limits_registry: ModelLimitsRegistry::new(),
        }
    }

    pub fn with_tokenizer(mut self, tokenizer: TokenizerType) -> Self {
        self.counter = TokenCounter::new(tokenizer);
        self
    }

    /// Validate a prompt before sending to LLM.
    ///
    /// # Arguments
    /// * `model_id` - The model that will receive the prompt
    /// * `prompt_text` - The full prompt text
    /// * `expected_response` - Expected response length
    pub fn validate(&self, model_id: &str, prompt_text: &str, expected_response: ResponseLength) -> ValidationResult {
        let limits = self.limits_registry.get(model_id);
        let input_tokens = self.counter.count(prompt_text);
        let output_tokens = self.counter.estimate_response_tokens(expected_response);

        let status = if input_tokens > limits.max_context_tokens {
            ValidationStatus::ExceedsMaxContext {
                tokens: input_tokens,
                limit: limits.max_context_tokens,
            }
        } else if input_tokens > limits.safe_input_tokens {
            ValidationStatus::ExceedsSafeLimit {
                tokens: input_tokens,
                limit: limits.safe_input_tokens,
            }
        } else if input_tokens > limits.warning_threshold() {
            ValidationStatus::ApproachingLimit {
                tokens: input_tokens,
                threshold: limits.warning_threshold(),
                limit: limits.safe_input_tokens,
            }
        } else {
            ValidationStatus::Ok
        };

        ValidationResult {
            status,
            input_tokens,
            estimated_output_tokens: output_tokens,
            total_estimated: input_tokens + output_tokens,
            model_limits: limits,
        }
    }

    /// Validate with message structure.
    pub fn validate_messages(&self, model_id: &str, messages: &[MessageForCounting], expected_response: ResponseLength) -> ValidationResult {
        let limits = self.limits_registry.get(model_id);
        let input_tokens = self.counter.count_messages(messages);
        let output_tokens = self.counter.estimate_response_tokens(expected_response);

        let status = if input_tokens > limits.max_context_tokens {
            ValidationStatus::ExceedsMaxContext {
                tokens: input_tokens,
                limit: limits.max_context_tokens,
            }
        } else if input_tokens > limits.safe_input_tokens {
            ValidationStatus::ExceedsSafeLimit {
                tokens: input_tokens,
                limit: limits.safe_input_tokens,
            }
        } else if input_tokens > limits.warning_threshold() {
            ValidationStatus::ApproachingLimit {
                tokens: input_tokens,
                threshold: limits.warning_threshold(),
                limit: limits.safe_input_tokens,
            }
        } else {
            ValidationStatus::Ok
        };

        ValidationResult {
            status,
            input_tokens,
            estimated_output_tokens: output_tokens,
            total_estimated: input_tokens + output_tokens,
            model_limits: limits,
        }
    }
}

impl Default for ContextValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of context validation
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub status: ValidationStatus,
    pub input_tokens: usize,
    pub estimated_output_tokens: usize,
    pub total_estimated: usize,
    pub model_limits: ModelLimits,
}

impl ValidationResult {
    /// Check if this validation passed.
    pub fn is_ok(&self) -> bool {
        matches!(self.status, ValidationStatus::Ok | ValidationStatus::ApproachingLimit { .. })
    }

    /// Check if this is just a warning.
    pub fn is_warning(&self) -> bool {
        matches!(self.status, ValidationStatus::ApproachingLimit { .. })
    }

    /// Check if this is an error (will be rejected).
    pub fn is_error(&self) -> bool {
        matches!(self.status, ValidationStatus::ExceedsMaxContext { .. } | ValidationStatus::ExceedsSafeLimit { .. })
    }

    /// Get how many tokens need to be removed to fit.
    pub fn tokens_to_remove(&self) -> usize {
        match &self.status {
            ValidationStatus::ExceedsMaxContext { tokens, limit } => tokens - limit,
            ValidationStatus::ExceedsSafeLimit { tokens, limit } => tokens - limit,
            _ => 0,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ValidationStatus {
    /// Within all limits
    Ok,
    /// Within limits but approaching threshold
    ApproachingLimit { tokens: usize, threshold: usize, limit: usize },
    /// Exceeds safe input limit (might work but risky)
    ExceedsSafeLimit { tokens: usize, limit: usize },
    /// Exceeds maximum context (will definitely fail)
    ExceedsMaxContext { tokens: usize, limit: usize },
}

impl std::fmt::Display for ValidationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ok => write!(f, "OK"),
            Self::ApproachingLimit { tokens, threshold, limit } => {
                write!(f, "Warning: {} tokens (threshold: {}, limit: {})", tokens, threshold, limit)
            }
            Self::ExceedsSafeLimit { tokens, limit } => {
                write!(f, "Exceeds safe limit: {} tokens > {} limit", tokens, limit)
            }
            Self::ExceedsMaxContext { tokens, limit } => {
                write!(f, "Exceeds max context: {} tokens > {} limit", tokens, limit)
            }
        }
    }
}

// ============================================================================
// Slice 4.11.4: Automatic Truncation
// ============================================================================

/// Automatic truncation to fit within context limits.
pub struct ContextTruncator {
    counter: TokenCounter,
}

impl Default for ContextTruncator {
    fn default() -> Self {
        Self::new()
    }
}

impl ContextTruncator {
    pub fn new() -> Self {
        Self { counter: TokenCounter::default() }
    }

    /// Truncate context to fit within limits.
    ///
    /// Truncation priority (removed first):
    /// 1. Old conversation history
    /// 2. Reference files (non-essential)
    /// 3. Large file contents (summarize instead)
    /// 4. Examples (keep fewer)
    /// 5. Never remove: task description, files being modified
    pub fn truncate(&self, context: &mut TruncatableContext, target_tokens: usize) -> TruncationResult {
        let initial_tokens = self.count_context(context);

        if initial_tokens <= target_tokens {
            return TruncationResult {
                tokens_removed: 0,
                items_removed: vec![],
                final_tokens: initial_tokens,
                success: true,
            };
        }

        let mut removed = Vec::new();
        let tokens_to_remove = initial_tokens - target_tokens;
        let mut tokens_removed = 0;

        // 1. Truncate old conversation history
        if let Some(ref mut history) = context.conversation_history {
            let history_tokens = history.iter().map(|h| self.counter.count(&h.content)).sum::<usize>();

            if history_tokens > 0 && history.len() > 3 {
                // Keep only last 3 turns
                let removed_entries: Vec<_> = history.drain(..history.len() - 3).collect();
                let removed_tokens: usize = removed_entries.iter().map(|h| self.counter.count(&h.content)).sum();
                tokens_removed += removed_tokens;
                removed.push(TruncationAction::HistoryTruncated {
                    entries_removed: removed_entries.len(),
                    tokens_saved: removed_tokens,
                });
            }
        }

        if tokens_removed >= tokens_to_remove {
            return self.finalize_result(context, removed, tokens_removed);
        }

        // 2. Remove reference files (least relevant first)
        if !context.reference_files.is_empty() {
            // Sort by relevance (assume order is relevance)
            let mut to_remove = Vec::new();
            while tokens_removed < tokens_to_remove && !context.reference_files.is_empty() {
                if let Some(file) = context.reference_files.pop() {
                    let file_tokens = self.counter.count(&file.content);
                    tokens_removed += file_tokens;
                    to_remove.push(file.path.clone());
                }
            }
            if !to_remove.is_empty() {
                removed.push(TruncationAction::ReferenceFilesRemoved { files: to_remove });
            }
        }

        if tokens_removed >= tokens_to_remove {
            return self.finalize_result(context, removed, tokens_removed);
        }

        // 3. Summarize large files being modified
        for file in &mut context.files_to_modify {
            let file_tokens = self.counter.count(&file.content);
            if file_tokens > 1000 && tokens_removed < tokens_to_remove {
                // Apply summarization
                let summarizer = FileSummarizer::new(500, 300);
                let ext = std::path::Path::new(&file.path).extension().and_then(|e| e.to_str()).unwrap_or("");

                let result = summarizer.summarize_if_needed(&file.content, ext);
                if result.was_summarized {
                    let saved = file_tokens - result.summary_tokens;
                    file.content = result.content;
                    tokens_removed += saved;
                    removed.push(TruncationAction::FileSummarized {
                        path: file.path.clone(),
                        tokens_saved: saved,
                    });
                }
            }
        }

        if tokens_removed >= tokens_to_remove {
            return self.finalize_result(context, removed, tokens_removed);
        }

        // 4. Reduce examples
        if context.examples.len() > 1 {
            let to_keep = 1;
            let examples_removed: Vec<_> = context.examples.drain(to_keep..).collect();
            let tokens_saved: usize = examples_removed.iter().map(|e| self.counter.count(e)).sum();
            tokens_removed += tokens_saved;
            removed.push(TruncationAction::ExamplesReduced {
                kept: to_keep,
                removed: examples_removed.len(),
                tokens_saved,
            });
        }

        self.finalize_result(context, removed, tokens_removed)
    }

    fn count_context(&self, context: &TruncatableContext) -> usize {
        let mut total = 0;

        // Task description (required)
        total += self.counter.count(&context.task_description);

        // Files to modify (required)
        for file in &context.files_to_modify {
            total += self.counter.count(&file.content);
        }

        // Reference files
        for file in &context.reference_files {
            total += self.counter.count(&file.content);
        }

        // History
        if let Some(ref history) = context.conversation_history {
            for entry in history {
                total += self.counter.count(&entry.content);
            }
        }

        // Examples
        for example in &context.examples {
            total += self.counter.count(example);
        }

        // Conventions
        if let Some(ref conv) = context.conventions {
            total += self.counter.count(conv);
        }

        total
    }

    fn finalize_result(&self, context: &TruncatableContext, removed: Vec<TruncationAction>, tokens_removed: usize) -> TruncationResult {
        let final_tokens = self.count_context(context);
        TruncationResult {
            tokens_removed,
            items_removed: removed,
            final_tokens,
            success: true,
        }
    }
}

/// Context that can be truncated
#[derive(Debug, Clone)]
pub struct TruncatableContext {
    /// Task description (never truncate)
    pub task_description: String,
    /// Files being modified (summarize, never remove)
    pub files_to_modify: Vec<FileContext>,
    /// Reference files (can be removed)
    pub reference_files: Vec<FileContext>,
    /// Conversation history (older entries removed first)
    pub conversation_history: Option<Vec<TruncatableHistoryEntry>>,
    /// Examples (can be reduced)
    pub examples: Vec<String>,
    /// Conventions (summarize if needed)
    pub conventions: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FileContext {
    pub path: String,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct TruncatableHistoryEntry {
    pub role: String,
    pub content: String,
}

/// Result of truncation operation
#[derive(Debug)]
pub struct TruncationResult {
    pub tokens_removed: usize,
    pub items_removed: Vec<TruncationAction>,
    pub final_tokens: usize,
    pub success: bool,
}

impl TruncationResult {
    /// Get a human-readable summary of what was truncated.
    pub fn summary(&self) -> String {
        if self.items_removed.is_empty() {
            return "No truncation needed.".to_string();
        }

        let actions: Vec<String> = self.items_removed.iter().map(|a| a.to_string()).collect();

        format!("Truncated {} tokens:\n- {}", self.tokens_removed, actions.join("\n- "))
    }
}

#[derive(Debug, Clone)]
pub enum TruncationAction {
    HistoryTruncated { entries_removed: usize, tokens_saved: usize },
    ReferenceFilesRemoved { files: Vec<String> },
    FileSummarized { path: String, tokens_saved: usize },
    ExamplesReduced { kept: usize, removed: usize, tokens_saved: usize },
}

impl std::fmt::Display for TruncationAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HistoryTruncated { entries_removed, tokens_saved } => {
                write!(f, "Removed {} history entries ({} tokens)", entries_removed, tokens_saved)
            }
            Self::ReferenceFilesRemoved { files } => {
                write!(f, "Removed reference files: {:?}", files)
            }
            Self::FileSummarized { path, tokens_saved } => {
                write!(f, "Summarized {} ({} tokens saved)", path, tokens_saved)
            }
            Self::ExamplesReduced { kept, removed, tokens_saved } => {
                write!(f, "Reduced examples: kept {}, removed {} ({} tokens)", kept, removed, tokens_saved)
            }
        }
    }
}

// ============================================================================
// Slice 4.11.5: Context Pressure Warning
// ============================================================================

/// Warning messages for context pressure.
pub struct ContextPressureWarning;

impl ContextPressureWarning {
    /// Generate a warning for approaching context limits.
    pub fn for_validation(result: &ValidationResult) -> Option<ContextWarning> {
        match &result.status {
            ValidationStatus::Ok => None,

            ValidationStatus::ApproachingLimit { tokens, threshold: _, limit } => {
                let percentage = (*tokens as f64 / *limit as f64) * 100.0;
                Some(ContextWarning {
                    severity: WarningSeverity::Low,
                    message: format!("Context at {:.0}% capacity ({} / {} tokens)", percentage, tokens, limit),
                    suggestion: "Consider reducing history or reference files if more context needed.".to_string(),
                })
            }

            ValidationStatus::ExceedsSafeLimit { tokens, limit } => Some(ContextWarning {
                severity: WarningSeverity::Medium,
                message: format!("Context exceeds safe limit: {} tokens (limit: {})", tokens, limit),
                suggestion: "Will attempt automatic truncation. Consider removing reference files.".to_string(),
            }),

            ValidationStatus::ExceedsMaxContext { tokens, limit } => Some(ContextWarning {
                severity: WarningSeverity::High,
                message: format!("Context exceeds maximum: {} tokens (limit: {})", tokens, limit),
                suggestion: "Must reduce context. Remove files or summarize large content.".to_string(),
            }),
        }
    }

    /// Generate warning for a specific context category.
    pub fn for_category(category: ContextCategory, used: usize, budget: usize) -> Option<ContextWarning> {
        let percentage = (used as f64 / budget as f64) * 100.0;

        if percentage < 80.0 {
            None
        } else if percentage < 100.0 {
            Some(ContextWarning {
                severity: WarningSeverity::Low,
                message: format!("{:?} context at {:.0}% ({} / {} tokens)", category, percentage, used, budget),
                suggestion: Self::suggestion_for_category(category),
            })
        } else {
            Some(ContextWarning {
                severity: WarningSeverity::Medium,
                message: format!("{:?} context budget exceeded: {} / {} tokens", category, used, budget),
                suggestion: Self::suggestion_for_category(category),
            })
        }
    }

    fn suggestion_for_category(category: ContextCategory) -> String {
        match category {
            ContextCategory::FilesToModify => "Consider summarizing large files or working on fewer files at once.".to_string(),
            ContextCategory::ReferenceFiles => "Remove less relevant reference files.".to_string(),
            ContextCategory::TaskAndHistory => "Conversation history will be automatically truncated.".to_string(),
            ContextCategory::Conventions => "Consider using a shorter conventions summary.".to_string(),
        }
    }
}

/// A context warning to display
#[derive(Debug, Clone)]
pub struct ContextWarning {
    pub severity: WarningSeverity,
    pub message: String,
    pub suggestion: String,
}

impl ContextWarning {
    /// Format for display in the feed.
    pub fn format_for_feed(&self) -> String {
        let icon = match self.severity {
            WarningSeverity::Low => "[!]",
            WarningSeverity::Medium => "[!!]",
            WarningSeverity::High => "[!!!]",
        };

        format!("{} {}\n   {}", icon, self.message, self.suggestion)
    }

    /// Format as a short status line.
    pub fn format_short(&self) -> String {
        format!("[CONTEXT] {}", self.message)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WarningSeverity {
    /// Approaching limits, no action needed yet
    Low,
    /// At limit, automatic action will be taken
    Medium,
    /// Over limit, user action required
    High,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // Token Counter tests

    #[test]
    fn test_simple_counting() {
        let counter = TokenCounter::default();

        // ~4 chars per token
        assert_eq!(counter.count("hello"), 2); // 5 chars -> 2 tokens
        assert_eq!(counter.count("hello world"), 3); // 11 chars -> 3 tokens

        // Longer text
        let long_text = "a".repeat(1000);
        assert_eq!(counter.count(&long_text), 250); // 1000 chars -> 250 tokens
    }

    #[test]
    fn test_empty_text_counting() {
        let counter = TokenCounter::default();
        assert_eq!(counter.count(""), 0);
    }

    #[test]
    fn test_claude_tokenizer_type() {
        let counter = TokenCounter::new(TokenizerType::Claude);
        let text = "a".repeat(1000);
        let count = counter.count(&text);

        // Claude should be slightly more efficient
        assert!(count < 250);
    }

    #[test]
    fn test_gpt_tokenizer_type() {
        let counter = TokenCounter::new(TokenizerType::Gpt);
        let text = "a".repeat(1000);
        let count = counter.count(&text);

        // GPT uses same as simple for now
        assert_eq!(count, 250);
    }

    #[test]
    fn test_message_counting() {
        let counter = TokenCounter::default();

        let messages = vec![
            MessageForCounting {
                role: "user".to_string(),
                content: "Hello, how are you?".to_string(),
            },
            MessageForCounting {
                role: "assistant".to_string(),
                content: "I'm doing well!".to_string(),
            },
        ];

        let count = counter.count_messages(&messages);
        // Content tokens + overhead
        assert!(count > 10);
    }

    #[test]
    fn test_response_length_estimates() {
        let counter = TokenCounter::default();

        assert_eq!(counter.estimate_response_tokens(ResponseLength::Short), 256);
        assert_eq!(counter.estimate_response_tokens(ResponseLength::Medium), 1024);
        assert_eq!(counter.estimate_response_tokens(ResponseLength::Long), 4096);
        assert_eq!(counter.estimate_response_tokens(ResponseLength::VeryLong), 8192);
        assert_eq!(counter.estimate_response_tokens(ResponseLength::Custom(500)), 500);
    }

    // Model Limits tests

    #[test]
    fn test_model_limits_claude_sonnet() {
        let limits = ModelLimits::for_model("claude-3-sonnet-20240229");

        assert_eq!(limits.max_context_tokens, 200_000);
        assert_eq!(limits.max_output_tokens, 16_384);
        assert_eq!(limits.safe_input_tokens, 182_000);
    }

    #[test]
    fn test_model_limits_claude_opus() {
        let limits = ModelLimits::for_model("claude-3-opus-20240229");

        assert_eq!(limits.max_context_tokens, 200_000);
        assert_eq!(limits.max_output_tokens, 32_768);
        assert_eq!(limits.safe_input_tokens, 166_000);
    }

    #[test]
    fn test_model_limits_unknown_model() {
        let limits = ModelLimits::for_model("unknown-model");

        // Should get conservative defaults
        assert_eq!(limits.max_context_tokens, 200_000);
        assert_eq!(limits.max_output_tokens, 8_192);
        assert_eq!(limits.safe_input_tokens, 190_000);
    }

    #[test]
    fn test_model_limits_custom() {
        let limits = ModelLimits::custom("my-model", 50_000, 2_000);

        assert_eq!(limits.max_context_tokens, 50_000);
        assert_eq!(limits.max_output_tokens, 2_000);
        // safe_input = 50000 - 2000 - 1000 = 47000
        assert_eq!(limits.safe_input_tokens, 47_000);
    }

    #[test]
    fn test_model_limits_is_safe() {
        let limits = ModelLimits::for_model("claude-sonnet");

        assert!(limits.is_safe(100_000, 4_000));
        assert!(!limits.is_safe(190_000, 4_000)); // Exceeds safe input
        assert!(!limits.is_safe(182_001, 4_000)); // Just over safe input
    }

    #[test]
    fn test_model_limits_remaining_safe() {
        let limits = ModelLimits::for_model("claude-sonnet");

        assert_eq!(limits.remaining_safe(100_000), 82_000);
        assert_eq!(limits.remaining_safe(182_000), 0);
        assert_eq!(limits.remaining_safe(200_000), 0);
    }

    #[test]
    fn test_model_limits_warning_threshold() {
        let limits = ModelLimits::for_model("claude-sonnet");

        // 80% of 182_000 = 145_600
        assert_eq!(limits.warning_threshold(), 145_600);
    }

    #[test]
    fn test_model_limits_registry() {
        let registry = ModelLimitsRegistry::new();

        let sonnet = registry.get("claude-sonnet");
        assert_eq!(sonnet.max_context_tokens, 200_000);

        let unknown = registry.get("unknown");
        assert_eq!(unknown.max_context_tokens, 200_000);
    }

    #[test]
    fn test_model_limits_registry_custom() {
        let mut registry = ModelLimitsRegistry::new();
        registry.register(ModelLimits::custom("my-custom", 64_000, 4_000));

        let limits = registry.get("my-custom");
        assert_eq!(limits.max_context_tokens, 64_000);
    }

    // Context Validator tests

    #[test]
    fn test_validator_ok_status() {
        let validator = ContextValidator::new();
        let result = validator.validate("claude-sonnet", "Short prompt", ResponseLength::Medium);

        assert!(result.is_ok());
        assert!(!result.is_error());
        assert!(!result.is_warning());
        assert_eq!(result.tokens_to_remove(), 0);
    }

    #[test]
    fn test_validator_approaching_limit() {
        let validator = ContextValidator::new();

        // Create prompt that is ~85% of safe limit (180k * 0.85 = 153k tokens)
        // At 4 chars/token, we need ~612k chars
        let large_prompt = "a".repeat(612_000);

        let result = validator.validate("claude-sonnet", &large_prompt, ResponseLength::Medium);

        assert!(result.is_ok());
        assert!(result.is_warning());
        assert!(!result.is_error());
    }

    #[test]
    fn test_validator_exceeds_safe_limit() {
        let validator = ContextValidator::new();

        // Create prompt that exceeds safe limit (180k tokens)
        // At 4 chars/token, we need > 720k chars
        let large_prompt = "a".repeat(760_000);

        let result = validator.validate("claude-sonnet", &large_prompt, ResponseLength::Medium);

        assert!(!result.is_ok());
        assert!(result.is_error());
        assert!(result.tokens_to_remove() > 0);
    }

    #[test]
    fn test_validator_exceeds_max_context() {
        let validator = ContextValidator::new();

        // Create prompt that exceeds max context (200k tokens)
        // At 4 chars/token, we need > 800k chars
        let large_prompt = "a".repeat(900_000);

        let result = validator.validate("claude-sonnet", &large_prompt, ResponseLength::Medium);

        assert!(!result.is_ok());
        assert!(result.is_error());
        matches!(result.status, ValidationStatus::ExceedsMaxContext { .. });
    }

    #[test]
    fn test_validator_with_messages() {
        let validator = ContextValidator::new();

        let messages = vec![
            MessageForCounting {
                role: "user".to_string(),
                content: "Hello".to_string(),
            },
            MessageForCounting {
                role: "assistant".to_string(),
                content: "Hi there!".to_string(),
            },
        ];

        let result = validator.validate_messages("claude-sonnet", &messages, ResponseLength::Short);

        assert!(result.is_ok());
        assert!(result.input_tokens < 100);
    }

    #[test]
    fn test_validator_with_tokenizer() {
        let validator = ContextValidator::new().with_tokenizer(TokenizerType::Claude);

        let result = validator.validate("claude-sonnet", "Test prompt", ResponseLength::Short);

        assert!(result.is_ok());
    }

    #[test]
    fn test_validation_status_display() {
        let ok = ValidationStatus::Ok;
        assert_eq!(format!("{}", ok), "OK");

        let approaching = ValidationStatus::ApproachingLimit {
            tokens: 150_000,
            threshold: 144_000,
            limit: 180_000,
        };
        assert!(format!("{}", approaching).contains("Warning"));

        let exceeds_safe = ValidationStatus::ExceedsSafeLimit { tokens: 185_000, limit: 180_000 };
        assert!(format!("{}", exceeds_safe).contains("Exceeds safe limit"));

        let exceeds_max = ValidationStatus::ExceedsMaxContext { tokens: 220_000, limit: 200_000 };
        assert!(format!("{}", exceeds_max).contains("Exceeds max context"));
    }

    // Truncation tests

    #[test]
    fn test_truncator_no_truncation_needed() {
        let truncator = ContextTruncator::new();

        let mut context = TruncatableContext {
            task_description: "Simple task".to_string(),
            files_to_modify: vec![],
            reference_files: vec![],
            conversation_history: None,
            examples: vec![],
            conventions: None,
        };

        let result = truncator.truncate(&mut context, 1000);

        assert_eq!(result.tokens_removed, 0);
        assert!(result.items_removed.is_empty());
        assert!(result.success);
    }

    #[test]
    fn test_truncator_removes_history() {
        let truncator = ContextTruncator::new();

        let mut context = TruncatableContext {
            task_description: "Task".to_string(),
            files_to_modify: vec![],
            reference_files: vec![],
            conversation_history: Some(vec![
                TruncatableHistoryEntry {
                    role: "user".to_string(),
                    content: "Message 1".to_string(),
                },
                TruncatableHistoryEntry {
                    role: "assistant".to_string(),
                    content: "Response 1".to_string(),
                },
                TruncatableHistoryEntry {
                    role: "user".to_string(),
                    content: "Message 2".to_string(),
                },
                TruncatableHistoryEntry {
                    role: "assistant".to_string(),
                    content: "Response 2".to_string(),
                },
                TruncatableHistoryEntry {
                    role: "user".to_string(),
                    content: "Message 3".to_string(),
                },
            ]),
            examples: vec![],
            conventions: None,
        };

        let result = truncator.truncate(&mut context, 10);

        // Should have truncated some history
        assert!(result.tokens_removed > 0);
        assert!(context.conversation_history.as_ref().unwrap().len() <= 3);
    }

    #[test]
    fn test_truncator_removes_reference_files() {
        let truncator = ContextTruncator::new();

        let mut context = TruncatableContext {
            task_description: "Task".to_string(),
            files_to_modify: vec![],
            reference_files: vec![
                FileContext {
                    path: "ref1.rs".to_string(),
                    content: "Large reference content".repeat(100),
                },
                FileContext {
                    path: "ref2.rs".to_string(),
                    content: "Another reference".repeat(100),
                },
            ],
            conversation_history: None,
            examples: vec![],
            conventions: None,
        };

        let result = truncator.truncate(&mut context, 10);

        assert!(result.tokens_removed > 0);
        assert!(context.reference_files.len() < 2);
    }

    #[test]
    fn test_truncator_reduces_examples() {
        let truncator = ContextTruncator::new();

        let mut context = TruncatableContext {
            task_description: "Task".to_string(),
            files_to_modify: vec![],
            reference_files: vec![],
            conversation_history: None,
            examples: vec!["Example 1".repeat(50), "Example 2".repeat(50), "Example 3".repeat(50)],
            conventions: None,
        };

        let result = truncator.truncate(&mut context, 10);

        assert!(result.tokens_removed > 0);
        assert_eq!(context.examples.len(), 1);
    }

    #[test]
    fn test_truncation_result_summary() {
        let result = TruncationResult {
            tokens_removed: 500,
            items_removed: vec![
                TruncationAction::HistoryTruncated {
                    entries_removed: 5,
                    tokens_saved: 200,
                },
                TruncationAction::ReferenceFilesRemoved {
                    files: vec!["file.rs".to_string()],
                },
            ],
            final_tokens: 1000,
            success: true,
        };

        let summary = result.summary();
        assert!(summary.contains("Truncated 500 tokens"));
        assert!(summary.contains("5 history entries"));
        assert!(summary.contains("file.rs"));
    }

    #[test]
    fn test_truncation_result_no_changes() {
        let result = TruncationResult {
            tokens_removed: 0,
            items_removed: vec![],
            final_tokens: 100,
            success: true,
        };

        assert_eq!(result.summary(), "No truncation needed.");
    }

    #[test]
    fn test_truncation_action_display() {
        let history = TruncationAction::HistoryTruncated {
            entries_removed: 3,
            tokens_saved: 150,
        };
        assert!(format!("{}", history).contains("3"));
        assert!(format!("{}", history).contains("150"));

        let files = TruncationAction::ReferenceFilesRemoved {
            files: vec!["a.rs".to_string(), "b.rs".to_string()],
        };
        assert!(format!("{}", files).contains("a.rs"));

        let summarized = TruncationAction::FileSummarized {
            path: "large.rs".to_string(),
            tokens_saved: 500,
        };
        assert!(format!("{}", summarized).contains("large.rs"));
        assert!(format!("{}", summarized).contains("500"));

        let examples = TruncationAction::ExamplesReduced {
            kept: 1,
            removed: 4,
            tokens_saved: 200,
        };
        assert!(format!("{}", examples).contains("kept 1"));
        assert!(format!("{}", examples).contains("removed 4"));
    }

    // Warning tests

    #[test]
    fn test_warning_generation() {
        let limits = ModelLimits::for_model("claude-sonnet");
        let result = ValidationResult {
            status: ValidationStatus::ApproachingLimit {
                tokens: 170_000,
                threshold: 144_000,
                limit: 180_000,
            },
            input_tokens: 170_000,
            estimated_output_tokens: 4_096,
            total_estimated: 174_096,
            model_limits: limits,
        };

        let warning = ContextPressureWarning::for_validation(&result);
        assert!(warning.is_some());
        assert_eq!(warning.unwrap().severity, WarningSeverity::Low);
    }

    #[test]
    fn test_no_warning_when_ok() {
        let limits = ModelLimits::for_model("claude-sonnet");
        let result = ValidationResult {
            status: ValidationStatus::Ok,
            input_tokens: 50_000,
            estimated_output_tokens: 4_096,
            total_estimated: 54_096,
            model_limits: limits,
        };

        let warning = ContextPressureWarning::for_validation(&result);
        assert!(warning.is_none());
    }

    #[test]
    fn test_warning_exceeds_safe() {
        let limits = ModelLimits::for_model("claude-sonnet");
        let result = ValidationResult {
            status: ValidationStatus::ExceedsSafeLimit { tokens: 185_000, limit: 180_000 },
            input_tokens: 185_000,
            estimated_output_tokens: 4_096,
            total_estimated: 189_096,
            model_limits: limits,
        };

        let warning = ContextPressureWarning::for_validation(&result);
        assert!(warning.is_some());
        assert_eq!(warning.unwrap().severity, WarningSeverity::Medium);
    }

    #[test]
    fn test_warning_exceeds_max() {
        let limits = ModelLimits::for_model("claude-sonnet");
        let result = ValidationResult {
            status: ValidationStatus::ExceedsMaxContext { tokens: 210_000, limit: 200_000 },
            input_tokens: 210_000,
            estimated_output_tokens: 4_096,
            total_estimated: 214_096,
            model_limits: limits,
        };

        let warning = ContextPressureWarning::for_validation(&result);
        assert!(warning.is_some());
        assert_eq!(warning.unwrap().severity, WarningSeverity::High);
    }

    #[test]
    fn test_warning_for_category_under_threshold() {
        let warning = ContextPressureWarning::for_category(ContextCategory::FilesToModify, 30, 100);
        assert!(warning.is_none());
    }

    #[test]
    fn test_warning_for_category_approaching() {
        let warning = ContextPressureWarning::for_category(ContextCategory::FilesToModify, 85, 100);
        assert!(warning.is_some());
        assert_eq!(warning.unwrap().severity, WarningSeverity::Low);
    }

    #[test]
    fn test_warning_for_category_exceeded() {
        let warning = ContextPressureWarning::for_category(ContextCategory::FilesToModify, 120, 100);
        assert!(warning.is_some());
        assert_eq!(warning.unwrap().severity, WarningSeverity::Medium);
    }

    #[test]
    fn test_warning_format_for_feed() {
        let warning = ContextWarning {
            severity: WarningSeverity::High,
            message: "Context exceeded".to_string(),
            suggestion: "Remove files".to_string(),
        };

        let formatted = warning.format_for_feed();
        assert!(formatted.contains("[!!!]"));
        assert!(formatted.contains("Context exceeded"));
        assert!(formatted.contains("Remove files"));
    }

    #[test]
    fn test_warning_format_short() {
        let warning = ContextWarning {
            severity: WarningSeverity::Low,
            message: "At 85% capacity".to_string(),
            suggestion: "Consider reducing".to_string(),
        };

        let formatted = warning.format_short();
        assert!(formatted.contains("[CONTEXT]"));
        assert!(formatted.contains("At 85% capacity"));
    }
}
