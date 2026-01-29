//! Test harness for running prompt tests.

use nexor::prompts::schemas::{DecompositionOutput, OutputValidator};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Test harness for running prompt tests.
pub struct PromptTestHarness {
    fixtures_dir: PathBuf,
    use_real_llm: bool,
    mock_responses: HashMap<String, String>,
}

impl PromptTestHarness {
    /// Create a new test harness with fixtures directory
    pub fn new(fixtures_dir: impl Into<PathBuf>) -> Self {
        Self {
            fixtures_dir: fixtures_dir.into(),
            use_real_llm: false,
            mock_responses: HashMap::new(),
        }
    }

    /// Enable real LLM calls (for integration tests)
    pub fn with_real_llm(mut self) -> Self {
        self.use_real_llm = true;
        self
    }

    /// Add a mock response for a prompt
    pub fn mock_response(mut self, prompt_hash: &str, response: &str) -> Self {
        self.mock_responses
            .insert(prompt_hash.to_string(), response.to_string());
        self
    }

    /// Load a test fixture by name
    pub fn load_fixture(&self, name: &str) -> Result<TestFixture, FixtureError> {
        let path = self.fixtures_dir.join(format!("{}.json", name));
        let content = std::fs::read_to_string(&path).map_err(|e| FixtureError::LoadFailed {
            path: path.clone(),
            error: e.to_string(),
        })?;
        serde_json::from_str(&content).map_err(|e| FixtureError::ParseFailed {
            path,
            error: e.to_string(),
        })
    }

    /// Run a prompt and capture the output
    pub async fn run_prompt(&self, prompt: &str) -> Result<PromptOutput, HarnessError> {
        // Compute prompt hash for mock lookup
        let hash = self.hash_prompt(prompt);

        if let Some(mock) = self.mock_responses.get(&hash) {
            return Ok(PromptOutput {
                response: mock.clone(),
                input_tokens: 0,
                output_tokens: 0,
                latency_ms: 0,
                used_mock: true,
            });
        }

        if !self.use_real_llm {
            return Err(HarnessError::NoMockAvailable(hash));
        }

        // Real LLM call would go here (requires M2)
        // For now, return error indicating real LLM not implemented
        Err(HarnessError::RealLlmNotImplemented)
    }

    /// Run a decomposition prompt test
    #[allow(dead_code)]
    pub async fn test_decomposition(
        &self,
        fixture_name: &str,
    ) -> Result<DecompositionTestResult, HarnessError> {
        let fixture = self.load_fixture(fixture_name)?;

        let prompt = format!(
            "Decompose this ticket:\n\n{}\n\n{}",
            fixture.input,
            fixture.additional_context.clone().unwrap_or_default()
        );

        let output = self.run_prompt(&prompt).await?;

        // Parse and validate output
        let parsed = OutputValidator::validate_decomposition(&output.response);

        let (parsed_output, validation_error) = match parsed {
            Ok(output) => (Some(output), None),
            Err(e) => (None, Some(e.to_string())),
        };

        Ok(DecompositionTestResult {
            fixture_name: fixture_name.to_string(),
            output,
            parsed_output,
            validation_error,
            expected: fixture.expected_output,
        })
    }

    /// Compute a hash for a prompt (for mock matching)
    pub fn hash_prompt(&self, prompt: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        prompt.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}

/// A test fixture with input and expected output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestFixture {
    /// Name/description of the test case
    pub name: String,
    /// The input to the prompt
    pub input: String,
    /// Additional context if needed
    pub additional_context: Option<String>,
    /// Expected output (for comparison)
    pub expected_output: Option<String>,
    /// Tags for filtering tests
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Output captured from running a prompt
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct PromptOutput {
    pub response: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub latency_ms: u64,
    pub used_mock: bool,
}

/// Result of a decomposition test
#[derive(Debug)]
#[allow(dead_code)]
pub struct DecompositionTestResult {
    pub fixture_name: String,
    pub output: PromptOutput,
    pub parsed_output: Option<DecompositionOutput>,
    pub validation_error: Option<String>,
    pub expected: Option<String>,
}

impl DecompositionTestResult {
    /// Check if the output parsed successfully
    pub fn parsed_ok(&self) -> bool {
        self.parsed_output.is_some()
    }

