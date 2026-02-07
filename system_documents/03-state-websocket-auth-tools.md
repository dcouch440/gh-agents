# Nexor State, WebSocket, Auth & Tools

Covers the centralized application state, real-time event system, authentication, and tool execution subsystems.

---

## 1. Application State

**File:** `src/server/state/mod.rs`

### AppState
`AppState` wraps `Arc<AppStateInner>` for cheap cloning across async tasks. Shared by all HTTP handlers via Axum's `State` extractor.

### AppStateInner Fields
```rust
pub(crate) struct AppStateInner {
    db: Option<PgPool>,                                    // Database connection pool
    server_repo: Arc<dyn ServerRepo>,                      // Primary DB access trait
    repos: Repos,                                          // 14 grouped repository trait objects
    events: EventBus,                                      // Unified broadcast channel (256 capacity)
    config: Arc<RwLock<AppConfig>>,                        // Mutable app configuration
    provider: Option<Arc<dyn LLMProvider + Send + Sync>>,  // Default LLM (Anthropic)
    provider_registry: ProviderRegistry,                   // Multi-provider routing
    mode_resolver: Option<Arc<ModeResolver>>,              // Router-based mode selection
    prompt_registry: Arc<PromptRegistry>,                  // System/agent prompts
    jwt_secret: Vec<u8>,                                   // Token signing secret
    default_agent_id: Option<Uuid>,                        // "home" agent for workflows
    chat_tx: mpsc::Sender<ConsumerMessage>,                // Channel to chat orchestrator
    response_streams: DashMap<Uuid, BufferedStream>,       // SSE response buffers
    cancellation_tokens: DashMap<Uuid, CancellationToken>, // Execution cancellation
    shutdown_token: CancellationToken,                     // Master shutdown signal
    ollama_toggle_cache: Arc<RwLock<(bool, Instant)>>,     // 60-second cache
}
```

### Initialization Flow (AppState::new, lines 127-203)
1. Creates PgRepo instances wrapped in Arc trait objects
2. Groups all repos into `Repos` struct (14 repositories)
3. Creates `EventBus` with 256-item broadcast capacity
4. Loads JWT secret from `NEXOR_JWT_SECRET` env or generates random (dev only)
5. Loads prompt registry from `prompts/` directory
6. Initializes LLM providers (Anthropic required, Ollama optional):
   - Each wrapped with `SafeStreamProvider` -> `RetryingProvider` -> `RateLimitedProvider`
7. Registers providers in `ProviderRegistry` by name
8. Creates `ModeResolver` for router-based mode selection
9. Looks up default agent by name "home" from DB
10. Returns `(state, orchestrator_rx)` tuple

### Public Accessor Methods
```
state.db()                    -> Option<&PgPool>
state.server_repo() / repo()  -> &Arc<dyn ServerRepo>
state.repos()                 -> &Repos (14 repos)
state.events()                -> &EventBus
state.config()                -> &Arc<RwLock<AppConfig>>
state.provider()              -> Option<&Arc<dyn LLMProvider>>
state.provider_registry()     -> &ProviderRegistry
state.provider_for("ollama")  -> Option<Arc<dyn LLMProvider>>
state.mode_resolver()         -> Option<&Arc<ModeResolver>>
state.prompt_registry()       -> &Arc<PromptRegistry>
state.jwt_secret()            -> &[u8]
state.default_agent_id()      -> Option<Uuid>
state.chat_tx()               -> &mpsc::Sender<ConsumerMessage>
```

Backward-compat accessors: `state.user_repo()`, `state.doc_repo()`, `state.output_schema_repo()`, etc.

### Response Streams (SSE Buffering)

```rust
struct BufferedStream {
    tx: broadcast::Sender<StreamChunk>,
    buffer: Vec<StreamChunk>,
    done: bool,
}
```

Late-connecting SSE clients replay missed tokens from buffer.

**StreamChunk enum:**
- `Token(String)` - Text token
- `ToolStart { name, tool_id }` - Tool execution started
- `ToolEnd { name, tool_id }` - Tool execution finished
- `DocUpdate { doc_id, title }` - Document created/updated
- `Done` - Stream completed
- `Error(String)` - Stream error

