# Epic: File-First Data Transport

> Vision: [vision-file-first-transport.md](./vision-file-first-transport.md)
> The vision is the source of truth for design decisions, prompt formats, architecture rationale, and examples. This document is the implementation plan.

## Overview

Two parallel work streams converging into a unified system where agents run in containers with a shared POSIX workspace and designers thread narrative context through the workflow in step order.

```
Stream A: Design Pipeline          Stream B: Infrastructure
(application logic)                (ops / containers / filesystem)

A1  Handoff threading              B1  JuiceFS deployment
 │                                  │
A2  Designer prompt rewrite        B2  Container workspace mounts
 │                                  │
A3  complete_design + handoff      B3  OverlayFS layer
 │                                  │
A4  Re-design propagation          B4  Overlay diff + denylist filter
 │                                  │
A5  Agent prompt simplification    B5  Parallel merge (diff3 + Haiku)
 │                                  │
A6  Builder prompt rewrite          │
 │                                  │
A7  Re-design trigger (user edit)   │
 │                                  │
 └──────────┬───────────────────────┘
            │
         Converge
            │
 C1  Tool model transition
 C2  Wire workspace into agent execution
 C3  Run isolation + pinning
 C4  User file upload
 C5  Execution observability (handoff-as-story)
 C6  Migration + backward compat
```

Stream A can ship incrementally — each phase improves designer/agent output quality with the current system. Stream B is infrastructure with a longer lead time. They converge once both are ready.

---

## Stream A — Step-Order Design Pass

The designer currently runs per-node independently. Each node's designer has no knowledge of other nodes' designs. Stream A changes this to a sequential pass that threads handoff context from step to step.

### A1 — Handoff Threading

Add previous step's handoff and next step's box text to the designer's context. Change designer dispatch from parallel-per-node to sequential-in-step-order.

**What changes:**
- New struct `PreviousStepHandoff { step_name, handoff_description }` in designer input
- `DesignerInput` gets `previous_step_handoff: Option<PreviousStepHandoff>` and `next_step_text: Option<String>`
- `build_workforce_designer_input()` populates from step metadata + stored designer outputs
- `pipeline/designer.rs` dispatch loop runs in topological order, passing each step's handoff to the next

**Depends on:** nothing — first move

### A2 — Designer Prompt Rewrite

Update the designer's prompt templates to use the new context and guide outward-focused `expected_output`.

**What changes:**
- `react_prompt.md`: rename `dispatch_instruction` → `task`, `upstream_topology` → `step_order`, add `{{previous_step}}` and `{{next_step}}` template vars
- `react_prompt.md`: add `{{current_design_handoff}}` var — shows the designer its OWN previous handoff so it can decide during re-design whether to update or keep it. Required for A4 to work. See vision §"Updated Designer Instruction Template"
- `react_system.md`: add guidelines for writing orientation handoffs, examples showing previous step reference in assignments, next step awareness in expected_output
- See vision §"Updated Designer Prompt" and §"Updated Designer Example" for exact formats

**Depends on:** A1

### A3 — `complete_design` Tool + Step Handoff

The designer writes a step-level handoff description when completing design. This is what the next step's designer sees.

**What changes:**
- `complete_design` tool schema gets `step_handoff: Option<String>`
- Handoff persisted as step metadata (new column or store artifact)
- Step-level handoff distinct from per-agent `expected_output` — see vision §"Step-Level Expected Output"

**Depends on:** A1, A2

### A4 — Re-Design Propagation

When a user edits a step's design and its handoff changes, cascade re-design forward through the workflow. Stop when a step absorbs the change without updating its own handoff.

**What changes:**
- After designer completes, compare new handoff to previous handoff
- If changed: trigger next step's designer with updated `<previous_step>`
- Designer's existing verify-and-skip pattern handles the "no change needed" case naturally
- Propagation continues until a step calls `complete_design` without updating handoff

**Depends on:** A3

### A5 — Agent Prompt Simplification

Collapse the agent prompt from 7 XML blocks to 3. The workspace (Stream B) replaces most injected context, but even without it, the handoff-oriented prompts are a strict improvement.

**What changes:**
- Refactor `TaskPromptBuilder` in `pipeline/agent_executor.rs`
- Remove: `upstream_artifacts`, `previous_agent_outputs`, `upstream_step_outputs`, `user_notes`
- Keep: `<previous_step>` (handoff text), `<assignment>`, `<expected_output>`
- System prompt becomes short role + perspective — see vision §"File-First Agent Prompt"
- Unify within-workforce agent handoffs: prior agent output uses `<previous_step>` (same tag as cross-step handoff) — see vision §"What Each Block Does"

