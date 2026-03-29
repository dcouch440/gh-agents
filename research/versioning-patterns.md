# Content-Addressable Versioning for Collaborative AI-Human Workspaces

Research compiled March 2026. Focused on patterns implementable in Rust/Axum/PostgreSQL.

---

## 1. Content-Addressable Storage Patterns in PostgreSQL

### The Git Object Model, Adapted to Relational Tables

Git's object store is a two-column table: object ID (SHA hash of content) and object content. Every blob, tree, and commit is indexed by the hash of its content. This maps cleanly to PostgreSQL.

**Core table: content blobs**

```sql
CREATE TABLE content_blobs (
    content_hash BYTEA PRIMARY KEY,  -- SHA-256, 32 bytes
    content_type TEXT NOT NULL,       -- 'topology', 'brief', 'agent_config', 'conversation'
    content      JSONB NOT NULL,
    byte_size    INTEGER NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

The key insight from Git: **separate identity from content**. The hash IS the identity. Two nodes with identical briefs share one `content_blobs` row. When a brief changes, a new row is created with a new hash; the old row persists until no version references it.

**INSERT ON CONFLICT for deduplication:**

```sql
INSERT INTO content_blobs (content_hash, content_type, content, byte_size)
VALUES ($1, $2, $3, $4)
ON CONFLICT (content_hash) DO NOTHING;
```

This is the foundational pattern from Richard Towers' CAS-in-Postgres approach. Workers hash their content, insert with `ON CONFLICT DO NOTHING`, and reference the hash as a foreign key. Concurrent writers producing identical content naturally deduplicate. The insert is idempotent.

**Blob-per-entity vs packed-per-entity:**

Two approaches for storing versioned entities:

| Approach | Description | Tradeoff |
|----------|-------------|----------|
| Blob-per-field | Each entity field (brief, config, etc.) is a separate content blob | Maximum deduplication. If only the brief changes, config blob is reused. More joins on restore. |
| Blob-per-entity | Entire entity state serialized as one blob | Simpler restore (one lookup per entity). Less deduplication when single fields change. |
| Blob-per-workspace | Entire workspace as one giant blob | Simplest restore. Zero deduplication. Impractical for >10 nodes. |

**Recommendation for 10-50 node workflows: blob-per-field.** With ~4 fields per node (topology position, brief, agent config, conversation state), a 50-node workflow has ~200 content blobs per version. When 1-3 nodes change, only 4-12 new blobs are created; the remaining ~188 are pointer reuse. This yields roughly 94-97% deduplication per incremental save.

### Indexing Strategies for Content Hash Lookups

**BYTEA primary key (32 bytes for SHA-256):**

PostgreSQL supports B-tree indexes on `BYTEA` natively. For a content-addressable store, the primary key IS the hash, so no separate index is needed for lookups by hash.

**Hash index vs B-tree for content_hash:**

| Property | Hash Index | B-Tree Index |
|----------|-----------|--------------|
| Equality lookup | ~5.9 us/query | ~9.2 us/query |
| Storage size | ~32 MB (1M rows) | ~56 MB (1M rows) |
| UNIQUE constraint | Not supported | Supported |
| WAL support | PostgreSQL 10+ | Always |
| Range queries | No | Yes |

Since the primary key creates a B-tree by default and we need the UNIQUE constraint, **stick with B-tree on the primary key**. The performance difference is negligible at the scale of a workspace versioning system (thousands of blobs, not millions). Hash indexes only matter at extreme scale.

**Hex string (CHAR(64)) vs BYTEA:**

Store as `BYTEA` (32 bytes), not hex string (64 bytes). Half the storage, faster comparison. Convert to hex only for display/API responses. In sqlx, `Vec<u8>` maps directly to `BYTEA`.

### Rust Implementation

```rust
use sha2::{Sha256, Digest};

fn content_hash(content: &serde_json::Value) -> Vec<u8> {
    // Canonical serialization: compact JSON, sorted keys
    let canonical = serde_json::to_string(content).unwrap();
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    hasher.finalize().to_vec()  // 32 bytes
}

