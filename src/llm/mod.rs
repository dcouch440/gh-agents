//! LLM integration layer

mod anthropic;
mod deepinfra;
mod grok;
mod noop;
mod ollama;
mod provider;
mod rate_limit;
mod registry;
mod retry;
mod sse_provider;
mod stream;
mod types;
mod xai;

pub use anthropic::*;
pub use deepinfra::*;
pub use grok::*;
pub use noop::*;
pub use ollama::*;
pub use provider::*;
pub use rate_limit::*;
pub use registry::*;
pub use retry::*;
pub use sse_provider::*;
pub use stream::*;
pub use types::*;
pub use xai::*;

/// Create a lightweight LLM client for one-off utility calls (title gen, summarization, etc.)
/// using the active provider profile from `constants::ACTIVE_PROVIDER`.
///
/// Does NOT include retry/rate-limit middleware — intended for fire-and-forget operations.
pub fn create_utility_client() -> Result<Box<dyn LLMProvider + Send + Sync>, LLMError> {
    match crate::constants::ACTIVE_PROVIDER {
        "deepinfra" => {
            // Utility calls sit on the chat hot path and several have no
            // timeout of their own, so they must not inherit the long chat
            // budget that exists to absorb DeepInfra's request queueing.
            let config = DeepInfraConfig::from_env()?
                .with_timeout_secs(crate::constants::DEEPINFRA_UTILITY_TIMEOUT_SECS)
                .with_default_effort(ReasoningEffort::None);
            Ok(Box::new(DeepInfraClient::with_config(config)?))
        }
        "xai" => Ok(Box::new(XaiClient::from_env()?)),
        "anthropic" => Ok(Box::new(AnthropicClient::from_env()?)),
        other => Err(LLMError::AuthError(format!(
            "Unknown active provider: {other}"
        ))),
    }
}
