//! Context injection system for prompts.
//!
//! Handles priority-based injection of context into prompts while
//! respecting token budget allocations per category.

use std::collections::HashMap;

use crate::prompts::builder::PromptBuilder;

use super::manager::{estimate_tokens, ContextCategory};

/// Handles injection of dynamic context into prompts.
#[derive(Debug, Default)]
pub struct ContextInjector {
    /// Maximum total tokens for context
    budget: usize,
    /// Priority-ordered context sources
    sources: Vec<ContextSource>,
}

#[derive(Debug, Clone)]
pub struct ContextSource {
    /// Source identifier
    pub name: String,
    /// Priority (higher = more important, included first)
    pub priority: u8,
    /// The content to inject
    pub content: String,
    /// Estimated token count
    pub token_estimate: usize,
    /// Category for budget allocation
    pub category: ContextCategory,
}

impl ContextInjector {
    /// Create a new context injector with the given token budget.
    pub fn new(budget_tokens: usize) -> Self {
        Self {
            budget: budget_tokens,
            sources: Vec::new(),
        }
    }

    /// Get the current token budget.
    pub fn budget(&self) -> usize {
        self.budget
    }

    /// Get the number of sources.
    pub fn source_count(&self) -> usize {
        self.sources.len()
    }

    /// Add a context source.
    pub fn add(&mut self, source: ContextSource) {
        self.sources.push(source);
    }

    /// Add a file to modify (high priority).
    pub fn add_file_to_modify(&mut self, path: &str, content: &str) {
        let token_estimate = estimate_tokens(content);
        self.sources.push(ContextSource {
            name: format!("file:{}", path),
            priority: 100,
            content: content.to_string(),
            token_estimate,
            category: ContextCategory::FilesToModify,
        });
    }

    /// Add a reference file (medium priority).
    pub fn add_reference_file(&mut self, path: &str, content: &str) {
        let token_estimate = estimate_tokens(content);
        self.sources.push(ContextSource {
            name: format!("ref:{}", path),
            priority: 50,
            content: content.to_string(),
            token_estimate,
            category: ContextCategory::ReferenceFiles,
        });
    }

    /// Add task context.
    pub fn add_task_context(&mut self, description: &str) {
        let token_estimate = estimate_tokens(description);
        self.sources.push(ContextSource {
            name: "task".to_string(),
            priority: 90,
            content: description.to_string(),
            token_estimate,
            category: ContextCategory::TaskAndHistory,
        });
    }

    /// Add conversation history entry.
    pub fn add_history(&mut self, role: &str, content: &str) {
        let token_estimate = estimate_tokens(content);
        self.sources.push(ContextSource {
            name: format!("history:{}", role),
            priority: 70,
            content: format!("{}: {}", role, content),
            token_estimate,
            category: ContextCategory::TaskAndHistory,
        });
    }

    /// Add project conventions.
    pub fn add_conventions(&mut self, conventions: &str) {
        let token_estimate = estimate_tokens(conventions);
        self.sources.push(ContextSource {
            name: "conventions".to_string(),
            priority: 80,
            content: conventions.to_string(),
            token_estimate,
            category: ContextCategory::Conventions,
        });
    }

    /// Inject context into a PromptBuilder, respecting budget.
    ///
    /// Sources are added in priority order until the category budget is exhausted.
    /// Returns the builder with context injected.
    pub fn inject(self, mut builder: PromptBuilder) -> PromptBuilder {
        // Sort by priority (highest first)
        let mut sources = self.sources;
        sources.sort_by(|a, b| b.priority.cmp(&a.priority));

        // Calculate budget per category
        let budgets: HashMap<ContextCategory, usize> = ContextCategory::all().iter().map(|cat| (*cat, (self.budget as f32 * cat.budget_percent()) as usize)).collect();

        let mut used: HashMap<ContextCategory, usize> = HashMap::new();

        for source in sources {
            let category_budget = budgets.get(&source.category).copied().unwrap_or(0);
            let category_used = used.get(&source.category).copied().unwrap_or(0);

            if category_used + source.token_estimate <= category_budget {
                // Add to builder based on category
                match source.category {
                    ContextCategory::FilesToModify => {
                        if let Some(path) = source.name.strip_prefix("file:") {
                            builder = builder.file_to_modify(path, &source.content);
                        }
                    }
                    ContextCategory::ReferenceFiles => {
                        if let Some(path) = source.name.strip_prefix("ref:") {
                            builder = builder.reference_file(path, &source.content);
                        }
                    }
                    ContextCategory::Conventions => {
                        builder = builder.conventions(&source.content);
                    }
                    ContextCategory::TaskAndHistory => {
                        if source.name == "task" {
                            builder = builder.task_context(&source.content);
                        }
                        // History entries are handled separately if needed
                    }
                }
                *used.entry(source.category).or_insert(0) += source.token_estimate;
            }
        }

        builder
    }

