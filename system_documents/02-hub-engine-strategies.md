# Nexor Hub: Execution Engine, Strategies & Filters

The Hub is the unified execution engine for ALL LLM interactions. It implements a strategy pattern where different execution modes are pluggable strategies executed by a single `ExecutionEngine` loop.

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────┐
│              API & Orchestrator Handlers             │
└──────────────────────┬──────────────────────────────┘
                       │
        ┌──────────────┼──────────────┐
        v              v              v
   run_chat()   run_interactive()   DAG Executor
        │              │              │
        └──────────────┼──────────────┘
                       │
              ┌────────v─────────┐
              │ Mode Resolver    │  (optional router LLM call)
              └────────┬─────────┘
                       │
        ┌──────────────v──────────────┐
        │  Create Strategy            │
        │  (Chat, DagStep, Router,    │
        │   InteractiveChat, Room,    │
        │   CavernousStep)            │
        └──────────────┬──────────────┘
                       │
        ┌──────────────v──────────────┐
        │   ExecutionEngine           │
        │  (LLM loop + filters)       │
        │                             │
        │  for round in max_rounds:   │
        │    1. Filters(on_start)     │
        │    2. LLM call              │
        │    3. Tool execute          │
        │    4. Filters(on_response)  │
        │    5. Filters(on_output)    │
        │    6. Strategy(on_complete) │
        └──────────────┬──────────────┘
                       │
        ┌──────────────v──────────────┐
        │ StreamSink + Recorder       │
        │ + Token Ledger              │
        └──────────────┬──────────────┘
                       │
              ┌────────v─────────┐
              │ ExecutionResult   │
              └──────────────────┘
```

---

## 1. Hub Entry Points

**File:** `src/server/hub/mod.rs`

### run_chat() (lines 77-195)
Primary entry point for chat sessions.
1. Loads agent from DB
2. Resolves mode: if `agent.router_id` set -> `ModeResolver` (new system); else -> legacy `agent_modes`
3. Builds `ChatConfig` with system prompt, tools, temperature
4. Creates `ChatStrategy`, `ExecutionEngine`, `SseSink`
5. Returns `ExecutionResult`

### run_interactive_chat() (lines 201-264)
Handles review queue interactions.
1. Loads `agent_execution` row
2. Resolves mode via `ModeResolver`
3. Uses `InteractiveChatStrategy` to preserve "awaiting_user" status
4. Streams response, records to `execution_messages`

### run_chat_with_config() (lines 299-341)
For workshop sessions with inline `DraftConfig` (no agent lookup from DB).

### classify_mode() (lines 346-413)
Uses `RouterStrategy` to classify user message into agent mode (legacy system).

### apply_mode_overlay() (lines 416-430)
Merges mode overrides (system_prompt suffix, temperature, model, tools) onto base config.

### schedule_stream_cleanup() (lines 453-459)
Removes SSE response stream after 120s delay for late-connecting clients.

---

## 2. ExecutionStrategy Trait

**File:** `src/server/hub/strategy.rs`

```rust
#[async_trait]
pub trait ExecutionStrategy: Send + Sync {
    fn system_prompt(&self) -> &str;
    fn tools(&self) -> Vec<Tool>;
    fn model_id(&self) -> &str;
    fn max_rounds(&self) -> u32;
    fn context_budget(&self) -> usize;
    fn streaming(&self) -> bool;
    fn temperature(&self) -> f32;

    async fn build_messages(&self, input: &str) -> Result<Vec<Message>, HubError>;
    async fn execute_tool(&self, name: &str, input: &Value) -> Value;
    async fn on_complete(&self, response: &str, usage: &TokenUsage) -> Result<(), HubError>;
}
```

**Data flow:**
1. `build_messages()` -> construct initial message list
2. Engine calls LLM with messages, tools, temperature, model_id
3. If `tool_use` stop reason -> `execute_tool()` called
4. Tool results injected, loop continues
5. On end turn -> `on_complete()` for recording/token ledger

---

## 3. ExecutionEngine: The LLM Loop

**File:** `src/server/hub/engine/mod.rs`

### ExecutionResult
```rust
pub struct ExecutionResult {
    pub content: String,
    pub content_blocks: Vec<ContentBlock>,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_usd: f32,
    pub rounds_used: u32,
}
```

### ExecutionEngine
```rust
pub struct ExecutionEngine {
    provider: Arc<dyn LLMProvider>,
    filters: Vec<Arc<dyn ExecutionFilter>>,
    filter_ctx: Option<FilterContext>,
}
```

### execute() Main Loop (lines 95-337)

```
1. strategy.build_messages(input) -> get initial messages
2. Filters on_start -> augment system_prompt + messages
3. For round in 0..max_rounds:
   a. Check context budget (chars)
   b. Build LLMRequest (model, tools, temperature)
   c. STREAMING PATH:
      - provider.send_message_stream() -> futures::Stream
      - StreamAccumulator buffers chunks
      - sink.token(text), sink.tool_start/end forwarded
      - Respects cancellation token via tokio::select!
   d. NON-STREAMING PATH:
      - provider.send_message() -> single LLMResponse
      - Same cancellation handling
   e. STOP REASON HANDLING:
      - ToolUse -> extract tool_use blocks, execute, append results, LOOP
      - EndTurn -> filters on_response (can request retry), on_output, BREAK
