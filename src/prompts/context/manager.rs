//! Context budget management for prompts.
//!
//! Manages token budget allocation, file selection, context requests,
//! and conversation history.

use std::collections::HashMap;

/// Manages context budget allocation for prompts.
#[derive(Debug, Clone)]
pub struct ContextBudget {
    /// Total token budget
    total_tokens: usize,
    /// Budget per category
    category_budgets: HashMap<ContextCategory, usize>,
    /// Used tokens per category
    category_used: HashMap<ContextCategory, usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContextCategory {
    /// Files being modified (40% default)
    FilesToModify,
    /// Reference files (25% default)
    ReferenceFiles,
    /// Task description + history (20% default)
    TaskAndHistory,
    /// Project conventions (15% default)
    Conventions,
}

impl ContextCategory {
    /// Budget percentage for this category
    pub fn budget_percent(&self) -> f32 {
        match self {
            Self::FilesToModify => 0.40,
            Self::ReferenceFiles => 0.25,
            Self::TaskAndHistory => 0.20,
            Self::Conventions => 0.15,
        }
    }

    /// All categories in priority order
    pub fn all() -> &'static [ContextCategory] {
        &[
            ContextCategory::FilesToModify,
            ContextCategory::ReferenceFiles,
            ContextCategory::TaskAndHistory,
            ContextCategory::Conventions,
        ]
    }
}

impl ContextBudget {
    /// Create with default budget allocation
    pub fn new(total_tokens: usize) -> Self {
        let mut category_budgets = HashMap::new();
        category_budgets.insert(ContextCategory::FilesToModify, (total_tokens as f64 * 0.40) as usize);
        category_budgets.insert(ContextCategory::ReferenceFiles, (total_tokens as f64 * 0.25) as usize);
        category_budgets.insert(ContextCategory::TaskAndHistory, (total_tokens as f64 * 0.20) as usize);
        category_budgets.insert(ContextCategory::Conventions, (total_tokens as f64 * 0.15) as usize);

        Self {
            total_tokens,
            category_budgets,
            category_used: HashMap::new(),
        }
    }

    /// Create with custom budget allocation
    pub fn with_allocation(total_tokens: usize, allocation: BudgetAllocation) -> Self {
        let mut category_budgets = HashMap::new();
        category_budgets.insert(ContextCategory::FilesToModify, (total_tokens as f64 * allocation.files_to_modify) as usize);
        category_budgets.insert(ContextCategory::ReferenceFiles, (total_tokens as f64 * allocation.reference_files) as usize);
        category_budgets.insert(ContextCategory::TaskAndHistory, (total_tokens as f64 * allocation.task_and_history) as usize);
        category_budgets.insert(ContextCategory::Conventions, (total_tokens as f64 * allocation.conventions) as usize);

        Self {
            total_tokens,
            category_budgets,
            category_used: HashMap::new(),
        }
    }

    /// Check if there's budget available for a category
    pub fn has_budget(&self, category: ContextCategory, tokens_needed: usize) -> bool {
        let budget = self.category_budgets.get(&category).copied().unwrap_or(0);
        let used = self.category_used.get(&category).copied().unwrap_or(0);
        used + tokens_needed <= budget
    }

    /// Try to allocate tokens from a category budget
    pub fn allocate(&mut self, category: ContextCategory, tokens: usize) -> Result<(), BudgetError> {
        if !self.has_budget(category, tokens) {
            return Err(BudgetError::CategoryExceeded {
                category,
                budget: self.category_budgets.get(&category).copied().unwrap_or(0),
                requested: tokens,
                used: self.category_used.get(&category).copied().unwrap_or(0),
            });
        }

        *self.category_used.entry(category).or_insert(0) += tokens;
        Ok(())
    }

    /// Get remaining budget for a category
    pub fn remaining(&self, category: ContextCategory) -> usize {
        let budget = self.category_budgets.get(&category).copied().unwrap_or(0);
        let used = self.category_used.get(&category).copied().unwrap_or(0);
        budget.saturating_sub(used)
    }

    /// Get total remaining budget
    pub fn total_remaining(&self) -> usize {
        self.total_tokens.saturating_sub(self.total_used())
    }

