# Milestone: Sticky Execution

**Depends on:** [MILESTONE_AGENT_SYSTEM.md](MILESTONE_AGENT_SYSTEM.md) — the agent system upgrade defines the protocols, governance, and runtime artifacts that sticky freezes.

**Concept:** Runtime artifacts — documents, designed agent prompts, step outputs, extracted beliefs — can be **frozen by the user** and reused across runs. The user progressively locks down the parts of their workflow that work well, leaving only the dynamic "pockets" open for fresh execution. Over time, the workflow evolves from fully open to a perfectly running machine.

---

## The Metaphor

A workflow is a machine the user builds over iterations.

- **Run 1:** Everything is open. Documents get generated, agents get designed, rooms debate. The user reviews every output.
- **Run 3:** The API spec came out great — sticky it. The architecture doc needs work — leave it open. The code reviewer agent's designed prompts were perfect — sticky them. The tester agent needs rethinking — leave it open.
- **Run 7:** Most of the machine is frozen. Only 2 documents and 1 agent design are still open pockets. Runs are fast, focused, and cheap.
- **Run 12:** Everything is sticky. The machine is done. It runs predictably every time. The user can un-sticky anything the moment something needs to change — pop a pocket open, let it regenerate, review it, sticky it again.

Stickies are the user's **stamps of approval** accumulating over time. The system never auto-stickies. The system never auto-invalidates. The user is always in control of what's frozen and what's open.

---

## What Gets Sticky

Every protocol produces runtime artifacts. Stickies operate at the **artifact level**, not the step level — a step can be partially sticky (some artifacts frozen, others pending).

### Documents (Documenter Protocol)

The clearest case. A documenter step has N document definitions. Each definition produces a document through the strategist → researcher → writer pipeline. When a document is stickied:

- The strategist sees it as "already satisfied" and plans only for pending documents
- No researcher or writer agent is created for it
- The designer only designs agents for the pending documents
- The sticky document's content is available to downstream steps as if freshly generated
- Required reading citations from the sticky document remain valid

**What's frozen:** The document content, title, and summary. Referenced by `protocol_document_def_id`.

### Agent Designs (Task Force / Room Protocols)

The Agent Designer produces a `DesignedAgentPrompt` for each agent in the roster — system prompt, task prompt, tool assignments, receives_from routing, and the designer's reasoning. When an agent design is stickied:

- The designer receives the sticky designs as "already designed" and only designs pending agents
- The executor uses the frozen system prompt, task prompt, and tool assignments for sticky agents
- If a NEW agent is added to the roster, only that one goes through the designer
- The sticky agent's governance patterns (scope boundaries, decision tracing, required reading instructions) from the upgraded designer are preserved exactly as approved

**What's frozen:** The `DesignedAgentPrompt` — system_prompt, task_prompt, tools[], receives_from[], reasoning. Referenced by `persisted_agent_id` + `step_id`.

### Step Outputs (Single / For-Each Steps)

A single or for-each step produces a `StepExecutionEnvelope` with structured data. When stickied:

- The step is entirely skipped
- The frozen envelope is injected into `DagExecutionState.completed_envelopes` and `var_outputs` as if the step just ran
- Downstream port resolution works unchanged — ports extract from the frozen envelope via json_path exactly as they would from a fresh one
- Decision trace data (reasoning, confidence, convention references from Slice 8) is preserved in the frozen envelope

**What's frozen:** The full `StepExecutionEnvelope`. Referenced by `workflow_step_id`.

### Beliefs (Belief Capture Protocol)

Belief capture extracts structured beliefs from upstream step outputs. When a belief set is stickied:

- Belief extraction is skipped for that source
- The frozen beliefs are loaded directly into the run's belief pool
- Room agents querying beliefs (via `query_beliefs` from Slice 9) see frozen beliefs alongside fresh ones
- Frozen beliefs retain their original confidence, tags, and source attribution

**What's frozen:** The set of beliefs from a specific source step. Referenced by `source_step_id`.

### What Should NOT Be Sticky

- **Room discussions** — The whole point of a room is fresh debate with current context. Member designs can be sticky, but the conversation itself should always be live. A user could still sticky the room's output envelope if they wanted to skip the entire room, but that's a step-output sticky, not a room-specific one.
- **Episodic memory reflections** — Reflections accumulate over time (Slice 4). They should never be frozen — each run should produce a new reflection that adds to the learning corpus.
- **Workflow query tool results** — These are live reads of DAG state (Slice 9). They return current data, not cached data.
- **Compliance validation results** — If required reading conventions change (Slice 5), compliance needs fresh evaluation.

