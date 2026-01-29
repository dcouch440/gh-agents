//! Assertion helpers for prompt testing.

use super::harness::DecompositionTestResult;

/// Assertion helpers for prompt testing
pub struct PromptAssertions;

impl PromptAssertions {
    /// Assert that output matches expected schema
    pub fn output_matches_schema(result: &DecompositionTestResult) -> AssertionResult {
        if result.parsed_ok() {
            AssertionResult::pass("Output matches schema")
        } else {
            AssertionResult::fail(format!(
                "Output doesn't match schema: {}",
                result
                    .validation_error
                    .as_deref()
                    .unwrap_or("Unknown error")
            ))
        }
    }

    /// Assert that output contains reasoning/thinking
    pub fn contains_reasoning(result: &DecompositionTestResult) -> AssertionResult {
        if let Some(ref parsed) = result.parsed_output {
            if parsed.thinking.is_empty() {
                return AssertionResult::fail("Thinking field is empty");
            }
            if parsed.thinking.len() < 50 {
                return AssertionResult::warn("Thinking is very short - may lack depth");
            }
            AssertionResult::pass("Output contains reasoning")
        } else {
            AssertionResult::skip("Cannot check reasoning - output didn't parse")
        }
    }

    /// Assert that no hallucinated files are referenced
    pub fn no_hallucinated_files(
        result: &DecompositionTestResult,
        known_files: &[&str],
    ) -> AssertionResult {
        if let Some(ref parsed) = result.parsed_output {
            let mut unknown = Vec::new();

            for slice in &parsed.slices {
                for task in &slice.tasks {
                    for file in &task.context_files {
                        if !known_files.iter().any(|kf| file.contains(kf)) {
                            unknown.push(file.clone());
                        }
                    }
                }
            }

            if unknown.is_empty() {
                AssertionResult::pass("No hallucinated files")
            } else {
                AssertionResult::fail(format!("Unknown files referenced: {:?}", unknown))
            }
        } else {
            AssertionResult::skip("Cannot check files - output didn't parse")
        }
    }

    /// Assert that slices are vertical (touch multiple layers)
    pub fn slices_are_vertical(result: &DecompositionTestResult) -> AssertionResult {
        if let Some(ref parsed) = result.parsed_output {
            let mut horizontal_slices = Vec::new();

            for slice in &parsed.slices {
                // Check for horizontal slice patterns
                let title_lower = slice.title.to_lowercase();
                if title_lower.contains("all tests")
                    || title_lower.contains("all db")
                    || title_lower.contains("all api")
                    || title_lower.starts_with("write tests")
                {
                    horizontal_slices.push(slice.title.clone());
                }
            }

            if horizontal_slices.is_empty() {
                AssertionResult::pass("Slices are vertical")
            } else {
                AssertionResult::fail(format!("Found horizontal slices: {:?}", horizontal_slices))
            }
        } else {
            AssertionResult::skip("Cannot check slices - output didn't parse")
        }
    }

    /// Assert minimum number of slices
    pub fn minimum_slices(result: &DecompositionTestResult, min: usize) -> AssertionResult {
        if result.slice_count() >= min {
            AssertionResult::pass(format!("Has {} slices (min {})", result.slice_count(), min))
        } else {
            AssertionResult::fail(format!(
                "Only {} slices, expected at least {}",
                result.slice_count(),
                min
            ))
        }
    }

    /// Assert each slice has acceptance criteria
    pub fn all_slices_have_acceptance_criteria(
        result: &DecompositionTestResult,
    ) -> AssertionResult {
        if let Some(ref parsed) = result.parsed_output {
            let missing: Vec<_> = parsed
                .slices
                .iter()
                .filter(|s| s.acceptance_criteria.is_empty())
                .map(|s| s.title.clone())
                .collect();

            if missing.is_empty() {
                AssertionResult::pass("All slices have acceptance criteria")
            } else {
                AssertionResult::fail(format!("Slices missing acceptance criteria: {:?}", missing))
            }
        } else {
            AssertionResult::skip("Cannot check criteria - output didn't parse")
        }
    }

    /// Assert each slice has at least one task
    pub fn all_slices_have_tasks(result: &DecompositionTestResult) -> AssertionResult {
        if let Some(ref parsed) = result.parsed_output {
            let missing: Vec<_> = parsed
                .slices
                .iter()
                .filter(|s| s.tasks.is_empty())
                .map(|s| s.title.clone())
                .collect();

            if missing.is_empty() {
                AssertionResult::pass("All slices have tasks")
            } else {
                AssertionResult::fail(format!("Slices missing tasks: {:?}", missing))
            }
        } else {
            AssertionResult::skip("Cannot check tasks - output didn't parse")
        }
    }

