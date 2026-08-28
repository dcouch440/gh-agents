//! Generic HTTP+SSE provider framework for cloud LLM APIs.
//!
//! Extracts the shared boilerplate that every SSE-based LLM provider repeats:
//! HTTP client construction, request dispatch, error mapping, retry-after
//! parsing, and the streaming SSE consumption loop.
//!
//! Provider-specific logic (request serialization, response parsing, SSE event
//! mapping, auth headers) is encapsulated behind `SseProviderAdapter`.
//!
//! ## Adding a new provider
//!
//! 1. Define a config + adapter struct implementing `SseProviderAdapter`.
//! 2. `pub type MyClient = SseHttpProvider<MyAdapter>;`
//! 3. That's it — HTTP plumbing, SSE loop, and retry-after are free.

use async_trait::async_trait;
use futures::{Stream, StreamExt};
use reqwest::header::HeaderMap;
use reqwest::Client;
use std::pin::Pin;
use std::time::Duration;

use super::provider::{LLMProvider, LLMResult};
use super::types::{LLMError, LLMRequest, LLMResponse, StreamChunk};

// ── Adapter trait ────────────────────────────────────────────────────────

/// Provider-specific hooks for SSE-based cloud LLM APIs.
///
/// Implement this trait once per provider. The generic `SseHttpProvider<A>`
/// supplies all shared HTTP and streaming logic.
pub trait SseProviderAdapter: Send + Sync + Clone + 'static {
    /// Provider name for logging (e.g., `"anthropic"`, `"xai"`).
    fn provider_name(&self) -> &'static str;

    /// Current model ID.
    fn model_id(&self) -> &str;

    /// Full endpoint URL for the API call.
    fn endpoint_url(&self) -> String;

    /// Build default HTTP headers (auth, content-type, version headers).
    fn default_headers(&self) -> Result<HeaderMap, LLMError>;

    /// Request timeout in seconds.
    fn timeout_secs(&self) -> u64;

    /// Serialize an `LLMRequest` into the provider's wire-format JSON body.
    ///
    /// `stream` indicates whether the request should enable streaming.
    fn build_request_body(&self, request: &LLMRequest, stream: bool) -> serde_json::Value;

    /// Parse a non-streaming JSON response body into `LLMResponse`.
    fn parse_response(&self, body: &[u8]) -> Result<LLMResponse, LLMError>;

    /// Parse one SSE `data: ...` line into a `StreamChunk`.
    ///
    /// Return `None` to skip the line (empty, unknown event type, etc.).
    ///
    /// Implement this when one wire event maps to at most one internal chunk.
    /// When it can map to several, implement [`Self::parse_sse_events`] instead
    /// and leave this returning `None`.
    fn parse_sse_line(&self, line: &str) -> Option<LLMResult<StreamChunk>>;

    /// Parse one SSE `data: ...` line into zero or more `StreamChunk`s.
    ///
    /// The default delegates to [`Self::parse_sse_line`], so existing adapters
    /// are unaffected. Override for wire formats where a single event yields
    /// several internal events — an OpenAI-compatible stream opening tool call
    /// *n* implicitly closes call *n-1*, and both the close and the open have
    /// to reach `StreamAccumulator` or the earlier call is dropped.
    fn parse_sse_events(&self, line: &str) -> Vec<LLMResult<StreamChunk>> {
        self.parse_sse_line(line).into_iter().collect()
    }

    /// Optional per-read timeout, distinct from the whole-request timeout.
    ///
    /// `None` keeps the previous behaviour of relying on the request timeout
    /// alone. Providers that queue requests need this: time-to-first-byte can
    /// legitimately be minutes, so the overall timeout must stay generous
    /// while a stalled connection is still detected promptly.
    fn read_timeout_secs(&self) -> Option<u64> {
        None
    }

    /// Map an HTTP error status + body to `LLMError`.
    fn handle_error(&self, status: u16, body: &str, retry_after_ms: Option<u64>) -> LLMError;

    /// Synthetic events to yield BEFORE the SSE loop (default: none).
    fn pre_stream_events(&self) -> Vec<StreamChunk> {
        vec![]
    }

    /// Synthetic events to yield AFTER the SSE loop (default: none).
    fn post_stream_events(&self) -> Vec<StreamChunk> {
        vec![]
    }
}

