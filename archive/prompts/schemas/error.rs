//! Error output schema for agent error/failure reporting.

use serde::{Deserialize, Serialize};

/// Output when an agent encounters an error or failure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorOutput {
    /// Type of error encountered
    pub error_type: ErrorType,

    /// Human-readable error message
    pub message: String,

    /// Attempted recovery (if any)
    #[serde(default)]
    pub attempted_recovery: Option<String>,

    /// Whether this needs human intervention
    pub needs_human: bool,

    /// Suggested next steps
    #[serde(default)]
    pub suggestions: Vec<String>,

    /// Technical details for debugging
    #[serde(default)]
    pub debug_info: Option<DebugInfo>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ErrorType {
    /// Error understanding the task
    TaskUnclear,
    /// Missing required context/files
    MissingContext,
    /// Code won't compile
    CompileError,
    /// Tests are failing
    TestFailure,
    /// External service error (API, git, etc.)
    ExternalError,
    /// Agent is stuck/confused
    Stuck,
    /// Output parsing failed
    ParseError,
    /// Unknown/unexpected error
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugInfo {
    /// Stack trace or error chain
    #[serde(default)]
    pub trace: Option<String>,

    /// Relevant file
    #[serde(default)]
    pub file: Option<String>,

    /// Line number
    #[serde(default)]
    pub line: Option<u32>,

    /// Raw error output
    #[serde(default)]
    pub raw_output: Option<String>,
}

impl ErrorOutput {
    /// Create an error output for unclear tasks
    pub fn task_unclear(message: impl Into<String>) -> Self {
        Self {
            error_type: ErrorType::TaskUnclear,
            message: message.into(),
            attempted_recovery: None,
            needs_human: true,
            suggestions: vec!["Clarify the task requirements".to_string()],
            debug_info: None,
        }
    }

    /// Create an error output for missing context
    pub fn missing_context(message: impl Into<String>, needed_files: Vec<String>) -> Self {
        Self {
            error_type: ErrorType::MissingContext,
            message: message.into(),
            attempted_recovery: None,
            needs_human: false,
            suggestions: needed_files
                .into_iter()
                .map(|f| format!("Provide file: {}", f))
                .collect(),
            debug_info: None,
        }
    }

    /// Create an error output for compile errors
    pub fn compile_error(
        message: impl Into<String>,
        file: Option<String>,
        line: Option<u32>,
    ) -> Self {
        Self {
            error_type: ErrorType::CompileError,
            message: message.into(),
            attempted_recovery: None,
            needs_human: false,
            suggestions: vec!["Fix the compilation error and retry".to_string()],
            debug_info: Some(DebugInfo {
                trace: None,
                file,
                line,
                raw_output: None,
            }),
        }
    }

    /// Create an error output for being stuck
    pub fn stuck(message: impl Into<String>, attempts: u32) -> Self {
        Self {
            error_type: ErrorType::Stuck,
            message: message.into(),
            attempted_recovery: Some(format!("Tried {} times", attempts)),
            needs_human: attempts >= 3,
            suggestions: if attempts >= 3 {
                vec!["Human review needed".to_string()]
            } else {
                vec![
                    "Try a different approach".to_string(),
                    "Escalate to higher tier".to_string(),
                ]
            },
            debug_info: None,
        }
    }

    /// Validate the error output
    pub fn validate(&self) -> Result<(), ErrorValidationError> {
        if self.message.is_empty() {
            return Err(ErrorValidationError::EmptyMessage);
        }

        // If needs human, should have suggestions
        if self.needs_human && self.suggestions.is_empty() {
            return Err(ErrorValidationError::NeedsHumanWithoutSuggestions);
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum ErrorValidationError {
    EmptyMessage,
    NeedsHumanWithoutSuggestions,
}

impl std::fmt::Display for ErrorValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyMessage => write!(f, "Error message cannot be empty"),
            Self::NeedsHumanWithoutSuggestions => {
                write!(f, "needs_human is true but no suggestions provided")
            }
        }
    }
}

impl std::error::Error for ErrorValidationError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_valid_error() -> ErrorOutput {
        ErrorOutput {
            error_type: ErrorType::CompileError,
            message: "Failed to compile".to_string(),
            attempted_recovery: None,
            needs_human: false,
            suggestions: vec!["Fix the error".to_string()],
            debug_info: None,
        }
    }

    #[test]
    fn test_valid_error() {
        let error = create_valid_error();
        assert!(error.validate().is_ok());
    }

    #[test]
    fn test_empty_message() {
        let mut error = create_valid_error();
        error.message = String::new();
        let result = error.validate();
        assert!(matches!(result, Err(ErrorValidationError::EmptyMessage)));
    }

    #[test]
    fn test_needs_human_without_suggestions() {
        let error = ErrorOutput {
            error_type: ErrorType::Stuck,
            message: "I'm stuck".to_string(),
            attempted_recovery: None,
            needs_human: true,
            suggestions: vec![],
            debug_info: None,
        };
        let result = error.validate();
        assert!(matches!(
            result,
            Err(ErrorValidationError::NeedsHumanWithoutSuggestions)
        ));
    }

    #[test]
    fn test_needs_human_with_suggestions_is_ok() {
        let error = ErrorOutput {
            error_type: ErrorType::Stuck,
            message: "I'm stuck".to_string(),
            attempted_recovery: None,
            needs_human: true,
            suggestions: vec!["Get help".to_string()],
            debug_info: None,
        };
        assert!(error.validate().is_ok());
    }

    #[test]
    fn test_task_unclear_factory() {
        let error = ErrorOutput::task_unclear("What should I do?");
        assert_eq!(error.error_type, ErrorType::TaskUnclear);
        assert!(error.needs_human);
        assert!(!error.suggestions.is_empty());
        assert!(error.validate().is_ok());
    }

    #[test]
    fn test_missing_context_factory() {
        let error = ErrorOutput::missing_context(
            "Need more files",
            vec!["src/config.rs".to_string(), "src/types.rs".to_string()],
        );
        assert_eq!(error.error_type, ErrorType::MissingContext);
        assert!(!error.needs_human);
        assert_eq!(error.suggestions.len(), 2);
        assert!(error.validate().is_ok());
    }

    #[test]
    fn test_compile_error_factory() {
        let error =
            ErrorOutput::compile_error("Type mismatch", Some("src/main.rs".to_string()), Some(42));
        assert_eq!(error.error_type, ErrorType::CompileError);
        assert!(error.debug_info.is_some());
        let debug = error.debug_info.as_ref().unwrap();
        assert_eq!(debug.file, Some("src/main.rs".to_string()));
        assert_eq!(debug.line, Some(42));
        assert!(error.validate().is_ok());
    }

    #[test]
    fn test_stuck_factory_low_attempts() {
        let error = ErrorOutput::stuck("Can't figure it out", 2);
        assert_eq!(error.error_type, ErrorType::Stuck);
        assert!(!error.needs_human);
        assert!(error.attempted_recovery.is_some());
        assert!(error.validate().is_ok());
    }

    #[test]
    fn test_stuck_factory_high_attempts() {
        let error = ErrorOutput::stuck("Can't figure it out", 3);
        assert_eq!(error.error_type, ErrorType::Stuck);
        assert!(error.needs_human);
        assert!(error
            .suggestions
            .contains(&"Human review needed".to_string()));
        assert!(error.validate().is_ok());
    }

    #[test]
    fn test_json_serialization() {
        let error = create_valid_error();
        let json = serde_json::to_string(&error).unwrap();
        let parsed: ErrorOutput = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.message, error.message);
    }

    #[test]
    fn test_error_type_serialization() {
        let error_type = ErrorType::TaskUnclear;
        let json = serde_json::to_string(&error_type).unwrap();
        assert_eq!(json, "\"task_unclear\"");

        let error_type = ErrorType::MissingContext;
        let json = serde_json::to_string(&error_type).unwrap();
        assert_eq!(json, "\"missing_context\"");

        let error_type = ErrorType::ExternalError;
        let json = serde_json::to_string(&error_type).unwrap();
        assert_eq!(json, "\"external_error\"");
    }

    #[test]
    fn test_debug_info_with_trace() {
        let error = ErrorOutput {
            error_type: ErrorType::Unknown,
            message: "Something went wrong".to_string(),
            attempted_recovery: None,
            needs_human: false,
            suggestions: vec![],
            debug_info: Some(DebugInfo {
                trace: Some("at line 1\nat line 2".to_string()),
                file: None,
                line: None,
                raw_output: Some("error output here".to_string()),
            }),
        };
        assert!(error.validate().is_ok());
        let debug = error.debug_info.unwrap();
        assert!(debug.trace.is_some());
        assert!(debug.raw_output.is_some());
    }
}
