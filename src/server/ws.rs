//! WebSocket handler for real-time updates

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    response::Response,
};
use futures::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::Mutex;
use tokio::time::interval;
use tracing::{debug, info, warn};
use uuid::Uuid;

use super::state::AppState;
use crate::types::UserId;

/// Ping interval for keeping connection alive (30 seconds)
const PING_INTERVAL: Duration = Duration::from_secs(30);

/// Serialize a server message to JSON text, returning None on failure.
fn serialize_msg(msg: &ServerMessage) -> Option<String> {
    match serde_json::to_string(msg) {
        Ok(json) => Some(json),
        Err(e) => {
            warn!("Failed to serialize ServerMessage: {}", e);
            None
        }
    }
}

/// Valid subscription channels
pub const CHANNEL_FEED: &str = "feed";
pub const CHANNEL_TASKS: &str = "tasks";
pub const CHANNEL_AGENTS: &str = "agents";
pub const CHANNEL_SESSIONS: &str = "sessions";
pub const CHANNEL_PIPELINES: &str = "pipelines";
pub const CHANNEL_ROUTING: &str = "routing";

/// Message sent from client to server
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    /// Subscribe to channels
    #[serde(rename = "subscribe")]
    Subscribe { channels: Vec<String> },
    /// Unsubscribe from channels
    #[serde(rename = "unsubscribe")]
    Unsubscribe { channels: Vec<String> },
    /// Subscribe to a specific pipeline run's events
    #[serde(rename = "subscribe_run")]
    SubscribeRun { run_id: Uuid },
    /// Unsubscribe from a specific pipeline run
    #[serde(rename = "unsubscribe_run")]
    UnsubscribeRun { run_id: Uuid },
}

/// Message sent from server to client
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    /// Acknowledge subscription changes
    #[serde(rename = "subscribed")]
    Subscribed { channels: Vec<String> },
    /// Feed item update
    #[serde(rename = "feed")]
    Feed { data: FeedUpdate },
    /// Task status update
    #[serde(rename = "task_update")]
    TaskUpdate { data: TaskUpdate },
    /// Agent status update
    #[serde(rename = "agent_update")]
    AgentUpdate { data: AgentUpdate },
    /// Session update
    #[serde(rename = "session_update")]
    SessionUpdate { data: SessionUpdate },
    /// Pipeline execution update
    #[serde(rename = "pipeline_update")]
    PipelineUpdate { data: PipelineUpdate },
    /// Tool routing update
    #[serde(rename = "routing_update")]
    RoutingUpdate { data: RoutingUpdate },
    /// Router request lifecycle update (pending → routed → completed)
    #[serde(rename = "router_request_update")]
    RouterRequestUpdate { data: RouterRequestEvent },
    /// New context arrived for a session
    #[serde(rename = "context_update")]
    ContextUpdate { data: ContextUpdateEvent },
    /// Room event (speaker start/end, tokens, turn complete)
    #[serde(rename = "room_update")]
    RoomUpdate { data: RoomUpdateEvent },
    /// Error message
    #[serde(rename = "error")]
    Error { message: String },
}

/// Feed item data broadcast to subscribers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedUpdate {
    pub id: Uuid,
    pub agent_id: String,
    pub content: String,
    pub item_type: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    #[serde(default)]
    pub user_id: Option<Uuid>,
}

/// Task update data broadcast to subscribers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskUpdate {
    pub id: Uuid,
    pub status: String,
    pub progress: Option<f32>,
    pub assigned_agent: Option<String>,
    #[serde(default)]
    pub user_id: Option<Uuid>,
}

/// Agent update data broadcast to subscribers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentUpdate {
    pub id: String,
    pub status: String,
    pub current_task: Option<Uuid>,
    #[serde(default)]
    pub user_id: Option<Uuid>,
}

/// Session update data broadcast to subscribers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionUpdate {
    pub id: Uuid,
    pub action: String, // "created", "updated", "deleted"
    pub title: Option<String>,
    pub mode_id: Option<String>,
    #[serde(default)]
    pub user_id: Option<Uuid>,
}

