//! Centralized event broadcasting for WebSocket and real-time updates.
//!
//! Single unified broadcast channel carrying [`Arc<BroadcastEnvelope>`] values.
//! All domain events (workflow, room, session) flow through one channel.
//! Events are pre-serialized at broadcast time so receivers get a cheap Arc clone
//! instead of cloning the full event data.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use tokio::sync::broadcast::{self, Receiver, Sender};
use uuid::Uuid;

use crate::server::ws::events::{ServerEvent, Topic};

/// Buffer size for the unified broadcast channel.
///
/// 16,384 provides headroom for workforce nodes (Designer + N agents) nested inside
/// a parent DAG, which generate bursts of designer-progress, agent-progress,
/// step-started/completed, and sub-workflow events. With `Arc<BroadcastEnvelope>`,
/// each slot holds an Arc pointer (~8 bytes), so memory overhead is ~128 KB.
const UNIFIED_CHANNEL_CAPACITY: usize = 16_384;

/// Pre-serialized event envelope for the broadcast channel.
///
/// Contains filtering metadata (cheap `Copy`/`Clone` fields) alongside
/// the pre-serialized JSON string. Wrapped in `Arc` on the broadcast channel
/// so each receiver clones a pointer, not the data.
#[derive(Debug)]
pub struct BroadcastEnvelope {
    /// Topic for subscription-based filtering.
    pub topic: Topic,
    /// User scope (`None` = broadcast to all subscribers of the topic).
    pub user_id: Option<Uuid>,
    /// Run scope for run-level filtering.
    pub run_id: Option<Uuid>,
    /// Pre-serialized JSON string, ready to send on the wire.
    pub json: String,
    /// Monotonic sequence number for ordering / gap detection.
    pub seq: u64,
}

/// Unified event bus. One channel, one sender, one type.
///
/// Events are serialized to JSON at broadcast time (once), then wrapped in
/// `Arc<BroadcastEnvelope>` so the broadcast channel distributes cheap Arc
/// clones to all receivers instead of cloning the full event data.
pub struct EventBus {
    tx: Sender<Arc<BroadcastEnvelope>>,
    seq: AtomicU64,
}

impl EventBus {
    /// Create a new EventBus with default capacity.
    pub fn new() -> Self {
        Self::with_capacity(UNIFIED_CHANNEL_CAPACITY)
    }

    /// Create a new EventBus with custom capacity (useful for tests).
    pub fn with_capacity(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self {
            tx,
            seq: AtomicU64::new(0),
        }
    }

    /// Broadcast an event to all subscribers. Fire-and-forget.
    ///
    /// Serializes the event to JSON once, wraps it in `Arc<BroadcastEnvelope>`,
    /// and sends to all receivers. Each receiver gets a cheap Arc clone.
    pub fn broadcast(&self, event: ServerEvent) {
        let topic = event.topic();
        let user_id = event.user_id();
        let run_id = event.run_id();

        let wire = event.into_wire_message();
        let json = match serde_json::to_string(&wire) {
            Ok(j) => j,
            Err(e) => {
                tracing::error!("Failed to serialize event: {}", e);
                return;
            }
        };

        let seq = self.seq.fetch_add(1, Ordering::Relaxed);

        let envelope = Arc::new(BroadcastEnvelope {
            topic,
            user_id,
            run_id,
            json,
            seq,
        });

        let _ = self.tx.send(envelope);
    }

    /// Subscribe to the event stream.
    pub fn subscribe(&self) -> Receiver<Arc<BroadcastEnvelope>> {
        self.tx.subscribe()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}
