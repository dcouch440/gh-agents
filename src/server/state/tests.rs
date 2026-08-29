#[cfg(test)]
mod tests {
    //! Tests for application state

    use super::super::*;
    use crate::server::ws::events::{
        RoomEvent, RoomEventKind, SessionEvent, SessionEventKind, WorkflowEvent, WorkflowEventKind,
    };

    use super::super::test_helpers::default_mock_repos;

    fn make_state() -> AppState {
        let repos = default_mock_repos();
        let (state, _rx) = AppState::with_repos(None, repos, AppConfig::default());
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
        assert_eq!(state.jwt_secret().len(), 32);
    }

    #[test]
    fn subscribe_returns_receiver() {
        let state = make_state();
        let _rx = state.events().subscribe();
    }

    #[test]
    fn broadcast_session_no_panic() {
        let state = make_state();
        state.broadcast_session(SessionEvent {
            session_id: Uuid::new_v4(),
            user_id: None,
            kind: SessionEventKind::Created {
                title: "Test".to_string(),
                mode_id: "chat".to_string(),
            },
        });
    }

    #[test]
    fn broadcast_room_no_panic() {
        let state = make_state();
        state.broadcast_room(RoomEvent {
            room_session_id: Uuid::new_v4(),
            run_id: None,
            user_id: None,
            kind: RoomEventKind::TurnComplete { turn_number: 1 },
        });
    }

    #[test]
    fn broadcast_workflow_no_panic() {
        let state = make_state();
        state.broadcast_workflow(WorkflowEvent {
            run_id: Some(Uuid::new_v4()),
            workflow_id: Uuid::new_v4(),
            user_id: None,
            kind: WorkflowEventKind::Started { total_steps: 3 },
        });
    }

    #[test]
    fn broadcast_and_receive() {
        let state = make_state();
        let mut rx = state.events().subscribe();
        state.broadcast_session(SessionEvent {
            session_id: Uuid::new_v4(),
            user_id: None,
            kind: SessionEventKind::Deleted,
        });
        let envelope = rx.try_recv().unwrap();
        assert_eq!(envelope.topic, crate::server::ws::events::Topic::Session);
        assert!(!envelope.json.is_empty());
    }

    #[test]
    fn get_response_stream_creates_new() {
        let state = make_state();
        let msg_id = Uuid::new_v4();
        let (buf, _rx, done) = state.get_response_stream(msg_id);
        assert!(buf.is_empty());
        assert!(!done);
    }

    #[test]
    fn get_response_stream_returns_existing() {
        let state = make_state();
        let msg_id = Uuid::new_v4();
        let (_buf1, _rx1, _) = state.get_response_stream(msg_id);
        let (_buf2, _rx2, _) = state.get_response_stream(msg_id);
    }

    #[test]
    fn send_stream_chunk_no_stream() {
        let state = make_state();
        let result = state.send_stream_chunk(Uuid::new_v4(), StreamChunk::Token("hi".into()));
        assert!(!result);
    }

    #[test]
    fn send_stream_chunk_with_stream() {
        let state = make_state();
        let msg_id = Uuid::new_v4();
        state.ensure_response_stream(msg_id);
        let result = state.send_stream_chunk(msg_id, StreamChunk::Token("hi".into()));
        assert!(result);
    }

    #[test]
    fn buffered_stream_replays_chunks() {
        let state = make_state();
        let msg_id = Uuid::new_v4();
        state.ensure_response_stream(msg_id);

        // Send chunks with no SSE client connected
        state.send_stream_chunk(msg_id, StreamChunk::Token("hello ".into()));
        state.send_stream_chunk(msg_id, StreamChunk::Token("world".into()));
        state.send_stream_chunk(msg_id, StreamChunk::Done);

        // Late subscriber gets the full buffer
        let (buf, _rx, done) = state.get_response_stream(msg_id);
        assert_eq!(buf.len(), 3);
        assert!(done);
        assert!(matches!(&buf[0], StreamChunk::Token(t) if t == "hello "));
        assert!(matches!(&buf[1], StreamChunk::Token(t) if t == "world"));
        assert!(matches!(&buf[2], StreamChunk::Done));
    }

    #[test]
    fn remove_response_stream() {
        let state = make_state();
        let msg_id = Uuid::new_v4();
        state.ensure_response_stream(msg_id);
        state.remove_response_stream(msg_id);
        let result = state.send_stream_chunk(msg_id, StreamChunk::Done);
        assert!(!result);
    }

    // ── Shutdown / cancellation tests ──────────────────────────────────────

    #[test]
    fn shutdown_token_accessible() {
        let state = make_state();
        assert!(!state.shutdown_token().is_cancelled());
        state.shutdown_token().cancel();
        assert!(state.shutdown_token().is_cancelled());
    }

    #[test]
    fn cancel_all_executions_cancels_every_token() {
        let state = make_state();
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        let (_, token1) = state.register_cancellation(id1);
        let (_, token2) = state.register_cancellation(id2);
        assert!(!token1.is_cancelled());
        assert!(!token2.is_cancelled());

        let count = state.cancel_all_executions();
        assert_eq!(count, 2);
        assert!(token1.is_cancelled());
        assert!(token2.is_cancelled());
    }

