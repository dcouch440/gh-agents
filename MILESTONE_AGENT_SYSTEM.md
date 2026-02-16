# Milestone: Agent System Upgrade

**Reference:** [research/PROTOCOL_DESIGN.md](research/PROTOCOL_DESIGN.md) — full research and design rationale
**Vision:** [VISION.md](VISION.md) — the destination

This milestone transforms nexor's agent system from capable execution into a governed, self-improving, user-controlled workshop. Each slice delivers end-to-end value and can be shipped independently.

---

## Prerequisites (Currently In Progress)

The template system is functionally complete on the backend (snapshot capture, restore, content versioning, run detail API). What remains before this milestone begins:

- [ ] Template management UI (frontend pages for list/create/delete templates)
- [ ] Content snapshot viewer (UI to view versioned prompts/outputs per run)

These are in-flight and should complete before Slice 1.

---

## Slice 1: Sub-DAG Execution Mode

**Goal:** Enable workflow steps to execute entire sub-workflows, unlocking nested composition and designer-created micro-pipelines.

**Why first:** This is the architectural foundation. Sub-DAGs let the designer decompose complex tasks into reusable workflow templates. Every subsequent slice benefits from the ability to compose protocols into larger flows.

### Ticket 1.1: `sub_workflow` Execution Mode — Backend

**Scope:** Add `sub_workflow` as a new execution mode alongside single, for_each, documenter, task_force, room, belief_capture.

**Work:**
- Add `sub_workflow_template_id` column to `workflow_steps` table (nullable UUID, references `run_templates.id`)
- Add `sub_workflow` branch in `run_dag_loop()` (in `src/server/hub/dag/mod.rs`)
- Create `src/server/hub/dag/sub_workflow/mod.rs`:
  - Resolve port inputs from parent step (same as any step)
  - Load the referenced template snapshot
  - Map parent port inputs to child workflow's initial `var_outputs`
  - Create child `WorkflowExecutionRow` linked to parent execution
  - Call `execute_workflow_via_engine()` with the child workflow context
  - Capture `WorkflowExecutionResult`
  - Wrap in `StepExecutionEnvelope` (same as any step)
  - Record in parent's `DagExecutionState`
- Add `sub_workflow` to template snapshot capture/restore (in `templates/mod.rs` and `templates/restore.rs`)
- Handle cancellation token propagation to child workflow
- Handle error propagation — child failure should fail the parent step

**Acceptance:**
- A workflow step with `execution_mode = "sub_workflow"` and a valid `sub_workflow_template_id` executes the referenced template
- Port inputs flow into the child workflow as initial variables
- Child workflow outputs are accessible to downstream parent steps via normal port routing
- Child workflow execution is recorded as a separate `WorkflowExecutionRow` linked to the parent
- Cancelling the parent cancels the child
- Tests: `cargo test hub::dag::tests::sub_workflow`

### Ticket 1.2: Sub-DAG WebSocket Events

**Scope:** Broadcast nested events so the UI can show sub-workflow execution inside the parent step.

**Work:**
- Add `WorkflowEventKind::SubWorkflowStarted { parent_step_id, child_execution_id, total_steps }`
- Add `WorkflowEventKind::SubWorkflowCompleted { parent_step_id, child_execution_id, status }`
- Child step events should include `parent_step_id` context so the frontend can nest them
- Broadcast on the parent workflow's channel (the UI is already listening there)

**Acceptance:**
- Frontend receives sub-workflow events nested within the parent step's execution
- Events include enough context to render a nested execution view

### Ticket 1.3: Sub-DAG UI — Canvas + Run Detail

**Scope:** Let users wire sub-workflows into their DAG on the canvas, and view nested execution in run detail.

**Work:**
- Canvas: New node type for sub_workflow steps (shows template name, port connectors)
- Step config panel: Template picker (dropdown of available templates for this workflow)
- Port mapping UI: Connect parent ports to child workflow entry/exit points
- Run detail: Nested execution view — expand a sub-workflow step to see its internal steps
- Run detail: Link to child execution's full run detail page