**Methods:**
```
ensure_response_stream(message_id)     // Create stream if missing
get_response_stream(message_id)        // Returns (buffer, live_receiver, is_done)
send_stream_chunk(message_id, chunk)   // Appends to buffer + broadcasts
remove_response_stream(message_id)     // Cleanup
```

### Cancellation Token Management
```
register_cancellation(id)              // Creates new token
register_child_cancellation(id, parent)// Child linked to parent
cancel_execution(id) -> bool           // Cancel by ID
remove_cancellation(id)                // Cleanup
cancel_all_executions() -> usize       // Cancel all (shutdown)
active_execution_count() -> usize      // Count active
shutdown_token()                       // Master shutdown
```

### Repos Container
**File:** `src/server/state/repos.rs`

All 14 repositories:
1. `users: Arc<dyn UserRepo>`
2. `documents: Arc<dyn DocumentRepo>`
3. `output_schemas: Arc<dyn OutputSchemaRepo>`
4. `prompt_templates: Arc<dyn PromptTemplateRepo>`
5. `workflows: Arc<dyn WorkflowRepo>`
6. `agent_executions: Arc<dyn AgentExecutionRepo>`
7. `token_ledger: Arc<dyn TokenLedgerRepo>`
8. `results: Arc<dyn ResultRepo>`
9. `tool_routers: Arc<dyn ToolRouterRepo>`
10. `context_store: Arc<dyn ContextStoreRepo>`
11. `router_requests: Arc<dyn RouterRequestRepo>`
12. `rooms: Arc<dyn RoomRepo>`
13. `tool_capabilities: Arc<dyn ToolCapabilityRepo>`
14. `system_config: Arc<dyn SystemConfigRepo>`

### Builder Pattern
**File:** `src/server/state/builder.rs`

`AppStateBuilder` provides fluent API for constructing `AppState`. Required fields: `server_repo`, `repos`, `config`. Provides `build_for_test()` convenience method.

---

## 2. WebSocket System

**File:** `src/server/ws/mod.rs` and `src/server/ws/events.rs`

### Connection Flow
1. Client opens WebSocket with JWT: `ws://host/ws?token=<jwt>`
2. Server validates token via `auth::verify_token()`
3. Client subscribes to topics and/or specific runs
4. Server streams events matching subscriptions

### ws_handler() (lines 47-62)
Axum WebSocket upgrade handler. Requires `?token=` query param. Upgrades HTTP to WebSocket.

### handle_socket() Loop (lines 65-194)
Three concurrent streams via `tokio::select!`:

1. **Periodic Ping** (every 30s) - Keeps connection alive
2. **Incoming Client Messages** - Delegates to `handle_client_message()`
3. **Broadcast Events** - Receives from EventBus, applies 3-layer filtering:
   - **Topic filter** - Client must be subscribed to event's topic
   - **User filter** - If event has user_id, only that user receives it
   - **Run filter** - If client has run subscriptions, only matching events pass

### Topics
```rust
pub enum Topic {
    Workflow,    // Workflow execution lifecycle
    Room,        // Multi-agent room sessions
    Session,     // Chat session lifecycle
}
```

### Client Messages
```rust
enum ClientMessage {
    Subscribe { topics: Vec<Topic> },
    Unsubscribe { topics: Vec<Topic> },
    SubscribeRun { run_id: Uuid },
    UnsubscribeRun { run_id: Uuid },
    Ping { ts: String },
}
```

### Control Messages (Server -> Client, not broadcast)
```rust
enum ControlMessage {
    Subscribed { topics: Vec<Topic> },
    Error { message: String },
    Pong { client_ts: String, server_ts: DateTime<Utc> },
}
```

### Server Events (Broadcast)
```rust
enum ServerEvent {
    Workflow(WorkflowEvent),
    Room(RoomEvent),
    Session(SessionEvent),
}
```

### WorkflowEvent
```rust
pub struct WorkflowEvent {
    pub run_id: Uuid,
    pub workflow_id: Uuid,
    pub user_id: Option<Uuid>,
    pub kind: WorkflowEventKind,
}
```

**WorkflowEventKind variants:**
| Variant | Data | Description |
|---------|------|-------------|
| `Started` | `total_steps` | Workflow execution started |
| `StepStarted` | `step_id, step_name, agent_id, execution_id` | Step execution started |
| `StepCompleted` | `step_id, step_name, output, input_tokens, output_tokens, duration_ms` | Step completed |
| `StepFailed` | `step_id, step_name, error` | Step failed |
| `StepPaused` | `step_id, step_name` | Step waiting for approval |
| `ForEachProgress` | `step_id, step_name, completed, total` | For-each batch progress |
| `Completed` | `duration_ms` | Workflow completed |
| `Failed` | `error` | Workflow failed |
| `Resumed` | `step_id` | Workflow resumed after pause |

