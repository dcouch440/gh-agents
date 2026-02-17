#[cfg(test)]
mod tests {
    use crate::server::services::chat::*;
    use crate::server::services::ServiceError;

    #[test]
    fn validate_rejects_empty_message() {
        let result = validate_message("  ", 5000);
        assert!(matches!(result, Err(ServiceError::Validation(_))));
    }

    #[test]
    fn validate_rejects_too_long() {
        let long = "a".repeat(5001);
        let result = validate_message(&long, 5000);
        assert!(matches!(result, Err(ServiceError::Validation(_))));
    }

    #[test]
    fn validate_accepts_valid_message() {
        let result = validate_message("Hello!", 5000);
        assert!(result.is_ok());
    }
}
