//! WebSocket handler for real-time updates

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
};
use futures::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tokio::time::interval;
use tracing::{debug, info, warn};
use uuid::Uuid;

use super::state::AppState;

/// Ping interval for keeping connection alive (30 seconds)
const PING_INTERVAL: Duration = Duration::from_secs(30);

/// Valid subscription channels
pub const CHANNEL_FEED: &str = "feed";
pub const CHANNEL_TASKS: &str = "tasks";
pub const CHANNEL_AGENTS: &str = "agents";

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
}

/// Task update data broadcast to subscribers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskUpdate {
    pub id: Uuid,
    pub status: String,
    pub progress: Option<f32>,
    pub assigned_agent: Option<String>,
}

/// Agent update data broadcast to subscribers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentUpdate {
    pub id: String,
    pub status: String,
    pub current_task: Option<Uuid>,
}

/// Shared subscriptions state for a client
type Subscriptions = Arc<Mutex<HashSet<String>>>;

/// WebSocket upgrade handler
///
/// Upgrades an HTTP connection to a WebSocket connection for real-time updates.
pub async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

/// Handle a WebSocket connection
async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();
    let subscriptions: Subscriptions = Arc::new(Mutex::new(HashSet::new()));

    // Subscribe to broadcast channels
    let mut feed_rx = state.subscribe_feed();
    let mut task_rx = state.subscribe_tasks();
    let mut agent_rx = state.subscribe_agents();

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
                                let response = handle_client_message(client_msg, &subscriptions).await;
                                if let Some(server_msg) = response {
                                    let json = serde_json::to_string(&server_msg).unwrap();
                                    if sender.send(Message::Text(json.into())).await.is_err() {
                                        warn!("Failed to send message to client");
                                        break;
                                    }
                                }
                            }
                            Err(e) => {
                                warn!("Invalid message format: {}", e);
                                let error_msg = ServerMessage::Error {
                                    message: format!("Invalid message format: {}", e),
                                };
                                let json = serde_json::to_string(&error_msg).unwrap();
                                if sender.send(Message::Text(json.into())).await.is_err() {
                                    break;
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
                if let Ok(update) = feed {
                    let subs = subscriptions.lock().await;
                    if subs.contains(CHANNEL_FEED) {
                        let msg = ServerMessage::Feed { data: update };
                        let json = serde_json::to_string(&msg).unwrap();
                        if sender.send(Message::Text(json.into())).await.is_err() {
                            break;
                        }
                    }
                }
            }

            // Handle task updates
            task = task_rx.recv() => {
                if let Ok(update) = task {
                    let subs = subscriptions.lock().await;
                    if subs.contains(CHANNEL_TASKS) {
                        let msg = ServerMessage::TaskUpdate { data: update };
                        let json = serde_json::to_string(&msg).unwrap();
                        if sender.send(Message::Text(json.into())).await.is_err() {
                            break;
                        }
                    }
                }
            }

            // Handle agent updates
            agent = agent_rx.recv() => {
                if let Ok(update) = agent {
                    let subs = subscriptions.lock().await;
                    if subs.contains(CHANNEL_AGENTS) {
                        let msg = ServerMessage::AgentUpdate { data: update };
                        let json = serde_json::to_string(&msg).unwrap();
                        if sender.send(Message::Text(json.into())).await.is_err() {
                            break;
                        }
                    }
                }
            }
        }
    }

    // Clean up subscriptions on disconnect
    let subs = subscriptions.lock().await;
    info!(
        "WebSocket connection closed, cleaning up {} subscription(s)",
        subs.len()
    );
    // Resources are automatically cleaned up when the function exits:
    // - broadcast receivers are dropped
    // - subscriptions HashSet is dropped
}

/// Handle a client message and return optional response
async fn handle_client_message(
    msg: ClientMessage,
    subscriptions: &Subscriptions,
) -> Option<ServerMessage> {
    match msg {
        ClientMessage::Subscribe { channels } => {
            let mut subs = subscriptions.lock().await;
            let valid_channels: Vec<String> = channels
                .into_iter()
                .filter(|c| is_valid_channel(c))
                .collect();

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
    }
}

/// Check if a channel name is valid
fn is_valid_channel(channel: &str) -> bool {
    matches!(channel, CHANNEL_FEED | CHANNEL_TASKS | CHANNEL_AGENTS)
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

        let response = handle_client_message(msg, &subscriptions).await;

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

        let response = handle_client_message(msg, &subscriptions).await;

        assert!(response.is_some());
        if let Some(ServerMessage::Subscribed { channels }) = response {
            assert_eq!(channels.len(), 1);
            assert!(channels.contains(&"tasks".to_string()));
        } else {
            panic!("Expected Subscribed response");
        }
    }
}
