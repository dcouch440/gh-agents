# Documenter: Write/Research phase outputs not persisted

## Problem

The documenter pipeline runs all three phases (strategy, research, write) successfully, but only the strategy phase persists its output. Research and write phase results are generated in memory, used to determine success/failure status, then discarded.

### What's missing

1. **`protocol_executions.output_content`** — research and write rows are created with status `running` via `create_execution_row()` but never updated after the LLM returns. `output_content`, `tokens_in`, `tokens_out`, `cost_usd`, `model`, and `status` remain NULL/running.

2. **`protocol_document_defs.document_id`** — the write phase checks for a linked `document_id` to call `update_document()`, but no code path creates and links a document entity automatically. If `document_id` is NULL (which it is by default), the generated document text is thrown away.

## Impact

- The generated document is not viewable after the workflow completes
- Token usage and cost are not tracked for research/write phases
- Protocol execution rows are left in `running` status permanently

## Expected behavior

- All three phase rows in `protocol_executions` should be updated to `complete` with `output_content`, token counts, cost, and model after the LLM returns
- The write phase should either create a document entity and link it to the doc def, or persist the content directly in `protocol_executions.output_content` so it's retrievable

## Where to fix

- `src/server/hub/dag/documenter/mod.rs` — after each `engine.execute()` call in `execute_research_phase` and `execute_write_phase`, update the protocol_executions row with the result
- Consider auto-creating a document entity in the write phase and linking it to the doc def
