# State Machine Audit Results

**Date:** 2026-04-04
**Scope:** Entire `src/` directory
**Skill:** `/state-machine`

## State Machines Found

15 lifecycle state machines identified (ordered transitions), 33 classification enums (unordered, not audited), and 5 string-based DB status fields.

---

## Machine 1: Workflow Execution Status (string-based)

**Table:** `workflow_executions.status`
**Valid values:** `pending`, `running`, `completed`, `failed`, `paused`, `workshop`, `workshop_running`

### Transition Table

| From | To | Trigger | Location |
|---|---|---|---|
| (insert) | `pending` | Creation | `db/pg_repo/collection.rs:305,393` |
| `pending` | `running` | DAG execution begins | `server/executors/collection_dag/mod.rs:389` |
| `pending` | `running` | Standalone workflow start | `server/api/workflows/run_handlers.rs:181` |
| `pending` | `running` | Service-level start | `server/services/workflows/run.rs:168` |
| `running` | `completed` | DAG succeeds | `server/executors/collection_dag/mod.rs:489` |
| `running` | `completed` | Standalone succeeds | `server/api/workflows/run_handlers.rs:203` |
| `running` | `completed` | Service succeeds | `server/services/workflows/run.rs:190` |
| `running` | `failed` | DAG error | `server/executors/collection_dag/mod.rs:536` |
| `running` | `failed` | Standalone error | `server/api/workflows/run_handlers.rs:220` |
| `running` | `failed` | Service error | `server/services/workflows/run.rs:209` |
| `running` | `paused` | AwaitingUser error | `server/executors/collection_dag/mod.rs:519` |
| `paused` | `completed` | Resume succeeds | `server/hub/dag/resume/mod.rs:263` |
| `paused` | `failed` | Resume fails | `server/hub/dag/resume/mod.rs:294` |
| (any) | `workshop` | Workshop mode setup | `server/api/workflows/workshop_handlers.rs:144,282,398` |
| `workshop` | `workshop_running` | Step execution starts | `server/api/workflows/workshop_handlers.rs:238` |
| `workshop_running` | `workshop` | Step execution ends | `server/api/workflows/workshop_handlers.rs:282` |

### State Diagram

```
                                  ┌──────────┐
                          ┌──────>│completed │
                          │       └──────────┘
┌───────┐    ┌───────┐    │       ┌──────────┐
│pending│───>│running│────┼──────>│  failed  │
└───────┘    └───────┘    │       └──────────┘
                          │       ┌──────────┐     ┌──────────┐
                          └──────>│  paused  │────>│completed │
                                  └──────────┘     │or failed │
                                                   └──────────┘

┌──────────┐    ┌──────────────────┐
│ workshop │<──>│workshop_running  │
└──────────┘    └──────────────────┘
```

### Findings

**F1. MISSING "running" FOR RESUME PATH (Intentional?)**
When a paused workflow resumes, it transitions directly `paused` -> `completed`/`failed` without going through `running` again. The `started_at` timestamp is already set, but any monitoring that checks `status = 'running'` will miss resumed executions.
- `server/hub/dag/resume/mod.rs:263,294`

**F2. WORKSHOP IS A DISCONNECTED SUBGRAPH**
The `workshop`/`workshop_running` states are an entirely separate lifecycle branch. Nothing transitions from the normal flow (`pending`/`running`/etc.) into workshop mode — it's set directly by API handlers. This is a parallel state machine sharing the same column.
- `server/api/workflows/workshop_handlers.rs:144,238,282,398`

**F3. NO CANCELLED STATUS (but column supports it)**
The `completed_at` SQL uses `WHEN status IN ('completed', 'failed')` — note: no `'cancelled'`. But the collection_run level DOES support `'cancelled'`. Workflow executions can't be cancelled independently of their collection run.

---

## Machine 2: Collection Run Status (string-based)

**Table:** `collection_runs.status`
**Valid values:** `running`, `completed`, `failed`, `paused`, `cancelled`

