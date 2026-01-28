//! Example selection logic for picking relevant examples based on task characteristics.

use super::*;

/// Selects relevant examples based on task characteristics.
pub struct ExampleSelector {
    max_examples: usize,
}

impl Default for ExampleSelector {
    fn default() -> Self {
        Self { max_examples: 2 }
    }
}

impl ExampleSelector {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set maximum number of examples to return
    pub fn with_max(mut self, max: usize) -> Self {
        self.max_examples = max;
        self
    }

    /// Select decomposition examples for a task
    pub fn select_decomposition(&self, task_description: &str) -> Vec<Example> {
        let mut examples = DecompositionExamples::for_task(task_description);
        examples.truncate(self.max_examples);
        examples
    }

    /// Select implementation examples for a task
    pub fn select_implementation(
        &self,
        task_description: &str,
        needs_context: bool,
    ) -> Vec<Example> {
        let all = ImplementationExamples::all();
        let mut selected = Vec::new();

        // If task needs context, include context request example
        if needs_context {
            if let Some(ex) = all.iter().find(|e| e.title.contains("Context")) {
                selected.push(ex.clone());
            }
        }

        // Add general implementation example
        selected.extend(self.match_keywords(task_description, &all));

        // Always include self-check example (important pattern)
        if let Some(ex) = all.iter().find(|e| e.title.contains("Self-Check")) {
            if !selected.iter().any(|s| s.title == ex.title) {
                selected.push(ex.clone());
            }
        }

        selected.truncate(self.max_examples);
        selected
    }

    /// Select review examples
    pub fn select_review(&self, has_issues: bool) -> Vec<Example> {
        let all = ReviewExamples::all();

        if has_issues {
            // Prioritize changes_requested example
            all.into_iter()
                .filter(|e| e.title.contains("Changes") || e.title.contains("Escalat"))
                .take(self.max_examples)
                .collect()
        } else {
            // Include approval example
            all.into_iter().take(self.max_examples).collect()
        }
    }

    /// Select recovery examples
    pub fn select_recovery(&self, attempt_count: u32) -> Vec<Example> {
        let all = RecoveryExamples::all();

        if attempt_count >= 3 {
            // Prioritize escalation example
            all.into_iter()
                .filter(|e| e.title.contains("Escalat"))
                .take(self.max_examples)
                .collect()
        } else {
            // Prioritize successful recovery example
            all.into_iter()
                .filter(|e| e.title.contains("Successful"))
                .take(self.max_examples)
                .collect()
        }
    }

    /// Match examples by keyword relevance
    fn match_keywords(&self, task: &str, examples: &[Example]) -> Vec<Example> {
        let task_lower = task.to_lowercase();
        let mut scored: Vec<(usize, &Example)> = examples
            .iter()
            .map(|ex| {
                let score = ex
                    .keywords
                    .iter()
                    .filter(|kw| task_lower.contains(*kw))
                    .count();
                (score, ex)
            })
            .filter(|(score, _)| *score > 0)
            .collect();

        scored.sort_by(|a, b| b.0.cmp(&a.0));
        scored.into_iter().map(|(_, ex)| ex.clone()).collect()
    }
}

/// Format multiple examples for prompt injection
pub fn format_examples(examples: &[Example]) -> String {
    if examples.is_empty() {
        return String::new();
    }

    let formatted: Vec<String> = examples.iter().map(|ex| ex.format_for_prompt()).collect();

    format!("## Examples\n\n{}", formatted.join("\n\n---\n\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_max_is_two() {
        let selector = ExampleSelector::new();
        assert_eq!(selector.max_examples, 2);
    }

    #[test]
    fn with_max_changes_limit() {
        let selector = ExampleSelector::new().with_max(5);
        assert_eq!(selector.max_examples, 5);
    }

    #[test]
    fn decomposition_selection() {
        let selector = ExampleSelector::new();
        let examples = selector.select_decomposition("Add user authentication");
        assert!(!examples.is_empty());
        assert!(examples.len() <= 2);
    }

    #[test]
    fn implementation_includes_self_check() {
        let selector = ExampleSelector::new().with_max(3);
        let examples = selector.select_implementation("Add a function", false);
        assert!(examples.iter().any(|e| e.title.contains("Self-Check")));
    }

    #[test]
    fn implementation_with_context_includes_context_example() {
        let selector = ExampleSelector::new().with_max(3);
        let examples = selector.select_implementation("Add a function", true);
        assert!(examples.iter().any(|e| e.title.contains("Context")));
    }

    #[test]
    fn review_with_issues_prioritizes_changes() {
        let selector = ExampleSelector::new();
        let examples = selector.select_review(true);
        assert!(examples
            .iter()
            .any(|e| e.title.contains("Changes") || e.title.contains("Escalat")));
    }

    #[test]
    fn review_without_issues_includes_approval() {
        let selector = ExampleSelector::new();
        let examples = selector.select_review(false);
        assert!(examples.iter().any(|e| e.title.contains("Approv")));
    }

    #[test]
    fn recovery_before_three_attempts_suggests_retry() {
        let selector = ExampleSelector::new();

        let examples = selector.select_recovery(2);
        assert!(examples.iter().any(|e| e.title.contains("Successful")));
    }

    #[test]
    fn recovery_after_three_attempts_suggests_escalation() {
        let selector = ExampleSelector::new();

        let examples = selector.select_recovery(3);
        assert!(examples.iter().any(|e| e.title.contains("Escalat")));
    }

    #[test]
    fn format_examples_empty_returns_empty() {
        let result = format_examples(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn format_examples_includes_header() {
        let examples = vec![DecompositionExamples::all().remove(0)];
        let result = format_examples(&examples);

        assert!(result.starts_with("## Examples"));
    }

    #[test]
    fn format_examples_joins_with_separator() {
        let examples = DecompositionExamples::all();
        let result = format_examples(&examples[0..2]);

        assert!(result.contains("---"));
    }
}
