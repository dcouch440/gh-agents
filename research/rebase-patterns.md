# Rebase/Revert UX and State Rollback Patterns

Research into how collaborative AI-human tools handle version rebase/revert UX and the technical patterns behind coherent state rollback.

**Context**: A workflow design tool where a user draws nodes on a canvas, an AI agent edits the same workspace via conversation, both have persistent chat sessions with history, and the user can save checkpoints and rebase (revert) to any checkpoint.

---

## 1. Conversation Coherence on Revert

The central question: when you revert workspace state, what happens to the chat history that references entities that no longer exist?

### Approaches in Production Systems

#### A. Truncate Messages to Checkpoint Timestamp

**Replit** uses this approach. When rolling back to a checkpoint, "Agent conversations are restored to the point of the checkpoint, maintaining context continuity." The conversation is literally rewound---messages after the checkpoint are removed from the active view. The technical mechanism: each checkpoint stores a reference to the conversation state (message count or cursor), and restore replays the conversation log up to that marker.

**Cursor** takes a similar approach. Restoring a checkpoint resets "all files to that point in the conversation." The user can then "write a new message to the AI agent" to continue from the reverted state. Checkpoints are ephemeral (session-scoped, stored in a hidden local directory), and the conversation effectively forks from the restore point.

**Pros**: Clean mental model. The conversation matches the workspace. No confusion about phantom references.
**Cons**: Destructive---user loses potentially valuable conversation context (debugging insights, decisions, reasoning).

#### B. Fork a New Session from the Checkpoint

**ChatGPT** uses conversation branching. When a user edits an earlier message, the system "quietly triggers a branch." The original conversation path is preserved, and a new branch diverges from the edit point. Users navigate between branches via a toggle selector. Each branch inherits conversation state at the split point.

This maps well to rebase: reverting to a checkpoint creates a new conversation branch rooted at that checkpoint. The old "future" conversation still exists but is on a dead branch.

**Pros**: Non-destructive. Full history preserved. User can reference old branches.
**Cons**: UI complexity. Users must understand branching. Navigation between branches can be confusing.

#### C. Inject a "Rebased to Version X" System Marker

No major production tool uses this as the primary approach, but it appears as a secondary pattern. In collaborative editing (Google Docs), version restores create a new version entry rather than truncating history---the document shows the restored content as a new revision with a system note.

This approach keeps the full conversation but inserts a synthetic message like:

```
[System] Workspace rebased to checkpoint "v3 - before agent redesign"
The following entities were removed: Node-7, Node-8, Node-9
Agent context has been updated to reflect the current workspace state.
```

**Pros**: No information loss. Clear audit trail. Simple implementation.
**Cons**: Conversation becomes incoherent---the AI's earlier messages still reference nodes that no longer exist. Risk of confusing both the user and the AI (if the AI reads its own history and hallucinates about removed entities).

### Recommendation

**Use approach B (fork) as the primary model, with A (truncate) as the user-facing simplification.**

Internally, store the full conversation tree (every branch). When the user reverts, create a new branch from the checkpoint and make it the active view. The old messages still exist in storage for audit/debugging but are hidden from the active chat. The AI's context window should only include messages from the active branch.

Implementation sketch:

```sql
-- Messages belong to branches, branches form a tree
CREATE TABLE chat_branches (
    id UUID PRIMARY KEY,
    session_id UUID NOT NULL REFERENCES chat_sessions(id),
    parent_branch_id UUID REFERENCES chat_branches(id),
    fork_point_message_id UUID REFERENCES chat_messages(id),
    checkpoint_id UUID REFERENCES checkpoints(id),  -- null for initial branch
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE chat_messages (
    id UUID PRIMARY KEY,
    branch_id UUID NOT NULL REFERENCES chat_branches(id),
    sequence_num INT NOT NULL,
    role TEXT NOT NULL,  -- 'user', 'assistant', 'system'
    content JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (branch_id, sequence_num)
);
```