// ── Generic provider ─────────────────────────────────────────────────────

/// Generic HTTP+SSE provider that delegates provider-specific logic to `A`.
///
/// Handles all shared boilerplate: HTTP client construction, request dispatch,
/// error handling with retry-after, and the streaming SSE consumption loop.
#[derive(Clone)]
pub struct SseHttpProvider<A: SseProviderAdapter> {
    client: Client,
    adapter: A,
}

impl<A: SseProviderAdapter> SseHttpProvider<A> {
    /// Create a new provider from an adapter.
    ///
    /// Builds a `reqwest::Client` with the adapter's default headers and timeout.
    pub fn new(adapter: A) -> Result<Self, LLMError> {
        let headers = adapter.default_headers()?;

        let mut builder = Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(adapter.timeout_secs()));
        if let Some(read_secs) = adapter.read_timeout_secs() {
            builder = builder.read_timeout(Duration::from_secs(read_secs));
        }
        let client = builder.build().map_err(LLMError::HttpError)?;

        Ok(Self { client, adapter })
    }

    /// Access the underlying adapter.
    pub fn adapter(&self) -> &A {
        &self.adapter
    }
}

// ── LLMProvider implementation ───────────────────────────────────────────

#[async_trait]
impl<A: SseProviderAdapter> LLMProvider for SseHttpProvider<A> {
    async fn send_message(&self, request: LLMRequest) -> LLMResult<LLMResponse> {
        let body = self.adapter.build_request_body(&request, false);

        let response = self
            .client
            .post(self.adapter.endpoint_url())
            .json(&body)
            .send()
            .await?;

        let status = response.status().as_u16();

        if !response.status().is_success() {
            let retry_after = parse_retry_after(&response);
            let body = response.text().await.unwrap_or_default();
            return Err(self.adapter.handle_error(status, &body, retry_after));
        }

        let bytes = response.bytes().await.map_err(LLMError::HttpError)?;
        self.adapter.parse_response(&bytes)
    }

    async fn send_message_stream(
        &self,
        request: LLMRequest,
    ) -> LLMResult<Pin<Box<dyn Stream<Item = LLMResult<StreamChunk>> + Send>>> {
        let body = self.adapter.build_request_body(&request, true);

        let response = self
            .client
            .post(self.adapter.endpoint_url())
            .json(&body)
            .send()
            .await?;

        let status = response.status().as_u16();

        if !response.status().is_success() {
            let retry_after = parse_retry_after(&response);
            let body = response.text().await.unwrap_or_default();
            return Err(self.adapter.handle_error(status, &body, retry_after));
        }

        let mut lines = super::stream::safe_line_stream_default(response.bytes_stream());

        // Clone adapter for the async block (requires Clone bound).
        let adapter = self.adapter.clone();

        let stream = async_stream::stream! {
            // Emit synthetic pre-stream events
            for event in adapter.pre_stream_events() {
                yield Ok(event);
            }

            // SSE consumption loop
            while let Some(line_result) = lines.next().await {
                match line_result {
                    Ok(line) => {
                        let line = line.trim().to_string();

                        // Skip empty lines and SSE event type lines
                        if line.is_empty() || line.starts_with("event:") {
                            continue;
                        }

                        for result in adapter.parse_sse_events(&line) {
                            yield result;
                        }
                    }
                    Err(e) => {
                        yield Err(e);
                        return;
                    }
                }
            }

            // Emit synthetic post-stream events
            for event in adapter.post_stream_events() {
                yield Ok(event);
            }
        };

        Ok(Box::pin(stream))
    }

    fn provider_name(&self) -> &'static str {
        self.adapter.provider_name()
    }

    fn model_id(&self) -> &str {
        self.adapter.model_id()
    }
}

// ── Shared utilities ─────────────────────────────────────────────────────

/// Extract `Retry-After` header value and convert to milliseconds.
///
/// Handles both integer and fractional second values.
pub(crate) fn parse_retry_after(response: &reqwest::Response) -> Option<u64> {
    response
        .headers()
        .get("retry-after")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<f64>().ok())
        .map(|secs| (secs * 1000.0) as u64)
}

#[cfg(test)]
mod tests;
