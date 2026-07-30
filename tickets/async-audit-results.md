# Async Audit Results

**Date:** 2026-04-04
**Scope:** Entire `src/` directory
**Skill:** `/audit-async`

## Checks Passed (Clean)

- **Mutex guards across .await** — All 48 lock instances properly scoped. No guards held across `.await` boundaries. std::sync::Mutex used correctly for CPU-only work in async contexts with explicit `{ }` scoping.
- **std::sync vs tokio::sync Mutex** — Correct usage everywhere. tokio::sync used only when guard spans await, std::sync used for non-blocking operations.
- **parking_lot** — Not used in codebase.
- **select! loops** — All futures pinned outside loops. Intervals created before loop entry. No recreation patterns.
- **Blocking Drop** — No custom `impl Drop for` in the codebase.
- **std::thread::sleep** — Not used (tokio::time::sleep used correctly).
- **std::net:: blocking I/O** — Not used (only type imports like IpAddr, SocketAddr).

---

## Finding 1: Blocking File I/O in Async Context

**Severity: High (Starvation)**

Synchronous `std::fs::` operations called from async functions without `spawn_blocking`. With Tokio's default 4-thread pool, concurrent blocking calls can freeze the runtime.

### Locations

| File | Function | Blocking Calls |
|---|---|---|
| `src/server/hub/execution/strategies/system_node/mod.rs:184` | `handle_complete_system()` (async) | `validate_written_files()` -> `std::fs::read_to_string`, `read_dir`, `remove_file` |
| `src/server/hub/execution/strategies/system_node/mod.rs:210` | `handle_run_command()` (async) | `validate_written_files()` -> same |
| `src/server/hub/execution/strategies/workflow_agent/mod.rs:82,113` | `host_run_command()` (async) | `snapshot_board_files()` -> `std::fs::read_to_string`, `read_dir` (in loop) |
| `src/server/services/system_node/file_reader.rs:46-174` | `read_agent_configs()`, `read_config()`, `read_topology()` (sync, called from async) | `std::fs::read_to_string` throughout |
| `src/server/services/workflow_agent/file_reader.rs:32-125` | `read_topology()`, `read_node()`, `read_all_nodes()`, `snapshot_board_files()` (sync, called from async) | `std::fs::read_to_string`, `read_dir` |
| `src/server/services/canvas_sync/filesystem.rs:12-74` | `write_node_file()`, `remove_node_file()`, `rewrite_topology()` (sync, called from async) | `std::fs::create_dir_all`, `write`, `remove_file` |
| `src/server/executors/dispatch/system_node.rs:89` | `run_system_node_task()` (async) | `std::fs::create_dir_all` |

### Fix

Option A (minimal): Wrap call sites in `tokio::task::spawn_blocking()`
Option B (thorough): Convert `file_reader.rs` functions to `async fn` using `tokio::fs::`

---

## Finding 2: Cancellation Safety — Workflow Status Stuck "Running"

**Severity: Critical (State corruption)**

**File:** `src/server/services/workflows/run.rs:165-222`

A spawned task marks execution as "running" before `execute_workflow_via_engine().await`, then updates to "completed" or "failed" after. If the task is cancelled during execution, the status stays "running" forever.

```
Line 167: update_status(execution_id, "running")  <- persisted
Line 171: execute_workflow_via_engine().await       <- cancellation point
Line 187: update_status(execution_id, "completed")  <- never runs
```

**Impact:** Workflow records permanently stuck in "running" state. UI shows stale progress. Retries may be blocked.

**Fix:** Use a guard that sets status to "cancelled" or "failed" on drop, or wrap the execution in a pattern that guarantees status update regardless of cancellation.

---

## Finding 3: Cancellation Safety — Protocol Phase Status Not Updated

**Severity: Critical (State corruption)**

**File:** `src/server/hub/dag/pipeline/agent_executor.rs:213-442`

For each workforce agent, creates a protocol execution row with status="running" (line 213-222), executes the agent LLM (line 351-359), and updates status to "complete"/"failed" (lines 369-442). If cancelled during agent execution, the phase row stays "running".

**Impact:** Database records for protocol execution phases remain in "running" status permanently.

**Fix:** Same guard pattern as Finding 2.

---

## Finding 4: Cancellation Safety — Step Output Not Persisted

**Severity: Critical (Data loss)**

**Files:** `src/server/hub/dag/single/mod.rs:159-169`, `src/server/hub/dag/utils/mod.rs:43-62`

After engine execution returns, the code records output in-memory (line 161) then snapshots to database (line 168). If cancelled between the in-memory update and the DB write, step outputs are lost.

Additionally, the `StepCompleted` broadcast event (lines 171-184) is skipped, so clients never receive completion notification.

**Impact:** Step outputs recorded in memory but never persisted. Downstream steps may use stale or missing data. UI shows stale progress.

---

## Finding 5: Cancellation Safety — Container Leak on Cancellation

**Severity: High (Resource leak)**

**Files:**
- `src/server/hub/dag/single/mod.rs:108-157`
- `src/server/hub/dag/pipeline/runner.rs:87-154`

Docker containers (with optional VPN sidecar) are created before execution and destroyed after. If cancelled during execution, `destroy_optional_container()` never runs.

```
Line 108: create_optional_container().await   <- container exists
Line 116: run_step_via_engine().await          <- cancellation point
Line 157: destroy_optional_container().await   <- never runs
```

**Impact:** Orphaned Docker containers consuming resources. Periodic reaper exists but containers linger longer than necessary.

**Fix:** Use an RAII-style guard struct that calls `destroy_optional_container` on drop, or use `scopeguard`.

---

## Summary

| # | Finding | Severity | Category |
|---|---|---|---|
| 1 | Blocking `std::fs::` in async (7 locations) | High | Starvation |
| 2 | Workflow status stuck "running" on cancellation | Critical | State corruption |
| 3 | Protocol phase status not updated on cancellation | Critical | State corruption |
| 4 | Step output not persisted to DB on cancellation | Critical | Data loss |
| 5 | Container orphaned on cancellation (2 locations) | High | Resource leak |

Checks 1-2 (mutex patterns), Check 4 (select! loops), and Check 6 (blocking Drop) all passed clean.
