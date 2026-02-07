# Nexor Executors & LLM Provider Layer

Covers the three executor types (chat, collection DAG, room) and the full LLM provider abstraction with retry, rate limiting, and streaming.

---

## 1. Architecture Overview

All LLM calls flow through a single path:

```
Executor (Chat/Collection/Room)
  -> Creates Strategy (ChatStrategy/DagStepStrategy/RoomSpeakerStrategy)
  -> ExecutionEngine.execute(strategy, input, sink)
    -> LLM Provider Stack:
       RateLimitedProvider (concurrency + RPM + backoff)
         -> RetryingProvider (exponential backoff + jitter)
           -> AnthropicClient / OllamaClient / GrokClient
    -> Returns ExecutionResult
```

---

## 2. Chat Executor

**File:** `src/server/executors/chat/mod.rs`

Background worker that processes queued chat messages.

### spawn_chat_consumer()
```rust
pub fn spawn_chat_consumer(
    state: AppState,
    chat_rx: mpsc::Receiver<ConsumerMessage>,
) -> tokio::task::JoinHandle<()>
```
Called at server startup. Spawns tokio task listening on `chat_rx` channel.

### run_chat_consumer()
Main loop:
1. Initializes LLM provider with retry + rate-limiting wrappers
2. For each `ConsumerMessage` received:
   - Spawns `handle_message()` as separate tokio task

### handle_message()
1. Determines agent_id (from message, session draft_config, or default)
2. Calls `hub::run_chat()` or `hub::run_chat_with_config()`
3. On error: sends error chunk via `state.send_stream_chunk()`
4. Schedules `stream_cleanup()` after 120s

### ConsumerMessage
```rust
pub struct ConsumerMessage {
    pub id: Uuid,           // message_id for SSE stream
    pub user_id: UserId,
    pub session_id: Option<Uuid>,
    pub agent_id: Option<Uuid>,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}
```

---

## 3. Collection DAG Executor

**File:** `src/server/executors/collection_dag/mod.rs`

Executes workflow collections (DAGs of workflows).

### CollectionDagExecutor
```rust
pub struct CollectionDagExecutor<R: WorkflowCollectionRepo + Send + Sync> {
    collection_repo: Arc<R>,
    workflow_repo: Arc<dyn WorkflowRepo>,
    state: Arc<AppState>,
}
```

### execute_collection()
Main entry point:
1. Verify LLM provider configured
2. Load collection definition + workflows + edges
3. Create `collection_run` record (status: "running")
4. Execute based on mode:
   - `"sequential"` -> `execute_collection_sequential()`
   - `"parallel"` -> `execute_collection_parallel()`
5. Update `collection_run` status

### Sequential Execution
```
topological_sort_workflows() -> ordered list
for each workflow in order:
  collect outputs from previously completed workflows
  execute_workflow_in_collection()
  store result
```

### Parallel Execution
```
Build dependency graph (in-degree, adjacency)
Spawn entry workflows (in_degree == 0) immediately
On completion: decrement in-degree for children
Spawn ready children recursively
Wait for all (join_all)
```

### execute_workflow_in_collection()
Per-workflow execution:
1. Create `workflow_execution` record
2. Load workflow steps + edges
3. Create `ExecutionEngine` from provider
4. Build `ContainerExecutionConfig` if container_enabled
5. Build `WorkflowExecutionContext`:
   ```rust
   WorkflowExecutionContext {
       stage_execution_id: workflow_exec.id,
       run_id: collection_run_id,
       user_id,
       prior_outputs: HashMap<String, JsonValue>,
       container_config,
   }
   ```
6. Call `hub::dag::execute_workflow_via_engine()`
7. Aggregate step outputs -> JSON
8. Update `workflow_execution` with outputs/status
9. Handle pause state (`AwaitingUser` error) specially

### Cross-Workflow Data Flow
```rust
fn collect_workflow_outputs(
    completed_workflows: &HashMap<Uuid, WorkflowExecutionRow>,
    workflow_repo: &dyn WorkflowRepo,
) -> HashMap<String, JsonValue>
```
Keys workflows by name: `$workflow_{name}` (e.g., `$workflow_analysis`). Downstream workflows reference via `{$workflow_analysis.result}`.

### topological_sort_workflows()
Kahn's algorithm for cycle detection. Returns error if cycle found.

---

## 4. Room Executor

**File:** `src/server/executors/room/mod.rs`

Orchestrates a single user turn in a multi-agent room.