---

## How It Works in Execution

### Pre-Run: Load Stickies into DAG State

In `execute_workflow_via_engine()`, after topological sort and port metadata prefetch, before entering `run_dag_loop()`:

1. Load all active stickies for this workflow from the `workflow_stickies` table
2. For each **step-output sticky** (single/for-each steps):
   - Deserialize the frozen `StepExecutionEnvelope`
   - Pre-populate `dag_state.completed_envelopes[step_id]`
   - Pre-populate `dag_state.var_outputs[output_variable_name]` with the envelope's structured data
   - Mark the step as completed in `dag_state.completed[step_id]`
3. For each **document sticky** and **agent-design sticky**: store in a `StickyManifest` passed to the DAG loop — individual protocols check this during their execution
4. For each **belief sticky**: load frozen beliefs into the run's belief pool
5. Broadcast `WorkflowEvent::StickiesLoaded { count, steps_skipped }` so the UI knows

### During Execution: Protocol-Level Sticky Checks

**In `run_dag_loop()`:**

When iterating sorted steps, before executing a step:
1. Check if `dag_state.completed.contains(step_id)` — if yes, this step was pre-populated from a sticky, skip it
2. Broadcast `StepEvent::StickySkipped { step_id, source_run_id }` so the UI shows the step as "using sticky from Run #N"

**In documenter execution** (`src/server/hub/dag/documenter/mod.rs`):

Before Phase 1 (strategy):
1. Check `sticky_manifest.documents_for_step(step_id)`
2. Partition `document_defs` into `sticky_defs` and `pending_defs`
3. If ALL docs are sticky: skip the entire step, compose an envelope from sticky contents
4. If SOME docs are sticky: pass `pending_defs` to the strategist, inject sticky doc summaries as "already completed" context
5. Phase 2 (research) and Phase 3 (write) only run for pending docs
6. Final envelope merges sticky doc outputs with freshly generated ones

**In task_force execution** (`src/server/hub/dag/task_force/mod.rs`):

Before Agent Designer call:
1. Check `sticky_manifest.agent_designs_for_step(step_id)`
2. Partition roster into `sticky_agents` and `pending_agents`
3. If ALL agents are sticky: skip the designer entirely, use frozen designs
4. If SOME agents are sticky: send only `pending_agents` to the designer, merge results with sticky designs
5. During sequential execution, each agent uses its design source (sticky or fresh) transparently

**In room execution** (`src/server/hub/dag/room_step/mod.rs`):

Before Agent Designer call:
1. Check `sticky_manifest.agent_designs_for_step(step_id)`
2. Same partition logic as task_force — sticky member designs skip the designer for those members
3. The room discussion itself always runs fresh (unless the entire step output is stickied)

**In belief_capture execution** (`src/server/hub/dag/belief_capture/mod.rs`):

Before extraction:
1. Check `sticky_manifest.beliefs_for_step(step_id)`
2. For sticky sources: load frozen beliefs directly, skip extraction
3. For non-sticky sources: extract normally
4. Merge sticky and fresh beliefs into the run's belief pool

### Post-Run: User Curates Stickies

After a run completes, the user reviews results and decides what to freeze:

1. User sees run results organized by step and artifact
2. Each artifact has a "Make Sticky" action
3. Creating a sticky stores the artifact's content (from the run's content_versions) and marks it active
4. Existing stickies can be "Released" (un-stickied) to force regeneration next run
5. Re-stickying from a newer run replaces the old frozen value

---

## Slice 1: Sticky Data Model and API

**Goal:** Establish the data model, repository layer, and REST API for managing stickies.

### Ticket 1.1: Sticky Schema and Repository

**Scope:** Create the database table for workflow stickies and the repository functions to manage them.

**Work:**

