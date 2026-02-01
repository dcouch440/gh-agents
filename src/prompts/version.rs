//! Prompt versioning and execution tracking.
//!
//! Provides version tracking for prompts to correlate LLM outputs
//! with the prompts that generated them, enabling debugging and replay.

use std::collections::HashMap;
use std::fmt;

/// Tracks the version of a prompt template.
/// Used to correlate LLM outputs with the prompt that generated them.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PromptVersion {
    /// The prompt family (e.g., "decomposition", "implementation", "review")
    pub family: String,
    /// Major version - breaking changes to output format
    pub major: u32,
    /// Minor version - new capabilities, backward compatible
    pub minor: u32,
    /// Patch version - bug fixes, wording improvements
    pub patch: u32,
    /// Optional commit hash for exact tracking
    pub commit: Option<String>,
}

impl PromptVersion {
    pub fn new(family: impl Into<String>, major: u32, minor: u32, patch: u32) -> Self {
        Self {
            family: family.into(),
            major,
            minor,
            patch,
            commit: None,
        }
    }

    pub fn with_commit(mut self, commit: impl Into<String>) -> Self {
        self.commit = Some(commit.into());
        self
    }

    /// Check if this version is compatible with another (same major version)
    pub fn is_compatible_with(&self, other: &PromptVersion) -> bool {
        self.family == other.family && self.major == other.major
    }

    /// Check if this version is newer than another
    pub fn is_newer_than(&self, other: &PromptVersion) -> bool {
        if self.family != other.family {
            return false;
        }
        (self.major, self.minor, self.patch) > (other.major, other.minor, other.patch)
    }

    /// Get a unique identifier for this version
    pub fn id(&self) -> String {
        match &self.commit {
            Some(c) => format!("{}-{}.{}.{}-{}", self.family, self.major, self.minor, self.patch, &c[..7.min(c.len())]),
            None => format!("{}-{}.{}.{}", self.family, self.major, self.minor, self.patch),
        }
    }

    /// Get just the semver portion (without family or commit)
    pub fn semver(&self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl fmt::Display for PromptVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.id())
    }
}

/// Registry of known prompt versions.
/// Used to look up prompts by version ID.
#[derive(Debug, Default)]
pub struct PromptRegistry {
    versions: HashMap<String, RegisteredPrompt>,
}

#[derive(Debug, Clone)]
pub struct RegisteredPrompt {
    pub version: PromptVersion,
    pub description: String,
    pub changelog: Option<String>,
    pub deprecated: bool,
}

impl RegisteredPrompt {
    pub fn new(version: PromptVersion, description: impl Into<String>) -> Self {
        Self {
            version,
            description: description.into(),
            changelog: None,
            deprecated: false,
        }
    }

    pub fn with_changelog(mut self, changelog: impl Into<String>) -> Self {
        self.changelog = Some(changelog.into());
        self
    }

    pub fn deprecated(mut self) -> Self {
        self.deprecated = true;
        self
    }
}

impl PromptRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a prompt version
    pub fn register(&mut self, prompt: RegisteredPrompt) {
        self.versions.insert(prompt.version.id(), prompt);
    }

    /// Look up a prompt by version ID
    pub fn get(&self, version_id: &str) -> Option<&RegisteredPrompt> {
        self.versions.get(version_id)
    }

    /// Check if a version ID is registered
    pub fn contains(&self, version_id: &str) -> bool {
        self.versions.contains_key(version_id)
    }

    /// Get all registered prompts
    pub fn all(&self) -> impl Iterator<Item = &RegisteredPrompt> {
        self.versions.values()
    }

    /// Get all versions for a prompt family
    pub fn versions_for_family(&self, family: &str) -> Vec<&RegisteredPrompt> {
        self.versions.values().filter(|p| p.version.family == family).collect()
    }

    /// Get the latest version for a prompt family (non-deprecated)
    pub fn latest(&self, family: &str) -> Option<&RegisteredPrompt> {
        self.versions_for_family(family)
            .into_iter()
            .filter(|p| !p.deprecated)
            .max_by(|a, b| (a.version.major, a.version.minor, a.version.patch).cmp(&(b.version.major, b.version.minor, b.version.patch)))
    }

    /// Get all prompt families
    pub fn families(&self) -> Vec<String> {
        let mut families: Vec<_> = self.versions.values().map(|p| p.version.family.clone()).collect();
        families.sort();
        families.dedup();
        families
    }

    /// Count of registered prompts
    pub fn len(&self) -> usize {
        self.versions.len()
    }

    /// Check if registry is empty
    pub fn is_empty(&self) -> bool {
        self.versions.is_empty()
    }
}

/// Record of a prompt execution for debugging/replay
#[derive(Debug, Clone)]
pub struct PromptExecution {
    /// Unique execution ID
    pub id: String,
    /// The prompt version used
    pub version: PromptVersion,
    /// Task ID this was for (if applicable)
    pub task_id: Option<String>,
    /// The full prompt text sent
    pub prompt_text: String,
    /// The response received
    pub response: Option<String>,
    /// Token counts
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Whether the output parsed successfully
    pub parse_success: bool,
    /// Error message if parsing failed
    pub parse_error: Option<String>,
}