### Transition Table

| From | To | Trigger | Location |
|---|---|---|---|
| (insert) | `running` | Creation | `db/pg_repo/collection.rs:241` |
| `running` | `completed` | All workflows succeed | `server/executors/collection_dag/mod.rs:104` |
| `running` | `paused` | AwaitingUser | `server/executors/collection_dag/mod.rs:112` |
| `running` | `failed` | Execution error | `server/executors/collection_dag/mod.rs:118` |
| `running` | `cancelled` | User cancellation | (via update_collection_run_status) |

### State Diagram

```
              ┌──────────┐
          ┌──>│completed │
          │   └──────────┘
┌───────┐ │   ┌──────────┐
│running│─┼──>│  failed  │
└───────┘ │   └──────────┘
          │   ┌──────────┐
          ├──>│  paused  │
          │   └──────────┘
          │   ┌──────────┐
          └──>│cancelled │
              └──────────┘
```

### Findings

**CLEAN.** Initial state is `running` (not `pending`), all transitions are from `running` to terminal states. No wildcard arms. Straightforward.

**NOTE:** No transition from `paused` back to `running`. Resume path only exists at workflow execution level, not collection run level. A paused collection run's status appears to stay `paused` even after its workflows resume and complete.

---

## Machine 3: Agent Execution Status (string-based)

**Table:** `agent_executions.status`
**Valid values:** `pending`, `completed`, `failed`, `cancelled`

### Transition Table

| From | To | Trigger | Location |
|---|---|---|---|
| (insert) | `pending` | DB default | `db/types/execution.rs:129` |
| `pending` | `completed` | Strategy completes | `server/hub/execution/strategies/mod.rs:111` |
| `pending` | `completed` | Manager dispatch success | `server/executors/manager_dispatch/mod.rs:179` |
| `pending` | `failed` | Manager dispatch error | `server/executors/manager_dispatch/mod.rs:221` |
| `pending` | `failed` | System node dispatch error | `server/executors/dispatch/system_node.rs:337` |
| `pending` | `cancelled` | User cancellation | `server/api/cancellation/mod.rs:39` |
| `pending` | `cancelled` | Cancel token triggered | `server/executors/manager_dispatch/mod.rs:207` |

### State Diagram

```
          ┌──────────┐
      ┌──>│completed │
      │   └──────────┘
┌───────┐ ┌──────────┐
│pending│>│  failed  │
└───────┘ └──────────┘
      │   ┌──────────┐
      └──>│cancelled │
          └──────────┘
```

### Findings

**F4. NO "running" STATE — SKIPPED ENTIRELY**
Agent executions go directly from `pending` to terminal states. There is no `status = 'running'` transition anywhere in the code. The execution is "running" implicitly while the engine processes it, but the DB never reflects this.
- If any UI or query checks `status = 'running'` for agent executions, it will always return empty.
- This is the opposite problem of Finding 2 in the async audit (workflow status stuck "running") — here, "running" is never set at all.

---

## Machine 4: Protocol Execution Status (string-based)

**Table:** `protocol_executions.status`
**Valid values:** `running`, `complete`, `failed`

### Transition Table

| From | To | Trigger | Location |
|---|---|---|---|
| (insert) | `running` | Phase creation | `server/hub/protocols/execution_recorder.rs` (via create_phase) |
| `running` | `complete` | Phase succeeds | `server/hub/protocols/execution_recorder.rs:137` |
| `running` | `failed` | Phase fails | `server/hub/protocols/execution_recorder.rs:139` |

### Findings

**F5. "complete" VS "completed" INCONSISTENCY**
Protocol executions use `"complete"` while every other status field in the system uses `"completed"`. This is a semantic inconsistency — any query or UI that joins across tables and checks for `status = 'completed'` will miss protocol execution rows.
- Protocol: `"complete"` at `db/pg_repo/protocol.rs:324`
- Workflow: `"completed"` at `db/pg_repo/collection.rs:349`
- Agent: `"completed"` at `db/pg_repo/execution.rs:58`
- Collection run: `"completed"` at `db/pg_repo/collection.rs:281`