When rebasing: create a new `chat_branch` with `fork_point_message_id` pointing to the last message at or before the checkpoint timestamp. Inject a system message on the new branch: "Workspace reverted to checkpoint X." The AI prompt builder queries only messages on the active branch lineage.

---

## 2. Multi-Entity Atomic Revert

When a version spans multiple entity types (topology, node configs, agent definitions, conversations), restoring atomically requires careful orchestration.

### The Entity Dependency Graph

In the Nexor context, a checkpoint must capture:

| Entity | Dependencies | Restore Order |
|--------|-------------|---------------|
| Workflow topology (nodes, edges) | None (root) | 1st |
| Node configurations | References node IDs | 2nd |
| Agent definitions | References node IDs | 2nd |
| Step routing rules | References node + edge IDs | 3rd |
| Conversation branches | References agent + node IDs | 4th |
| Execution history | References all above | Skip (read-only audit) |

### Transaction Pattern

PostgreSQL's transactional guarantees make this straightforward at the database level. The key insight: **revert is a write operation, not a time-travel query.**

```sql
BEGIN;
-- 1. Snapshot the current state (for undo-the-undo)
INSERT INTO checkpoints (workflow_id, type, label)
VALUES ($1, 'auto_pre_revert', 'Auto-save before revert to ' || $target_label);

-- 2. Delete entities created after checkpoint (reverse dependency order)
DELETE FROM step_routing_rules WHERE workflow_id = $1 AND created_at > $checkpoint_ts;
DELETE FROM workflow_step_configs WHERE step_id IN (
    SELECT id FROM workflow_steps WHERE workflow_id = $1 AND created_at > $checkpoint_ts
);
DELETE FROM workflow_steps WHERE workflow_id = $1 AND created_at > $checkpoint_ts;
DELETE FROM workflow_edges WHERE workflow_id = $1 AND created_at > $checkpoint_ts;

-- 3. Restore modified entities from checkpoint snapshot
-- (overwrite current rows with snapshot values)
UPDATE workflow_steps SET ... FROM checkpoint_step_snapshots WHERE ...;
UPDATE workflow_edges SET ... FROM checkpoint_edge_snapshots WHERE ...;

-- 4. Fork conversation branch
INSERT INTO chat_branches (session_id, parent_branch_id, fork_point_message_id, checkpoint_id)
VALUES (...);

COMMIT;
```

### Snapshot Storage Strategy

Two approaches:

**A. Full snapshot (Memento pattern)**: Store a complete JSONB blob of all entities at each checkpoint. Simple to restore---just overwrite. Expensive on storage for large workflows.

**B. Event log with replay (Event Sourcing)**: Store individual change events. To restore, replay events up to the checkpoint version. More storage-efficient, but restore is slower and more complex.

**Recommended: Hybrid.** Store lightweight diffs for auto-checkpoints (events), full snapshots for explicit user saves. This mirrors Photoshop's approach: history states are incremental, snapshots are full copies.

```sql
CREATE TABLE checkpoint_snapshots (
    id UUID PRIMARY KEY,
    checkpoint_id UUID NOT NULL REFERENCES checkpoints(id),
    entity_type TEXT NOT NULL,        -- 'steps', 'edges', 'configs', etc.
    entity_id UUID NOT NULL,
    snapshot_data JSONB NOT NULL,      -- full entity state at checkpoint time
    UNIQUE (checkpoint_id, entity_type, entity_id)
);
```

### Partial Failure Handling

PostgreSQL transactions handle this naturally---if any step fails, the entire transaction rolls back. The critical rule: **never do external side effects inside the revert transaction.** If revert triggers webhooks, agent notifications, or file system changes, those must happen after the transaction commits, with compensating actions if they fail.

Pattern:
1. Execute revert in a single DB transaction
2. On commit, emit events to a transactional outbox table
3. A background worker processes outbox events (notify agents, update caches, etc.)
4. If a side effect fails, the DB state is already correct; the side effect retries

---

## 3. Cascading Revert and Orphaned References

### The Problem

After checkpoint C3, the AI agent:
- Created Node-7 (with agent_id A-12)
- Created Node-8 (referencing Node-7 as upstream)
- Modified Node-3's config to reference Node-7's output