impl PromptExecution {
    pub fn new(version: PromptVersion, prompt_text: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            version,
            task_id: None,
            prompt_text,
            response: None,
            input_tokens: None,
            output_tokens: None,
            timestamp: chrono::Utc::now(),
            parse_success: false,
            parse_error: None,
        }
    }

    pub fn with_task(mut self, task_id: impl Into<String>) -> Self {
        self.task_id = Some(task_id.into());
        self
    }

    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    pub fn record_response(&mut self, response: String, input_tokens: u32, output_tokens: u32) {
        self.response = Some(response);
        self.input_tokens = Some(input_tokens);
        self.output_tokens = Some(output_tokens);
    }

    pub fn record_parse_result(&mut self, success: bool, error: Option<String>) {
        self.parse_success = success;
        self.parse_error = error;
    }

    /// Get total tokens used (input + output)
    pub fn total_tokens(&self) -> Option<u32> {
        match (self.input_tokens, self.output_tokens) {
            (Some(i), Some(o)) => Some(i + o),
            _ => None,
        }
    }

    /// Check if execution completed (has response)
    pub fn is_complete(&self) -> bool {
        self.response.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_version_creation() {
        let version = PromptVersion::new("decomposition", 1, 2, 3);
        assert_eq!(version.family, "decomposition");
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 2);
        assert_eq!(version.patch, 3);
        assert!(version.commit.is_none());
    }

    #[test]
    fn test_prompt_version_with_commit() {
        let version = PromptVersion::new("review", 1, 0, 0).with_commit("abc123def456");
        assert_eq!(version.commit, Some("abc123def456".to_string()));
        assert_eq!(version.id(), "review-1.0.0-abc123d");
    }

    #[test]
    fn test_prompt_version_id() {
        let version = PromptVersion::new("implementation", 2, 1, 0);
        assert_eq!(version.id(), "implementation-2.1.0");
    }

    #[test]
    fn test_prompt_version_semver() {
        let version = PromptVersion::new("test", 1, 2, 3);
        assert_eq!(version.semver(), "1.2.3");
    }

    #[test]
    fn test_prompt_version_compatibility() {
        let v1 = PromptVersion::new("decomposition", 1, 0, 0);
        let v2 = PromptVersion::new("decomposition", 1, 1, 0);
        let v3 = PromptVersion::new("decomposition", 2, 0, 0);
        let v4 = PromptVersion::new("review", 1, 0, 0);

        assert!(v1.is_compatible_with(&v2)); // Same major
        assert!(v2.is_compatible_with(&v1));
        assert!(!v1.is_compatible_with(&v3)); // Different major
        assert!(!v1.is_compatible_with(&v4)); // Different family
    }

    #[test]
    fn test_prompt_version_newer_than() {
        let v1 = PromptVersion::new("test", 1, 0, 0);
        let v2 = PromptVersion::new("test", 1, 1, 0);
        let v3 = PromptVersion::new("test", 2, 0, 0);

        assert!(v2.is_newer_than(&v1));
        assert!(v3.is_newer_than(&v2));
        assert!(!v1.is_newer_than(&v2));
    }

    #[test]
    fn test_prompt_registry() {
        let mut registry = PromptRegistry::new();

        registry.register(RegisteredPrompt::new(PromptVersion::new("decomposition", 1, 0, 0), "Initial decomposition prompt"));

        registry.register(RegisteredPrompt::new(PromptVersion::new("decomposition", 1, 1, 0), "Improved decomposition prompt"));

        registry.register(RegisteredPrompt::new(PromptVersion::new("decomposition", 1, 2, 0), "Old version").deprecated());

        assert_eq!(registry.len(), 3);
        assert!(registry.contains("decomposition-1.0.0"));
        assert!(!registry.contains("decomposition-2.0.0"));
    }

    #[test]
    fn test_prompt_registry_latest() {
        let mut registry = PromptRegistry::new();

        registry.register(RegisteredPrompt::new(PromptVersion::new("test", 1, 0, 0), "v1"));
        registry.register(RegisteredPrompt::new(PromptVersion::new("test", 1, 1, 0), "v1.1"));
        registry.register(RegisteredPrompt::new(PromptVersion::new("test", 2, 0, 0), "v2").deprecated());

        let latest = registry.latest("test").unwrap();
        assert_eq!(latest.version.semver(), "1.1.0"); // v2 is deprecated
    }

    #[test]
    fn test_prompt_registry_families() {
        let mut registry = PromptRegistry::new();

        registry.register(RegisteredPrompt::new(PromptVersion::new("decomposition", 1, 0, 0), "decomp"));
        registry.register(RegisteredPrompt::new(PromptVersion::new("review", 1, 0, 0), "review"));

        let families = registry.families();
        assert_eq!(families, vec!["decomposition", "review"]);
    }

    #[test]
    fn test_prompt_execution() {
        let version = PromptVersion::new("test", 1, 0, 0);
        let mut execution = PromptExecution::new(version, "Test prompt".to_string()).with_task("task-123");

        assert!(!execution.is_complete());
        assert_eq!(execution.task_id, Some("task-123".to_string()));

        execution.record_response("Test response".to_string(), 100, 50);
        assert!(execution.is_complete());
        assert_eq!(execution.total_tokens(), Some(150));

        execution.record_parse_result(true, None);
        assert!(execution.parse_success);
    }

    #[test]
    fn test_prompt_execution_parse_failure() {
        let version = PromptVersion::new("test", 1, 0, 0);
        let mut execution = PromptExecution::new(version, "Test prompt".to_string());

        execution.record_response("Invalid JSON".to_string(), 100, 20);
        execution.record_parse_result(false, Some("Expected JSON".to_string()));

        assert!(!execution.parse_success);
        assert_eq!(execution.parse_error, Some("Expected JSON".to_string()));
    }
}
