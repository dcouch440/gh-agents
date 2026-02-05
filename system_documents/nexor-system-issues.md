# nexor System Issues & Limitations

Known limitations, bugs, and areas for improvement discovered during system exploration.

**Purpose:** Track technical debt and potential fixes for future development work.

---

## 1. Output Schema Validation

**Current Behavior:**
- Output schemas are instruction-based only
- Schema JSON injected into system prompt with instructions: "You MUST respond with valid JSON matching this schema"
- Response parsed from JSON or markdown code blocks via `parse_structured_output()`
- NO runtime validation against JSON Schema specification

**Issue:**
- LLM may produce invalid JSON (syntax errors)
- LLM may produce valid JSON that doesn't conform to schema
- Malformed JSON silently results in `None` structured output
- Non-conformant structures pass through without warning

**Impact:**
- Downstream workflow steps may receive:
  - Missing fields they expect
  - Wrong data types
  - Unexpected structure
  - Null values where data was expected
- Variable resolution fails silently (leaves `{variable}` in template)
- Difficult to debug data flow issues

**Potential Fix:**
- Add JSON Schema validation after parsing structured output
- Use `jsonschema` crate or similar validator
- Validation modes:
  - **Strict**: Reject non-conformant outputs, fail step execution
  - **Lenient**: Log warnings but continue
  - **Off**: Current behavior (instruction-only)
- Store validation errors in `agent_executions` table
- UI feedback: Show schema conformance status

**Code Location:**
- `/Users/davidcouch/Dev/gh-agents/src/server/executors/dag/mod.rs:1130-1162` (`parse_structured_output` function)
- `/Users/davidcouch/Dev/gh-agents/src/server/executors/dag/mod.rs:478-488` (schema injection)

---

## 2. Variable Resolution Error Handling

**Current Behavior:**
- Variable resolution via `resolve_variables()` function
- Missing variables left as literal `{variable_name}` string
- No error thrown, no warning logged
- Applies to both workflow-level and collection-level variables

**Issue:**
- Silent failures make debugging very difficult
- Agents receive literal `{variable}` strings in prompts
- Unclear whether variable name is wrong or upstream step failed
- No visibility into which variables resolved successfully

**Impact:**
- LLM may interpret `{variable}` as instruction rather than missing data
- Workflow appears to succeed but produces incorrect results
- Developers spend time debugging variable names vs data flow
- No clear signal that data dependency failed

**Potential Fix:**
- Add optional strict mode for variable resolution
- Validation levels:
  - **Strict**: Error on any unresolved variable, fail step execution
  - **Warn**: Log warning but continue, track in execution record
  - **Lenient**: Current behavior (silent)
- Store resolution status in `agent_executions`:
  - `variables_resolved`: array of successfully resolved variables
  - `variables_unresolved`: array of failed variable references
- UI feedback: Show variable resolution status in execution logs
- Workflow validation: Pre-flight check of variable paths before execution

**Code Location:**
- `/Users/davidcouch/Dev/gh-agents/src/server/executors/dag/mod.rs:165-243` (`resolve_variables` function)
- `/Users/davidcouch/Dev/gh-agents/src/server/executors/dag/mod.rs:245-271` (for-each resolution)

---

## 3. For-Each Cost Management

**Current Behavior:**
- For-each mode iterates ALL elements in array
- No iteration limits, warnings, or cost estimates
- Each iteration = full LLM execution (multiplier effect)
- Array size determined entirely by upstream step output

**Issue:**
- Large arrays cause massive, unexpected LLM costs
- 1000-element array with sonnet = 1000× sonnet cost
- No preview or confirmation of iteration count
- No way to limit iterations or sample array

**Impact:**
- Users discover high costs after execution completes
- Workflows with variable-size arrays have unpredictable costs
- No protection against runaway costs
- Difficult to budget for workflow execution

**Potential Fix:**

**Short-term (warnings):**
- Calculate estimated cost before for-each execution
- Display confirmation prompt if iterations > threshold (e.g., 50)
- Log iteration count and projected cost in execution record
- UI: Show cost estimate in workflow editor when for_each step selected

**Long-term (limits):**
- Add `max_iterations` field to workflow steps
- Add `sampling_strategy` option:
  - "all": Current behavior
  - "first_n": Take first N elements
  - "random_sample": Random sample of N elements
  - "stratified": Sample across subgroups
- Add cost budgets at workflow/collection level
- Halt execution if budget exceeded

**Code Location:**
- `/Users/davidcouch/Dev/gh-agents/src/server/executors/dag/mod.rs:898-998` (for-each execution)
- `/Users/davidcouch/Dev/gh-agents/src/server/executors/dag/mod.rs:359-435` (`resolve_for_each_array` function)