**Acceptance:**
- User can drag a sub-workflow step onto the canvas
- User can select a template to reference
- User can wire port connections in and out
- Run detail shows sub-workflow step with expandable nested view
- Clicking through opens the child run's detail page

### Ticket 1.4: Sub-DAG Node Assistant Support

**Scope:** The node assistant knows how to configure sub-workflow steps.

**Work:**
- Add archetype block for sub_workflow in `config/protocols/node_assistant/`
- Node assistant tools: `set_sub_workflow_template(template_id)` or similar
- Assistant can list available templates, suggest which one to use, configure port mapping
- Add to board_overview rendering so assistant sees sub-workflow steps

**Acceptance:**
- User can ask the assistant to "wire in [template name] as a sub-step"
- Assistant can configure sub-workflow steps through tool calls

---

## Slice 2: Agent Designer Governance Upgrade

**Goal:** Teach the Agent Designer the governance and execution pattern research so it propagates to every agent it designs.

**Why second:** This is the highest-leverage change. Every task force, documenter, and room execution flows through the designer. Upgrading the designer upgrades everything downstream.

### Ticket 2.1: New BOCA Beliefs for Governance

**Scope:** Add 8 new beliefs to the Agent Designer's system prompt encoding findings from AGENT_GOVERNANCE.md and AGENT_EXECUTION_PATTERNS.md.

**Work:**
- Edit `config/protocols/agent_designer/designer/system.md`
- Add the 8 beliefs specified in PROTOCOL_DESIGN.md Section 3:
  - `[scope_boundaries | 0.85]`
  - `[instruction_as_checklist | 0.85]`
  - `[convention_citation | 0.80]`
  - `[self_assessment | 0.75]`
  - `[narrative_handoff | 0.80]`
  - `[required_reading | 0.85]`
  - `[refusal_over_guessing | 0.80]`
  - `[decision_tracing | 0.75]`
- Update the `<what_you_produce>` section to reference new beliefs in the system prompt and task prompt generation guidelines

**Acceptance:**
- Agent Designer generates prompts that include scope boundaries, convention citation instructions, and self-assessment requests
- Run a task force with the new designer and verify generated prompts contain governance patterns
- No regression in existing prompt quality

### Ticket 2.2: Enhanced Archetype Guidance

**Scope:** Update the archetype guidance blocks that the designer receives per protocol type.

**Work:**
- Update `config/protocols/node_assistant/task_force/block.md` — add `<archetype_designer>` section with governance-specific guidance (scope control, required reading distribution, handoff quality, decision tracing, self-assessment)
- Update `config/protocols/node_assistant/documenter/block.md` — add guidance about writer quality standards, convention adherence
- Update `config/protocols/node_assistant/room/block.md` — add guidance about personality differentiation, grounded debate, anti-sycophancy
- These blocks feed into the designer via `{{.Designer.archetype_guidance}}`

**Acceptance:**
- Task force designer prompts include scope boundaries and decision tracing for each agent
- Documenter designer prompts include quality standards for writers
- Room designer prompts include personality differentiation guidance

---

## Slice 3: Protocol Prompt Enhancement

**Goal:** Upgrade the static protocol prompts to implement research findings directly.

**Why third:** These are config-only changes (no Rust code) that immediately improve agent output quality. Quick wins after the designer is upgraded.

### Ticket 3.1: Enhanced Documenter Writer Prompt

**Scope:** Replace the one-line writer system prompt with a full quality-standards prompt.

**Work:**
- Edit `config/protocols/documenter/writer/system.md`
- Add identity, quality standards, and audience sections as specified in PROTOCOL_DESIGN.md Section 5
- Key additions: documents are for AI consumption, must be specific/structured/actionable/scoped, include correct AND incorrect examples for conventions

