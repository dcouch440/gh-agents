# Documenter Phase 2 — Hidden Chain Execution & Document Generation

## Summary

Phase 1 (complete) gets the Documenter running on the canvas and producing structured strategy output (analysis + per-document plans). Phase 2 takes that strategy and actually generates the documents by spawning hidden research + writer agent chains behind the scenes.

## Depends On

- Phase 1: Documenter strategy output working end-to-end (protocol synced, document defs CRUD, DocumenterPromptFilter, auto-wiring)

## Scope

### Hidden Step Infrastructure
- Add `visible boolean DEFAULT true` to `workflow_steps` table
- Hidden step support in DAG executor — canvas filters out `visible=false` steps, executor runs them normally
- Frontend: React Flow excludes hidden steps from rendering

### Documenter Expander
- Post-execution hook on documenter steps: parse strategy JSON output
- For each document in the strategy, create hidden workflow steps:
  1. **Researcher agent** — receives full upstream context + `research_strategy` from the plan, uses `required_capabilities` to determine tools
  2. **Writer agent** — receives researcher output + `writing_instructions` + `target_audience`, produces final document content
- Hidden steps are wired as a chain: documenter → researcher → writer (per document)

### Capability-Based Agent Routing
- Match `required_capabilities` from strategy output to agents via `tool_capabilities` table
- If capabilities are empty (context is sufficient), skip the researcher step and send directly to writer

### Document Writer Agent
- New system agent in `config/protocols.yaml` — specialized for long-form document writing
- System prompt focused on structured document generation with tone/audience awareness

### Document Persistence & Real-Time Updates
- Document save hook: hidden writer output → `documents.content` column, WebSocket broadcast
- `protocol_document_defs.document_id` FK linking each def to its produced document
- Real-time document population: blank document → content streams in via WebSocket

### Document Canvas Nodes
- New React Flow node type for document output display
- Document port resolution: `resolve_port_inputs()` pulls from document content when edge source is a document node
- Documents appear as expandable output nodes connected to the documenter step

## Key Design Decisions (from Phase 1 planning)

- **Full context to all agents** — each hidden agent (researcher + writer) gets ALL upstream context, not just filtered subsets
- **Strategy as the lens** — the per-document strategy from the Documenter Strategist is what differentiates focus, not context filtering
- **Multi-document support** — a single documenter step can produce N documents, each with its own hidden research→write chain
