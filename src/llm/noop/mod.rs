//! No-op LLM provider for when API keys are not configured

use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;

use super::provider::{LLMProvider, LLMResult};
use super::types::{LLMError, LLMRequest, LLMResponse, StreamChunk};

/// A no-op LLM provider that returns errors indicating the API key is not configured.
///
/// Use this provider as a fallback when ANTHROPIC_API_KEY is not set,
/// allowing the application to start without a configured LLM provider.
#[derive(Debug, Clone, Default)]
pub struct NoOpProvider;

impl NoOpProvider {
    /// Create a new no-op provider
    pub fn new() -> Self {
        Self
    }

    /// Create the error returned by all operations
    fn not_configured_error() -> LLMError {
        LLMError::AuthError(
            "LLM provider not configured: ANTHROPIC_API_KEY environment variable is not set"
                .to_string(),
        )
    }
}

#[async_trait]
impl LLMProvider for NoOpProvider {
    async fn send_message(&self, _request: LLMRequest) -> LLMResult<LLMResponse> {
        Err(Self::not_configured_error())
    }

    async fn send_message_stream(
        &self,
        _request: LLMRequest,
    ) -> LLMResult<Pin<Box<dyn Stream<Item = LLMResult<StreamChunk>> + Send>>> {
        Err(Self::not_configured_error())
    }

    fn provider_name(&self) -> &'static str {
        "noop"
    }

    fn model_id(&self) -> &str {
        "none"
    }
}

#[cfg(test)]
mod tests;