    /// Get total used tokens
    pub fn total_used(&self) -> usize {
        self.category_used.values().sum()
    }

    /// Get usage summary
    pub fn summary(&self) -> BudgetSummary {
        BudgetSummary {
            total_budget: self.total_tokens,
            total_used: self.total_used(),
            by_category: self
                .category_budgets
                .keys()
                .map(|cat| {
                    (
                        *cat,
                        CategoryUsage {
                            budget: self.category_budgets.get(cat).copied().unwrap_or(0),
                            used: self.category_used.get(cat).copied().unwrap_or(0),
                        },
                    )
                })
                .collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BudgetAllocation {
    pub files_to_modify: f64,
    pub reference_files: f64,
    pub task_and_history: f64,
    pub conventions: f64,
}

impl Default for BudgetAllocation {
    fn default() -> Self {
        Self {
            files_to_modify: 0.40,
            reference_files: 0.25,
            task_and_history: 0.20,
            conventions: 0.15,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BudgetSummary {
    pub total_budget: usize,
    pub total_used: usize,
    pub by_category: HashMap<ContextCategory, CategoryUsage>,
}

#[derive(Debug, Clone)]
pub struct CategoryUsage {
    pub budget: usize,
    pub used: usize,
}

impl CategoryUsage {
    pub fn remaining(&self) -> usize {
        self.budget.saturating_sub(self.used)
    }

    pub fn percentage_used(&self) -> f64 {
        if self.budget == 0 {
            0.0
        } else {
            (self.used as f64 / self.budget as f64) * 100.0
        }
    }
}

#[derive(Debug)]
pub enum BudgetError {
    CategoryExceeded {
        category: ContextCategory,
        budget: usize,
        requested: usize,
        used: usize,
    },
}

impl std::fmt::Display for BudgetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CategoryExceeded { category, budget, requested, used } => {
                write!(f, "{:?} budget exceeded: {} used + {} requested > {} budget", category, used, requested, budget)
            }
        }
    }
}

impl std::error::Error for BudgetError {}

// ============================================================================
// File Selection
// ============================================================================

/// Selects files based on relevance to a task.
pub struct FileSelector {
    /// Base path for file operations
    base_path: std::path::PathBuf,
}

impl FileSelector {
    pub fn new(base_path: impl Into<std::path::PathBuf>) -> Self {
        Self { base_path: base_path.into() }
    }

    /// Get the base path.
    pub fn base_path(&self) -> &std::path::Path {
        &self.base_path
    }

    /// Score and rank files by relevance to a task.
    ///
    /// Returns files sorted by relevance (highest first).
    pub fn select_relevant(&self, task_description: &str, available_files: &[FileInfo], max_files: usize) -> Vec<ScoredFile> {
        let keywords = self.extract_keywords(task_description);

        let mut scored: Vec<ScoredFile> = available_files.iter().map(|file| self.score_file(file, &keywords)).collect();

        // Sort by score descending
        scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        // Take top N
        scored.truncate(max_files);
        scored
    }

    fn score_file(&self, file: &FileInfo, keywords: &[String]) -> ScoredFile {
        let mut score = 0.0;

        // Filename match (highest weight)
        for keyword in keywords {
            if file.path.to_lowercase().contains(&keyword.to_lowercase()) {
                score += 10.0;
            }
        }

        // File extension relevance
        if let Some(ext) = std::path::Path::new(&file.path).extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();
            match ext_str.as_str() {
                "rs" | "py" | "ts" | "js" | "go" => score += 2.0, // Code files
                "md" | "txt" => score += 1.0,                     // Documentation
                "toml" | "yaml" | "json" => score += 1.5,         // Config
                _ => {}
            }
        }

        // Content match (if content available)
        if let Some(ref content) = file.content_preview {
            for keyword in keywords {
                if content.to_lowercase().contains(&keyword.to_lowercase()) {
                    score += 3.0;
                }
            }
        }

        // Import/reference boost (simplified)
        if file.path.contains("mod.rs") || file.path.contains("lib.rs") {
            score += 1.5; // Module roots are often useful
        }

        ScoredFile {
            path: file.path.clone(),
            score,
            relevance_reason: self.explain_relevance(&file.path, score, keywords),
        }
    }

    fn extract_keywords(&self, text: &str) -> Vec<String> {
        // Simple keyword extraction
        // Could be enhanced with NLP/stemming
        text.split_whitespace()
            .filter(|w| w.len() > 3)
            .filter(|w| !STOP_WORDS.contains(&w.to_lowercase().as_str()))
            .map(|w| w.to_lowercase())
            .collect()
    }

    fn explain_relevance(&self, path: &str, score: f64, keywords: &[String]) -> String {
        let matching: Vec<_> = keywords.iter().filter(|kw| path.to_lowercase().contains(&kw.to_lowercase())).collect();

        if matching.is_empty() {
            format!("Relevance score: {:.1}", score)
        } else {
            format!("Matches keywords: {:?} (score: {:.1})", matching, score)
        }
    }
}

const STOP_WORDS: &[&str] = &[
    "the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for", "of", "with", "by", "from", "this", "that", "these", "those", "is", "are", "was", "were", "be", "been", "being", "have",
    "has", "had", "do", "does", "did", "will", "would", "could", "should", "may", "might",
];

/// Information about a file for selection
#[derive(Debug, Clone)]
pub struct FileInfo {
    pub path: String,
    pub size_bytes: usize,
    pub content_preview: Option<String>,
}

/// A file with its relevance score
#[derive(Debug, Clone)]
pub struct ScoredFile {
    pub path: String,
    pub score: f64,
    pub relevance_reason: String,
}

impl ScoredFile {
    pub fn is_highly_relevant(&self) -> bool {
        self.score >= 10.0
    }
}

// ============================================================================
// Context Request Protocol
// ============================================================================

/// Handles context requests from agents.
pub struct ContextRequestHandler {
    /// Pending requests
    pending: Vec<ContextRequest>,
    /// Fulfilled requests
    fulfilled: Vec<FulfilledRequest>,
}

impl ContextRequestHandler {
    pub fn new(_base_path: impl Into<std::path::PathBuf>) -> Self {
        Self {
            pending: Vec::new(),
            fulfilled: Vec::new(),
        }
    }

    /// Add a context request from an agent
    pub fn add_request(&mut self, request: ContextRequest) {
        self.pending.push(request);
    }

    /// Process pending requests and return fulfilled context
    pub fn fulfill_pending(&mut self, available_files: &[FileInfo]) -> Vec<FulfilledRequest> {
        // Drain pending into a separate vector to avoid borrow issues
        let pending: Vec<ContextRequest> = self.pending.drain(..).collect();
        let mut fulfilled = Vec::new();

        for request in pending {
            let result = self.fulfill_request(&request, available_files);
            fulfilled.push(result.clone());
            self.fulfilled.push(result);
        }

        fulfilled
    }

    fn fulfill_request(&self, request: &ContextRequest, available_files: &[FileInfo]) -> FulfilledRequest {
        match &request.request_type {
            ContextRequestType::SpecificFile(path) => {
                if let Some(file) = available_files.iter().find(|f| f.path == *path) {
                    FulfilledRequest {
                        original_request: request.clone(),
                        status: FulfillmentStatus::Fulfilled,
                        content: file.content_preview.clone(),
                        alternatives: vec![],
                    }
                } else {
                    // Try to find similar files
                    let alternatives: Vec<String> = available_files
                        .iter()
                        .filter(|f| {
                            let req_name = std::path::Path::new(path).file_name();
                            let file_name = std::path::Path::new(&f.path).file_name();
                            req_name == file_name
                        })
                        .map(|f| f.path.clone())
                        .take(3)
                        .collect();

                    FulfilledRequest {
                        original_request: request.clone(),
                        status: FulfillmentStatus::NotFound,
                        content: None,
                        alternatives,
                    }
                }
            }
            ContextRequestType::FilesMatching(pattern) => {
                let matching: Vec<_> = available_files.iter().filter(|f| f.path.contains(pattern)).collect();

                if matching.is_empty() {
                    FulfilledRequest {
                        original_request: request.clone(),
                        status: FulfillmentStatus::NotFound,
                        content: None,
                        alternatives: vec![],
                    }
                } else {
                    let paths: Vec<_> = matching.iter().map(|f| f.path.clone()).collect();
                    FulfilledRequest {
                        original_request: request.clone(),
                        status: FulfillmentStatus::Fulfilled,
                        content: Some(format!("Found {} files matching '{}':\n{}", matching.len(), pattern, paths.join("\n"))),
                        alternatives: vec![],
                    }
                }
            }
            ContextRequestType::FunctionDefinition { name, file_hint } => {
                // Search for function in files
                let search_files: Vec<_> = if let Some(hint) = file_hint {
                    available_files.iter().filter(|f| f.path.contains(hint)).collect()
                } else {
                    available_files.iter().collect()
                };

                for file in search_files {
                    if let Some(ref content) = file.content_preview {
                        if content.contains(&format!("fn {}", name)) || content.contains(&format!("def {}", name)) {
                            return FulfilledRequest {
                                original_request: request.clone(),
                                status: FulfillmentStatus::Fulfilled,
                                content: Some(format!("Found '{}' in {}", name, file.path)),
                                alternatives: vec![file.path.clone()],
                            };
                        }
                    }
                }

                FulfilledRequest {
                    original_request: request.clone(),
                    status: FulfillmentStatus::NotFound,
                    content: None,
                    alternatives: vec![],
                }
            }
        }
    }

    /// Get all pending requests
    pub fn pending_requests(&self) -> &[ContextRequest] {
        &self.pending
    }

    /// Get history of fulfilled requests
    pub fn fulfilled_history(&self) -> &[FulfilledRequest] {
        &self.fulfilled
    }
}

/// A request for additional context from an agent
#[derive(Debug, Clone)]
pub struct ContextRequest {
    /// What type of context is needed
    pub request_type: ContextRequestType,
    /// Why this context is needed
    pub reason: String,
    /// Priority of this request
    pub priority: RequestPriority,
}

#[derive(Debug, Clone)]
pub enum ContextRequestType {
    /// Request a specific file by path
    SpecificFile(String),
    /// Request files matching a pattern
    FilesMatching(String),
    /// Request a function definition
    FunctionDefinition { name: String, file_hint: Option<String> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestPriority {
    /// Required to continue
    Required,
    /// Would be helpful
    Helpful,
}

/// Result of fulfilling a context request
#[derive(Debug, Clone)]
pub struct FulfilledRequest {
    pub original_request: ContextRequest,
    pub status: FulfillmentStatus,
    pub content: Option<String>,
    pub alternatives: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FulfillmentStatus {
    Fulfilled,
    NotFound,
    TooLarge,
}

// ============================================================================
// Conversation History Management
// ============================================================================

/// Manages conversation history within token budget.
pub struct HistoryManager {
    /// Maximum tokens for history
    budget_tokens: usize,
    /// All history entries
    entries: Vec<ConversationEntry>,
    /// Entries marked as important (preserved during truncation)
    important_indices: std::collections::HashSet<usize>,
}

impl HistoryManager {
    pub fn new(budget_tokens: usize) -> Self {
        Self {
            budget_tokens,
            entries: Vec::new(),
            important_indices: std::collections::HashSet::new(),
        }
    }

    /// Get the budget tokens.
    pub fn budget_tokens(&self) -> usize {
        self.budget_tokens
    }

    /// Get the total number of entries.
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Add a new history entry
    pub fn add(&mut self, entry: ConversationEntry) {
        // Auto-mark as important if it contains certain keywords
        let is_important = self.is_automatically_important(&entry);
        let index = self.entries.len();
        self.entries.push(entry);

        if is_important {
            self.important_indices.insert(index);
        }
    }

    /// Mark an entry as important (won't be truncated)
    pub fn mark_important(&mut self, index: usize) {
        if index < self.entries.len() {
            self.important_indices.insert(index);
        }
    }

    /// Get history that fits within budget
    pub fn get_within_budget(&self) -> Vec<&ConversationEntry> {
        let mut result = Vec::new();
        let mut total_tokens = 0;

        // First, add all important entries
        for (i, entry) in self.entries.iter().enumerate() {
            if self.important_indices.contains(&i) {
                let tokens = estimate_tokens(&entry.content);
                if total_tokens + tokens <= self.budget_tokens {
                    result.push((i, entry));
                    total_tokens += tokens;
                }
            }
        }

        // Then add recent entries (in reverse order, most recent first)
        for (i, entry) in self.entries.iter().enumerate().rev() {
            if self.important_indices.contains(&i) {
                continue; // Already added
            }

            let tokens = estimate_tokens(&entry.content);
            if total_tokens + tokens <= self.budget_tokens {
                result.push((i, entry));
                total_tokens += tokens;
            }
        }

        // Sort by original index to maintain chronological order
        result.sort_by_key(|(i, _)| *i);
        result.into_iter().map(|(_, e)| e).collect()
    }

    /// Get truncation summary
    pub fn truncation_summary(&self) -> HistorySummary {
        let total_entries = self.entries.len();
        let within_budget = self.get_within_budget().len();

        HistorySummary {
            total_entries,
            entries_kept: within_budget,
            entries_truncated: total_entries - within_budget,
            important_count: self.important_indices.len(),
        }
    }

    fn is_automatically_important(&self, entry: &ConversationEntry) -> bool {
        let content_lower = entry.content.to_lowercase();

        // Mark as important if contains decision indicators
        let decision_keywords = ["decided", "agreed", "confirmed", "approved", "rejected", "error", "failed", "bug", "issue"];

        decision_keywords.iter().any(|kw| content_lower.contains(kw))
    }
}

/// A conversation history entry
#[derive(Debug, Clone)]
pub struct ConversationEntry {
    pub role: String, // "user" | "assistant" | "system"
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub metadata: Option<HistoryMetadata>,
}

#[derive(Debug, Clone)]
pub struct HistoryMetadata {
    pub task_id: Option<String>,
    pub was_error: bool,
    pub contains_decision: bool,
}

#[derive(Debug, Clone)]
pub struct HistorySummary {
    pub total_entries: usize,
    pub entries_kept: usize,
    pub entries_truncated: usize,
    pub important_count: usize,
}

/// Simple token estimation (roughly 4 chars per token).
///
/// Note: This is a placeholder. Ticket 4.11 will implement proper token counting.
pub fn estimate_tokens(text: &str) -> usize {
    text.len().div_ceil(4)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_budget_new() {
        let budget = ContextBudget::new(10000);

        assert_eq!(budget.remaining(ContextCategory::FilesToModify), 4000);
        assert_eq!(budget.remaining(ContextCategory::ReferenceFiles), 2500);
        assert_eq!(budget.remaining(ContextCategory::TaskAndHistory), 2000);
        assert_eq!(budget.remaining(ContextCategory::Conventions), 1500);
    }

    #[test]
    fn test_context_budget_allocate() {
        let mut budget = ContextBudget::new(10000);

        // Should succeed
        assert!(budget.allocate(ContextCategory::FilesToModify, 1000).is_ok());
        assert_eq!(budget.remaining(ContextCategory::FilesToModify), 3000);

        // Should succeed again
        assert!(budget.allocate(ContextCategory::FilesToModify, 2000).is_ok());
        assert_eq!(budget.remaining(ContextCategory::FilesToModify), 1000);

        // Should fail - exceeds remaining
        assert!(budget.allocate(ContextCategory::FilesToModify, 2000).is_err());
    }

    #[test]
    fn test_context_budget_has_budget() {
        let mut budget = ContextBudget::new(10000);

        assert!(budget.has_budget(ContextCategory::FilesToModify, 4000));
        assert!(!budget.has_budget(ContextCategory::FilesToModify, 4001));

        budget.allocate(ContextCategory::FilesToModify, 3000).unwrap();
        assert!(budget.has_budget(ContextCategory::FilesToModify, 1000));
        assert!(!budget.has_budget(ContextCategory::FilesToModify, 1001));
    }

    #[test]
    fn test_context_budget_total_used() {
        let mut budget = ContextBudget::new(10000);

        budget.allocate(ContextCategory::FilesToModify, 1000).unwrap();
        budget.allocate(ContextCategory::ReferenceFiles, 500).unwrap();
        budget.allocate(ContextCategory::TaskAndHistory, 200).unwrap();

        assert_eq!(budget.total_used(), 1700);
        assert_eq!(budget.total_remaining(), 8300);
    }

    #[test]
    fn test_context_budget_with_custom_allocation() {
        let allocation = BudgetAllocation {
            files_to_modify: 0.50,
            reference_files: 0.30,
            task_and_history: 0.15,
            conventions: 0.05,
        };

        let budget = ContextBudget::with_allocation(10000, allocation);

        assert_eq!(budget.remaining(ContextCategory::FilesToModify), 5000);
        assert_eq!(budget.remaining(ContextCategory::ReferenceFiles), 3000);
        assert_eq!(budget.remaining(ContextCategory::TaskAndHistory), 1500);
        assert_eq!(budget.remaining(ContextCategory::Conventions), 500);
    }

    #[test]
    fn test_budget_summary() {
        let mut budget = ContextBudget::new(10000);
        budget.allocate(ContextCategory::FilesToModify, 1000).unwrap();

        let summary = budget.summary();
        assert_eq!(summary.total_budget, 10000);
        assert_eq!(summary.total_used, 1000);

        let files_usage = summary.by_category.get(&ContextCategory::FilesToModify).unwrap();
        assert_eq!(files_usage.budget, 4000);
        assert_eq!(files_usage.used, 1000);
        assert_eq!(files_usage.remaining(), 3000);
        assert!((files_usage.percentage_used() - 25.0).abs() < 0.01);
    }

    #[test]
    fn test_context_category_budget_totals_100() {
        let total: f32 = ContextCategory::all().iter().map(|c| c.budget_percent()).sum();
        assert!((total - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_estimate_tokens() {
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("a"), 1);
        assert_eq!(estimate_tokens("abcd"), 1);
        assert_eq!(estimate_tokens("abcde"), 2);
        assert_eq!(estimate_tokens("abcdefgh"), 2);
    }

    // FileSelector tests

    #[test]
    fn test_file_selector_filename_match() {
        let selector = FileSelector::new("/project");

        let files = vec![
            FileInfo {
                path: "src/config.rs".to_string(),
                size_bytes: 100,
                content_preview: None,
            },
            FileInfo {
                path: "src/main.rs".to_string(),
                size_bytes: 100,
                content_preview: None,
            },
        ];

        let scored = selector.select_relevant("update config loading", &files, 10);

        // config.rs should be first (keyword match)
        assert_eq!(scored[0].path, "src/config.rs");
        assert!(scored[0].score > scored[1].score);
    }

    #[test]
    fn test_file_selector_content_match() {
        let selector = FileSelector::new("/project");

        let files = vec![
            FileInfo {
                path: "src/a.rs".to_string(),
                size_bytes: 100,
                content_preview: Some("fn load_config() {}".to_string()),
            },
            FileInfo {
                path: "src/b.rs".to_string(),
                size_bytes: 100,
                content_preview: Some("fn main() {}".to_string()),
            },
        ];

        let scored = selector.select_relevant("update config loading", &files, 10);

        // a.rs should be first (content match for "config")
        assert_eq!(scored[0].path, "src/a.rs");
        assert!(scored[0].score > scored[1].score);
    }

    #[test]
    fn test_file_selector_max_files() {
        let selector = FileSelector::new("/project");

        let files: Vec<FileInfo> = (0..10)
            .map(|i| FileInfo {
                path: format!("src/file{}.rs", i),
                size_bytes: 100,
                content_preview: None,
            })
            .collect();

        let scored = selector.select_relevant("test", &files, 3);

        assert_eq!(scored.len(), 3);
    }

    #[test]
    fn test_file_selector_mod_rs_boost() {
        let selector = FileSelector::new("/project");

        let files = vec![
            FileInfo {
                path: "src/utils.rs".to_string(),
                size_bytes: 100,
                content_preview: None,
            },
            FileInfo {
                path: "src/mod.rs".to_string(),
                size_bytes: 100,
                content_preview: None,
            },
        ];

        let scored = selector.select_relevant("unknown task", &files, 10);

        // mod.rs should have higher score due to boost
        let mod_rs = scored.iter().find(|s| s.path == "src/mod.rs").unwrap();
        let utils_rs = scored.iter().find(|s| s.path == "src/utils.rs").unwrap();
        assert!(mod_rs.score > utils_rs.score);
    }

    #[test]
    fn test_scored_file_is_highly_relevant() {
        let high = ScoredFile {
            path: "test.rs".to_string(),
            score: 15.0,
            relevance_reason: "high".to_string(),
        };
        let low = ScoredFile {
            path: "test.rs".to_string(),
            score: 5.0,
            relevance_reason: "low".to_string(),
        };

        assert!(high.is_highly_relevant());
        assert!(!low.is_highly_relevant());
    }

    // ContextRequestHandler tests

    #[test]
    fn test_context_request_handler_specific_file() {
        let mut handler = ContextRequestHandler::new("/project");

        let files = vec![FileInfo {
            path: "src/main.rs".to_string(),
            size_bytes: 100,
            content_preview: Some("fn main() {}".to_string()),
        }];

        handler.add_request(ContextRequest {
            request_type: ContextRequestType::SpecificFile("src/main.rs".to_string()),
            reason: "Need to modify".to_string(),
            priority: RequestPriority::Required,
        });

        let results = handler.fulfill_pending(&files);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].status, FulfillmentStatus::Fulfilled);
        assert!(results[0].content.is_some());
    }

    #[test]
    fn test_context_request_handler_file_not_found() {
        let mut handler = ContextRequestHandler::new("/project");

        let files = vec![FileInfo {
            path: "src/main.rs".to_string(),
            size_bytes: 100,
            content_preview: None,
        }];

        handler.add_request(ContextRequest {
            request_type: ContextRequestType::SpecificFile("src/other.rs".to_string()),
            reason: "Need to check".to_string(),
            priority: RequestPriority::Helpful,
        });

        let results = handler.fulfill_pending(&files);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].status, FulfillmentStatus::NotFound);
    }

    #[test]
    fn test_context_request_handler_files_matching() {
        let mut handler = ContextRequestHandler::new("/project");

        let files = vec![
            FileInfo {
                path: "src/config/mod.rs".to_string(),
                size_bytes: 100,
                content_preview: None,
            },
            FileInfo {
                path: "src/config/loader.rs".to_string(),
                size_bytes: 100,
                content_preview: None,
            },
            FileInfo {
                path: "src/main.rs".to_string(),
                size_bytes: 100,
                content_preview: None,
            },
        ];

        handler.add_request(ContextRequest {
            request_type: ContextRequestType::FilesMatching("config".to_string()),
            reason: "Need config files".to_string(),
            priority: RequestPriority::Required,
        });

        let results = handler.fulfill_pending(&files);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].status, FulfillmentStatus::Fulfilled);
        assert!(results[0].content.as_ref().unwrap().contains("2 files"));
    }

    #[test]
    fn test_context_request_handler_function_definition() {
        let mut handler = ContextRequestHandler::new("/project");

        let files = vec![FileInfo {
            path: "src/utils.rs".to_string(),
            size_bytes: 100,
            content_preview: Some("pub fn calculate_total(items: &[i32]) -> i32 { items.iter().sum() }".to_string()),
        }];

        handler.add_request(ContextRequest {
            request_type: ContextRequestType::FunctionDefinition {
                name: "calculate_total".to_string(),
                file_hint: None,
            },
            reason: "Need to understand function".to_string(),
            priority: RequestPriority::Required,
        });

        let results = handler.fulfill_pending(&files);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].status, FulfillmentStatus::Fulfilled);
        assert!(results[0].content.as_ref().unwrap().contains("calculate_total"));
    }

    #[test]
    fn test_context_request_handler_pending_and_fulfilled() {
        let mut handler = ContextRequestHandler::new("/project");

        handler.add_request(ContextRequest {
            request_type: ContextRequestType::SpecificFile("test.rs".to_string()),
            reason: "test".to_string(),
            priority: RequestPriority::Helpful,
        });

        assert_eq!(handler.pending_requests().len(), 1);
        assert_eq!(handler.fulfilled_history().len(), 0);

        let files = vec![FileInfo {
            path: "test.rs".to_string(),
            size_bytes: 100,
            content_preview: None,
        }];

        handler.fulfill_pending(&files);

        assert_eq!(handler.pending_requests().len(), 0);
        assert_eq!(handler.fulfilled_history().len(), 1);
    }

    // HistoryManager tests

    #[test]
    fn test_history_manager_add_entries() {
        let mut manager = HistoryManager::new(1000);

        manager.add(ConversationEntry {
            role: "user".to_string(),
            content: "Hello".to_string(),
            timestamp: chrono::Utc::now(),
            metadata: None,
        });

        manager.add(ConversationEntry {
            role: "assistant".to_string(),
            content: "Hi there!".to_string(),
            timestamp: chrono::Utc::now(),
            metadata: None,
        });

        assert_eq!(manager.entry_count(), 2);
    }

    #[test]
    fn test_history_manager_budget_limiting() {
        // Small budget that can only fit a few entries
        // Each message is ~50 chars = ~12 tokens, so budget of 30 tokens fits ~2-3 entries
        let mut manager = HistoryManager::new(30);

        // Add entries that exceed budget
        for i in 0..5 {
            manager.add(ConversationEntry {
                role: "user".to_string(),
                content: format!("This is a longer message number {} with more content", i),
                timestamp: chrono::Utc::now(),
                metadata: None,
            });
        }

        let within_budget = manager.get_within_budget();

        // Should not return all entries due to budget
        assert!(within_budget.len() < 5);
    }

    #[test]
    fn test_history_manager_important_entries_preserved() {
        let mut manager = HistoryManager::new(50); // Small budget

        // Add a regular entry
        manager.add(ConversationEntry {
            role: "user".to_string(),
            content: "Regular message".to_string(),
            timestamp: chrono::Utc::now(),
            metadata: None,
        });

        // Mark it as important
        manager.mark_important(0);

        // Add more entries that would push out the first one
        for i in 0..5 {
            manager.add(ConversationEntry {
                role: "user".to_string(),
                content: format!("Later message {}", i),
                timestamp: chrono::Utc::now(),
                metadata: None,
            });
        }

        let within_budget = manager.get_within_budget();

        // The important entry should be preserved
        let has_important = within_budget.iter().any(|e| e.content == "Regular message");
        assert!(has_important);
    }

    #[test]
    fn test_history_manager_auto_important_keywords() {
        let mut manager = HistoryManager::new(1000);

        // Add entry with decision keyword
        manager.add(ConversationEntry {
            role: "assistant".to_string(),
            content: "We decided to use Rust for this project".to_string(),
            timestamp: chrono::Utc::now(),
            metadata: None,
        });

        // Add entry with error keyword
        manager.add(ConversationEntry {
            role: "assistant".to_string(),
            content: "There was an error in the build".to_string(),
            timestamp: chrono::Utc::now(),
            metadata: None,
        });

        // Add regular entry
        manager.add(ConversationEntry {
            role: "user".to_string(),
            content: "What should we do next?".to_string(),
            timestamp: chrono::Utc::now(),
            metadata: None,
        });

        let summary = manager.truncation_summary();

        // Two entries should be auto-marked as important
        assert_eq!(summary.important_count, 2);
    }

    #[test]
    fn test_history_manager_truncation_summary() {
        let mut manager = HistoryManager::new(20);

        for i in 0..10 {
            manager.add(ConversationEntry {
                role: "user".to_string(),
                content: format!("Message {}", i),
                timestamp: chrono::Utc::now(),
                metadata: None,
            });
        }

        let summary = manager.truncation_summary();

        assert_eq!(summary.total_entries, 10);
        assert!(summary.entries_truncated > 0);
        assert_eq!(summary.entries_kept + summary.entries_truncated, summary.total_entries);
    }

    #[test]
    fn test_history_manager_chronological_order() {
        let mut manager = HistoryManager::new(1000);

        manager.add(ConversationEntry {
            role: "user".to_string(),
            content: "First".to_string(),
            timestamp: chrono::Utc::now(),
            metadata: None,
        });

        manager.add(ConversationEntry {
            role: "assistant".to_string(),
            content: "Second".to_string(),
            timestamp: chrono::Utc::now(),
            metadata: None,
        });

        manager.add(ConversationEntry {
            role: "user".to_string(),
            content: "Third".to_string(),
            timestamp: chrono::Utc::now(),
            metadata: None,
        });

        let entries = manager.get_within_budget();

        assert_eq!(entries[0].content, "First");
        assert_eq!(entries[1].content, "Second");
        assert_eq!(entries[2].content, "Third");
    }
}
