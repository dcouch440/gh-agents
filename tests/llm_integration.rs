//! Integration tests for LLM layer
//!
//! These tests require ANTHROPIC_API_KEY to be set.
//! Run with: ANTHROPIC_API_KEY=your-key cargo test --test llm_integration

use futures::StreamExt;
use nexor::llm::{AnthropicClient, LLMProvider, LLMRequest, Message, StreamChunk};

/// Helper to skip test if API key not available
fn require_api_key() -> Option<String> {
    std::env::var(nexor::constants::ENV_ANTHROPIC_API_KEY).ok()
}

#[tokio::test]
async fn test_send_message_basic() {
    let Some(_) = require_api_key() else {
        eprintln!("Skipping test: ANTHROPIC_API_KEY not set");
        return;
    };

    let client = AnthropicClient::from_env().expect("Failed to create client");

    let request = LLMRequest::new(
        "claude-3-haiku-20240307",
        vec![Message::user("Say 'hello' and nothing else.")],
    )
    .with_max_tokens(50);

    let response = client.send_message(request).await.expect("Request failed");

    assert!(!response.content.is_empty());
    assert!(response.usage.input_tokens > 0);
    assert!(response.usage.output_tokens > 0);
}

#[tokio::test]
async fn test_send_message_with_system() {
    let Some(_) = require_api_key() else {
        eprintln!("Skipping test: ANTHROPIC_API_KEY not set");
        return;
    };

    let client = AnthropicClient::from_env().expect("Failed to create client");

    let request = LLMRequest::new(
        "claude-3-haiku-20240307",
        vec![Message::user("What are you?")],
    )
    .with_system("You are a helpful pirate. Always respond like a pirate.")
    .with_max_tokens(100);

    let response = client.send_message(request).await.expect("Request failed");

    // Should contain pirate-like language
    let content_lower = response.content.to_lowercase();
    assert!(
        content_lower.contains("arr")
            || content_lower.contains("matey")
            || content_lower.contains("pirate")
            || content_lower.contains("ahoy")
            || content_lower.contains("ye")
            || content_lower.contains("aye"),
        "Expected pirate response, got: {}",
        response.content
    );
}

#[tokio::test]
async fn test_send_message_streaming() {
    let Some(_) = require_api_key() else {
        eprintln!("Skipping test: ANTHROPIC_API_KEY not set");
        return;
    };

    let client = AnthropicClient::from_env().expect("Failed to create client");

    let request = LLMRequest::new(
        "claude-3-haiku-20240307",
        vec![Message::user("Count from 1 to 5, one number per line.")],
    )
    .with_max_tokens(100)
    .with_streaming();

    let mut stream = client
        .send_message_stream(request)
        .await
        .expect("Failed to start stream");

    let mut content = String::new();
    let mut chunk_count = 0;
    let mut got_message_start = false;
    let mut got_message_stop = false;

    while let Some(result) = stream.next().await {
        let chunk = result.expect("Stream error");
        chunk_count += 1;

        match chunk {
            StreamChunk::MessageStart {
                model,
                input_tokens,
            } => {
                got_message_start = true;
                assert!(!model.is_empty());
                assert!(input_tokens > 0);
            }
            StreamChunk::ContentDelta { text, .. } => {
                content.push_str(&text);
            }
            StreamChunk::MessageStop => {
                got_message_stop = true;
            }
            _ => {}
        }
    }

    assert!(got_message_start, "Expected message_start event");
    assert!(got_message_stop, "Expected message_stop event");
    assert!(chunk_count > 1, "Expected multiple chunks");
    assert!(!content.is_empty(), "Expected content from stream");
    assert!(content.contains('1'), "Expected content to contain numbers");
}

#[tokio::test]
async fn test_provider_trait_methods() {
    let Some(_) = require_api_key() else {
        eprintln!("Skipping test: ANTHROPIC_API_KEY not set");
        return;
    };

    let client = AnthropicClient::from_env().expect("Failed to create client");

    assert_eq!(client.provider_name(), "anthropic");
    assert!(!client.model_id().is_empty());
}
