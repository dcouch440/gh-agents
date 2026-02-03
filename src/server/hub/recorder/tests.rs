//! Tests for execution recorder

use crate::db::traits::{MockAgentExecutionRepo, MockServerRepo, MockTokenLedgerRepo};
use crate::server::hub::recorder::ExecutionRecorder;
use crate::types::UserId;
use uuid::Uuid;

#[tokio::test]
async fn record_chat_message_global() {
    let mut mock = MockServerRepo::new();
    mock.expect_insert_chat_message()
        .withf(|_uid, _id, role, content| role == "user" && content == "hello")
        .returning(|_, _, _, _| Ok(()));

    let recorder = ExecutionRecorder::new(&mock, None, None);
    recorder
        .record_chat_message(UserId::new(), None, Uuid::new_v4(), "user", "hello")
        .await
        .unwrap();
}

#[tokio::test]
async fn record_chat_message_session() {
    let mut mock = MockServerRepo::new();
    mock.expect_insert_session_message()
        .withf(|_uid, _sid, _id, role, content| role == "assistant" && content == "hi there")
        .returning(|_, _, _, _, _| Ok(()));

    let session_id = Uuid::new_v4();
    let recorder = ExecutionRecorder::new(&mock, None, None);
    recorder
        .record_chat_message(
            UserId::new(),
            Some(session_id),
            Uuid::new_v4(),
            "assistant",
            "hi there",
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn record_tokens_without_repo_fails() {
    let mock = MockServerRepo::new();
    let recorder = ExecutionRecorder::new(&mock, None, None);
    let result = recorder
        .record_tokens(
            Uuid::new_v4(),
            Some(Uuid::new_v4()),
            "claude-3",
            100,
            50,
            0.01,
        )
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn record_tokens_with_repo() {
    let mock = MockServerRepo::new();
    let mut tl_mock = MockTokenLedgerRepo::new();
    tl_mock
        .expect_insert_ledger_entry()
        .returning(|uid, aeid, _model, inp, out, cost| {
            Ok(crate::db::TokenLedgerRow {
                id: Uuid::new_v4(),
                user_id: uid,
                agent_execution_id: aeid,
                model_id: "claude-3".to_string(),
                input_tokens: inp,
                output_tokens: out,
                cost_usd: cost,
                created_at: chrono::Utc::now(),
            })
        });

    let recorder = ExecutionRecorder::new(&mock, None, Some(&tl_mock));
    recorder
        .record_tokens(
            Uuid::new_v4(),
            Some(Uuid::new_v4()),
            "claude-3",
            100,
            50,
            0.01,
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn record_agent_execution_without_repo_fails() {
    let mock = MockServerRepo::new();
    let recorder = ExecutionRecorder::new(&mock, None, None);
    let result = recorder
        .record_agent_execution(
            Uuid::new_v4(),
            Uuid::new_v4(),
            None,
            false,
            None,
            "system prompt",
            "user prompt",
            None,
        )
        .await;
    assert!(result.is_err());
}