When reverting to C3:
- Node-7 and Node-8 must be deleted
- Node-3's config must be restored to its C3 state
- Agent A-12 becomes orphaned (no node references it)
- Any routing rules referencing Node-7/8 must be removed
- The conversation referencing these nodes becomes stale

### Strategies

#### A. Snapshot-Based Restore (Recommended)

Rather than trying to compute what to delete/modify, simply **overwrite the entire entity set** from the checkpoint snapshot. This avoids the cascading problem entirely.

```
restore(checkpoint):
    delete ALL steps for workflow
    delete ALL edges for workflow
    insert steps FROM checkpoint_snapshots WHERE checkpoint_id = target
    insert edges FROM checkpoint_snapshots WHERE checkpoint_id = target
    -- etc.
```

This is the approach used by Replit (Git reset + Neon branch promotion) and game engines (full state restore from snapshot). It trades efficiency for correctness---you never have partial state.

**Orphaned agent definitions**: If agents are workflow-scoped, the snapshot restore handles them. If agents are shared across workflows, add a reference-counting cleanup pass after revert (or use soft-delete with periodic garbage collection).

#### B. Dependency-Tracked Delete (Alternative)

If full snapshot restore is too expensive, track dependencies explicitly:

```sql
-- Track which checkpoint created each entity
ALTER TABLE workflow_steps ADD COLUMN created_in_checkpoint UUID REFERENCES checkpoints(id);
ALTER TABLE workflow_edges ADD COLUMN created_in_checkpoint UUID REFERENCES checkpoints(id);

-- Revert: delete everything created after the target checkpoint
DELETE FROM workflow_steps
WHERE workflow_id = $1
  AND created_in_checkpoint IN (
      SELECT id FROM checkpoints
      WHERE workflow_id = $1 AND created_at > $target_checkpoint_created_at
  );
```

This is simpler but misses **modifications** to pre-existing entities. You still need snapshots for entities that existed at the checkpoint but were later modified.

#### C. Soft Delete with Version Tagging

Never physically delete entities. Instead, every entity carries a `valid_from_version` and `valid_to_version`:

```sql
ALTER TABLE workflow_steps ADD COLUMN valid_from_version INT NOT NULL;
ALTER TABLE workflow_steps ADD COLUMN valid_to_version INT;  -- NULL = current

-- Query for a specific version
SELECT * FROM workflow_steps
WHERE workflow_id = $1
  AND valid_from_version <= $target_version
  AND (valid_to_version IS NULL OR valid_to_version > $target_version);
```

This is the temporal tables approach. It avoids deletes entirely, making revert a simple version pointer change. The tradeoff: every query must include version filtering, and storage grows with every change.

### Recommendation

**Use snapshot-based restore (A) for explicit checkpoints, with temporal versioning (C) as the underlying storage model.** This gives you:
- Fast revert (change the "current version" pointer)
- Point-in-time queries (for preview before revert)
- No cascading delete complexity
- Full audit trail

---

## 4. Auto-Save vs. Explicit Save

### What Production Tools Do

| Tool | Strategy | Details |
|------|----------|---------|
| **Replit** | Auto + explicit | Auto-checkpoint at every "major request" completion. User can also name/tag checkpoints. |
| **Cursor** | Auto only | Checkpoint before every Agent code edit. Ephemeral, session-scoped. No explicit save. |
| **Figma** | Auto + explicit | Auto-save continuously. User can manually "Add to Version History" with a name/description. Branches auto-checkpoint on create and merge. |
| **Google Docs** | Auto + explicit | Continuous auto-save. User can "Name this version." Version history shows auto-saves grouped by time. |
| **Photoshop** | Auto + explicit | Every tool action creates a history state (auto). User can create named Snapshots. States cap at configurable limit (default 20, max 1000). |
| **VS Code** | Auto | Local History creates automatic entries on save/edit. No explicit checkpoint UI. |
| **Git** | Explicit only | User explicitly commits. No auto-save of working directory state. |

