//! Tests for application state

use super::*;
use crate::db::traits::MockServerRepo;

fn make_state() -> AppState {
    let mut mock = MockServerRepo::new();
    mock.expect_health_check().returning(|| true);
    let repo: Arc<dyn ServerRepo> = Arc::new(mock);
    let (state, _rx) = AppState::with_repo(None, repo, AppConfig::default());
    state
}

#[test]
fn stream_chunk_variants() {
    let token = StreamChunk::Token("hello".into());
    let done = StreamChunk::Done;
    let err = StreamChunk::Error("oops".into());
    match token {
        StreamChunk::Token(s) => assert_eq!(s, "hello"),
        _ => panic!(),
    }
    assert!(matches!(done, StreamChunk::Done));
    match err {
        StreamChunk::Error(s) => assert_eq!(s, "oops"),
        _ => panic!(),
    }
}

#[test]
fn orchestrator_message_construction() {
    let msg = ConsumerMessage {
        id: Uuid::new_v4(),
        user_id: UserId::new(),
        session_id: None,
        agent_id: None,
        content: "do stuff".into(),
        timestamp: Utc::now(),
    };
    assert_eq!(msg.content, "do stuff");
}

#[test]
fn app_state_new_creates_valid_state() {
    let state = make_state();
    assert_eq!(state.jwt_secret.len(), 32);
}

#[test]
fn subscribe_feed_returns_receiver() {
    let state = make_state();
    let _rx = state.subscribe_feed();
}

#[test]
fn subscribe_tasks_returns_receiver() {
    let state = make_state();
    let _rx = state.subscribe_tasks();
}

#[test]
fn subscribe_agents_returns_receiver() {
    let state = make_state();
    let _rx = state.subscribe_agents();
}

#[test]
fn broadcast_feed_no_panic() {
    let state = make_state();
    state.broadcast_feed(FeedUpdate {
        id: Uuid::new_v4(),
        agent_id: "a".into(),
        content: "c".into(),
        item_type: "info".into(),
        timestamp: Utc::now(),
        user_id: None,
    });
}

#[test]
fn broadcast_task_no_panic() {
    let state = make_state();
    state.broadcast_task(TaskUpdate {
        id: Uuid::new_v4(),
        status: "pending".into(),
        progress: None,
        assigned_agent: None,
        user_id: None,
    });
}

#[test]
fn broadcast_agent_no_panic() {
    let state = make_state();
    state.broadcast_agent(AgentUpdate {
        id: "agent-1".into(),
        status: "idle".into(),
        current_task: None,
        user_id: None,
    });
}

#[tokio::test]
async fn get_response_stream_creates_new() {
    let state = make_state();
    let msg_id = Uuid::new_v4();
    let (buf, _rx, done) = state.get_response_stream(msg_id).await;
    assert!(buf.is_empty());
    assert!(!done);
}

#[tokio::test]
async fn get_response_stream_returns_existing() {
    let state = make_state();
    let msg_id = Uuid::new_v4();
    let (_buf1, _rx1, _) = state.get_response_stream(msg_id).await;
    let (_buf2, _rx2, _) = state.get_response_stream(msg_id).await;
}

#[tokio::test]
async fn send_stream_chunk_no_stream() {
    let state = make_state();
    let result = state.send_stream_chunk(Uuid::new_v4(), StreamChunk::Token("hi".into())).await;
    assert!(!result);
}

#[tokio::test]
async fn send_stream_chunk_with_stream() {
    let state = make_state();
    let msg_id = Uuid::new_v4();
    state.ensure_response_stream(msg_id).await;
    let result = state.send_stream_chunk(msg_id, StreamChunk::Token("hi".into())).await;
    assert!(result);
}

#[tokio::test]
async fn buffered_stream_replays_chunks() {
    let state = make_state();
    let msg_id = Uuid::new_v4();
    state.ensure_response_stream(msg_id).await;

    // Send chunks with no SSE client connected
    state.send_stream_chunk(msg_id, StreamChunk::Token("hello ".into())).await;
    state.send_stream_chunk(msg_id, StreamChunk::Token("world".into())).await;
    state.send_stream_chunk(msg_id, StreamChunk::Done).await;

    // Late subscriber gets the full buffer
    let (buf, _rx, done) = state.get_response_stream(msg_id).await;
    assert_eq!(buf.len(), 3);
    assert!(done);
    assert!(matches!(&buf[0], StreamChunk::Token(t) if t == "hello "));
    assert!(matches!(&buf[1], StreamChunk::Token(t) if t == "world"));
    assert!(matches!(&buf[2], StreamChunk::Done));
}

#[tokio::test]
async fn remove_response_stream() {
    let state = make_state();
    let msg_id = Uuid::new_v4();
    state.ensure_response_stream(msg_id).await;
    state.remove_response_stream(msg_id).await;
    let result = state.send_stream_chunk(msg_id, StreamChunk::Done).await;
    assert!(!result);
}
