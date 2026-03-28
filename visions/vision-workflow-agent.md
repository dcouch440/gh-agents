# Workflow Agent — Vision

## What It Is

A conversational agent that helps users design workflow topology — nodes, edges, and node descriptions. It works in a file-based repository that stays in sync with the user's canvas via live sync. Like Claude Code for workflow design: the user talks, the agent edits files, the user sees changes on the canvas in real-time.

## Architecture: Live Sync + Single Agent

The frontend sends every canvas change to the backend live (debounced). The backend DB is always the current board state. The agent works in a repo that syncs bidirectionally with the DB:

- User edits canvas → frontend sends change → DB updates → repo syncs
- Agent edits repo via `run_command` → DB updates → frontend gets websocket update

One agent. One repo. No dispatch layer. No stale snapshots.

```
User edits canvas
    ↓
Frontend sends change (debounced) → backend writes to DB
    ↓
Agent's repo reflects current board state
    ↓
User sends message
    ↓
Agent reads repo (always fresh), responds, edits files via run_command
    ↓
File changes sync to DB → frontend gets websocket update → canvas updates
    ↓
User sees changes, continues conversation
```

### Live Sync Infrastructure

The WebSocket infrastructure already exists:
- **Backend → Frontend**: `EventBus.broadcast()` with pre-serialized envelopes. Dispatch store already uses this pattern (`DISPATCH_STREAM_TOKEN`, `DISPATCH_COMPLETED`).
- **Frontend → Backend**: `WebSocketContext.send()` exists but isn't wired for canvas events yet.
- **Debounce**: `useAutoSave.ts` already debounces at 500ms for step configs — same pattern for canvas elements.
- **Diffing**: Board serializer's `diff.rs` can diff any two snapshots incrementally.

The gap is wiring canvas element changes through the websocket to persist in `canvas_snapshots`. The pieces exist on both sides.

### Filesystem Watchers

The agent has full shell access — it can read with `cat`, write with heredocs, use `sed`, `python`, anything. We can't parse shell commands to figure out what got read or written. Instead, two filesystem watchers run in the container alongside the agent. The agent doesn't know they exist.

**Write watcher** (inotify/fswatch on the repo directory). Any file create, modify, or delete triggers:
1. Validate the file (JSON schema check for `topology.json`, existence check for `nodes/*.md`)
2. Reject invalid writes (revert the file, return error to the agent via `run_command` output)
3. Sync valid writes to DB immediately
4. Broadcast websocket update to frontend
5. Update `last_modified` timestamp for that file

**Read watcher** (inotify `IN_ACCESS` / fanotify). Any file read updates `last_read_by_agent` for that file. This is silent — the agent just uses `cat` or whatever and the read is tracked automatically.

This is the same write-time validation the system node agent uses, extended with read tracking and live DB sync.

### Concurrent Edit Resolution

The user and agent can edit the same node simultaneously through different paths:
- **User** → frontend debounce (500ms) → websocket → DB → repo
- **Agent** → `run_command` → filesystem watcher → DB → websocket → frontend

Two rules handle conflicts:

**1. Read-before-write with freshness check.** The write watcher checks `last_read_by_agent >= last_modified` before accepting a write. If the file was modified after the agent's last read (user edited it between the agent's read and write), the write is rejected:

```
Error: nodes/research.md modified since your last read. Read it again.
```

The agent re-reads (tracked by the read watcher), sees the user's version, and adapts. One extra tool call, not a wasted turn.

**2. Debounce cancellation on agent write.** When the write watcher syncs a file to DB, the backend sends a websocket event with the node ID. The frontend cancels any pending debounce for that node and applies the agent's version. If the debounce already fired and is in flight, the backend rejects it — the user's write has an older timestamp than the agent's write.

```
Agent writes nodes/research.md → write watcher validates → DB updates (last_modified = T1)
    ↓
Websocket fires → frontend cancels pending debounce for research
    ↓
If debounce already in flight → backend sees user's write is based on pre-T1 state → rejected
    ↓
Frontend applies agent's version via websocket
    ↓
User edits again if they want to override → fresh debounce → syncs normally
```

No locks, no ETags, no pausing. Just per-file timestamps tracked by the filesystem watchers. The agent can't write stale. The user's pending edits don't overwrite the agent. If the user wants to override the agent, they edit again — their next debounce carries a fresh timestamp and succeeds.

### Why This Works Now (and Didn't Before)

The original vision was a repo-based conversational agent, but it had three problems:

