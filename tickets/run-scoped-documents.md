# Run-Scoped Documents & `read_document_by_def_id` Tool

## Problem

When a documenter step runs, it produces documents based on document definitions (`protocol_document_defs`). Currently:

1. **Documents are not run-scoped.** The first run creates a document and links it via `protocol_document_defs.document_id`. Every subsequent run **overwrites the same document row** in place.
2. **Concurrent runs race.** If the same workflow runs 5 times simultaneously, all 5 runs read/write the same document — data corruption.
3. **Downstream agents can't read documents.** The `read_document` tool requires a `document_id` (from the `documents` table), but agents only have access to document def names/IDs. The actual `document_id` is never surfaced in prompts or envelopes.
4. **The documenter envelope carries status only.** Output is `{ "documents": [{ "name": "...", "status": "complete" }] }` — no document IDs or content.

## Current Architecture

```
protocol_document_defs          documents
┌──────────────────────┐       ┌──────────────────────┐
│ id (def UUID)        │       │ id (doc UUID)        │
│ step_id              │       │ content              │
│ name                 │       │ title                │
│ description          │       │ workflow_id          │
│ target_length        │       │ source_protocol_     │
│ document_id ─────────┼──────>│   step_id            │
│   (nullable, shared) │       │ (NO run_id)          │
└──────────────────────┘       └──────────────────────┘
```

- `document_id` on the def is a **global singleton** — all runs share one document
- `persistence.rs` calls `determine_persist_action()`: if `document_id` exists → `Update`, else → `CreateAndLink`
- `link_document_to_def(def_id, doc.id)` sets the global pointer

## Proposed Solution

### 1. New join table: `run_documents`

```sql
CREATE TABLE run_documents (
    id           uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    run_id       uuid NOT NULL,    -- workflow_runs.id
    def_id       uuid NOT NULL REFERENCES protocol_document_defs(id),
    document_id  uuid NOT NULL REFERENCES documents(id),
    created_at   timestamptz DEFAULT now(),
    UNIQUE(run_id, def_id)         -- one document per def per run
);
CREATE INDEX idx_run_documents_run_def ON run_documents(run_id, def_id);
```

This maps `(run_id, def_id) → document_id`. Each run produces its own document instances.

### 2. Update documenter persistence

**File:** `src/server/hub/dag/documenter/persistence.rs`

Change `determine_persist_action` logic:
- Always check `run_documents` for `(run_id, def_id)` first
- If found → `Update` that document
- If not found → `CreateAndLink` a new document, insert into `run_documents`
- The existing `protocol_document_defs.document_id` can be kept as "latest for UI preview" or deprecated

`DocumentPersistContext` needs a new field:
```rust
pub(super) struct DocumentPersistContext {
    pub run_id: Uuid,          // NEW — current workflow run
    pub document_id: Option<Uuid>,
    pub def_id: Option<Uuid>,
    pub doc_name: String,
    pub user_id: Uuid,
    pub workflow_id: Uuid,
    pub step_id: Uuid,
}
```

### 3. New tool: `read_document_by_def_id`

```rust
// Input: { "def_id": "uuid" }
// Backend resolves: (def_id, current_run_id) → document_id → content
```

- Registered in `src/tools/registry/` alongside `read_document`
- Implementation in `src/server/tools/documents/mod.rs`
- The `run_id` is NOT passed by the agent — it's injected from execution context (see below)
- Returns document content, title, etc. or `{ "error": "Document not yet generated" }` if the documenter step hasn't run yet in this run

### 4. Run-scoped tool context

During DAG execution, the `run_id` is known by the executor. It needs to flow into the tool execution context so `read_document_by_def_id` can resolve the correct document without the agent knowing the run ID.

**Current flow:** `ExecutionEngine` → `ExecutionStrategy` → `execute_tool()` → tool handler
**Needed:** The tool handler needs access to `run_id`. Options:
- Add `run_id: Option<Uuid>` to `DagStepStrategy` (it already has step context)
- Pass through a `ToolExecutionContext` struct that includes `run_id`
- Or store `run_id` on `AppState` per-execution (less clean)

The `DagStepStrategy` approach is cleanest — it already carries `workflow_id`, `step_id`, etc.

### 5. Document def IDs in agent prompts

The `DocumenterPromptFilter` (`src/server/hub/engine/filters/documenter_prompt/mod.rs`) currently formats defs as:
```
1. "API Reference" — Complete REST API docs (target: ~2500 characters)
```

It needs to include the def ID so agents can reference it:
```
1. "API Reference" (def_id: abc-123) — Complete REST API docs (target: ~2500 characters)
```

This way downstream agents know which `def_id` to pass to `read_document_by_def_id`.

## Files to Modify

| File | Change |
|------|--------|
| `migrations/NNNN_run_scoped_documents.sql` | **New** — `run_documents` table |
| `src/db/mod.rs` | Add `RunDocumentRow` struct |
| `src/db/traits.rs` | Add `get_run_document`, `create_run_document` to `WorkflowRepo` |
| `src/db/pg_repo/mod.rs` | Implement queries |
| `src/server/hub/dag/documenter/persistence.rs` | Run-scoped create/update logic |
| `src/server/hub/dag/documenter/phases.rs` | Pass `run_id` into `DocumentPersistContext` |
| `src/server/tools/documents/mod.rs` | Add `execute_read_document_by_def_id` |
| `src/tools/registry/mod.rs` | Register `read_document_by_def_id` tool |
| `src/server/hub/dag/single/mod.rs` | Pass `run_id` through to tool execution |
| `src/server/hub/engine/filters/documenter_prompt/mod.rs` | Include def IDs in prompt |
| `src/server/hub/protocols/compilers/documenter/prompt.rs` | Update `format_document_defs_section` |

## Key Design Decisions

- **Join table over column**: `run_documents` keeps run-scoping isolated from the general `documents` table which also stores static KB docs and user uploads
- **Agent doesn't know `run_id`**: The backend injects it from execution context — agents just call `read_document_by_def_id(def_id)` and get the right document for their run
- **Backward compatible**: Existing `protocol_document_defs.document_id` can be kept as "latest" pointer for UI display (canvas document preview), or updated to always point to the most recent run's document
- **Cleanup policy**: Old `run_documents` entries can be cleaned up with run retention (future concern, not blocking)

## Open Questions

1. Should `protocol_document_defs.document_id` be deprecated or kept as "latest for UI"?
2. Should the documenter envelope be updated to include document IDs so downstream steps can also get them via port resolution (in addition to the tool)?
3. Retention policy for per-run documents — keep all, keep N most recent, or clean up with the run?
