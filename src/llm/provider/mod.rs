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
    async fn send_message_stream(
        &self,
        request: LLMRequest,
    ) -> LLMResult<Pin<Box<dyn Stream<Item = LLMResult<StreamChunk>> + Send>>>;

    /// Get the provider name for logging/debugging
    fn provider_name(&self) -> &'static str;

    /// Get the model being used
    fn model_id(&self) -> &str;
}

#[cfg(test)]
mod tests;
