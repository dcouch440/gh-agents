# Production Test Coverage — Critical Gap Remediation

## Context

A comprehensive audit of the Nexor backend revealed significant test coverage gaps across production-critical systems. The container module is well-tested (100 tests, 100% coverage after DockerCli trait refactoring), but the rest of the system has serious blind spots:

- **DAG workflow execution**: 0% coverage on 14 core functions (608+ lines)
- **REST API authorization**: 13% endpoint coverage (19 of 145 endpoints)
- **Database layer**: 20% method coverage (41 of 204 methods), all `#[ignore]`
- **LLM providers**: RetryingProvider wrapper + Ollama client have zero integration tests
- **WebSocket lifecycle**: Zero tests for connection drops, reconnection, concurrency

**Key decisions:**
- Mock-based testing where possible (no Docker/Postgres required for unit tests)
- Integration tests use existing `test_utils` DB harness for database layer
- Each phase is independently shippable and testable
- Phases ordered by production risk: data integrity > auth > core execution > API surface

---

## Part 1: Database Transaction Safety & Core Queries

> **Risk:** CRITICAL — Concurrent writes can silently lose data in production.
> **Effort:** 3-4 days
> **Dependencies:** None (can start immediately)

Seven "delete-all-then-insert" transaction methods use READ COMMITTED isolation. Two concurrent `set_agent_tools()` calls for the same agent can interleave, leaving the agent with neither caller's intended tool set. Same pattern affects rooms, collections, and workflows.

### 1A. Fix Transaction Isolation

**Severity:** CRITICAL
**Files:** `src/db/pg_repo/mod.rs` (7 methods)

The following methods all share the same anti-pattern:
```rust
let mut tx = self.pool.begin().await?;
sqlx::query("DELETE FROM ... WHERE parent_id = $1").execute(&mut *tx).await?;
for item_id in items {
    sqlx::query("INSERT INTO ...").execute(&mut *tx).await?;
}
tx.commit().await?;
```

**Tasks:**
- [ ] Add `SET TRANSACTION ISOLATION LEVEL SERIALIZABLE` to: `set_agent_tools()`, `set_agent_context()`, `set_router_tools()`, `set_mode_tools()`, `set_room_members()`, `set_collection_edges()`, `set_step_agents()`
- [ ] Add retry-on-serialization-failure wrapper for these methods (Postgres returns `40001` on serialization conflict)
- [ ] Write concurrent-write test for `set_agent_tools()` — two tasks writing different tool sets simultaneously, verify one wins cleanly
- [ ] Write concurrent-write test for `set_room_members()` — same pattern

### 1B. Agent Execution Repository Tests

**Severity:** HIGH
**Files:** `src/db/pg_repo/tests.rs`, `src/db/traits/mod.rs`
**Effort:** 1 day

Agent executions are the core state record for every workflow step. Zero tests exist.

**Tasks:**
- [ ] `create_agent_execution` — verify all fields persisted correctly
- [ ] `update_agent_execution_status` — verify status transitions (running → completed, running → failed, running → cancelled)
- [ ] `update_agent_execution_routing` — verify routing_analysis JSON stored correctly
- [ ] `list_agent_executions` — verify user_id filtering (cross-tenant isolation)
- [ ] `list_completed_executions_for_step_ids` — verify array IN clause with empty array, single ID, many IDs
- [ ] `set_execution_exemplary` — verify boolean toggle

### 1C. Token Ledger & Cost Accuracy Tests

**Severity:** HIGH
**Files:** `src/db/pg_repo/tests.rs`
**Effort:** 0.5 day

Cost reporting aggregates with no isolation — concurrent inserts during SUM can undercount.

**Tasks:**
- [ ] `insert_ledger_entry` — verify deduplication (or document that duplicates are possible)
- [ ] `get_user_spend` — verify aggregation with multiple entries, verify user_id filtering
- [ ] `get_model_breakdown` — verify GROUP BY with multiple models
- [ ] Concurrent test: insert ledger entry while running `get_user_spend`, verify consistency

### 1D. Workflow & Room Repository Tests

