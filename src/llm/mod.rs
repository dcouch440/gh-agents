//! LLM integration layer

mod anthropic;
mod grok;
mod noop;
mod ollama;
mod provider;
mod rate_limit;
mod registry;
mod retry;
mod stream;
mod types;
mod xai;

pub use anthropic::*;
pub use grok::*;
pub use noop::*;
pub use ollama::*;
pub use provider::*;
pub use rate_limit::*;
pub use registry::*;
pub use retry::*;
pub use stream::*;
pub use types::*;
pub use xai::*;

/// Create a lightweight LLM client for one-off utility calls (title gen, summarization, etc.)
/// using the active provider profile from `constants::ACTIVE_PROVIDER`.
///
/// Does NOT include retry/rate-limit middleware — intended for fire-and-forget operations.
pub fn create_utility_client() -> Result<Box<dyn LLMProvider + Send + Sync>, LLMError> {
    match crate::constants::ACTIVE_PROVIDER {
        "xai" => Ok(Box::new(XaiClient::from_env()?)),
        "anthropic" => Ok(Box::new(AnthropicClient::from_env()?)),
        other => Err(LLMError::AuthError(format!(
            "Unknown active provider: {other}"
        ))),
    }
}
