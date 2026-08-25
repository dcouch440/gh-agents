//! Reusable stream safety utilities for LLM providers.
//!
//! Provides two layers of protection that apply to all providers:
//!
//! 1. **`safe_line_stream`** — Converts raw HTTP byte streams into complete
//!    UTF-8 lines without corrupting multi-byte characters split across TCP
//!    chunk boundaries. Enforces a buffer size cap to prevent OOM.
//!
//! 2. **`SafeStreamProvider`** — Middleware wrapper (like `RetryingProvider`)
//!    that stops yielding stream events after the first error, preventing
//!    consumers from seeing events in an inconsistent post-error state.

use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use futures::{Stream, StreamExt};
use tokio_util::bytes::Bytes;

use super::provider::{LLMProvider, LLMResult};
use super::types::{LLMError, LLMRequest, LLMResponse, StreamChunk};

/// Default maximum stream buffer size (10 MB).
pub const DEFAULT_MAX_STREAM_BUFFER: usize = 10 * 1024 * 1024;

// ── Safe line stream utility ─────────────────────────────────────────

/// Convert a raw HTTP byte stream into complete UTF-8 lines.
///
/// Buffers raw bytes and splits on `\n` boundaries at the byte level,
/// avoiding the UTF-8 corruption that occurs when `from_utf8_lossy` is
/// applied to TCP chunks that split multi-byte sequences.
///
/// Returns each line as a `String` (without the trailing newline).
/// Empty lines are yielded — callers decide whether to skip them.
///
/// Yields `LLMError::StreamError` if the buffer exceeds `max_buffer` bytes,
/// or `LLMError::StreamTransport` (carrying the underlying `reqwest::Error`)
/// if the transport fails mid-stream.
pub fn safe_line_stream(
    byte_stream: impl Stream<Item = Result<Bytes, reqwest::Error>> + Send + 'static,
    max_buffer: usize,
) -> Pin<Box<dyn Stream<Item = Result<String, LLMError>> + Send>> {
    let stream = async_stream::stream! {
        let mut byte_buf: Vec<u8> = Vec::new();
        let mut byte_stream = std::pin::pin!(byte_stream);

        while let Some(chunk_result) = byte_stream.next().await {
            match chunk_result {
                Ok(bytes) => {
                    byte_buf.extend_from_slice(&bytes);

                    if byte_buf.len() > max_buffer {
                        yield Err(LLMError::StreamError(
                            format!(
                                "Stream buffer exceeded {} bytes — aborting",
                                max_buffer
                            )
                        ));
                        return;
                    }

                    while let Some(pos) = byte_buf.iter().position(|&b| b == b'\n') {
                        let line_bytes: Vec<u8> = byte_buf.drain(..=pos).collect();
                        let line = String::from_utf8_lossy(
                            &line_bytes[..line_bytes.len() - 1],
                        ).into_owned();
                        yield Ok(line);
                    }
                }
                Err(e) => {
                    // Preserve the reqwest error rather than stringifying it —
                    // `e.to_string()` renders only the top-level Display and
                    // discards the source chain, which is what retry
                    // classification needs to recognise a timeout.
                    yield Err(LLMError::StreamTransport(e));
                    return;
                }
            }
        }
    };
    Box::pin(stream)
}

/// Convenience wrapper using the default 10 MB buffer cap.
pub fn safe_line_stream_default(
    byte_stream: impl Stream<Item = Result<Bytes, reqwest::Error>> + Send + 'static,
) -> Pin<Box<dyn Stream<Item = Result<String, LLMError>> + Send>> {
    safe_line_stream(byte_stream, DEFAULT_MAX_STREAM_BUFFER)
}

// ── SafeStreamProvider middleware ─────────────────────────────────────

/// Middleware wrapper that stops a stream after the first error.
///
/// Compose this as the outermost layer in the provider chain:
///
/// ```ignore
/// SafeStreamProvider::new(
///     RetryingProvider::with_defaults(
///         RateLimitedProvider::with_defaults(provider)
///     )
/// )
/// ```
///
/// For `send_message` (non-streaming), this is a passthrough.
/// For `send_message_stream`, it wraps the returned stream so that
/// once an `Err` is yielded, no further items are produced.
pub struct SafeStreamProvider<P: LLMProvider> {
    inner: Arc<P>,
}

impl<P: LLMProvider + 'static> SafeStreamProvider<P> {
    pub fn new(provider: P) -> Self {
        Self {
            inner: Arc::new(provider),
        }
    }
}

#[async_trait]
impl<P: LLMProvider + 'static> LLMProvider for SafeStreamProvider<P> {
    async fn send_message(&self, request: LLMRequest) -> LLMResult<LLMResponse> {
        self.inner.send_message(request).await
    }

    async fn send_message_stream(
        &self,
        request: LLMRequest,
    ) -> LLMResult<Pin<Box<dyn Stream<Item = LLMResult<StreamChunk>> + Send>>> {
        let inner_stream = self.inner.send_message_stream(request).await?;

        let stream = async_stream::stream! {
            let mut inner = std::pin::pin!(inner_stream);
            while let Some(result) = inner.next().await {
                match &result {
                    Err(_) => {
                        yield result;
                        return; // Stop after first error
                    }
                    Ok(_) => yield result,
                }
            }
        };

        Ok(Box::pin(stream))
    }

    fn provider_name(&self) -> &'static str {
        self.inner.provider_name()
    }

    fn model_id(&self) -> &str {
        self.inner.model_id()
    }
}

#[cfg(test)]
mod tests;
