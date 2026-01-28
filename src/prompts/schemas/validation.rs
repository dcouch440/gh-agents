//! Unified output validation for all schema types.

use super::*;
use serde::de::DeserializeOwned;

/// Unified output validation for all schema types.
pub struct OutputValidator;

impl OutputValidator {
    /// Parse and validate decomposition output
    pub fn validate_decomposition(json: &str) -> ValidationResult<DecompositionOutput> {
        Self::parse_and_validate::<DecompositionOutput>(json).and_then(|output| {
            output
                .validate()
                .map_err(|e| OutputValidationError::ContentInvalid(e.to_string()))?;
            Ok(output)
        })
    }

    /// Parse and validate task result output
    pub fn validate_task_result(json: &str) -> ValidationResult<TaskResultOutput> {
        Self::parse_and_validate::<TaskResultOutput>(json).and_then(|output| {
            output
                .validate()
                .map_err(|e| OutputValidationError::ContentInvalid(e.to_string()))?;
            Ok(output)
        })
    }

    /// Parse and validate review output
    pub fn validate_review(json: &str) -> ValidationResult<ReviewOutput> {
        Self::parse_and_validate::<ReviewOutput>(json).and_then(|output| {
            output
                .validate()
                .map_err(|e| OutputValidationError::ContentInvalid(e.to_string()))?;
            Ok(output)
        })
    }

    /// Parse and validate error output
    pub fn validate_error(json: &str) -> ValidationResult<ErrorOutput> {
        Self::parse_and_validate::<ErrorOutput>(json).and_then(|output| {
            output
                .validate()
                .map_err(|e| OutputValidationError::ContentInvalid(e.to_string()))?;
            Ok(output)
        })
    }

    /// Generic parse and validate
    fn parse_and_validate<T: DeserializeOwned>(json: &str) -> ValidationResult<T> {
        // Try to extract JSON from the response (might have markdown fences)
        let clean_json = Self::extract_json(json);

        serde_json::from_str(&clean_json).map_err(|e| OutputValidationError::ParseFailed {
            error: e.to_string(),
            hint: Self::generate_parse_hint(&e, &clean_json),
        })
    }

    /// Extract JSON from a response that might have markdown fences
    pub fn extract_json(text: &str) -> String {
        // Look for JSON in code blocks
        if let Some(start) = text.find("```json") {
            if let Some(end) = text[start + 7..].find("```") {
                return text[start + 7..start + 7 + end].trim().to_string();
            }
        }

        // Look for just code blocks
        if let Some(start) = text.find("```") {
            if let Some(end) = text[start + 3..].find("```") {
                return text[start + 3..start + 3 + end].trim().to_string();
            }
        }

        // Try to find JSON object directly
        if let Some(start) = text.find('{') {
            if let Some(end) = text.rfind('}') {
                return text[start..=end].to_string();
            }
        }

        text.to_string()
    }

    /// Generate a helpful hint for parse errors
    fn generate_parse_hint(error: &serde_json::Error, json: &str) -> String {
        let err_msg = error.to_string();

        // Check structural issues first
        if json.is_empty() {
            return "Response was empty. Please output valid JSON matching the schema.".to_string();
        }

        if !json.starts_with('{') {
            return "Response should be a JSON object starting with '{'. Remove any text before the JSON.".to_string();
        }

        // Check for specific error types
        if err_msg.contains("missing field") {
            if let Some(field) = err_msg.split('`').nth(1) {
                return format!(
                    "Add the missing '{}' field to your response. Check the schema for required fields.",
                    field
                );
            }
        }

        if err_msg.contains("expected") {
            return "Check that field types match the schema (strings vs numbers, arrays vs objects).".to_string();
        }

        "Check that your JSON is valid and matches the expected schema.".to_string()
    }
}

pub type ValidationResult<T> = Result<T, OutputValidationError>;

#[derive(Debug, Clone)]
pub enum OutputValidationError {
    /// JSON couldn't be parsed
    ParseFailed { error: String, hint: String },
    /// JSON parsed but content is invalid
    ContentInvalid(String),
}

impl OutputValidationError {
    /// Get a message suitable for a retry prompt
    pub fn retry_message(&self) -> String {
        match self {
            Self::ParseFailed { error, hint } => {
                format!(
                    "Your output couldn't be parsed as JSON.\n\n\
                     Error: {}\n\n\
                     Hint: {}\n\n\
                     Please try again with valid JSON matching the schema.",
                    error, hint
                )
            }
            Self::ContentInvalid(msg) => {
                format!(
                    "Your output was valid JSON but failed validation.\n\n\
                     Issue: {}\n\n\
                     Please fix this issue and try again.",
                    msg
                )
            }
        }
    }
}

impl std::fmt::Display for OutputValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ParseFailed { error, .. } => write!(f, "Parse failed: {}", error),
            Self::ContentInvalid(msg) => write!(f, "Validation failed: {}", msg),
        }
    }
}

