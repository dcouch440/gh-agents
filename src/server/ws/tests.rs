//! Tests for WebSocket handlers

use super::*;

#[test]
fn valid_channels() {
    assert!(is_valid_channel("feed"));
    assert!(is_valid_channel("tasks"));
    assert!(is_valid_channel("agents"));
    assert!(!is_valid_channel("invalid"));
    assert!(!is_valid_channel(""));
}

#[test]
fn client_message_subscribe_deserialize() {
    let json = r#"{"type": "subscribe", "channels": ["feed", "tasks"]}"#;
    let msg: ClientMessage = serde_json::from_str(json).unwrap();
    match msg {
        ClientMessage::Subscribe { channels } => {
            assert_eq!(channels, vec!["feed", "tasks"]);
        }
        _ => panic!("Expected Subscribe message"),
    }
}

#[test]
fn client_message_unsubscribe_deserialize() {
    let json = r#"{"type": "unsubscribe", "channels": ["feed"]}"#;
    let msg: ClientMessage = serde_json::from_str(json).unwrap();
    match msg {
        ClientMessage::Unsubscribe { channels } => {
            assert_eq!(channels, vec!["feed"]);
        }
        _ => panic!("Expected Unsubscribe message"),
    }
}

#[test]
fn server_message_subscribed_serialize() {
    let msg = ServerMessage::Subscribed {
        channels: vec!["feed".to_string(), "tasks".to_string()],
    };
    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains(r#""type":"subscribed""#));
    assert!(json.contains("feed"));
    assert!(json.contains("tasks"));
}

#[test]
fn server_message_feed_serialize() {
    let msg = ServerMessage::Feed {
        data: FeedUpdate {
            id: Uuid::nil(),
            agent_id: "agent-1".to_string(),
            content: "Test content".to_string(),
            item_type: "agent_report".to_string(),
            timestamp: chrono::Utc::now(),
            user_id: None,
        },
    };
    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains(r#""type":"feed""#));
    assert!(json.contains("agent-1"));
}

#[test]
fn server_message_task_update_serialize() {
    let msg = ServerMessage::TaskUpdate {
        data: TaskUpdate {
            id: Uuid::nil(),
            status: "running".to_string(),
            progress: Some(0.5),
            assigned_agent: Some("agent-1".to_string()),
            user_id: None,
        },
    };
    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains(r#""type":"task_update""#));
    assert!(json.contains("running"));
}

#[test]
fn server_message_agent_update_serialize() {
    let msg = ServerMessage::AgentUpdate {
        data: AgentUpdate {
            id: "agent-1".to_string(),
            status: "busy".to_string(),
            current_task: Some(Uuid::nil()),
            user_id: None,
        },
    };
    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains(r#""type":"agent_update""#));
    assert!(json.contains("busy"));
}

#[tokio::test]
async fn handle_subscribe_message() {
    let subscriptions: Subscriptions = Arc::new(Mutex::new(HashSet::new()));
    let msg = ClientMessage::Subscribe {
        channels: vec!["feed".to_string(), "invalid".to_string()],
    };

    let run_subs: RunSubscriptions = Arc::new(Mutex::new(HashSet::new()));
    let response = handle_client_message(msg, &subscriptions, &run_subs).await;

    assert!(response.is_some());
    if let Some(ServerMessage::Subscribed { channels }) = response {
        // Only valid channel should be subscribed
        assert_eq!(channels.len(), 1);
        assert!(channels.contains(&"feed".to_string()));
    } else {
        panic!("Expected Subscribed response");
    }
}

#[tokio::test]
async fn handle_unsubscribe_message() {
    let subscriptions: Subscriptions = Arc::new(Mutex::new(HashSet::new()));
    subscriptions.lock().await.insert("feed".to_string());
    subscriptions.lock().await.insert("tasks".to_string());

    let msg = ClientMessage::Unsubscribe {
        channels: vec!["feed".to_string()],
    };

    let run_subs: RunSubscriptions = Arc::new(Mutex::new(HashSet::new()));
    let response = handle_client_message(msg, &subscriptions, &run_subs).await;

    assert!(response.is_some());
    if let Some(ServerMessage::Subscribed { channels }) = response {
        assert_eq!(channels.len(), 1);
        assert!(channels.contains(&"tasks".to_string()));
    } else {
        panic!("Expected Subscribed response");
    }
}

#[test]
fn channel_constants_are_correct() {
    assert_eq!(CHANNEL_FEED, "feed");
    assert_eq!(CHANNEL_TASKS, "tasks");
    assert_eq!(CHANNEL_AGENTS, "agents");
}

#[test]
fn ping_interval_is_30_seconds() {
    assert_eq!(PING_INTERVAL, Duration::from_secs(30));
}

#[test]
fn server_message_error_serialize() {
    let msg = ServerMessage::Error {
        message: "something went wrong".to_string(),
    };
    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains(r#""type":"error""#));
    assert!(json.contains("something went wrong"));
}

#[test]
fn server_message_error_roundtrip_contains_message() {
    let msg = ServerMessage::Error {
        message: "bad request".to_string(),
    };
    let json = serde_json::to_string(&msg).unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["type"], "error");
    assert_eq!(value["message"], "bad request");
}

#[test]
fn client_message_invalid_type_fails() {
    let json = r#"{"type": "unknown", "channels": []}"#;
    let result = serde_json::from_str::<ClientMessage>(json);
    assert!(result.is_err());
}

#[test]
fn client_message_missing_type_fails() {
    let json = r#"{"channels": ["feed"]}"#;
    let result = serde_json::from_str::<ClientMessage>(json);
    assert!(result.is_err());
}

#[test]
fn client_message_missing_channels_fails() {
    let json = r#"{"type": "subscribe"}"#;
    let result = serde_json::from_str::<ClientMessage>(json);
    assert!(result.is_err());
}

#[test]
fn client_message_empty_channels() {
    let json = r#"{"type": "subscribe", "channels": []}"#;
    let msg: ClientMessage = serde_json::from_str(json).unwrap();
    match msg {
        ClientMessage::Subscribe { channels } => {
            assert!(channels.is_empty());
        }
        _ => panic!("Expected Subscribe"),
    }
}

#[test]
fn feed_update_deserialize() {
    let json = r#"{
            "id": "00000000-0000-0000-0000-000000000000",
            "agent_id": "a1",
            "content": "hello",
            "item_type": "log",
            "timestamp": "2024-01-01T00:00:00Z"
        }"#;
    let update: FeedUpdate = serde_json::from_str(json).unwrap();
    assert_eq!(update.agent_id, "a1");
    assert_eq!(update.content, "hello");
    assert_eq!(update.item_type, "log");
    assert_eq!(update.id, Uuid::nil());
}

#[test]
fn task_update_with_none_fields() {
    let update = TaskUpdate {
        id: Uuid::nil(),
        status: "pending".to_string(),
        progress: None,
        assigned_agent: None,
        user_id: None,
    };
    let json = serde_json::to_string(&update).unwrap();
    assert!(json.contains("pending"));
    assert!(json.contains("null"));

    let roundtrip: TaskUpdate = serde_json::from_str(&json).unwrap();
    assert!(roundtrip.progress.is_none());
    assert!(roundtrip.assigned_agent.is_none());
}

#[test]
fn task_update_with_some_fields() {
    let update = TaskUpdate {
        id: Uuid::nil(),
        status: "done".to_string(),
        progress: Some(1.0),
        assigned_agent: Some("bot".to_string()),
        user_id: None,
    };
    let json = serde_json::to_string(&update).unwrap();
    let roundtrip: TaskUpdate = serde_json::from_str(&json).unwrap();
    assert_eq!(roundtrip.progress, Some(1.0));
    assert_eq!(roundtrip.assigned_agent.as_deref(), Some("bot"));
}

#[test]
fn agent_update_with_none_task() {
    let update = AgentUpdate {
        id: "a1".to_string(),
        status: "idle".to_string(),
        current_task: None,
        user_id: None,
    };
    let json = serde_json::to_string(&update).unwrap();
    let roundtrip: AgentUpdate = serde_json::from_str(&json).unwrap();
    assert!(roundtrip.current_task.is_none());
    assert_eq!(roundtrip.status, "idle");
}

#[test]
fn agent_update_roundtrip() {
    let task_id = Uuid::new_v4();
    let update = AgentUpdate {
        id: "agent-x".to_string(),
        status: "working".to_string(),
        current_task: Some(task_id),
        user_id: None,
    };
    let json = serde_json::to_string(&update).unwrap();
    let roundtrip: AgentUpdate = serde_json::from_str(&json).unwrap();
    assert_eq!(roundtrip.current_task, Some(task_id));
    assert_eq!(roundtrip.id, "agent-x");
}

#[test]
fn feed_update_roundtrip() {
    let id = Uuid::new_v4();
    let now = chrono::Utc::now();
    let update = FeedUpdate {
        id,
        agent_id: "bot-1".to_string(),
        content: "did something".to_string(),
        item_type: "action".to_string(),
        timestamp: now,
        user_id: None,
    };
    let json = serde_json::to_string(&update).unwrap();
    let roundtrip: FeedUpdate = serde_json::from_str(&json).unwrap();
    assert_eq!(roundtrip.id, id);
    assert_eq!(roundtrip.agent_id, "bot-1");
    assert_eq!(roundtrip.content, "did something");
}

#[tokio::test]
async fn subscribe_empty_channels() {
    let subscriptions: Subscriptions = Arc::new(Mutex::new(HashSet::new()));
    let msg = ClientMessage::Subscribe { channels: vec![] };
    let run_subs: RunSubscriptions = Arc::new(Mutex::new(HashSet::new()));
    let response = handle_client_message(msg, &subscriptions, &run_subs).await;
    if let Some(ServerMessage::Subscribed { channels }) = response {
        assert!(channels.is_empty());
    } else {
        panic!("Expected Subscribed response");
    }
}

#[tokio::test]
async fn subscribe_all_valid_channels() {
    let subscriptions: Subscriptions = Arc::new(Mutex::new(HashSet::new()));
    let msg = ClientMessage::Subscribe {
        channels: vec![
            "feed".to_string(),
            "tasks".to_string(),
            "agents".to_string(),
        ],
    };
    let run_subs: RunSubscriptions = Arc::new(Mutex::new(HashSet::new()));
    let response = handle_client_message(msg, &subscriptions, &run_subs).await;
    if let Some(ServerMessage::Subscribed { channels }) = response {
        assert_eq!(channels.len(), 3);
    } else {
        panic!("Expected Subscribed response");
    }
}

#[tokio::test]
async fn subscribe_all_invalid_channels() {
    let subscriptions: Subscriptions = Arc::new(Mutex::new(HashSet::new()));
    let msg = ClientMessage::Subscribe {
        channels: vec!["foo".to_string(), "bar".to_string()],
    };
    let run_subs: RunSubscriptions = Arc::new(Mutex::new(HashSet::new()));
    let response = handle_client_message(msg, &subscriptions, &run_subs).await;
    if let Some(ServerMessage::Subscribed { channels }) = response {
        assert!(channels.is_empty());
    } else {
        panic!("Expected Subscribed response");
    }
}

#[tokio::test]
async fn subscribe_idempotent() {
    let subscriptions: Subscriptions = Arc::new(Mutex::new(HashSet::new()));
    let run_subs: RunSubscriptions = Arc::new(Mutex::new(HashSet::new()));
    let msg1 = ClientMessage::Subscribe {
        channels: vec!["feed".to_string()],
    };
    let msg2 = ClientMessage::Subscribe {
        channels: vec!["feed".to_string()],
    };
    handle_client_message(msg1, &subscriptions, &run_subs).await;
    let response = handle_client_message(msg2, &subscriptions, &run_subs).await;
    if let Some(ServerMessage::Subscribed { channels }) = response {
        assert_eq!(channels.len(), 1);
    } else {
        panic!("Expected Subscribed response");
    }
}

#[tokio::test]
async fn unsubscribe_nonexistent_channel() {
    let subscriptions: Subscriptions = Arc::new(Mutex::new(HashSet::new()));
    subscriptions.lock().await.insert("feed".to_string());

    let msg = ClientMessage::Unsubscribe {
        channels: vec!["tasks".to_string()],
    };
    let run_subs: RunSubscriptions = Arc::new(Mutex::new(HashSet::new()));
    let response = handle_client_message(msg, &subscriptions, &run_subs).await;
    if let Some(ServerMessage::Subscribed { channels }) = response {
        assert_eq!(channels.len(), 1);
        assert!(channels.contains(&"feed".to_string()));
    } else {
        panic!("Expected Subscribed response");
    }
}

#[tokio::test]
async fn unsubscribe_all() {
    let subscriptions: Subscriptions = Arc::new(Mutex::new(HashSet::new()));
    subscriptions.lock().await.insert("feed".to_string());
    subscriptions.lock().await.insert("tasks".to_string());

    let msg = ClientMessage::Unsubscribe {
        channels: vec!["feed".to_string(), "tasks".to_string()],
    };
    let run_subs: RunSubscriptions = Arc::new(Mutex::new(HashSet::new()));
    let response = handle_client_message(msg, &subscriptions, &run_subs).await;
    if let Some(ServerMessage::Subscribed { channels }) = response {
        assert!(channels.is_empty());
    } else {
        panic!("Expected Subscribed response");
    }
}

#[tokio::test]
async fn unsubscribe_empty_channels() {
    let subscriptions: Subscriptions = Arc::new(Mutex::new(HashSet::new()));
    subscriptions.lock().await.insert("feed".to_string());

    let msg = ClientMessage::Unsubscribe { channels: vec![] };
    let run_subs: RunSubscriptions = Arc::new(Mutex::new(HashSet::new()));
    let response = handle_client_message(msg, &subscriptions, &run_subs).await;
    if let Some(ServerMessage::Subscribed { channels }) = response {
        assert_eq!(channels.len(), 1);
    } else {
        panic!("Expected Subscribed response");
    }
}

#[tokio::test]
async fn subscribe_then_unsubscribe_then_subscribe() {
    let subscriptions: Subscriptions = Arc::new(Mutex::new(HashSet::new()));

    let msg = ClientMessage::Subscribe {
        channels: vec!["feed".to_string()],
    };
    let run_subs: RunSubscriptions = Arc::new(Mutex::new(HashSet::new()));
    handle_client_message(msg, &subscriptions, &run_subs).await;

    let msg = ClientMessage::Unsubscribe {
        channels: vec!["feed".to_string()],
    };
    let run_subs: RunSubscriptions = Arc::new(Mutex::new(HashSet::new()));
    handle_client_message(msg, &subscriptions, &run_subs).await;

    let msg = ClientMessage::Subscribe {
        channels: vec!["feed".to_string(), "agents".to_string()],
    };
    let run_subs: RunSubscriptions = Arc::new(Mutex::new(HashSet::new()));
    let response = handle_client_message(msg, &subscriptions, &run_subs).await;
    if let Some(ServerMessage::Subscribed { channels }) = response {
        assert_eq!(channels.len(), 2);
        assert!(channels.contains(&"feed".to_string()));
        assert!(channels.contains(&"agents".to_string()));
    } else {
        panic!("Expected Subscribed response");
    }
}

#[test]
fn is_valid_channel_boundary_cases() {
    assert!(!is_valid_channel("Feed"));
    assert!(!is_valid_channel("FEED"));
    assert!(!is_valid_channel("feed "));
    assert!(!is_valid_channel(" feed"));
    assert!(!is_valid_channel("task"));
    assert!(!is_valid_channel("agent"));
}

#[test]
fn server_message_feed_contains_all_fields() {
    let id = Uuid::new_v4();
    let ts = chrono::Utc::now();
    let msg = ServerMessage::Feed {
        data: FeedUpdate {
            id,
            agent_id: "a".to_string(),
            content: "c".to_string(),
            item_type: "t".to_string(),
            timestamp: ts,
            user_id: None,
        },
    };
    let json = serde_json::to_string(&msg).unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["type"], "feed");
    assert_eq!(value["data"]["agent_id"], "a");
    assert_eq!(value["data"]["content"], "c");
    assert_eq!(value["data"]["item_type"], "t");
    assert_eq!(value["data"]["id"], id.to_string());
}

#[test]
fn server_message_task_update_json_structure() {
    let msg = ServerMessage::TaskUpdate {
        data: TaskUpdate {
            id: Uuid::nil(),
            status: "queued".to_string(),
            progress: Some(0.0),
            assigned_agent: None,
            user_id: None,
        },
    };
    let json = serde_json::to_string(&msg).unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["type"], "task_update");
    assert_eq!(value["data"]["status"], "queued");
    assert_eq!(value["data"]["progress"], 0.0);
    assert!(value["data"]["assigned_agent"].is_null());
}

#[test]
fn server_message_agent_update_json_structure() {
    let task_id = Uuid::new_v4();
    let msg = ServerMessage::AgentUpdate {
        data: AgentUpdate {
            id: "ag-1".to_string(),
            status: "idle".to_string(),
            current_task: Some(task_id),
            user_id: None,
        },
    };
    let json = serde_json::to_string(&msg).unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["type"], "agent_update");
    assert_eq!(value["data"]["id"], "ag-1");
    assert_eq!(value["data"]["current_task"], task_id.to_string());
}