Migration:
```sql
CREATE TABLE workflow_stickies (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workflow_id UUID NOT NULL REFERENCES workflows(id) ON DELETE CASCADE,
    step_id UUID NOT NULL REFERENCES workflow_steps(id) ON DELETE CASCADE,
    artifact_type TEXT NOT NULL,
    artifact_id UUID,
    artifact_label TEXT NOT NULL,
    frozen_value JSONB NOT NULL,
    source_run_id UUID NOT NULL REFERENCES workflow_executions(id),
    content_version_id UUID REFERENCES content_versions(id),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    created_by UUID REFERENCES users(id),
    UNIQUE(workflow_id, step_id, artifact_type, artifact_id)
);

CREATE INDEX idx_stickies_workflow ON workflow_stickies(workflow_id);
CREATE INDEX idx_stickies_step ON workflow_stickies(step_id);

COMMENT ON TABLE workflow_stickies IS 'User-frozen runtime artifacts that persist across workflow runs';
COMMENT ON COLUMN workflow_stickies.artifact_type IS 'One of: document, agent_design, step_output, belief_set';
COMMENT ON COLUMN workflow_stickies.artifact_id IS 'References the specific artifact — doc_def_id for documents, agent_id for designs, NULL for step_output/belief_set';
COMMENT ON COLUMN workflow_stickies.artifact_label IS 'Human-readable label for UI display (e.g., "API Specification", "Scanner Agent Design")';
COMMENT ON COLUMN workflow_stickies.frozen_value IS 'The frozen artifact content — document body, DesignedAgentPrompt, StepExecutionEnvelope, or belief array';
COMMENT ON COLUMN workflow_stickies.content_version_id IS 'Links to the content_versions entry this was frozen from, for provenance tracking';
```

The UNIQUE constraint ensures one sticky per artifact. Re-stickying from a newer run replaces the old row (upsert).

`artifact_type` values and what `artifact_id` references:
- `"document"` → `artifact_id` = `protocol_document_defs.id`
- `"agent_design"` → `artifact_id` = `persisted_agents.id` (scoped to step via `step_id`)
- `"step_output"` → `artifact_id` = NULL (the entire step output, one per step)
- `"belief_set"` → `artifact_id` = NULL (all beliefs from this source step)

Types:
```rust
pub struct WorkflowStickyRow {
    pub id: Uuid,
    pub workflow_id: Uuid,
    pub step_id: Uuid,
    pub artifact_type: String,
    pub artifact_id: Option<Uuid>,
    pub artifact_label: String,
    pub frozen_value: serde_json::Value,
    pub source_run_id: Uuid,
    pub content_version_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub created_by: Option<Uuid>,
}

pub struct StickyManifest {
    pub step_output_stickies: HashMap<Uuid, WorkflowStickyRow>,
    pub document_stickies: HashMap<Uuid, Vec<WorkflowStickyRow>>,
    pub agent_design_stickies: HashMap<Uuid, Vec<WorkflowStickyRow>>,
    pub belief_stickies: HashMap<Uuid, WorkflowStickyRow>,
}
```

`StickyManifest` is built once per run from all active stickies for the workflow. Keyed by `step_id` for fast lookup during execution.

Repository functions (new module `src/db/stickies/mod.rs`):
- `list_stickies_for_workflow(workflow_id) → Vec<WorkflowStickyRow>` — all active stickies
- `list_stickies_for_step(step_id) → Vec<WorkflowStickyRow>` — stickies for one step
- `create_sticky(workflow_id, step_id, artifact_type, artifact_id, label, frozen_value, source_run_id, content_version_id, user_id) → WorkflowStickyRow` — upsert (ON CONFLICT replace)
- `delete_sticky(sticky_id)` — remove a single sticky (un-sticky)
- `delete_stickies_for_step(step_id)` — release all stickies on a step
- `delete_all_stickies(workflow_id)` — release everything (reset the machine)
- `get_sticky(sticky_id) → Option<WorkflowStickyRow>` — single lookup

**Acceptance:**
- Stickies can be created, listed, and deleted per workflow and step
- UNIQUE constraint prevents duplicate stickies on the same artifact
- Upsert semantics: re-stickying replaces the old frozen value
- Cascade delete: removing a step removes its stickies
- Tests: `cargo test db::stickies::tests`

### Ticket 1.2: Sticky REST API

**Scope:** CRUD endpoints for managing stickies from the frontend.

**Work:**

New API module `src/server/api/stickies/mod.rs`:

```
GET    /api/workflows/:workflow_id/stickies
POST   /api/workflows/:workflow_id/stickies
DELETE /api/workflows/:workflow_id/stickies/:sticky_id
DELETE /api/workflows/:workflow_id/stickies  (bulk delete, query param ?step_id= or ?all=true)
```

**`GET /api/workflows/:workflow_id/stickies`**

Returns all active stickies for a workflow, grouped by step for UI rendering. Each sticky includes:
- `id`, `step_id`, `step_name` (joined from workflow_steps)
- `artifact_type`, `artifact_id`, `artifact_label`
- `source_run_id`, `source_run_number` (joined from workflow_executions)
- `created_at`, `created_by`
- `frozen_value` is NOT returned in the list endpoint (can be large). Use the detail endpoint or content version to inspect.

**`POST /api/workflows/:workflow_id/stickies`**