impl std::error::Error for OutputValidationError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json_from_markdown() {
        let input = "Here's my response:\n```json\n{\"thinking\": \"test\"}\n```\nDone!";
        let extracted = OutputValidator::extract_json(input);
        assert_eq!(extracted, "{\"thinking\": \"test\"}");
    }

    #[test]
    fn test_extract_plain_json() {
        let input = "{\"thinking\": \"test\"}";
        let extracted = OutputValidator::extract_json(input);
        assert_eq!(extracted, "{\"thinking\": \"test\"}");
    }

    #[test]
    fn test_extract_json_with_surrounding_text() {
        let input = "Let me think about this... {\"thinking\": \"test\"} That's my answer.";
        let extracted = OutputValidator::extract_json(input);
        assert_eq!(extracted, "{\"thinking\": \"test\"}");
    }

    #[test]
    fn test_extract_json_from_plain_code_block() {
        let input = "```\n{\"thinking\": \"test\"}\n```";
        let extracted = OutputValidator::extract_json(input);
        assert_eq!(extracted, "{\"thinking\": \"test\"}");
    }

    #[test]
    fn test_validate_decomposition_valid() {
        let json = r#"{
            "thinking": "Analyzing...",
            "slices": [{
                "title": "Slice 1",
                "description": "First slice",
                "tasks": [{
                    "title": "Task 1",
                    "tier": "worker",
                    "estimated_complexity": "medium",
                    "context_files": []
                }],
                "dependencies": [],
                "acceptance_criteria": ["Compiles"]
            }],
            "questions": [],
            "risks": []
        }"#;
        let result = OutputValidator::validate_decomposition(json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_decomposition_missing_field() {
        let json = r#"{
            "slices": [],
            "questions": [],
            "risks": []
        }"#;
        let result = OutputValidator::validate_decomposition(json);
        assert!(matches!(
            result,
            Err(OutputValidationError::ParseFailed { .. })
        ));
    }

    #[test]
    fn test_validate_decomposition_content_invalid() {
        let json = r#"{
            "thinking": "",
            "slices": [],
            "questions": [],
            "risks": []
        }"#;
        let result = OutputValidator::validate_decomposition(json);
        assert!(matches!(
            result,
            Err(OutputValidationError::ContentInvalid(_))
        ));
    }

    #[test]
    fn test_validate_task_result_valid() {
        let json = r#"{
            "phase": "implementing",
            "thinking": "Working on it...",
            "progress": {
                "current_step": "Writing code",
                "completed_steps": [],
                "remaining_steps": []
            },
            "code_changes": [],
            "status": "in_progress"
        }"#;
        let result = OutputValidator::validate_task_result(json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_review_valid() {
        let json = r#"{
            "thinking": "Reviewing...",
            "verdict": "approved",
            "issues": [],
            "positive_notes": ["Good code"],
            "summary": "LGTM"
        }"#;
        let result = OutputValidator::validate_review(json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_error_valid() {
        let json = r#"{
            "error_type": "compile_error",
            "message": "Failed to compile",
            "needs_human": false,
            "suggestions": []
        }"#;
        let result = OutputValidator::validate_error(json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_retry_message_parse_failed() {
        let error = OutputValidationError::ParseFailed {
            error: "missing field `thinking`".to_string(),
            hint: "Add the missing field".to_string(),
        };
        let message = error.retry_message();
        assert!(message.contains("couldn't be parsed"));
        assert!(message.contains("missing field"));
    }

    #[test]
    fn test_retry_message_content_invalid() {
        let error = OutputValidationError::ContentInvalid("Empty thinking".to_string());
        let message = error.retry_message();
        assert!(message.contains("valid JSON but failed validation"));
        assert!(message.contains("Empty thinking"));
    }

    #[test]
    fn test_validate_with_markdown_wrapper() {
        let json = r#"Here is my response:

```json
{
    "thinking": "Analyzing...",
    "slices": [{
        "title": "Slice 1",
        "description": "First slice",
        "tasks": [{
            "title": "Task 1",
            "tier": "worker",
            "estimated_complexity": "low",
            "context_files": []
        }],
        "dependencies": [],
        "acceptance_criteria": ["Done"]
    }],
    "questions": [],
    "risks": []
}
```

Let me know if you need anything else!"#;
        let result = OutputValidator::validate_decomposition(json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_hint_missing_field() {
        let json = "{}";
        let error = serde_json::from_str::<DecompositionOutput>(json).unwrap_err();
        let hint = OutputValidator::generate_parse_hint(&error, json);
        assert!(hint.contains("missing"));
    }

    #[test]
    fn test_parse_hint_empty_input() {
        let error = serde_json::from_str::<DecompositionOutput>("").unwrap_err();
        let hint = OutputValidator::generate_parse_hint(&error, "");
        assert!(hint.contains("empty"));
    }

    #[test]
    fn test_parse_hint_not_json_object() {
        let error = serde_json::from_str::<DecompositionOutput>("hello").unwrap_err();
        let hint = OutputValidator::generate_parse_hint(&error, "hello");
        assert!(hint.contains("starting with '{'"));
    }

    #[test]
    fn test_display_parse_failed() {
        let error = OutputValidationError::ParseFailed {
            error: "syntax error".to_string(),
            hint: "fix it".to_string(),
        };
        let display = format!("{}", error);
        assert!(display.contains("Parse failed"));
        assert!(display.contains("syntax error"));
    }

    #[test]
    fn test_display_content_invalid() {
        let error = OutputValidationError::ContentInvalid("bad content".to_string());
        let display = format!("{}", error);
        assert!(display.contains("Validation failed"));
        assert!(display.contains("bad content"));
    }
}