### RoomEvent
```rust
pub struct RoomEvent {
    pub room_session_id: Uuid,
    pub run_id: Option<Uuid>,
    pub user_id: Option<Uuid>,
    pub kind: RoomEventKind,
}
```

**RoomEventKind variants:**
| Variant | Data | Description |
|---------|------|-------------|
| `SpeakerStart` | `agent_id, agent_name, speaker_order, turn_number` | Speaker took turn |
| `SpeakerToken` | `agent_id, agent_name, ..., content` | Streaming token |
| `SpeakerEnd` | `agent_id, agent_name, ..., content` | Speaker finished |
| `TurnComplete` | `turn_number` | All speakers done |
| `SessionComplete` | `turn_number` | Room session ended |

### SessionEvent
```rust
pub struct SessionEvent {
    pub session_id: Uuid,
    pub user_id: Option<Uuid>,
    pub kind: SessionEventKind,
}
```

**SessionEventKind:** `Created { title, mode_id }`, `Updated { title, mode_id }`, `Deleted`

### Wire Format (JSON sent to clients)
```json
{
  "topic": "workflow",
  "event": "step_started",
  "ts": "2024-01-01T00:00:00Z",
  "run_id": "abc-123",
  "user_id": "def-456",
  "data": { /* variant-specific fields */ }
}
```

### Broadcasting from Code
```rust
state.broadcast(ServerEvent::Workflow(event));
state.broadcast_workflow(event);
state.broadcast_room(event);
state.broadcast_session(event);
```

---

## 3. Authentication

**File:** `src/server/auth/mod.rs`

### JWT Claims
```rust
pub struct Claims {
    pub sub: String,         // User ID (UUID string)
    pub email: String,
    pub is_admin: bool,      // Default false
    pub exp: usize,          // Expiration timestamp
    pub iat: usize,          // Issued-at timestamp
}
```

### Password Hashing
- `hash_password(password) -> Result<String>` - Argon2 with random salt (OsRng)
- `verify_password(password, hash) -> bool` - Constant-time comparison

### Token Management
- `create_token(secret, duration_hours, user_id, email, is_admin) -> Result<String>` - HS256 JWT
- `verify_token(token, secret) -> Result<Claims>` - Decodes + validates expiration

### AuthUser Extractor
```rust
pub struct AuthUser {
    pub user_id: UserId,
    pub claims: Claims,
}
```

Used as Axum handler argument. Extraction logic:
1. Tries `Authorization: Bearer <token>` header
2. Falls back to `?token=<token>` query param (needed for SSE/EventSource)
3. Validates token, parses user_id from claims.sub
4. Returns 401 if any step fails

### Integration
- JWT secret from `NEXOR_JWT_SECRET` env or random (dev)
- Middleware `require_auth()` checks token before protected routes
- Token duration: 24 hours

---

## 4. Tools System

**File:** `src/server/tools/mod.rs`

### Tool Categories

**Codebase Exploration (Read-Only):**
- `read_file` - Read file with optional focus parameter for large files
- `list_files` - List directory contents
- `search_files` - Grep-based search across codebase

**Document Management:**
- `create_doc` - Create new document (returns doc_id, ref_tag)
- `update_doc` - Update document content/title/tags
- `search_docs` - Full-text search documents

**Reasoning:**
- `think` - Scratchpad for step-by-step reasoning (no-op, echoes back)

**Structured Output Validation:**
- `submit_prd` - Validate and store PRD as document
- `submit_ticket` - Validate decomposition ticket

### Key Functions

**`filtered_tools(allowed: &[String]) -> Vec<Tool>`**
Returns all tools if `allowed` is empty. Filters to allowed list if provided.

**`agent_tools() -> Vec<Tool>`**
Returns all available tool definitions with name, description, input_schema (JSON Schema).

**`execute_tool(name, input, state, user_id, session_id) -> Value`**
Main dispatcher. Routes to appropriate handler by tool name.

### Tool Implementation Details

