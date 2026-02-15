# Workshop System — Master Ticket

## Context

The canvas IS the workshop — a permanent, mutable workbench where users configure, test, and iterate on their workflow. Real runs are separate, immutable executions against frozen "run templates." Run results are viewed in a dedicated separate page, never on the canvas.

The staged step execution system (Phase 0) has been built and is working. This document captures the larger vision for the workshop system that builds on top of it.

### Mental Model (Git Analogy)
- **Workshop** = git working tree (always mutable, always there)
- **Run Template** = git commit (frozen snapshot, promoted explicitly by user)
- **Run** = CI execution against a commit (immutable results)
- **Rebase** = git checkout (restore workshop from a past run's template)

### Key Design Decisions
1. **Single persistent workshop per workflow** — auto-created, not user-created. Users can rebase to any past run.
2. **Workshop = live database** — documents, agent configs, prompts, notes, topology are the workshop state.
3. **Run templates are explicit** — user "promotes" current workshop to a run template. Runs execute against templates, not live DB.
4. **Run results in separate viewer** — canvas stays clean as the workshop. No run results overlaid on canvas.
5. **Workshop outputs persist** — step-by-step execution results are the user's working material.
6. **Assistant sees workshop only** — no past run results. Reasons about definitions/structure, never quotes content as fact.
7. **Documents are functions** — content depends on input. Assistant understands definitions (what should this produce?), not content (what did it produce?).

---

## Dependency Graph

```
Phase 0 (DONE)  ──→  Staged step execution engine
  |
  ├──→ Phase 1  ──→  Workshop persistence (evolve staging into permanent workshop)
  |     |
  |     ├──→ Phase 2  ──→  Run templates (promote/freeze mechanism)
  |     |     |
  |     |     ├──→ Phase 3  ──→  Execution history (dedicated run viewer)
  |     |     |     |
  |     |     └──→ Phase 4  ──→  Rebase (restore workshop from past run template)
  |     |
  |     └──→ Phase 5  ──→  Assistant behavior (structure-aware, not content-factual)
```

---

## Phase 0: Staged Step Execution (COMPLETED)

The staging system is built and working:
- `POST /api/workflows/:id/staging` — create staging run
- `POST /api/workflows/:id/staging/:run_id/steps/:step_id/execute` — execute one step
- `GET /api/workflows/:id/staging/:run_id` — get staging run status
- DAG state reconstruction from content_version snapshots
- Step readiness checking with next_executable_steps guidance
- 7 unit tests passing, all integrated with existing DAG executors

---

## Phase 1: Workshop Persistence

Evolve staging into a permanent per-workflow workshop. Auto-create on first access. Workshop step outputs persist as the user's working material. Same staging engine underneath.

### What changes from current staging:
- Auto-create workshop on first access (no explicit "create staging run")
- Workshop has a permanent execution context per workflow (not ephemeral)
- Workshop step outputs persist as "current workshop state"
- Re-running a step overwrites its previous workshop output
- Downstream steps can use upstream workshop outputs as input (already works via staging DAG reconstruction)

### Backend changes needed:
- Workshop endpoint: GET-or-create workshop for a workflow
- Workshop step execution: reuse staging engine
- Workshop outputs: persist in content_versions tagged as "workshop"
- Remove ephemeral staging run creation (or wrap it)

---

## Phase 2: Run Templates

"Promote to run template" endpoint freezes current workshop state. "Run Workflow" reads from frozen template instead of live DB. Template = tagged collection of content_version snapshots.

### What gets frozen:
- Document definitions and content
- Agent configurations
- Prompt templates
- System prompts
- Assistant notes
- DAG topology (steps, edges, ports)

### Backend changes needed:
- POST /api/workflows/:id/promote-template — snapshot everything
- Template storage: collection of content_version snapshots with a template_id
- Modify run_workflow to read from template instead of live DB
- GET /api/workflows/:id/templates — list templates

---

## Phase 3: Execution History

Dedicated run history page (frontend). API for per-step results of any historical run. Drill-in to individual runs with per-step output cards.

### What exists today:
- Backend: list_workflow_executions (status, timestamps, aggregated outputs)
- Backend: getStepLastRun (per-step, but only LAST run)
- Backend: run_snapshots + content_versions (full audit trail per run)
- Frontend: "Last Run" tab on DynamicNode (shows most recent only)
- Frontend: Execution panel sidebar with run selector

### What's missing:
- API for per-step results of ANY historical run (not just last)
- Dedicated run history PAGE (not sidebar)
- Run detail view: drill into run → see per-step outputs, metrics, timing

---

## Phase 4: Rebase

"Rebase workshop to Run #N" restores live DB state from a run's template snapshots. Like git checkout — workshop becomes a copy of that run's template.

### How it works:
1. User views past run in execution history
2. Clicks "Rebase workshop to this run"
3. Backend reads the run's template snapshots
4. Overwrites live DB rows with snapshot values
5. Workshop now reflects the state from that template

### Safety: auto-snapshot current workshop before rebase.

---

## Phase 5: Assistant Behavior

Assistants understand they're preparing for the next run. Can read content to evaluate quality but never quote it or present it as fact. Structure-aware, not content-factual. Independent of Phases 2-4.

### Key principles:
1. Assistant CAN read document content (to evaluate quality/effectiveness)
2. Assistant NEVER quotes content or presents it as factual output
3. Assistant speaks ABOUT content, not FROM content
4. Documents are functions — content depends on input
5. Assistant is the "all-knowing expert for how information will be handled in the lifecycle of a run"

### What to change:
- System prompt updates in config/protocols/node_assistant/
- Tool descriptions in src/server/hub/strategies/chat/tools.rs
- Board overview prompt in src/server/hub/board_overview/mod.rs
