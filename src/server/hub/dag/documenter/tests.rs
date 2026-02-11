#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use serde_json::json;

    use crate::config::protocols::{roles, vars};
    use crate::server::hub::dag::documenter::{
        build_documents_output, determine_persist_action, DocumentPersistAction,
    };
    use crate::server::hub::dag::utils::StepOutput;
    use crate::server::hub::protocols::context::{build_context_block, ContextDocument};
    use crate::server::hub::protocols::json_utils::extract_json_from_llm_response;
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

    // ── extract_json_from_llm_response tests ──────────────────────────────────────

    #[test]
    fn parse_strategy_output_raw_json() {
        let raw = r#"{"documents": [{"name": "API Ref"}]}"#;
        let result = extract_json_from_llm_response(raw);
        assert_eq!(result, raw);
    }

    #[test]
    fn parse_strategy_output_with_json_fence() {
        let fenced = r#"```json
{"documents": [{"name": "API Ref"}]}
```"#;
        let result = extract_json_from_llm_response(fenced);
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["documents"][0]["name"], "API Ref");
    }

    #[test]
    fn parse_strategy_output_with_bare_fence() {
        let fenced = r#"```
{"documents": [{"name": "Guide"}]}
```"#;
        let result = extract_json_from_llm_response(fenced);
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["documents"][0]["name"], "Guide");
    }

    #[test]
    fn parse_strategy_output_with_surrounding_text() {
        let messy = r#"Here is the plan:
{"documents": [{"name": "Overview"}]}
That's it."#;
        let result = extract_json_from_llm_response(messy);
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
        let result = extract_json_from_llm_response(content);
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

    // ── writer role resolve tests ──────────────────────────────────────

    #[test]
    fn writer_resolve_without_context() {
        let mut vars = HashMap::new();
        vars.insert(vars::system::DOC_NAME.into(), "Bitcoin Report".into());
        vars.insert(
            vars::agent::WRITER_PROMPT.into(),
            "Write a Bitcoin analysis report.".into(),
        );
        vars.insert(vars::system::SELECTED_CONTEXT.into(), String::new());
        vars.insert(
            vars::agent::RESEARCH_CONTENT.into(),
            "BTC is trading at $95,000 with strong support.".into(),
        );
        let ctx = roles::DOCUMENTER_WRITER.resolve(&vars);
        assert!(ctx.user_prompt.contains("Write a Bitcoin analysis report."));
        assert!(ctx.user_prompt.contains("Research findings:"));
        assert!(ctx.user_prompt.contains("BTC is trading at $95,000"));
        assert!(!ctx.user_prompt.contains("<context>"));
    }

    #[test]
    fn writer_resolve_with_context() {
        let context_block = build_context_block(
            &[],
            &[ContextDocument {
                short_id: "abc12345".into(),
                title: "Market Data".into(),
                content: "Current price: $95,000".into(),
            }],
        );
        let mut vars = HashMap::new();
        vars.insert(vars::system::DOC_NAME.into(), "Projection".into());
        vars.insert(
            vars::agent::WRITER_PROMPT.into(),
            "Write a projection report.".into(),
        );
        vars.insert(vars::system::SELECTED_CONTEXT.into(), context_block);
        vars.insert(
            vars::agent::RESEARCH_CONTENT.into(),
            "Research indicates bullish trend.".into(),
        );
        let ctx = roles::DOCUMENTER_WRITER.resolve(&vars);
        assert!(ctx.user_prompt.contains("Write a projection report."));
        assert!(ctx.user_prompt.contains("<context>"));
        assert!(ctx.user_prompt.contains("<document_abc12345"));
        assert!(ctx.user_prompt.contains("Market Data"));
        assert!(ctx.user_prompt.contains("Research findings:"));
        assert!(ctx
            .user_prompt
            .contains("Research indicates bullish trend."));
    }

    #[test]
    fn writer_resolve_context_between_instructions_and_research() {
        let ctx_block = "<context>\n<document_test>data</document_test>\n</context>";
        let mut vars = HashMap::new();
        vars.insert(vars::system::DOC_NAME.into(), "Test".into());
        vars.insert(
            vars::agent::WRITER_PROMPT.into(),
            "Instructions here.".into(),
        );
        vars.insert(vars::system::SELECTED_CONTEXT.into(), ctx_block.into());
        vars.insert(
            vars::agent::RESEARCH_CONTENT.into(),
            "Findings here.".into(),
        );
        let ctx = roles::DOCUMENTER_WRITER.resolve(&vars);
        let instr_pos = ctx.user_prompt.find("Instructions here.").unwrap();
        let ctx_pos = ctx.user_prompt.find("<context>").unwrap();
        let findings_pos = ctx.user_prompt.find("Research findings:").unwrap();
        assert!(
            instr_pos < ctx_pos,
            "instructions should come before context"
        );
        assert!(
            ctx_pos < findings_pos,
            "context should come before findings"
        );
    }

    // ── researcher role resolve tests ───────────────────────────────────

    #[test]
    fn researcher_resolve_without_context() {
        let mut vars = HashMap::new();
        vars.insert(vars::system::DOC_NAME.into(), "Bitcoin".into());
        vars.insert(
            vars::agent::RESEARCH_STRATEGY.into(),
            "Analyze Bitcoin price movements.".into(),
        );
        vars.insert(vars::system::SELECTED_CONTEXT.into(), String::new());
        let ctx = roles::DOCUMENTER_RESEARCHER.resolve(&vars);
        assert_eq!(ctx.user_prompt, "Analyze Bitcoin price movements.");
    }

    #[test]
    fn researcher_resolve_with_context() {
        let context_block = build_context_block(
            &[],
            &[ContextDocument {
                short_id: "def67890".into(),
                title: "Price History".into(),
                content: "Historical data from 2024.".into(),
            }],
        );
        let mut vars = HashMap::new();
        vars.insert(vars::system::DOC_NAME.into(), "Bitcoin".into());
        vars.insert(
            vars::agent::RESEARCH_STRATEGY.into(),
            "Analyze price trends.".into(),
        );
        vars.insert(vars::system::SELECTED_CONTEXT.into(), context_block);
        let ctx = roles::DOCUMENTER_RESEARCHER.resolve(&vars);
        assert!(ctx.user_prompt.starts_with("Analyze price trends."));
        assert!(ctx.user_prompt.contains("<context>"));
        assert!(ctx.user_prompt.contains("<document_def67890"));
        assert!(ctx.user_prompt.contains("Price History"));
    }

    #[test]
    fn researcher_resolve_preserves_multiline_strategy() {
        let strategy = "Step 1: Gather data\nStep 2: Analyze trends\nStep 3: Summarize";
        let mut vars = HashMap::new();
        vars.insert(vars::system::DOC_NAME.into(), "Test".into());
        vars.insert(vars::agent::RESEARCH_STRATEGY.into(), strategy.into());
        vars.insert(vars::system::SELECTED_CONTEXT.into(), String::new());
        let ctx = roles::DOCUMENTER_RESEARCHER.resolve(&vars);
        assert_eq!(ctx.user_prompt, strategy);
    }
}
