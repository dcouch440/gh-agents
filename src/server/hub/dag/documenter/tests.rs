#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::server::hub::dag::documenter::{
        build_context_block, build_documents_output, compose_research_prompt, compose_write_prompt,
        determine_persist_action, extract_json_content, ContextDocument, DocumentPersistAction,
    };
    use crate::server::hub::dag::utils::StepOutput;
    use crate::server::ws::events::WorkflowEventKind;

    #[test]
    fn documenter_output_json_shape() {
        let statuses = vec![
            json!({"name": "API Reference", "status": "complete"}),
            json!({"name": "Architecture Guide", "status": "failed", "error": "research failed"}),
        ];

        let output = build_documents_output(statuses);
        let docs = output["documents"].as_array().unwrap();
        assert_eq!(docs.len(), 2);
        assert_eq!(docs[0]["name"], "API Reference");
        assert_eq!(docs[0]["status"], "complete");
        assert_eq!(docs[1]["status"], "failed");
        assert_eq!(docs[1]["error"], "research failed");
    }

    #[test]
    fn documenter_result_constructs_step_output() {
        let structured =
            build_documents_output(vec![json!({"name": "README", "status": "complete"})]);
        let raw = serde_json::to_string_pretty(&structured).unwrap();

        let output = StepOutput {
            variable_name: "documenter_output".into(),
            structured_output: Some(structured.clone()),
            raw_output: raw.clone(),
        };

        assert_eq!(output.variable_name, "documenter_output");
        assert!(output.structured_output.is_some());
        assert!(output.raw_output.contains("README"));
    }

    #[test]
    fn ws_documenter_phase_progress_serializes() {
        let event = WorkflowEventKind::DocumenterPhaseProgress {
            step_id: uuid::Uuid::nil(),
            phase: "research".into(),
            completed: 2,
            total: 3,
            document_name: Some("API Reference".into()),
        };

        let json = serde_json::to_value(&event).unwrap();
        // serde(rename_all = "snake_case") wraps in the variant name
        let inner = &json["documenter_phase_progress"];
        assert_eq!(inner["phase"], "research");
        assert_eq!(inner["completed"], 2);
        assert_eq!(inner["total"], 3);
        assert_eq!(inner["document_name"], "API Reference");
    }

    #[test]
    fn ws_documenter_phase_progress_omits_none_doc_name() {
        let event = WorkflowEventKind::DocumenterPhaseProgress {
            step_id: uuid::Uuid::nil(),
            phase: "strategy".into(),
            completed: 1,
            total: 1,
            document_name: None,
        };

        let json = serde_json::to_value(&event).unwrap();
        let inner = &json["documenter_phase_progress"];
        assert_eq!(inner["phase"], "strategy");
        assert!(inner.get("document_name").is_none());
    }

    // ── build_context_block tests ────────────────────────────────────────

    fn make_context_docs() -> Vec<ContextDocument> {
        vec![
            ContextDocument {
                short_id: "550e8400".into(),
                title: "API Spec".into(),
                content: "OpenAPI specification content".into(),
            },
            ContextDocument {
                short_id: "a1b2c3d4".into(),
                title: "Style Guide".into(),
                content: "Style guide content".into(),
            },
            ContextDocument {
                short_id: "deadbeef".into(),
                title: "Architecture".into(),
                content: "Architecture overview".into(),
            },
        ]
    }

    #[test]
    fn build_context_block_empty_docs_returns_empty() {
        let result = build_context_block(&[], &[]);
        assert!(result.is_empty());
    }

    #[test]
    fn build_context_block_empty_docs_with_ids_returns_empty() {
        let result = build_context_block(&["550e8400".into()], &[]);
        assert!(result.is_empty());
    }

    #[test]
    fn build_context_block_empty_ids_includes_all_docs() {
        let docs = make_context_docs();
        let result = build_context_block(&[], &docs);
        assert!(result.contains("<context>"));
        assert!(result.contains("</context>"));
        assert!(result.contains("<document_550e8400"));
        assert!(result.contains("<document_a1b2c3d4"));
        assert!(result.contains("<document_deadbeef"));
        assert!(result.contains("API Spec"));
        assert!(result.contains("Style Guide"));
        assert!(result.contains("Architecture"));
    }

    #[test]
    fn build_context_block_filters_by_assigned_ids() {
        let docs = make_context_docs();
        let ids = vec!["550e8400".into(), "deadbeef".into()];
        let result = build_context_block(&ids, &docs);
        assert!(result.contains("<document_550e8400"));
        assert!(result.contains("<document_deadbeef"));
        assert!(!result.contains("<document_a1b2c3d4"));
        assert!(result.contains("API Spec"));
        assert!(!result.contains("Style Guide"));
        assert!(result.contains("Architecture"));
    }

    #[test]
    fn build_context_block_no_matching_ids_returns_empty() {
        let docs = make_context_docs();
        let ids = vec!["00000000".into()];
        let result = build_context_block(&ids, &docs);
        assert!(result.is_empty());
    }

    #[test]
    fn build_context_block_single_doc_format() {
        let docs = vec![ContextDocument {
            short_id: "abcd1234".into(),
            title: "Test Doc".into(),
            content: "Test content".into(),
        }];
        let result = build_context_block(&[], &docs);
        assert!(result.starts_with("<context>"));
        assert!(result.ends_with("</context>"));
        assert!(result.contains("<document_abcd1234 title=\"Test Doc\">"));
        assert!(result.contains("Test content"));
        assert!(result.contains("</document_abcd1234>"));
    }

    // ── extract_json_content tests ──────────────────────────────────────

    #[test]
    fn parse_strategy_output_raw_json() {
        let raw = r#"{"documents": [{"name": "API Ref"}]}"#;
        let result = extract_json_content(raw);
        assert_eq!(result, raw);
    }

    #[test]
    fn parse_strategy_output_with_json_fence() {
        let fenced = r#"```json
{"documents": [{"name": "API Ref"}]}
```"#;
        let result = extract_json_content(fenced);
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["documents"][0]["name"], "API Ref");
    }

    #[test]
    fn parse_strategy_output_with_bare_fence() {
        let fenced = r#"```
{"documents": [{"name": "Guide"}]}
```"#;
        let result = extract_json_content(fenced);
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["documents"][0]["name"], "Guide");
    }

    #[test]
    fn parse_strategy_output_with_surrounding_text() {
        let messy = r#"Here is the plan:
{"documents": [{"name": "Overview"}]}
That's it."#;
        let result = extract_json_content(messy);
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["documents"][0]["name"], "Overview");
    }

    #[test]
    fn parse_strategy_output_with_json_fence_and_preamble() {
        let content = r#"I'll create a documentation plan:

```json
{"documents": [{"name": "Architecture", "capabilities": ["web_search"]}]}
```

This plan covers the main topics."#;
        let result = extract_json_content(content);
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["documents"][0]["name"], "Architecture");
    }

    // ── build_context_block with upstream context docs ──────────────────

    #[test]
    fn build_context_block_with_upstream_docs() {
        let upstream_docs = vec![
            ContextDocument {
                short_id: "up000001".into(),
                title: "Project Requirements".into(),
                content: "The system must support real-time notifications.".into(),
            },
            ContextDocument {
                short_id: "up000002".into(),
                title: "API Constraints".into(),
                content: "Rate limit: 100 req/s per user.".into(),
            },
        ];

        // No id filtering — should include all upstream docs
        let result = build_context_block(&[], &upstream_docs);
        assert!(result.contains("<context>"));
        assert!(result.contains("<document_up000001"));
        assert!(result.contains("Project Requirements"));
        assert!(result.contains("real-time notifications"));
        assert!(result.contains("<document_up000002"));
        assert!(result.contains("API Constraints"));
        assert!(result.contains("Rate limit"));
    }

    #[test]
    fn build_context_block_filters_upstream_by_id() {
        let docs = vec![
            ContextDocument {
                short_id: "up000001".into(),
                title: "Included".into(),
                content: "This should appear.".into(),
            },
            ContextDocument {
                short_id: "up000002".into(),
                title: "Excluded".into(),
                content: "This should not appear.".into(),
            },
        ];

        let ids = vec!["up000001".into()];
        let result = build_context_block(&ids, &docs);
        assert!(result.contains("<document_up000001"));
        assert!(result.contains("Included"));
        assert!(!result.contains("<document_up000002"));
        assert!(!result.contains("Excluded"));
    }

    // ── determine_persist_action tests ──────────────────────────────────

    #[test]
    fn persist_action_update_when_document_exists() {
        let doc_id = uuid::Uuid::new_v4();
        let def_id = uuid::Uuid::new_v4();
        let action = determine_persist_action(Some(doc_id), Some(def_id));
        assert_eq!(action, DocumentPersistAction::Update(doc_id));
    }

    #[test]
    fn persist_action_update_even_without_def() {
        let doc_id = uuid::Uuid::new_v4();
        let action = determine_persist_action(Some(doc_id), None);
        assert_eq!(action, DocumentPersistAction::Update(doc_id));
    }

    #[test]
    fn persist_action_create_when_no_document_but_def_exists() {
        let def_id = uuid::Uuid::new_v4();
        let action = determine_persist_action(None, Some(def_id));
        assert_eq!(action, DocumentPersistAction::CreateAndLink(def_id));
    }

    #[test]
    fn persist_action_skip_when_nothing_available() {
        let action = determine_persist_action(None, None);
        assert_eq!(action, DocumentPersistAction::Skip);
    }

    // ── compose_write_prompt tests ──────────────────────────────────────

    #[test]
    fn write_prompt_without_context() {
        let prompt = compose_write_prompt(
            "Write a Bitcoin analysis report.",
            "",
            "BTC is trading at $95,000 with strong support.",
        );
        assert!(prompt.starts_with("Write a Bitcoin analysis report."));
        assert!(prompt.contains("---\n\nResearch findings:"));
        assert!(prompt.contains("BTC is trading at $95,000"));
        assert!(!prompt.contains("<context>"));
    }

    #[test]
    fn write_prompt_with_context() {
        let ctx = build_context_block(
            &[],
            &[ContextDocument {
                short_id: "abc12345".into(),
                title: "Market Data".into(),
                content: "Current price: $95,000".into(),
            }],
        );
        let prompt = compose_write_prompt(
            "Write a projection report.",
            &ctx,
            "Research indicates bullish trend.",
        );
        assert!(prompt.starts_with("Write a projection report."));
        assert!(prompt.contains("<context>"));
        assert!(prompt.contains("<document_abc12345"));
        assert!(prompt.contains("Market Data"));
        assert!(prompt.contains("---\n\nResearch findings:"));
        assert!(prompt.contains("Research indicates bullish trend."));
    }

    #[test]
    fn write_prompt_context_appears_between_instructions_and_research() {
        let ctx = "<context>\n<document_test>data</document_test>\n</context>";
        let prompt = compose_write_prompt("Instructions here.", ctx, "Findings here.");
        let ctx_pos = prompt.find("<context>").unwrap();
        let instr_pos = prompt.find("Instructions here.").unwrap();
        let findings_pos = prompt.find("Research findings:").unwrap();
        assert!(instr_pos < ctx_pos, "instructions should come before context");
        assert!(ctx_pos < findings_pos, "context should come before findings");
    }

    // ── compose_research_prompt tests ───────────────────────────────────

    #[test]
    fn research_prompt_without_context() {
        let prompt = compose_research_prompt("Analyze Bitcoin price movements.", "");
        assert_eq!(prompt, "Analyze Bitcoin price movements.");
    }

    #[test]
    fn research_prompt_with_context() {
        let ctx = build_context_block(
            &[],
            &[ContextDocument {
                short_id: "def67890".into(),
                title: "Price History".into(),
                content: "Historical data from 2024.".into(),
            }],
        );
        let prompt = compose_research_prompt("Analyze price trends.", &ctx);
        assert!(prompt.starts_with("Analyze price trends."));
        assert!(prompt.contains("<context>"));
        assert!(prompt.contains("<document_def67890"));
        assert!(prompt.contains("Price History"));
    }

    #[test]
    fn research_prompt_preserves_multiline_strategy() {
        let strategy = "Step 1: Gather data\nStep 2: Analyze trends\nStep 3: Summarize";
        let prompt = compose_research_prompt(strategy, "");
        assert_eq!(prompt, strategy);
    }
}