---

## 4. Document Token Budget

**Current Behavior:**
- Documents concatenate into prompts at execution time
- Agent-level documents appended first
- Step-level documents appended second
- No size limits, warnings, or token counting
- Full document content always included

**Issue:**
- Large documents can exceed model context windows
- No visibility into document size vs available budget
- Concatenation is all-or-nothing (no truncation)
- Context budget shared by: system prompt + documents + conversation history + new input

**Impact:**
- Execution failures when context exceeds limits
- Truncated context (older messages dropped)
- Unclear which documents are too large
- No guidance on document sizing

**Potential Fix:**

**Token Counting:**
- Add token counting for documents (use tiktoken or model-specific tokenizer)
- Store `token_count` field in `documents` table
- Calculate total context budget per execution
- Display token usage in workflow editor

**Budget Management:**
- Show total document tokens vs available context
- Warning when documents exceed threshold (e.g., 50% of context)
- Truncation strategies:
  - Drop lowest-priority documents first
  - Truncate individual documents (keep first N tokens)
  - Summarize long documents
- Per-step document budget limits

**UI Improvements:**
- Document editor shows token count as you type
- Workflow editor shows total context usage
- Execution logs show context breakdown

**Code Location:**
- `/Users/davidcouch/Dev/gh-agents/src/server/executors/dag/mod.rs:328-354` (document concatenation)
- `/Users/davidcouch/Dev/gh-agents/src/db/mod.rs` (DocumentRow definition)

---

## 5. Tool Assignment UX

**Current Behavior:**
- Manual tool assignment via `agent_tools` join table
- API: `PUT /api/agents/:id/tools` with array of tool IDs
- No guidance on which tools are needed
- No validation that tools match agent's purpose
- All-or-nothing assignment (must specify complete list)

**Issue:**
- Users don't know which tools to assign
- Over-tooling: Assign all tools "just in case" (slower, more confusing for LLM)
- Under-tooling: Missing critical tools, step fails
- No recommendations based on agent role or task

**Impact:**
- Inefficient agent execution (unnecessary tool descriptions)
- Failed executions (missing tools)
- Poor user experience (trial and error)
- Inconsistent tool usage across agents

**Potential Fix:**

**Tool Recommendations:**
- Suggest tools based on:
  - Agent system prompt analysis (keywords: "file", "git", "test", "research")
  - Agent role/pattern (Analyzer, Executor, Researcher, etc.)
  - Historical usage (similar agents used these tools)
- UI: "Recommended tools" section in agent editor
- Auto-assign option based on agent type

**Tool Validation:**
- Warn if agent has many tools but simple task
- Warn if agent system prompt mentions tool not assigned
- Suggest removing unused tools (based on execution history)

**Tool Groups/Presets:**
- Predefined tool sets:
  - "Code Editor": read_file, write_file, edit_file, list_files
  - "Git Manager": git_status, git_diff, git_commit, git_branch
  - "Test Runner": run_tests, run_command
  - "Researcher": web_search, x_search
- One-click assignment of tool groups

**Code Location:**
- `/Users/davidcouch/Dev/gh-agents/src/server/api/tools/mod.rs` (tool assignment API)
- `/Users/davidcouch/Dev/gh-agents/src/agents/execution_tools.rs` (tool definitions)

---

## 6. Workflow Cycle Detection

**Current Behavior:**
- Cycle detection during topological sort at execution time
- Uses Kahn's algorithm (checks if sorted list length matches node count)
- Error: "Cycle detected in workflow" (execution fails)
- No validation when edges are created or updated

**Issue:**
- Users don't discover cycles until workflow executes
- May have already completed expensive steps before cycle detected
- Cycle error doesn't identify which edges form the cycle
- No prevention at edge creation time (API level)

**Impact:**
- Failed workflow executions after resource consumption
- Poor debugging experience (which edges are problematic?)
- Wasted time and cost
- Frustrating user experience

**Potential Fix:**

**API-Level Validation:**
- Validate DAG on edge creation: `POST /api/workflows/:id/edges`
- Validate DAG on edge update/deletion
- Run topological sort before persisting edge
- Return specific error: "Adding this edge would create a cycle: A → B → C → A"

**Cycle Identification:**
- When cycle detected, identify the edges involved
- Return: `cycle_edges: [{from: A, to: B}, {from: B, to: C}, {from: C, to: A}]`
- UI: Highlight cycle edges in workflow graph

**Batch Validation:**
- Validate entire workflow on save
- Pre-flight check before execution
- Workflow status: "invalid" if cycle exists