**Severity:** HIGH
**Files:** `src/db/pg_repo/tests.rs`
**Effort:** 1 day

**Tasks:**
- [ ] `set_edges()` — verify transaction atomicity, test with empty edge list
- [ ] `add_step_document` / `remove_step_document` — verify ON CONFLICT behavior
- [ ] `create_room_session` — verify initial state
- [ ] `get_room_transcript` — verify JOIN ordering (messages must be chronological)
- [ ] `delete_workflow` — verify cascade behavior (steps, edges, ports, routing rules)
- [ ] `delete_document` — verify no orphaned `agent_context` records remain

---

## Part 2: Authentication & Authorization Hardening

> **Risk:** HIGH — Cross-tenant data access and token edge cases untested.
> **Effort:** 2-3 days
> **Dependencies:** None (can run parallel with Part 1)

### 2A. JWT Edge Cases

**Severity:** HIGH
**Files:** `src/server/auth/tests.rs`
**Effort:** 0.5 day

**Tasks:**
- [ ] **Expired token rejection** — create token with `exp` in the past, verify `verify_token()` returns Err
- [ ] **Malformed JWT** — test corrupted base64, missing `.` separators, empty string, whitespace-only
- [ ] **Missing claims** — JWT with no `sub`, no `email`, no `exp`
- [ ] **Future `iat`** — token with `iat` in the future (clock skew scenario)
- [ ] **Empty bearer prefix** — `Authorization: Bearer ` with no token after space
- [ ] **Admin flag in claims** — verify `is_admin` claim is read correctly, verify non-admin token doesn't grant admin access

### 2B. Cross-Tenant Authorization Tests

**Severity:** CRITICAL
**Files:** `src/server/api/agents/tests.rs` (expand), new test files for other modules
**Effort:** 1.5 days

No test verifies that user A cannot access user B's resources.

**Tasks:**
- [ ] Create test helper: `create_two_users_with_tokens(state) -> (token_a, token_b)`
- [ ] **Agents**: User A creates agent, user B cannot GET/PATCH/DELETE it (returns 404, not 403)
- [ ] **Agents**: `list_agents` returns only the authed user's agents
- [ ] **Sessions**: User A creates session, user B cannot access it
- [ ] **Workflows**: User A creates workflow, user B cannot list/modify it
- [ ] **Documents**: User A creates document, user B cannot read it
- [ ] **Agent Executions**: User A's execution not visible to user B

### 2C. Input Validation Tests

**Severity:** MEDIUM
**Files:** Various `src/server/api/*/tests.rs`
**Effort:** 1 day

**Tasks:**
- [ ] **Agent name** — empty string, whitespace-only, MAX_TITLE_LENGTH + 1 characters
- [ ] **System prompt** — MAX_PROMPT_LENGTH + 1 characters
- [ ] **Email validation** — spaces, no TLD, multiple @, unicode, SQL-like strings
- [ ] **Password** — less than 8 chars, empty string, unicode, very long (10k chars)
- [ ] **UUID path params** — `"not-a-uuid"` in agent/task/session paths (verify 400 not panic)
- [ ] **JSON body** — malformed JSON, missing required fields, extra unknown fields

---

## Part 3: DAG Workflow Execution Integration Tests

> **Risk:** CRITICAL — The core product (workflow execution) has zero integration tests.
> **Effort:** 4-5 days
> **Dependencies:** Parts 1-2 recommended but not required

This is the highest-complexity phase. The DAG executor orchestrates LLM calls, container lifecycle, VPN tunnels, approval gates, and state propagation across steps. All of this is untested.

### 3A. Test Infrastructure

**Severity:** CRITICAL (enabler for all other 3x tests)
**Files:** `src/server/hub/dag/tests.rs` (expand)
**Effort:** 1 day

**Tasks:**
- [ ] Create `MockLLMProvider` that returns configurable responses per call (already exists in engine tests — extract and share)
- [ ] Create `make_workflow(steps, edges)` helper that builds a full `WorkflowExecutionContext` with mock DB rows
- [ ] Create `mock_db_repos()` that returns `MockRepos` with in-memory agent/step/execution storage
- [ ] Create `make_single_step_workflow(agent, prompt)` convenience helper
- [ ] Verify helpers compile and basic single-step execution works end-to-end