### The Tradeoffs

**Auto-only (Cursor model)**:
- Pro: Zero user friction. Safety net always present.
- Con: Version noise. Hard to find meaningful restore points. "Which of these 47 checkpoints is the one I want?"
- Con: Storage costs scale with activity, not intent.

**Explicit-only (Git model)**:
- Pro: Clean history. Every version is intentional and meaningful.
- Con: Users forget to save. Destructive operations without a safety net cause data loss.
- Con: Requires user discipline.

**Hybrid (Figma/Replit model)**:
- Pro: Auto-save provides safety net. Explicit saves provide meaningful landmarks.
- Con: Two mental models. Must clearly distinguish auto vs. explicit in UI.

### Recommendation for Nexor

**Hybrid with two tiers:**

1. **Auto-checkpoints** (invisible to user, capped at ~50): Created before every AI agent operation and every destructive user action (delete node, bulk edit, rebase). These are the safety net. Stored with type `auto` and garbage-collected after 7 days or when the cap is reached (FIFO).

2. **Explicit checkpoints** (visible, unlimited): Created when the user clicks "Save Version" or presses a hotkey. Named, described, permanent. These are the landmarks the user sees in the version history UI.

The UI shows only explicit checkpoints by default, with a "Show all versions" toggle that reveals auto-checkpoints.

```sql
CREATE TABLE checkpoints (
    id UUID PRIMARY KEY,
    workflow_id UUID NOT NULL REFERENCES workflows(id),
    version_num INT NOT NULL,  -- monotonic per workflow
    checkpoint_type TEXT NOT NULL CHECK (checkpoint_type IN ('auto', 'explicit')),
    label TEXT,  -- user-provided for explicit, auto-generated for auto
    trigger TEXT,  -- 'user_save', 'pre_generate', 'pre_revert', 'pre_delete', 'agent_complete'
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (workflow_id, version_num)
);
```

Auto-checkpoint triggers:
- `pre_generate`: Before AI agent starts designing
- `pre_revert`: Before any rebase operation (undo-the-undo safety)
- `pre_delete`: Before bulk delete operations
- `agent_complete`: After AI agent finishes a design pass

---

## 5. Version Branching: Linear History vs. Tree

### How Tools Handle Post-Revert Edits

After rebasing to version V5 and making new changes, what happens to V6, V7, V8 (the old "future")?

#### A. Linear with Discard (Cursor, Photoshop default)

Old future versions are discarded (or dimmed and inaccessible). The history is always a single line. Revert + edit = the old future is gone.

Photoshop's default mode: "Selecting a state and then changing the image eliminates all states that come after."

**Tradeoff**: Simple mental model, but information loss. Users may want to return to V8 after deciding the revert was wrong.

#### B. Linear with "Non-Linear Mode" (Photoshop option)

Photoshop's "Allow Non-Linear History" option: reverting to V5 and making edits adds V9 at the end of the list. V6, V7, V8 remain in the list and are still accessible. The list is linear (flat), but the logical relationships form a tree.

**Tradeoff**: All states accessible, but the flat list becomes confusing. Users see V5, V6, V7, V8, V9 but V9 is based on V5, not V8. The ordering implies a sequence that doesn't exist.

#### C. Tree/DAG (Helix Editor, Git, ChatGPT)

The **Helix editor** implements a full revision tree. Each revision has a `parent` and `last_child`. Undo follows the parent chain, redo follows the last_child chain. When you undo to V5 and make a new edit, V9 becomes V5's new `last_child`. V6 (the old child) is still in the tree and reachable via time-based navigation or explicit tree traversal.

Key data structure:
```rust
struct Revision {
    parent: usize,          // index of parent revision
    last_child: usize,      // most recent child (redo target)
    transaction: Transaction, // forward changes
    inversion: Transaction,   // reverse changes (for undo)
    timestamp: Instant,
}
```

Navigation between arbitrary revisions uses **Lowest Common Ancestor (LCA)**: find the shared ancestor, aggregate inversions going "up," then aggregate forward transactions going "down."

