#[cfg(test)]
mod tests {
    use super::super::NoOpProvider;
    use crate::llm::{LLMError, LLMProvider, LLMRequest, Message};

    #[tokio::test]
    async fn send_message_returns_auth_error() {
        let provider = NoOpProvider::new();
        let request = LLMRequest::new("test-model", vec![Message::user("Hello")]);

        let result = provider.send_message(request).await;

        assert!(result.is_err());
        let error = result.err().unwrap();
        assert!(matches!(error, LLMError::AuthError(_)));
        assert!(error.to_string().contains("ANTHROPIC_API_KEY"));
    }

    #[tokio::test]
    async fn send_message_stream_returns_auth_error() {
        let provider = NoOpProvider::new();
        let request = LLMRequest::new("test-model", vec![Message::user("Hello")]);

        let result = provider.send_message_stream(request).await;

        // Use match to avoid Debug trait requirement on the Ok type
        match result {
            Err(error) => {
                assert!(matches!(error, LLMError::AuthError(_)));
                assert!(error.to_string().contains("ANTHROPIC_API_KEY"));
            }
            Ok(_) => panic!("Expected error, got Ok"),
        }
    }

    #[test]
    fn provider_name_returns_noop() {
        let provider = NoOpProvider::new();
        assert_eq!(provider.provider_name(), "noop");
    }

    #[test]
    fn model_id_returns_none() {
        let provider = NoOpProvider::new();
        assert_eq!(provider.model_id(), "none");
    }

    #[test]
    fn default_creates_provider() {
        let provider = NoOpProvider;
        assert_eq!(provider.provider_name(), "noop");
    }
}