### 3B. Linear & Branching Workflow Execution

**Severity:** CRITICAL
**Files:** `src/server/hub/dag/tests.rs`
**Effort:** 1.5 days

**Tasks:**
- [ ] **Single step** — one LLM step, verify envelope output, token counts, cost
- [ ] **Linear chain** (A → B → C) — verify execution order, variable propagation from A to B to C via `{variable.path}` interpolation
- [ ] **Fan-out** (A → B, A → C) — verify both B and C receive A's output
- [ ] **Fan-in** (A → C, B → C) — verify C waits for both A and B, receives both outputs via ports
- [ ] **Diamond** (A → B, A → C, B → D, C → D) — verify D receives both B and C outputs
- [ ] **Variable propagation** — step A sets `var_outputs["result"]`, step B's prompt contains `{result}`, verify interpolation

### 3C. For-Each & Chained Pipeline Execution

**Severity:** CRITICAL
**Files:** `src/server/hub/dag/tests.rs`
**Effort:** 1 day

**Tasks:**
- [ ] **Single for-each step** — 3-item array, verify 3 parallel LLM calls, aggregated envelope
- [ ] **For-each with routing** — `routing_mode="label"`, items with labels, verify label-based routing
- [ ] **Chained for-each** (Phase 6B) — two consecutive for-each steps, verify chain detection + per-item pipeline execution
- [ ] **For-each with empty array** — verify graceful handling (no LLM calls, empty aggregate)
- [ ] **For-each not-array error** — reference resolves to string instead of array, verify `HubError::ForEachNotArray`

### 3D. Error Handling & Cancellation

**Severity:** HIGH
**Files:** `src/server/hub/dag/tests.rs`
**Effort:** 1 day

**Tasks:**
- [ ] **Cancellation before first step** — cancel token fired before execution starts, verify clean exit
- [ ] **Cancellation mid-workflow** — 3-step workflow, cancel after step 1 completes, verify step 2 never executes
- [ ] **LLM provider error** — mock provider returns `LLMError::ApiError`, verify `HubError::Internal` propagation
- [ ] **Agent not found** — step references nonexistent agent_id, verify `HubError::AgentNotFound`
- [ ] **Cycle detection** — A → B → A edges, verify `HubError::DagCycle`
- [ ] **Port resolution failure** — step references port that doesn't exist in upstream output, verify error

### 3E. Approval Gates & Resume

**Severity:** HIGH
**Files:** `src/server/hub/dag/tests.rs`
**Effort:** 0.5 day

**Tasks:**
- [ ] **Interactive step pauses** — step with `approval_required=true`, verify `HubError::DagPaused` returned
- [ ] **Resume from approval** — create paused workflow state, call `resume_dag_from_approval()` with approved output, verify execution continues from paused step
- [ ] **Resume with variable propagation** — verify approved output feeds into downstream step's variable resolution

---

## Part 4: LLM Provider Integration Tests

> **Risk:** HIGH — The production retry/rate-limit wrappers and Ollama client are untested.
> **Effort:** 2-3 days
> **Dependencies:** None (can run parallel with Parts 1-3)

### 4A. RetryingProvider Wrapper

**Severity:** HIGH
**Files:** `src/llm/retry/tests.rs` (expand)
**Effort:** 1 day

The `with_retry()` function is well-tested, but `RetryingProvider` (the actual wrapper used in production) has zero tests.

**Tasks:**
- [ ] **Retry on transient error** — mock provider fails once with 500, succeeds second call
- [ ] **No retry on auth error** — mock provider returns 401, verify single attempt
- [ ] **Retry on rate limit** — mock provider returns 429 with `retry_after_ms`, verify backoff
- [ ] **Max retries exhausted** — mock provider fails 4 times, verify error propagated
- [ ] **Stream retry** — `send_message_stream()` with transient failure on first attempt, verify retry succeeds
- [ ] **Timeout handling** — mock provider hangs, verify timeout triggers retry