### Key Types
```rust
pub struct RoomMemberWithAgent {
    pub member: RoomMemberRow,
    pub agent: AgentRow,
}

pub struct SpeakerResult {
    pub agent_id: Uuid,
    pub agent_name: String,
    pub content: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub speaker_order: i32,
}

pub struct RoomTurnResult {
    pub turn_number: i32,
    pub speakers: Vec<SpeakerResult>,
    pub session_completed: bool,
}
```

### execute_room_turn()
```rust
pub async fn execute_room_turn(
    state: &AppState,
    provider: Arc<dyn LLMProvider>,
    room: &RoomRow,
    session: &RoomSessionRow,
    members: &[RoomMemberWithAgent],
    user_message: &str,
    user_id: Uuid,
    cancel: Option<&CancellationToken>,
) -> Result<RoomTurnResult, HubError>
```

**Flow:**
1. Parse `@mentions` from user message
2. Load room transcript
3. Determine speaker order:
   - If `gatekeeper_enabled`: `call_gatekeeper()` (LLM selects speakers)
   - Otherwise: `fallback_speaker_order()` (heuristic based on mentions)
4. **For each speaker sequentially:**
   a. Resolve agent's mode (system prompt, tools)
   b. Build room context preamble (participants, roles)
   c. Append agent context documents
   d. Build speaker prompt (transcript + user message)
   e. Override tools if `!room.tools_enabled`
   f. Create `agent_execution` record
   g. Build `RoomSpeakerStrategy`
   h. Execute via engine with `RoomStreamSink` (WS broadcasting)
   i. Record response as execution_message
   j. Broadcast `SpeakerEnd` event
5. Increment turn counter
6. Check turn limit -> mark completed if reached
7. Broadcast `TurnComplete` or `SessionComplete`

### build_room_context()
```rust
pub(crate) fn build_room_context(
    room: &RoomRow, member: &RoomMemberRow,
    agent: &AgentRow, members: &[RoomMemberWithAgent],
) -> String
```
Injects room name, member's role, and list of other participants. Instructs agent to build on previous points without repetition.

### call_gatekeeper()
```rust
async fn call_gatekeeper(
    provider: &Arc<dyn LLMProvider>,
    room: &RoomRow,
    members: &[RoomMemberWithAgent],
    user_message: &str,
    mentions: &[String],
    transcript_tail: &str,
) -> Result<Vec<SpeakerSelection>, HubError>
```
Builds roster prompt. Calls LLM (non-streaming). Parses response for speaker selections. Falls back to fallback order if parse fails.

### RoomStreamSink
```rust
struct RoomStreamSink {
    state: AppState,
    room_session_id: Uuid,
    run_id: Option<Uuid>,
    agent_id: Uuid,
    agent_name: String,
    speaker_order: i32,
    turn_number: i32,
    user_id: Uuid,
}
```
Broadcasts tokens via WebSocket `RoomEvent::SpeakerToken` to room subscribers.

---

## 5. LLM Provider Trait

**File:** `src/llm/provider/mod.rs`

```rust
#[async_trait]
pub trait LLMProvider: Send + Sync {
    async fn send_message(&self, request: LLMRequest) -> LLMResult<LLMResponse>;
    async fn send_message_stream(
        &self, request: LLMRequest,
    ) -> LLMResult<Pin<Box<dyn Stream<Item = LLMResult<StreamChunk>> + Send>>>;
    fn provider_name(&self) -> &'static str;
    fn model_id(&self) -> &str;
}
```

---

## 6. Core LLM Types

**File:** `src/llm/types/mod.rs`

### LLMError
```rust
pub enum LLMError {
    HttpError(reqwest::Error),
    ApiError { status: u16, message: String },
    RateLimited { retry_after_ms: u64 },
    AuthError(String),
    ParseError(String),
    StreamError(String),
    Timeout(u64),
    MaxRetriesExceeded(u32),
}
```

### Message
```rust
pub enum Role { User, Assistant }

pub enum MessageContent {
    Text(String),
    Blocks(Vec<ContentBlock>),
}

pub struct Message {
    pub role: Role,
    pub content: MessageContent,
}

impl Message {
    pub fn user(content) -> Self;
    pub fn assistant(content) -> Self;
    pub fn assistant_with_blocks(blocks) -> Self;
    pub fn tool_results(results) -> Self;
}
```

### LLMRequest
```rust
pub struct LLMRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub system: Option<String>,
    pub max_tokens: u32,
    pub temperature: f32,        // default 0.7
    pub stream: bool,
    pub tools: Vec<Tool>,
}
```

