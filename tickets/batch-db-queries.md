# Batch DB Queries — Fix N+1 Patterns in Prompt Composition & Pipeline Snapshot

## Objective

Eliminate per-item database round trips in two hot paths: prompt composition and pipeline snapshot building.

---

## Issues

### 1. Document N+1 in `prompts.rs`

**File:** `src/server/hub/dag/utils/prompts.rs:154-175`

Per-step prompt composition calls `d_repo.get_document(sd.document_id).await` inside a loop — one DB query per document. If a step has 5 attached documents, that's 5 sequential queries.

**Fix:** Collect all document IDs from `list_step_documents`, then batch-fetch with a single `get_documents_by_ids()` query (may need a new repo method).

### 2. Per-Step `get_step()` in Pipeline Snapshot

**File:** `src/server/services/pipeline/snapshot.rs:139-148`

Upstream step context is loaded by calling `repo.get_step(upstream_id).await` in a loop — one query per upstream step.

**Fix:** Use `list_steps(workflow_id)` to load all steps once, then filter in-memory. The step list is already available in most call paths.

### 3. Serial Agent Queries in Protocol Resolution

**File:** `src/server/services/protocols/resolve.rs:47-111`

Three functions (`resolve_agent_names`, `resolve_agent_schemas`, `resolve_agent_tools`) each loop over ports and call `get_persisted_agent(port.agent_id).await` per unique agent. Dedup prevents true N+1, but queries are still serial.

**Fix:** Collect unique agent IDs, batch-fetch with `get_agents_by_ids()` (already exists in the agents repo).

---

## Impact

These patterns add latency to every step execution. Batching reduces DB round trips from O(N) to O(1) per operation.

## Verification

- `cargo test hub::dag::` — DAG tests pass
- `cargo test services::pipeline::` — pipeline tests pass
- `cargo test services::protocols::` — protocol tests pass