**Code Location:**
- `/Users/davidcouch/Dev/gh-agents/src/server/executors/dag/mod.rs:82-131` (`topological_sort` function)
- `/Users/davidcouch/Dev/gh-agents/src/server/api/workflows/mod.rs` (edge creation API)

---

## 7. Interactive Review Timeout

**Current Behavior:**
- Interactive review sets status to `awaiting_user`
- Workflow execution pauses (throws `DagPaused` error)
- No timeout or expiration
- Workflow remains paused indefinitely until user approves/rejects
- No notifications or reminders

**Issue:**
- Workflows can pause forever if user forgets
- No automatic cleanup of stale review requests
- Unclear which workflows are waiting vs abandoned
- No timeout configuration per step or workflow

**Impact:**
- Orphaned workflow executions
- Unclear workflow state in UI
- Resources held unnecessarily
- No visibility into "waiting for how long?"

**Potential Fix:**

**Timeout Configuration:**
- Add `review_timeout_seconds` field to workflow steps
- Default: null (no timeout, current behavior)
- Optional: Set explicit timeout (e.g., 3600 = 1 hour)

**Timeout Behavior:**
- When timeout exceeded:
  - Option 1: Auto-reject review (fail workflow)
  - Option 2: Auto-approve review (continue workflow) - dangerous
  - Option 3: Notify user, extend timeout
- Store timeout expiration in `agent_executions` table
- Background job checks for expired reviews

**Notifications:**
- Email/webhook notification when review requested
- Reminder notification at 50% of timeout
- Timeout notification when expired

**UI Improvements:**
- Show "waiting for review" duration
- Show timeout countdown
- Allow extending timeout from UI

**Code Location:**
- `/Users/davidcouch/Dev/gh-agents/src/server/executors/dag/mod.rs:713-834` (`execute_interactive_review`)
- `/Users/davidcouch/Dev/gh-agents/src/server/api/agent_executions/mod.rs` (approve endpoint)

---

## 8. Collection Execution Observability

**Current Behavior:**
- Collections execute workflows as DAG nodes
- Limited WebSocket updates during execution
- Hard to see which workflows are running/blocked/completed
- No visibility into workflow dependencies at runtime
- Error messages often unclear about which workflow failed

**Issue:**
- Difficult to debug collection-level failures
- Unclear why workflow is blocked (waiting on what?)
- No visualization of collection execution state
- Limited logs for cross-workflow data flow

**Impact:**
- Poor debugging experience
- Users don't understand why collections are slow
- Hard to identify bottlenecks
- Difficult to optimize collection structure

**Potential Fix:**

**Enhanced WebSocket Updates:**
- Broadcast collection-level events:
  - `CollectionStarted`: { collection_id, workflow_count }
  - `WorkflowStarted`: { workflow_id, dependencies_completed }
  - `WorkflowCompleted`: { workflow_id, outputs, downstream_workflows }
  - `WorkflowFailed`: { workflow_id, error, impact_on_downstream }
  - `CollectionPaused`: { workflow_id, reason }
  - `CollectionCompleted`: { collection_id, total_cost, duration }

**Dependency Graph State:**
- Send current dependency graph state:
  - completed_workflows: [A, B]
  - running_workflows: [C, D]
  - blocked_workflows: [E (waiting on C), F (waiting on D)]
  - failed_workflows: []
- UI: Live visualization of collection DAG with status colors

**Execution Timeline:**
- Log workflow start/end times
- Show critical path (longest dependency chain)
- Identify parallelization opportunities

**Cross-Workflow Variable Tracking:**
- Log when workflows reference cross-workflow variables
- Show data lineage: "Workflow B uses $workflow_a.findings"
- Validate variable references before execution

**Code Location:**
- `/Users/davidcouch/Dev/gh-agents/src/server/executors/collection_dag/mod.rs:152-257` (parallel execution)
- `/Users/davidcouch/Dev/gh-agents/src/server/executors/collection_dag/mod.rs:113-150` (sequential execution)
- WebSocket broadcast logic throughout collection_dag module

---

## Priority Recommendations

**High Priority:**
1. **Output Schema Validation** - Data integrity issue affecting downstream steps
2. **Variable Resolution Error Handling** - Silent failures are hard to debug
3. **Workflow Cycle Detection** - Prevent wasted execution before runtime

**Medium Priority:**
4. **For-Each Cost Management** - Cost control for production workflows
5. **Tool Assignment UX** - Improve agent creation experience
6. **Collection Execution Observability** - Better debugging and monitoring

**Low Priority:**
7. **Document Token Budget** - Useful but not blocking
8. **Interactive Review Timeout** - Edge case for long-running reviews

---

## Notes

- These issues discovered through codebase exploration
- Not blocking for basic functionality
- Represent opportunities for improvement
- Consider user impact and implementation complexity when prioritizing