4. strategy.on_complete(final_content, usage)
5. sink.done()
6. Return ExecutionResult
```

### Filter Retry (lines 273-297)
- Each filter's `on_response()` can return `ResponseAction::Retry` with feedback
- Max 1 retry per filter per execution (tracked via `filter_retried` set)
- If retry: append response + feedback message, continue loop

### Filter Output Transform (lines 299-307)
- Each filter's `on_output()` can transform final content (e.g., fix JSON)

### Cancellation (throughout)
```rust
tokio::select! {
    biased;  // Check cancellation first
    _ = ct.cancelled() => Err(HubError::Cancelled),
    next = stream.next() => next,
}
```

---

## 4. Execution Filters

**File:** `src/server/hub/engine/filters/mod.rs`

### Filter Trait
```rust
#[async_trait]
pub trait ExecutionFilter: Send + Sync {
    fn name(&self) -> &str;

    async fn on_start(
        &self, ctx: &FilterContext,
        system_prompt: String, messages: Vec<Message>,
    ) -> Result<(String, Vec<Message>), HubError>;

    async fn on_response(
        &self, ctx: &FilterContext, response: &LLMResponse,
    ) -> Result<ResponseAction, HubError>;

    async fn on_output(
        &self, ctx: &FilterContext, content: String,
    ) -> Result<String, HubError>;
}
```

### FilterContext
```rust
pub struct FilterContext {
    pub model_id: String,
    pub agent_id: Uuid,
    pub step_id: Option<Uuid>,
    pub round: u32,
    pub has_output_schema: bool,
    pub output_schema: Option<JsonValue>,
    pub metadata: HashMap<String, JsonValue>,
}
```

---

### Filter 1: SchemaEnhancementFilter
**File:** `src/server/hub/engine/filters/schema_enhancement/mod.rs`

**Hook:** `on_start`
**Condition:** `ctx.has_output_schema`
**Action:** Appends to system prompt:
```
Do NOT:
- Wrap in ```json ... ```
- Include text before/after JSON
- Add explanatory sentences
- Omit optional fields (use null)
```
**Effect:** Reduces common LLM output formatting mistakes.

---

### Filter 2: ReasoningTraceFilter
**File:** `src/server/hub/engine/filters/reasoning_trace/mod.rs`

**Hook:** `on_start` + `on_output`
**Condition:** `ctx.has_output_schema`

**on_start:** Appends instruction to system prompt:
```
Wrap response in: {"reasoning": "...", "result": <schema>}
```

**on_output:** Parses output JSON. If has `{"reasoning": "...", "result": ...}` structure, extracts and returns just `result`. Otherwise passes through.

**Effect:** LLM reasons step-by-step; downstream receives clean schema only.

---

### Filter 3: SchemaValidationRetryFilter
**File:** `src/server/hub/engine/filters/schema_validation_retry/mod.rs`

**Hook:** `on_response`
**Condition:** `ctx.has_output_schema`

**Action:** Tries to parse response as JSON. If invalid or primitive type, returns `Retry` with specific error context. If wrapped in markdown code fences, returns `Retry` with warning.

**Effect:** Forces retries until output is valid JSON object/array.

---

### Filter 4: PartialJsonRecoveryFilter
**File:** `src/server/hub/engine/filters/partial_json_recovery/mod.rs`

**Hook:** `on_output`
**Condition:** `ctx.has_output_schema`

**Action:** If already valid JSON, passes through. Otherwise `recover_truncated_json()`:
- Scans for unclosed `{`, `[` while respecting strings & escapes
- Builds stack of expected closers
- Appends missing `]`, `}` in reverse order
- Verifies result is valid JSON

**Effect:** Makes incomplete JSON outputs (from MaxTokens cutoff) parseable.

---

### Filter 5: FewShotFilter
**File:** `src/server/hub/engine/filters/few_shot/mod.rs`

**Hook:** `on_start`
**Dependencies:** `AgentExecutionRepo`

**Action:**
1. Loads up to 3 exemplary executions for `(agent_id, step_id)`
2. For each: extracts input/output pairs
3. Prepends as user/assistant message pairs before actual prompt
4. Augments system prompt: "The following conversation turns demonstrate successful examples"

**Effect:** LLM learns from concrete input/output examples.

---

### Filter 6: AgentGuidanceFilter
**File:** `src/server/hub/engine/filters/agent_guidance/mod.rs`

**Hook:** `on_start`
**Dependencies:** `ServerRepo` (queries `agent_guidances` table)

**Action:**
1. Loads guidance rows for `(agent_id, optional step_id)`
2. Extracts suggestions array
3. Appends to system prompt:
```
## Agent Guidance
You MUST follow these instructions derived from prior feedback:
- suggestion 1
- suggestion 2
```

**Effect:** Persistent feedback loop (CrewAI-style learning).

---

### Filter 7: DebateVerificationFilter
**File:** `src/server/hub/engine/filters/debate_verification/mod.rs`

**Hook:** `on_start` (captures prompt) + `on_response` (runs verification)
**Dependencies:** `LLMProvider`, `ServerRepo`, `AgentExecutionRepo`, `TokenLedgerRepo`

**on_response Flow:**
1. If no `verification_agent_ids` -> accept
2. Otherwise, launch **parallel tasks per verification agent** (JoinSet):
   a. Load verification agent from DB
   b. Build verification system prompt (agent expertise)
   c. Build user message (original task + primary response)
   d. Create `agent_execution` record (audit trail)
   e. LLM call with timeout (10s)
   f. Parse JSON critique: `{"approved": bool, "issues": [{"severity", "description", "suggestion"}]}`
   g. Record token usage to token_ledger
3. **Merge results:** If all approved -> accept. Else -> `Retry` with merged feedback.

**Effect:** Secondary agents review and critique primary agent's work, triggering retries.

---

## 5. All Strategies

### 5.1 ChatStrategy
**File:** `src/server/hub/strategies/chat/mod.rs`

**Purpose:** Interactive chat sessions (user-facing)

| Property | Value |
|----------|-------|
| max_rounds | 10 |
| context_budget | 480,000 chars |
| streaming | **true** |
| tools | Server tools (agent management, docs, etc.) |

**build_messages():** Loads session history. Optionally injects distilled prior context (haiku extracts relevant summary). Appends user's current message.

**execute_tool():** Via `tools::execute_tool()` -> server tools (read_file, search_files, create_doc, etc.)

**on_complete():**
- Records token usage to token_ledger
- Saves assistant response to chat_messages (or session_messages)
- **Auto-names session** via haiku_summarize_title from first exchange
- **Compacts history** if session exceeds threshold (~100 messages)

---

### 5.2 DagStepStrategy
**File:** `src/server/hub/strategies/dag_step/mod.rs`

**Purpose:** Single workflow step execution (non-interactive)

| Property | Value |
|----------|-------|
| max_rounds | 15 |
| streaming | **false** |
| tools | Execution tools (file ops, git, shell, docker) |

**build_messages():** Single user message with pre-composed user_prompt (template rendered with variables + port inputs).

**execute_tool():** Branches on execution mode:
- **Container mode:** `execution_tools::execute_tool_in_container()`
- **Local mode:** `execution_context` -> `execute_execution_tool()`
- **No context:** Returns error JSON

**on_complete():**
- Records token usage
- Parses structured output via `parse_structured_output()`:
  1. Direct JSON parse (trimmed)
  2. Extract from ` ```json ... ``` ` fence
  3. Extract from ` ``` ... ``` ` fence
  4. Find `{ ... }` and parse
- Updates agent_execution: status="completed", output, structured_output

---

### 5.3 RouterStrategy
**File:** `src/server/hub/strategies/router/mod.rs`

**Purpose:** Tool routing (single LLM call returning JSON decision)

| Property | Value |
|----------|-------|
| max_rounds | 1 |
| streaming | false |
| tools | **Empty** (router outputs JSON directly) |
| temperature | 0.0 (deterministic) |

**build_messages():** Single user message (full routing prompt).

**on_complete():** Optional token ledger write.

---

### 5.4 InteractiveChatStrategy
**File:** `src/server/hub/strategies/interactive_chat/mod.rs`

**Purpose:** Review queue conversations (user reviews agent execution)

Similar to ChatStrategy but:
- Loads conversation from `execution_messages` (not chat_messages)
- Streams response
- Records response as execution_message
- **Preserves "awaiting_user" status** (user must explicitly approve)

---

### 5.5 RoomSpeakerStrategy
**File:** `src/server/hub/strategies/room_speaker/mod.rs`

**Purpose:** Single agent speaking turn in a multi-agent room

| Property | Value |
|----------|-------|
| max_rounds | 5 |
| streaming | **true** |
| tools | Optional execution tools (if room.tools_enabled) |

---

### 5.6 CavernousStepStrategy
**File:** `src/server/hub/strategies/cavernous/mod.rs`

**Purpose:** Routing config selection (2-phase LLM interaction)

Uses `Arc<RwLock<CavernousState>>` for interior mutability.

**Phase 1 (SearchingConfigs):** LLM generates search query (3-8 words) from task description.

**Phase 2 (SelectingConfig):** Orchestrator performs document search using generated query. LLM selects best config from options via JSON response.

| Property | Value |
|----------|-------|
| max_rounds | 1 per phase |
| streaming | false |
| tools | Empty |
| temperature | 0.2 (low for deterministic selection) |

---

## 6. ModeResolver

**File:** `src/server/hub/mode_resolver/mod.rs`

Data-only service: `agent + input -> ResolvedModeConfig` via LLM router.

### ResolvedModeConfig
```rust
pub struct ResolvedModeConfig {
    pub system_prompt: String,
    pub tools: Vec<Tool>,
    pub tool_names: Vec<String>,
    pub temperature: f32,
    pub max_tokens: i32,
    pub selected_mode_id: Option<Uuid>,
    pub selected_mode_key: Option<String>,
    pub capabilities: Vec<String>,
}
```

### resolve() Flow (lines 87-234)
1. If `agent.router_id` is None -> return agent defaults
2. Load router + modes from DB
3. Build classification prompt (user_input + context_hint + mode list)
4. Call `RouterStrategy` LLM (deterministic, single turn)
5. Parse mode key from JSON response
6. Find mode (with fallback to first)
7. Load mode's explicit tools + capability-based tools
8. Union agent tools if `append_to_agent_tools`
9. Merge system prompts if `append_to_agent_system_prompt`
10. Return resolved config

### Tool Resolution
- **Explicit tools:** Direct mode -> tool mapping
- **Capability-based tools:** Load capabilities for mode -> load all tools tagged with those capabilities -> union with explicit tools (deduplicated)

---

## 7. DAG Executor

**File:** `src/server/hub/dag/mod.rs`

### Core Functions

**execute_workflow_via_engine():** Main entry point for DAG execution. Processes steps in topological order, respecting dependencies.

**resume_dag_from_approval():** Resumes a paused DAG after interactive step approval. Called from `approve_execution` and `close_room_session` API handlers.

### Step Processing
For each step in topological order:
1. Wait for all parent steps to complete
2. Resolve port inputs from upstream envelopes
3. Resolve variables (`{variable.path}`) in prompt template
4. Determine execution mode:
   - `single` -> execute once
   - `for_each` -> iterate array items (parallel with JoinSet)
   - `cavernous` -> 2-phase document routing
   - `room` -> multi-agent room session
   - Interactive -> pause and wait for user
5. Wrap result in `StepExecutionEnvelope`
6. Store in `step_outputs` map
7. Broadcast WebSocket events

### DAG Utils
**File:** `src/server/hub/dag/utils/mod.rs`

- **topological_sort():** Kahn's algorithm with cycle detection. Deterministic ordering via `display_order`.
- **find_entry_steps():** Steps with no incoming edges.
- **get_parent_steps() / get_child_steps():** Via edge lookup.
- **resolve_variables():** Replaces `{variable}` and `{variable.dot.path}` in templates.
- **resolve_path():** Navigates dot-paths (e.g., `features.content.0.name`).
- **resolve_for_each_array():** Extracts array for iteration.
- **extract_for_each_label():** Gets label from element's field.
- **resolve_port_inputs():** Extracts data from upstream envelopes via `json_path`.

### Chained For-Each Pipeline (Phase 6B)

**detect_for_each_chains():** Identifies consecutive for-each steps connected by single edges.
- Must have exactly one for-each child
- Child must have exactly one parent
- Only records chains of length >= 2
- Fan-out/fan-in break chains

**execute_for_each_chain():** Per-item pipeline execution.
- Each item flows through all chain stages sequentially
- Between items: parallel (tokio::task::JoinSet)

---

## 8. PromptRegistry

**File:** `src/server/hub/prompt_registry/mod.rs`

Loads markdown templates from `prompts/` directory at startup.

```rust
pub struct PromptRegistry {
    prompts: HashMap<String, String>,
    base_dir: PathBuf,
}
```

- **load_from_dir():** Recursively scans for `.md` files. Key = relative path minus `.md` (e.g., `system/distiller`).
- **render(key, vars):** Replaces `{variable}` placeholders with map values. Unknown variables left as-is.
- **render_inline(template, vars):** Renders arbitrary template strings.

---

## 9. ExecutionRecorder

**File:** `src/server/hub/recorder/mod.rs`

Centralizes all DB writes during execution.

```rust
pub struct ExecutionRecorder<'a> {
    repo: &'a dyn ServerRepo,
    agent_execution_repo: Option<&'a dyn AgentExecutionRepo>,
    token_ledger_repo: Option<&'a dyn TokenLedgerRepo>,
}
```

Methods:
- `record_chat_message()` -> chat_messages or session_messages
- `record_agent_execution()` -> agent_executions row
- `update_agent_execution()` -> status, output, structured_output
- `record_execution_message()` -> execution_messages (per tool call/text block)
- `record_tokens()` -> token_ledger with cost_usd

---

## 10. StreamSink

**File:** `src/server/hub/streaming/mod.rs`

```rust
#[async_trait]
pub trait StreamSink: Send + Sync {
    async fn token(&self, text: &str);
    async fn tool_start(&self, name: &str, id: &str);
    async fn tool_end(&self, name: &str, id: &str);
    async fn error(&self, msg: &str);
    async fn done(&self);
}
```

**Implementations:**
- `SseSink` -> Routes tokens to AppState's buffered response stream for SSE clients
- `NullSink` -> Discards all output (background/non-interactive)
- `RoomStreamSink` -> Broadcasts to room subscribers via WebSocket

---

## 11. Error Types

**File:** `src/server/hub/error/mod.rs`

```rust
pub enum HubError {
    ProviderNotConfigured,
    LlmCallFailed { round: u32, source: LLMError },
    ContextBudgetExceeded { chars: usize, round: u32 },
    ToolFailed { tool_name: String, reason: String },
    MaxRoundsExhausted { max: u32 },
    UnknownMode { mode_id: String },
    DagCycle,
    UnresolvedVariable { path: String },
    ForEachNotArray { reference: String },
    AgentNotFound { step_id: Uuid, agent_id: Uuid },
    AwaitingUser { step_id: Uuid, execution_id: Uuid },
    PortResolutionFailed { step_id: Uuid, reason: String },
    ProviderUnavailable { provider, step_id, agent_name },
    Cancelled,
    StreamInterrupted { execution_id: Uuid },
    PromptNotFound { key: String },
    PromptRenderFailed { key: String, var: String },
    Db(sqlx::Error),
    Internal(anyhow::Error),
}
```

---

## 12. Token Accounting & Cost

**File:** `src/server/hub/strategies/mod.rs`

```rust
pub fn compute_cost(model_id: &str, input_tokens: i64, output_tokens: i64) -> f32 {
    // Rates per 1M tokens:
    // Local models -> $0.00
    // Opus -> $15 in / $75 out
    // Sonnet -> $3 in / $15 out
    // Haiku -> $0.25 in / $1.25 out
    // GPT-4o -> $2.5 in / $10 out
    // GPT-4 -> $30 in / $60 out
    // Default -> $1 in / $3 out
}
```

---

## 13. WebSocket Broadcasting from DAG

```rust
fn broadcast_workflow_event(state: &AppState, ctx: &WorkflowExecutionContext, workflow_id: Uuid, kind: WorkflowEventKind) {
    state.broadcast_workflow(WorkflowEvent {
        run_id: ctx.run_id,
        workflow_id,
        user_id: Some(ctx.user_id),
        kind,
    });
}
```

WorkflowEventKind variants: `Started`, `StepStarted`, `StepCompleted`, `StepFailed`, `StepPaused`, `ForEachProgress`, `Completed`, `Failed`, `Resumed`.