1. **Stale snapshots** — the user edits the canvas between turns, agent's repo is outdated
2. **History incoherence** — agent's history references board states that no longer exist
3. **Lock pressure** — keeping the repo in sync requires locking or expensive round-tripping

Live sync eliminates problem 1 — the repo is always current because the DB is always current. Problem 2 is handled the same way the system node agent handles it: the `<current_state>` is rebuilt every turn from the repo, so the agent always sees what actually exists regardless of what its history says. Problem 3 disappears because sync is continuous and bidirectional — no locks, no snapshots per message.

### "Submit" Becomes "Generate"

Live sync separates two things that "submit" currently bundles:

- **Save state** — continuous, automatic. Every canvas change persists to DB via websocket.
- **Trigger execution** — explicit user action. The user clicks "Generate" when they want system node agents to configure the nodes and runtime agents to execute.

The board serializer's diff logic stays the same — it just diffs against the last generation instead of the last submit.

## Key Design Principles

**The user owns the canvas.** The agent edits the repo, changes sync to the canvas. The user can undo, overwrite, or ignore anything. If the user edits the canvas, the repo reflects it on the next sync.

**The repo is the board.** The agent reads and writes the same structure the canvas uses. No separate data model. The repo is a file-based projection of the DB board state.

**Conversational with live context.** The agent has a persistent session with conversation history. Every turn, the system prompt includes a `<current_state>` block rebuilt from the repo — same pattern as the system node agent. History might reference old board states, but `<current_state>` is always fresh.

**System node agents own the layer below.** The workflow agent writes node descriptions (task text). The system node agent reads that as `<task>`, names the node, designs the agent team, writes the output description in `config.json`. The workflow agent never configures agents within nodes.

**Cancellation is free.** Every file write syncs to DB immediately. The user can kill the agent at any point and keep whatever's on the board. No cleanup, no rollback, no orphaned state. If they want to revert, they rebase to a saved version.

## Repository

The agent works in a repo synced to the workflow's board state at `{mount}/workflows/{wf_id}/board/`. Same file-based pattern as the system node agent, one level up.

```
./
├── topology.json
└── nodes/
    ├── research.md
    ├── fact_checker.md
    └── report.md
```

### `topology.json`

Node dependency graph. Same shape as the system node agent's topology.

```json
{
  "nodes": {
    "research": { "depends_on": [] },
    "fact_checker": { "depends_on": ["research"] },
    "report": { "depends_on": ["fact_checker"] }
  }
}
```

Slugs, not UUIDs. Backend maps slugs to step IDs during sync.

