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
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Query(query): Query<WsQuery>,
) -> Result<Response, axum::http::StatusCode> {
    let token = query.token.ok_or(axum::http::StatusCode::UNAUTHORIZED)?;

    let claims = super::auth::verify_token(&token, &state.jwt_secret)
        .map_err(|_| axum::http::StatusCode::UNAUTHORIZED)?;

    let user_id = uuid::Uuid::parse_str(&claims.sub)
        .map(UserId)
        .map_err(|_| axum::http::StatusCode::UNAUTHORIZED)?;

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
    run_subscriptions: &RunSubscriptions,
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
    matches!(
        channel,
        CHANNEL_FEED
            | CHANNEL_TASKS
            | CHANNEL_AGENTS
            | CHANNEL_SESSIONS
            | CHANNEL_PIPELINES
            | CHANNEL_ROUTING
    )
}

#[cfg(test)]
mod tests;