    /// Assert dependencies are valid (reference existing slices)
    pub fn dependencies_are_valid(result: &DecompositionTestResult) -> AssertionResult {
        if let Some(ref parsed) = result.parsed_output {
            let titles: std::collections::HashSet<_> =
                parsed.slices.iter().map(|s| &s.title).collect();

            let mut invalid = Vec::new();
            for slice in &parsed.slices {
                for dep in &slice.dependencies {
                    if !titles.contains(dep) {
                        invalid.push((slice.title.clone(), dep.clone()));
                    }
                }
            }

            if invalid.is_empty() {
                AssertionResult::pass("All dependencies are valid")
            } else {
                AssertionResult::fail(format!(
                    "Invalid dependencies: {:?}",
                    invalid
                        .iter()
                        .map(|(s, d)| format!("{} -> {}", s, d))
                        .collect::<Vec<_>>()
                ))
            }
        } else {
            AssertionResult::skip("Cannot check dependencies - output didn't parse")
        }
    }
}

/// Result of running an assertion
#[derive(Debug, Clone)]
pub enum AssertionResult {
    Pass(String),
    Fail(String),
    Warn(String),
    Skip(String),
}

impl AssertionResult {
    pub fn pass(msg: impl Into<String>) -> Self {
        Self::Pass(msg.into())
    }

    pub fn fail(msg: impl Into<String>) -> Self {
        Self::Fail(msg.into())
    }

    pub fn warn(msg: impl Into<String>) -> Self {
        Self::Warn(msg.into())
    }

    pub fn skip(msg: impl Into<String>) -> Self {
        Self::Skip(msg.into())
    }

    pub fn is_pass(&self) -> bool {
        matches!(self, Self::Pass(_))
    }

    pub fn is_fail(&self) -> bool {
        matches!(self, Self::Fail(_))
    }

    pub fn is_warn(&self) -> bool {
        matches!(self, Self::Warn(_))
    }

    pub fn is_skip(&self) -> bool {
        matches!(self, Self::Skip(_))
    }

    pub fn message(&self) -> &str {
        match self {
            Self::Pass(m) | Self::Fail(m) | Self::Warn(m) | Self::Skip(m) => m,
        }
    }
}

/// Run multiple assertions and collect results
pub struct AssertionSuite {
    results: Vec<(String, AssertionResult)>,
}

impl AssertionSuite {
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
        }
    }

    pub fn add(&mut self, name: impl Into<String>, result: AssertionResult) {
        self.results.push((name.into(), result));
    }

    pub fn all_passed(&self) -> bool {
        self.results.iter().all(|(_, r)| !r.is_fail())
    }

    pub fn has_failures(&self) -> bool {
        self.results.iter().any(|(_, r)| r.is_fail())
    }

    pub fn failures(&self) -> Vec<&(String, AssertionResult)> {
        self.results.iter().filter(|(_, r)| r.is_fail()).collect()
    }

    #[allow(dead_code)]
    pub fn warnings(&self) -> Vec<&(String, AssertionResult)> {
        self.results.iter().filter(|(_, r)| r.is_warn()).collect()
    }

    pub fn passes(&self) -> Vec<&(String, AssertionResult)> {
        self.results.iter().filter(|(_, r)| r.is_pass()).collect()
    }

    pub fn summary(&self) -> String {
        let passed = self.results.iter().filter(|(_, r)| r.is_pass()).count();
        let failed = self.results.iter().filter(|(_, r)| r.is_fail()).count();
        let warned = self.results.iter().filter(|(_, r)| r.is_warn()).count();
        let skipped = self
            .results
            .iter()
            .filter(|(_, r)| matches!(r, AssertionResult::Skip(_)))
            .count();

        format!(
            "{} passed, {} failed, {} warnings, {} skipped",
            passed, failed, warned, skipped
        )
    }

    pub fn detailed_report(&self) -> String {
        let mut lines = Vec::new();

        for (name, result) in &self.results {
            let status = match result {
                AssertionResult::Pass(_) => "[PASS]",
                AssertionResult::Fail(_) => "[FAIL]",
                AssertionResult::Warn(_) => "[WARN]",
                AssertionResult::Skip(_) => "[SKIP]",
            };
            lines.push(format!("{} {}: {}", status, name, result.message()));
        }

        lines.push(String::new());
        lines.push(self.summary());

        lines.join("\n")
    }
}