---

## Machine 5: TaskStatus (in-memory, task_registry)

**Enum:** `server/state/task_registry/mod.rs:17`
**Variants:** `Running`, `Completed`, `Cancelled`, `Failed`

### Transition Table

| From | To | Trigger | Location |
|---|---|---|---|
| (new) | `Running` | `spawn_task()` | `task_registry/mod.rs:121` |
| `Running` | `Completed` | `mark_completed()` | `task_registry/mod.rs:148` |
| `Running` | `Failed` | `mark_failed()` | `task_registry/mod.rs:156` |
| `Running` | `Cancelled` | `cancel_task()` | `task_registry/mod.rs:138` |
| `Running` | `Cancelled` | `cancel_all()` | `task_registry/mod.rs:184` |

### Findings

**CLEAN.** All transitions explicitly guarded with `if entry.status == TaskStatus::Running`. No wildcard arms. Terminal states are truly terminal. `cleanup_before()` correctly preserves `Running` entries while removing old terminal entries.

---

## Machine 6: DispatchStatus (rendering layer)

**Enum:** `server/hub/context/dispatch_status/types.rs:8`
**Variants:** `InProgress`, `Completed`, `Failed`, `Cancelled`

### Findings

**CLEAN.** Maps 1:1 from TaskStatus via explicit match at `fetch.rs:48-53`. Has `unreachable!()` for `TaskStatus::Running` (pre-filtered). No wildcard arms.

---

## Machine 7: Node Status (rendering layer, string-based)

**Field:** `NodeSnapshot.status` at `server/hub/board/state/types.rs:134`
**Values derived from:** `server/services/workflow_agent/state.rs:169-209`

| Priority | Status | Condition |
|---|---|---|
| 1 | `configuring` | TaskStatus::Running in active_tasks |
| 2 | `running` | DAG step actively executing |
| 3 | `error` | dispatch.status == "failed" |
| 4 | `completed` | step.pinned or has run_results_summary |
| 5 | `configured` | step.child_workflow_id is Some |
| 6 | `described` | step.description is non-empty |
| 7 | `idle` | Default fallback |

### Findings

**CLEAN.** Pure derivation function, no mutations. Priority-ordered fallthrough with explicit checks, no wildcards.

---

## Cross-Cutting Findings

### F6. ALL DB STATUS FIELDS ARE STRINGS, NOT ENUMS

Every status column uses `String` / `TEXT` with string literals scattered across the codebase. There is no compile-time enforcement that status values are valid. A typo like `"complted"` would silently produce an invalid state.

Affected tables: `workflow_executions`, `collection_runs`, `agent_executions`, `protocol_executions`, `room_sessions`

The in-memory enums (`TaskStatus`, `DispatchStatus`) are properly typed with compile-time enforcement.

### F7. NO WILDCARDS ON STATE ENUMS

Zero wildcard (`_` or `other`) match arms found on any state/status enum. All matches are explicit. Adding a new variant to `TaskStatus` or `DispatchStatus` will produce a compile error at every match site. This is excellent.

---

## Summary

| # | Finding | Severity | Type |
|---|---|---|---|
| F1 | Resume skips "running" status | Low | Missing transition |
| F2 | Workshop is disconnected subgraph | Info | Architecture |
| F3 | No "cancelled" for workflow executions | Medium | Missing state |
| F4 | Agent execution never enters "running" | Medium | Missing state |
| F5 | "complete" vs "completed" inconsistency | Medium | Naming bug |
| F6 | All DB statuses are untyped strings | Medium | Type safety |
| F7 | No wildcard match arms on enums | Pass | Clean |

No dead states. No trapping states. No contradictory transitions. No wildcard absorption.

The main actionable finding is F5 ("complete" vs "completed") which is likely a bug that could cause join/query issues. F4 and F1 are design decisions worth documenting but may be intentional.
