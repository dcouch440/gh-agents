//! Tests for Ollama provider

use super::*;
use crate::llm::{ContentBlock, LLMRequest, Message, StopReason, StreamChunk, Tool};

// ── Config ────────────────────────────────────────────────────────────

#[test]
fn config_with_model_overrides_default() {
    let config = OllamaConfig {
        base_url: "http://localhost:11434".to_string(),
        model: "llama3.1".to_string(),
        timeout_secs: 300,
    };
    let config = config.with_model("mistral:latest");
    assert_eq!(config.model, "mistral:latest");
}

#[test]
fn config_with_base_url_overrides_default() {
    let config = OllamaConfig {
        base_url: "http://localhost:11434".to_string(),
        model: "llama3.1".to_string(),
        timeout_secs: 300,
    };
    let config = config.with_base_url("http://192.168.1.100:11434");
    assert_eq!(config.base_url, "http://192.168.1.100:11434");
}

#[test]
fn new_rejects_empty_model() {
    let config = OllamaConfig {
        base_url: "http://localhost:11434".to_string(),
        model: String::new(),
        timeout_secs: 300,
    };
    let result = OllamaClient::new(config);
    assert!(result.is_err());
}

#[test]
fn new_accepts_valid_config() {
    let config = OllamaConfig {
        base_url: "http://localhost:11434".to_string(),
        model: "llama3.1".to_string(),
        timeout_secs: 300,
    };
    let client = OllamaClient::new(config).unwrap();
    assert_eq!(client.config.model, "llama3.1");
}

// ── Request building ──────────────────────────────────────────────────

#[test]
fn build_request_includes_system_as_first_message() {
    let client = make_client();
    let request =
        LLMRequest::new("llama3.1", vec![Message::user("Hello")]).with_system("You are helpful.");

    let api_req = client.build_request(&request, false);

    assert_eq!(api_req.messages.len(), 2);
    assert_eq!(api_req.messages[0].role, "system");
    assert_eq!(
        api_req.messages[0].content.as_deref(),
        Some("You are helpful.")
    );
    assert_eq!(api_req.messages[1].role, "user");
}

#[test]
fn build_request_uses_config_model_when_request_model_empty() {
    let client = make_client();
    let request = LLMRequest::new("", vec![Message::user("Hello")]);

    let api_req = client.build_request(&request, false);
    assert_eq!(api_req.model, "llama3.1");
}

#[test]
fn build_request_uses_request_model_when_set() {
    let client = make_client();
    let request = LLMRequest::new("codellama:7b", vec![Message::user("Hello")]);

    let api_req = client.build_request(&request, false);
    assert_eq!(api_req.model, "codellama:7b");
}

#[test]
fn build_request_maps_tools() {
    let client = make_client();
    let tools = vec![Tool {
        name: "get_weather".to_string(),
        description: "Get weather for a city".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": { "city": { "type": "string" } }
        }),
    }];
    let request = LLMRequest::new("llama3.1", vec![Message::user("Weather?")]).with_tools(tools);

    let api_req = client.build_request(&request, false);
    assert!(api_req.tools.is_some());

    let tools = api_req.tools.unwrap();
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].function.name, "get_weather");
    assert_eq!(tools[0].tool_type, "function");
}

#[test]
fn build_request_omits_tools_when_empty() {
    let client = make_client();
    let request = LLMRequest::new("llama3.1", vec![Message::user("Hello")]);

    let api_req = client.build_request(&request, false);
    assert!(api_req.tools.is_none());
}

#[test]
fn build_request_sets_stream_flag() {
    let client = make_client();
    let request = LLMRequest::new("llama3.1", vec![Message::user("Hello")]);

    let non_stream = client.build_request(&request, false);
    assert!(!non_stream.stream);

    let stream = client.build_request(&request, true);
    assert!(stream.stream);
}

#[test]
fn build_request_sets_temperature_and_max_tokens() {
    let client = make_client();
    let mut request =
        LLMRequest::new("llama3.1", vec![Message::user("Hello")]).with_max_tokens(2048);
    request.temperature = 0.3;

    let api_req = client.build_request(&request, false);
    let opts = api_req.options.unwrap();
    assert_eq!(opts.temperature, Some(0.3));
    assert_eq!(opts.num_predict, Some(2048));
}

// ── Response parsing ──────────────────────────────────────────────────

