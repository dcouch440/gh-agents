# Bug: agent_executions not linked to workflow_executions

## Summary

When workflows run via the `POST /workflows/:id/run` endpoint, the resulting `agent_executions` rows have `workflow_execution_id = NULL`. This means there's no way to query per-step execution details for a given workflow run.

## Evidence

```sql
SELECT id, workflow_execution_id, status
FROM agent_executions
WHERE id = '38367ed2-d843-4329-bad7-b50e5dc8459c';

-- workflow_execution_id is NULL
```

All 7 workflow executions for workflow `8f3206ea` have zero linked agent_executions.

## Root Cause

The `run_workflow` handler in `src/server/api/workflows/mod.rs` creates a `WorkflowExecutionContext` with `stage_execution_id` set to the workflow execution ID, but the DAG executor's step execution code path likely doesn't pass this through when creating `agent_executions` rows.

## Impact

- Cannot show per-step breakdown in historical run view (only aggregated outputs)
- Cannot trace which agent produced which output for a past run
- Cannot show token usage, duration, or errors per step in history

## Files to Investigate

- `src/server/hub/dag/mod.rs` — `execute_single_step` and how it creates agent_executions
- `src/server/api/workflows/mod.rs` — `run_workflow` handler, how `WorkflowExecutionContext` is constructed
- `src/db/traits/mod.rs` — `create_agent_execution` signature