Creates a sticky from a completed run's artifact.

Request body:
```json
{
  "step_id": "uuid",
  "artifact_type": "document",
  "artifact_id": "uuid-of-doc-def",
  "source_run_id": "uuid-of-the-run-to-freeze-from"
}
```

The handler:
1. Validates the run exists and belongs to this workflow
2. Validates the step exists in this workflow
3. Loads the artifact's value from the run:
   - `document`: Load from `content_versions` where source_id = doc_def_id and run_snapshot.run_id = source_run_id
   - `agent_design`: Load from `agent_designer_outputs` where run_id = source_run_id and agent_id = artifact_id
   - `step_output`: Load from `run_snapshots` where step_id and run_id, content_type = "envelope"
   - `belief_set`: Load from `beliefs` where source_step_id = step_id and workflow_execution_id = source_run_id
4. Derives `artifact_label` from the artifact (document title, agent name, step name)
5. Upserts the sticky row

**`DELETE /api/workflows/:workflow_id/stickies/:sticky_id`**

Un-stickies a single artifact. The next run will regenerate it.

**`DELETE /api/workflows/:workflow_id/stickies?step_id=X`**

Un-stickies all artifacts on a step. Useful for "regenerate this entire step."

**`DELETE /api/workflows/:workflow_id/stickies?all=true`**

Releases all stickies. Resets the machine to fully open.

**Acceptance:**
- All CRUD operations work and validate ownership/existence
- Creating a sticky from a run correctly loads and freezes the artifact value
- List endpoint groups stickies by step for UI consumption
- Bulk delete supports step-scoped and workflow-scoped release
- Tests: `cargo test server::api::stickies::tests`

---

## Slice 2: Sticky-Aware Execution

**Goal:** Each protocol respects stickies during execution — skipping frozen artifacts, executing only pending ones, and merging results.

### Ticket 2.1: DAG Loop Sticky Pre-Loading

**Scope:** Before the DAG loop starts, load stickies and pre-populate execution state for fully-stickied steps.

**Work:**

In `execute_workflow_via_engine()` (after port metadata prefetch, before `run_dag_loop()`):

1. Load all stickies: `let stickies = state.repo().stickies().list_for_workflow(workflow_id).await?`
2. Build `StickyManifest::from_rows(stickies)` — partitions into step_output, document, agent_design, belief categories by step_id
3. For each **step-output sticky**:
   - Deserialize `frozen_value` as `StepExecutionEnvelope`
   - Call `dag_state.record_step_output(step_id, output, envelope)` — same function used during normal execution
   - This pre-populates `completed`, `completed_envelopes`, and `var_outputs`
   - The step is now "already done" from the DAG loop's perspective
4. Pass `sticky_manifest` into `run_dag_loop()` as a new parameter

In `run_dag_loop()`, when iterating steps:
1. If `dag_state.completed.contains_key(&step.id)`:
   - Check if this was from a sticky (check `sticky_manifest.step_output_stickies`)
   - Broadcast `StepEvent::StickyUsed { step_id, step_name, source_run_id }` — distinct from regular completion events so the UI can differentiate
   - Skip execution, continue to next step
2. Pass `sticky_manifest` reference to protocol executors that support partial stickies

Snapshot integration:
- When `snapshot_content()` runs post-step, sticky-skipped steps should NOT create new snapshots — the frozen content is already versioned from the source run
- Add a check: if step was resolved from sticky, skip snapshot capture for that step

