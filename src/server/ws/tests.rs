#[cfg(test)]
mod tests {
    //! Tests for the unified WebSocket event system.

    use super::super::*;

    // ============================================================================
    // Topic serde
    // ============================================================================

    #[test]
    fn topic_serialize_snake_case() {
        assert_eq!(
            serde_json::to_string(&Topic::Workflow).unwrap(),
            r#""workflow""#
        );
        assert_eq!(serde_json::to_string(&Topic::Room).unwrap(), r#""room""#);
        assert_eq!(
            serde_json::to_string(&Topic::Session).unwrap(),
            r#""session""#
        );
    }

    #[test]
    fn topic_deserialize_snake_case() {
        let t: Topic = serde_json::from_str(r#""workflow""#).unwrap();
        assert_eq!(t, Topic::Workflow);
        let t: Topic = serde_json::from_str(r#""room""#).unwrap();
        assert_eq!(t, Topic::Room);
        let t: Topic = serde_json::from_str(r#""session""#).unwrap();
        assert_eq!(t, Topic::Session);
    }

    #[test]
    fn topic_invalid_deserialize_fails() {
        let result = serde_json::from_str::<Topic>(r#""invalid""#);
        assert!(result.is_err());
    }

    // ============================================================================
    // ClientMessage deserialization
    // ============================================================================

    #[test]
    fn client_message_subscribe_deserialize() {
        let json = r#"{"type": "subscribe", "topics": ["workflow", "room"]}"#;
        let msg: ClientMessage = serde_json::from_str(json).unwrap();
        match msg {
            ClientMessage::Subscribe { topics } => {
                assert_eq!(topics.len(), 2);
                assert_eq!(topics[0], Topic::Workflow);
                assert_eq!(topics[1], Topic::Room);
            }
            _ => panic!("Expected Subscribe"),
        }
    }

    #[test]
    fn client_message_unsubscribe_deserialize() {
        let json = r#"{"type": "unsubscribe", "topics": ["session"]}"#;
        let msg: ClientMessage = serde_json::from_str(json).unwrap();
        match msg {
            ClientMessage::Unsubscribe { topics } => {
                assert_eq!(topics, vec![Topic::Session]);
            }
            _ => panic!("Expected Unsubscribe"),
        }
    }

    #[test]
    fn client_message_subscribe_run_deserialize() {
        let json = r#"{"type": "subscribe_run", "run_id": "00000000-0000-0000-0000-000000000001"}"#;
        let msg: ClientMessage = serde_json::from_str(json).unwrap();
        match msg {
            ClientMessage::SubscribeRun { run_id } => {
                assert_eq!(run_id, uuid::Uuid::from_u128(1));
            }
            _ => panic!("Expected SubscribeRun"),
        }
    }

    #[test]
    fn client_message_unsubscribe_run_deserialize() {
        let json =
            r#"{"type": "unsubscribe_run", "run_id": "00000000-0000-0000-0000-000000000002"}"#;
        let msg: ClientMessage = serde_json::from_str(json).unwrap();
        match msg {
            ClientMessage::UnsubscribeRun { run_id } => {
                assert_eq!(run_id, uuid::Uuid::from_u128(2));
            }
            _ => panic!("Expected UnsubscribeRun"),
        }
    }

    #[test]
    fn client_message_ping_deserialize() {
        let json = r#"{"type": "ping", "ts": "2024-01-01T00:00:00.000Z"}"#;
        let msg: ClientMessage = serde_json::from_str(json).unwrap();
        match msg {
            ClientMessage::Ping { ts } => {
                assert_eq!(ts, "2024-01-01T00:00:00.000Z");
            }
            _ => panic!("Expected Ping"),
        }
    }

    #[test]
    fn client_message_invalid_type_fails() {
        let json = r#"{"type": "unknown", "topics": []}"#;
        let result = serde_json::from_str::<ClientMessage>(json);
        assert!(result.is_err());
    }

    #[test]
    fn client_message_missing_type_fails() {
        let json = r#"{"topics": ["workflow"]}"#;
        let result = serde_json::from_str::<ClientMessage>(json);
        assert!(result.is_err());
    }

    #[test]
    fn client_message_invalid_json_fails() {
        let result = serde_json::from_str::<ClientMessage>("not json");
        assert!(result.is_err());
    }

    // ============================================================================
    // ControlMessage serialization
    // ============================================================================

    #[test]
    fn control_message_subscribed_serialize() {
        let msg = ControlMessage::Subscribed {
            topics: vec![Topic::Workflow, Topic::Session],
        };
        let json = serde_json::to_string(&msg).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["type"], "subscribed");
        assert_eq!(value["topics"][0], "workflow");
        assert_eq!(value["topics"][1], "session");
    }

    #[test]
    fn control_message_error_serialize() {
        let msg = ControlMessage::Error {
            message: "bad request".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["type"], "error");
        assert_eq!(value["message"], "bad request");
    }

    #[test]
    fn control_message_pong_serialize() {
        let msg = ControlMessage::Pong {
            client_ts: "2024-01-01T00:00:00.000Z".to_string(),
            server_ts: chrono::Utc::now(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["type"], "pong");
        assert_eq!(value["client_ts"], "2024-01-01T00:00:00.000Z");
        assert!(value["server_ts"].is_string());
    }

    #[test]
    fn control_message_events_missed_serialize() {
        let msg = ControlMessage::EventsMissed {
            missed_count: 42,
            message: "Missed 42 events. Re-fetch state via REST.".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["type"], "events_missed");
        assert_eq!(value["missed_count"], 42);
        assert_eq!(
            value["message"],
            "Missed 42 events. Re-fetch state via REST."
        );
    }

    // ============================================================================
    // WireMessage serialization
    // ============================================================================

    #[test]
    fn wire_message_serialize_full() {
        let run_id = uuid::Uuid::new_v4();
        let user_id = uuid::Uuid::new_v4();
        let wire = WireMessage {
            topic: Topic::Workflow,
            event: "step_started".to_string(),
            ts: chrono::Utc::now(),
            run_id: Some(run_id),
            user_id: Some(user_id),
            data: serde_json::json!({"step_id": "abc"}),
        };
        let json = serde_json::to_string(&wire).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["topic"], "workflow");
        assert_eq!(value["event"], "step_started");
        assert!(value["ts"].is_string());
        assert_eq!(value["run_id"], run_id.to_string());
        assert_eq!(value["user_id"], user_id.to_string());
        assert_eq!(value["data"]["step_id"], "abc");
    }

    #[test]
    fn wire_message_skips_none_ids() {
        let wire = WireMessage {
            topic: Topic::Session,
            event: "created".to_string(),
            ts: chrono::Utc::now(),
            run_id: None,
            user_id: None,
            data: serde_json::json!({}),
        };
        let json = serde_json::to_string(&wire).unwrap();
        assert!(!json.contains("run_id"));
        assert!(!json.contains("user_id"));
    }

    // ============================================================================
    // WorkflowEvent → WireMessage
    // ============================================================================

    #[test]
    fn workflow_started_wire_message() {
        let run_id = uuid::Uuid::new_v4();
        let wf_id = uuid::Uuid::new_v4();
        let event = WorkflowEvent {
            run_id: Some(run_id),
            workflow_id: wf_id,
            user_id: None,
            kind: WorkflowEventKind::Started { total_steps: 5 },
        };
        let wire = ServerEvent::Workflow(event).into_wire_message();
        assert_eq!(wire.topic, Topic::Workflow);
        assert_eq!(wire.event, "started");
        assert_eq!(wire.run_id, Some(run_id));
        assert_eq!(wire.data["workflow_id"], wf_id.to_string());
        assert_eq!(wire.data["total_steps"], 5);
    }

    #[test]
    fn workflow_step_completed_wire_message() {
        let step_id = uuid::Uuid::new_v4();
        let agent_id = uuid::Uuid::new_v4();
        let event = WorkflowEvent {
            run_id: Some(uuid::Uuid::new_v4()),
            workflow_id: uuid::Uuid::new_v4(),
            user_id: Some(uuid::Uuid::new_v4()),
            kind: WorkflowEventKind::StepCompleted {
                step_id,
                step_name: "Research".to_string(),
                agent_id: Some(agent_id),
                output: None,
                input_tokens: Some(100),
                output_tokens: Some(50),
                duration_ms: Some(1234),
            },
        };
        let wire = ServerEvent::Workflow(event).into_wire_message();
        assert_eq!(wire.event, "step_completed");
        assert_eq!(wire.data["step_id"], step_id.to_string());
        assert_eq!(wire.data["step_name"], "Research");
        assert_eq!(wire.data["input_tokens"], 100);
        assert_eq!(wire.data["output_tokens"], 50);
        assert_eq!(wire.data["duration_ms"], 1234);
    }

    #[test]
    fn workflow_failed_wire_message() {
        let event = WorkflowEvent {
            run_id: Some(uuid::Uuid::new_v4()),
            workflow_id: uuid::Uuid::new_v4(),
            user_id: None,
            kind: WorkflowEventKind::Failed {
                error: "timeout".to_string(),
            },
        };
        let wire = ServerEvent::Workflow(event).into_wire_message();
        assert_eq!(wire.event, "failed");
        assert_eq!(wire.data["error"], "timeout");
    }

    // ============================================================================
    // RoomEvent → WireMessage
    // ============================================================================

    #[test]
    fn room_speaker_start_wire_message() {
        let session_id = uuid::Uuid::new_v4();
        let agent_id = uuid::Uuid::new_v4();
        let event = RoomEvent {
            room_session_id: session_id,
            run_id: None,
            user_id: None,
            kind: RoomEventKind::SpeakerStart {
                agent_id,
                agent_name: "Writer".to_string(),
                speaker_order: 1,
                turn_number: 2,
            },
        };
        let wire = ServerEvent::Room(event).into_wire_message();
        assert_eq!(wire.topic, Topic::Room);
        assert_eq!(wire.event, "speaker_start");
        assert_eq!(wire.data["room_session_id"], session_id.to_string());
        assert_eq!(wire.data["agent_name"], "Writer");
        assert_eq!(wire.data["speaker_order"], 1);
        assert_eq!(wire.data["turn_number"], 2);
    }

    #[test]
    fn room_session_complete_wire_message() {
        let event = RoomEvent {
            room_session_id: uuid::Uuid::new_v4(),
            run_id: Some(uuid::Uuid::new_v4()),
            user_id: None,
            kind: RoomEventKind::SessionComplete { turn_number: 5 },
        };
        let wire = ServerEvent::Room(event).into_wire_message();
        assert_eq!(wire.event, "session_complete");
        assert_eq!(wire.data["turn_number"], 5);
        assert!(wire.run_id.is_some());
    }

    // ============================================================================
    // SessionEvent → WireMessage
    // ============================================================================

    #[test]
    fn session_created_wire_message() {
        let session_id = uuid::Uuid::new_v4();
        let user_id = uuid::Uuid::new_v4();
        let event = SessionEvent {
            session_id,
            user_id: Some(user_id),
            kind: SessionEventKind::Created {
                title: "Hello".to_string(),
                mode_id: "chat".to_string(),
            },
        };
        let wire = ServerEvent::Session(event).into_wire_message();
        assert_eq!(wire.topic, Topic::Session);
        assert_eq!(wire.event, "created");
        assert_eq!(wire.user_id, Some(user_id));
        assert_eq!(wire.data["session_id"], session_id.to_string());
        assert_eq!(wire.data["title"], "Hello");
        assert_eq!(wire.data["mode_id"], "chat");
    }

    #[test]
    fn session_deleted_wire_message() {
        let session_id = uuid::Uuid::new_v4();
        let event = SessionEvent {
            session_id,
            user_id: None,
            kind: SessionEventKind::Deleted,
        };
        let wire = ServerEvent::Session(event).into_wire_message();
        assert_eq!(wire.event, "deleted");
        assert_eq!(wire.data["session_id"], session_id.to_string());
    }

    // ============================================================================
    // ServerEvent accessors
    // ============================================================================

    #[test]
    fn server_event_topic() {
        let wf = ServerEvent::Workflow(WorkflowEvent {
            run_id: Some(uuid::Uuid::nil()),
            workflow_id: uuid::Uuid::nil(),
            user_id: None,
            kind: WorkflowEventKind::Started { total_steps: 1 },
        });
        assert_eq!(wf.topic(), Topic::Workflow);

        let room = ServerEvent::Room(RoomEvent {
            room_session_id: uuid::Uuid::nil(),
            run_id: None,
            user_id: None,
            kind: RoomEventKind::TurnComplete { turn_number: 1 },
        });
        assert_eq!(room.topic(), Topic::Room);

        let session = ServerEvent::Session(SessionEvent {
            session_id: uuid::Uuid::nil(),
            user_id: None,
            kind: SessionEventKind::Deleted,
        });
        assert_eq!(session.topic(), Topic::Session);
    }

    #[test]
    fn server_event_user_id() {
        let uid = uuid::Uuid::new_v4();
        let event = ServerEvent::Workflow(WorkflowEvent {
            run_id: Some(uuid::Uuid::nil()),
            workflow_id: uuid::Uuid::nil(),
            user_id: Some(uid),
            kind: WorkflowEventKind::Completed { duration_ms: None },
        });
        assert_eq!(event.user_id(), Some(uid));

        let event = ServerEvent::Session(SessionEvent {
            session_id: uuid::Uuid::nil(),
            user_id: None,
            kind: SessionEventKind::Deleted,
        });
        assert_eq!(event.user_id(), None);
    }

    #[test]
    fn server_event_run_id() {
        let rid = uuid::Uuid::new_v4();
        let event = ServerEvent::Workflow(WorkflowEvent {
            run_id: Some(rid),
            workflow_id: uuid::Uuid::nil(),
            user_id: None,
            kind: WorkflowEventKind::Started { total_steps: 1 },
        });
        assert_eq!(event.run_id(), Some(rid));

        let event = ServerEvent::Session(SessionEvent {
            session_id: uuid::Uuid::nil(),
            user_id: None,
            kind: SessionEventKind::Deleted,
        });
        assert_eq!(event.run_id(), None);
    }

    // ============================================================================
    // handle_client_message (now sync — uses std::sync::Mutex)
    // ============================================================================

    #[test]
    fn handle_subscribe_topics() {
        let topics: TopicSubscriptions = Arc::new(Mutex::new(HashSet::new()));
        let run_subs: RunSubscriptions = Arc::new(Mutex::new(HashSet::new()));
        let msg = ClientMessage::Subscribe {
            topics: vec![Topic::Workflow, Topic::Room],
        };
        let response = handle_client_message(msg, &topics, &run_subs);
        assert!(response.is_some());
        if let Some(ControlMessage::Subscribed { topics: current }) = response {
            assert_eq!(current.len(), 2);
        } else {
            panic!("Expected Subscribed");
        }
    }

    #[test]
    fn handle_unsubscribe_topics() {
        let topics: TopicSubscriptions = Arc::new(Mutex::new(HashSet::new()));
        topics.lock().unwrap().insert(Topic::Workflow);
        topics.lock().unwrap().insert(Topic::Session);

        let run_subs: RunSubscriptions = Arc::new(Mutex::new(HashSet::new()));
        let msg = ClientMessage::Unsubscribe {
            topics: vec![Topic::Workflow],
        };
        let response = handle_client_message(msg, &topics, &run_subs);
        if let Some(ControlMessage::Subscribed { topics: current }) = response {
            assert_eq!(current.len(), 1);
            assert!(current.contains(&Topic::Session));
        } else {
            panic!("Expected Subscribed");
        }
    }

    #[test]
    fn handle_subscribe_run() {
        let topics: TopicSubscriptions = Arc::new(Mutex::new(HashSet::new()));
        let run_subs: RunSubscriptions = Arc::new(Mutex::new(HashSet::new()));
        let run_id = uuid::Uuid::new_v4();
        let msg = ClientMessage::SubscribeRun { run_id };
        let response = handle_client_message(msg, &topics, &run_subs);
        assert!(response.is_none());
        assert!(run_subs.lock().unwrap().contains(&run_id));
    }

    #[test]
    fn handle_unsubscribe_run() {
        let topics: TopicSubscriptions = Arc::new(Mutex::new(HashSet::new()));
        let run_subs: RunSubscriptions = Arc::new(Mutex::new(HashSet::new()));
        let run_id = uuid::Uuid::new_v4();
        run_subs.lock().unwrap().insert(run_id);

        let msg = ClientMessage::UnsubscribeRun { run_id };
        let response = handle_client_message(msg, &topics, &run_subs);
        assert!(response.is_none());
        assert!(!run_subs.lock().unwrap().contains(&run_id));
    }

    #[test]
    fn handle_ping() {
        let topics: TopicSubscriptions = Arc::new(Mutex::new(HashSet::new()));
        let run_subs: RunSubscriptions = Arc::new(Mutex::new(HashSet::new()));
        let msg = ClientMessage::Ping {
            ts: "2024-01-01T00:00:00.000Z".to_string(),
        };
        let response = handle_client_message(msg, &topics, &run_subs);
        if let Some(ControlMessage::Pong { client_ts, .. }) = response {
            assert_eq!(client_ts, "2024-01-01T00:00:00.000Z");
        } else {
            panic!("Expected Pong");
        }
    }

    #[test]
    fn subscribe_idempotent() {
        let topics: TopicSubscriptions = Arc::new(Mutex::new(HashSet::new()));
        let run_subs: RunSubscriptions = Arc::new(Mutex::new(HashSet::new()));

        let msg = ClientMessage::Subscribe {
            topics: vec![Topic::Workflow],
        };
        handle_client_message(msg, &topics, &run_subs);

        let msg = ClientMessage::Subscribe {
            topics: vec![Topic::Workflow],
        };
        let response = handle_client_message(msg, &topics, &run_subs);
        if let Some(ControlMessage::Subscribed { topics: current }) = response {
            assert_eq!(current.len(), 1);
        } else {
            panic!("Expected Subscribed");
        }
    }

    // ============================================================================
    // Miscellaneous
    // ============================================================================

    #[test]
    fn ping_interval_is_30_seconds() {
        assert_eq!(PING_INTERVAL, Duration::from_secs(30));
    }

    #[test]
    fn pong_timeout_is_90_seconds() {
        assert_eq!(PONG_TIMEOUT, Duration::from_secs(90));
        // 3 missed pings (30s each) before disconnect
        assert!(PONG_TIMEOUT > PING_INTERVAL * 2);
    }

    // ============================================================================
    // Test helpers
    // ============================================================================

    fn make_workflow_event(user_id: Option<uuid::Uuid>, run_id: uuid::Uuid) -> ServerEvent {
        ServerEvent::Workflow(WorkflowEvent {
            run_id: Some(run_id),
            workflow_id: uuid::Uuid::new_v4(),
            user_id,
            kind: WorkflowEventKind::Started { total_steps: 1 },
        })
    }

    fn make_session_event(user_id: Option<uuid::Uuid>) -> ServerEvent {
        ServerEvent::Session(SessionEvent {
            session_id: uuid::Uuid::new_v4(),
            user_id,
            kind: SessionEventKind::Deleted,
        })
    }

    /// Pure function replicating the filtering logic from `handle_socket`.
    /// This allows exhaustive testing without a real WebSocket connection.
    fn event_passes_filters(
        evt: &ServerEvent,
        subscribed_topics: &HashSet<Topic>,
        user_id: Option<uuid::Uuid>,
        run_subs: &HashSet<uuid::Uuid>,
    ) -> bool {
        // Topic filter
        if !subscribed_topics.contains(&evt.topic()) {
            return false;
        }
        // User filter: events with a user_id only go to that user
        if let Some(event_uid) = evt.user_id() {
            if !user_id.map(|u| u == event_uid).unwrap_or(false) {
                return false;
            }
        }
        // Run filter: if client has run subscriptions, only matching events pass
        if let Some(rid) = evt.run_id() {
            if !run_subs.is_empty() && !run_subs.contains(&rid) {
                return false;
            }
        }
        true
    }

    // ============================================================================
    // EventBus tests
    // ============================================================================

    use crate::server::state::EventBus;

    #[tokio::test]
    async fn eventbus_broadcast_received_by_subscriber() {
        let bus = EventBus::new();
        let mut rx = bus.subscribe();
        let run_id = uuid::Uuid::new_v4();
        bus.broadcast(make_workflow_event(None, run_id));

        let envelope = rx.recv().await.unwrap();
        assert_eq!(envelope.topic, Topic::Workflow);
        assert_eq!(envelope.run_id, Some(run_id));
        assert!(!envelope.json.is_empty());
    }

    #[tokio::test]
    async fn eventbus_multiple_subscribers_receive_same_event() {
        let bus = EventBus::new();
        let mut rx1 = bus.subscribe();
        let mut rx2 = bus.subscribe();
        let run_id = uuid::Uuid::new_v4();
        bus.broadcast(make_workflow_event(None, run_id));

        let e1 = rx1.recv().await.unwrap();
        let e2 = rx2.recv().await.unwrap();
        assert_eq!(e1.run_id, Some(run_id));
        assert_eq!(e2.run_id, Some(run_id));
        // Both receivers share the same Arc — same json content
        assert_eq!(e1.json, e2.json);
    }

    #[tokio::test]
    async fn eventbus_no_subscribers_doesnt_panic() {
        let bus = EventBus::new();
        // No subscribers — broadcast should silently succeed (fire-and-forget)
        bus.broadcast(make_workflow_event(None, uuid::Uuid::new_v4()));
    }

    #[tokio::test]
    async fn eventbus_lagged_receiver_gets_lagged_error() {
        use tokio::sync::broadcast::error::RecvError;

        let bus = EventBus::with_capacity(2);
        let mut rx = bus.subscribe();

        // Broadcast 3 events into a buffer of size 2 — first event is evicted
        bus.broadcast(make_workflow_event(None, uuid::Uuid::new_v4()));
        bus.broadcast(make_workflow_event(None, uuid::Uuid::new_v4()));
        bus.broadcast(make_workflow_event(None, uuid::Uuid::new_v4()));

        match rx.recv().await {
            Err(RecvError::Lagged(n)) => assert_eq!(n, 1),
            other => panic!("Expected Lagged(1), got {:?}", other),
        }
    }

    #[tokio::test]
    async fn eventbus_subscriber_only_receives_after_subscribe() {
        let bus = EventBus::new();

        // Broadcast before subscribing
        bus.broadcast(make_workflow_event(None, uuid::Uuid::new_v4()));

        // Now subscribe
        let mut rx = bus.subscribe();

        // Broadcast after subscribing
        let run_id = uuid::Uuid::new_v4();
        bus.broadcast(make_workflow_event(None, run_id));

        // Should only get the second event
        let envelope = rx.recv().await.unwrap();
        assert_eq!(envelope.run_id, Some(run_id));
    }

    #[tokio::test]
    async fn eventbus_closed_channel_returns_closed() {
        use tokio::sync::broadcast::error::RecvError;

        let bus = EventBus::new();
        let mut rx = bus.subscribe();

        // Drop the bus (and its sender) to close the channel
        drop(bus);

        match rx.recv().await {
            Err(RecvError::Closed) => {} // expected
            other => panic!("Expected Closed, got {:?}", other),
        }
    }

    #[tokio::test]
    async fn eventbus_envelope_contains_valid_json() {
        let bus = EventBus::new();
        let mut rx = bus.subscribe();
        let run_id = uuid::Uuid::new_v4();
        bus.broadcast(make_workflow_event(None, run_id));

        let envelope = rx.recv().await.unwrap();
        // JSON should be valid and parseable
        let value: serde_json::Value = serde_json::from_str(&envelope.json).unwrap();
        assert_eq!(value["topic"], "workflow");
        assert_eq!(value["event"], "started");
        assert!(value["data"]["total_steps"].is_number());
    }

    #[tokio::test]
    async fn eventbus_seq_monotonically_increasing() {
        let bus = EventBus::new();
        let mut rx = bus.subscribe();

        bus.broadcast(make_workflow_event(None, uuid::Uuid::new_v4()));
        bus.broadcast(make_workflow_event(None, uuid::Uuid::new_v4()));
        bus.broadcast(make_workflow_event(None, uuid::Uuid::new_v4()));

        let e1 = rx.recv().await.unwrap();
        let e2 = rx.recv().await.unwrap();
        let e3 = rx.recv().await.unwrap();

        assert_eq!(e1.seq, 0);
        assert_eq!(e2.seq, 1);
        assert_eq!(e3.seq, 2);
    }

    #[tokio::test]
    async fn eventbus_envelope_preserves_user_id() {
        let bus = EventBus::new();
        let mut rx = bus.subscribe();
        let user_id = uuid::Uuid::new_v4();
        bus.broadcast(make_workflow_event(Some(user_id), uuid::Uuid::new_v4()));

        let envelope = rx.recv().await.unwrap();
        assert_eq!(envelope.user_id, Some(user_id));
    }

    #[tokio::test]
    async fn eventbus_envelope_session_has_no_run_id() {
        let bus = EventBus::new();
        let mut rx = bus.subscribe();
        bus.broadcast(make_session_event(None));

        let envelope = rx.recv().await.unwrap();
        assert_eq!(envelope.topic, Topic::Session);
        assert_eq!(envelope.run_id, None);
    }

    // ============================================================================
    // Event filtering tests
    // ============================================================================

    #[test]
    fn filter_passes_subscribed_topic() {
        let topics = HashSet::from([Topic::Workflow]);
        let evt = make_workflow_event(None, uuid::Uuid::new_v4());
        assert!(event_passes_filters(&evt, &topics, None, &HashSet::new()));
    }

    #[test]
    fn filter_rejects_unsubscribed_topic() {
        // Subscribed to Room only → a Workflow event is rejected
        let topics = HashSet::from([Topic::Room]);
        let evt = make_workflow_event(None, uuid::Uuid::new_v4());
        assert!(!event_passes_filters(&evt, &topics, None, &HashSet::new()));

        // Subscribed to Workflow only → a Session event is rejected
        let topics = HashSet::from([Topic::Workflow]);
        let evt = make_session_event(None);
        assert!(!event_passes_filters(&evt, &topics, None, &HashSet::new()));
    }

    #[test]
    fn filter_user_scoped_event_passes_for_correct_user() {
        let user_id = uuid::Uuid::new_v4();
        let topics = HashSet::from([Topic::Workflow]);
        let evt = make_workflow_event(Some(user_id), uuid::Uuid::new_v4());
        assert!(event_passes_filters(
            &evt,
            &topics,
            Some(user_id),
            &HashSet::new()
        ));
    }

    #[test]
    fn filter_user_scoped_event_rejected_for_wrong_user() {
        let event_user = uuid::Uuid::new_v4();
        let client_user = uuid::Uuid::new_v4();
        let topics = HashSet::from([Topic::Workflow]);
        let evt = make_workflow_event(Some(event_user), uuid::Uuid::new_v4());
        assert!(!event_passes_filters(
            &evt,
            &topics,
            Some(client_user),
            &HashSet::new()
        ));
    }

    #[test]
    fn filter_unscoped_event_passes_for_any_user() {
        let topics = HashSet::from([Topic::Workflow]);
        let evt = make_workflow_event(None, uuid::Uuid::new_v4());
        // Client has a user_id, but event is unscoped (user_id=None) → passes
        assert!(event_passes_filters(
            &evt,
            &topics,
            Some(uuid::Uuid::new_v4()),
            &HashSet::new()
        ));
    }

    #[test]
    fn filter_run_scoped_passes_when_subscribed() {
        let run_id = uuid::Uuid::new_v4();
        let topics = HashSet::from([Topic::Workflow]);
        let run_subs = HashSet::from([run_id]);
        let evt = make_workflow_event(None, run_id);
        assert!(event_passes_filters(&evt, &topics, None, &run_subs));
    }

    #[test]
    fn filter_run_scoped_rejected_when_not_subscribed() {
        let run_id = uuid::Uuid::new_v4();
        let other_run = uuid::Uuid::new_v4();
        let topics = HashSet::from([Topic::Workflow]);
        let run_subs = HashSet::from([other_run]);
        let evt = make_workflow_event(None, run_id);
        assert!(!event_passes_filters(&evt, &topics, None, &run_subs));
    }

    #[test]
    fn filter_run_passes_when_no_run_subscriptions() {
        let topics = HashSet::from([Topic::Workflow]);
        let evt = make_workflow_event(None, uuid::Uuid::new_v4());
        // Empty run subscriptions → all run events pass
        assert!(event_passes_filters(&evt, &topics, None, &HashSet::new()));
    }

    // ============================================================================
    // Step-config event wire messages
    // ============================================================================

    #[test]
    fn workflow_step_config_updated_wire_message() {
        let step_id = uuid::Uuid::new_v4();
        let wf_id = uuid::Uuid::new_v4();
        let event = WorkflowEvent {
            run_id: None,
            workflow_id: wf_id,
            user_id: Some(uuid::Uuid::new_v4()),
            kind: WorkflowEventKind::StepConfigUpdated { step_id },
        };
        let wire = ServerEvent::Workflow(event).into_wire_message();
        assert_eq!(wire.event, "step_config_updated");
        assert_eq!(wire.run_id, None);
        assert_eq!(wire.data["workflow_id"], wf_id.to_string());
        assert_eq!(wire.data["step_id"], step_id.to_string());
    }

    #[test]
    fn filter_none_run_id_passes_when_no_run_subscriptions() {
        let topics = HashSet::from([Topic::Workflow]);
        let evt = ServerEvent::Workflow(WorkflowEvent {
            run_id: None,
            workflow_id: uuid::Uuid::new_v4(),
            user_id: None,
            kind: WorkflowEventKind::RosterChanged {
                step_id: uuid::Uuid::new_v4(),
            },
        });
        assert!(event_passes_filters(&evt, &topics, None, &HashSet::new()));
    }

    #[test]
    fn workflow_plan_updated_wire_message() {
        let step_id = uuid::Uuid::new_v4();
        let wf_id = uuid::Uuid::new_v4();
        let event = WorkflowEvent {
            run_id: None,
            workflow_id: wf_id,
            user_id: Some(uuid::Uuid::new_v4()),
            kind: WorkflowEventKind::PlanUpdated {
                step_id,
                content: "## Direction\n- Build auth system".to_string(),
            },
        };
        let wire = ServerEvent::Workflow(event).into_wire_message();
        assert_eq!(wire.event, "plan_updated");
        assert_eq!(wire.run_id, None);
        assert_eq!(wire.data["workflow_id"], wf_id.to_string());
        assert_eq!(wire.data["step_id"], step_id.to_string());
        assert_eq!(wire.data["content"], "## Direction\n- Build auth system");
    }

    #[test]
    fn filter_none_run_id_passes_with_run_subscriptions() {
        let topics = HashSet::from([Topic::Workflow]);
        let evt = ServerEvent::Workflow(WorkflowEvent {
            run_id: None,
            workflow_id: uuid::Uuid::new_v4(),
            user_id: None,
            kind: WorkflowEventKind::RosterChanged {
                step_id: uuid::Uuid::new_v4(),
            },
        });
        let run_subs = HashSet::from([uuid::Uuid::new_v4()]);
        // None run_id skips the run filter entirely → passes
        assert!(event_passes_filters(&evt, &topics, None, &run_subs));
    }

    // ============================================================================
    // Canvas sync wire contract
    // ============================================================================
    //
    // The frontend blocks Generate until it has seen an ack for its newest
    // `seq`. A rename or shape change on either side of this contract would not
    // fail to compile — it would just make every Generate wait out its timeout
    // and then read a board the server had not finished writing.

    #[test]
    fn canvas_node_created_carries_seq() {
        let json = r#"{
            "type": "canvas_node_created",
            "workflow_id": "550e8400-e29b-41d4-a716-446655440000",
            "element_id": "box-1",
            "x": 1.0, "y": 2.0, "width": 3.0, "height": 4.0,
            "text": "do the thing",
            "seq": 7
        }"#;

        let msg: ClientMessage = serde_json::from_str(json).unwrap();
        assert!(msg.is_canvas_mutation());
        assert_eq!(msg.canvas_seq(), Some(7));
        assert_eq!(msg.canvas_element_id(), Some("box-1"));
    }

    #[test]
    fn canvas_message_without_seq_still_deserializes() {
        // A client that predates acks must keep working — it simply is not acked.
        let json = r#"{
            "type": "canvas_text_changed",
            "workflow_id": "550e8400-e29b-41d4-a716-446655440000",
            "element_id": "box-1",
            "text": "hello"
        }"#;

        let msg: ClientMessage = serde_json::from_str(json).unwrap();
        assert!(msg.is_canvas_mutation());
        assert_eq!(msg.canvas_seq(), None);
        assert_eq!(msg.canvas_element_id(), Some("box-1"));
    }

    #[test]
    fn every_canvas_variant_round_trips_seq_and_element_id() {
        let wf = "550e8400-e29b-41d4-a716-446655440000";
        let cases = [
            format!(
                r#"{{"type":"canvas_element_moved","workflow_id":"{wf}","element_id":"e1","x":0.0,"y":0.0,"width":1.0,"height":1.0,"seq":1}}"#
            ),
            format!(
                r#"{{"type":"canvas_text_changed","workflow_id":"{wf}","element_id":"e2","text":"t","seq":2}}"#
            ),
            format!(
                r#"{{"type":"canvas_node_created","workflow_id":"{wf}","element_id":"e3","x":0.0,"y":0.0,"width":1.0,"height":1.0,"text":"t","seq":3}}"#
            ),
            format!(
                r#"{{"type":"canvas_edge_created","workflow_id":"{wf}","element_id":"e4","source_element_id":"a","target_element_id":"b","seq":4}}"#
            ),
            format!(
                r#"{{"type":"canvas_node_deleted","workflow_id":"{wf}","element_id":"e5","seq":5}}"#
            ),
            format!(
                r#"{{"type":"canvas_edge_deleted","workflow_id":"{wf}","element_id":"e6","seq":6}}"#
            ),
        ];

        for (i, json) in cases.iter().enumerate() {
            let msg: ClientMessage = serde_json::from_str(json).unwrap();
            let expected_seq = (i + 1) as u64;
            assert!(
                msg.is_canvas_mutation(),
                "case {i} is not a canvas mutation"
            );
            assert_eq!(msg.canvas_seq(), Some(expected_seq), "case {i} seq");
            assert_eq!(
                msg.canvas_element_id(),
                Some(format!("e{expected_seq}").as_str()),
                "case {i} element_id"
            );
        }
    }

    #[test]
    fn non_canvas_message_has_no_seq_or_element_id() {
        let msg = ClientMessage::Ping {
            ts: "2025-01-01T00:00:00Z".to_string(),
        };
        assert!(!msg.is_canvas_mutation());
        assert_eq!(msg.canvas_seq(), None);
        assert_eq!(msg.canvas_element_id(), None);
    }

    #[test]
    fn canvas_ack_serializes_to_the_shape_the_client_matches_on() {
        let ack = ControlMessage::CanvasAck {
            seq: 12,
            element_id: "box-9".to_string(),
            error: None,
        };
        let value: serde_json::Value = serde_json::to_value(&ack).unwrap();

        assert_eq!(value["type"], "canvas_ack");
        assert_eq!(value["seq"], 12);
        assert_eq!(value["element_id"], "box-9");
        assert!(value["error"].is_null());
    }

    #[test]
    fn canvas_ack_carries_the_rejection_reason() {
        let ack = ControlMessage::CanvasAck {
            seq: 3,
            element_id: "box-1".to_string(),
            error: Some("Canvas sync queue full".to_string()),
        };
        let value: serde_json::Value = serde_json::to_value(&ack).unwrap();

        assert_eq!(value["type"], "canvas_ack");
        assert_eq!(value["error"], "Canvas sync queue full");
    }
}
