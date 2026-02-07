//! Tests for the unified WebSocket event system.

use super::events::*;
use super::*;

// ============================================================================
// Topic serde
// ============================================================================

#[test]
fn topic_serialize_snake_case() {
    assert_eq!(serde_json::to_string(&Topic::Workflow).unwrap(), r#""workflow""#);
    assert_eq!(serde_json::to_string(&Topic::Room).unwrap(), r#""room""#);
    assert_eq!(serde_json::to_string(&Topic::Session).unwrap(), r#""session""#);
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
    let json = r#"{"type": "unsubscribe_run", "run_id": "00000000-0000-0000-0000-000000000002"}"#;
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
        run_id,
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
        run_id: uuid::Uuid::new_v4(),
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
        run_id: uuid::Uuid::new_v4(),
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
        run_id: uuid::Uuid::nil(),
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
        run_id: uuid::Uuid::nil(),
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
        run_id: rid,
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
// handle_client_message
// ============================================================================

#[tokio::test]
async fn handle_subscribe_topics() {
    let topics: TopicSubscriptions = Arc::new(Mutex::new(HashSet::new()));
    let run_subs: RunSubscriptions = Arc::new(Mutex::new(HashSet::new()));
    let msg = ClientMessage::Subscribe {
        topics: vec![Topic::Workflow, Topic::Room],
    };
    let response = handle_client_message(msg, &topics, &run_subs).await;
    assert!(response.is_some());
    if let Some(ControlMessage::Subscribed { topics: current }) = response {
        assert_eq!(current.len(), 2);
    } else {
        panic!("Expected Subscribed");
    }
}

#[tokio::test]
async fn handle_unsubscribe_topics() {
    let topics: TopicSubscriptions = Arc::new(Mutex::new(HashSet::new()));
    topics.lock().await.insert(Topic::Workflow);
    topics.lock().await.insert(Topic::Session);

    let run_subs: RunSubscriptions = Arc::new(Mutex::new(HashSet::new()));
    let msg = ClientMessage::Unsubscribe {
        topics: vec![Topic::Workflow],
    };
    let response = handle_client_message(msg, &topics, &run_subs).await;
    if let Some(ControlMessage::Subscribed { topics: current }) = response {
        assert_eq!(current.len(), 1);
        assert!(current.contains(&Topic::Session));
    } else {
        panic!("Expected Subscribed");
    }
}

#[tokio::test]
async fn handle_subscribe_run() {
    let topics: TopicSubscriptions = Arc::new(Mutex::new(HashSet::new()));
    let run_subs: RunSubscriptions = Arc::new(Mutex::new(HashSet::new()));
    let run_id = uuid::Uuid::new_v4();
    let msg = ClientMessage::SubscribeRun { run_id };
    let response = handle_client_message(msg, &topics, &run_subs).await;
    assert!(response.is_none());
    assert!(run_subs.lock().await.contains(&run_id));
}

#[tokio::test]
async fn handle_unsubscribe_run() {
    let topics: TopicSubscriptions = Arc::new(Mutex::new(HashSet::new()));
    let run_subs: RunSubscriptions = Arc::new(Mutex::new(HashSet::new()));
    let run_id = uuid::Uuid::new_v4();
    run_subs.lock().await.insert(run_id);

    let msg = ClientMessage::UnsubscribeRun { run_id };
    let response = handle_client_message(msg, &topics, &run_subs).await;
    assert!(response.is_none());
    assert!(!run_subs.lock().await.contains(&run_id));
}

#[tokio::test]
async fn handle_ping() {
    let topics: TopicSubscriptions = Arc::new(Mutex::new(HashSet::new()));
    let run_subs: RunSubscriptions = Arc::new(Mutex::new(HashSet::new()));
    let msg = ClientMessage::Ping {
        ts: "2024-01-01T00:00:00.000Z".to_string(),
    };
    let response = handle_client_message(msg, &topics, &run_subs).await;
    if let Some(ControlMessage::Pong { client_ts, .. }) = response {
        assert_eq!(client_ts, "2024-01-01T00:00:00.000Z");
    } else {
        panic!("Expected Pong");
    }
}

#[tokio::test]
async fn subscribe_idempotent() {
    let topics: TopicSubscriptions = Arc::new(Mutex::new(HashSet::new()));
    let run_subs: RunSubscriptions = Arc::new(Mutex::new(HashSet::new()));

    let msg = ClientMessage::Subscribe {
        topics: vec![Topic::Workflow],
    };
    handle_client_message(msg, &topics, &run_subs).await;

    let msg = ClientMessage::Subscribe {
        topics: vec![Topic::Workflow],
    };
    let response = handle_client_message(msg, &topics, &run_subs).await;
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
