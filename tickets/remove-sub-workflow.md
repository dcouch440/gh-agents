# Remove Dead `sub_workflow` Execution Mode

## Objective

Delete all code related to the `sub_workflow` execution mode. It was scaffolded end-to-end but never wired into any user-facing flow. No step is ever created with `execution_mode = "sub_workflow"`. The actual child workflow mechanism is the workforce pipeline (`child_workflow_id` + `task_agent_roster`), which is a completely separate system.

---

## Context

`sub_workflow` was designed to run an entire workflow as a child execution within a single step, using `sub_workflow_template_id` (references `run_templates`) and `parent_execution_id` (tracks execution hierarchy). Workforce child workflows use a different mechanism entirely: `child_workflow_id` on `workflow_steps` + roster entries with `child_step_id`, managed by the pipeline service.

The two systems share no tables, no fields, and no execution paths.

---

## Scope

~575 lines across 19 files. DB columns stay (committed migrations).

### Backend (254 lines)

| File | Lines | What |
|------|-------|------|
| `src/server/api/workflows/last_run_handlers.rs` | 96 | `child_execution_id`/`child_steps` fields on `StepLastRunResponse` + entire `sub_workflow` branch |
| `src/server/services/run_results/mod.rs` | 106 | `child_execution_id`/`child_steps` fields + `ChildStepResult` struct + `build_sub_workflow_result()` function |
| `src/db/pg_repo/collection.rs` | 37 | `create_child_execution()` and `list_child_executions()` |
| `src/server/api/workflows/types.rs` | 10 | `sub_workflow_template_id` field + `ChildStepResult` struct |
| `src/db/types/workflow.rs` | 4 | `sub_workflow_template_id` field + its default |
| `src/server/services/steps/mod.rs` | 1 | `sub_workflow_template_id` in `StepPayload` |

### Frontend (321 lines)

| File | Lines | What |
|------|-------|------|
| `frontend/src/stores/workflowExecutionStore/wsHandler.ts` | 76 | 3 WS event handlers for sub_workflow events |
| `frontend/src/stores/workflowExecutionStore/workflowExecutionStore.test.ts` | ~150 | Tests for dead WS events |
| `frontend/src/stores/activity/parseWsEvent.ts` | 31 | 3 case blocks for SUB_WORKFLOW events |
| `frontend/src/components/canvas/CanvasNode/registry.ts` | 12 | sub_workflow variant config + resolution |
| `frontend/src/components/canvas/mappers/nodes.ts` | 20 | sub_workflow node mapping block |
| `frontend/src/stores/activity/activityMessages.ts` | 10 | Message mapping entries |
| `frontend/src/types/activity.ts` | 12 | 3 event type constants + type definitions |
| `frontend/src/types/ws.ts` | 3 | SUB_WORKFLOW event type enum values |
| `frontend/src/types/workflow.ts` | 2 | `sub_workflow_template_id` fields |
| `frontend/src/components/canvas/CanvasNode/types.ts` | 1 | SUB_WORKFLOW enum value |
| `frontend/src/components/canvas/nodeDimensions.ts` | 1 | Dimensions config entry |
| `frontend/src/components/canvas/portPlacements.ts` | 2 | Port config entry |
| `frontend/src/components/canvas/canvasKinds/index.ts` | 1 | SUB_WORKFLOW enum value |

### Not Removing

- DB columns (`sub_workflow_template_id`, `parent_execution_id`) — committed migrations, nullable, no harm
- `WorkflowCollectionRepo` trait methods (`create_child_execution`, `list_child_executions`) — evaluate whether collection DAG uses them; if not, remove with the pg_repo impls

---

## Verification

```bash
~/.cargo/bin/cargo check
~/.cargo/bin/cargo clippy
~/.cargo/bin/cargo test
npx tsc --noEmit
npx eslint .
npx vitest run
```