**Acceptance:**
- Documenter writer produces more structured, convention-compliant documents
- Documents include examples of correct and incorrect patterns
- A/B comparison: run documenter with old vs new prompt, assess quality difference

### Ticket 3.2: Enhanced Documenter Strategist Guidance

**Scope:** Improve the strategist's prompt to produce better research plans and writer instructions.

**Work:**
- Edit `config/protocols/documenter/strategist/prompt.md` — add strategy principles section
- Guide strategist to direct researchers to discover (not assume), give writers specific structural instructions, handle required reading dependencies

**Acceptance:**
- Strategy output includes more specific writer instructions (document structure, section requirements)
- Research plans reference required reading documents when available

### Ticket 3.3: Enhanced Task Force Agent Prompt

**Scope:** Upgrade the task force agent system prompt with governance patterns.

**Work:**
- Edit `config/protocols/task_force/agent/system.md`
- Add authority section (L4 autonomy, scope boundaries, refusal criteria)
- Add required reading section (read docs first, cite conventions)
- Add output requirements (decisions + reasoning, confidence, out-of-scope findings)
- Restructure to separate identity/authority/required_reading/mission/upstream_context/output
- See PROTOCOL_DESIGN.md Section 6 for full template

**Acceptance:**
- Task force agents produce output that includes decision reasoning and confidence assessments
- Agents report out-of-scope findings separately from their primary deliverable
- Agents that have document_read cite required reading in their output

### Ticket 3.4: Enhanced Room Member System Prompt Pattern

**Scope:** Create a room member prompt pattern with personality, grounding, and disagreement handling.

**Work:**
- This is generated by the Agent Designer, not static config — so the enhancement is in the designer's beliefs (Ticket 2.1) and archetype guidance (Ticket 2.2)
- Additionally, update the meeting gatekeeper prompt (`config/protocols/meeting/gatekeeper/system.md`) with enhanced speaker selection criteria:
  - Prioritize diverse perspectives over agreeing voices
  - Evidence-holders speak before general-knowledge agents
  - Build on disagreement for productive dialogue

**Acceptance:**
- Room discussions produce more diverse perspectives (fewer unanimous agreements)
- Gatekeeper selects speakers that create productive dialogue
- Room agents reference beliefs in their arguments

---

## Slice 4: Episodic Memory System

**Goal:** Enable agents to learn from past runs by storing reflections and injecting them as context in future runs.

**Why fourth:** This is the "self-improving" capability. Once protocols can remember what worked and what didn't, quality improves run over run without manual prompt tuning.

### Ticket 4.1: Episodic Memory Schema

**Scope:** Create the database table for storing run reflections.

**Work:**
- New migration: `run_reflections` table
  ```sql
  CREATE TABLE run_reflections (
      id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
      workflow_execution_id UUID NOT NULL REFERENCES workflow_executions(id),
      workflow_id UUID NOT NULL REFERENCES workflows(id),
      step_id UUID REFERENCES workflow_steps(id),
      reflection_type TEXT NOT NULL,  -- 'documenter' | 'task_force' | 'room' | 'single' | 'workflow'
      task_tags TEXT[] DEFAULT '{}',
      outcome TEXT NOT NULL,  -- 'success' | 'partial' | 'failure'
      what_worked TEXT,
      what_didnt TEXT,
      key_learning TEXT,
      convention_violations JSONB DEFAULT '[]',
      per_agent_assessment JSONB,  -- { "agent_name": { "quality": 4, "note": "..." } }
      tokens_used INTEGER,
      duration_ms INTEGER,
      created_at TIMESTAMPTZ DEFAULT NOW()
  );
  CREATE INDEX idx_run_reflections_workflow ON run_reflections(workflow_id);
  CREATE INDEX idx_run_reflections_type ON run_reflections(reflection_type);
  CREATE INDEX idx_run_reflections_tags ON run_reflections USING gin(task_tags);
  ```