### 4B. Ollama Client

**Severity:** HIGH
**Files:** `src/llm/ollama/tests.rs` (expand)
**Effort:** 1 day

Ollama client has zero integration tests — all critical paths untested.

**Tasks:**
- [ ] **`parse_response()`** — valid JSON response → `LLMResponse` with text content, token counts
- [ ] **`parse_response()` with tool calls** — response containing tool_calls array → `ContentBlock::ToolUse`
- [ ] **`parse_stream_chunk()`** — NDJSON line → `StreamEvent::ContentDelta`
- [ ] **`parse_stream_chunk()` done marker** — `{"done": true}` → `StreamEvent::MessageStop`
- [ ] **`health_check()`** — mock HTTP 200 → true, mock timeout → false
- [ ] **`validate_model()`** — model in tags list → Ok, model not found → Err
- [ ] **Buffer overflow protection** — streaming response > 10MB limit → error (not OOM)

### 4C. Anthropic Streaming Integration

**Severity:** MEDIUM
**Files:** `src/llm/anthropic/tests.rs` (expand)
**Effort:** 0.5 day

SSE line parsing is well-tested (17 cases), but full stream flow is not.

**Tasks:**
- [ ] **`parse_retry_after()` header** — "5" → 5000ms, "0.5" → 500ms, missing header → None
- [ ] **Full stream accumulation** — sequence of SSE events → complete `LLMResponse`
- [ ] **Stream with tool use** — ToolUseStart + InputJsonDelta + ContentBlockStop → accumulated tool call

---

## Part 5: WebSocket & API Endpoint Tests

> **Risk:** MEDIUM-HIGH — 87% of endpoints untested, WebSocket lifecycle has zero tests.
> **Effort:** 4-5 days
> **Dependencies:** Part 2A (auth helpers needed)

### 5A. WebSocket Connection Lifecycle

**Severity:** HIGH
**Files:** `src/server/ws/tests.rs` (expand)
**Effort:** 1 day

85 tests cover message serialization. Zero tests cover actual connection behavior.

**Tasks:**
- [ ] **Subscription and event receipt** — subscribe to topic, broadcast event, verify client receives it
- [ ] **User-scoped events** — user A subscribes, broadcast event for user B, verify user A does NOT receive it
- [ ] **Run-scoped events** — subscribe to specific run_id, verify only matching events received
- [ ] **Lagged client** — fill broadcast buffer, verify client receives `Lagged` error and recovers
- [ ] **Concurrent subscribe/unsubscribe** — rapid topic changes don't panic or deadlock
- [ ] **Invalid token on connect** — verify WebSocket upgrade rejected with 401

### 5B. Workflow API Endpoints

**Severity:** HIGH
**Files:** `src/server/api/workflows/tests.rs` (new)
**Effort:** 1 day

16 endpoints, 0 tests.

**Tasks:**
- [ ] **CRUD lifecycle** — create workflow → get → update title → list (appears) → delete → list (gone)
- [ ] **Create step** — verify step linked to workflow, agent_id validated
- [ ] **Add edge** — verify source/target step validation
- [ ] **Step ports** — create input port, create output port, list both
- [ ] **Routing rules** — create rule for step, update, delete
- [ ] **Step documents** — attach document to step, verify join, remove
- [ ] **Auth required** — all endpoints return 401 without token
- [ ] **Not found** — nonexistent workflow_id returns 404

### 5C. Room & Session API Endpoints

**Severity:** HIGH
**Files:** `src/server/api/rooms/tests.rs` (new), `src/server/api/sessions/tests.rs` (new)
**Effort:** 1.5 days

28 endpoints combined, 0 tests.

**Tasks:**
- [ ] **Room CRUD** — create → get → update → delete
- [ ] **Room members** — add member → list → set members (replace all) → remove member
- [ ] **Room sessions** — create session → get → send message → get transcript → close
- [ ] **Session CRUD** — create → get → update title → delete
- [ ] **Session chat** — send message → get history → clear history
- [ ] **Session config** — update draft config, verify persistence
- [ ] **Save session agent** — verify agent created from draft config