**Git** uses a DAG (not a tree, because merges create nodes with multiple parents). Branches are named pointers into the DAG. Revert creates a new commit that undoes changes; reset moves the branch pointer backward.

**ChatGPT** uses a tree. Editing a message creates a new branch. The original and new branches coexist, navigable via a toggle.

#### D. Linear with Backup Branch (Figma)

Figma creates a checkpoint before merge/revert operations. If you restore version V5, Figma first snapshots the current state. The history remains linear, but the "pre-restore" snapshot acts as a named recovery point. Old futures are implicitly available through this backup.

### Recommendation for Nexor

**Use a tree model internally, present a linear view by default.**

Store versions as a tree (each version has a `parent_version_id`). The "active branch" is a pointer to the current leaf. When the user reverts, they fork a new branch. The old branch's versions are retained but hidden from the default view.

```sql
CREATE TABLE checkpoints (
    id UUID PRIMARY KEY,
    workflow_id UUID NOT NULL,
    parent_checkpoint_id UUID REFERENCES checkpoints(id),  -- tree structure
    branch_id UUID NOT NULL REFERENCES version_branches(id),
    version_num INT NOT NULL,
    -- ...
);

CREATE TABLE version_branches (
    id UUID PRIMARY KEY,
    workflow_id UUID NOT NULL,
    parent_branch_id UUID REFERENCES version_branches(id),
    fork_point_checkpoint_id UUID REFERENCES checkpoints(id),
    is_active BOOLEAN NOT NULL DEFAULT true,
    label TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

Default UI: shows a linear list of checkpoints on the active branch. "Show branches" reveals the tree. This mirrors Git's default UX (linear log, branch visualization on demand).

---

## 6. Real-World System Architectures

### Google Docs: Operation Log + Periodic Snapshots

- **Storage**: Bigtable column-family model. Each document has an operation log (append-only) and periodic state snapshots.
- **Version history**: Rebuilds document state by replaying operations on the last snapshot before the target timestamp.
- **Revert**: Creates a *new* version with the old content. History is never truncated. The restored state appears as the latest version. This is an "undo via new commit" model---identical to `git revert` semantics.
- **Collaboration**: Uses Operational Transformation (OT). Undo creates a new operation that reverses the original. This is critical for multi-user consistency: simply removing the original operation would break transformations applied to subsequent operations.
- **Lesson for Nexor**: Undo-as-new-operation is safer than history truncation in collaborative systems. Even with a single AI agent as the "collaborator," creating a new version (rather than deleting old ones) preserves the ability to undo-the-undo.

### Photoshop: Bounded History States + Named Snapshots

- **Architecture**: Linear list of state diffs, capped at a configurable limit (default 20, max 1000). Each state is a diff from the previous state.
- **Snapshots**: Full-state captures that persist for the session. Snapshots sit at the top of the History panel, immune to the state cap. Users create them explicitly as "save points" before risky operations.
- **Non-linear mode**: Allows reverting to an earlier state without discarding later states. New edits append to the end. The list becomes non-chronological but nothing is lost.
- **Lesson for Nexor**: The two-tier model (auto states + explicit snapshots) is battle-tested. The key insight: snapshots are full copies (expensive but reliable), states are diffs (cheap but require sequential replay). Nexor should use the same split: auto-checkpoints store diffs, explicit saves store full snapshots.

### VS Code: File-Level Timeline with Multiple Sources

- **Architecture**: The Timeline view aggregates entries from multiple providers---Git commits, local history saves, and extension-contributed entries.
- **Local History**: Auto-saves file state on every save. Stored as full file copies in `.history/` within the workspace. Users can diff any two points or restore from any entry.
- **Persistence**: Undo/redo stack survives file close/reopen (if file content hasn't changed externally). This is critical for long-lived editing sessions.
- **Lesson for Nexor**: Aggregating multiple history sources (user actions, agent actions, system events) into a single timeline view is powerful. Each source contributes entries with timestamps; the UI merges them chronologically.

### Game Engines: Deterministic Snapshot/Restore

- **Rollback Netcode (GGPO)**: Games store complete state snapshots at every frame. On input mismatch, the engine loads the snapshot, replays corrected inputs forward. This requires deterministic simulation---same inputs always produce same outputs.
- **State Requirements**: "The game must be able to snapshot and restore a previous game state." The ideal implementation stores state "contiguously in memory...within one big C struct" that can be copied with `memcpy`.
- **Incremental Rollback**: Some engines snapshot only changes (dirty flags) rather than full state, dramatically reducing memory use.
- **Level Design Undo**: Uses Command Pattern (execute/undo methods) with Transaction Grouping (related operations bundled into atomic units). Dependencies between objects require careful ordering---"handling dependencies between objects" and "reverting the entire procedural generation process to its previous state" when a seed changes.
- **Lesson for Nexor**: Transaction grouping is essential. An AI agent's "design pass" produces many individual changes (add node, add edge, configure node, etc.), but these should be grouped into a single undoable unit. The checkpoint should wrap the entire agent operation, not individual mutations.

### Figma: Git-Inspired Branching for Design

- **Version History**: Continuous auto-save. Named versions on demand. Restoring a version creates a new checkpoint first (safety net).
- **Branching**: Branches are full copies of the file. Changes in a branch are isolated. Merging creates checkpoints on both the branch and the main file. Conflict resolution is visual---side-by-side comparison of conflicting frames/components.
- **Lesson for Nexor**: Figma's "checkpoint before merge/restore" pattern is worth adopting. Always auto-checkpoint before revert. This makes revert non-destructive even with a linear history model.

### Replit: Unified Snapshot Across Code + Database + Conversation

- **Architecture**: Each checkpoint bundles three things: a Git commit (code state), a Neon database branch (data state), and agent conversation context.
- **Database Branching**: Uses Neon's copy-on-write branching. Creating a checkpoint requests a new Neon branch at the checkpoint timestamp---"no full data copy required." Rollback promotes the branch to replace the current database.
- **Preview Before Restore**: Users can run prior versions without affecting current state. The system provisions a temporary compute endpoint for the branch.
- **Conversation Context**: Restored to the checkpoint point, maintaining continuity. Messages after the checkpoint are not shown in the restored session.
- **Lesson for Nexor**: The Replit model is the closest analogue to Nexor's use case. The key architectural decisions: (1) checkpoint = atomic bundle of all state layers, (2) database branching via copy-on-write for efficient snapshots, (3) conversation truncated to checkpoint, not preserved in full.

---

## 7. Practical Recommendations for Nexor (Rust/PostgreSQL)

### Architecture Summary

```
                    ┌─────────────────────────────────────────┐
                    │              Checkpoint                  │
                    │                                          │
                    │  ┌──────────┐ ┌──────────┐ ┌──────────┐│
                    │  │ Topology │ │  Configs  │ │   Chat   ││
                    │  │ Snapshot │ │ Snapshot  │ │  Branch  ││
                    │  └──────────┘ └──────────┘ └──────────┘│
                    │                                          │
                    │  version_num: 7                          │
                    │  parent: checkpoint_6                    │
                    │  branch: main                            │
                    │  type: explicit                          │
                    │  trigger: user_save                      │
                    └─────────────────────────────────────────┘