**Important: `<previous_step>` has two contexts.** At design time (in the designer's prompt), it's the handoff *description* — what the step WILL produce. At execution time (in the agent's prompt), it's the *actual text output* of whoever ran before — shaped by `expected_output`. `TaskPromptBuilder` populates the runtime version from the actual step/agent output, not from the design-time handoff. See vision §"What the Next Step Receives" (line 546-552).

**Depends on:** A1-A3 (needs handoff data to populate `<previous_step>`)

### A6 — Builder Prompt Rewrite

Update the builder's mental model from "data routing" to "scheduling." The builder stops thinking about what data flows between steps and starts thinking about what work happens in what order.

**What changes:**
- Builder system prompt (`config/archetype/workforce/builder/system.md`) gets workspace context: agents run in containers with shared `/workspace/`, shell access and web search are always available
- Capabilities list narrows to things the shell can't do (external integrations, APIs)
- Builder receives `<previous_step>` handoff (already wired in A1) — add guidance: "Read what the previous step will hand off to understand what's coming in"
- Remove data-routing language from builder instructions
- See vision §"Builder — From Routing to Scheduling" for the exact mental model shift

**Depends on:** A1 (needs handoff threading in place)

### A7 — Re-Design Trigger (User Edit)

When a user edits a step's design in the config panel (changes roster, modifies node text, rewrites system prompt), detect the change and trigger re-design for that step + propagation forward.

**What changes:**
- Frontend: config panel edit submits changes to API
- Backend: API endpoint detects meaningful edit (roster change, node text change) vs. cosmetic edit
- On meaningful edit: re-trigger builder + designer for the edited step
- After designer completes: A4's propagation logic cascades forward if handoff changed
- See vision §"Re-Design Propagation" for the trigger description

**Depends on:** A4 (needs propagation mechanism)

---

## Stream B — Infrastructure

Real POSIX filesystem backed by JuiceFS. Agents run in containers with the workspace mounted. OverlayFS isolates writes per step.

### B1 — JuiceFS Deployment

Stand up JuiceFS using existing Postgres (metadata) and S3/MinIO (data). Validate FUSE mounts work in the target deployment environment.

**What to figure out:**
- Deployment target: Kubernetes CSI driver vs Docker Compose FUSE mount
- JuiceFS format and mount configuration
- Integration with existing `system_store` service or parallel path
- Performance validation with typical file sizes and counts

**Depends on:** nothing — can start immediately in parallel with Stream A

### B2 — Container Workspace Mounts

Extend existing `ContainerConfig` to mount JuiceFS at `/workspace/`. The container execution layer (`src/execution/container/`) already exists with full Docker lifecycle management.

**What changes:**
- `ContainerConfig` gets workspace mount configuration
- Container creation adds JuiceFS bind mount at `/workspace/`
- Env vars for package manager redirection (pip, npm, cargo → `/tmp/`) — see vision §"Container Model"

**Depends on:** B1

### B3 — OverlayFS Layer

JuiceFS as read-only lower layer, writable OverlayFS upper per container. Agent sees merged view, writes go to local overlay at native speed.

**What changes:**
- Container setup creates overlay: JuiceFS lower (read-only) + local upper (writable)
- Agent sees single merged `/workspace/`
- Writes hit local disk (fast), reads fall through to JuiceFS

**Depends on:** B2

### B4 — Overlay Diff + Denylist Filter

On step completion, diff the overlay against the base. Filter out junk (`.git/`, `node_modules/`, `__pycache__/`, etc). Persist clean files back to JuiceFS.

**What changes:**
- Post-step hook: walk OverlayFS upper directory
- Apply denylist filter (static config, `.gitignore`-style patterns) — see vision §"Container Model" for full list
- Persist surviving files to JuiceFS
- Tear down container + overlay

**Depends on:** B3

### B5 — Parallel Merge

When parallel steps both modify the workspace, merge their clean diffs. Auto-merge for non-conflicting changes, Haiku for conflict hunks.

**What changes:**
- After parallel batch completes: compare clean diffs from each step's overlay
- New files / single-step modifications: accept directly
- Multi-step modifications to same file: `diff3` three-way merge
- Conflict hunks → Haiku call (~20 lines context per hunk) — see vision §"Merge Strategy"
- Binary file policy: last-write-wins or keep-both

**Depends on:** B4

---

## Convergence

Once both streams are ready, wire them together.

### C1 — Tool Model Transition

Rework how agents get tools. Shell access becomes implicit. Store tools transition to backward compat. The designer's `tools` list becomes almost always empty.

**What changes:**
- Shell access (ls, cat, grep, python, pip, npm, curl) is always available in containers — never listed by designer
- Web search / X search remain model-native — never listed by designer
- Store tools (`store_read_file`, `store_write_file`) remain available alongside filesystem for backward compat
- Capability registry narrows: only contains things the shell and model can't do (external integrations, APIs, domain-specific tools)
- Designer `tools` field is almost always `[]` — see vision §"Tool Model"

**Depends on:** B2 (needs containers with shell access)

### C2 — Wire Workspace into Agent Execution

Connect the simplified agent prompts (Stream A) to the real filesystem (Stream B). Agents execute in containers with `/workspace/` mounted, prompts reference the filesystem, handoffs thread through.

**What changes:**
- Agent execution strategy uses container with workspace mount
- System prompt includes workspace grounding: "Your working directory is /workspace/ where other steps have contributed and will contribute after you." — see vision §"Agent Prompt Pattern"
- Execution envelopes carry metadata only; workspace carries data
- DAG edges become scheduling only; workspace handles data propagation

**Depends on:** A5, B4, C1

### C3 — Run Isolation + Pinning

Fresh workspace per run. Pinned step files pre-loaded from sealed state.

**What changes:**
- Run start: create empty JuiceFS volume (or clear workspace path)
- Pinned steps: pre-load sealed files into fresh workspace, replay envelope with zero tokens
- Run end: workspace teardown (or archive for debugging)
- Extend existing pin system (`workshop/context.rs`) to work with workspace files

**Depends on:** C2

### C4 — User File Upload

Users can upload files to the workspace before or during a run — reference data, seed files, configuration.

**What changes:**
- API endpoint for uploading files to a workflow's workspace
- Frontend UI for file upload (drag-and-drop or file picker)
- Uploaded files appear in `/workspace/` alongside agent-produced files
- Agents never see the user's original prompt or chat messages — only the designer's assignment and workspace files

**Depends on:** C2 (needs workspace infrastructure)

### C5 — Execution Observability (Handoff-as-Story)

The handoff text becomes the per-step observability layer. The UI shows each step's text output as a readable narrative — what was done, what was produced, where it lives.

**What changes:**
- Execution output view shows step handoffs as story paragraphs
- No need to track which files each step created — the handoff tells the story
- Click into workspace to browse actual files
- See vision §"Observability" for the display format

**Depends on:** C2 (needs handoff-oriented execution output)

### C6 — Migration + Backward Compat

Existing workflows transition to file-first. Decide on feature flag vs. hard cutover.

**What to figure out:**
- Do existing workflows need to keep working during transition?
- Can store tools coexist with filesystem access? (Vision says yes — backward compat)
- Migration path for existing designer configs (old `expected_output` format → new)

**Depends on:** C2

---

## Open Questions

1. **Sequential designer latency** — Running designers in step order adds wall-clock time. For a 10-step workflow, that's 10 sequential designer calls. Acceptable? Or does the design pass need a "parallel where possible, sequential where handoff matters" hybrid?

2. **JuiceFS deployment target** — Kubernetes CSI driver vs Docker Compose FUSE? Shapes B1-B3 significantly.

3. **OverlayFS in containers** — Does the container runtime allow OverlayFS-in-OverlayFS? May need host-level setup or privileged containers.

4. **Workspace size limits** — Agents can write unbounded data. Need a per-run or per-step quota?

5. **Parallel step ordering within merge** — When merging parallel diffs, does the order matter? If step A and B both create `README.md`, which is "base" for diff3?

6. **Store tools end-of-life** — Vision says store tools remain for backward compat. When do they get removed? Or do they become a thin wrapper over workspace files?

7. **Re-design trigger granularity (A7)** — What counts as a "meaningful edit" that triggers re-design? Roster changes and node text changes clearly qualify. What about system prompt tweaks? Capability additions? Need to define the boundary between "user polish" and "structural change."

8. **Capability registry scope (C1)** — How much of the current capability registry survives? The vision says tools list is almost always empty. Do we keep the registry for the rare integrations, or replace it entirely?
