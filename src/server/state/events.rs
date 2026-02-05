//! Centralized event broadcasting for WebSocket and real-time updates.
//!
//! Groups all broadcast channels into a single struct, providing typed
//! subscribe and broadcast methods for each event type.

use tokio::sync::broadcast::{self, Receiver, Sender};

use crate::server::ws::{
    AgentUpdate, ContextUpdateEvent, FeedUpdate, PipelineUpdate, RoomUpdateEvent,
    RouterRequestEvent, RoutingUpdate, SessionUpdate, TaskUpdate,
};

/// Buffer sizes for broadcast channels.
pub struct ChannelSizes {
    pub high: usize,
    pub normal: usize,
    pub low: usize,
}

impl Default for ChannelSizes {
    fn default() -> Self {
        Self {
            high: crate::constants::CHANNEL_BROADCAST_HIGH,
            normal: crate::constants::CHANNEL_BROADCAST,
            low: crate::constants::CHANNEL_BROADCAST_LOW,
        }
    }
}

/// All broadcast channels grouped together.
///
/// Provides typed subscribe/broadcast methods for each event type,
/// eliminating the need to access individual channel senders directly.
pub struct EventBus {
    feed: Sender<FeedUpdate>,
    tasks: Sender<TaskUpdate>,
    agents: Sender<AgentUpdate>,
    sessions: Sender<SessionUpdate>,
    pipelines: Sender<PipelineUpdate>,
    routing: Sender<RoutingUpdate>,
    router_requests: Sender<RouterRequestEvent>,
    context_updates: Sender<ContextUpdateEvent>,
    room_updates: Sender<RoomUpdateEvent>,
}

impl EventBus {
    /// Create a new EventBus with default channel sizes.
    pub fn new() -> Self {
        Self::with_sizes(ChannelSizes::default())
    }

    /// Create a new EventBus with custom channel sizes.
    pub fn with_sizes(sizes: ChannelSizes) -> Self {
        Self {
            feed: broadcast::channel(sizes.high).0,
            tasks: broadcast::channel(sizes.normal).0,
            agents: broadcast::channel(sizes.low).0,
            sessions: broadcast::channel(sizes.low).0,
            pipelines: broadcast::channel(sizes.normal).0,
            routing: broadcast::channel(sizes.high).0,
            router_requests: broadcast::channel(sizes.normal).0,
            context_updates: broadcast::channel(sizes.low).0,
            room_updates: broadcast::channel(sizes.normal).0,
        }
    }

    // =========================================================================
    // Subscribe methods
    // =========================================================================

    /// Subscribe to feed updates.
    pub fn subscribe_feed(&self) -> Receiver<FeedUpdate> {
        self.feed.subscribe()
    }

    /// Subscribe to task updates.
    pub fn subscribe_tasks(&self) -> Receiver<TaskUpdate> {
        self.tasks.subscribe()
    }

    /// Subscribe to agent updates.
    pub fn subscribe_agents(&self) -> Receiver<AgentUpdate> {
        self.agents.subscribe()
    }

    /// Subscribe to session updates.
    pub fn subscribe_sessions(&self) -> Receiver<SessionUpdate> {
        self.sessions.subscribe()
    }

    /// Subscribe to pipeline execution updates.
    pub fn subscribe_pipelines(&self) -> Receiver<PipelineUpdate> {
        self.pipelines.subscribe()
    }

    /// Subscribe to routing updates.
    pub fn subscribe_routing(&self) -> Receiver<RoutingUpdate> {
        self.routing.subscribe()
    }

    /// Subscribe to router request lifecycle events.
    pub fn subscribe_router_requests(&self) -> Receiver<RouterRequestEvent> {
        self.router_requests.subscribe()
    }

    /// Subscribe to context store updates.
    pub fn subscribe_context_updates(&self) -> Receiver<ContextUpdateEvent> {
        self.context_updates.subscribe()
    }

    /// Subscribe to room events.
    pub fn subscribe_room_updates(&self) -> Receiver<RoomUpdateEvent> {
        self.room_updates.subscribe()
    }

    // =========================================================================
    // Broadcast methods (fire-and-forget, ignore send errors)
    // =========================================================================

    /// Broadcast a feed update to all subscribers.
    pub fn broadcast_feed(&self, update: FeedUpdate) {
        let _ = self.feed.send(update);
    }

    /// Broadcast a task update to all subscribers.
    pub fn broadcast_task(&self, update: TaskUpdate) {
        let _ = self.tasks.send(update);
    }

    /// Broadcast an agent update to all subscribers.
    pub fn broadcast_agent(&self, update: AgentUpdate) {
        let _ = self.agents.send(update);
    }

    /// Broadcast a session update to all subscribers.
    pub fn broadcast_session(&self, update: SessionUpdate) {
        let _ = self.sessions.send(update);
    }

    /// Broadcast a pipeline execution update to all subscribers.
    pub fn broadcast_pipeline(&self, update: PipelineUpdate) {
        let _ = self.pipelines.send(update);
    }

    /// Broadcast a routing update to all subscribers.
    pub fn broadcast_routing(&self, update: RoutingUpdate) {
        let _ = self.routing.send(update);
    }

    /// Broadcast a router request event to all subscribers.
    pub fn broadcast_router_request(&self, event: RouterRequestEvent) {
        let _ = self.router_requests.send(event);
    }

    /// Broadcast a context update event to all subscribers.
    pub fn broadcast_context_update(&self, event: ContextUpdateEvent) {
        let _ = self.context_updates.send(event);
    }

    /// Broadcast a room event to all subscribers.
    pub fn broadcast_room_update(&self, event: RoomUpdateEvent) {
        let _ = self.room_updates.send(event);
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}
