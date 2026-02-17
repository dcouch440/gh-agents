//! Shared validation helpers used across service modules.

use crate::constants::{MAX_PROMPT_LENGTH, MAX_TITLE_LENGTH};

use super::error::ServiceError;

/// Validate a name field: non-empty after trim, within `MAX_TITLE_LENGTH`.
pub fn validate_name(name: &str, field: &str) -> Result<(), ServiceError> {
    if name.trim().is_empty() || name.len() > MAX_TITLE_LENGTH {
        return Err(ServiceError::validation(format!(
            "{field} must be non-empty and within length limit"
        )));
    }
    Ok(())
}

/// Validate a prompt string: within `MAX_PROMPT_LENGTH`.
pub fn validate_prompt(prompt: &str) -> Result<(), ServiceError> {
    if prompt.len() > MAX_PROMPT_LENGTH {
        return Err(ServiceError::validation("Prompt exceeds maximum length"));
    }
    Ok(())
}

/// Validate a required string field is non-empty after trim.
pub fn validate_required(value: &str, field: &str) -> Result<(), ServiceError> {
    if value.trim().is_empty() {
        return Err(ServiceError::validation(format!("{field} is required")));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_name_rejects_empty() {
        assert!(validate_name("", "name").is_err());
        assert!(validate_name("   ", "name").is_err());
    }

    #[test]
    fn validate_name_rejects_too_long() {
        let long = "a".repeat(MAX_TITLE_LENGTH + 1);
        assert!(validate_name(&long, "name").is_err());
    }

    #[test]
    fn validate_name_accepts_valid() {
        assert!(validate_name("Valid Name", "name").is_ok());
        assert!(validate_name(&"a".repeat(MAX_TITLE_LENGTH), "name").is_ok());
    }

    #[test]
    fn validate_prompt_rejects_too_long() {
        let long = "x".repeat(MAX_PROMPT_LENGTH + 1);
        assert!(validate_prompt(&long).is_err());
    }

    #[test]
    fn validate_prompt_accepts_valid() {
        assert!(validate_prompt("short prompt").is_ok());
    }

    #[test]
    fn validate_required_rejects_empty() {
        assert!(validate_required("", "field").is_err());
        assert!(validate_required("  ", "field").is_err());
    }

    #[test]
    fn validate_required_accepts_nonempty() {
        assert!(validate_required("value", "field").is_ok());
    }
}
