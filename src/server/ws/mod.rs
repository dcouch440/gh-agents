//! WebSocket handler for real-time updates.
//!
//! Single unified event channel with topic-based and run-scoped filtering.

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
use serde::Deserialize;
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::Mutex;
use tokio::time::interval;
use tracing::{debug, info, warn};

use super::state::AppState;
use crate::types::UserId;

pub mod events;
pub use events::*;

/// Ping interval for keeping connection alive (30 seconds).
const PING_INTERVAL: Duration = Duration::from_secs(30);

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
/// Requires a valid JWT token in query params.
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Query(query): Query<WsQuery>,
) -> Result<Response, axum::http::StatusCode> {
    let token = query.token.ok_or(axum::http::StatusCode::UNAUTHORIZED)?;

    let claims = super::auth::verify_token(&token, &state.jwt_secret())
        .map_err(|_| axum::http::StatusCode::UNAUTHORIZED)?;

    let user_id = uuid::Uuid::parse_str(&claims.sub)
        .map(UserId)
        .map_err(|_| axum::http::StatusCode::UNAUTHORIZED)?;

    Ok(ws.on_upgrade(move |socket| handle_socket(socket, state, Some(user_id))))
}

/// Handle a WebSocket connection.
async fn handle_socket(socket: WebSocket, state: AppState, user_id: Option<UserId>) {
    let (mut sender, mut receiver) = socket.split();
    let topics: TopicSubscriptions = Arc::new(Mutex::new(HashSet::new()));
    let run_subs: RunSubscriptions = Arc::new(Mutex::new(HashSet::new()));

    // Single event receiver from the unified EventBus
    let mut event_rx = state.events().subscribe();
    let mut ping_interval = interval(PING_INTERVAL);

    info!("WebSocket client connected");

    loop {
        tokio::select! {
            // Periodic ping to keep connection alive
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
                                let response = handle_client_message(
                                    client_msg, &topics, &run_subs,
                                ).await;
                                if let Some(ctrl) = response {
                                    if let Ok(json) = serde_json::to_string(&ctrl) {
                                        if sender.send(Message::Text(json)).await.is_err() {
                                            warn!("Failed to send message to client");
                                            break;
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
                    }
                    Some(Err(e)) => {
                        warn!("WebSocket error: {}", e);
                        break;
                    }
                    None => {
                        info!("Connection closed by peer");
                        break;
                    }
                    _ => {}
                }
            }

            // Handle broadcast events (single unified arm)
            event = event_rx.recv() => {
                match event {
                    Ok(evt) => {
                        // Topic filter
                        let subscribed = {
                            let subs = topics.lock().await;
                            subs.contains(&evt.topic())
                        };
                        if !subscribed {
                            continue;
                        }

                        // User filter: events with a user_id only go to that user
                        if let Some(event_uid) = evt.user_id() {
                            if !user_id.map(|u| u.0 == event_uid).unwrap_or(false) {
                                continue;
                            }
                        }

                        // Run filter: if client has run subscriptions, only matching events pass
                        if let Some(rid) = evt.run_id() {
                            let runs = run_subs.lock().await;
                            if !runs.is_empty() && !runs.contains(&rid) {
                                continue;
                            }
                        }

                        // Serialize and send
                        let wire = evt.into_wire_message();
                        if let Ok(json) = serde_json::to_string(&wire) {
                            if sender.send(Message::Text(json)).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(RecvError::Lagged(n)) => {
                        warn!("Event receiver lagged, skipped {} messages", n);
                    }
                    Err(RecvError::Closed) => break,
                }
            }
        }
    }

    // Clean up on disconnect
    let subs = topics.lock().await;
    info!(
        "WebSocket connection closed, cleaning up {} subscription(s)",
        subs.len()
    );
}

/// todo: Create announce class to handle info!() commands.

/// Handle a client message and return optional control response.
async fn handle_client_message(
    msg: ClientMessage,
    topics: &TopicSubscriptions,
    run_subs: &RunSubscriptions,
) -> Option<ControlMessage> {
    match msg {
        ClientMessage::Subscribe { topics: requested } => {
            let mut subs = topics.lock().await;
            for topic in &requested {
                subs.insert(*topic);
                info!("Client subscribed to topic: {:?}", topic);
            }
            let current: Vec<Topic> = subs.iter().copied().collect();
            Some(ControlMessage::Subscribed { topics: current })
        }
        ClientMessage::Unsubscribe { topics: requested } => {
            let mut subs = topics.lock().await;
            for topic in &requested {
                subs.remove(topic);
                info!("Client unsubscribed from topic: {:?}", topic);
            }
            let current: Vec<Topic> = subs.iter().copied().collect();
            Some(ControlMessage::Subscribed { topics: current })
        }
        ClientMessage::SubscribeRun { run_id } => {
            let mut runs = run_subs.lock().await;
            runs.insert(run_id);
            info!("Client subscribed to run: {}", run_id);
            None
        }
        ClientMessage::UnsubscribeRun { run_id } => {
            let mut runs = run_subs.lock().await;
            runs.remove(&run_id);
            info!("Client unsubscribed from run: {}", run_id);
            None
        }
        ClientMessage::Ping { ts } => Some(ControlMessage::Pong {
            client_ts: ts,
            server_ts: chrono::Utc::now(),
        }),
    }
}

#[cfg(test)]
mod tests;