**read_file:** Validates path doesn't escape project root. If <= TRUNCATE_SMALL_FILE bytes, returns full content. If larger, calls `haiku_read_file()` for summarization. Returns `{ path, content/summary, line_count, size_bytes, summarized }`.

**list_files:** Validates path. Separates files/directories. Skips hidden files. Returns sorted lists.

**search_files:** Runs `grep -rn` with file type filters. Limits results (default 20). Returns `{ matches, line_numbers, truncated_content }`.

**think:** No-op. Returns `{ recorded: true, length }`. Allows model to reason step-by-step.

**create_doc:** Validates title/content. Generates kebab-case ref_tag. Creates document with type "architecture". Spawns background haiku summary. Returns `{ doc_id, ref_tag, title }`.

**update_doc:** Updates content/title/tags. Spawns background summary regeneration.

**search_docs:** Full-text search via repo. Returns `{ results: [{ id, title, ref_tag, summary, snippet }], count }`.

**submit_prd:** Validates required fields (title, problem_statement, technical_approach, goals, non_goals, user_stories, milestones, complexity). If valid, formats as markdown, creates document with type "prd". Returns `{ valid, doc_id, ref_tag }` or `{ valid: false, errors }`.

**submit_ticket:** Validates required fields (title, description, acceptance_criteria, files_to_modify, complexity, role). Returns `{ valid, ticket }` or errors.

### Haiku Helpers (Small/Fast Model)
- `haiku_read_file(prompt) -> Option<String>` - Summarizes large files
- `haiku_summarize(content) -> Option<String>` - 2-3 sentence summary for indexing
- `haiku_summarize_title(content) -> Option<String>` - Short title (3-6 words)
- `haiku_extract_context(summary, current_message) -> Option<String>` - Relevant context extraction

### Path Safety
```rust
match file_path.canonicalize() {
    Ok(canonical) if !canonical.starts_with(&cwd) => {
        return json!({ "error": "Path is outside project" });
    }
    // ... proceed safely
}
```

---

## 5. Router Service

**File:** `src/server/router_service/mod.rs`

Stateless LLM-based tool routing service.

**route_request() workflow:**
1. Loads router config and assigned tools
2. Builds routing prompt with tool specs
3. Calls router LLM (via provider from state)
4. Parses JSON decision
5. Logs to `router_requests` repo
6. Returns `RouteResult` (Sync/Async/NoAction)

---

## 6. Event Bus

**File:** `src/server/state/events.rs`

```rust
pub struct EventBus {
    tx: broadcast::Sender<ServerEvent>,
}
```

- Capacity: 256 items (~1-2 seconds at peak throughput)
- Fire-and-forget semantics
- `subscribe() -> broadcast::Receiver<ServerEvent>`
- `broadcast(event)` - Send to all subscribers

---

## 7. Key Constants

```
CHANNEL_ORCHESTRATOR             = 100   (chat message channel capacity)
UNIFIED_CHANNEL_CAPACITY         = 256   (EventBus capacity)
PING_INTERVAL                    = 30s   (WebSocket ping)
STREAM_CLEANUP_DELAY             = 120s  (SSE buffer lifetime)
OLLAMA_CACHE_TTL                 = 60s   (Toggle cache)
SHUTDOWN_DRAIN_TIMEOUT_SECS      = 30    (Graceful shutdown)
```

---

## 8. Data Flow Examples

### Chat Message -> Streamed Response
```
POST /api/chat { message }
  -> Creates message_id, ensures response stream
  -> Queues ConsumerMessage to chat_tx
  -> Returns 202 { message_id }

[Background: Chat Consumer]
  -> Receives ConsumerMessage
  -> hub::run_chat() with ChatStrategy
  -> ExecutionEngine streams tokens
  -> SseSink calls state.send_stream_chunk() per token
  -> Chunks buffered + broadcast

GET /api/chat/{message_id}/stream
  -> get_response_stream() returns (buffer, receiver)
  -> Replays buffer, then subscribes to live receiver
  -> Client sees seamless SSE stream
```

### Workflow Event Broadcasting
```
DAG executor runs step
  -> state.broadcast_workflow(WorkflowEvent { step_started })
  -> EventBus broadcasts to all WS connections
  -> Each WS client filters: topic(Workflow) + user_id + run_id
  -> Matching clients receive WireMessage JSON
```