#[test]
fn parse_response_extracts_text_and_tokens() {
    let body = serde_json::json!({
        "model": "llama3.1",
        "message": {
            "role": "assistant",
            "content": "Hello! How can I help?"
        },
        "done": true,
        "prompt_eval_count": 20,
        "eval_count": 50
    });

    let response = OllamaClient::parse_response(&body.to_string()).unwrap();
    assert_eq!(response.content, "Hello! How can I help?");
    assert_eq!(response.model, "llama3.1");
    assert_eq!(response.stop_reason, StopReason::EndTurn);
    assert_eq!(response.usage.input_tokens, 20);
    assert_eq!(response.usage.output_tokens, 50);
    assert_eq!(response.content_blocks.len(), 1);
}

#[test]
fn parse_response_handles_tool_calls() {
    let body = serde_json::json!({
        "model": "llama3.1",
        "message": {
            "role": "assistant",
            "content": "",
            "tool_calls": [{
                "function": {
                    "name": "get_weather",
                    "arguments": { "city": "London" }
                }
            }]
        },
        "done": true,
        "prompt_eval_count": 30,
        "eval_count": 15
    });

    let response = OllamaClient::parse_response(&body.to_string()).unwrap();
    assert_eq!(response.stop_reason, StopReason::ToolUse);
    assert_eq!(response.content_blocks.len(), 1);
    match &response.content_blocks[0] {
        ContentBlock::ToolUse { name, input, .. } => {
            assert_eq!(name, "get_weather");
            assert_eq!(input["city"], "London");
        }
        _ => panic!("expected ToolUse content block"),
    }
}

#[test]
fn parse_response_handles_missing_token_counts() {
    let body = serde_json::json!({
        "model": "llama3.1",
        "message": {
            "role": "assistant",
            "content": "Hi"
        },
        "done": true
    });

    let response = OllamaClient::parse_response(&body.to_string()).unwrap();
    assert_eq!(response.usage.input_tokens, 0);
    assert_eq!(response.usage.output_tokens, 0);
}

#[test]
fn parse_response_rejects_invalid_json() {
    let result = OllamaClient::parse_response("not json");
    assert!(result.is_err());
}

// ── Stream chunk parsing ──────────────────────────────────────────────

#[test]
fn parse_stream_chunk_extracts_content_delta() {
    let line =
        r#"{"model":"llama3.1","message":{"role":"assistant","content":"Hello"},"done":false}"#;
    let result = OllamaClient::parse_stream_chunk(line).unwrap().unwrap();
    match result {
        StreamChunk::ContentDelta { text, index } => {
            assert_eq!(text, "Hello");
            assert_eq!(index, 0);
        }
        _ => panic!("expected ContentDelta"),
    }
}

#[test]
fn parse_stream_chunk_handles_done_with_tokens() {
    let line = r#"{"model":"llama3.1","message":{"role":"assistant","content":""},"done":true,"eval_count":100,"prompt_eval_count":50}"#;
    let result = OllamaClient::parse_stream_chunk(line).unwrap().unwrap();
    match result {
        StreamChunk::MessageDelta {
            stop_reason,
            output_tokens,
        } => {
            assert_eq!(stop_reason, Some(StopReason::EndTurn));
            assert_eq!(output_tokens, Some(100));
        }
        _ => panic!("expected MessageDelta"),
    }
}

#[test]
fn parse_stream_chunk_skips_empty_lines() {
    assert!(OllamaClient::parse_stream_chunk("").is_none());
    assert!(OllamaClient::parse_stream_chunk("   ").is_none());
}

// ── Provider trait ────────────────────────────────────────────────────

#[test]
fn provider_name_is_ollama() {
    let client = make_client();
    assert_eq!(client.provider_name(), "ollama");
}

#[test]
fn model_id_returns_configured_model() {
    let client = make_client();
    assert_eq!(client.model_id(), "llama3.1");
}

// ── Message conversion ────────────────────────────────────────────────

#[test]
fn ollama_message_from_text_message() {
    let msg = Message::user("Hello");
    let ollama_msg = OllamaMessage::from_llm_message(&msg);
    assert_eq!(ollama_msg.role, "user");
    assert_eq!(ollama_msg.content.as_deref(), Some("Hello"));
}

#[test]
fn ollama_message_from_assistant_message() {
    let msg = Message::assistant("World");
    let ollama_msg = OllamaMessage::from_llm_message(&msg);
    assert_eq!(ollama_msg.role, "assistant");
    assert_eq!(ollama_msg.content.as_deref(), Some("World"));
}

// ── Helpers ───────────────────────────────────────────────────────────

fn make_client() -> OllamaClient {
    OllamaClient::new(OllamaConfig {
        base_url: "http://localhost:11434".to_string(),
        model: "llama3.1".to_string(),
        timeout_secs: 300,
    })
    .unwrap()
}
