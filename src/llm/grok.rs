//! xAI Grok research client for real-time web and X search.
//!
//! Uses the Grok Responses API with server-side `web_search` and `x_search`
//! tools. Grok handles the entire search loop autonomously — we send a query,
//! it searches, browses, and returns a synthesized answer with citations.

use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

use super::types::{LLMError, TokenUsage};
use crate::constants;

// ── Config ──────────────────────────────────────────────────────────────────

/// Configuration for the Grok research client.
#[derive(Debug, Clone)]
pub struct GrokConfig {
    /// xAI API key (Bearer token).
    pub api_key: String,
    /// Base URL for the xAI API.
    pub base_url: String,
    /// Model to use for research queries.
    pub model: String,
    /// Request timeout in seconds.
    pub timeout_secs: u64,
    /// Maximum output tokens.
    pub max_tokens: u32,
    /// Maximum agentic search turns.
    pub max_search_turns: u32,
}

impl GrokConfig {
    /// Create config from environment variables.
    ///
    /// Reads `XAI_API_KEY` (required) and `XAI_MODEL` (optional).
    pub fn from_env() -> Result<Self, LLMError> {
        let api_key = std::env::var(constants::ENV_XAI_API_KEY).map_err(|_| LLMError::AuthError(format!("{} not set", constants::ENV_XAI_API_KEY)))?;

        let model = std::env::var(constants::ENV_XAI_MODEL).unwrap_or_else(|_| constants::XAI_RESEARCH_MODEL.to_string());

        Ok(Self {
            api_key,
            base_url: constants::XAI_DEFAULT_BASE_URL.to_string(),
            model,
            timeout_secs: constants::XAI_RESEARCH_TIMEOUT_SECS,
            max_tokens: constants::XAI_RESEARCH_MAX_TOKENS,
            max_search_turns: constants::XAI_RESEARCH_MAX_SEARCH_TURNS,
        })
    }

    /// Set the model.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Set the base URL (useful for testing).
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }
}

// ── Request / Response Types ────────────────────────────────────────────────

/// Which sources to include in the research.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResearchSource {
    Web,
    X,
}

/// Filters for web search.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WebSearchFilters {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_domains: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub excluded_domains: Option<Vec<String>>,
}

/// Filters for X search.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct XSearchFilters {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_x_handles: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub excluded_x_handles: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_date: Option<String>,
}

/// A request for the Grok research client.
#[derive(Debug, Clone)]
pub struct ResearchRequest {
    /// The research query / question.
    pub query: String,
    /// Which sources to search (default: both web and X).
    pub sources: Vec<ResearchSource>,
    /// Optional web search filters.
    pub web_filters: Option<WebSearchFilters>,
    /// Optional X search filters.
    pub x_filters: Option<XSearchFilters>,
    /// Optional system prompt override.
    pub system_prompt: Option<String>,
}

impl ResearchRequest {
    /// Create a new research request with default settings (both sources).
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            sources: vec![ResearchSource::Web, ResearchSource::X],
            web_filters: None,
            x_filters: None,
            system_prompt: None,
        }
    }
}

/// A citation from the research response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Citation {
    pub title: String,
    pub url: String,
    pub source_type: String,
}

/// Response from a Grok research query.
#[derive(Debug, Clone)]
pub struct ResearchResponse {
    /// The synthesized answer text.
    pub answer: String,
    /// Citations found during research.
    pub citations: Vec<Citation>,
    /// Token usage for this request.
    pub usage: TokenUsage,
}

// ── API Wire Types (serde) ──────────────────────────────────────────────────

/// Request body for the xAI Responses API.
#[derive(Debug, Serialize)]
struct XAIRequest {
    model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_output_tokens: Option<u32>,
    input: Vec<XAIMessage>,
    tools: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct XAIMessage {
    role: String,
    content: String,
}

/// Top-level response from the xAI Responses API.
#[derive(Debug, Deserialize)]
struct XAIResponse {
    #[serde(default)]
    output: Vec<XAIOutputItem>,
    #[serde(default)]
    usage: XAIUsage,
}

#[derive(Debug, Deserialize)]
struct XAIOutputItem {
    #[serde(rename = "type", default)]
    item_type: String,
    #[serde(default)]
    content: Vec<XAIContentBlock>,
}

#[derive(Debug, Deserialize)]
struct XAIContentBlock {
    #[serde(rename = "type", default)]
    block_type: String,
    #[serde(default)]
    text: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct XAIUsage {
    #[serde(default)]
    input_tokens: u32,
    #[serde(default)]
    output_tokens: u32,
}

/// Error response from xAI API.
#[derive(Debug, Deserialize)]
struct XAIErrorResponse {
    #[serde(default)]
    error: Option<XAIErrorDetail>,
}

#[derive(Debug, Deserialize)]
struct XAIErrorDetail {
    #[serde(default)]
    message: String,
}

// ── Client ──────────────────────────────────────────────────────────────────

/// Client for xAI Grok research queries.
///
/// Not an `LLMProvider` — this is a purpose-built client for server-side
/// agentic search via the Responses API.
#[derive(Clone)]
pub struct GrokResearchClient {
    client: Client,
    config: GrokConfig,
}

impl GrokResearchClient {
    /// Create a new client from config.
    pub fn new(config: GrokConfig) -> Result<Self, LLMError> {
        if config.api_key.is_empty() {
            return Err(LLMError::AuthError("xAI API key cannot be empty".to_string()));
        }

        let mut headers = HeaderMap::new();
        headers.insert(
            "authorization",
            HeaderValue::from_str(&format!("Bearer {}", config.api_key)).map_err(|_| LLMError::AuthError("Invalid xAI API key format".to_string()))?,
        );
        headers.insert("content-type", HeaderValue::from_static("application/json"));

        let client = Client::builder()
            .default_headers(headers)
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()
            .map_err(LLMError::HttpError)?;

        Ok(Self { client, config })
    }