impl Default for AssertionSuite {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prompts::harness::PromptOutput;
    use nexor::prompts::schemas::{
        ComplexityOutput, DecompositionOutput, SliceOutput, TaskOutput, TierOutput,
    };

    fn create_valid_result() -> DecompositionTestResult {
        DecompositionTestResult {
            fixture_name: "test".to_string(),
            output: PromptOutput {
                response: "".to_string(),
                input_tokens: 0,
                output_tokens: 0,
                latency_ms: 0,
                used_mock: true,
            },
            parsed_output: Some(DecompositionOutput {
                thinking: "This is detailed thinking about the decomposition task at hand."
                    .to_string(),
                slices: vec![
                    SliceOutput {
                        title: "User model + migration".to_string(),
                        description: "Create users table".to_string(),
                        tasks: vec![TaskOutput {
                            title: "Create migration".to_string(),
                            tier: TierOutput::Worker,
                            estimated_complexity: ComplexityOutput::Low,
                            context_files: vec!["src/db/mod.rs".to_string()],
                        }],
                        dependencies: vec![],
                        acceptance_criteria: vec!["Table exists".to_string()],
                    },
                    SliceOutput {
                        title: "Password hashing".to_string(),
                        description: "Add argon2 hashing".to_string(),
                        tasks: vec![TaskOutput {
                            title: "Add hashing".to_string(),
                            tier: TierOutput::Worker,
                            estimated_complexity: ComplexityOutput::Medium,
                            context_files: vec!["src/auth/mod.rs".to_string()],
                        }],
                        dependencies: vec!["User model + migration".to_string()],
                        acceptance_criteria: vec!["Passwords hashed".to_string()],
                    },
                ],
                questions: vec![],
                risks: vec![],
            }),
            validation_error: None,
            expected: None,
        }
    }

    fn create_unparsed_result() -> DecompositionTestResult {
        DecompositionTestResult {
            fixture_name: "test".to_string(),
            output: PromptOutput {
                response: "invalid".to_string(),
                input_tokens: 0,
                output_tokens: 0,
                latency_ms: 0,
                used_mock: true,
            },
            parsed_output: None,
            validation_error: Some("Parse error".to_string()),
            expected: None,
        }
    }

    #[test]
    fn test_output_matches_schema_pass() {
        let result = create_valid_result();
        let assertion = PromptAssertions::output_matches_schema(&result);
        assert!(assertion.is_pass());
    }

    #[test]
    fn test_output_matches_schema_fail() {
        let result = create_unparsed_result();
        let assertion = PromptAssertions::output_matches_schema(&result);
        assert!(assertion.is_fail());
    }

    #[test]
    fn test_contains_reasoning_pass() {
        let result = create_valid_result();
        let assertion = PromptAssertions::contains_reasoning(&result);
        assert!(assertion.is_pass());
    }

    #[test]
    fn test_contains_reasoning_short_warns() {
        let mut result = create_valid_result();
        if let Some(ref mut parsed) = result.parsed_output {
            parsed.thinking = "Short".to_string();
        }
        let assertion = PromptAssertions::contains_reasoning(&result);
        assert!(assertion.is_warn());
    }

    #[test]
    fn test_contains_reasoning_empty_fails() {
        let mut result = create_valid_result();
        if let Some(ref mut parsed) = result.parsed_output {
            parsed.thinking = String::new();
        }
        let assertion = PromptAssertions::contains_reasoning(&result);
        assert!(assertion.is_fail());
    }

    #[test]
    fn test_contains_reasoning_skip_when_unparsed() {
        let result = create_unparsed_result();
        let assertion = PromptAssertions::contains_reasoning(&result);
        assert!(assertion.is_skip());
    }

    #[test]
    fn test_no_hallucinated_files_pass() {
        let result = create_valid_result();
        let known = vec!["src/db/mod.rs", "src/auth/mod.rs"];
        let assertion = PromptAssertions::no_hallucinated_files(&result, &known);
        assert!(assertion.is_pass());
    }

    #[test]
    fn test_no_hallucinated_files_fail() {
        let result = create_valid_result();
        let known = vec!["src/other.rs"];
        let assertion = PromptAssertions::no_hallucinated_files(&result, &known);
        assert!(assertion.is_fail());
    }

    #[test]
    fn test_slices_are_vertical_pass() {
        let result = create_valid_result();
        let assertion = PromptAssertions::slices_are_vertical(&result);
        assert!(assertion.is_pass());
    }

    #[test]
    fn test_slices_are_vertical_fail() {
        let mut result = create_valid_result();
        if let Some(ref mut parsed) = result.parsed_output {
            parsed.slices[0].title = "Write tests for all modules".to_string();
        }
        let assertion = PromptAssertions::slices_are_vertical(&result);
        assert!(assertion.is_fail());
    }