async fn store_blob(
    pool: &PgPool,
    content: &serde_json::Value,
    content_type: &str,
) -> Result<Vec<u8>, sqlx::Error> {
    let hash = content_hash(content);
    let byte_size = serde_json::to_string(content).unwrap().len() as i32;

    sqlx::query(
        "INSERT INTO content_blobs (content_hash, content_type, content, byte_size)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (content_hash) DO NOTHING"
    )
    .bind(&hash)
    .bind(content_type)
    .bind(content)
    .bind(byte_size)
    .execute(pool)
    .await?;

    Ok(hash)
}
```

**Critical: canonical serialization.** The same logical JSON can serialize differently (key order, whitespace). Use `serde_json::to_string` which produces deterministic output for the same `serde_json::Value`. If accepting user-provided JSON strings directly, parse-then-reserialize to ensure canonical form.

---

## 2. Snapshot vs Diff-Based Versioning

### The Core Tradeoff

| Property | Full Snapshot | Diff-Based |
|----------|--------------|------------|
| Restore speed | O(1) -- read one version | O(n) -- apply n diffs from base |
| Storage per version | O(entities) | O(changed entities) |
| Diff computation | O(entities) -- compare two snapshots | O(1) -- stored directly |
| Implementation complexity | Low | High (diff format, apply logic, corruption propagation) |
| Corruption resilience | One bad version is isolated | One bad diff corrupts all downstream |

### Why Full Snapshots Win Here

For a workspace with 10-50 nodes where 1-3 change per version, the "snapshot" vs "diff" distinction becomes less meaningful when combined with content-addressable storage. Here is why:

**A CAS-backed snapshot IS space-efficient like a diff.** Each version stores a manifest of hash references, not copies. A 50-node manifest is ~50 hash pointers (50 x 32 bytes = 1.6 KB). When 2 nodes change, only 2 new content blobs are stored. The manifest itself is tiny. This gives you:

- O(1) restore: read the manifest, fetch all blobs by hash
- O(changed) storage: only new blobs occupy space
- Zero diff-application complexity: no patching, no ordering concerns
- Git uses exactly this model (tree objects are manifests of blob hashes)

**Recommended approach: snapshot manifests with content-addressed blobs.**

### The Manifest Pattern (Git's Tree Object for Workspaces)

```sql
CREATE TABLE workspace_versions (
    version_id    UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id  UUID NOT NULL REFERENCES workspaces(id),
    version_name  TEXT,                          -- user-provided label
    version_number INTEGER NOT NULL,             -- monotonic per workspace
    parent_version_id UUID REFERENCES workspace_versions(version_id),
    topology_hash BYTEA NOT NULL REFERENCES content_blobs(content_hash),
    created_by    UUID NOT NULL,                 -- user or 'system'
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (workspace_id, version_number)
);