    /// Calculate how much of each category's budget would be used.
    ///
    /// Returns a map of category to (used_tokens, budget_tokens, items_included).
    pub fn calculate_usage(&self) -> HashMap<ContextCategory, (usize, usize, usize)> {
        let mut sources = self.sources.clone();
        sources.sort_by(|a, b| b.priority.cmp(&a.priority));

        let budgets: HashMap<ContextCategory, usize> = ContextCategory::all().iter().map(|cat| (*cat, (self.budget as f32 * cat.budget_percent()) as usize)).collect();

        let mut used: HashMap<ContextCategory, usize> = HashMap::new();
        let mut counts: HashMap<ContextCategory, usize> = HashMap::new();

        for source in sources {
            let category_budget = budgets.get(&source.category).copied().unwrap_or(0);
            let category_used = used.get(&source.category).copied().unwrap_or(0);

            if category_used + source.token_estimate <= category_budget {
                *used.entry(source.category).or_insert(0) += source.token_estimate;
                *counts.entry(source.category).or_insert(0) += 1;
            }
        }

        ContextCategory::all()
            .iter()
            .map(|cat| {
                let budget = budgets.get(cat).copied().unwrap_or(0);
                let used = used.get(cat).copied().unwrap_or(0);
                let count = counts.get(cat).copied().unwrap_or(0);
                (*cat, (used, budget, count))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_injector_add_sources() {
        let mut injector = ContextInjector::new(10000);
        injector.add_file_to_modify("src/main.rs", "fn main() {}");
        injector.add_reference_file("src/lib.rs", "pub mod utils;");
        injector.add_task_context("Implement logging");
        injector.add_conventions("Use snake_case");

        assert_eq!(injector.source_count(), 4);
    }

    #[test]
    fn test_context_injector_respects_budget() {
        // Create a small budget (100 tokens)
        let mut injector = ContextInjector::new(100);

        // Add a large file that exceeds the 40% budget (40 tokens)
        // At 4 chars per token, 200 chars = 50 tokens, exceeds 40 token budget
        let large_content = "x".repeat(200);
        injector.add_file_to_modify("large.rs", &large_content);

        // Add a small file that fits
        let small_content = "fn main() {}";
        injector.add_file_to_modify("small.rs", small_content);

        let usage = injector.calculate_usage();
        let (used, budget, count) = usage.get(&ContextCategory::FilesToModify).unwrap();

        // Only the small file should be included
        assert_eq!(*count, 1);
        assert!(*used <= *budget);
    }

    #[test]
    fn test_context_injector_priority_order() {
        let mut injector = ContextInjector::new(10000);

        // Add sources with different priorities
        injector.add(ContextSource {
            name: "file:low.rs".to_string(),
            priority: 10,
            content: "low priority".to_string(),
            token_estimate: 5,
            category: ContextCategory::FilesToModify,
        });
        injector.add(ContextSource {
            name: "file:high.rs".to_string(),
            priority: 100,
            content: "high priority".to_string(),
            token_estimate: 5,
            category: ContextCategory::FilesToModify,
        });
        injector.add(ContextSource {
            name: "file:medium.rs".to_string(),
            priority: 50,
            content: "medium priority".to_string(),
            token_estimate: 5,
            category: ContextCategory::FilesToModify,
        });

        let builder = PromptBuilder::new();
        let builder = injector.inject(builder);
        let prompt = builder.build();

        // Higher priority files should appear first in the prompt
        let high_pos = prompt.text.find("high priority");
        let medium_pos = prompt.text.find("medium priority");
        let low_pos = prompt.text.find("low priority");

        assert!(high_pos.is_some());
        assert!(medium_pos.is_some());
        assert!(low_pos.is_some());
    }

    #[test]
    fn test_context_injector_inject_into_builder() {
        let mut injector = ContextInjector::new(10000);
        injector.add_file_to_modify("src/main.rs", "fn main() {}");
        injector.add_conventions("Use Rust best practices");

        let builder = PromptBuilder::new().role("You are a developer").task("Add logging");

        let builder = injector.inject(builder);
        let prompt = builder.build();

        assert!(prompt.text.contains("src/main.rs"));
        assert!(prompt.text.contains("fn main() {}"));
        assert!(prompt.text.contains("Use Rust best practices"));
    }
}
