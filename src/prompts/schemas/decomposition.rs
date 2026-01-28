//! Decomposition output schema for orchestrator ticket breakdown.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Output from the orchestrator decomposition prompt.
/// This is what the LLM returns when breaking down a ticket into slices.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecompositionOutput {
    /// The agent's reasoning through decomposition steps
    pub thinking: String,

    /// The vertical slices produced
    pub slices: Vec<SliceOutput>,

    /// Clarifying questions if requirements are unclear
    #[serde(default)]
    pub questions: Vec<String>,

    /// Potential risks or unknowns identified
    #[serde(default)]
    pub risks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SliceOutput {
    /// Short descriptive title
    pub title: String,

    /// What this slice accomplishes
    pub description: String,

    /// Tasks within this slice
    pub tasks: Vec<TaskOutput>,

    /// Titles of slices this depends on
    #[serde(default)]
    pub dependencies: Vec<String>,

    /// How to verify this slice is done
    pub acceptance_criteria: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskOutput {
    /// Specific task title
    pub title: String,

    /// Which agent tier should handle this
    pub tier: TierOutput,

    /// Task complexity estimate
    pub estimated_complexity: ComplexityOutput,

    /// Files the agent will need
    #[serde(default)]
    pub context_files: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TierOutput {
    Worker,
    Utility,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ComplexityOutput {
    Low,
    Medium,
    High,
}

impl DecompositionOutput {
    /// Validate the decomposition output
    pub fn validate(&self) -> Result<(), ValidationError> {
        // Must have thinking
        if self.thinking.is_empty() {
            return Err(ValidationError::MissingField("thinking"));
        }

        // Must have at least one slice (unless there are questions)
        if self.slices.is_empty() && self.questions.is_empty() {
            return Err(ValidationError::EmptySlices);
        }

        // Each slice must have acceptance criteria
        for slice in &self.slices {
            if slice.acceptance_criteria.is_empty() {
                return Err(ValidationError::MissingAcceptanceCriteria(
                    slice.title.clone(),
                ));
            }

            // Each slice must have at least one task
            if slice.tasks.is_empty() {
                return Err(ValidationError::EmptyTasks(slice.title.clone()));
            }
        }

        // Check for circular dependencies
        self.check_circular_dependencies()?;

        Ok(())
    }

    fn check_circular_dependencies(&self) -> Result<(), ValidationError> {
        // Build dependency map
        let titles: HashSet<_> = self.slices.iter().map(|s| &s.title).collect();

        for slice in &self.slices {
            for dep in &slice.dependencies {
                if !titles.contains(dep) {
                    return Err(ValidationError::InvalidDependency {
                        slice: slice.title.clone(),
                        dependency: dep.clone(),
                    });
                }
            }
        }

        // Simple cycle detection (could be more sophisticated)
        for slice in &self.slices {
            let mut visited = HashSet::new();
            if self.has_cycle(&slice.title, &mut visited) {
                return Err(ValidationError::CircularDependency(slice.title.clone()));
            }
        }

        Ok(())
    }

    fn has_cycle(&self, title: &str, visited: &mut HashSet<String>) -> bool {
        if visited.contains(title) {
            return true;
        }
        visited.insert(title.to_string());

        if let Some(slice) = self.slices.iter().find(|s| s.title == title) {
            for dep in &slice.dependencies {
                if self.has_cycle(dep, visited) {
                    return true;
                }
            }
        }

        visited.remove(title);
        false
    }
}

/// Validation errors for decomposition output
#[derive(Debug, Clone)]
pub enum ValidationError {
    MissingField(&'static str),
    EmptySlices,
    EmptyTasks(String),
    MissingAcceptanceCriteria(String),
    InvalidDependency { slice: String, dependency: String },
    CircularDependency(String),
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingField(field) => write!(f, "Missing required field: {}", field),
            Self::EmptySlices => write!(f, "No slices produced (and no clarifying questions)"),
            Self::EmptyTasks(slice) => write!(f, "Slice '{}' has no tasks", slice),
            Self::MissingAcceptanceCriteria(slice) => {
                write!(f, "Slice '{}' missing acceptance criteria", slice)
            }
            Self::InvalidDependency { slice, dependency } => {
                write!(
                    f,
                    "Slice '{}' depends on unknown slice '{}'",
                    slice, dependency
                )
            }
            Self::CircularDependency(slice) => {
                write!(
                    f,
                    "Circular dependency detected involving slice '{}'",
                    slice
                )
            }
        }
    }
}

impl std::error::Error for ValidationError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_valid_output() -> DecompositionOutput {
        DecompositionOutput {
            thinking: "Analyzing the ticket...".to_string(),
            slices: vec![SliceOutput {
                title: "Slice 1".to_string(),
                description: "First slice".to_string(),
                tasks: vec![TaskOutput {
                    title: "Task 1".to_string(),
                    tier: TierOutput::Worker,
                    estimated_complexity: ComplexityOutput::Medium,
                    context_files: vec![],
                }],
                dependencies: vec![],
                acceptance_criteria: vec!["Compiles".to_string()],
            }],
            questions: vec![],
            risks: vec![],
        }
    }

    #[test]
    fn test_valid_decomposition() {
        let output = create_valid_output();
        assert!(output.validate().is_ok());
    }

    #[test]
    fn test_missing_thinking() {
        let mut output = create_valid_output();
        output.thinking = String::new();
        let result = output.validate();
        assert!(matches!(
            result,
            Err(ValidationError::MissingField("thinking"))
        ));
    }

    #[test]
    fn test_empty_slices_without_questions() {
        let output = DecompositionOutput {
            thinking: "Thinking...".to_string(),
            slices: vec![],
            questions: vec![],
            risks: vec![],
        };
        let result = output.validate();
        assert!(matches!(result, Err(ValidationError::EmptySlices)));
    }

    #[test]
    fn test_empty_slices_with_questions_is_ok() {
        let output = DecompositionOutput {
            thinking: "Thinking...".to_string(),
            slices: vec![],
            questions: vec!["What are the requirements?".to_string()],
            risks: vec![],
        };
        assert!(output.validate().is_ok());
    }

    #[test]
    fn test_missing_acceptance_criteria() {
        let mut output = create_valid_output();
        output.slices[0].acceptance_criteria = vec![];
        let result = output.validate();
        assert!(matches!(
            result,
            Err(ValidationError::MissingAcceptanceCriteria(_))
        ));
    }

    #[test]
    fn test_empty_tasks() {
        let mut output = create_valid_output();
        output.slices[0].tasks = vec![];
        let result = output.validate();
        assert!(matches!(result, Err(ValidationError::EmptyTasks(_))));
    }

    #[test]
    fn test_invalid_dependency() {
        let mut output = create_valid_output();
        output.slices[0].dependencies = vec!["Nonexistent Slice".to_string()];
        let result = output.validate();
        assert!(matches!(
            result,
            Err(ValidationError::InvalidDependency { .. })
        ));
    }

    #[test]
    fn test_circular_dependency() {
        let output = DecompositionOutput {
            thinking: "Thinking...".to_string(),
            slices: vec![
                SliceOutput {
                    title: "A".to_string(),
                    description: "Slice A".to_string(),
                    tasks: vec![TaskOutput {
                        title: "Task".to_string(),
                        tier: TierOutput::Worker,
                        estimated_complexity: ComplexityOutput::Low,
                        context_files: vec![],
                    }],
                    dependencies: vec!["B".to_string()],
                    acceptance_criteria: vec!["Done".to_string()],
                },
                SliceOutput {
                    title: "B".to_string(),
                    description: "Slice B".to_string(),
                    tasks: vec![TaskOutput {
                        title: "Task".to_string(),
                        tier: TierOutput::Worker,
                        estimated_complexity: ComplexityOutput::Low,
                        context_files: vec![],
                    }],
                    dependencies: vec!["A".to_string()],
                    acceptance_criteria: vec!["Done".to_string()],
                },
            ],
            questions: vec![],
            risks: vec![],
        };
        let result = output.validate();
        assert!(matches!(
            result,
            Err(ValidationError::CircularDependency(_))
        ));
    }

    #[test]
    fn test_json_serialization() {
        let output = create_valid_output();
        let json = serde_json::to_string(&output).unwrap();
        let parsed: DecompositionOutput = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.thinking, output.thinking);
    }

    #[test]
    fn test_tier_serialization() {
        let tier = TierOutput::Worker;
        let json = serde_json::to_string(&tier).unwrap();
        assert_eq!(json, "\"worker\"");

        let tier = TierOutput::Utility;
        let json = serde_json::to_string(&tier).unwrap();
        assert_eq!(json, "\"utility\"");
    }

    #[test]
    fn test_complexity_serialization() {
        let complexity = ComplexityOutput::Low;
        let json = serde_json::to_string(&complexity).unwrap();
        assert_eq!(json, "\"low\"");
    }
}