    #[test]
    fn test_minimum_slices_pass() {
        let result = create_valid_result();
        let assertion = PromptAssertions::minimum_slices(&result, 2);
        assert!(assertion.is_pass());
    }

    #[test]
    fn test_minimum_slices_fail() {
        let result = create_valid_result();
        let assertion = PromptAssertions::minimum_slices(&result, 5);
        assert!(assertion.is_fail());
    }

    #[test]
    fn test_all_slices_have_acceptance_criteria_pass() {
        let result = create_valid_result();
        let assertion = PromptAssertions::all_slices_have_acceptance_criteria(&result);
        assert!(assertion.is_pass());
    }

    #[test]
    fn test_all_slices_have_acceptance_criteria_fail() {
        let mut result = create_valid_result();
        if let Some(ref mut parsed) = result.parsed_output {
            parsed.slices[0].acceptance_criteria = vec![];
        }
        let assertion = PromptAssertions::all_slices_have_acceptance_criteria(&result);
        assert!(assertion.is_fail());
    }

    #[test]
    fn test_all_slices_have_tasks_pass() {
        let result = create_valid_result();
        let assertion = PromptAssertions::all_slices_have_tasks(&result);
        assert!(assertion.is_pass());
    }

    #[test]
    fn test_dependencies_are_valid_pass() {
        let result = create_valid_result();
        let assertion = PromptAssertions::dependencies_are_valid(&result);
        assert!(assertion.is_pass());
    }

    #[test]
    fn test_dependencies_are_valid_fail() {
        let mut result = create_valid_result();
        if let Some(ref mut parsed) = result.parsed_output {
            parsed.slices[1].dependencies = vec!["Nonexistent slice".to_string()];
        }
        let assertion = PromptAssertions::dependencies_are_valid(&result);
        assert!(assertion.is_fail());
    }

    #[test]
    fn test_assertion_result_methods() {
        let pass = AssertionResult::pass("ok");
        assert!(pass.is_pass());
        assert!(!pass.is_fail());
        assert_eq!(pass.message(), "ok");

        let fail = AssertionResult::fail("bad");
        assert!(!fail.is_pass());
        assert!(fail.is_fail());
        assert_eq!(fail.message(), "bad");

        let warn = AssertionResult::warn("caution");
        assert!(warn.is_warn());
        assert_eq!(warn.message(), "caution");

        let skip = AssertionResult::skip("skipped");
        assert!(skip.is_skip());
        assert_eq!(skip.message(), "skipped");
    }

    #[test]
    fn test_assertion_suite_empty() {
        let suite = AssertionSuite::new();
        assert!(suite.all_passed());
        assert!(!suite.has_failures());
        assert!(suite.failures().is_empty());
    }

    #[test]
    fn test_assertion_suite_with_passes() {
        let mut suite = AssertionSuite::new();
        suite.add("test1", AssertionResult::pass("ok"));
        suite.add("test2", AssertionResult::pass("good"));
        assert!(suite.all_passed());
        assert_eq!(suite.passes().len(), 2);
    }

    #[test]
    fn test_assertion_suite_with_failure() {
        let mut suite = AssertionSuite::new();
        suite.add("test1", AssertionResult::pass("ok"));
        suite.add("test2", AssertionResult::fail("bad"));
        assert!(!suite.all_passed());
        assert!(suite.has_failures());
        assert_eq!(suite.failures().len(), 1);
    }

    #[test]
    fn test_assertion_suite_summary() {
        let mut suite = AssertionSuite::new();
        suite.add("p1", AssertionResult::pass("ok"));
        suite.add("p2", AssertionResult::pass("ok"));
        suite.add("f1", AssertionResult::fail("bad"));
        suite.add("w1", AssertionResult::warn("caution"));
        suite.add("s1", AssertionResult::skip("skipped"));

        let summary = suite.summary();
        assert!(summary.contains("2 passed"));
        assert!(summary.contains("1 failed"));
        assert!(summary.contains("1 warnings"));
        assert!(summary.contains("1 skipped"));
    }

    #[test]
    fn test_assertion_suite_detailed_report() {
        let mut suite = AssertionSuite::new();
        suite.add("schema", AssertionResult::pass("Output matches schema"));
        suite.add("reasoning", AssertionResult::fail("Missing thinking"));

        let report = suite.detailed_report();
        assert!(report.contains("[PASS] schema"));
        assert!(report.contains("[FAIL] reasoning"));
    }
}
