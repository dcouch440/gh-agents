//! Tests for streaming sinks

use crate::server::hub::streaming::{NullSink, SseSink, StreamSink};
use crate::server::state::AppState;
use uuid::Uuid;

#[tokio::test]
async fn null_sink_all_methods_are_noop() {
    let sink = NullSink;
    sink.token("hello").await;
    sink.tool_start("search", "t1").await;
    sink.tool_end("search", "t1").await;
    sink.error("boom").await;
    sink.done().await;
}

#[tokio::test]
async fn sse_sink_sends_chunks() {
    use crate::db::traits::MockServerRepo;
    use crate::types::AppConfig;
    use std::sync::Arc;

    let mut mock = MockServerRepo::new();
    mock.expect_health_check().returning(|| true);
    let repo: Arc<dyn crate::db::traits::ServerRepo> = Arc::new(mock);
    let (state, _rx) = AppState::with_repo(None, repo, AppConfig::default());

    let msg_id = Uuid::new_v4();
    state.ensure_response_stream(msg_id);

    let sink = SseSink::new(state.clone(), msg_id);
    sink.token("hi").await;
    sink.tool_start("search", "t1").await;
    sink.tool_end("search", "t1").await;
    sink.error("oops").await;
    sink.done().await;

    let (buf, _rx, done) = state.get_response_stream(msg_id);
    assert_eq!(buf.len(), 5);
    assert!(done);
}