### 5D. Remaining Endpoint Coverage

**Severity:** MEDIUM
**Files:** Various `src/server/api/*/tests.rs`
**Effort:** 1.5 days

**Tasks:**
- [ ] **Tools** — CRUD + agent tool assignment (`set_agent_tools`, `get_agent_tools`)
- [ ] **Tool Routers** — CRUD + tool assignment + router modes
- [ ] **Documents** — CRUD + search
- [ ] **Output Schemas** — CRUD
- [ ] **Prompt Templates** — CRUD
- [ ] **Collections** — CRUD + run + status
- [ ] **Agent Executions** — list + get + approve + cancel + messages + streaming
- [ ] **Results** — list + get + delete
- [ ] **System Config** — list + upsert + delete
- [ ] **Costs** — get with date range filtering

---

## Part 6: Cost Calculation & Observability

> **Risk:** MEDIUM — Silent cost miscalculation, no alerting on failures.
> **Effort:** 1 day
> **Dependencies:** Part 3A (mock infrastructure)

### 6A. Cost Calculation Tests

**Severity:** MEDIUM
**Files:** `src/server/hub/strategies/dag_step/tests.rs`
**Effort:** 0.5 day

`compute_cost(model_id, input_tokens, output_tokens)` has zero tests. Pricing could be silently wrong.

**Tasks:**
- [ ] **Known model pricing** — verify cost for `claude-sonnet-4-20250514` matches expected $/token
- [ ] **Unknown model fallback** — verify default pricing used for unrecognized model
- [ ] **Zero tokens** — verify 0 cost returned
- [ ] **Large token counts** — verify no overflow (u64 arithmetic)
- [ ] **Cost accumulation across steps** — verify total_cost_usd sums correctly in multi-step workflow

### 6B. Executor Error Path Tests

**Severity:** MEDIUM
**Files:** `src/server/executors/chat/tests.rs` (expand)
**Effort:** 0.5 day

**Tasks:**
- [ ] **LLM provider init failure** — provider returns error, verify error message streamed to client
- [ ] **Stream timeout** — 120s cleanup timeout, verify resources freed (mock with `tokio::time::pause()`)
- [ ] **Draft config deserialization failure** — malformed JSON in draft_config, verify graceful error

---

## Phase Summary

| Part | Focus | Items | Effort | Risk Addressed |
|------|-------|-------|--------|----------------|
| **1** | Database transaction safety & queries | 4 sections, ~25 tests | 3-4 days | Data integrity, race conditions |
| **2** | Auth & authorization hardening | 3 sections, ~20 tests | 2-3 days | Cross-tenant access, token exploits |
| **3** | DAG workflow integration tests | 5 sections, ~25 tests | 4-5 days | Core product correctness |
| **4** | LLM provider integration tests | 3 sections, ~20 tests | 2-3 days | Retry/rate-limit reliability |
| **5** | WebSocket & API endpoint tests | 4 sections, ~40 tests | 4-5 days | API surface coverage |
| **6** | Cost calculation & observability | 2 sections, ~10 tests | 1 day | Financial accuracy |
| | **Total** | | **16-21 days** | |

**Parallelization:** Parts 1, 2, and 4 can all run in parallel. Part 3 benefits from Part 1 (DB fixtures) but isn't blocked by it. Part 5 depends on Part 2A (auth test helpers).

---

## Out of Scope (Acceptable Risk)

- **Docker escape scenarios** — Assumes Docker daemon is secure; kernel-level CVEs are outside app testing scope
- **Symlink traversal in containers** — Low probability given `--cap-drop=ALL` and `--security-opt=no-new-privileges`
- **SQL injection** — All queries use parameterized `sqlx::query().bind()` with compile-time checking; explicit injection tests add minimal value
- **Anthropic/Grok `from_env()` tests** — Env var parsing is trivial (`std::env::var`); testing adds little value
- **Load/stress testing** — Performance testing is a separate initiative; this ticket focuses on correctness
- **Frontend tests** — Covered by separate FRONTEND_STATE_SYSTEM ticket
