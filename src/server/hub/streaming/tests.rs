#[cfg(test)]
mod tests {
    //! Tests for streaming sinks

    use crate::server::hub::streaming::{NullSink, SseSink, StreamSink};
    use crate::server::state::AppState;
    use serde_json::json;
    use uuid::Uuid;

    #[tokio::test]
    async fn null_sink_all_methods_are_noop() {
        let sink = NullSink;
        sink.token("hello").await;
        sink.tool_start("search", "t1", &json!({"query": "test"}))
            .await;
        sink.tool_end("search", "t1", &json!({"results": []}))
            .await;
        sink.error("boom").await;
        sink.done().await;
    }

    #[tokio::test]
    async fn sse_sink_sends_chunks() {
        use crate::server::state::test_helpers::default_mock_repos;
        use crate::types::AppConfig;

        let repos = default_mock_repos();
        let (state, _rx) = AppState::with_repos(None, repos, AppConfig::default());

        let msg_id = Uuid::new_v4();
        state.ensure_response_stream(msg_id);

        let sink = SseSink::new(state.clone(), msg_id);
        sink.token("hi").await;
        sink.tool_start("search", "t1", &json!({"query": "test"}))
            .await;
        sink.tool_end("search", "t1", &json!({"ok": true})).await;
        sink.error("oops").await;
        sink.done().await;

        let (buf, _rx, done) = state.get_response_stream(msg_id);
        assert_eq!(buf.len(), 5);
        assert!(done);
    }
}