CREATE TABLE version_node_states (
    version_id    UUID NOT NULL REFERENCES workspace_versions(version_id),
    node_id       UUID NOT NULL,
    brief_hash    BYTEA NOT NULL REFERENCES content_blobs(content_hash),
    config_hash   BYTEA NOT NULL REFERENCES content_blobs(content_hash),
    conversation_hash BYTEA REFERENCES content_blobs(content_hash),
    PRIMARY KEY (version_id, node_id)
);
```

A version is a `workspace_versions` row (the commit) plus its `version_node_states` rows (the tree). Each node_state row points to content blobs for its brief, config, and conversation. When creating a new version, only changed blobs need new `content_blobs` entries.

**Creating a new version (pseudocode):**

```rust
async fn create_version(
    pool: &PgPool,
    workspace_id: Uuid,
    name: Option<&str>,
    current_state: &WorkspaceState,
) -> Result<Uuid, Error> {
    let mut tx = pool.begin().await?;

    // 1. Store topology blob
    let topo_hash = store_blob(&mut tx, &current_state.topology, "topology").await?;

    // 2. Get next version number
    let version_number = next_version_number(&mut tx, workspace_id).await?;

    // 3. Create version record
    let version_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO workspace_versions
         (version_id, workspace_id, version_name, version_number, topology_hash, created_by)
         VALUES ($1, $2, $3, $4, $5, $6)"
    )
    .bind(version_id)
    .bind(workspace_id)
    .bind(name)
    .bind(version_number)
    .bind(&topo_hash)
    .bind(current_state.user_id)
    .execute(&mut *tx)
    .await?;

    // 4. Store per-node blobs and manifest entries
    for node in &current_state.nodes {
        let brief_hash = store_blob(&mut tx, &node.brief, "brief").await?;
        let config_hash = store_blob(&mut tx, &node.config, "agent_config").await?;
        let conv_hash = match &node.conversation {
            Some(conv) => Some(store_blob(&mut tx, conv, "conversation").await?),
            None => None,
        };

        sqlx::query(
            "INSERT INTO version_node_states
             (version_id, node_id, brief_hash, config_hash, conversation_hash)
             VALUES ($1, $2, $3, $4, $5)"
        )
        .bind(version_id)
        .bind(node.id)
        .bind(&brief_hash)
        .bind(&config_hash)
        .bind(&conv_hash)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(version_id)
}
```

**Restoring a version:**

```rust
async fn restore_version(
    pool: &PgPool,
    version_id: Uuid,
) -> Result<WorkspaceState, Error> {
    // 1. Load version metadata + topology in one query
    let version = sqlx::query_as::<_, VersionRow>(
        "SELECT wv.*, cb.content as topology
         FROM workspace_versions wv
         JOIN content_blobs cb ON cb.content_hash = wv.topology_hash
         WHERE wv.version_id = $1"
    )
    .bind(version_id)
    .fetch_one(pool)
    .await?;

    // 2. Load all node states with their content blobs in one query
    let nodes = sqlx::query_as::<_, NodeStateRow>(
        "SELECT vns.node_id,
                b.content  as brief,
                c.content  as config,
                cv.content as conversation
         FROM version_node_states vns
         JOIN content_blobs b  ON b.content_hash  = vns.brief_hash
         JOIN content_blobs c  ON c.content_hash  = vns.config_hash
         LEFT JOIN content_blobs cv ON cv.content_hash = vns.conversation_hash
         WHERE vns.version_id = $1"
    )
    .bind(version_id)
    .fetch_all(pool)
    .await?;

    // Restore is 2 queries regardless of version depth
    Ok(WorkspaceState::from_version(version, nodes))
}
```

Restore is always exactly 2 queries (version + all node states with blob joins), regardless of how many versions exist. No diff chain to replay.

---

## 3. Session / Conversation History Versioning

### The Challenge

Conversation histories are the trickiest entity to version because:
1. They are append-only by nature (messages accumulate over time)
2. They may have foreign key references (message IDs referenced by tool calls, evaluations)
3. Users expect conversations to "make sense" after a rebase -- you cannot restore a conversation mid-stream where the AI is discussing a topology that no longer exists
4. They can be large (hundreds of messages across a multi-step agent run)

### Pattern A: Conversation as Immutable Blob (Recommended)

Treat the conversation array as a single content blob, like any other versioned entity. On checkpoint, hash and store the full message array. On restore, load it wholesale.

```sql
-- The content_blobs table stores serialized message arrays:
-- {"messages": [{"role": "user", "content": "..."}, ...]}
-- Hash covers the entire array.
```

**On rebase (revert to checkpoint):**
1. Restore the conversation blob from the target version
2. Create a NEW session with the restored messages as context
3. Append a system message: "Workspace was reverted to checkpoint '{name}'. Prior conversation context restored."
4. Do NOT mutate or delete the old session -- it becomes part of the version that was abandoned

This is how VS Code's chat checkpoints work: "snapshot-style versioning to AI chat sessions, enabling developers to restore both workspace and conversation history to a previous state." The old conversation is not destroyed; a new branch is created from the checkpoint.

**Why new session, not truncate:**
- Foreign keys from old session messages remain valid (nothing was deleted)
- Analytics and audit trail are preserved
- The agent gets a clean context window with only relevant history
- No risk of "dangling reference" messages pointing to tool calls that happened after the checkpoint

### Pattern B: Conversation Forking (Advanced)

Inspired by LangGraph and Claude Code's session management:

```sql
CREATE TABLE sessions (
    session_id     UUID PRIMARY KEY,
    workspace_id   UUID NOT NULL,
    parent_session_id UUID REFERENCES sessions(session_id),
    forked_at_message_index INTEGER,  -- message count at fork point
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE session_messages (
    message_id     UUID PRIMARY KEY,
    session_id     UUID NOT NULL REFERENCES sessions(session_id),
    message_index  INTEGER NOT NULL,
    role           TEXT NOT NULL,
    content        JSONB NOT NULL,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (session_id, message_index)
);
```

On rebase, create a new session that inherits messages up to the fork point via its parent chain. Messages after the fork point exist in the old session but are not visible in the new one.

**Loading a forked session's messages:**

```sql
-- For a forked session, load parent messages up to fork point, then own messages
WITH RECURSIVE session_chain AS (
    SELECT session_id, parent_session_id, forked_at_message_index, 0 as depth
    FROM sessions WHERE session_id = $1
    UNION ALL
    SELECT s.session_id, s.parent_session_id, s.forked_at_message_index, sc.depth + 1
    FROM sessions s
    JOIN session_chain sc ON s.session_id = sc.parent_session_id
)
SELECT sm.* FROM session_messages sm
JOIN session_chain sc ON sm.session_id = sc.session_id
WHERE sm.message_index < COALESCE(
    (SELECT forked_at_message_index FROM session_chain WHERE depth = sc.depth - 1),
    2147483647
)
ORDER BY sc.depth DESC, sm.message_index;
```

This is more complex but enables viewing divergent conversation branches. Only use if the product requires branch visualization.

### Recommendation

Start with Pattern A (conversation-as-blob). It is simpler, fits the CAS model cleanly, and handles the common "revert and continue" workflow. Pattern B adds value only if users need to compare or switch between conversation branches.

**Conversation blob sizing:** A 200-message conversation is roughly 50-100KB of JSON. At 50 nodes, worst case is 5MB per version for conversations alone. With CAS deduplication, unchanged node conversations share blobs, so incremental cost is only the conversations that changed.

---

## 4. Rust Crates for Content Hashing and CAS

### Hashing Crates

| Crate | Purpose | Downloads | Notes |
|-------|---------|-----------|-------|
| `sha2` | SHA-256 hashing | 393M+ | Pure Rust, hardware-accelerated on aarch64. v0.11.0 (March 2026). The standard choice. |
| `blake3` | BLAKE3 hashing | 80M+ | 2-3x faster than SHA-256. Merkle tree structure enables verified streaming. v1.8.3. |
| `digest` | Trait abstraction | -- | Common `Digest` trait used by sha2, blake3, etc. Swap algorithms without changing callsites. |
| `hex` | Hex encoding | -- | Convert hash bytes to/from hex strings for display. |

**SHA-256 vs BLAKE3:**

BLAKE3 is faster and has built-in Merkle tree support, but SHA-256 is the industry standard for CAS. PostgreSQL has native `sha256()` since v11, enabling server-side hash verification if needed. For a workspace versioning system where hash computation is not on the hot path, **SHA-256 is the pragmatic choice** -- universally understood, matches PostgreSQL's built-in function, and sufficient performance.

### Higher-Level CAS Crates

There is no widely-adopted Rust crate providing a complete "content-addressable store backed by PostgreSQL." The ecosystem provides primitives:

| Crate | Purpose |
|-------|---------|
| `merkle-tree-db` | Merkle tree over any KV backend. Useful if you want Merkle proofs. |
| `bao` / `bao-tree` | BLAKE3 verified streaming. Useful for large blob integrity. |
| `iroh-blobs` | Content-addressed blob store from the Iroh project (n0-computer). Designed for P2P, may be overweight for server-side Postgres. |

**Recommendation:** Build the CAS layer directly. It is ~100 lines of Rust:
1. `sha2` for hashing
2. `sqlx` for Postgres interaction
3. A `ContentStore` struct wrapping `PgPool` with `put(content) -> Hash` and `get(hash) -> Content`

```rust
use sha2::{Sha256, Digest};
use sqlx::PgPool;

pub struct ContentHash(pub [u8; 32]);

impl ContentHash {
    pub fn of(data: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(data);
        Self(hasher.finalize().into())
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn to_hex(&self) -> String {
        hex::encode(&self.0)
    }
}

pub struct ContentStore {
    pool: PgPool,
}

impl ContentStore {
    pub async fn put(&self, content: &serde_json::Value, content_type: &str) -> Result<ContentHash> {
        let canonical = serde_json::to_vec(content)?;
        let hash = ContentHash::of(&canonical);

        sqlx::query(
            "INSERT INTO content_blobs (content_hash, content_type, content, byte_size)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (content_hash) DO NOTHING"
        )
        .bind(hash.as_bytes())
        .bind(content_type)
        .bind(content)
        .bind(canonical.len() as i32)
        .execute(&self.pool)
        .await?;

        Ok(hash)
    }

    pub async fn get(&self, hash: &ContentHash) -> Result<serde_json::Value> {
        let row = sqlx::query_scalar::<_, serde_json::Value>(
            "SELECT content FROM content_blobs WHERE content_hash = $1"
        )
        .bind(hash.as_bytes())
        .fetch_one(&self.pool)
        .await?;

        Ok(row)
    }
}
```

---

## 5. Real-World Examples

### Figma: Version History

**Architecture:** Figma documents are trees of objects, each with an ID and property map. The server tracks the latest value per property per object (`Map<ObjectID, Map<Property, Value>>`). Changes are property-level last-write-wins, ordered by a central server.

**Version history:** Figma auto-saves checkpoints every 30 minutes and on user action. Named versions are user-created snapshots. Restoring a version creates a copy of the document at that point -- the original history is preserved.

**Relevance to our design:** Figma's property-level tracking is overkill for our use case (we version entire entities, not individual fields). But their named-checkpoint + auto-checkpoint pattern maps directly. Consider:
- Auto-checkpoint on every board save (the system-generated "unnamed" versions)
- User-named checkpoints on explicit "Save Version" action
- Restore creates a new version that points to the old version's content, not a destructive rewrite

### Notion: Page History

**Architecture:** Notion auto-saves a snapshot every 10 minutes while actively editing, and on session end (2 minutes of inactivity). Snapshots are full page state, not diffs.

**Retention:** Free tier retains 7 days; paid tiers retain 30+ days. This implies they do garbage-collect old snapshots.

**Relevance:** The 10-minute auto-snapshot with session-end detection is a good model for workspace versioning. The retention policy with garbage collection is essential for storage management.

### Linear: Event Sourcing

**Architecture:** Linear stores changes as discrete events with server-assigned monotonic IDs. Each event modifies a single attribute. The server persists events and materializes current state. This is essentially event sourcing with property-level granularity.

**Relevance:** Linear's approach is closer to our needs for the conversation history (append-only events). For workspace topology and configs, the snapshot approach is simpler.

### Upwelling: Collaborative Writing with Rebasing

**Architecture:** Upwelling (Ink & Switch) organizes documents into layers. Unmerged layers are "drafts" that float on top of the merged stack. When one draft merges, all other drafts automatically rebase on top. Uses Automerge CRDTs for keystroke-level tracking.

**Key insight for our design:** Upwelling's "drafts float atop the stack" pattern is relevant for AI-human co-editing. When the AI agent makes changes, those changes are a "draft" that can be reviewed before being merged into the versioned history. The rebase ensures that human edits and AI edits stay compatible.

### Datomic: Content-Addressed Entity Versioning

**Architecture:** Datomic stores immutable facts (datoms) and computes entity hashes as "an unordered combination of hashes from each key/value pair." This enables structural sharing: "new root node to acknowledge observations as of a point in time, but everything else could very well be pointers to things we already knew."

**Relevance:** This is exactly the pattern we should use. Each version creates a new manifest (root node) pointing to entity hashes, most of which are shared with prior versions. The Datomic team confirms this achieves idempotent transactions: "transacting identical snapshots produces no changes."

---

## 6. PostgreSQL-Specific Patterns

### JSONB vs TEXT for Content Storage

| Property | JSONB | TEXT |
|----------|-------|------|
| Write speed (200MB) | 2,187ms | 779ms |
| Read speed (50KB objects) | 57ms | 1,600ms (cast to JSONB) |
| Storage overhead | ~18% larger than TEXT | Baseline |
| Queryability | Full operator support (@>, ?, #>>) | None without casting |
| Compression | TOAST-compressed | TOAST-compressed |

**Recommendation: JSONB for content_blobs.content.** The CAS layer stores content that will be queried and returned as JSON to the frontend. JSONB provides:
- 7x faster reads when extracting fields
- Native indexing with GIN if we ever need to search content
- No need for parse-on-read

The 18% storage overhead is negligible given deduplication savings.

### Compression: PGLZ vs LZ4

PostgreSQL 14+ supports LZ4 compression for TOAST. Benchmarks show:

| Property | PGLZ (default) | LZ4 |
|----------|----------------|-----|
| Compression ratio | 2.23x | 2.07x |
| Write speed | Baseline | 80% faster |
| Read speed | Baseline | 20% faster |
| Parallel read scaling | Degrades past CPU count | Scales linearly |
| Compression threshold | Must achieve 25% reduction | Must not increase size |

**Recommendation: LZ4 for the content_blobs table.** Set it at table creation:

```sql
CREATE TABLE content_blobs (
    content_hash BYTEA PRIMARY KEY,
    content_type TEXT NOT NULL,
    content      JSONB NOT NULL COMPRESSION lz4,
    byte_size    INTEGER NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

LZ4 is the clear winner for our workload: mostly reads (restore operations), moderate writes (checkpoint creation), and JSON content that compresses well. The marginal compression ratio loss (2.07x vs 2.23x) is irrelevant given the deduplication savings.

### TOAST Behavior

PostgreSQL TOASTs values exceeding ~2KB. For our content types:

| Content Type | Typical Size | TOAST Behavior |
|-------------|-------------|----------------|
| Topology (50 nodes) | 5-20 KB | TOASTed, compressed |
| Node brief | 0.5-5 KB | May or may not TOAST |
| Agent config | 1-10 KB | Usually TOASTed |
| Conversation (200 msgs) | 50-100 KB | Always TOASTed |

TOASTed reads add ~2x overhead for compressed values and ~5x for externally stored values. With LZ4, decompression is fast enough that the overhead is minimal for our access pattern (restore reads all blobs in bulk, not individual field extraction).

### CASCADE Behavior for Version Deletion

**Do NOT cascade delete from workspace_versions to content_blobs.** Content blobs are shared across versions. Deleting one version must not remove blobs still referenced by other versions.

**Schema with safe deletion:**

```sql
-- Version deletion: cascade to manifest rows only
ALTER TABLE version_node_states
    ADD CONSTRAINT fk_version
    FOREIGN KEY (version_id)
    REFERENCES workspace_versions(version_id)
    ON DELETE CASCADE;

-- Blob references: restrict deletion (blobs are shared)
ALTER TABLE version_node_states
    ADD CONSTRAINT fk_brief
    FOREIGN KEY (brief_hash)
    REFERENCES content_blobs(content_hash)
    ON DELETE RESTRICT;
```

**Garbage collection for orphaned blobs:**

```sql
-- Run periodically (e.g., after version pruning)
DELETE FROM content_blobs cb
WHERE NOT EXISTS (
    SELECT 1 FROM workspace_versions wv WHERE wv.topology_hash = cb.content_hash
)
AND NOT EXISTS (
    SELECT 1 FROM version_node_states vns
    WHERE vns.brief_hash = cb.content_hash
       OR vns.config_hash = cb.content_hash
       OR vns.conversation_hash = cb.content_hash
);
```

This is analogous to `git gc` -- unreachable objects are collected after references are removed. Run this asynchronously, not inline with version deletion.

### Index Strategy Summary

```sql
-- Primary key on content_hash: B-tree, covers all lookups
-- Already created by PRIMARY KEY constraint

-- Version lookup by workspace (most common query pattern)
CREATE INDEX idx_versions_workspace
    ON workspace_versions (workspace_id, version_number DESC);

-- Node states by version (always fetched as a set)
-- Already covered by PRIMARY KEY (version_id, node_id)

-- Optional: find all versions referencing a specific blob
-- (useful for garbage collection, not needed for normal operations)
CREATE INDEX idx_node_states_brief ON version_node_states (brief_hash);
CREATE INDEX idx_node_states_config ON version_node_states (config_hash);
CREATE INDEX idx_node_states_conversation ON version_node_states (conversation_hash);
```

---

## 7. Recommended Architecture

### Complete Schema

```sql
-- Content-addressable blob store
CREATE TABLE content_blobs (
    content_hash BYTEA PRIMARY KEY,
    content_type TEXT NOT NULL,
    content      JSONB NOT NULL COMPRESSION lz4,
    byte_size    INTEGER NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Workspace version (analogous to a Git commit)
CREATE TABLE workspace_versions (
    version_id        UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id      UUID NOT NULL REFERENCES workspaces(id),
    version_name      TEXT,
    version_number    INTEGER NOT NULL,
    parent_version_id UUID REFERENCES workspace_versions(version_id),
    topology_hash     BYTEA NOT NULL REFERENCES content_blobs(content_hash),
    created_by        UUID NOT NULL,
    is_auto           BOOLEAN NOT NULL DEFAULT false,  -- auto-save vs named
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (workspace_id, version_number)
);

CREATE INDEX idx_versions_workspace
    ON workspace_versions (workspace_id, version_number DESC);

-- Per-node content manifest (analogous to a Git tree)
CREATE TABLE version_node_states (
    version_id        UUID NOT NULL REFERENCES workspace_versions(version_id) ON DELETE CASCADE,
    node_id           UUID NOT NULL,
    brief_hash        BYTEA NOT NULL REFERENCES content_blobs(content_hash),
    config_hash       BYTEA NOT NULL REFERENCES content_blobs(content_hash),
    conversation_hash BYTEA REFERENCES content_blobs(content_hash),
    PRIMARY KEY (version_id, node_id)
);

-- Conversation sessions (tracks which session is active per workspace)
CREATE TABLE workspace_sessions (
    session_id        UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id      UUID NOT NULL REFERENCES workspaces(id),
    forked_from_version_id UUID REFERENCES workspace_versions(version_id),
    is_active         BOOLEAN NOT NULL DEFAULT true,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

### Operation Costs

| Operation | Queries | New Rows (50-node, 2-node change) |
|-----------|---------|-----------------------------------|
| Create checkpoint | 1 tx (batched inserts) | 1 version + 50 node_states + 8 new blobs |
| Restore checkpoint | 2 SELECTs (version + node_states JOIN blobs) | 0 |
| List versions | 1 SELECT | 0 |
| Delete old version | 1 DELETE (cascades to node_states) | 0 |
| Garbage collect blobs | 1 DELETE with NOT EXISTS | negative rows |

### Storage Estimates (50-node workflow)

| Component | Per Version (full) | Per Version (incremental, 2 nodes changed) |
|-----------|-------------------|---------------------------------------------|
| workspace_versions row | ~200 bytes | ~200 bytes |
| version_node_states rows | 50 x ~130 bytes = 6.5 KB | 50 x ~130 bytes = 6.5 KB |
| New content_blobs | 50 x 4 fields x ~5KB avg = 1 MB | 2 x 4 fields x ~5KB = 40 KB |
| Total new storage | ~1 MB | ~47 KB |

At 47 KB per incremental version, you can store 21,000 versions per GB. Aggressive auto-save (every 30 seconds) would produce ~120 versions/hour, consuming ~5.6 MB/hour or ~45 MB for a full workday. Easily manageable.

### Version Retention Policy

Follow the Notion/Figma pattern:
- Keep all named versions indefinitely
- Keep auto-versions for 30 days
- After 30 days, keep one auto-version per day for 90 days
- After 90 days, keep one auto-version per week

```sql
-- Prune old auto-versions (keep at most 1 per day older than 30 days)
DELETE FROM workspace_versions wv
WHERE wv.is_auto = true
  AND wv.created_at < now() - interval '30 days'
  AND wv.version_id NOT IN (
      SELECT DISTINCT ON (workspace_id, created_at::date)
             version_id
      FROM workspace_versions
      WHERE is_auto = true
      ORDER BY workspace_id, created_at::date, created_at DESC
  );
-- Then run blob garbage collection
```

---

## Sources

- [Content-addressable storage with Postgres - Richard Towers](https://www.richard-towers.com/2020/06/06/content-addressable-storage-postgres.html)
- [Git in Postgres - Andrew Nesbitt](https://nesbitt.io/2026/02/26/git-in-postgres.html)
- [Git's database internals: packed object store - GitHub Blog](https://github.blog/open-source/git/gits-database-internals-i-packed-object-store/)
- [Git Internals - Git Objects](https://git-scm.com/book/en/v2/Git-Internals-Git-Objects)
- [Multi-temporal versioning in Postgres - HASH](https://hash.dev/blog/multi-temporal-versioning)
- [Datomic and Content Addressable Techniques - Latacora](https://www.latacora.com/blog/2024/09/13/datomic-and-content-addressable-techniques/)
- [How Figma's multiplayer technology works - Figma Blog](https://www.figma.com/blog/how-figmas-multiplayer-technology-works/)
- [Understanding sync engines: Figma, Linear, Google Docs - Liveblocks](https://liveblocks.io/blog/understanding-sync-engines-how-figma-linear-and-google-docs-work)
- [Upwelling: Real-time collaboration with version control - Ink & Switch](https://www.inkandswitch.com/upwelling/)
- [Checkpoint/Restore Systems for AI Agents - Eunomia](https://eunomia.dev/blog/2025/05/11/checkpointrestore-systems-evolution-techniques-and-applications-in-ai-agents/)
- [Re-Introducing Hash Indexes in PostgreSQL - Haki Benita](https://hakibenita.com/postgresql-hash-index)
- [PostgreSQL PGLZ vs LZ4 - Tiger Data](https://www.tigerdata.com/blog/optimizing-postgresql-performance-compression-pglz-vs-lz4)
- [JSON vs JSONB, PGLZ vs LZ4 - depesz](https://www.depesz.com/2025/11/29/using-json-json-vs-jsonb-pglz-vs-lz4-key-optimization-parsing-speed/)
- [JSONB performance and TOAST - pganalyze](https://pganalyze.com/blog/5mins-postgres-jsonb-toast)
- [CAS at scale - Design Gurus](https://www.designgurus.io/answers/detail/how-would-you-implement-contentaddressable-storage-at-scale)
- [SHA-2 crate - crates.io](https://crates.io/crates/sha2)
- [BLAKE3 crate - crates.io](https://crates.io/crates/blake3)
- [CAS keyword - crates.io](https://crates.io/keywords/cas)
- [pgMemento - Audit trail with schema versioning](https://github.com/pgMemento/pgMemento)
- [Git-like versioning in Postgres - Specfy](https://www.specfy.io/blog/7-git-like-versioning-in-postgres)
- [Content-Addressable Storage - Wikipedia](https://en.wikipedia.org/wiki/Content-addressable_storage)