No positions. The backend auto-layouts from the dependency graph using `compute_execution_levels` (Kahn's algorithm). Same level = same x, offset y within level. Users drag to adjust after.

### `nodes/{slug}.md`

Markdown, not JSON. Node descriptions are briefs — they can be multi-paragraph documents with scope, quality criteria, constraints, and context. Markdown lets the workflow agent write at the level of detail the system node agent needs to expand from.

```markdown
# Research

Research competitor pricing data from public sources.

## Scope
- Focus on direct competitors in the project management SaaS space
- 2024-2025 data only
- Top 5 competitors by market share

## Quality Criteria
- Every pricing claim backed by a public source URL
- Distinguish between published pricing and estimated/reported pricing
- Flag any data older than 6 months

## Constraints
- Do not contact competitors directly
- Public sources only: pricing pages, press releases, analyst reports
```

The entire markdown file is the task text that the system node agent receives as `<task>`. The system node agent reads this brief and decides:
- **What agents are needed** — one researcher or a team
- **What name to give the node** — writes `config.json` with a display name
- **What this step produces** — writes `config.json` with an output description (used as `<previous_step>` for downstream nodes)
- **How to design the agent team** — writes `topology.json` + `agents/*.json` with system prompts that encode the methodology, quality criteria, and domain expertise the brief describes

The workflow agent writes the brief — intent, scope, quality standards, constraints. The system node agent turns the brief into a working system. Simple descriptions work too — a node file can be a single line ("Summarize the research into a blog post") when the task is simple. Markdown scales from one line to a full spec.

### Slugs and Names

Nodes have two identifiers:
- **Slug** — file identifier in the repo (`research`, `fact_checker`, `unnamed_01`). Immutable once created. Used in `topology.json` keys and `nodes/{slug}.md` filenames.
- **Name** — display name in the UI ("Market Research", "Fact Checker"). Set by the system node agent when it writes `config.json`. Absent until the system node agent runs.

When the agent creates nodes via `run_command`, it picks meaningful slugs — `research`, `fact_checker`. The agent uses slugs in file operations and names in conversation with the user.

### User-Created Nodes

When the user draws a node on the canvas, live sync persists it to DB. The backend projects it into the repo:

1. Assign slug: `unnamed_01`, `unnamed_02`, etc. (incrementing counter)
2. Create `nodes/unnamed_01.md` with the user's text as the content
3. Add to `topology.json` with no edges

```markdown
<!-- nodes/unnamed_01.md — user typed "Research competitors" in the box -->
Research competitors
```

```json
// topology.json — new node, no dependencies yet
{
  "nodes": {
    "unnamed_01": { "depends_on": [] }
  }
}
```

User-drawn edges update `topology.json` the same way — the backend adds the dependency.

The workflow agent sees user-created nodes in `<current_state>` and can work with them — update descriptions, wire edges, or reference them in conversation. The slug stays `unnamed_XX` unless the agent creates a new slug and migrates the node.

### Sync: Repo ↔ DB

**Repo → DB (agent made changes):**
1. Diff `topology.json` against existing steps → create new, remove missing, update edges
2. Diff `nodes/*.md` against step descriptions → update task text where changed
3. Auto-layout positions from dependency graph
4. Broadcast websocket updates → frontend canvas updates

**DB → Repo (user made changes on canvas):**
1. Canvas change arrives via websocket → DB updates
2. Repo files regenerated from DB state before the agent's next turn
3. Agent sees updated files in `<current_state>`

## Agent Context

### `<current_state>` (rebuilt every turn)

Same pattern as the system node agent. The backend reads the repo and builds a compact XML summary injected into the system prompt. This is the agent's ground truth — regardless of what conversation history says, `<current_state>` reflects the actual board.

The agent's context window compresses over long conversations — file contents from early turns get lost. The agent should not trust its memory of file contents. `<current_state>` provides the summary; the agent must read any file before modifying it.

```xml
<current_state description="rebuilt every turn from the live board. Always trust this over your conversation history. Read any file before modifying it — writes to unread files are rejected.">
  <topology description="slugs are file identifiers (topology.json keys, nodes/{slug}.md filenames). Names are display names set by the system node agent on Generate — absent until then. Statuses: idle (not yet generated), configuring (system node agent active — edits won't take effect until next Generate), configured (agent team ready), running (executing), completed (last run succeeded), error (failed).">
    <node slug="research" name="Market Research" depends_on="" status="configured" agents="Scanner, Crawler">
      Research competitor pricing data from public sources.
      <last_run>Found 15 competitor profiles across 4 markets. Saved to competitor_data.json.</last_run>
    </node>
    <node slug="fact_checker" depends_on="research" status="configuring">
      Verify claims against authoritative sources.
    </node>
    <node slug="unnamed_01" depends_on="fact_checker" status="idle">
      Produce summary report with verified data.
    </node>
  </topology>
</current_state>
```

**Read-before-write enforcement:** The backend rejects writes to any file the agent hasn't read this turn. This prevents the agent from writing based on stale history — it always works from the current file contents.

**Node statuses:**

| Status | Meaning | UI indicator |
|--------|---------|-------------|
| `idle` | No configuration yet. Description exists but system node agent hasn't run. | Default |
| `configuring` | System node agent is actively running. Changes to this node's description won't take effect until the next Generate. | Spinning gear |
| `configured` | System node agent has completed. Agent team is designed and ready. | Green |
| `running` | Runtime agents are executing. | Spinning gear (different color) |
| `completed` | Last run finished successfully. | Checkmark |
| `error` | Last run or configuration failed. | Red |

**Node attributes:**

| Attribute | Source | Purpose |
|-----------|--------|---------|
| `slug` | `topology.json` key | File identifier — used in file operations |
| `name` | System node agent's `config.json` | Display name — used in conversation. Absent until system node agent runs. |
| `depends_on` | `topology.json` | Which nodes feed into this one |
| `status` | DB step status | Current activity state (see table above) |
| `agents` | System node agent's roster | Comma-separated agent names (only present when configured) |

**Node children:**

| Element | Source | Purpose |
|---------|--------|---------|
| Text content | First line or heading of `nodes/{slug}.md` | Brief summary of what this node does. For long briefs, the agent reads the full file. |
| `<last_run>` | Run summarizer | What happened last time this node executed (only present if the node has run) |

When the board is empty:
```xml
<current_state description="rebuilt every turn from the live board. Always trust this over your conversation history. Read any file before modifying it — writes to unread files are rejected.">
  <topology status="empty" />
</current_state>
```

### Conversation history

A session is created on the user's first message, capturing the current board state as context. Persistent across messages from that point. One session per workflow — the user talks to the same agent with the same history for the lifetime of the workflow (unless a rebase rewinds the session).

## Tools

| Tool | Purpose |
|------|---------|
| `run_command` | Shell access to read and write repo files |
| `think` | Internal reasoning (not shown to user) |

The agent reads the repo to understand the current board, writes `topology.json` and `nodes/*.md` to make changes. Node files are markdown — they can range from a single line to a full multi-section brief. Each file write syncs to DB immediately via the filesystem watcher — the frontend sees changes on the canvas in real-time as the agent works.

System node agents are triggered separately when the user clicks "Generate," not by the workflow agent.

**System prompt:** [`config/workflow_agent/system.md`](../config/workflow_agent/system.md)

## Versioning and Rebase

A version is a complete checkpoint of everything — board topology, every node's agent configuration, and every conversation history across the workflow. When you rebase, the entire system rewinds coherently.

### What a Version Captures

A version snapshot has three layers:

```
Version Snapshot
├── board/                              # Workflow topology layer
│   ├── topology.json                   # Nodes + edges
│   └── nodes/                          # Per-node task descriptions
│       ├── research.md
│       ├── fact_checker.md
│       └── report.md
│
├── node_repos/                         # System node agent layer (per node)
│   ├── research/
│   │   ├── config.json                 # Name + output description
│   │   ├── topology.json               # Agent dependency graph
│   │   └── agents/                     # Agent configs
│   │       ├── scanner.json
│   │       └── crawler.json
│   ├── fact_checker/
│   │   └── ...
│   └── report/
│       └── ...
│
└── sessions/                           # Conversation history layer
    ├── workflow_agent/                  # Workflow agent chat history
    │   └── messages[]                  # All chat_messages for this session
    └── node_sessions/                  # Per-node system agent session histories
        ├── research_session/
        │   └── messages[]
        ├── fact_checker_session/
        │   └── messages[]
        └── report_session/
            └── messages[]
```

**Board layer** — the workflow agent's repo. `topology.json` and `nodes/*.md`. This is the topology the user sees on the canvas.

**Node repos layer** — each node's system node agent repo on JuiceFS at `{mount}/workflows/{wf_id}/system_node/{step_id}/`. These contain `config.json` (name + output description), `topology.json` (agent dependency graph), and `agents/*.json` (agent configs). This is the agent team configuration per node.

**Sessions layer** — conversation histories. The workflow agent's chat session (`chat_messages` scoped by `session_id`), plus each node's system node agent session (also `chat_messages`, identified by `draft_config->>'role' = 'system_agent'` and `draft_config->>'step_id'`). These are the memories that let agents continue coherently across dispatches via `build_pruned_instruction`.

### Storage Strategy

Content is stored in the existing `content_versions` table (SHA-256 deduplicated, immutable, append-only). A new `version_snapshots` join table links versions to their content — separate from `run_snapshots`, which is execution-scoped.

Content is packed by logical entity, not per-file. One entry per node repo (all agent configs packed together), not one entry per agent file. This keeps the join table small and restore simple — one read per entity.

**Three content types:**

| Content Type | Source ID | Content |
|-------------|-----------|---------|
| `board_state` | workflow_id | Full board: topology + all node markdown briefs |
| `node_repo` | step_id | Entire system node agent repo: config + agent topology + all agent files |
| `session_history` | session_id | Full message array for one session |

**`board_state` content:**
```json
{
  "topology": {
    "research": { "depends_on": [] },
    "fact_checker": { "depends_on": ["research"] },
    "report": { "depends_on": ["fact_checker"] }
  },
  "nodes": {
    "research": "# Research\n\nResearch competitor pricing data...\n\n## Scope\n...",
    "fact_checker": "Verify claims against authoritative sources.",
    "report": "Produce summary report with verified data."
  }
}
```

**`node_repo` content:**
```json
{
  "config": { "name": "Market Research", "description": "Structured dataset of competitor pricing..." },
  "topology": { "agents": { "scanner": { "depends_on": [] }, "crawler": { "depends_on": [] } } },
  "agents": {
    "scanner": { "name": "Scanner", "system_prompt": "...", "assignment": "...", "expected_output": "...", "capabilities": [] },
    "crawler": { "name": "Crawler", "system_prompt": "...", "assignment": "...", "expected_output": "...", "capabilities": [] }
  }
}
```

**`session_history` content:**
```json
[
  { "role": "user", "content": "...", "timestamp": "..." },
  { "role": "assistant", "content": "...", "timestamp": "..." }
]
```

Deduplication: if a node repo didn't change between Version 2 and Version 3, the content hash is identical — stored once, referenced twice. A 10-node workflow where the user changed one node creates ~3 new content rows (board state, the changed node repo, maybe a session), not 10+.

**Schema:**

```sql
CREATE TABLE workflow_versions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workflow_id UUID NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
    user_id UUID NOT NULL,
    name TEXT NOT NULL DEFAULT '',
    description TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX idx_wv_workflow ON workflow_versions(workflow_id, created_at DESC);

CREATE TABLE version_snapshots (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    version_id UUID NOT NULL REFERENCES workflow_versions(id) ON DELETE CASCADE,
    source_id UUID NOT NULL,
    content_type TEXT NOT NULL,
    content_version_id UUID NOT NULL REFERENCES content_versions(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT uq_version_snapshot UNIQUE (version_id, source_id, content_type)
);

CREATE INDEX idx_vs_version ON version_snapshots(version_id);
CREATE INDEX idx_vs_content ON version_snapshots(content_version_id);
```

**Key properties:**
- `version_snapshots` is separate from `run_snapshots` — different scopes, different cardinality, no shared columns beyond `content_version_id`
- Unique constraint on `(version_id, source_id, content_type)` — one snapshot per entity per version, no duplicates
- `ON DELETE CASCADE` from `workflow_versions` — deleting a version cleans up its snapshots
- Content versions are never deleted by cascade — they're shared across versions via dedup
- `idx_vs_version` — fast "load all snapshots for this version" (the restore path)
- `idx_vs_content` — fast "which versions reference this content" (for future pruning)

### How It Works

**Saving a version:**

1. User clicks "Save Version" (or system auto-saves before large agent changes)
2. Backend creates a `workflow_versions` row
3. Backend packs and snapshots three content types:
   - **`board_state`**: Read `topology.json` + all `nodes/*.md` → pack into one JSON object → `find_or_create_version()` → link via `version_snapshots`
   - **`node_repo`** (per node): Read each node's `config.json` + `topology.json` + `agents/*.json` from JuiceFS → pack into one JSON object per node → `find_or_create_version()` per node → link via `version_snapshots`
   - **`session_history`** (per session): Read `chat_messages` for the workflow agent session + all node sessions → serialize each → `find_or_create_version()` per session → link via `version_snapshots`
4. Dedup: `find_or_create_version()` hashes each packed object. Unchanged entities reuse existing content rows. A 10-node workflow with one changed node creates ~3 new content rows.

**Rebasing:**

1. User selects a version to rebase to
2. Backend auto-saves current state as a backup version (same as today's rebase pattern)
3. Restore process:
   - **Board layer**: Write `topology.json` and `nodes/*.md` back to the workflow agent's repo → sync to DB → websocket update → canvas reverts
   - **Node repos**: Write each node's `config.json`, `topology.json`, `agents/*.json` back to JuiceFS at `{mount}/workflows/{wf_id}/system_node/{step_id}/` → sync to DB
   - **Sessions**: Truncate `chat_messages` for each affected session to the checkpoint timestamp, then re-insert the snapshotted messages. Or simpler: clear and rewrite.
   - **DB state**: Restore steps, edges, rosters, agents from the node repo files (same as the existing `restore_workflow_from_snapshot` but driven by files instead of a monolithic JSONB blob)
4. `<current_state>` rebuilds on the workflow agent's next turn — agent sees the reverted board
5. Each node's system node agent sees its reverted `<current_state>` on next dispatch

**Agent experience after rebase:**

```
User saves Version 1: research → report
Agent adds nodes: research → fact_checker → validator → report (Version 2 auto-saved)
User rebases to Version 1
    ↓
Agent's next turn:
  - <current_state> shows: research → report (2 nodes)
  - Conversation history is from Version 1 (before the agent added nodes)
  - Agent has no memory of fact_checker or validator
  - Agent responds naturally to whatever the user says next
```

The conversation history rewind is what makes this coherent. The agent doesn't see "I added 3 nodes" in its history and then wonder where they went. Its history matches the board state.

### Relationship to Run Templates

The existing `run_templates` system captures a `WorkflowSnapshot` for reproducible execution — it's an execution-layer checkpoint. The new `workflow_versions` system captures the design-layer checkpoint — topology, node configs, and conversations.

They coexist:
- **`workflow_versions`** — design-layer: board topology + node repos + session histories. Used by the workflow agent and user for iterating on workflow design.
- **`run_templates`** — execution-layer: full DB state (steps, edges, ports, routing rules, protocols, rosters, agents, tools). Used for reproducible runs and execution-level rebase.

A user might save many design versions while iterating with the workflow agent, then save a run template when the design is finalized and ready for execution.

### No Undo, Just Save and Rebase

There is no granular undo, no timeline scrubbing, no "go back one step." The conversation and the board are interleaved — every agent message, user message, file write, and canvas edit is a point in a shared timeline. Rewinding one without the other creates incoherence.

Instead: **save explicitly, rebase explicitly.** The user saves a version when they want a checkpoint. Everything between checkpoints is just the live state. If they need to go back, they rebase to a saved version.

The user's options when something goes wrong:
- **Rebase** to a saved version (reverts everything coherently — board, node configs, conversations)
- **Ask the agent** to revert specific changes ("remove the validator node you just added")
- **Edit the canvas** manually

Canvas-level ctrl+z only undoes the user's own unsaved visual edits (dragging, resizing) that haven't synced via live sync yet.

### Known Limitation: No Granular History

A real "go back to any point" would require versioning every action across the conversation and board state atomically — git for a collaborative human-AI workspace. This is a future system. The current design is save/rebase only.

## Visibility and Cancellation

Agent actions stream into the chat window in real-time. The user sees what the agent is doing — reading files, writing nodes, wiring edges — as it happens.

This serves two purposes:

1. **Transparency.** The user sees changes appearing on the canvas in real-time as the agent works. No black box.
2. **Early cancellation.** The user can kill the agent at any point and keep whatever's on the board. Every file write has already synced to DB. No cleanup, no rollback — just stop and keep what's there. If the user wants to revert, they rebase to a saved version.

## System Node Agent Cascade

When the user clicks "Generate", the backend dispatches system node agents for each new or changed node. The cascade runs in topological order:

1. Root nodes first (no dependencies) — can run in parallel
2. Each system node agent writes `config.json` with name + output description
3. Output description becomes `<previous_step>` for downstream nodes
4. Downstream system node agents fire sequentially as upstream completes
5. If a system node agent's description doesn't change from last run, cascade stops

During the cascade, the user can keep talking to the workflow agent and editing the canvas. System node agents operate on individual nodes — they don't conflict with topology-level changes.

## What Changes vs System Node Agent

| Aspect | System Node Agent | Workflow Agent |
|--------|-------------------|----------------|
| **Scope** | Agents within one node | Nodes + edges across the workflow |
| **Trigger** | "Generate" / upstream cascade | User message in conversation |
| **Input** | Node task text + upstream context | Board state + user message + history |
| **Output** | config.json, topology.json, agents/*.json | topology.json, nodes/*.md |
| **Consumer** | file_executor → runtime agents | Backend sync → DB → frontend canvas |
| **Session** | Per-step, across dispatches | Per-workflow, persistent conversation |
| **Board state source** | Own filesystem (stable) | Live-synced repo (reflects current canvas) |
| **Lifecycle** | Fires on generate, completes, exits | Persistent — lives across the conversation |

## What Stays The Same

- Container + JuiceFS workspace
- `run_command` for file writes
- Write-time validation (JSON schema for `topology.json`, existence check for `nodes/*.md`)
- `ExecutionStrategy` trait
- `compute_execution_levels` for auto-layout and execution ordering
- System node agent cascade on node description changes
- Websocket updates for frontend sync

## Open Questions

- **Partial builds**: Can the agent make incremental changes (add one node) or does it always write the full topology? Incremental is more natural for conversation; full rewrite is simpler for sync.
- **Generate timing**: Should the agent be able to suggest "you should Generate now" in its response, or just let the user decide?
- **Auto-save versions**: Should the system auto-save a version before the agent makes changes, or only when the user explicitly saves? Auto-save is safer but creates version noise. Content dedup keeps storage cheap either way.
- **Session restore strategy**: On rebase, do we truncate + rewrite `chat_messages`, or create a new session with the snapshotted messages and retire the old one? New session is cleaner but breaks session_id references.
- **Version pruning**: Content dedup keeps storage efficient, but `workflow_versions` rows accumulate. Auto-prune after N versions, or keep all?
- **Manager relationship**: Does this replace the current L1 manager assistant + L2 manager builder? Or coexist as a different mode?
