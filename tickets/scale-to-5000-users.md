# Scale to 5000+ Concurrent Users

## Context

Audited the full backend for hardcoded limits, algorithmic hotspots, and concurrency patterns that would break or degrade at 5000+ simultaneous users. The broadcast buffer (now 16K) and port-resolution edge index were already fixed. Everything below remains.

---

## Critical: Hard Limits That Block 5000 Users

### WebSocket connection cap is 2,000
`src/constants.rs:111` — `WS_MAX_CONNECTIONS = 2000`. Users beyond this get 503. Bump to 10,000 or make env-configurable.

### Per-IP WebSocket limit breaks behind proxies
`src/constants.rs:113` — `WS_MAX_CONNECTIONS_PER_IP = 20`. Corporate proxies and load balancers share one IP. Raise to 1,000+ or switch to per-user tracking.

### Database pool defaults to 10 connections
`src/db/mod.rs:1053` — `max_connections` defaults to 10 (overridable via `DB_MAX_CONNECTIONS` env). At 5000 users doing concurrent reads/writes, connection starvation is guaranteed. Default should be 50-100.

### LLM concurrency semaphore is 5
`src/constants.rs:100` — `RATE_LIMIT_MAX_CONCURRENT_CALLS = 5`. Global semaphore across all users. 4,995 users queue. Raise to 50-100 (or per-provider).

### LLM rate limit is 25 RPM
`src/constants.rs:102` — `RATE_LIMIT_REQUESTS_PER_MINUTE = 25`. Global token bucket. At 5000 users this is 0.005 req/user/min. Raise to 1,000+ or defer to Anthropic's own 429 back-pressure.

### HTTP API rate limit is 2 req/sec per IP
`src/server/mod.rs:127-137` — Governor config: `per_second(2)`, `burst_size(50)`. Behind a reverse proxy all users share one IP. Raise to 20+ req/sec or switch to per-user-id limiting.

### Auth rate limit is 6 req/sec per IP
`src/server/mod.rs:113-121` — Governor config: `per_second(6)`, `burst_size(10)`. Mass login events behind a proxy are throttled to 6/sec. Raise to 50+ or per-user-id.

---

## High: Will Degrade Performance at Scale

### Orchestrator chat channel buffer is 100
`src/constants.rs:126` — `CHANNEL_ORCHESTRATOR = 100`. The mpsc channel between HTTP handlers and the background chat consumer. 5000 users x bursts = back-pressure on handlers. Raise to 1,000-5,000.

### Scheduler batch size is 5
`src/constants.rs:172` — `SCHEDULER_BATCH_SIZE = 5`, polled every 100ms = 50 tasks/sec max throughput. At 5000 concurrent users this creates massive backlogs. Raise to 50-100.

### Container creation semaphore is 10
`src/constants.rs:450` — `CONTAINER_MAX_CONCURRENT_CREATES = 10`. For-each loops creating containers serialize after 10 in-flight. Raise to 50.

### Broadcast channel capacity may still lag at 5000 users
`src/server/state/events.rs:22` — Now 16,384. With 5000 connections subscribed to the same channel, slow consumers will lag and miss events. Consider 65,536 or topic-scoped sub-channels.

### Response stream buffers are unbounded
`src/server/state/mod.rs` — `BufferedStream.buffer: Vec<StreamChunk>` grows without limit. A 10K-token response ~ 1MB. 5000 users x 2 active streams = 10GB. Add a per-stream cap (e.g. 10MB) and reduce the 120-second cleanup delay to 30s.

### Cancellation tokens may leak
`src/server/state/mod.rs` — `DashMap<Uuid, CancellationToken>`. Only 4 `remove_cancellation()` call sites found (all in chat executor). Workflow, sub-workflow, and dispatch execution paths may not clean up. Audit all paths and add TTL-based sweeping.

### No per-connection subscription limit
`src/server/ws/mod.rs` — `TopicSubscriptions` and `RunSubscriptions` are unbounded HashSets. A malicious client can subscribe to 100K run IDs. Add `MAX_RUN_SUBSCRIPTIONS = 1000`.

