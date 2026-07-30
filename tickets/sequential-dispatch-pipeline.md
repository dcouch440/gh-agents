# Dispatch pipeline runs system node agents sequentially instead of in parallel

## Problem

When a board is submitted (dispatch/generate), the system node agents for each node execute **sequentially** even when the nodes have no dependencies on each other (fan-out pattern). A board with 5 independent nodes takes 5x as long as it should.

## Root Cause

`src/server/services/dispatch/sequential.rs:82-132`

The `run_sequential_design_pipeline()` function iterates all steps in topological order and **directly awaits** each dispatch:

```rust
for step_id in &topo_order {
    // ...
    if has_instruction {
        run_system_node_dispatch(...).await;   // Line 105 — blocks loop
    } else {
        run_system_node_propagation(...).await; // Line 121 — blocks loop
    }
    // Re-reads step to check handoff changes (lines 133-154)
}
```

Each step waits for the previous step to fully complete (LLM call, file sync, DB persist) before the next one starts. For a fan-out of N independent nodes, this is N sequential LLM round-trips instead of 1 parallel batch.

## Why It Exists

After each dispatch completes, the code re-reads the step from the DB (lines 133-154) to check if the handoff description changed, which triggers downstream propagation. The sequential design ensures propagation inputs are fresh. This is only needed for steps that are downstream of other dispatched steps — independent fan-out nodes don't need this ordering.

## What's NOT the Problem

The rest of the system correctly parallelizes:
- **DAG executor** (`hub/dag/orchestration/mod.rs:297-370`) — uses `JoinSet` for multi-step levels
- **Collection DAG executor** (`executors/collection_dag/mod.rs:214-227`) — spawns workflows in parallel with `join_all`
- **Workforce agent executor** (`hub/dag/pipeline/agent_executor.rs:113-173`) — uses `JoinSet` for same-level agents
- **Topological sort** (`hub/dag/utils/graph.rs:69-133`) — correctly groups independent steps into the same level

The sequential bottleneck is only in the dispatch pipeline entry point.

## Fix

Group dispatch steps by topological level (the sort already exists at line 68) and run each level in parallel:

```rust
let levels = topological_sort_levels(&steps, &edges);
for level in levels {
    let mut join_set = tokio::task::JoinSet::new();
    for step_id in level {
        // ... spawn dispatch ...
        join_set.spawn(async move {
            run_system_node_dispatch(...).await
        });
    }
    while let Some(result) = join_set.join_next().await {
        // collect results
    }
    // Check handoff propagation AFTER level completes
}
```

This preserves the propagation check (only needed between levels, not within a level) while parallelizing independent nodes.

## Key Files

- `src/server/services/dispatch/sequential.rs:31-177` — the sequential pipeline (primary fix target)
- `src/server/services/dispatch/sequential.rs:238-247` — `run_system_node_dispatch()` wrapper
- `src/server/api/board/mod.rs:230-240` — where the sequential pipeline is spawned
- `src/server/hub/dag/utils/graph.rs:69-133` — `topological_sort_levels()` (reuse this)

## Found by

User report + `/audit-async` investigation