/// Pipeline execution update broadcast to subscribers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineUpdate {
    pub run_id: Uuid,
    pub pipeline_id: Uuid,
    pub event: String,
    pub stage_number: Option<i32>,
    pub stage_name: Option<String>,
    pub agent_id: Option<String>,
    pub output: Option<String>,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub duration_ms: Option<u64>,
    pub user_input: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    #[serde(default)]
    pub user_id: Option<Uuid>,
}

/// Tool routing event broadcast to subscribers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingUpdate {
    pub request_id: Uuid,
    pub tool_name: String,
    pub cluster_name: String,
    pub status: String,
    pub duration_ms: Option<u64>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    #[serde(default)]
    pub user_id: Option<Uuid>,
}

/// Router request lifecycle event broadcast to subscribers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterRequestEvent {
    pub request_id: Uuid,
    pub session_id: Uuid,
    pub run_id: Option<Uuid>,
    pub intent: String,
    pub status: String,
    pub routed_tool: Option<String>,
    pub passdown: Option<String>,
    pub result: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    #[serde(default)]
    pub user_id: Option<Uuid>,
}

/// Context store update event broadcast to subscribers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextUpdateEvent {
    pub session_id: Uuid,
    pub run_id: Option<Uuid>,
    pub source: String,
    pub priority: f32,
    pub content_preview: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    #[serde(default)]
    pub user_id: Option<Uuid>,
}

/// Room event (speaker lifecycle, tokens, turn management).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomUpdateEvent {
    pub room_session_id: Uuid,
    pub run_id: Option<Uuid>,
    /// Event type: "speaker_start", "speaker_token", "speaker_end", "turn_complete", "session_complete"
    pub event: String,
    pub agent_id: Option<Uuid>,
    pub agent_name: Option<String>,
    pub content: Option<String>,
    pub speaker_order: Option<i32>,
    pub turn_number: Option<i32>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    #[serde(default)]
    pub user_id: Option<Uuid>,
}

/// Shared subscriptions state for a client
type Subscriptions = Arc<Mutex<HashSet<String>>>;
/// Run-scoped subscriptions (pipeline run IDs the client wants events for)
type RunSubscriptions = Arc<Mutex<HashSet<Uuid>>>;

/// Query parameters for WebSocket connection
#[derive(Debug, Deserialize)]
pub struct WsQuery {
    pub token: Option<String>,
}

/// WebSocket upgrade handler
///
/// Upgrades an HTTP connection to a WebSocket connection for real-time updates.
/// Requires a valid JWT token in query params.
pub async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>, Query(query): Query<WsQuery>) -> Result<Response, axum::http::StatusCode> {
    let token = query.token.ok_or(axum::http::StatusCode::UNAUTHORIZED)?;

    let claims = super::auth::verify_token(&token, &state.jwt_secret).map_err(|_| axum::http::StatusCode::UNAUTHORIZED)?;

    let user_id = uuid::Uuid::parse_str(&claims.sub).map(UserId).map_err(|_| axum::http::StatusCode::UNAUTHORIZED)?;

    Ok(ws.on_upgrade(move |socket| handle_socket(socket, state, Some(user_id))))
}