**Acceptance:**
- Steps with step-output stickies are pre-populated in DAG state and skipped during execution
- Downstream steps receive sticky outputs through normal port resolution (no special handling needed — they're in `completed_envelopes`)
- Sticky-skipped steps broadcast a distinct event type
- Sticky-skipped steps don't create redundant content version snapshots
- Token/cost accounting: sticky-skipped steps contribute 0 tokens and $0 cost to the run total
- Tests: `cargo test hub::dag::tests::sticky_preload`

### Ticket 2.2: Documenter Partial Sticky Execution

**Scope:** The documenter protocol skips sticky documents and only generates pending ones.

**Work:**

In `execute_documenter_step()`:

1. Before Phase 1, check `sticky_manifest.document_stickies.get(&step_id)`
2. Partition document_defs:
   ```rust
   let sticky_doc_ids: HashSet<Uuid> = manifest.iter().map(|s| s.artifact_id.unwrap()).collect();
   let (sticky_defs, pending_defs): (Vec<_>, Vec<_>) = document_defs
       .iter()
       .partition(|d| sticky_doc_ids.contains(&d.id));
   ```
3. **All sticky:** Skip the entire documenter step. Build an envelope from the sticky document values (merge all frozen_value JSONs into a single documents array). Record as completed.
4. **Partial sticky:**
   - Pass only `pending_defs` to the strategist in Phase 1
   - Inject a `<completed_documents>` block into the strategist's context listing sticky doc names and summaries so it knows the full picture: "3 of 5 documents are already completed. Plan research and writing for the remaining 2."
   - Phase 2 (research) and Phase 3 (write) only create agents for pending docs
   - The Agent Designer only receives pending doc agents in its `DesignerInput`
   - Final envelope merges: sticky doc outputs (from frozen_value) + fresh doc outputs (from writer execution)
5. **None sticky:** Normal execution, no changes

Content versioning: for sticky docs that weren't regenerated, link to their existing content_version_id from the sticky row (no new snapshot needed).

**Acceptance:**
- Documenter with all-sticky docs skips entirely and produces a valid envelope
- Documenter with partial stickies only runs strategy/research/write for pending docs
- Strategist receives context about which docs are already done
- Designer only designs agents for pending docs (no wasted LLM calls)
- Final envelope is indistinguishable from a full run — downstream consumers don't know which docs were sticky
- Tests: `cargo test hub::dag::documenter::tests::sticky_partial`, `cargo test hub::dag::documenter::tests::sticky_full`

### Ticket 2.3: Task Force and Room Partial Sticky Execution

**Scope:** The task force and room protocols skip sticky agent designs and only run the designer for pending agents.

**Work:**

**Task Force** (`execute_task_force_step()`):

1. Before Agent Designer call, check `sticky_manifest.agent_design_stickies.get(&step_id)`
2. Partition roster:
   ```rust
   let sticky_agent_ids: HashSet<Uuid> = manifest.iter().map(|s| s.artifact_id.unwrap()).collect();
   let (sticky_roster, pending_roster): (Vec<_>, Vec<_>) = roster
       .iter()
       .partition(|r| sticky_agent_ids.contains(&r.agent_id));
   ```
3. **All sticky:** Skip the designer entirely. Deserialize frozen `DesignedAgentPrompt` values for each agent.
4. **Partial sticky:** Send only `pending_roster` agents to the designer. Merge designer output with deserialized sticky designs.
5. **None sticky:** Normal execution.
6. During sequential agent execution, each agent uses its design source (sticky or fresh) transparently — the executor doesn't need to know which is which.

**Room** (`execute_room_step()`):

Same logic for member designs:
1. Check sticky manifest for agent designs scoped to this step
2. Partition members into sticky/pending
3. Designer only designs pending members
4. Merge sticky + fresh designs
5. Room discussion runs normally with all members using their designs (sticky or fresh)

**Acceptance:**
- Task force with all-sticky agent designs skips the designer call entirely
- Task force with partial stickies only sends pending agents to the designer
- Room follows the same partial-sticky pattern for member designs
- Sequential/discussion execution is agnostic to design source
- No regression in receives_from routing — sticky agents' routing declarations are preserved
- Tests: `cargo test hub::dag::task_force::tests::sticky_designs`, `cargo test hub::dag::room_step::tests::sticky_designs`

### Ticket 2.4: Belief Capture Sticky Execution

**Scope:** Belief capture skips extraction for sources with sticky beliefs.

**Work:**

In `execute_belief_capture_step()`:

1. Check `sticky_manifest.belief_stickies.get(&step_id)`
2. For each upstream source that has a sticky:
   - Load frozen beliefs from `frozen_value` (array of belief objects)
   - Insert into the run's belief pool directly (same table, new `workflow_execution_id`, flagged as `source_phase = 'sticky'`)
   - Skip the extraction LLM call for that source
3. For upstream sources without stickies: extract normally
4. Final output merges sticky and fresh beliefs

A new `source_phase` value `'sticky'` distinguishes frozen beliefs from freshly extracted ones. The `query_beliefs` tool (Slice 9) can filter or include sticky beliefs transparently.

**Acceptance:**
- Belief capture with all-sticky sources skips all extraction
- Belief capture with partial stickies only extracts from non-sticky sources
- Sticky beliefs are inserted with `source_phase = 'sticky'` for traceability
- Query tools return sticky beliefs alongside fresh ones
- Tests: `cargo test hub::dag::belief_capture::tests::sticky_beliefs`

### Ticket 2.5: Sticky WebSocket Events

**Scope:** Broadcast sticky-specific events so the UI can show what was skipped vs executed.

**Work:**

New event kinds:
- `StepStickyUsed { step_id, step_name, artifact_count, source_run_id }` — a step was fully skipped via sticky
- `StepPartialSticky { step_id, step_name, sticky_count, pending_count }` — a step has partial stickies, executing only pending artifacts
- `StickyManifestLoaded { total_stickies, steps_fully_skipped, steps_partially_skipped }` — pre-run summary

These broadcast on the workflow's channel. The frontend can show:
- A step flash with "Using sticky from Run #N" for fully skipped steps
- A progress indicator showing "2 of 5 documents — 3 sticky" for partial execution
- A pre-run summary: "This run will skip 4 steps via stickies"

**Acceptance:**
- All sticky events are broadcast at the right moments
- Events include enough context for the UI to render sticky status
- Events are distinct from normal step completion events

---

## Slice 3: Sticky UI

**Goal:** The user can see, create, and manage stickies through the frontend — both before runs (machine status) and after runs (curation).

### Ticket 3.1: Pre-Run Machine Status Panel

**Scope:** Before starting a run, show the user the current sticky state of their workflow — what's frozen, what's pending, and from which run each sticky came.

**Work:**

New component: `StickyStatusPanel` — rendered when the user clicks "Run" (before actual execution starts).

Layout:
```
┌─ Run Preparation ────────────────────────────────────────┐
│                                                          │
│  Step 1: API Documentation [DOCUMENTER]                  │
│  ├─ API Specification         ✅ Sticky (Run #7)  [⟳]  │
│  ├─ Architecture Overview     ✅ Sticky (Run #7)  [⟳]  │
│  └─ Security Checklist        ○  Pending                │
│                                                          │
│  Step 2: Code Analysis [TASK FORCE]                      │
│  ├─ Scanner Agent Design      ✅ Sticky (Run #8)  [⟳]  │
│  ├─ Reviewer Agent Design     ✅ Sticky (Run #8)  [⟳]  │
│  └─ Reporter Agent Design     ○  Pending                │
│                                                          │
│  Step 3: Security Scan [SINGLE]                          │
│  └─ Step Output               ✅ Sticky (Run #8)  [⟳]  │
│                                                          │
│  Step 4: Review Meeting [ROOM]                           │
│  └─ Full Discussion           ○  Pending                │
│                                                          │
│  Summary: 5 stickies, 3 steps will execute               │
│                                                          │
│  [Release All]                          [Run ▶]         │
└──────────────────────────────────────────────────────────┘
```

- `[⟳]` button releases a single sticky (un-sticky, will regenerate)
- `[Release All]` clears all stickies for a fresh run
- `[Run ▶]` starts execution with current sticky configuration
- Steps with ALL artifacts sticky show a "fully sticky" badge
- Steps with NO stickies show as fully pending

Data source: `GET /api/workflows/:id/stickies` + `GET /api/workflows/:id/steps` (for step names and archetype info)

Artifact discovery per step archetype:
- **Documenter:** List document definitions as individual artifacts
- **Task Force / Room:** List roster agents as individual artifacts (designs)
- **Single / For-Each:** Show one artifact: "Step Output"
- **Belief Capture:** Show one artifact: "Extracted Beliefs"
- **Sub-Workflow:** Show one artifact: "Sub-Workflow Output"

**Acceptance:**
- Panel shows all steps with their artifacts and sticky/pending status
- User can release individual stickies or all stickies
- Panel shows which run each sticky came from
- Panel shows a summary count (X stickies, Y steps will execute)
- After releasing stickies, the panel updates immediately
- Run button starts execution with current configuration

### Ticket 3.2: Post-Run Sticky Curation

**Scope:** After a run completes, the user reviews results and decides what to freeze.

**Work:**

New component: `RunStickyReview` — shown on the run detail page after a run completes.

Layout:
```
┌─ Run #9 Complete — Curate Stickies ──────────────────────┐
│                                                          │
│  Step 1: API Documentation                               │
│  ├─ Security Checklist  ✅ Generated  [Make Sticky]     │
│  │   Preview: "## Security Requirements\n1. Auth..."    │
│  │                                                      │
│  Step 2: Code Analysis                                   │
│  ├─ Reporter Design     ✅ Designed   [Make Sticky]     │
│  │   System: "You are a security reporting specialist.." │
│  ├─ Scanner Output      ✅ Completed  [Make Sticky]     │
│  ├─ Reviewer Output     ✅ Completed  [Make Sticky]     │
│  └─ Reporter Output     ✅ Completed  [Make Sticky]     │
│                                                          │
│  Step 4: Review Meeting                                  │
│  └─ Discussion Output   ✅ Completed  [Make Sticky]     │
│                                                          │
│  Already Sticky (unchanged this run):                    │
│  ├─ API Specification       ✅ Sticky (Run #7)          │
│  ├─ Architecture Overview   ✅ Sticky (Run #7)          │
│  ├─ Scanner Agent Design    ✅ Sticky (Run #8)          │
│  └─ Reviewer Agent Design   ✅ Sticky (Run #8)          │
│                                                          │
│  [Sticky All New]                                        │
└──────────────────────────────────────────────────────────┘
```

- Each freshly generated artifact has a `[Make Sticky]` button
- Clicking it calls `POST /api/workflows/:id/stickies` with the artifact info
- Already-sticky artifacts (carried forward) are shown separately as confirmation
- `[Sticky All New]` bulk-stickies everything generated in this run
- Preview text shows a truncated view of the artifact content so the user can assess quality before freezing

**Acceptance:**
- Post-run view shows all generated artifacts with sticky controls
- User can sticky individual artifacts or bulk-sticky all
- Already-sticky artifacts are shown for context
- Preview text helps user assess quality before freezing
- Stickying an artifact immediately updates the pre-run panel for next run

### Ticket 3.3: Canvas Sticky Indicators

**Scope:** Show sticky status directly on workflow step nodes in the canvas view.

**Work:**

On each step node in the canvas:
- Show a small badge or indicator for sticky artifacts
- Examples:
  - Documenter node: `📄 3/5 sticky`
  - Task force node: `🤖 2/3 designs sticky`
  - Single node: `✅ Output sticky`
  - No stickies: no badge (clean)
- Clicking the badge opens the step's sticky detail (same info as pre-run panel, scoped to one step)

Fetch stickies as part of the workflow load (piggyback on existing workflow data fetch, don't add a separate call per step).

**Acceptance:**
- Canvas nodes show sticky status at a glance
- Badges update when stickies are created or released
- Clicking a badge shows detail for that step's stickies
- Steps with no stickies show no badge (no visual noise)

### Ticket 3.4: Sticky Status in Run Execution View

**Scope:** During and after a run, show which steps used stickies vs executed fresh.

**Work:**

In the run execution / run detail view:
- Steps resolved from stickies show a distinct visual treatment (e.g., a "sticky" icon or muted color) instead of the normal execution animation
- Sticky-skipped steps show: "Used sticky from Run #N" with a link to the source run
- Partially-stickied steps show: "3 sticky, 2 generated" in the step detail
- Token/cost summary excludes sticky-skipped steps (or shows them as $0)
- Timeline view: sticky steps appear as instant (no duration bar) since they weren't executed

**Acceptance:**
- Run view clearly distinguishes sticky-skipped steps from executed steps
- Sticky steps link to their source run for provenance
- Partial stickies show the breakdown
- Cost summary accurately reflects only the work that was actually done

---

## Slice 4: Sticky + Template Integration

**Goal:** Templates can carry stickies, creating pre-loaded workflow snapshots that include both structure and frozen runtime values.

### Ticket 4.1: Stickies in Template Snapshots

**Scope:** When capturing a workflow template, include active stickies. When restoring from a template, restore its stickies.

**Work:**

In `capture_workflow_snapshot()` (`src/server/hub/dag/templates/mod.rs`):
- Add `stickies: Vec<WorkflowStickyRow>` to `WorkflowSnapshot`
- When capturing, load all active stickies for the workflow and include them

In `restore_workflow_from_snapshot()`:
- After restoring steps, edges, agents, etc., restore stickies:
  - Map old step_ids / artifact_ids to new IDs (same as the existing ID remapping in restore)
  - Insert restored stickies with `source_run_id` pointing to the original run (provenance preserved)
  - Set `created_by` to the user who created the template instance

At execution time from a template:
- Stickies from the template are already in the workflow's sticky table
- Pre-run loading picks them up automatically
- No changes needed in the execution path

**Why this matters:** A template with stickies is a **pre-loaded machine**. The user builds a workflow, freezes the stable parts, then saves it as a template. Anyone who instantiates that template gets the frozen artifacts for free — they only need to run the open pockets. This is how you create reusable, production-grade workflow templates.

**Acceptance:**
- Template capture includes active stickies
- Template restore recreates stickies with correctly remapped IDs
- Restored stickies work in the pre-run loading phase
- Provenance is preserved: restored stickies show they came from the original source run
- Tests: `cargo test hub::dag::templates::tests::sticky_capture_restore`

### Ticket 4.2: Template Sticky Preview

**Scope:** When browsing templates, show which artifacts are pre-loaded (sticky) and which are open.

**Work:**

In the template list / detail UI:
- Show sticky count per template: "12 artifacts pre-loaded, 4 open"
- Template detail shows the same sticky breakdown as the pre-run panel
- When creating a new workflow from a template, show a preview: "This template comes with 12 pre-loaded artifacts. You can release any of them after creation."

**Acceptance:**
- Template list shows sticky counts
- Template detail shows the pre-loaded artifact breakdown
- User understands what they're getting before instantiating a template

---

## Slice Summary

| Slice | Tickets | Backend | Frontend | Depends On |
|-------|---------|---------|----------|------------|
| **1. Data Model + API** | 2 | Migration + repository + REST endpoints | None | Content versioning (prereq) |
| **2. Sticky-Aware Execution** | 5 | DAG loop pre-loading + per-protocol partial execution + WS events | None | Slice 1 |
| **3. Sticky UI** | 4 | None | Pre-run panel + post-run curation + canvas badges + run view | Slices 1, 2 |
| **4. Template Integration** | 2 | Snapshot capture/restore with stickies | Template preview | Slices 1, 2, 3 + Templates (prereq) |

### Recommended Execution Order

```
Slice 1 (Data Model) ──→ Slice 2 (Execution) ──→ Slice 3 (UI)
                                                      │
                                                      ↓
                                                Slice 4 (Templates)
```

Sequential — each slice builds on the previous. Slice 1 is pure data layer (fast). Slice 2 is the core execution work. Slice 3 makes it usable. Slice 4 extends it to templates.

---

## Edge Cases and Design Decisions

### What happens if a step's configuration changes but its output is sticky?

**Nothing automatic.** The user chose to sticky it — we respect that. The system does NOT auto-invalidate stickies when upstream inputs change, when the step's agent is modified, or when the workflow topology changes. The user is the curator.

However, the pre-run panel COULD show a warning icon: "This step's configuration has changed since this sticky was created (Run #7)." Implementation: compare the step's current config hash with the config hash at the time the sticky was created. This is informational only — the user decides whether to release it.

### What about for_each steps?

A for_each step iterates over an array and produces a `ForEachAggregateEnvelope`. The sticky is on the aggregate — the entire iteration result is frozen or open. Individual iteration items are not independently stickiable (too granular, and the iteration array may change between runs).

### What if a step is deleted from the workflow?

`ON DELETE CASCADE` on `workflow_stickies.step_id` handles this automatically. Removing a step removes its stickies.

### What if the same agent appears in multiple steps?

Stickies are scoped to `(step_id, artifact_type, artifact_id)`. An agent design sticky for Agent X in Step A is independent of Agent X in Step B. This is correct — the designer may produce different prompts for the same agent depending on the step's context.

### Cost and token reporting

Sticky-skipped steps contribute 0 input tokens, 0 output tokens, and $0 cost. The run summary should show: "Total cost: $X.XX (Y steps executed, Z steps from stickies)." This makes the cost savings from stickies visible and satisfying.

### Interaction with episodic memory (Agent System Slice 4)

Reflections should always be generated for the full run, including noting which steps used stickies. A reflection like "Steps 1-3 used stickies, Step 4 was freshly generated and produced high-quality output" is valid learning. Reflections are never stickied — they accumulate.

### Interaction with required reading (Agent System Slice 5)

If a required reading document changes, the compliance validation filter (Slice 5) should run against sticky outputs too — a sticky output might now violate updated conventions. The compliance check can flag this: "Sticky output from Run #7 has 2 convention violations against the updated [document name]." The user then decides whether to release the sticky and regenerate.

---

## Success Criteria

When this milestone is complete:

1. **Runs are incremental** — only the open pockets execute. Sticky artifacts skip regeneration entirely. Cost and time scale with the amount of actual work, not the total workflow size.

2. **The user curates their machine** — every artifact from every run can be reviewed, frozen, or released. The workflow progressively hardens as the user locks in the parts they're satisfied with.

3. **Partial execution is seamless** — a documenter with 3 sticky docs and 2 pending docs produces an output indistinguishable from a full run. Downstream steps don't know or care which docs were sticky.

4. **Templates carry stickies** — a template with pre-loaded artifacts is a deployable, production-ready pipeline. Instantiate it and only the open pockets need work.

5. **Stickies are visible** — the canvas shows sticky status at a glance, the pre-run panel shows the full machine state, the run view shows what was skipped vs executed, and the cost summary reflects only actual work done.