    /// Create a client from environment variables.
    pub fn from_env() -> Result<Self, LLMError> {
        let config = GrokConfig::from_env()?;
        Self::new(config)
    }

    /// Execute a research query.
    pub async fn research(&self, request: &ResearchRequest) -> Result<ResearchResponse, LLMError> {
        let body = self.build_request_body(request);
        let url = format!("{}/v1/responses", self.config.base_url);

        tracing::debug!(model = %self.config.model, query = %request.query, "Sending Grok research request");

        let response = self.client.post(&url).json(&body).send().await.map_err(LLMError::HttpError)?;

        let status = response.status().as_u16();
        if !response.status().is_success() {
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok())
                .map(|secs| secs * 1000);

            let body_text = response.text().await.unwrap_or_default();
            return Err(Self::handle_error(status, &body_text, retry_after));
        }

        let api_response: XAIResponse = response.json().await.map_err(|e| LLMError::ParseError(format!("Failed to parse xAI response: {}", e)))?;

        Ok(Self::parse_response(api_response))
    }

    /// Build the JSON request body for the Responses API.
    fn build_request_body(&self, request: &ResearchRequest) -> XAIRequest {
        let system_prompt = request.system_prompt.clone().unwrap_or_else(|| {
            "You are a research assistant. Provide thorough, well-sourced answers. \
             Always cite your sources with URLs when available. Be factual and concise."
                .to_string()
        });

        let mut input = vec![
            XAIMessage {
                role: "system".to_string(),
                content: system_prompt,
            },
            XAIMessage {
                role: "user".to_string(),
                content: request.query.clone(),
            },
        ];

        // If no sources specified, default doesn't add any messages
        let _ = &mut input;

        let mut tools = Vec::new();

        for source in &request.sources {
            match source {
                ResearchSource::Web => {
                    let mut tool = serde_json::json!({ "type": "web_search" });
                    if let Some(ref filters) = request.web_filters {
                        let mut filter_obj = serde_json::Map::new();
                        if let Some(ref domains) = filters.allowed_domains {
                            filter_obj.insert("allowed_domains".into(), serde_json::json!(domains));
                        }
                        if let Some(ref domains) = filters.excluded_domains {
                            filter_obj.insert("excluded_domains".into(), serde_json::json!(domains));
                        }
                        if !filter_obj.is_empty() {
                            tool.as_object_mut().unwrap().insert("filters".into(), serde_json::Value::Object(filter_obj));
                        }
                    }
                    tools.push(tool);
                }
                ResearchSource::X => {
                    let mut tool = serde_json::json!({ "type": "x_search" });
                    if let Some(ref filters) = request.x_filters {
                        let tool_obj = tool.as_object_mut().unwrap();
                        if let Some(ref handles) = filters.allowed_x_handles {
                            tool_obj.insert("allowed_x_handles".into(), serde_json::json!(handles));
                        }
                        if let Some(ref handles) = filters.excluded_x_handles {
                            tool_obj.insert("excluded_x_handles".into(), serde_json::json!(handles));
                        }
                        if let Some(ref date) = filters.from_date {
                            tool_obj.insert("from_date".into(), serde_json::json!(date));
                        }
                        if let Some(ref date) = filters.to_date {
                            tool_obj.insert("to_date".into(), serde_json::json!(date));
                        }
                    }
                    tools.push(tool);
                }
            }
        }

        XAIRequest {
            model: self.config.model.clone(),
            max_output_tokens: Some(self.config.max_tokens),
            input,
            tools,
        }
    }

