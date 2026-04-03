//! WebSocket handler for real-time updates.
//!
//! Single unified event channel with topic-based and run-scoped filtering.
//! Events arrive as `Arc<BroadcastEnvelope>` with pre-serialized JSON,
//! eliminating per-connection serialization overhead.

use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::{
    extract::{
        connect_info::ConnectInfo,
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    response::Response,
};
use futures::{sink::SinkExt, stream::StreamExt};
use serde::Deserialize;
use tokio::sync::broadcast::error::RecvError;
use tokio::time::interval;
use tracing::{debug, warn};

use super::state::AppState;
use crate::types::UserId;

pub mod events;
pub use events::*;

/// Ping interval for keeping connection alive (30 seconds).
const PING_INTERVAL: Duration = Duration::from_secs(30);
/// Duration without a pong before a connection is considered dead.
const PONG_TIMEOUT: Duration = Duration::from_secs(crate::constants::WS_PONG_TIMEOUT_SECS);

/// Shared topic subscriptions for a client.
type TopicSubscriptions = Arc<Mutex<HashSet<Topic>>>;
/// Run-scoped subscriptions (specific run IDs the client wants events for).
type RunSubscriptions = Arc<Mutex<HashSet<uuid::Uuid>>>;

/// Query parameters for WebSocket connection.
#[derive(Debug, Deserialize)]
pub struct WsQuery {
    pub token: Option<String>,
}

/// WebSocket upgrade handler.
///
/// Upgrades an HTTP connection to a WebSocket connection for real-time updates.
/// Requires a valid JWT token in query params. Enforces global and per-IP
/// connection limits, and configures frame/message size limits.
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Query(query): Query<WsQuery>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Result<Response, axum::http::StatusCode> {
    let token = query.token.ok_or(axum::http::StatusCode::UNAUTHORIZED)?;

    let claims = super::auth::verify_token(&token, state.jwt_secret())
        .map_err(|_| axum::http::StatusCode::UNAUTHORIZED)?;

    let user_id = uuid::Uuid::parse_str(&claims.sub)
        .map(UserId)
        .map_err(|_| axum::http::StatusCode::UNAUTHORIZED)?;

    let ip = addr.ip();
    if !state.try_acquire_ws_connection(ip) {
        warn!("WebSocket connection limit reached for {}", ip);
        return Err(axum::http::StatusCode::SERVICE_UNAVAILABLE);
    }

    Ok(ws
        .max_frame_size(crate::constants::WS_MAX_FRAME_SIZE)
        .max_message_size(crate::constants::WS_MAX_MESSAGE_SIZE)
        .on_upgrade(move |socket| handle_socket(socket, state, Some(user_id), ip)))
}

/// Handle a WebSocket connection.
async fn handle_socket(
    socket: WebSocket,
    state: AppState,
    user_id: Option<UserId>,
    ip: std::net::IpAddr,
) {
    let (mut sender, mut receiver) = socket.split();
    let topics: TopicSubscriptions = Arc::new(Mutex::new(HashSet::new()));
    let run_subs: RunSubscriptions = Arc::new(Mutex::new(HashSet::new()));

    // Single event receiver from the unified EventBus
    let mut event_rx = state.events().subscribe();
    let mut ping_interval = interval(PING_INTERVAL);
    let mut last_pong = Instant::now();

    debug!("WebSocket client connected");

    loop {
        tokio::select! {
            // Periodic ping to keep connection alive
            _ = ping_interval.tick() => {
                // Check if client has responded to recent pings
                if last_pong.elapsed() > PONG_TIMEOUT {
                    warn!("Client pong timeout exceeded, disconnecting");
                    break;
                }
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
                                if client_msg.is_canvas_mutation() {
                                    let state_clone = state.clone();
                                    let uid = user_id;
                                    tokio::spawn(async move {
                                        if let Err(e) = crate::server::services::canvas_sync::handle_canvas_message(
                                            client_msg, &state_clone, uid,
                                        ).await {
                                            warn!(error = %e, "Canvas sync failed");
                                        }
                                    });
                                } else {
                                    let response = handle_client_message(
                                        client_msg, &topics, &run_subs,
                                    );
                                    if let Some(ctrl) = response {
                                        if let Ok(json) = serde_json::to_string(&ctrl) {
                                            if sender.send(Message::Text(json)).await.is_err() {
                                                warn!("Failed to send message to client");
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                warn!("Invalid message format: {}", e);
                                let ctrl = ControlMessage::Error {
                                    message: format!("Invalid message format: {}", e),
                                };
                                if let Ok(json) = serde_json::to_string(&ctrl) {
                                    if sender.send(Message::Text(json)).await.is_err() {
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) => {
                        debug!("Client initiated close");
                        break;
                    }
                    Some(Ok(Message::Ping(data))) => {
                        debug!("Received ping from client");
                        if sender.send(Message::Pong(data)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(Message::Pong(_))) => {
                        last_pong = Instant::now();
                        debug!("Received pong from client");
                    }
                    Some(Err(e)) => {
                        warn!("WebSocket error: {}", e);
                        break;
                    }
                    None => {
                        debug!("Connection closed by peer");
                        break;
                    }
                    _ => {}
                }
            }

            // Handle broadcast events (single unified arm)
            event = event_rx.recv() => {
                match event {
                    Ok(envelope) => {
                        // Topic filter
                        let subscribed = {
                            let subs = topics.lock().expect("topic lock poisoned");
                            subs.contains(&envelope.topic)
                        };
                        if !subscribed {
                            continue;
                        }

                        // User filter: events with a user_id only go to that user
                        if let Some(event_uid) = envelope.user_id {
                            if !user_id.map(|u| u.0 == event_uid).unwrap_or(false) {
                                continue;
                            }
                        }

                        // Run filter: if client has run subscriptions, only matching events pass
                        if let Some(rid) = envelope.run_id {
                            let runs = run_subs.lock().expect("run_subs lock poisoned");
                            if !runs.is_empty() && !runs.contains(&rid) {
                                continue;
                            }
                        }

                        // Send pre-serialized JSON — no per-connection serialization
                        if sender.send(Message::Text(envelope.json.clone())).await.is_err() {
                            break;
                        }
                    }
                    Err(RecvError::Lagged(n)) => {
                        warn!("Event receiver lagged, skipped {} messages", n);
                        let ctrl = ControlMessage::EventsMissed {
                            missed_count: n,
                            message: format!("Missed {} events. Re-fetch state via REST.", n),
                        };
                        if let Ok(json) = serde_json::to_string(&ctrl) {
                            if sender.send(Message::Text(json)).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(RecvError::Closed) => break,
                }
            }
        }
    }

    // Clean up on disconnect
    state.release_ws_connection(ip);
    let subs = topics.lock().expect("topic lock poisoned");
    debug!(
        "WebSocket connection closed, cleaning up {} subscription(s)",
        subs.len()
    );
}

/// Handle a client message and return optional control response.
fn handle_client_message(
    msg: ClientMessage,
    topics: &TopicSubscriptions,
    run_subs: &RunSubscriptions,
) -> Option<ControlMessage> {
    match msg {
        ClientMessage::Subscribe { topics: requested } => {
            let mut subs = topics.lock().expect("topic lock poisoned");
            for topic in &requested {
                subs.insert(*topic);
                debug!("Client subscribed to topic: {:?}", topic);
            }
            let current: Vec<Topic> = subs.iter().copied().collect();
            Some(ControlMessage::Subscribed { topics: current })
        }
        ClientMessage::Unsubscribe { topics: requested } => {
            let mut subs = topics.lock().expect("topic lock poisoned");
            for topic in &requested {
                subs.remove(topic);
                debug!("Client unsubscribed from topic: {:?}", topic);
            }
            let current: Vec<Topic> = subs.iter().copied().collect();
            Some(ControlMessage::Subscribed { topics: current })
        }
        ClientMessage::SubscribeRun { run_id } => {
            let mut runs = run_subs.lock().expect("run_subs lock poisoned");
            runs.insert(run_id);
            debug!("Client subscribed to run: {}", run_id);
            None
        }
        ClientMessage::UnsubscribeRun { run_id } => {
            let mut runs = run_subs.lock().expect("run_subs lock poisoned");
            runs.remove(&run_id);
            debug!("Client unsubscribed from run: {}", run_id);
            None
        }
        ClientMessage::Ping { ts } => Some(ControlMessage::Pong {
            client_ts: ts,
            server_ts: chrono::Utc::now(),
        }),
        // Canvas mutations are handled asynchronously — they never reach here.
        _ => None,
    }
}

#[cfg(test)]
mod tests;