### HTTP clients are per-instance, not shared
`src/llm/anthropic/mod.rs:99-112`, `src/github/client.rs:54-59` — Each `ExecutionEngine` builds its own `reqwest::Client`. No connection reuse across engines. Create a singleton shared client in AppState with HTTP/2 and `pool_max_idle_per_host(64)`.

---

## Medium: Algorithmic Hot-Path Inefficiencies

### N+1 agent fetch in downstream routing context
`src/server/hub/dag/mod.rs:168-233` — `gather_downstream_routing_context()` calls `get_persisted_agent(rule.agent_id)` inside a nested loop (child_steps x rules). 10 children x 5 rules = 50 individual DB queries. Batch-fetch all agent IDs into a HashMap upfront.

### Linear search in for-each routing
`src/server/hub/dag/for_each/iteration.rs:80-87` — `.find()` over routing rules for every iteration item. O(items x rules). Build a `HashMap<label_value, agent_id>` before the loop.

### Linear search + string allocation in workforce designer
`src/server/hub/dag/workforce/mod.rs:657-666` — `.find()` with `id.to_string()` comparison for each prompt entry. Build a `HashMap<String, &TaskAgentRosterRow>` before the loop.

### Linear search inside sort comparator (collection DAG)
`src/server/executors/collection_dag/mod.rs:588-594` — `.find()` inside `sort_by_key` = O(n^2 log n). Build a `HashMap<workflow_id, display_order>` before sorting.

### `get_parent_steps` / `get_child_steps` scan all edges
`src/server/hub/dag/utils/graph.rs:74-90` — O(E) per call with no caching. These are called in the DAG loop per step. Extend the `incoming_edges` index pattern to also cover outgoing edges, and thread the indexes into these utility functions.

### TOCTOU race in WebSocket connection counting
`src/server/state/mod.rs:645-658` — Loads count, checks limit, then increments non-atomically. Under 5000 concurrent connects, the limit can be exceeded. Use `fetch_update` (atomic CAS loop) instead of load-then-add.

---

## Checklist

### Hard limits
- [ ] Bump `WS_MAX_CONNECTIONS` to 10,000 (or env-configurable)
- [ ] Bump `WS_MAX_CONNECTIONS_PER_IP` to 1,000 (or switch to per-user tracking)
- [ ] Bump default DB pool `max_connections` to 50
- [ ] Bump `RATE_LIMIT_MAX_CONCURRENT_CALLS` to 50-100
- [ ] Bump `RATE_LIMIT_REQUESTS_PER_MINUTE` to 1,000+
- [ ] Bump HTTP API Governor to 20+ req/sec (or per-user-id keying)
- [ ] Bump auth Governor to 50+ req/sec (or per-user-id keying)

### Buffers and channels
- [ ] Bump `CHANNEL_ORCHESTRATOR` to 1,000
- [ ] Bump `SCHEDULER_BATCH_SIZE` to 50-100
- [ ] Bump `CONTAINER_MAX_CONCURRENT_CREATES` to 50
- [ ] Bump `UNIFIED_CHANNEL_CAPACITY` to 65,536 (or add topic-scoped channels)
- [ ] Cap `BufferedStream.buffer` at 10MB and reduce cleanup delay to 30s
- [ ] Add `MAX_RUN_SUBSCRIPTIONS` per WebSocket connection (1,000)

### Shared resources
- [ ] Audit all cancellation token cleanup paths; add TTL sweep
- [ ] Share a singleton `reqwest::Client` via AppState (HTTP/2, pooled)

### Algorithmic fixes
- [ ] Batch-fetch agents in `gather_downstream_routing_context`
- [ ] HashMap index for for-each routing rules
- [ ] HashMap index for workforce designer roster lookup
- [ ] HashMap index for collection DAG sort comparator
- [ ] Extend edge adjacency index to outgoing edges; use in `get_parent_steps`/`get_child_steps`
- [ ] Fix TOCTOU race in `try_acquire_ws_connection` (atomic CAS)