Builder methods: `new()`, `with_system()`, `with_max_tokens()`, `with_streaming()`, `with_tools()`.

### LLMResponse
```rust
pub struct LLMResponse {
    pub content: String,
    pub content_blocks: Vec<ContentBlock>,
    pub model: String,
    pub stop_reason: StopReason,
    pub usage: TokenUsage,
}

pub enum StopReason { EndTurn, MaxTokens, StopSequence, ToolUse }

pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
}
```

### ContentBlock
```rust
pub enum ContentBlock {
    Text { text: String },
    ToolUse { id: String, name: String, input: Value },
    ToolResult { tool_use_id: String, content: String },
}
```

### Tool
```rust
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,  // JSON Schema
}
```

### Streaming
```rust
pub enum StreamChunk {
    ContentDelta { text: String, index: usize },
    MessageStart { model: String, input_tokens: u32 },
    ContentBlockStart { index: usize },
    ContentBlockStop { index: usize },
    MessageDelta { stop_reason: Option<StopReason>, output_tokens: Option<u32> },
    ToolUseStart { index: usize, id: String, name: String },
    InputJsonDelta { index: usize, partial_json: String },
    MessageStop,
    Ping,
}

pub struct StreamAccumulator {
    pub content: String,
    pub content_blocks: Vec<ContentBlock>,
    pub model: Option<String>,
    pub stop_reason: Option<StopReason>,
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
}

impl StreamAccumulator {
    pub fn apply(&mut self, chunk: &StreamChunk);  // Accumulate
    pub fn build(self) -> Option<LLMResponse>;     // Finalize
}
```

---

## 7. Anthropic Client

**File:** `src/llm/anthropic/mod.rs`

### Config
```rust
pub struct AnthropicConfig {
    pub api_key: String,        // ANTHROPIC_API_KEY env
    pub base_url: String,       // https://api.anthropic.com
    pub model: String,          // ANTHROPIC_MODEL env
    pub timeout_secs: u64,      // 120s default
}
```

### Implementation
- `send_message()`: POST `/v1/messages` with JSON. Handles 401 (auth), 429 (rate limit), 5xx.
- `send_message_stream()`: POST `/v1/messages` with `stream: true`. Returns SSE stream parsed into `StreamChunk` via `parse_sse_line()`.

---

## 8. Ollama Client

**File:** `src/llm/ollama/mod.rs`

### Config
```rust
pub struct OllamaConfig {
    pub base_url: String,       // http://localhost:11434
    pub model: String,          // OLLAMA_MODEL env
    pub timeout_secs: u64,      // 300s (local models are slow)
}
```

Supports streaming (newline-delimited JSON) and non-streaming.

---

## 9. Retry Wrapper

**File:** `src/llm/retry/mod.rs`

### BackoffConfig
```rust
pub struct BackoffConfig {
    pub initial_delay: Duration,   // 100ms
    pub max_delay: Duration,       // 30s
    pub multiplier: f64,           // 2.0
    pub jitter: f64,               // 0.1 (10%)
    pub max_retries: u32,          // 5
}
```

### RetryPolicy
```rust
pub enum RetryPolicy {
    Default,  // Retries: RateLimited, 5xx, Timeout, HttpError
    Never,    // Never retries: AuthError, ParseError, StreamError
    Always,
}
```

### RetryingProvider
```rust
pub struct RetryingProvider<P: LLMProvider> {
    inner: Arc<P>,
    config: BackoffConfig,
    policy: RetryPolicy,
}
```

Implements `LLMProvider`. For streaming, retries connection but not individual chunks.

---

## 10. Rate Limit Wrapper

**File:** `src/llm/rate_limit/mod.rs`

### RateLimitConfig
```rust
pub struct RateLimitConfig {
    pub max_concurrent_calls: usize,       // 10
    pub requests_per_minute: usize,        // 60
    pub global_backoff_initial_ms: u64,    // 1000
    pub global_backoff_max_ms: u64,        // 60000
}
```

### Three Mechanisms
1. **Semaphore** - Limits concurrent API calls (prevents thundering herd)
2. **Token Bucket** - Enforces RPM limit (refills at rate per second)
3. **Global Backoff** - On 429, all callers wait before retrying

### RateLimitedProvider
```rust
pub struct RateLimitedProvider<P: LLMProvider> {
    inner: Arc<P>,
    semaphore: Arc<Semaphore>,
    token_bucket: Option<Arc<Mutex<TokenBucket>>>,
    global_backoff: Arc<RwLock<GlobalBackoff>>,
}
```

