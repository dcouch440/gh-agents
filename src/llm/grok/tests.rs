#[cfg(test)]
mod tests {
    //! Tests for Grok provider

    use super::super::*;

    #[test]
    fn config_defaults() {
        // Simulate env not set — use direct construction
        let config = GrokConfig {
            api_key: "test-key".to_string(),
            base_url: constants::XAI_DEFAULT_BASE_URL.to_string(),
            model: constants::XAI_RESEARCH_MODEL.to_string(),
            timeout_secs: constants::XAI_RESEARCH_TIMEOUT_SECS,
            max_tokens: constants::XAI_RESEARCH_MAX_TOKENS,
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
        assert_eq!(
            body.tools[0]["filters"]["allowed_domains"][0],
            "rust-lang.org"
        );
    }

    #[test]
    fn build_request_body_x_with_filters() {
        let client = GrokResearchClient::new(GrokConfig {
            api_key: "test".to_string(),
            base_url: "https://api.x.ai".to_string(),
            model: "grok-4-1-fast".to_string(),
            timeout_secs: 120,
            max_tokens: 4096,
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
                        annotations: vec![],
                        text: Some("Rust is a systems programming language.".to_string()),
                    },
                    XAIContentBlock {
                        block_type: "text".to_string(),
                        annotations: vec![],
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
                        annotations: vec![],
                        text: Some("should be ignored".to_string()),
                    }],
                },
                XAIOutputItem {
                    item_type: "message".to_string(),
                    content: vec![XAIContentBlock {
                        block_type: "text".to_string(),
                        annotations: vec![],
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
        let err = GrokResearchClient::handle_error(
            401,
            r#"{"error":{"message":"Invalid API key"}}"#,
            None,
        );
        match err {
            LLMError::AuthError(msg) => assert!(msg.contains("Invalid API key")),
            other => panic!("Expected AuthError, got: {:?}", other),
        }
    }

    #[test]
    fn handle_error_rate_limited() {
        let err = GrokResearchClient::handle_error(
            429,
            r#"{"error":{"message":"Rate limited"}}"#,
            Some(5000),
        );
        match err {
            LLMError::RateLimited { retry_after_ms } => assert_eq!(retry_after_ms, 5000),
            other => panic!("Expected RateLimited, got: {:?}", other),
        }
    }

    #[test]
    fn handle_error_generic() {
        let err = GrokResearchClient::handle_error(
            500,
            r#"{"error":{"message":"Internal error"}}"#,
            None,
        );
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

    #[test]
    fn build_request_body_includes_inline_citations() {
        let client = GrokResearchClient::new(GrokConfig {
            api_key: "test".to_string(),
            base_url: "https://api.x.ai".to_string(),
            model: "grok-4-1-fast".to_string(),
            timeout_secs: 120,
            max_tokens: 4096,
        })
        .unwrap();

        let req = ResearchRequest::new("test");
        let body = client.build_request_body(&req);

        let serialized = serde_json::to_value(&body).unwrap();
        let include = serialized["include"].as_array().unwrap();
        assert_eq!(include.len(), 1);
        assert_eq!(include[0], "inline_citations");
    }

    #[test]
    fn parse_response_extracts_citations() {
        let api_response = XAIResponse {
            output: vec![XAIOutputItem {
                item_type: "message".to_string(),
                content: vec![XAIContentBlock {
                    block_type: "text".to_string(),
                    text: Some("Rust is great.".to_string()),
                    annotations: vec![
                        XAIAnnotation {
                            annotation_type: "url_citation".to_string(),
                            title: Some("Rust Lang".to_string()),
                            url: Some("https://rust-lang.org".to_string()),
                        },
                        XAIAnnotation {
                            annotation_type: "url_citation".to_string(),
                            title: Some("Rust Blog".to_string()),
                            url: Some("https://blog.rust-lang.org".to_string()),
                        },
                    ],
                }],
            }],
            usage: XAIUsage::default(),
        };

        let result = GrokResearchClient::parse_response(api_response);
        assert_eq!(result.citations.len(), 2);
        assert_eq!(result.citations[0].title, "Rust Lang");
        assert_eq!(result.citations[0].url, "https://rust-lang.org");
        assert_eq!(result.citations[0].source_type, "web");
        assert_eq!(result.citations[1].url, "https://blog.rust-lang.org");
    }

    #[test]
    fn parse_response_deduplicates_citations() {
        let api_response = XAIResponse {
            output: vec![XAIOutputItem {
                item_type: "message".to_string(),
                content: vec![
                    XAIContentBlock {
                        block_type: "text".to_string(),
                        text: Some("Part one.".to_string()),
                        annotations: vec![XAIAnnotation {
                            annotation_type: "url_citation".to_string(),
                            title: Some("Rust".to_string()),
                            url: Some("https://rust-lang.org".to_string()),
                        }],
                    },
                    XAIContentBlock {
                        block_type: "text".to_string(),
                        text: Some("Part two.".to_string()),
                        annotations: vec![XAIAnnotation {
                            annotation_type: "url_citation".to_string(),
                            title: Some("Rust".to_string()),
                            url: Some("https://rust-lang.org".to_string()),
                        }],
                    },
                ],
            }],
            usage: XAIUsage::default(),
        };

        let result = GrokResearchClient::parse_response(api_response);
        assert_eq!(result.citations.len(), 1);
        assert_eq!(result.citations[0].url, "https://rust-lang.org");
    }

    #[test]
    fn parse_response_no_annotations_empty_citations() {
        let api_response = XAIResponse {
            output: vec![XAIOutputItem {
                item_type: "message".to_string(),
                content: vec![XAIContentBlock {
                    block_type: "text".to_string(),
                    text: Some("No citations here.".to_string()),
                    annotations: vec![],
                }],
            }],
            usage: XAIUsage::default(),
        };

        let result = GrokResearchClient::parse_response(api_response);
        assert!(result.citations.is_empty());
        assert_eq!(result.answer, "No citations here.");
    }
}