- Add `RunReflectionRow` to `src/db/mod.rs`
- Add repository functions: `insert_run_reflection()`, `get_reflections_by_workflow()`, `get_reflections_by_type_and_tags()`

**Acceptance:**
- Run reflections can be stored and queried by workflow, type, and tags
- Schema supports per-agent assessments for task force runs
- GIN index enables tag-based similarity matching

### Ticket 4.2: Reflection Generation

**Scope:** After each protocol execution, generate a structured reflection.

**Work:**
- Add a post-execution step in each protocol's execution path:
  - `documenter/mod.rs` — after all phases complete, assess strategy/research/write quality
  - `task_force/mod.rs` — after all agents complete, assess per-agent quality
  - `room_step/mod.rs` — after discussion ends, assess discussion quality
- Reflection generation can be:
  - **Simple (v1):** Heuristic-based — did all phases succeed? Were token budgets exceeded? Were there retries?
  - **Advanced (v2):** LLM-generated — a lightweight model summarizes what happened and produces the reflection
- Store reflection via `insert_run_reflection()`
- Tag reflections based on the step's context (mission type, document types, agent roles)

**Acceptance:**
- Every protocol execution produces a run reflection stored in the DB
- Reflections capture outcome, what worked, what didn't, and key learnings
- Task force reflections include per-agent quality assessments

### Ticket 4.3: Reflection Retrieval and Injection

**Scope:** Before each protocol execution, retrieve relevant past reflections and inject them as context.

**Work:**
- In each protocol's pre-execution setup:
  - Query `run_reflections` for the top 3 most similar past runs (by workflow_id + reflection_type + tags)
  - Format reflections as a `<past_experience>` block
  - Inject into the protocol's context (for documenter: strategist prompt; for task force: designer context; for room: gatekeeper context)
- Similarity matching: start with workflow_id + type match, then tag overlap scoring
- Limit to reflections from last 30 days (relevance decay)

**Acceptance:**
- Protocols receive relevant past run reflections as context
- Reflections influence behavior: documenter adjusts strategy based on past failures, task force avoids known pitfalls
- Old reflections (>30 days) are excluded

### Ticket 4.4: Episodic Memory API + UI

**Scope:** Let users view and manage run reflections.

**Work:**
- API: `GET /api/workflows/:id/reflections` — list reflections for workflow
- API: `GET /api/workflows/:wid/executions/:eid/reflection` — get reflection for specific run
- API: `DELETE /api/workflows/:wid/reflections/:rid` — delete a stale reflection
- UI: Add reflection panel to run detail page (expandable section showing what the system learned)
- UI: Add reflections tab to workflow settings (list of all reflections, delete stale ones)

**Acceptance:**
- User can see what the system learned from each run
- User can delete reflections that are no longer relevant
- Reflection data is visible in the run detail page

---

## Slice 5: Required Reading Pipeline Enhancement

**Goal:** Make required reading actually stick — agents cite conventions, and compliance is verified.

**Why fifth:** By this point, the documenter produces better docs (Slice 3), the designer distributes them (Slice 2), and agents can learn from past compliance failures (Slice 4). This slice closes the loop with verification.

### Ticket 5.1: Convention Citation in Agent Output

**Scope:** The assistant assigns required reading, the designer distributes it with citation instructions.

**Responsibility chain:**
- **Assistant** is the librarian — it reads documents, understands what matters, and records document IDs in its notes under `## Required Reading`. The assistant has the domain context to know which conventions apply to this node's work.
- **Designer** is the syllabus printer — it sees Required Reading in the assistant's notes and generates prompts that instruct agents to `read_document(document_id)` before starting, and to cite what they reference.
- **Agents** are the students — they read the documents at runtime and cite specific sections in their output.

**Work:**
- This is mostly covered by Ticket 2.1 (`[convention_citation | 0.80]` and `[required_reading | 0.85]` beliefs)
- Additional: when the designer sees Required Reading in the notes, it should generate a specific instruction block:
  ```
  After reading the required documents, reference specific sections
  in your output: "Per [document name] section [X]: [convention] → [what I did]"
  ```
