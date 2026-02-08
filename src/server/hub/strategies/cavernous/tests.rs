//! Tests for cavernous step strategy

use uuid::Uuid;

use crate::db::{AgentRow, DocumentSearchResult};
use crate::llm::TokenUsage;
use crate::server::hub::strategies::cavernous::{
    parse_selection_response, CavernousStepConfig, CavernousStepStrategy,
};
use crate::server::hub::strategy::ExecutionStrategy;

fn make_search_results() -> Vec<DocumentSearchResult> {
    vec![
        DocumentSearchResult {
            id: Uuid::new_v4(),
            title: "routing:full_stack".into(),
            summary: Some("Full stack implementation".into()),
            ref_tag: None,
            snippet: "Frontend + backend + DB".into(),
        },
        DocumentSearchResult {
            id: Uuid::new_v4(),
            title: "routing:backend_only".into(),
            summary: Some("Backend API implementation".into()),
            ref_tag: None,
            snippet: "REST API + DB schema".into(),
        },
        DocumentSearchResult {
            id: Uuid::new_v4(),
            title: "routing:microservice".into(),
            summary: Some("Microservice pattern".into()),
            ref_tag: None,
            snippet: "Containerized services".into(),
        },
    ]
}

fn make_agent() -> AgentRow {
    AgentRow {
        id: Uuid::new_v4(),
        user_id: None,
        tier: None,
        name: "test-agent".into(),
        system_prompt: "You are a test agent".into(),
        persona_style: None,
        model_provider: "anthropic".into(),
        model_id: "claude-sonnet-4-20250514".into(),
        model_max_tokens: 4096,
        model_temperature: 0.7,
        status: None,
        router_mode: None,
        router_id: None,
        output_schema_id: None,
        version: 1,
    }
}

#[test]
fn parse_selection_basic() {
    let response = r#"{"selected_index": 1, "reasoning": "backend_only is the best fit"}"#;
    let result = parse_selection_response(response, 3).unwrap();
    assert_eq!(result.selected_index, 1);
    assert_eq!(result.reasoning, "backend_only is the best fit");
}

#[test]
fn parse_selection_from_code_fence() {
    let response = "Here is my selection:\n\
        ```json\n\
        {\"selected_index\": 0, \"reasoning\": \"full stack covers everything\"}\n\
        ```";
    let result = parse_selection_response(response, 3).unwrap();
    assert_eq!(result.selected_index, 0);
}

#[test]
fn parse_selection_with_surrounding_text() {
    let response = "Based on my analysis, I recommend:\n\
        {\"selected_index\": 2, \"reasoning\": \"microservice is ideal\"}\n\
        This config matches the requirements.";
    let result = parse_selection_response(response, 3).unwrap();
    assert_eq!(result.selected_index, 2);
}

#[test]
fn parse_selection_out_of_range() {
    let response = r#"{"selected_index": 5, "reasoning": "bad"}"#;
    assert!(parse_selection_response(response, 3).is_err());
}

#[test]
fn parse_selection_missing_index() {
    let response = r#"{"reasoning": "no index"}"#;
    assert!(parse_selection_response(response, 3).is_err());
}

#[test]
fn parse_selection_no_json() {
    let response = "I think option 1 is best";
    assert!(parse_selection_response(response, 3).is_err());
}

#[tokio::test]
async fn phase_transition_search_to_select() {
    let config = CavernousStepConfig {
        agent: make_agent(),
        user_prompt: "Build a REST API".into(),
        user_id: Uuid::new_v4(),
        run_id: Uuid::new_v4(),
    };

    let strategy = CavernousStepStrategy::new(config);

    // Phase 1: should produce search messages
    let messages = strategy.build_messages("").await.unwrap();
    assert_eq!(messages.len(), 1);

    // Simulate Phase 1 completion
    let usage = TokenUsage {
        input_tokens: 10,
        output_tokens: 5,
    };
    strategy
        .on_complete("REST API user management routing config", &usage)
        .await
        .unwrap();
    assert_eq!(
        strategy.search_query().await.unwrap(),
        "REST API user management routing config"
    );

    // Transition to Phase 2
    let results = make_search_results();
    let expected_id = results[1].id;
    strategy.set_search_results(results).await;

    // Phase 2: should produce selection messages
    let messages = strategy.build_messages("").await.unwrap();
    assert_eq!(messages.len(), 1);

    // Simulate Phase 2 completion
    strategy
        .on_complete(
            r#"{"selected_index": 1, "reasoning": "backend_only matches"}"#,
            &usage,
        )
        .await
        .unwrap();
    assert_eq!(strategy.selected_document_id().await.unwrap(), expected_id);
    assert_eq!(
        strategy.selection_reasoning().await.unwrap(),
        "backend_only matches"
    );
}

#[test]
fn cavernous_strategy_properties() {
    let strategy = CavernousStepStrategy::new(CavernousStepConfig {
        agent: make_agent(),
        user_prompt: "test".into(),
        user_id: Uuid::new_v4(),
        run_id: Uuid::new_v4(),
    });

    assert_eq!(strategy.max_rounds(), 1);
    assert!(!strategy.streaming());
    assert!(strategy.tools().is_empty());
    assert!((strategy.temperature() - 0.2).abs() < f32::EPSILON);
    assert_eq!(strategy.model_id(), "claude-sonnet-4-20250514");
}