    #[test]
    fn cancel_all_executions_returns_zero_when_empty() {
        let state = make_state();
        assert_eq!(state.cancel_all_executions(), 0);
    }

    #[test]
    fn active_execution_count_tracks_registrations() {
        let state = make_state();
        assert_eq!(state.active_execution_count(), 0);
        let id = Uuid::new_v4();
        let (registration, _) = state.register_cancellation(id);
        assert_eq!(state.active_execution_count(), 1);
        state.remove_cancellation(id, registration);
        assert_eq!(state.active_execution_count(), 0);
    }

    // ── Two registrations under one key ──────────────────────────────────
    //
    // Design pipelines are keyed by workflow id rather than by anything unique
    // to the run, so a second Design click while the first is still working
    // files a second token under the same key. These pin down that neither one
    // can displace or retire the other.

    #[test]
    fn registering_twice_under_one_key_keeps_both_cancellable() {
        let state = make_state();
        let id = Uuid::new_v4();

        let (_, first) = state.register_cancellation(id);
        let (_, second) = state.register_cancellation(id);

        assert_eq!(state.active_execution_count(), 2);
        assert!(state.cancel_execution(id));
        assert!(first.is_cancelled());
        assert!(second.is_cancelled());
    }

    #[test]
    fn removing_one_registration_leaves_the_other_cancellable() {
        let state = make_state();
        let id = Uuid::new_v4();

        let (first_registration, first) = state.register_cancellation(id);
        let (_, second) = state.register_cancellation(id);

        // The first pipeline finishes and cleans up after itself.
        state.remove_cancellation(id, first_registration);

        assert_eq!(state.active_execution_count(), 1);
        assert!(state.cancel_execution(id));
        assert!(!first.is_cancelled());
        assert!(second.is_cancelled());
    }

    #[test]
    fn removing_the_last_registration_drops_the_key() {
        let state = make_state();
        let id = Uuid::new_v4();

        let (first_registration, _) = state.register_cancellation(id);
        let (second_registration, _) = state.register_cancellation(id);
        state.remove_cancellation(id, first_registration);
        state.remove_cancellation(id, second_registration);

        assert_eq!(state.active_execution_count(), 0);
        // Nothing left to cancel — the endpoint answers "not generating".
        assert!(!state.cancel_execution(id));
    }

    #[test]
    fn removing_an_unknown_registration_is_a_no_op() {
        let state = make_state();
        let id = Uuid::new_v4();
        let (_, token) = state.register_cancellation(id);

        state.remove_cancellation(id, Uuid::new_v4());
        state.remove_cancellation(Uuid::new_v4(), Uuid::new_v4());

        assert_eq!(state.active_execution_count(), 1);
        assert!(state.cancel_execution(id));
        assert!(token.is_cancelled());
    }

    #[test]
    fn cancel_all_executions_counts_every_registration_not_every_key() {
        let state = make_state();
        let id = Uuid::new_v4();
        let (_, first) = state.register_cancellation(id);
        let (_, second) = state.register_cancellation(id);

        assert_eq!(state.cancel_all_executions(), 2);
        assert!(first.is_cancelled());
        assert!(second.is_cancelled());
    }

    // ── WebSocket connection tracking tests ──────────────────────────────

    #[test]
    fn ws_connection_acquire_and_release() {
        let state = make_state();
        let ip: std::net::IpAddr = "127.0.0.1".parse().unwrap();
        assert_eq!(state.ws_connection_count(), 0);

        assert!(state.try_acquire_ws_connection(ip));
        assert_eq!(state.ws_connection_count(), 1);

        state.release_ws_connection(ip);
        assert_eq!(state.ws_connection_count(), 0);
    }

    #[test]
    fn ws_connection_per_ip_limit() {
        let state = make_state();
        let ip: std::net::IpAddr = "10.0.0.1".parse().unwrap();

        for _ in 0..crate::constants::WS_MAX_CONNECTIONS_PER_IP {
            assert!(state.try_acquire_ws_connection(ip));
        }
        // Next one from same IP should be rejected
        assert!(!state.try_acquire_ws_connection(ip));

        // But a different IP should still work
        let other_ip: std::net::IpAddr = "10.0.0.2".parse().unwrap();
        assert!(state.try_acquire_ws_connection(other_ip));
    }

    #[test]
    fn ws_connection_release_cleans_up_ip_entry() {
        let state = make_state();
        let ip: std::net::IpAddr = "192.168.1.1".parse().unwrap();

        assert!(state.try_acquire_ws_connection(ip));
        assert!(state.try_acquire_ws_connection(ip));
        assert_eq!(state.ws_connection_count(), 2);

        state.release_ws_connection(ip);
        assert_eq!(state.ws_connection_count(), 1);

        state.release_ws_connection(ip);
        assert_eq!(state.ws_connection_count(), 0);
    }
}
