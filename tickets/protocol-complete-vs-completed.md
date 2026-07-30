# Protocol status uses "complete" instead of "completed"

## Problem

The `protocol_executions` table uses `"complete"` as its success status, while every other status field in the system uses `"completed"`:

| Table | Success Status |
|---|---|
| `workflow_executions` | `"completed"` |
| `collection_runs` | `"completed"` |
| `agent_executions` | `"completed"` |
| `room_sessions` | `"completed"` |
| **`protocol_executions`** | **`"complete"`** |

Any query or UI that joins across tables and filters on `status = 'completed'` will silently miss protocol execution rows.

## Locations

**Where "complete" is set:**
- `src/db/pg_repo/protocol.rs:324` — SQL UPDATE uses `$2` param
- `src/server/hub/protocols/execution_recorder.rs:137` — passes `"complete"` to `update_phase()`

**Where "completed" is used (for reference):**
- `src/db/pg_repo/collection.rs:349` — workflow executions
- `src/db/pg_repo/collection.rs:281` — collection runs
- `src/db/pg_repo/execution.rs:58` — agent executions

## Fix

1. Change `"complete"` to `"completed"` in `execution_recorder.rs:137`
2. Run a DB migration: `UPDATE protocol_executions SET status = 'completed' WHERE status = 'complete'`
3. Update the SQL completion timestamp logic in `pg_repo/protocol.rs` if it checks for `'complete'` specifically

## Found by

`/state-machine` audit (Finding F5)
