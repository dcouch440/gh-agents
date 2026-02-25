// ============================================================================
// buildHistoricalEntries — Reconstruct ActivityEntry[] from persisted execution data
//
// Pure function: (execution, stepResults) → ActivityEntry[]
// Converts workflow execution history from the REST API into the same
// ActivityEntry format used by the live WebSocket flight recorder.
// ============================================================================

import { Collections } from '@/utils/collections'
import { ACTIVITY } from '@/types/activity'
import type { ActivityEvent } from '@/types/activity'
import type { ActivityEntry } from './activityStore'
import type { WorkflowExecutionSummary, RunStepResult } from '@/types/workflow'

// ── Helpers ──────────────────────────────────────────────────────────────────

let histSeq = 0

const makeEntry = (event: ActivityEvent, ts: string, runId: string): ActivityEntry => {
  const seq = histSeq++
  return {
    id: `hist_${seq}`,
    seq: -(seq + 1),
    event,
    ts,
    runId,
    userId: null,
    receivedAt: new Date(ts).getTime(),
  }
}

// ── Builder ──────────────────────────────────────────────────────────────────

const buildHistoricalEntries = (
  execution: WorkflowExecutionSummary,
  steps: readonly RunStepResult[],
): ActivityEntry[] => {
  histSeq = 0
  const entries: ActivityEntry[] = []
  const runId = execution.id
  const workflowId = execution.workflow_id

  // Filter once, sort once
  const activeSteps = Collections.filterMap(steps, (s) =>
    s.status !== 'skipped' ? s : null,
  )
  const sortedSteps = Collections.sortedCopy(activeSteps, (a, b) => {
    if (a.started_at === null && b.started_at === null) return 0
    if (a.started_at === null) return 1
    if (b.started_at === null) return -1
    return new Date(a.started_at).getTime() - new Date(b.started_at).getTime()
  })

  // Workflow started
  if (execution.started_at !== null) {
    entries.push(
      makeEntry(
        { type: ACTIVITY.WORKFLOW_STARTED, workflowId, totalSteps: activeSteps.length },
        execution.started_at,
        runId,
      ),
    )
  }

  for (const step of sortedSteps) {
    const stepName = step.step_name ?? 'Unknown step'
    const stepId = step.step_id

    // Step started
    if (step.started_at !== null) {
      entries.push(
        makeEntry(
          {
            type: ACTIVITY.WORKFLOW_STEP_STARTED,
            workflowId,
            stepId,
            stepName,
            agentId: null,
            executionId: step.execution_id,
          },
          step.started_at,
          runId,
        ),
      )
    }

    // Step completed or failed
    if (step.status === 'completed' || step.status === 'success') {
      const ts = step.completed_at ?? step.started_at ?? execution.started_at ?? new Date().toISOString()
      entries.push(
        makeEntry(
          {
            type: ACTIVITY.WORKFLOW_STEP_COMPLETED,
            workflowId,
            stepId,
            stepName,
            agentId: null,
            output: step.output,
            inputTokens: step.input_tokens,
            outputTokens: step.output_tokens,
            durationMs: step.duration_ms,
          },
          ts,
          runId,
        ),
      )
    } else if (step.status === 'failed') {
      const ts = step.completed_at ?? step.started_at ?? execution.started_at ?? new Date().toISOString()
      entries.push(
        makeEntry(
          {
            type: ACTIVITY.WORKFLOW_STEP_FAILED,
            workflowId,
            stepId,
            stepName,
            error: step.error ?? 'Unknown error',
          },
          ts,
          runId,
        ),
      )
    }
  }

  // Workflow completed or failed
  if (execution.status === 'completed' || execution.status === 'success') {
    const ts = execution.completed_at ?? execution.started_at ?? new Date().toISOString()
    const durationMs =
      execution.started_at !== null && execution.completed_at !== null
        ? new Date(execution.completed_at).getTime() - new Date(execution.started_at).getTime()
        : null
    entries.push(makeEntry({ type: ACTIVITY.WORKFLOW_COMPLETED, workflowId, durationMs }, ts, runId))
  } else if (execution.status === 'failed') {
    const ts = execution.completed_at ?? execution.started_at ?? new Date().toISOString()
    entries.push(
      makeEntry({ type: ACTIVITY.WORKFLOW_FAILED, workflowId, error: execution.error ?? 'Unknown error' }, ts, runId),
    )
  }

  return entries
}

export { buildHistoricalEntries }