```

### Schema Design

```sql
-- Version branches form a tree
CREATE TABLE version_branches (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workflow_id UUID NOT NULL REFERENCES workflows(id),
    parent_branch_id UUID REFERENCES version_branches(id),
    fork_point_checkpoint_id UUID REFERENCES checkpoints(id),
    is_active BOOLEAN NOT NULL DEFAULT true,
    label TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Checkpoints are ordered within a branch
CREATE TABLE checkpoints (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workflow_id UUID NOT NULL REFERENCES workflows(id),
    branch_id UUID NOT NULL REFERENCES version_branches(id),
    parent_checkpoint_id UUID REFERENCES checkpoints(id),
    version_num INT NOT NULL,
    checkpoint_type TEXT NOT NULL CHECK (checkpoint_type IN ('auto', 'explicit')),
    trigger TEXT NOT NULL,  -- 'user_save', 'pre_generate', 'pre_revert', 'agent_complete'
    label TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (workflow_id, version_num)
);

-- Full entity snapshots per checkpoint
CREATE TABLE checkpoint_entity_snapshots (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    checkpoint_id UUID NOT NULL REFERENCES checkpoints(id),
    entity_type TEXT NOT NULL,  -- 'workflow_step', 'workflow_edge', 'step_config', etc.
    entity_id UUID NOT NULL,
    snapshot_data JSONB NOT NULL,
    UNIQUE (checkpoint_id, entity_type, entity_id)
);

-- Chat branches mirror version branches
CREATE TABLE chat_branches (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id UUID NOT NULL REFERENCES chat_sessions(id),
    parent_branch_id UUID REFERENCES chat_branches(id),
    fork_point_message_id UUID REFERENCES chat_messages(id),
    checkpoint_id UUID REFERENCES checkpoints(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE chat_messages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    branch_id UUID NOT NULL REFERENCES chat_branches(id),
    sequence_num INT NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('user', 'assistant', 'system')),
    content JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (branch_id, sequence_num)
);
```

### Revert Algorithm (Rust pseudocode)

```rust
async fn revert_to_checkpoint(
    pool: &PgPool,
    workflow_id: Uuid,
    target_checkpoint_id: Uuid,
) -> Result<RevertResult> {
    let mut tx = pool.begin().await?;

    // 1. Load target checkpoint and its snapshots
    let target = load_checkpoint(&mut tx, target_checkpoint_id).await?;
    let snapshots = load_snapshots(&mut tx, target_checkpoint_id).await?;

    // 2. Auto-save current state (undo-the-undo safety net)
    let pre_revert_checkpoint = create_checkpoint(
        &mut tx, workflow_id, CheckpointType::Auto, "pre_revert",
    ).await?;
    snapshot_current_state(&mut tx, workflow_id, pre_revert_checkpoint.id).await?;

    // 3. Fork version branch
    let current_branch = get_active_branch(&mut tx, workflow_id).await?;
    let new_branch = create_branch(
        &mut tx, workflow_id, current_branch.id, target_checkpoint_id,
    ).await?;
    set_active_branch(&mut tx, workflow_id, new_branch.id).await?;

    // 4. Restore entity state (atomic: delete all, insert from snapshot)
    delete_all_workflow_entities(&mut tx, workflow_id).await?;
    for snapshot in &snapshots {
        restore_entity_from_snapshot(&mut tx, snapshot).await?;
    }

    // 5. Fork conversation branch
    let chat_session = get_chat_session(&mut tx, workflow_id).await?;
    let last_msg_at_checkpoint = get_last_message_before(
        &mut tx, chat_session.id, target.created_at,
    ).await?;
    let new_chat_branch = fork_chat_branch(
        &mut tx, chat_session.id, last_msg_at_checkpoint.id, target_checkpoint_id,
    ).await?;

    // 6. Inject system message on new branch
    insert_system_message(
        &mut tx, new_chat_branch.id,
        format!("Workspace reverted to checkpoint: {}", target.label.unwrap_or_default()),
    ).await?;

    tx.commit().await?;

    // 7. Post-commit side effects (non-transactional)
    notify_active_agents(workflow_id, RevertEvent { target_checkpoint_id }).await;

    Ok(RevertResult { new_branch, pre_revert_checkpoint })
}
```

### Key Design Decisions

1. **Checkpoint = atomic bundle**: Each checkpoint captures topology + configs + chat cursor. They are restored together or not at all.

2. **Snapshot-based restore**: Full entity snapshots for explicit checkpoints. Avoids cascading delete complexity entirely---delete everything, insert from snapshot.

3. **Tree internally, linear externally**: Version history is a tree (branches on revert). Default UI shows the active branch as a linear list. Power users can toggle branch view.

4. **Chat fork, not truncate**: Conversation branches on revert. Old messages preserved but hidden from active view. AI context window built from active branch only.

5. **Auto-checkpoint before destructive ops**: Always create a safety checkpoint before Generate, Revert, and bulk Delete. Cap auto-checkpoints per workflow (e.g., 50). Garbage-collect oldest when cap is reached.

6. **Explicit checkpoints are permanent**: User-created saves never auto-expire. They carry names and descriptions. These are the anchors in the version timeline.

7. **Post-revert agent notification**: After revert commits, notify the AI agent that workspace state has changed. The agent's next response should acknowledge the current state, not reference removed entities. Include a system prompt injection: "The workspace was reverted. Current entities: [list]. Do not reference entities from prior versions."

---

## Sources

- [Replit Checkpoints and Rollbacks](https://docs.replit.com/replitai/checkpoints-and-rollbacks)
- [Inside Replit's Snapshot Engine](https://blog.replit.com/inside-replits-snapshot-engine)
- [Replit App History Powered by Neon Branches](https://neon.com/blog/replit-app-history-powered-by-neon-branches)
- [Refact.ai Agent Rollback](https://docs.refact.ai/features/autonomous-agent/rollback/)
- [Cursor Checkpoints](https://stevekinney.com/courses/ai-development/cursor-checkpoints)
- [ChatGPT Conversation Branching](https://knowledge.buka.sh/the-hidden-fork-how-editing-messages-in-chatgpt-lets-you-branch-conversations/)
- [ChatGPT Branch Conversations](https://scalevise.com/resources/chatgpt-branch-conversations/)
- [Helix Editor History and Undo System](https://deepwiki.com/helix-editor/helix/2.6-history-and-undo-system)
- [Undo/Redo in Level Design (Wayline)](https://www.wayline.io/blog/undo-redo-level-design)
- [Rollback Netcode Architecture (SnapNet)](https://www.snapnet.dev/blog/netcode-architectures-part-2-rollback/)
- [Memento Design Pattern (Refactoring Guru)](https://refactoring.guru/design-patterns/memento)
- [Figma Guide to Branching](https://help.figma.com/hc/en-us/articles/360063144053-Guide-to-branching)
- [Figma Version History](https://help.figma.com/hc/en-us/articles/360038006754-View-a-file-s-version-history)
- [Google Docs System Design (AlgoMaster)](https://blog.algomaster.io/p/google-docs-system-design-interview)
- [Photoshop History Panel](https://helpx.adobe.com/photoshop/desktop/get-started/set-up-toolbars-panels/history-panel-overview.html)
- [Photoshop Snapshots](https://helpx.adobe.com/photoshop/desktop/get-started/set-up-toolbars-panels/create-work-snapshots.html)
- [VS Code Timeline](https://rsw.io/how-to-use-the-vs-code-timeline-to-recover-a-file-navigating-changes-between-commits/)
- [Autosave vs Explicit Save UX (Damian Wajer)](https://www.damianwajer.com/blog/autosave/)
- [Event Sourcing Pattern (Microsoft)](https://learn.microsoft.com/en-us/azure/architecture/patterns/event-sourcing)
- [Event Sourcing in Relational DB (SoftwareMill)](https://softwaremill.com/implementing-event-sourcing-using-a-relational-database/)
- [Multi-Temporal Versioning in Postgres (HASH)](https://hash.dev/blog/multi-temporal-versioning)
- [PostgreSQL Transactions](https://www.postgresql.org/docs/current/tutorial-transactions.html)
- [Cascading Updates and Deletes in SQL](https://softwarepatternslexicon.com/sql/data-integrity-and-validation-patterns/cascading-updates-and-deletes/)
- [Version Control (Wikipedia)](https://en.wikipedia.org/wiki/Version_control)
- [AI Agent Versioning and Rollback (Medium)](https://medium.com/@nraman.n6/versioning-rollback-lifecycle-management-of-ai-agents-treating-intelligence-as-deployable-deac757e4dea)
- [Windsurf Cascade Agent Refactoring](https://markaicode.com/windsurf-cascade-agent-autonomous-refactoring/)