#[test]
fn client_message_debug_format() {
    let msg = ClientMessage::Subscribe {
        channels: vec!["feed".to_string()],
    };
    let debug = format!("{:?}", msg);
    assert!(debug.contains("Subscribe"));
    assert!(debug.contains("feed"));
}

#[test]
fn server_message_debug_format() {
    let msg = ServerMessage::Error {
        message: "test".to_string(),
    };
    let debug = format!("{:?}", msg);
    assert!(debug.contains("Error"));
    assert!(debug.contains("test"));
}

#[test]
fn client_message_clone() {
    let msg = ClientMessage::Subscribe {
        channels: vec!["feed".to_string()],
    };
    let cloned = msg.clone();
    match cloned {
        ClientMessage::Subscribe { channels } => {
            assert_eq!(channels, vec!["feed"]);
        }
        _ => panic!("Expected Subscribe"),
    }
}

#[test]
fn server_message_clone() {
    let msg = ServerMessage::Error {
        message: "err".to_string(),
    };
    let cloned = msg.clone();
    match cloned {
        ServerMessage::Error { message } => assert_eq!(message, "err"),
        _ => panic!("Expected Error"),
    }
}

#[test]
fn invalid_json_fails_to_parse() {
    let result = serde_json::from_str::<ClientMessage>("not json");
    assert!(result.is_err());
}

#[test]
fn client_message_wrong_channels_type_fails() {
    let json = r#"{"type": "subscribe", "channels": "feed"}"#;
    let result = serde_json::from_str::<ClientMessage>(json);
    assert!(result.is_err());
}
