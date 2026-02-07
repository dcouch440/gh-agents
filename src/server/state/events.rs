//! Centralized event broadcasting for WebSocket and real-time updates.
//!
//! Single unified broadcast channel carrying [`ServerEvent`] values.
//! All domain events (workflow, room, session) flow through one channel.

use tokio::sync::broadcast::{self, Receiver, Sender};

use crate::server::ws::events::ServerEvent;

/// Buffer size for the unified broadcast channel.
///
/// 256 provides ~1-2 seconds of buffer at peak throughput (room token streaming).
const UNIFIED_CHANNEL_CAPACITY: usize = 256;

/// Unified event bus. One channel, one sender, one type.
pub struct EventBus {
    tx: Sender<ServerEvent>,
}

impl EventBus {
    /// Create a new EventBus with default capacity.
    pub fn new() -> Self {
        Self::with_capacity(UNIFIED_CHANNEL_CAPACITY)
    }

    /// Create a new EventBus with custom capacity (useful for tests).
    pub fn with_capacity(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self { tx }
    }

    /// Broadcast an event to all subscribers. Fire-and-forget.
    pub fn broadcast(&self, event: ServerEvent) {
        let _ = self.tx.send(event);
    }

    /// Subscribe to the event stream.
    pub fn subscribe(&self) -> Receiver<ServerEvent> {
        self.tx.subscribe()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}