**Request flow:**
```
1. wait_for_backoff()      // Global 429 backoff
2. semaphore.acquire()     // Concurrency limit
3. acquire_rpm_token()     // RPM limit
4. inner.send_message()    // Actual call
5. on_success() or on_rate_limited(retry_after_ms)
```

---

## 11. Provider Registry

**File:** `src/llm/registry/mod.rs`

```rust
pub struct ProviderRegistry {
    providers: HashMap<String, Arc<dyn LLMProvider + Send + Sync>>,
    default_name: String,
}

impl ProviderRegistry {
    pub fn register(&mut self, name: &str, provider: Arc<dyn LLMProvider>);
    pub fn get(&self, name: &str) -> Option<&Arc<dyn LLMProvider>>;
    pub fn default_provider(&self) -> Option<&Arc<dyn LLMProvider>>;
}
```

Allows per-step provider selection (e.g., fast model for routing, powerful for analysis).

---

## 12. No-Op Provider

**File:** `src/llm/noop/mod.rs`

Fallback when no API key is configured. Returns `AuthError` on any call. Allows app to start without LLM; fails gracefully when features are used.

---

## 13. Provider Initialization

At startup (`AppState::new()`):
1. For each provider type (Anthropic, Ollama):
   - Load config from env
   - Create client instance
   - Wrap: `SafeStreamProvider` -> `RetryingProvider` -> `RateLimitedProvider`
2. Register in `ProviderRegistry` by name ("anthropic", "ollama")
3. Default provider set as primary

**Full wrapper stack:**
```
AnthropicClient (HTTP to Anthropic API)
  -> SafeStreamProvider (safe streaming handling)
    -> RetryingProvider (exponential backoff + jitter, max 5 retries)
      -> RateLimitedProvider (10 concurrent, 60 RPM, global 429 backoff)
```

---

## 14. End-to-End Execution Flows

### Chat: User Message -> Streamed Response
```
POST /api/chat { message }
  -> Queues ConsumerMessage to chat_tx
  -> Returns 202 { message_id }

[Chat Consumer Background Task]
  -> Receives ConsumerMessage
  -> handle_message() spawns task
  -> hub::run_chat(state, provider, user_id, message, session_id)
    -> Load agent, resolve mode
    -> Create ChatStrategy + ExecutionEngine + SseSink
    -> engine.execute(strategy, input, sink)
      -> For each round:
        -> provider.send_message_stream(request)
          -> RateLimitedProvider.wait + acquire + acquire
            -> RetryingProvider.with_retry
              -> AnthropicClient.POST /v1/messages (SSE)
        -> Stream tokens to sink.token() -> state.send_stream_chunk()
        -> If ToolUse: execute tools, loop
        -> If EndTurn: filters, on_complete, return
    -> strategy.on_complete() records to DB

GET /api/chat/{message_id}/stream
  -> Replays buffer + subscribes to live broadcast
  -> SSE stream to client
```

### Collection: Execute Workflow DAG
```
POST /api/collections/{id}/run
  -> CollectionDagExecutor.execute_collection()
    -> topological_sort or parallel spawn
    -> For each workflow:
      -> execute_workflow_in_collection()
        -> hub::dag::execute_workflow_via_engine()
          -> For each step in topo order:
            -> Resolve ports, variables, mode
            -> Create DagStepStrategy
            -> engine.execute(strategy) [non-streaming]
            -> Wrap result in StepExecutionEnvelope
            -> Broadcast WorkflowEvent
```

### Room: Multi-Agent Turn
```
POST /api/room-sessions/{id}/messages
  -> execute_room_turn()
    -> Gatekeeper selects speakers (or fallback)
    -> For each speaker:
      -> Resolve mode, build context
      -> Create RoomSpeakerStrategy
      -> engine.execute(strategy, msg, RoomStreamSink)
        -> Tokens broadcast via WS RoomEvent::SpeakerToken
      -> Record execution + response
    -> Increment turn, broadcast TurnComplete
```

---

## 15. Cancellation & Shutdown

### Per-Execution Cancellation
- Each execution gets a `CancellationToken` stored in `AppState.cancellation_tokens`
- Used in `tokio::select! { biased; _ = ct.cancelled() => ... }` during LLM calls
- API: `POST /agent-executions/{id}/cancel`

### Graceful Shutdown
1. Server receives SIGTERM
2. `shutdown_token` cancelled
3. Chat consumer exits loop
4. `drain_active_executions()` waits for running tasks (with timeout)
5. Container reaper cleans up orphaned containers
