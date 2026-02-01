//! LLM Provider trait abstraction

use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

use super::types::{LLMError, LLMRequest, LLMResponse, StreamChunk};

/// Result type for LLM operations
pub type LLMResult<T> = Result<T, LLMError>;

/// Trait for LLM providers (Anthropic, etc.)
#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// Send a message and wait for complete response
    async fn send_message(&self, request: LLMRequest) -> LLMResult<LLMResponse>;

    /// Send a message and receive streaming response
    async fn send_message_stream(&self, request: LLMRequest) -> LLMResult<Pin<Box<dyn Stream<Item = LLMResult<StreamChunk>> + Send>>>;

    /// Get the provider name for logging/debugging
    fn provider_name(&self) -> &'static str;

    /// Get the model being used
    fn model_id(&self) -> &str;
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::StreamExt;

    /// Mock provider for testing that the trait can be implemented
    struct MockProvider {
        model: String,
        response_content: String,
    }

    impl MockProvider {
        fn new(model: &str, response: &str) -> Self {
            Self {
                model: model.to_string(),
                response_content: response.to_string(),
            }
        }
    }

    #[async_trait]
    impl LLMProvider for MockProvider {
        async fn send_message(&self, _request: LLMRequest) -> LLMResult<LLMResponse> {
            Ok(LLMResponse {
                content: self.response_content.clone(),
                content_blocks: vec![],
                model: self.model.clone(),
                stop_reason: super::super::types::StopReason::EndTurn,
                usage: super::super::types::TokenUsage {
                    input_tokens: 10,
                    output_tokens: 20,
                },
            })
        }

        async fn send_message_stream(&self, _request: LLMRequest) -> LLMResult<Pin<Box<dyn Stream<Item = LLMResult<StreamChunk>> + Send>>> {
            let content = self.response_content.clone();
            let model = self.model.clone();

            let stream = futures::stream::iter(vec![
                Ok(StreamChunk::MessageStart { model, input_tokens: 10 }),
                Ok(StreamChunk::ContentDelta { text: content, index: 0 }),
                Ok(StreamChunk::MessageDelta {
                    stop_reason: Some(super::super::types::StopReason::EndTurn),
                    output_tokens: Some(20),
                }),
                Ok(StreamChunk::MessageStop),
            ]);

            Ok(Box::pin(stream))
        }

        fn provider_name(&self) -> &'static str {
            "mock"
        }

        fn model_id(&self) -> &str {
            &self.model
        }
    }

    #[tokio::test]
    async fn mock_provider_send_message() {
        let provider = MockProvider::new("test-model", "Hello, world!");
        let request = super::super::types::LLMRequest::new("test-model", vec![super::super::types::Message::user("Hi")]);

        let response = provider.send_message(request).await.unwrap();
        assert_eq!(response.content, "Hello, world!");
        assert_eq!(response.model, "test-model");
    }

    #[tokio::test]
    async fn mock_provider_stream() {
        let provider = MockProvider::new("test-model", "Streamed content");
        let request = super::super::types::LLMRequest::new("test-model", vec![super::super::types::Message::user("Hi")]);

        let mut stream = provider.send_message_stream(request).await.unwrap();
        let mut chunks = Vec::new();

        while let Some(chunk) = stream.next().await {
            chunks.push(chunk.unwrap());
        }

        assert_eq!(chunks.len(), 4);
        matches!(&chunks[0], StreamChunk::MessageStart { .. });
        matches!(&chunks[3], StreamChunk::MessageStop);
    }

    #[test]
    fn provider_name_and_model() {
        let provider = MockProvider::new("claude-3-opus", "test");
        assert_eq!(provider.provider_name(), "mock");
        assert_eq!(provider.model_id(), "claude-3-opus");
    }
}