/// Handle a WebSocket connection
async fn handle_socket(socket: WebSocket, state: AppState, user_id: Option<UserId>) {
    let (mut sender, mut receiver) = socket.split();
    let subscriptions: Subscriptions = Arc::new(Mutex::new(HashSet::new()));
    let run_subscriptions: RunSubscriptions = Arc::new(Mutex::new(HashSet::new()));

    // Subscribe to broadcast channels
    let mut feed_rx = state.subscribe_feed();
    let mut task_rx = state.subscribe_tasks();
    let mut agent_rx = state.subscribe_agents();
    let mut session_rx = state.subscribe_sessions();
    let mut pipeline_rx = state.subscribe_pipelines();
    let mut routing_rx = state.subscribe_routing();
    let mut router_request_rx = state.subscribe_router_requests();
    let mut context_update_rx = state.subscribe_context_updates();

    // Ping interval for keeping connection alive
    let mut ping_interval = interval(PING_INTERVAL);

    info!("WebSocket client connected");

    loop {
        tokio::select! {
            // Send periodic pings to keep connection alive
            _ = ping_interval.tick() => {
                debug!("Sending ping to client");
                if sender.send(Message::Ping(vec![])).await.is_err() {
                    warn!("Failed to send ping, connection may be dead");
                    break;
                }
            }

            // Handle incoming client messages
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        match serde_json::from_str::<ClientMessage>(&text) {
                            Ok(client_msg) => {
                                let response = handle_client_message(client_msg, &subscriptions, &run_subscriptions).await;
                                if let Some(server_msg) = response {
                                    if let Some(json) = serialize_msg(&server_msg) {
                                        if sender.send(Message::Text(json)).await.is_err() {
                                            warn!("Failed to send message to client");
                                            break;
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                warn!("Invalid message format: {}", e);
                                let error_msg = ServerMessage::Error {
                                    message: format!("Invalid message format: {}", e),
                                };
                                if let Some(json) = serialize_msg(&error_msg) {
                                    if sender.send(Message::Text(json)).await.is_err() {
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) => {
                        info!("Client initiated close");
                        break;
                    }
                    Some(Ok(Message::Ping(data))) => {
                        debug!("Received ping from client");
                        if sender.send(Message::Pong(data)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Pong(_))) => {
                        debug!("Received pong from client");
                        // Connection is healthy, continue
                    }
                    Some(Err(e)) => {
                        warn!("WebSocket error: {}", e);
                        break;
                    }
                    None => {
                        // Connection closed
                        info!("Connection closed by peer");
                        break;
                    }
                    _ => {}
                }
            }

            // Handle feed updates
            feed = feed_rx.recv() => {
                match feed {
                    Ok(update) => {
                        let subs = subscriptions.lock().await;
                        if subs.contains(CHANNEL_FEED) {
                            let should_send = update.user_id.is_none()
                                || user_id.map(|u| Some(u.0) == update.user_id).unwrap_or(false);
                            if should_send {
                                let msg = ServerMessage::Feed { data: update };
                                if let Some(json) = serialize_msg(&msg) {
                                    if sender.send(Message::Text(json)).await.is_err() {
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    Err(RecvError::Lagged(n)) => {
                        warn!("Feed receiver lagged, skipped {} messages", n);
                    }
                    Err(RecvError::Closed) => break,
                }
            }

            // Handle task updates
            task = task_rx.recv() => {
                match task {
                    Ok(update) => {
                        let subs = subscriptions.lock().await;
                        if subs.contains(CHANNEL_TASKS) {
                            let should_send = update.user_id.is_none()
                                || user_id.map(|u| Some(u.0) == update.user_id).unwrap_or(false);
                            if should_send {
                                let msg = ServerMessage::TaskUpdate { data: update };
                                if let Some(json) = serialize_msg(&msg) {
                                    if sender.send(Message::Text(json)).await.is_err() {
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    Err(RecvError::Lagged(n)) => {
                        warn!("Task receiver lagged, skipped {} messages", n);
                    }
                    Err(RecvError::Closed) => break,
                }
            }

            // Handle agent updates
            agent = agent_rx.recv() => {
                match agent {
                    Ok(update) => {
                        let subs = subscriptions.lock().await;
                        if subs.contains(CHANNEL_AGENTS) {
                            let should_send = update.user_id.is_none()
                                || user_id.map(|u| Some(u.0) == update.user_id).unwrap_or(false);
                            if should_send {
                                let msg = ServerMessage::AgentUpdate { data: update };
                                if let Some(json) = serialize_msg(&msg) {
                                    if sender.send(Message::Text(json)).await.is_err() {
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    Err(RecvError::Lagged(n)) => {
                        warn!("Agent receiver lagged, skipped {} messages", n);
                    }
                    Err(RecvError::Closed) => break,
                }
            }

            // Handle session updates
            session = session_rx.recv() => {
                match session {
                    Ok(update) => {
                        let subs = subscriptions.lock().await;
                        if subs.contains(CHANNEL_SESSIONS) {
                            let should_send = update.user_id.is_none()
                                || user_id.map(|u| Some(u.0) == update.user_id).unwrap_or(false);
                            if should_send {
                                let msg = ServerMessage::SessionUpdate { data: update };
                                if let Some(json) = serialize_msg(&msg) {
                                    if sender.send(Message::Text(json)).await.is_err() {
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    Err(RecvError::Lagged(n)) => {
                        warn!("Session receiver lagged, skipped {} messages", n);
                    }
                    Err(RecvError::Closed) => break,
                }
            }

            // Handle pipeline updates
            pipeline = pipeline_rx.recv() => {
                match pipeline {
                    Ok(update) => {
                        let subs = subscriptions.lock().await;
                        if subs.contains(CHANNEL_PIPELINES) {
                            let should_send = update.user_id.is_none()
                                || user_id.map(|u| Some(u.0) == update.user_id).unwrap_or(false);
                            if should_send {
                                let msg = ServerMessage::PipelineUpdate { data: update };
                                if let Some(json) = serialize_msg(&msg) {
                                    if sender.send(Message::Text(json)).await.is_err() {
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    Err(RecvError::Lagged(n)) => {
                        warn!("Pipeline receiver lagged, skipped {} messages", n);
                    }
                    Err(RecvError::Closed) => break,
                }
            }

            // Handle routing updates
            routing = routing_rx.recv() => {
                match routing {
                    Ok(update) => {
                        let subs = subscriptions.lock().await;
                        if subs.contains(CHANNEL_ROUTING) {
                            let should_send = update.user_id.is_none()
                                || user_id.map(|u| Some(u.0) == update.user_id).unwrap_or(false);
                            if should_send {
                                let msg = ServerMessage::RoutingUpdate { data: update };
                                if let Some(json) = serialize_msg(&msg) {
                                    if sender.send(Message::Text(json)).await.is_err() {
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    Err(RecvError::Lagged(n)) => {
                        warn!("Routing receiver lagged, skipped {} messages", n);
                    }
                    Err(RecvError::Closed) => break,
                }
            }

            // Handle router request lifecycle events (filtered by run subscription)
            rr = router_request_rx.recv() => {
                match rr {
                    Ok(event) => {
                        let subs = subscriptions.lock().await;
                        if subs.contains(CHANNEL_ROUTING) {
                            let should_send = event.user_id.is_none()
                                || user_id.map(|u| Some(u.0) == event.user_id).unwrap_or(false);
                            // If the event has a run_id, also check run subscriptions
                            let run_match = if let Some(rid) = event.run_id {
                                let runs = run_subscriptions.lock().await;
                                runs.contains(&rid)
                            } else {
                                true
                            };
                            if should_send && run_match {
                                let msg = ServerMessage::RouterRequestUpdate { data: event };
                                if let Some(json) = serialize_msg(&msg) {
                                    if sender.send(Message::Text(json)).await.is_err() {
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    Err(RecvError::Lagged(n)) => {
                        warn!("Router request receiver lagged, skipped {} messages", n);
                    }
                    Err(RecvError::Closed) => break,
                }
            }

            // Handle context store updates (filtered by run subscription)
            ctx = context_update_rx.recv() => {
                match ctx {
                    Ok(event) => {
                        let subs = subscriptions.lock().await;
                        if subs.contains(CHANNEL_ROUTING) {
                            let should_send = event.user_id.is_none()
                                || user_id.map(|u| Some(u.0) == event.user_id).unwrap_or(false);
                            let run_match = if let Some(rid) = event.run_id {
                                let runs = run_subscriptions.lock().await;
                                runs.contains(&rid)
                            } else {
                                true
                            };
                            if should_send && run_match {
                                let msg = ServerMessage::ContextUpdate { data: event };
                                if let Some(json) = serialize_msg(&msg) {
                                    if sender.send(Message::Text(json)).await.is_err() {
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    Err(RecvError::Lagged(n)) => {
                        warn!("Context update receiver lagged, skipped {} messages", n);
                    }
                    Err(RecvError::Closed) => break,
                }
            }
        }
    }

    // Clean up subscriptions on disconnect
    let subs = subscriptions.lock().await;
    info!("WebSocket connection closed, cleaning up {} subscription(s)", subs.len());
    // Resources are automatically cleaned up when the function exits:
    // - broadcast receivers are dropped
    // - subscriptions HashSet is dropped
}

/// Handle a client message and return optional response
async fn handle_client_message(msg: ClientMessage, subscriptions: &Subscriptions, run_subscriptions: &RunSubscriptions) -> Option<ServerMessage> {
    match msg {
        ClientMessage::Subscribe { channels } => {
            let mut subs = subscriptions.lock().await;
            let valid_channels: Vec<String> = channels.into_iter().filter(|c| is_valid_channel(c)).collect();

            for channel in &valid_channels {
                subs.insert(channel.clone());
                info!("Client subscribed to: {}", channel);
            }

            let current: Vec<String> = subs.iter().cloned().collect();
            Some(ServerMessage::Subscribed { channels: current })
        }
        ClientMessage::Unsubscribe { channels } => {
            let mut subs = subscriptions.lock().await;

            for channel in &channels {
                subs.remove(channel);
                info!("Client unsubscribed from: {}", channel);
            }

            let current: Vec<String> = subs.iter().cloned().collect();
            Some(ServerMessage::Subscribed { channels: current })
        }
        ClientMessage::SubscribeRun { run_id } => {
            let mut runs = run_subscriptions.lock().await;
            runs.insert(run_id);
            info!("Client subscribed to run: {}", run_id);
            None
        }
        ClientMessage::UnsubscribeRun { run_id } => {
            let mut runs = run_subscriptions.lock().await;
            runs.remove(&run_id);
            info!("Client unsubscribed from run: {}", run_id);
            None
        }
    }
}

/// Check if a channel name is valid
fn is_valid_channel(channel: &str) -> bool {
    matches!(channel, CHANNEL_FEED | CHANNEL_TASKS | CHANNEL_AGENTS | CHANNEL_SESSIONS | CHANNEL_PIPELINES | CHANNEL_ROUTING)
}

#[cfg(test)]
mod tests {
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

        let msg = ClientMessage::Unsubscribe { channels: vec!["feed".to_string()] };

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
        let msg = ServerMessage::Error { message: "bad request".to_string() };
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
            channels: vec!["feed".to_string(), "tasks".to_string(), "agents".to_string()],
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
        let msg1 = ClientMessage::Subscribe { channels: vec!["feed".to_string()] };
        let msg2 = ClientMessage::Subscribe { channels: vec!["feed".to_string()] };
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

        let msg = ClientMessage::Unsubscribe { channels: vec!["tasks".to_string()] };
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

        let msg = ClientMessage::Subscribe { channels: vec!["feed".to_string()] };
        let run_subs: RunSubscriptions = Arc::new(Mutex::new(HashSet::new()));
        handle_client_message(msg, &subscriptions, &run_subs).await;

        let msg = ClientMessage::Unsubscribe { channels: vec!["feed".to_string()] };
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
        let msg = ClientMessage::Subscribe { channels: vec!["feed".to_string()] };
        let debug = format!("{:?}", msg);
        assert!(debug.contains("Subscribe"));
        assert!(debug.contains("feed"));
    }

    #[test]
    fn server_message_debug_format() {
        let msg = ServerMessage::Error { message: "test".to_string() };
        let debug = format!("{:?}", msg);
        assert!(debug.contains("Error"));
        assert!(debug.contains("test"));
    }

    #[test]
    fn client_message_clone() {
        let msg = ClientMessage::Subscribe { channels: vec!["feed".to_string()] };
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
        let msg = ServerMessage::Error { message: "err".to_string() };
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
}