    /// Parse the xAI API response into our ResearchResponse.
    fn parse_response(api_response: XAIResponse) -> ResearchResponse {
        let mut answer_parts = Vec::new();
        let citations = Vec::new();

        for item in &api_response.output {
            if item.item_type == "message" {
                for block in &item.content {
                    if block.block_type == "text" {
                        if let Some(ref text) = block.text {
                            answer_parts.push(text.clone());
                        }
                    }
                }
            }
        }

        let answer = answer_parts.join("\n\n");

        // TODO: Parse citations from search result metadata when xAI adds
        // structured citation data to the Responses API. For now, citations
        // are embedded inline in the answer text by Grok.

        ResearchResponse {
            answer,
            citations,
            usage: TokenUsage {
                input_tokens: api_response.usage.input_tokens,
                output_tokens: api_response.usage.output_tokens,
            },
        }
    }

    /// Map xAI error responses to LLMError.
    fn handle_error(status: u16, body: &str, retry_after_ms: Option<u64>) -> LLMError {
        let message = serde_json::from_str::<XAIErrorResponse>(body)
            .ok()
            .and_then(|e| e.error)
            .map(|e| e.message)
            .unwrap_or_else(|| body.to_string());

        match status {
            401 => LLMError::AuthError(message),
            429 => LLMError::RateLimited {
                retry_after_ms: retry_after_ms.unwrap_or(60000),
            },
            _ => LLMError::ApiError { status, message },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_defaults() {
        // Simulate env not set — use direct construction
        let config = GrokConfig {
            api_key: "test-key".to_string(),
            base_url: constants::XAI_DEFAULT_BASE_URL.to_string(),
            model: constants::XAI_RESEARCH_MODEL.to_string(),
            timeout_secs: constants::XAI_RESEARCH_TIMEOUT_SECS,
            max_tokens: constants::XAI_RESEARCH_MAX_TOKENS,
            max_search_turns: constants::XAI_RESEARCH_MAX_SEARCH_TURNS,
        };

        assert_eq!(config.base_url, "https://api.x.ai");
        assert_eq!(config.model, "grok-4-1-fast");
        assert_eq!(config.timeout_secs, 120);
        assert_eq!(config.max_tokens, 4096);
    }

    #[test]
    fn config_builder_methods() {
        let config = GrokConfig {
            api_key: "key".to_string(),
            base_url: constants::XAI_DEFAULT_BASE_URL.to_string(),
            model: constants::XAI_RESEARCH_MODEL.to_string(),
            timeout_secs: 120,
            max_tokens: 4096,
            max_search_turns: 10,
        }
        .with_model("grok-3")
        .with_base_url("http://localhost:8080");

        assert_eq!(config.model, "grok-3");
        assert_eq!(config.base_url, "http://localhost:8080");
    }

    #[test]
    fn build_request_body_both_sources() {
        let client = GrokResearchClient::new(GrokConfig {
            api_key: "test".to_string(),
            base_url: "https://api.x.ai".to_string(),
            model: "grok-4-1-fast".to_string(),
            timeout_secs: 120,
            max_tokens: 4096,
            max_search_turns: 10,
        })
        .unwrap();

        let req = ResearchRequest::new("What is Rust?");
        let body = client.build_request_body(&req);

        assert_eq!(body.model, "grok-4-1-fast");
        assert_eq!(body.input.len(), 2); // system + user
        assert_eq!(body.input[0].role, "system");
        assert_eq!(body.input[1].role, "user");
        assert_eq!(body.input[1].content, "What is Rust?");
        assert_eq!(body.tools.len(), 2); // web_search + x_search
        assert_eq!(body.tools[0]["type"], "web_search");
        assert_eq!(body.tools[1]["type"], "x_search");
    }

    #[test]
    fn build_request_body_web_only() {
        let client = GrokResearchClient::new(GrokConfig {
            api_key: "test".to_string(),
            base_url: "https://api.x.ai".to_string(),
            model: "grok-4-1-fast".to_string(),
            timeout_secs: 120,
            max_tokens: 4096,
            max_search_turns: 10,
        })
        .unwrap();

        let req = ResearchRequest {
            query: "test".to_string(),
            sources: vec![ResearchSource::Web],
            web_filters: Some(WebSearchFilters {
                allowed_domains: Some(vec!["rust-lang.org".to_string()]),
                excluded_domains: None,
            }),
            x_filters: None,
            system_prompt: None,
        };

        let body = client.build_request_body(&req);
        assert_eq!(body.tools.len(), 1);
        assert_eq!(body.tools[0]["type"], "web_search");
        assert_eq!(body.tools[0]["filters"]["allowed_domains"][0], "rust-lang.org");
    }

    #[test]
    fn build_request_body_x_with_filters() {
        let client = GrokResearchClient::new(GrokConfig {
            api_key: "test".to_string(),
            base_url: "https://api.x.ai".to_string(),
            model: "grok-4-1-fast".to_string(),
            timeout_secs: 120,
            max_tokens: 4096,
            max_search_turns: 10,
        })
        .unwrap();

        let req = ResearchRequest {
            query: "AI news".to_string(),
            sources: vec![ResearchSource::X],
            web_filters: None,
            x_filters: Some(XSearchFilters {
                allowed_x_handles: Some(vec!["elonmusk".to_string()]),
                excluded_x_handles: None,
                from_date: Some("2026-01-01".to_string()),
                to_date: None,
            }),
            system_prompt: None,
        };

        let body = client.build_request_body(&req);
        assert_eq!(body.tools.len(), 1);
        assert_eq!(body.tools[0]["type"], "x_search");
        assert_eq!(body.tools[0]["allowed_x_handles"][0], "elonmusk");
        assert_eq!(body.tools[0]["from_date"], "2026-01-01");
    }

    #[test]
    fn parse_response_extracts_text() {
        let api_response = XAIResponse {
            output: vec![XAIOutputItem {
                item_type: "message".to_string(),
                content: vec![
                    XAIContentBlock {
                        block_type: "text".to_string(),
                        text: Some("Rust is a systems programming language.".to_string()),
                    },
                    XAIContentBlock {
                        block_type: "text".to_string(),
                        text: Some("It focuses on safety and performance.".to_string()),
                    },
                ],
            }],
            usage: XAIUsage {
                input_tokens: 50,
                output_tokens: 100,
            },
        };

        let result = GrokResearchClient::parse_response(api_response);
        assert!(result.answer.contains("systems programming"));
        assert!(result.answer.contains("safety and performance"));
        assert_eq!(result.usage.input_tokens, 50);
        assert_eq!(result.usage.output_tokens, 100);
    }

    #[test]
    fn parse_response_skips_non_message_items() {
        let api_response = XAIResponse {
            output: vec![
                XAIOutputItem {
                    item_type: "tool_call".to_string(),
                    content: vec![XAIContentBlock {
                        block_type: "text".to_string(),
                        text: Some("should be ignored".to_string()),
                    }],
                },
                XAIOutputItem {
                    item_type: "message".to_string(),
                    content: vec![XAIContentBlock {
                        block_type: "text".to_string(),
                        text: Some("actual answer".to_string()),
                    }],
                },
            ],
            usage: XAIUsage::default(),
        };

        let result = GrokResearchClient::parse_response(api_response);
        assert_eq!(result.answer, "actual answer");
    }

    #[test]
    fn handle_error_auth() {
        let err = GrokResearchClient::handle_error(401, r#"{"error":{"message":"Invalid API key"}}"#, None);
        match err {
            LLMError::AuthError(msg) => assert!(msg.contains("Invalid API key")),
            other => panic!("Expected AuthError, got: {:?}", other),
        }
    }

    #[test]
    fn handle_error_rate_limited() {
        let err = GrokResearchClient::handle_error(429, r#"{"error":{"message":"Rate limited"}}"#, Some(5000));
        match err {
            LLMError::RateLimited { retry_after_ms } => assert_eq!(retry_after_ms, 5000),
            other => panic!("Expected RateLimited, got: {:?}", other),
        }
    }

    #[test]
    fn handle_error_generic() {
        let err = GrokResearchClient::handle_error(500, r#"{"error":{"message":"Internal error"}}"#, None);
        match err {
            LLMError::ApiError { status, message } => {
                assert_eq!(status, 500);
                assert!(message.contains("Internal error"));
            }
            other => panic!("Expected ApiError, got: {:?}", other),
        }
    }

    #[test]
    fn handle_error_unparseable_body() {
        let err = GrokResearchClient::handle_error(503, "not json", None);
        match err {
            LLMError::ApiError { status, message } => {
                assert_eq!(status, 503);
                assert_eq!(message, "not json");
            }
            other => panic!("Expected ApiError, got: {:?}", other),
        }
    }

    #[test]
    fn empty_api_key_rejected() {
        let result = GrokResearchClient::new(GrokConfig {
            api_key: String::new(),
            base_url: "https://api.x.ai".to_string(),
            model: "grok-4-1-fast".to_string(),
            timeout_secs: 120,
            max_tokens: 4096,
            max_search_turns: 10,
        });
        assert!(result.is_err());
    }

    #[test]
    fn research_request_defaults() {
        let req = ResearchRequest::new("test query");
        assert_eq!(req.query, "test query");
        assert_eq!(req.sources.len(), 2);
        assert!(req.sources.contains(&ResearchSource::Web));
        assert!(req.sources.contains(&ResearchSource::X));
        assert!(req.web_filters.is_none());
        assert!(req.x_filters.is_none());
        assert!(req.system_prompt.is_none());
    }
}