    /// Check if there are slices in the output
    pub fn has_slices(&self) -> bool {
        self.parsed_output
            .as_ref()
            .map(|p| !p.slices.is_empty())
            .unwrap_or(false)
    }

    /// Get the number of slices
    pub fn slice_count(&self) -> usize {
        self.parsed_output
            .as_ref()
            .map(|p| p.slices.len())
            .unwrap_or(0)
    }
}

#[derive(Debug)]
pub enum HarnessError {
    Fixture(FixtureError),
    NoMockAvailable(String),
    RealLlmNotImplemented,
    #[allow(dead_code)]
    LlmError(String),
}

impl From<FixtureError> for HarnessError {
    fn from(e: FixtureError) -> Self {
        HarnessError::Fixture(e)
    }
}

impl std::fmt::Display for HarnessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Fixture(e) => write!(f, "{}", e),
            Self::NoMockAvailable(hash) => {
                write!(f, "No mock available for prompt hash: {}", hash)
            }
            Self::RealLlmNotImplemented => write!(f, "Real LLM not implemented yet (requires M2)"),
            Self::LlmError(e) => write!(f, "LLM error: {}", e),
        }
    }
}

impl std::error::Error for HarnessError {}

#[derive(Debug)]
pub enum FixtureError {
    LoadFailed { path: PathBuf, error: String },
    ParseFailed { path: PathBuf, error: String },
}

impl std::fmt::Display for FixtureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LoadFailed { path, error } => {
                write!(f, "Failed to load fixture {:?}: {}", path, error)
            }
            Self::ParseFailed { path, error } => {
                write!(f, "Failed to parse fixture {:?}: {}", path, error)
            }
        }
    }
}

impl std::error::Error for FixtureError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_harness_creation() {
        let harness = PromptTestHarness::new("tests/prompts/fixtures");
        assert!(!harness.use_real_llm);
        assert!(harness.mock_responses.is_empty());
    }

    #[test]
    fn test_harness_with_real_llm() {
        let harness = PromptTestHarness::new("fixtures").with_real_llm();
        assert!(harness.use_real_llm);
    }

    #[test]
    fn test_harness_with_mock() {
        let harness =
            PromptTestHarness::new("fixtures").mock_response("abc123", r#"{"test": true}"#);
        assert!(harness.mock_responses.contains_key("abc123"));
    }

    #[test]
    fn test_hash_prompt_consistent() {
        let harness = PromptTestHarness::new("fixtures");
        let hash1 = harness.hash_prompt("Hello");
        let hash2 = harness.hash_prompt("Hello");
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hash_prompt_different() {
        let harness = PromptTestHarness::new("fixtures");
        let hash1 = harness.hash_prompt("Hello");
        let hash2 = harness.hash_prompt("World");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_fixture_load_missing() {
        let harness = PromptTestHarness::new("tests/prompts/fixtures/nonexistent");
        let result = harness.load_fixture("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_decomposition_result_no_output() {
        let result = DecompositionTestResult {
            fixture_name: "test".to_string(),
            output: PromptOutput {
                response: "".to_string(),
                input_tokens: 0,
                output_tokens: 0,
                latency_ms: 0,
                used_mock: true,
            },
            parsed_output: None,
            validation_error: Some("Parse error".to_string()),
            expected: None,
        };
        assert!(!result.parsed_ok());
        assert!(!result.has_slices());
        assert_eq!(result.slice_count(), 0);
    }

    #[tokio::test]
    async fn test_run_prompt_with_mock() {
        let harness = PromptTestHarness::new("fixtures")
            .mock_response("fixture-test", r#"{"response": "ok"}"#);

        // Use a prompt that hashes to the mock key
        let hash = harness.hash_prompt("test prompt");
        let harness = harness.mock_response(&hash, r#"{"response": "ok"}"#);

        let result = harness.run_prompt("test prompt").await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.used_mock);
        assert_eq!(output.response, r#"{"response": "ok"}"#);
    }

    #[tokio::test]
    async fn test_run_prompt_no_mock() {
        let harness = PromptTestHarness::new("fixtures");
        let result = harness.run_prompt("unmocked prompt").await;
        assert!(matches!(result, Err(HarnessError::NoMockAvailable(_))));
    }
}