- Update the designer's `<what_you_produce>` section to mandate citation instructions when required reading exists
- Enhance the node assistant's `<required_reading_behavior>` so that when the user shares a document, the assistant reads it, confirms understanding, and records it with a note about why it matters for this node

**Acceptance:**
- Assistant records required reading with context about why each document matters
- Designer generates prompts that instruct agents to read and cite conventions
- Task force agents with required reading produce output that cites specific convention sections
- Citations are traceable to actual document content

### Ticket 5.2: Compliance Validation Filter

**Scope:** Add a post-execution filter that checks agent output against required reading conventions.

**Work:**
- New execution filter: `ConventionComplianceFilter` in `src/server/hub/engine/filters/`
- After agent execution, if required reading documents exist:
  - Load the relevant convention documents
  - Use a lightweight LLM call to check output against conventions
  - Produce a compliance score (0-100) and list of violations
  - Store compliance results in `protocol_executions` or a new table
- If compliance score is below threshold, flag for assistant review (don't block execution)
- Configuration: compliance check is opt-in per step (some steps don't have conventions)

**Acceptance:**
- Agent output is checked against required reading conventions
- Compliance score and violations are stored and visible in run detail
- Low compliance scores are flagged for assistant/user review
- Compliance check does not block execution (informational, not gating)

---

## Slice 6: Belief Pipeline Enhancement

**Goal:** Improve belief quality so rooms receive high-fidelity compressed context.

### Ticket 6.1: Confidence Calibration for Chat Beliefs

**Scope:** Enhance the chat belief extractor to produce accurately calibrated confidence levels.

**Work:**
- Edit `config/protocols/chat_belief_extraction/system.md`
- Add confidence calibration section as specified in PROTOCOL_DESIGN.md Section 8:
  - HIGH: User directly stated this
  - MEDIUM: User implied through discussion
  - LOW: Inferred from context
- Add instruction: "Do not inflate confidence. A medium-confidence belief that accurately reflects its uncertainty is more valuable than a high-confidence belief that overstates what the user actually said."

**Acceptance:**
- Chat beliefs have more accurately calibrated confidence levels
- Implicit beliefs are tagged medium, not high
- Inferred beliefs are tagged low, not medium

### Ticket 6.2: Causal Chain Preservation for Runtime Beliefs

**Scope:** Enhance the runtime belief extractor to preserve relationships, not just facts.

**Work:**
- Edit `config/protocols/belief_capture/extractor/system.md`
- Add extraction quality section emphasizing causal chains:
  - BAD: "A security vulnerability was found."
  - GOOD: "The auth endpoint (/api/v1/auth/login) is vulnerable to timing attacks because..."
  - Pattern: WHAT → WHERE → WHY → IMPACT
- Add source attribution requirement: which agent, which step, what evidence

**Acceptance:**
- Runtime beliefs preserve causal relationships
- Each belief traces back to its source agent and step
- Room agents receive beliefs with enough context to have informed discussions

### Ticket 6.3: Belief Quality Dashboard

**Scope:** UI to view and manage beliefs across the board.

**Work:**
- Belief explorer panel: view all beliefs for a workflow/board, filterable by:
  - Source type (chat vs runtime)
  - Confidence level
  - Tags
  - Source node
  - Belief type (fact, opinion, requirement, etc.)
- Superseded belief indicator: show which beliefs have been replaced
- Cross-source tension view: highlight contradictions between nodes
- Belief detail: view the source conversation/output that produced each belief

**Acceptance:**
- User can see all beliefs at a glance, filter by any dimension
- Superseded beliefs are clearly marked
- Contradictions are highlighted for resolution

---

## Slice 7: Assistant Run Observation

**Goal:** The assistant can observe runs, take notes, grade output, and communicate with the user during execution.

**Why seventh:** This is the vision's core feature — the assistant as the all-knowing workshop expert. It depends on everything above: sub-DAGs give it more to observe, episodic memory lets it learn, beliefs give it context, governance gives it the right posture.

### Ticket 7.1: Run Observation Tools for Node Assistant

**Scope:** Give the node assistant tools to trigger and observe protocol executions.

**Work:**
- New tools for the node assistant:
  - `execute_step(step_id)` — trigger a single step execution and observe it
  - `get_step_output(step_id, execution_id)` — view the output of a completed step
  - `get_run_status(execution_id)` — check current execution status
  - `grade_output(step_id, execution_id, score, notes)` — record the assistant's quality assessment
- Tools should return structured summaries, not raw data (context budget management)
- Execution observation: the assistant receives step completion events while the run is in progress

**Acceptance:**
- Node assistant can trigger a step execution from the chat interface
- Node assistant can view step outputs after completion
- Node assistant can record quality grades with notes
- Grades are stored and visible in run detail

### Ticket 7.2: Mid-Run User Messaging

**Scope:** The user can send messages to the assistant during a workflow execution, and the assistant responds between steps.

**Work:**
- Polling mechanism: between DAG steps, check for pending user messages in the node assistant's chat
- If messages exist: pause DAG execution, route messages to the assistant, let it respond, then resume
- The assistant can use its observations and run context to answer questions about what's happening
- Add a "run in progress" indicator to the assistant chat showing which step is executing
- User messages during a run should include the current execution context (which step just completed, what's next)

**Acceptance:**
- User can type messages during a workflow run
- Assistant responds between steps (not mid-step)
- Assistant has context about the current run state when responding
- Run resumes after the assistant responds

### Ticket 7.3: Run Observation Notes

**Scope:** The assistant takes notes during runs and updates them for future runs.

**Work:**
- Extend the `update_notes` tool to support a "## Run Observations" section
- During/after a run, the assistant can record:
  - What it observed about agent behavior
  - Quality issues or convention violations it noticed
  - Suggestions for improving the workflow
- Run observations feed into the next run as context (separate from episodic memory — these are the assistant's personal notes)
- Notes persist across sessions (already true for existing notes)

**Acceptance:**
- Assistant records structured observations during runs
- Observations are available in subsequent runs as assistant context
- Run observation notes are separate from configuration notes

### Ticket 7.4: Assistant Grading UI

**Scope:** Display the assistant's quality grades in the run detail view.

**Work:**
- Run detail page: show assistant grade per step (if graded)
- Grade format: score (1-5) + notes
- Historical grades: trend line showing quality over time for a workflow
- Filter runs by grade: find all runs where the assistant flagged low quality

**Acceptance:**
- Run detail shows assistant grades per step
- User can see quality trends over time
- Low-quality runs are easy to find and review

---

## Slice 8: Decision Tracing

**Goal:** Every agent decision is traceable — from mission objective through each agent's reasoning to final output.

### Ticket 8.1: Decision Record Schema

**Scope:** Add structured reasoning capture to agent execution.

**Work:**
- Add `reasoning` column to `agent_executions` table (TEXT, nullable)
- Add `confidence` column to `agent_executions` table (FLOAT, nullable)
- Add `convention_references` column to `agent_executions` table (TEXT[], nullable)
- When parsing structured output from agents (in `run_step_via_engine()`), extract:
  - `reasoning` field if present in the output
  - `confidence` field if present
  - `convention_references` if present
- Store these alongside the existing execution record

**Acceptance:**
- Agent executions that include reasoning/confidence fields have them stored in the DB
- Fields are queryable for audit and debugging

### Ticket 8.2: Decision Trace View in Run Detail

**Scope:** Display the decision chain across agents in the run detail page.

**Work:**
- Run detail: for each step, show reasoning, confidence, and convention references (if available)
- Cross-step trace: visual flow showing how each agent's output influenced the next agent's decisions
- Convention reference links: click a convention reference to see the source document
- Confidence heatmap: color-code steps by confidence (green = high, yellow = medium, red = low)

**Acceptance:**
- User can trace decisions across the full DAG execution
- Convention references link to source documents
- Low-confidence steps are visually highlighted

### Ticket 8.3: Trace-Based Debugging

**Scope:** When a run produces unexpected output, the user can trace back to the root cause.

**Work:**
- "Why did this happen?" view: select any output and trace backwards through the DAG:
  - What was the input to this step?
  - What did the agent's reasoning say?
  - What upstream outputs influenced this?
  - What conventions were (or weren't) cited?
- Diff view: compare two runs of the same workflow to see where they diverged
- Export trace: download a complete execution trace as JSON for external analysis

**Acceptance:**
- User can trace any output back to its root inputs and reasoning
- Run comparison shows where executions diverged
- Complete traces are exportable

---

## Slice Summary

| Slice | Tickets | Backend | Frontend | Config | Depends On |
|-------|---------|---------|----------|--------|------------|
| **1. Sub-DAG** | 4 | Migration + execution mode + events | Canvas node + run detail nesting | Node assistant archetype | Templates complete |
| **2. Designer Upgrade** | 2 | None | None | 8 new beliefs + archetype guidance | None |
| **3. Protocol Prompts** | 4 | None | None | Writer, strategist, task force, gatekeeper | Slice 2 (designer should distribute) |
| **4. Episodic Memory** | 4 | Migration + generation + retrieval | Reflection panel + management | None | None |
| **5. Required Reading** | 2 | Compliance filter | Compliance display in run detail | Designer citation belief | Slices 2, 3 |
| **6. Belief Pipeline** | 3 | None | Belief explorer dashboard | Chat + runtime extractor prompts | None |
| **7. Assistant Observation** | 4 | Observation tools + polling | Run indicator + grading UI | Notes structure | Slices 1-6 (benefits from all) |
| **8. Decision Tracing** | 3 | Migration + extraction | Trace view + debugging | None | Slice 3 (agents must produce reasoning) |

### Recommended Execution Order

```
Slice 2 (Designer) ──→ Slice 3 (Prompts) ──→ Slice 5 (Required Reading)
                                                      │
Slice 1 (Sub-DAG) ────────────────────────────────────┤
                                                      │
Slice 4 (Episodic Memory) ───────────────────────────┤
                                                      │
Slice 6 (Belief Pipeline) ───────────────────────────┤
                                                      ↓
                                               Slice 7 (Assistant)
                                                      │
                                                      ↓
                                               Slice 8 (Tracing)
```

Slices 1, 2, 4, and 6 can start in parallel. Slice 3 depends on 2. Slice 5 depends on 2+3. Slice 7 benefits from everything. Slice 8 depends on agents producing reasoning (Slice 3).

---

## Success Criteria

When this milestone is complete:

1. **The assistant is the expert advisor** — it observes runs, grades output, takes notes, and talks to the user during execution. It never oversteps.

2. **Agents follow orders** — required reading is consumed, conventions are cited, compliance is measured. The 27% instruction success rate from AgentIF is mitigated through checklists, structural enforcement, and post-execution validation.

3. **The designer propagates research** — every generated prompt includes governance patterns (scope boundaries, decision tracing, self-assessment). The research library is not shelf-ware — it's actively distributed through the system.

4. **The system learns** — episodic memory captures what worked and what didn't. Quality improves run over run without manual prompt tuning.

5. **Beliefs compress context** — runtime events are distilled into structured beliefs. Rooms receive high-fidelity context. The telephone game is defeated.

6. **Sub-DAGs compose** — complex tasks decompose into reusable template-backed sub-workflows. The designer can create micro-pipelines. The user stays in control of the execution shape.

7. **Decisions are traceable** — every agent choice traces back to its reasoning, its inputs, and the conventions that guided it. "Why did it do that?" is always answerable.
